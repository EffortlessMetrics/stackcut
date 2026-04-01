use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use stackcut_artifact::{
    compute_fingerprint, read_plan, render_summary, write_diagnostics_envelope, write_plan,
    write_summary,
};
use stackcut_core::{
    parse_config, plan as build_plan, structural_validate, DiagnosticLevel, Overrides,
    PathFamilyRule, StackcutConfig,
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
    /// Initialize stackcut in a repository.
    Init {
        /// Repository path.
        #[arg(long, default_value = ".")]
        repo: PathBuf,
        /// Overwrite existing stackcut.toml.
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
        Commands::Init { repo, force } => cmd_init(&repo, force),
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

fn cmd_init(repo: &Path, force: bool) -> Result<i32> {
    let repo_root = stackcut_git::discover_repo_root(repo)
        .with_context(|| format!("failed to discover git repo from {}", repo.display()))?;

    let config_path = repo_root.join("stackcut.toml");
    if config_path.exists() && !force {
        eprintln!("stackcut.toml already exists (use --force to overwrite)");
        return Ok(ExitCode::StructuralError as i32);
    }

    let config = generate_initial_config(&repo_root);
    let toml_content = render_config_toml(&config);

    fs::write(&config_path, &toml_content)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    println!("wrote {}", config_path.display());

    let stackcut_dir = repo_root.join(".stackcut");
    if !stackcut_dir.exists() {
        fs::create_dir_all(&stackcut_dir)
            .with_context(|| format!("failed to create {}", stackcut_dir.display()))?;
        println!("created {}", stackcut_dir.display());
    }

    Ok(ExitCode::Success as i32)
}

fn generate_initial_config(repo_root: &Path) -> StackcutConfig {
    let manifest_candidates = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "pom.xml",
        "build.gradle",
    ];
    let manifest_files: Vec<String> = manifest_candidates
        .iter()
        .filter(|f| repo_root.join(f).exists())
        .map(|f| f.to_string())
        .collect();

    let lock_candidates = [
        "Cargo.lock",
        "package-lock.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "poetry.lock",
        "go.sum",
    ];
    let lock_files: Vec<String> = lock_candidates
        .iter()
        .filter(|f| repo_root.join(f).exists())
        .map(|f| f.to_string())
        .collect();

    let mut path_families: Vec<PathFamilyRule> = Vec::new();
    for dir_name in &["src", "crates", "packages"] {
        let dir = repo_root.join(dir_name);
        if let Ok(entries) = fs::read_dir(&dir) {
            let mut subdirs: Vec<String> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect();
            subdirs.sort();
            for name in subdirs {
                path_families.push(PathFamilyRule {
                    prefix: format!("{}/{}/", dir_name, name),
                    family: name,
                });
            }
        }
    }

    let test_candidates = ["tests/", "test/", "specs/", "spec/", "__tests__/"];
    let mut test_prefixes: Vec<String> = test_candidates
        .iter()
        .filter(|d| repo_root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();
    if test_prefixes.is_empty() {
        test_prefixes.push("tests/".to_string());
    }

    let doc_candidates = ["docs/", "doc/", "adr/"];
    let doc_prefixes: Vec<String> = doc_candidates
        .iter()
        .filter(|d| repo_root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();

    let ops_candidates = [".github/", "ci/", ".circleci/", ".gitlab-ci/"];
    let ops_prefixes: Vec<String> = ops_candidates
        .iter()
        .filter(|d| repo_root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();

    let generated_candidates = ["dist/", "build/", "generated/", "fixtures/generated/"];
    let mut generated_prefixes: Vec<String> = generated_candidates
        .iter()
        .filter(|d| repo_root.join(d).is_dir())
        .map(|d| d.to_string())
        .collect();
    if generated_prefixes.is_empty() {
        generated_prefixes.push("generated/".to_string());
    }

    StackcutConfig {
        version: 1,
        generated_prefixes,
        manifest_files,
        lock_files,
        test_prefixes,
        doc_prefixes,
        ops_prefixes,
        path_families,
        review_budget: None,
    }
}

fn render_config_toml(config: &StackcutConfig) -> String {
    let mut out = String::new();

    out.push_str("# stackcut configuration\n");
    out.push_str("# See https://github.com/stackcut/stackcut for documentation\n");
    out.push_str(&format!("version = {}\n", config.version));

    out.push('\n');
    out.push_str("# Files treated as generated output (mechanical, not reviewed individually)\n");
    out.push_str(&format!(
        "generated_prefixes = {}\n",
        format_string_array(&config.generated_prefixes)
    ));

    out.push('\n');
    out.push_str("# Package manifest files (grouped with lock files in the same slice)\n");
    out.push_str(&format!(
        "manifest_files = {}\n",
        format_string_array(&config.manifest_files)
    ));

    out.push('\n');
    out.push_str("# Lock files (always move with their manifest)\n");
    out.push_str(&format!(
        "lock_files = {}\n",
        format_string_array(&config.lock_files)
    ));

    out.push('\n');
    out.push_str("# Directories containing tests\n");
    out.push_str(&format!(
        "test_prefixes = {}\n",
        format_string_array(&config.test_prefixes)
    ));

    out.push('\n');
    out.push_str("# Directories containing documentation\n");
    out.push_str(&format!(
        "doc_prefixes = {}\n",
        format_string_array(&config.doc_prefixes)
    ));

    out.push('\n');
    out.push_str("# Directories containing CI/ops configuration\n");
    out.push_str(&format!(
        "ops_prefixes = {}\n",
        format_string_array(&config.ops_prefixes)
    ));

    if !config.path_families.is_empty() {
        out.push('\n');
        out.push_str("# Path-to-family mappings for the planner\n");
        out.push_str("# The planner groups files by family; files under the same prefix\n");
        out.push_str("# are assumed to belong together.\n");
        for rule in &config.path_families {
            out.push_str(&format!(
                "\n[[path_families]]\nprefix = \"{}\"\nfamily = \"{}\"\n",
                rule.prefix, rule.family
            ));
        }
    }

    out.push('\n');
    out.push_str("# Optional: maximum files per slice before a review-budget warning fires\n");
    match config.review_budget {
        Some(budget) => out.push_str(&format!("review_budget = {}\n", budget)),
        None => out.push_str("# review_budget = 15\n"),
    }

    out
}

fn format_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let inner: Vec<String> = items.iter().map(|s| format!("\"{}\"", s)).collect();
    format!("[{}]", inner.join(", "))
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
        ChangeStatus, EditUnit, InclusionReason, Plan, PlanSource, ProofSurface, Slice, SliceKind,
        UnitKind, PLAN_VERSION,
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
                _ => {} // help subcommand auto-added by clap
            }
        }
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

    // ── Init command tests ─────────────────────────────────────────────

    #[test]
    fn init_subcommand_exists() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(names.contains(&"init"), "CLI missing 'init' subcommand");
    }

    #[test]
    fn init_has_repo_and_force_args() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let init_cmd = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "init")
            .expect("init subcommand not found");
        let arg_names: Vec<&str> = init_cmd
            .get_arguments()
            .map(|a| a.get_id().as_str())
            .collect();
        assert!(arg_names.contains(&"repo"), "init missing --repo arg");
        assert!(arg_names.contains(&"force"), "init missing --force arg");
    }

    #[test]
    fn generate_initial_config_detects_repo_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create known files and dirs
        fs::write(root.join("Cargo.toml"), "[package]").unwrap();
        fs::create_dir_all(root.join("src/core")).unwrap();
        fs::create_dir_all(root.join("src/git")).unwrap();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::create_dir_all(root.join(".github")).unwrap();

        let config = generate_initial_config(root);

        assert_eq!(config.version, 1);
        assert!(config.manifest_files.contains(&"Cargo.toml".to_string()));
        assert!(!config.manifest_files.contains(&"package.json".to_string()));

        assert!(config.test_prefixes.contains(&"tests/".to_string()));
        assert!(config.ops_prefixes.contains(&".github/".to_string()));

        // Should detect src/core/ and src/git/ as path families
        let family_names: Vec<&str> = config
            .path_families
            .iter()
            .map(|r| r.family.as_str())
            .collect();
        assert!(family_names.contains(&"core"), "missing core family");
        assert!(family_names.contains(&"git"), "missing git family");

        let prefixes: Vec<&str> = config
            .path_families
            .iter()
            .map(|r| r.prefix.as_str())
            .collect();
        assert!(prefixes.contains(&"src/core/"), "missing src/core/ prefix");
        assert!(prefixes.contains(&"src/git/"), "missing src/git/ prefix");
    }

    #[test]
    fn generate_initial_config_fallback_defaults() {
        // Empty directory: should get fallback defaults for tests and generated
        let dir = tempfile::tempdir().unwrap();
        let config = generate_initial_config(dir.path());

        assert!(
            config.test_prefixes.contains(&"tests/".to_string()),
            "should have tests/ fallback"
        );
        assert!(
            config
                .generated_prefixes
                .contains(&"generated/".to_string()),
            "should have generated/ fallback"
        );
        assert!(config.manifest_files.is_empty());
        assert!(config.lock_files.is_empty());
        assert!(config.path_families.is_empty());
    }

    #[test]
    fn render_config_toml_produces_valid_parseable_toml() {
        let config = StackcutConfig {
            version: 1,
            generated_prefixes: vec!["dist/".to_string(), "generated/".to_string()],
            manifest_files: vec!["Cargo.toml".to_string()],
            lock_files: vec!["Cargo.lock".to_string()],
            test_prefixes: vec!["tests/".to_string()],
            doc_prefixes: vec!["docs/".to_string()],
            ops_prefixes: vec![".github/".to_string()],
            path_families: vec![
                PathFamilyRule {
                    prefix: "src/core/".to_string(),
                    family: "core".to_string(),
                },
                PathFamilyRule {
                    prefix: "src/git/".to_string(),
                    family: "git".to_string(),
                },
            ],
            review_budget: None,
        };

        let toml_str = render_config_toml(&config);

        // Should contain comments
        assert!(toml_str.contains("# stackcut configuration"));
        assert!(toml_str.contains("# review_budget = 15"));

        // Should parse back to a valid StackcutConfig
        let parsed: StackcutConfig = toml::from_str(&toml_str).expect("rendered TOML should parse");
        assert_eq!(parsed.version, config.version);
        assert_eq!(parsed.generated_prefixes, config.generated_prefixes);
        assert_eq!(parsed.manifest_files, config.manifest_files);
        assert_eq!(parsed.lock_files, config.lock_files);
        assert_eq!(parsed.test_prefixes, config.test_prefixes);
        assert_eq!(parsed.doc_prefixes, config.doc_prefixes);
        assert_eq!(parsed.ops_prefixes, config.ops_prefixes);
        assert_eq!(parsed.path_families, config.path_families);
        assert_eq!(parsed.review_budget, config.review_budget);
    }

    #[test]
    fn render_config_toml_empty_arrays() {
        let config = StackcutConfig {
            version: 1,
            generated_prefixes: vec![],
            manifest_files: vec![],
            lock_files: vec![],
            test_prefixes: vec![],
            doc_prefixes: vec![],
            ops_prefixes: vec![],
            path_families: vec![],
            review_budget: None,
        };

        let toml_str = render_config_toml(&config);
        let parsed: StackcutConfig = toml::from_str(&toml_str).expect("rendered TOML should parse");
        assert_eq!(parsed.version, 1);
        assert!(parsed.path_families.is_empty());
    }
}
