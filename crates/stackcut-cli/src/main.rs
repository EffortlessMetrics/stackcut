use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use stackcut_artifact::{
    compute_fingerprint, read_plan, render_summary, scaffold_overrides, write_diagnostics_envelope,
    write_plan, write_summary,
};
use stackcut_core::{
    parse_config, plan as build_plan, structural_validate, DiagnosticLevel, Overrides,
    StackcutConfig,
};
use std::fs;
use std::path::{Path, PathBuf};

/// Stable exit codes for the CLI.
///
/// Every command path resolves to exactly one exit code. The `main` function
/// wraps `run()` in a top-level catch that maps unexpected `anyhow::Error`
/// to `InternalBug` (10).
#[repr(u8)]
pub enum ExitCode {
    Success = 0,
    StructuralError = 1,
    RecompositionFailure = 2,
    OverrideConflict = 3,
    UnsupportedSurface = 4,
    InternalBug = 10,
}

#[derive(Debug, Parser)]
#[command(
    name = "stackcut",
    version,
    about = "Deterministic diff-to-stack compiler"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Create a stack plan from a git range.
    Plan {
        #[arg(long)]
        base: String,
        #[arg(long)]
        head: String,
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        #[arg(long, default_value = ".stackcut")]
        out_dir: PathBuf,
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        overrides: Option<PathBuf>,
    },
    /// Render a stored plan as readable Markdown.
    Explain { plan: PathBuf },
    /// Validate a stored plan.
    Validate {
        plan: PathBuf,
        #[arg(long)]
        exact: bool,
    },
    /// Materialize a stored plan into a patch series.
    Materialize {
        plan: PathBuf,
        #[arg(long, default_value = ".stackcut/patches")]
        out_dir: PathBuf,
        #[arg(long)]
        dry_run: bool,
    },
    /// Check repo readiness for stackcut.
    Doctor {
        /// Repository path to check.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
    },
    /// Generate an override.toml scaffold from a plan's ambiguities.
    ScaffoldOverrides {
        /// Path to the plan.json to scaffold from.
        plan: PathBuf,
        /// Output path for the generated override.toml.
        #[arg(long, default_value = ".stackcut/override.toml")]
        output: PathBuf,
        /// Overwrite existing file without prompting.
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("internal error: {e:#}");
            ExitCode::InternalBug as i32
        }
    };
    std::process::exit(code);
}

fn run() -> Result<i32> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Plan {
            base,
            head,
            repo,
            out_dir,
            config,
            overrides,
        } => cmd_plan(
            &repo,
            &base,
            &head,
            &out_dir,
            config.as_deref(),
            overrides.as_deref(),
        ),
        Commands::Explain { plan } => cmd_explain(&plan),
        Commands::Validate { plan, exact } => cmd_validate(&plan, exact),
        Commands::Materialize {
            plan,
            out_dir,
            dry_run,
        } => cmd_materialize(&plan, &out_dir, dry_run),
        Commands::Doctor { repo } => cmd_doctor(&repo),
        Commands::ScaffoldOverrides {
            plan,
            output,
            force,
        } => cmd_scaffold_overrides(&plan, &output, force),
    }
}

fn cmd_plan(
    repo: &Path,
    base: &str,
    head: &str,
    out_dir: &Path,
    config_path: Option<&Path>,
    override_path: Option<&Path>,
) -> Result<i32> {
    let repo_root = stackcut_git::discover_repo_root(repo)
        .with_context(|| format!("failed to discover git repo from {}", repo.display()))?;

    let default_config = existing_path(repo_root.join("stackcut.toml"));
    let default_overrides = existing_path(repo_root.join(".stackcut/override.toml"));

    let config = match config_path.or(default_config.as_deref()) {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let (config, config_diagnostics) = parse_config(&contents)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            for diag in &config_diagnostics {
                eprintln!("config: {:?} {}: {}", diag.level, diag.code, diag.message);
            }
            config
        }
        None => StackcutConfig::default(),
    };
    let overrides =
        load_toml_or_default::<Overrides>(override_path.or(default_overrides.as_deref()))?;

    let (source, units) = stackcut_git::collect_edit_units(&repo_root, base, head, &config)?;
    let plan = build_plan(source, units, &config, &overrides);

    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let plan_path = out_dir.join("plan.json");
    let summary_path = out_dir.join("summary.md");
    let diagnostics_path = out_dir.join("diagnostics.json");

    write_plan(&plan_path, &plan)?;
    write_summary(&summary_path, &plan)?;
    write_diagnostics_envelope(&diagnostics_path, &plan)?;

    println!("wrote {}", plan_path.display());
    println!("wrote {}", summary_path.display());
    println!("wrote {}", diagnostics_path.display());
    Ok(ExitCode::Success as i32)
}

fn cmd_explain(plan_path: &Path) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    print!("{}", render_summary(&plan));
    Ok(ExitCode::Success as i32)
}

fn cmd_validate(plan_path: &Path, exact: bool) -> Result<i32> {
    let plan = read_plan(plan_path)?;

    // Version check: reject plans with unsupported versions
    if plan.version != stackcut_core::PLAN_VERSION {
        eprintln!(
            "error: plan version '{}' is not supported (expected '{}')",
            plan.version,
            stackcut_core::PLAN_VERSION
        );
        return Ok(ExitCode::StructuralError as i32);
    }

    // Fingerprint verification (if present)
    if let Some(ref fp) = plan.fingerprint {
        let computed = compute_fingerprint(&plan);
        if *fp != computed {
            eprintln!(
                "warning: plan fingerprint mismatch (expected {}, got {})",
                fp, computed
            );
        }
    }

    let diagnostics = structural_validate(&plan);
    let has_errors = diagnostics
        .iter()
        .any(|d| d.level == DiagnosticLevel::Error);

    if diagnostics.is_empty() {
        println!("structural validation: ok");
    } else {
        println!("structural validation:");
        for diagnostic in &diagnostics {
            println!(
                "- {:?} {}: {}",
                diagnostic.level, diagnostic.code, diagnostic.message
            );
        }
    }

    if has_errors {
        return Ok(ExitCode::StructuralError as i32);
    }

    if exact {
        let repo_root = plan
            .source
            .repo_root
            .as_ref()
            .map(PathBuf::from)
            .context("plan is missing source.repo_root; exact validation is unavailable")?;
        match stackcut_git::validate_exact_recomposition(&repo_root, &plan) {
            Ok(()) => println!("exact recomposition: ok"),
            Err(e) => {
                eprintln!("exact recomposition failed: {e}");
                return Ok(ExitCode::RecompositionFailure as i32);
            }
        }
    }

    Ok(ExitCode::Success as i32)
}

fn cmd_materialize(plan_path: &Path, out_dir: &Path, dry_run: bool) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    let repo_root = plan
        .source
        .repo_root
        .as_ref()
        .map(PathBuf::from)
        .context("plan is missing source.repo_root; cannot materialize patches")?;
    let written = stackcut_git::materialize_patches(&repo_root, &plan, out_dir, dry_run)?;
    for path in written {
        println!("{}", path.display());
    }
    Ok(ExitCode::Success as i32)
}

// ── Doctor command ─────────────────────────────────────────────────────

#[derive(Debug)]
struct DoctorCheck {
    name: String,
    status: DoctorStatus,
    message: String,
}

#[derive(Debug, PartialEq)]
enum DoctorStatus {
    Ok,
    Warning,
    Error,
}

impl std::fmt::Display for DoctorStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoctorStatus::Ok => write!(f, "ok"),
            DoctorStatus::Warning => write!(f, "warn"),
            DoctorStatus::Error => write!(f, "ERROR"),
        }
    }
}

fn check_git_repo(repo: &Path) -> (DoctorCheck, Option<PathBuf>) {
    match stackcut_git::discover_repo_root(repo) {
        Ok(root) => {
            let check = DoctorCheck {
                name: "git-repo".to_string(),
                status: DoctorStatus::Ok,
                message: format!("git repository found at {}", root.display()),
            };
            (check, Some(root))
        }
        Err(_) => {
            let check = DoctorCheck {
                name: "git-repo".to_string(),
                status: DoctorStatus::Error,
                message: "no git repository found".to_string(),
            };
            (check, None)
        }
    }
}

fn check_config_file(repo_root: &Path) -> DoctorCheck {
    let config_path = repo_root.join("stackcut.toml");
    if config_path.exists() {
        DoctorCheck {
            name: "config-file".to_string(),
            status: DoctorStatus::Ok,
            message: "stackcut.toml found".to_string(),
        }
    } else {
        DoctorCheck {
            name: "config-file".to_string(),
            status: DoctorStatus::Warning,
            message: "stackcut.toml not found — defaults will be used".to_string(),
        }
    }
}

fn check_config_parse(repo_root: &Path) -> (Option<DoctorCheck>, Option<StackcutConfig>) {
    let config_path = repo_root.join("stackcut.toml");
    if !config_path.exists() {
        return (None, None);
    }
    let contents = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(e) => {
            return (
                Some(DoctorCheck {
                    name: "config-parse".to_string(),
                    status: DoctorStatus::Error,
                    message: format!("failed to read stackcut.toml: {e}"),
                }),
                None,
            );
        }
    };
    match parse_config(&contents) {
        Ok((config, diagnostics)) => {
            let check = if diagnostics.is_empty() {
                DoctorCheck {
                    name: "config-parse".to_string(),
                    status: DoctorStatus::Ok,
                    message: "config parses cleanly".to_string(),
                }
            } else {
                let msgs: Vec<String> = diagnostics.iter().map(|d| d.message.clone()).collect();
                DoctorCheck {
                    name: "config-parse".to_string(),
                    status: DoctorStatus::Warning,
                    message: format!("config parsed with warnings: {}", msgs.join("; ")),
                }
            };
            (Some(check), Some(config))
        }
        Err(e) => (
            Some(DoctorCheck {
                name: "config-parse".to_string(),
                status: DoctorStatus::Error,
                message: format!("config parse error: {e}"),
            }),
            None,
        ),
    }
}

fn check_path_families(config: &StackcutConfig) -> DoctorCheck {
    if config.path_families.is_empty() {
        DoctorCheck {
            name: "path-families".to_string(),
            status: DoctorStatus::Warning,
            message: "no path_families configured — family inference will use directory heuristics"
                .to_string(),
        }
    } else {
        DoctorCheck {
            name: "path-families".to_string(),
            status: DoctorStatus::Ok,
            message: format!(
                "{} path_families rule(s) configured",
                config.path_families.len()
            ),
        }
    }
}

fn check_override_file(repo_root: &Path) -> DoctorCheck {
    let override_path = repo_root.join(".stackcut/override.toml");
    if override_path.exists() {
        DoctorCheck {
            name: "override-file".to_string(),
            status: DoctorStatus::Ok,
            message: "override.toml found at .stackcut/override.toml".to_string(),
        }
    } else {
        DoctorCheck {
            name: "override-file".to_string(),
            status: DoctorStatus::Ok,
            message: ".stackcut/override.toml not found — no overrides active".to_string(),
        }
    }
}

fn check_output_directory(repo_root: &Path) -> DoctorCheck {
    let dir = repo_root.join(".stackcut");
    if dir.is_dir() {
        DoctorCheck {
            name: "output-dir".to_string(),
            status: DoctorStatus::Ok,
            message: ".stackcut/ directory exists".to_string(),
        }
    } else if dir.exists() {
        DoctorCheck {
            name: "output-dir".to_string(),
            status: DoctorStatus::Error,
            message: ".stackcut exists but is not a directory — this will block normal operation"
                .to_string(),
        }
    } else {
        DoctorCheck {
            name: "output-dir".to_string(),
            status: DoctorStatus::Ok,
            message: ".stackcut/ directory not found — will be created on first plan".to_string(),
        }
    }
}

fn check_manifest_coverage(repo_root: &Path, config: &StackcutConfig) -> DoctorCheck {
    // Match by exact path OR by filename, consistent with classify_path in
    // stackcut-core which checks `entry == file_name || entry == path`.
    let found: Vec<String> = config
        .manifest_files
        .iter()
        .filter(|entry| {
            // Direct path match (e.g. "Cargo.toml" at repo root)
            if repo_root.join(entry).exists() {
                return true;
            }
            // Filename match: scan repo root entries for a matching name.
            // The planner matches any file whose basename equals the entry,
            // so we check immediate children of the repo root.
            if let Ok(entries) = fs::read_dir(repo_root) {
                for dir_entry in entries.flatten() {
                    if let Some(name) = dir_entry.file_name().to_str() {
                        if name == entry.as_str() {
                            return true;
                        }
                    }
                }
            }
            false
        })
        .cloned()
        .collect();

    if found.is_empty() {
        DoctorCheck {
            name: "manifest-coverage".to_string(),
            status: DoctorStatus::Warning,
            message: format!(
                "no configured manifest files found (checked: {})",
                config.manifest_files.join(", ")
            ),
        }
    } else {
        DoctorCheck {
            name: "manifest-coverage".to_string(),
            status: DoctorStatus::Ok,
            message: format!("manifest files found: {}", found.join(", ")),
        }
    }
}

fn check_codeowners(repo_root: &Path) -> DoctorCheck {
    let candidates = ["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"];
    for candidate in &candidates {
        if repo_root.join(candidate).exists() {
            return DoctorCheck {
                name: "codeowners".to_string(),
                status: DoctorStatus::Ok,
                message: format!("CODEOWNERS found at {candidate}"),
            };
        }
    }
    DoctorCheck {
        name: "codeowners".to_string(),
        status: DoctorStatus::Warning,
        message: "no CODEOWNERS file found".to_string(),
    }
}

fn run_doctor_checks(repo: &Path) -> Vec<DoctorCheck> {
    let mut checks = Vec::new();

    // 1. Git repo check — also returns the discovered root (comment A fix)
    let (git_check, repo_root) = check_git_repo(repo);
    checks.push(git_check);

    let repo_root = match repo_root {
        Some(root) => root,
        None => return checks,
    };

    // 2. Config file
    checks.push(check_config_file(&repo_root));

    // 3. Config parse — also returns the parsed config for reuse (comment B fix)
    let (parse_check, parsed_config) = check_config_parse(&repo_root);
    if let Some(check) = parse_check {
        checks.push(check);
    }

    // Use parsed config if available, otherwise fall back to defaults
    let config = parsed_config.unwrap_or_default();

    // 4. Path families (uses shared parsed config)
    checks.push(check_path_families(&config));

    // 5. Override file
    checks.push(check_override_file(&repo_root));

    // 6. Output directory
    checks.push(check_output_directory(&repo_root));

    // 7. Manifest coverage (uses shared parsed config)
    checks.push(check_manifest_coverage(&repo_root, &config));

    // 8. CODEOWNERS
    checks.push(check_codeowners(&repo_root));

    checks
}

fn cmd_doctor(repo: &Path) -> Result<i32> {
    println!("stackcut doctor\n");

    let checks = run_doctor_checks(repo);

    let mut ok_count = 0u32;
    let mut warn_count = 0u32;
    let mut error_count = 0u32;

    for check in &checks {
        match check.status {
            DoctorStatus::Ok => ok_count += 1,
            DoctorStatus::Warning => warn_count += 1,
            DoctorStatus::Error => error_count += 1,
        }
        println!("[{}] {}: {}", check.status, check.name, check.message);
    }

    println!(
        "\n{} ok, {} warnings, {} errors",
        ok_count, warn_count, error_count
    );

    if error_count > 0 {
        Ok(ExitCode::StructuralError as i32)
    } else {
        Ok(ExitCode::Success as i32)
    }
}

fn cmd_scaffold_overrides(plan_path: &Path, output: &Path, force: bool) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    let toml_text = scaffold_overrides(&plan);

    if output.exists() && !force {
        eprintln!(
            "error: {} already exists (use --force to overwrite)",
            output.display()
        );
        return Ok(ExitCode::StructuralError as i32);
    }

    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(output, &toml_text)
        .with_context(|| format!("failed to write {}", output.display()))?;

    println!("wrote {}", output.display());
    Ok(ExitCode::Success as i32)
}

fn load_toml_or_default<T>(path: Option<&Path>) -> Result<T>
where
    T: DeserializeOwned + Default,
{
    match path {
        Some(path) => {
            let contents = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let value = toml::from_str::<T>(&contents)
                .with_context(|| format!("failed to parse {}", path.display()))?;
            Ok(value)
        }
        None => Ok(T::default()),
    }
}

fn existing_path(path: PathBuf) -> Option<PathBuf> {
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use stackcut_core::{
        ChangeStatus, EditUnit, InclusionReason, PathFamilyRule, Plan, PlanSource, ProofSurface,
        Slice, SliceKind, UnitKind, PLAN_VERSION,
    };

    /// Build a minimal valid Plan with the given version string.
    fn minimal_plan(version: &str) -> Plan {
        let unit = EditUnit {
            id: "path:src/main.rs".to_string(),
            path: "src/main.rs".to_string(),
            old_path: None,
            status: ChangeStatus::Modified,
            kind: UnitKind::Behavior,
            family: "cli".to_string(),
            notes: Vec::new(),
        };
        Plan {
            version: version.to_string(),
            source: PlanSource {
                repo_root: None,
                base: "aaa".to_string(),
                head: "bbb".to_string(),
                head_tree: None,
            },
            units: vec![unit],
            slices: vec![Slice {
                id: "behavior-cli".to_string(),
                title: "Behavior: cli".to_string(),
                kind: SliceKind::Behavior,
                families: vec!["cli".to_string()],
                members: vec!["path:src/main.rs".to_string()],
                depends_on: Vec::new(),
                reasons: vec![InclusionReason {
                    code: "family-grouping".to_string(),
                    message: "test".to_string(),
                }],
                proof_surface: ProofSurface::default(),
            }],
            ambiguities: Vec::new(),
            diagnostics: Vec::new(),
            fingerprint: None,
        }
    }

    /// Strategy that generates version strings guaranteed to differ from PLAN_VERSION.
    fn arb_non_plan_version() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9._-]{1,20}".prop_filter("version must differ from PLAN_VERSION", |v| {
            v != PLAN_VERSION
        })
    }

    // ── Snapshot tests: CLI --help output (Task 15.3) ───────────────────
    // Validates: Requirements 22.3

    #[test]
    fn snapshot_cli_has_expected_subcommands() {
        use clap::CommandFactory;
        let cmd = Cli::command();

        // Collect subcommand names
        let subcommand_names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();

        assert!(
            subcommand_names.contains(&"plan"),
            "CLI missing 'plan' subcommand"
        );
        assert!(
            subcommand_names.contains(&"explain"),
            "CLI missing 'explain' subcommand"
        );
        assert!(
            subcommand_names.contains(&"validate"),
            "CLI missing 'validate' subcommand"
        );
        assert!(
            subcommand_names.contains(&"materialize"),
            "CLI missing 'materialize' subcommand"
        );
        assert!(
            subcommand_names.contains(&"doctor"),
            "CLI missing 'doctor' subcommand"
        );
        assert!(
            subcommand_names.contains(&"scaffold-overrides"),
            "CLI missing 'scaffold-overrides' subcommand"
        );
    }

    #[test]
    fn snapshot_cli_help_output_stable() {
        use clap::CommandFactory;

        // Root help
        let mut cmd = Cli::command();
        let mut buf = Vec::new();
        cmd.write_long_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();

        assert!(help.contains("stackcut"), "Root help missing program name");
        assert!(help.contains("plan"), "Root help missing 'plan' subcommand");
        assert!(
            help.contains("explain"),
            "Root help missing 'explain' subcommand"
        );
        assert!(
            help.contains("validate"),
            "Root help missing 'validate' subcommand"
        );
        assert!(
            help.contains("materialize"),
            "Root help missing 'materialize' subcommand"
        );
        assert!(
            help.contains("doctor"),
            "Root help missing 'doctor' subcommand"
        );
        assert!(
            help.contains("scaffold-overrides"),
            "Root help missing 'scaffold-overrides' subcommand"
        );

        // Stability: generating help twice produces identical output
        let mut cmd2 = Cli::command();
        let mut buf2 = Vec::new();
        cmd2.write_long_help(&mut buf2).unwrap();
        let help2 = String::from_utf8(buf2).unwrap();
        assert_eq!(help, help2, "CLI help output is not stable across calls");
    }

    #[test]
    fn snapshot_subcommand_help_contains_expected_args() {
        use clap::CommandFactory;
        let cmd = Cli::command();

        for sub in cmd.get_subcommands() {
            let name = sub.get_name().to_string();
            let mut sub_clone = sub.clone();
            let mut buf = Vec::new();
            sub_clone.write_long_help(&mut buf).unwrap();
            let help = String::from_utf8(buf).unwrap();

            match name.as_str() {
                "plan" => {
                    assert!(help.contains("--base"), "plan help missing --base");
                    assert!(help.contains("--head"), "plan help missing --head");
                    assert!(help.contains("--repo"), "plan help missing --repo");
                    assert!(help.contains("--out-dir"), "plan help missing --out-dir");
                }
                "explain" => {
                    assert!(
                        help.contains("<PLAN>") || help.contains("plan"),
                        "explain help missing plan argument"
                    );
                }
                "validate" => {
                    assert!(help.contains("--exact"), "validate help missing --exact");
                }
                "materialize" => {
                    assert!(
                        help.contains("--dry-run"),
                        "materialize help missing --dry-run"
                    );
                    assert!(
                        help.contains("--out-dir"),
                        "materialize help missing --out-dir"
                    );
                }
                "doctor" => {
                    assert!(help.contains("--repo"), "doctor help missing --repo");
                }
                "scaffold-overrides" => {
                    assert!(
                        help.contains("--output"),
                        "scaffold-overrides help missing --output"
                    );
                    assert!(
                        help.contains("--force"),
                        "scaffold-overrides help missing --force"
                    );
                }
                _ => {} // help subcommand auto-added by clap
            }
        }
    }

    // ── Doctor command tests ────────────────────────────────────────────

    #[test]
    fn doctor_check_config_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let check = check_config_file(dir.path());
        assert_eq!(check.status, DoctorStatus::Warning);
        assert!(check.message.contains("not found"));
    }

    #[test]
    fn doctor_check_config_file_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("stackcut.toml"), "").unwrap();
        let check = check_config_file(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
    }

    #[test]
    fn doctor_check_config_parse_valid() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("stackcut.toml"), "version = 1\n").unwrap();
        let (check, config) = check_config_parse(dir.path());
        let check = check.unwrap();
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("parses cleanly"));
        assert!(config.is_some(), "parsed config should be returned");
    }

    #[test]
    fn doctor_check_config_parse_invalid() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("stackcut.toml"), "{{bad toml").unwrap();
        let (check, config) = check_config_parse(dir.path());
        let check = check.unwrap();
        assert_eq!(check.status, DoctorStatus::Error);
        assert!(config.is_none(), "no config on parse failure");
    }

    #[test]
    fn doctor_check_config_parse_unknown_keys() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("stackcut.toml"),
            "version = 1\nfoo_unknown = true\n",
        )
        .unwrap();
        let (check, config) = check_config_parse(dir.path());
        let check = check.unwrap();
        assert_eq!(check.status, DoctorStatus::Warning);
        assert!(check.message.contains("warnings"));
        assert!(config.is_some(), "config returned even with warnings");
    }

    #[test]
    fn doctor_check_config_parse_absent() {
        let dir = tempfile::tempdir().unwrap();
        let (check, config) = check_config_parse(dir.path());
        assert!(check.is_none());
        assert!(config.is_none());
    }

    #[test]
    fn doctor_check_path_families_empty() {
        // Config with empty path_families
        let config = StackcutConfig {
            path_families: vec![],
            ..StackcutConfig::default()
        };
        let check = check_path_families(&config);
        assert_eq!(check.status, DoctorStatus::Warning);
        assert!(check.message.contains("no path_families"));
    }

    #[test]
    fn doctor_check_path_families_present() {
        let config = StackcutConfig {
            path_families: vec![PathFamilyRule {
                prefix: "src/".to_string(),
                family: "core".to_string(),
            }],
            ..StackcutConfig::default()
        };
        let check = check_path_families(&config);
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("1 path_families rule(s)"));
    }

    #[test]
    fn doctor_check_override_file_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".stackcut")).unwrap();
        fs::write(dir.path().join(".stackcut/override.toml"), "").unwrap();
        let check = check_override_file(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("override.toml found"));
    }

    #[test]
    fn doctor_check_override_file_absent() {
        let dir = tempfile::tempdir().unwrap();
        let check = check_override_file(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("not found"));
    }

    #[test]
    fn doctor_check_output_directory_present() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".stackcut")).unwrap();
        let check = check_output_directory(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("exists"));
    }

    #[test]
    fn doctor_check_output_directory_absent() {
        let dir = tempfile::tempdir().unwrap();
        let check = check_output_directory(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("will be created"));
    }

    #[test]
    fn doctor_check_output_directory_is_regular_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".stackcut"), "not a directory").unwrap();
        let check = check_output_directory(dir.path());
        assert_eq!(check.status, DoctorStatus::Error);
        assert!(check.message.contains("not a directory"));
    }

    #[test]
    fn doctor_check_manifest_coverage_found() {
        let dir = tempfile::tempdir().unwrap();
        let config = StackcutConfig::default();
        // Use default config which includes Cargo.toml
        fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        let check = check_manifest_coverage(dir.path(), &config);
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("Cargo.toml"));
    }

    #[test]
    fn doctor_check_manifest_coverage_none() {
        let dir = tempfile::tempdir().unwrap();
        let config = StackcutConfig::default();
        let check = check_manifest_coverage(dir.path(), &config);
        assert_eq!(check.status, DoctorStatus::Warning);
        assert!(check.message.contains("no configured manifest files found"));
    }

    #[test]
    fn doctor_check_manifest_coverage_matches_by_filename() {
        let dir = tempfile::tempdir().unwrap();
        // The file is at the root level with a matching basename
        let config = StackcutConfig {
            manifest_files: vec!["Cargo.toml".to_string()],
            ..StackcutConfig::default()
        };
        // Place the file at the repo root (filename match)
        fs::write(dir.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        let check = check_manifest_coverage(dir.path(), &config);
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("Cargo.toml"));
    }

    #[test]
    fn doctor_check_codeowners_github() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join(".github")).unwrap();
        fs::write(dir.path().join(".github/CODEOWNERS"), "* @team\n").unwrap();
        let check = check_codeowners(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains(".github/CODEOWNERS"));
    }

    #[test]
    fn doctor_check_codeowners_root() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("CODEOWNERS"), "* @team\n").unwrap();
        let check = check_codeowners(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("CODEOWNERS"));
    }

    #[test]
    fn doctor_check_codeowners_docs() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("docs")).unwrap();
        fs::write(dir.path().join("docs/CODEOWNERS"), "* @team\n").unwrap();
        let check = check_codeowners(dir.path());
        assert_eq!(check.status, DoctorStatus::Ok);
        assert!(check.message.contains("docs/CODEOWNERS"));
    }

    #[test]
    fn doctor_check_codeowners_missing() {
        let dir = tempfile::tempdir().unwrap();
        let check = check_codeowners(dir.path());
        assert_eq!(check.status, DoctorStatus::Warning);
        assert!(check.message.contains("no CODEOWNERS file found"));
    }

    #[test]
    fn doctor_check_git_repo_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let (check, root) = check_git_repo(dir.path());
        assert_eq!(check.status, DoctorStatus::Error);
        assert!(check.message.contains("no git repository found"));
        assert!(root.is_none());
    }

    // Feature: stackcut-v01-completion, Property 22: Unsupported Plan Version Rejection
    // **Validates: Requirements 13.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unsupported_plan_version_rejected(version in arb_non_plan_version()) {
            let plan = minimal_plan(&version);
            let dir = tempfile::tempdir().unwrap();
            let plan_path = dir.path().join("plan.json");
            let json = serde_json::to_string_pretty(&plan).unwrap();
            std::fs::write(&plan_path, format!("{json}\n")).unwrap();

            let result = cmd_validate(&plan_path, false).unwrap();
            prop_assert_eq!(
                result,
                ExitCode::StructuralError as i32,
                "Expected exit code 1 for unsupported version '{}', got {}",
                version,
                result
            );
        }
    }

    // ── scaffold-overrides tests ───────────────────────────────────────

    #[test]
    fn scaffold_overrides_writes_file_and_returns_success() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let output = dir.path().join("override.toml");
        let result = cmd_scaffold_overrides(&plan_path, &output, false).unwrap();
        assert_eq!(result, ExitCode::Success as i32);
        assert!(output.exists(), "override.toml should be created");

        let contents = std::fs::read_to_string(&output).unwrap();
        assert!(
            contents.contains("version = 1"),
            "output should contain version = 1"
        );
    }

    #[test]
    fn scaffold_overrides_refuses_overwrite_without_force() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let output = dir.path().join("override.toml");
        std::fs::write(&output, "existing content").unwrap();

        let result = cmd_scaffold_overrides(&plan_path, &output, false).unwrap();
        assert_eq!(
            result,
            ExitCode::StructuralError as i32,
            "should refuse to overwrite without --force"
        );

        // Original content should be preserved
        let contents = std::fs::read_to_string(&output).unwrap();
        assert_eq!(contents, "existing content");
    }

    #[test]
    fn scaffold_overrides_overwrites_with_force() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let output = dir.path().join("override.toml");
        std::fs::write(&output, "existing content").unwrap();

        let result = cmd_scaffold_overrides(&plan_path, &output, true).unwrap();
        assert_eq!(result, ExitCode::Success as i32);

        let contents = std::fs::read_to_string(&output).unwrap();
        assert!(
            contents.contains("version = 1"),
            "overwritten file should contain scaffold output"
        );
    }
}
