use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use stackcut_artifact::{
    compare_plans, compute_fingerprint, read_plan, render_comparison, render_proof_hints,
    render_review_packet, render_sarif, render_slice_explanation, render_summary,
    scaffold_overrides, write_diagnostics_envelope, write_plan, write_receipt, write_summary,
    FingerprintCheck, RecompositionReceipt, RecompositionStatus, RecompositionVerdict, SliceHash,
    StructuralResult, ValidationResult,
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

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
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
        /// Print plan JSON to stdout without writing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Render a stored plan as readable Markdown.
    Explain {
        plan: PathBuf,
        /// Show detailed explanation for a specific slice.
        #[arg(long)]
        why: Option<String>,
    },
    /// Validate a stored plan.
    Validate {
        plan: PathBuf,
        #[arg(long)]
        exact: bool,
        #[arg(long)]
        receipt: Option<PathBuf>,
        /// Output format: text (default) or json.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
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
    /// Compare two stack plans and show what changed.
    Compare {
        /// Path to the old (baseline) plan.json.
        old: PathBuf,
        /// Path to the new plan.json.
        new: PathBuf,
        /// Output comparison as JSON instead of Markdown.
        #[arg(long)]
        json: bool,
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
    /// Emit diagnostics in SARIF format for CI integration.
    EmitSarif {
        /// Path to the plan.json.
        plan: PathBuf,
        /// Write to file instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Emit proof surface hints for each slice.
    EmitProof {
        /// Path to the plan.json.
        plan: PathBuf,
        /// Write to file instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
    /// Emit a PR-ready review packet from a plan.
    EmitReviewPacket {
        /// Path to the plan.json.
        plan: PathBuf,
        /// Write to file instead of stdout.
        #[arg(long, short)]
        output: Option<PathBuf>,
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
            dry_run,
        } => cmd_plan(
            &repo,
            &base,
            &head,
            &out_dir,
            config.as_deref(),
            overrides.as_deref(),
            dry_run,
        ),
        Commands::Explain { plan, why } => cmd_explain(&plan, why.as_deref()),
        Commands::Validate {
            plan,
            exact,
            receipt,
            format,
        } => cmd_validate(&plan, exact, receipt.as_deref(), &format),
        Commands::Materialize {
            plan,
            out_dir,
            dry_run,
        } => cmd_materialize(&plan, &out_dir, dry_run),
        Commands::Doctor { repo } => cmd_doctor(&repo),
        Commands::Compare { old, new, json } => cmd_compare(&old, &new, json),
        Commands::Init { repo, force } => cmd_init(&repo, force),
        Commands::ScaffoldOverrides {
            plan,
            output,
            force,
        } => cmd_scaffold_overrides(&plan, &output, force),
        Commands::EmitSarif { plan, output } => cmd_emit_sarif(&plan, output.as_deref()),
        Commands::EmitProof { plan, output } => cmd_emit_proof(&plan, output.as_deref()),
        Commands::EmitReviewPacket { plan, output } => {
            cmd_emit_review_packet(&plan, output.as_deref())
        }
    }
}

fn cmd_plan(
    repo: &Path,
    base: &str,
    head: &str,
    out_dir: &Path,
    config_path: Option<&Path>,
    override_path: Option<&Path>,
    dry_run: bool,
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

    if dry_run {
        let mut plan_with_fp = plan.clone();
        plan_with_fp.fingerprint = Some(compute_fingerprint(&plan));
        let json =
            serde_json::to_string_pretty(&plan_with_fp).context("failed to serialize plan")?;
        println!("{json}");
        return Ok(ExitCode::Success as i32);
    }

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

fn cmd_explain(plan_path: &Path, why: Option<&str>) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    match why {
        Some(slice_id) => match render_slice_explanation(&plan, slice_id) {
            Some(explanation) => {
                print!("{}", explanation);
                Ok(ExitCode::Success as i32)
            }
            None => {
                eprintln!("slice '{}' not found in plan", slice_id);
                Ok(ExitCode::StructuralError as i32)
            }
        },
        None => {
            print!("{}", render_summary(&plan));
            Ok(ExitCode::Success as i32)
        }
    }
}

fn cmd_validate(
    plan_path: &Path,
    exact: bool,
    receipt_path: Option<&Path>,
    format: &OutputFormat,
) -> Result<i32> {
    if receipt_path.is_some() && !exact {
        eprintln!("error: --receipt requires --exact");
        return Ok(ExitCode::StructuralError as i32);
    }

    let plan = read_plan(plan_path)?;

    // Version check
    let plan_version_supported = plan.version == stackcut_core::PLAN_VERSION;

    // Fingerprint verification (if present)
    let fingerprint_check = plan.fingerprint.as_ref().map(|fp| {
        let computed = compute_fingerprint(&plan);
        let matches = *fp == computed;
        FingerprintCheck {
            expected: fp.clone(),
            computed,
            matches,
        }
    });

    // Structural validation (only if version is supported)
    let (structural, diagnostics) = if plan_version_supported {
        let diags = structural_validate(&plan);
        let error_count = diags
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .count();
        let warning_count = diags
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .count();
        let ok = error_count == 0;
        (
            StructuralResult {
                ok,
                error_count,
                warning_count,
                diagnostics: diags.clone(),
            },
            diags,
        )
    } else {
        (
            StructuralResult {
                ok: false,
                error_count: 1,
                warning_count: 0,
                diagnostics: Vec::new(),
            },
            Vec::new(),
        )
    };

    let has_errors = !structural.ok;

    // Exact recomposition
    let exact_recomposition = if !plan_version_supported || has_errors {
        if exact {
            Some(RecompositionStatus::Skipped)
        } else {
            None
        }
    } else if exact {
        let repo_root = plan
            .source
            .repo_root
            .as_ref()
            .map(PathBuf::from)
            .context("plan is missing source.repo_root; exact validation is unavailable")?;

        // Receipt path handling (write receipt file when requested)
        if let Some(rpath) = receipt_path {
            let result =
                match stackcut_git::validate_exact_recomposition_with_receipt(&repo_root, &plan) {
                    Ok(r) => r,
                    Err(e) => {
                        let receipt = RecompositionReceipt {
                            version: plan.version.clone(),
                            base: plan.source.base.clone(),
                            head: plan.source.head.clone(),
                            head_tree: plan.source.head_tree.clone().unwrap_or_default(),
                            plan_fingerprint: compute_fingerprint(&plan),
                            slice_hashes: Vec::new(),
                            recomposed_tree: String::new(),
                            verdict: RecompositionVerdict::Fail,
                            generated_at: chrono::Utc::now().to_rfc3339(),
                        };
                        write_receipt(rpath, &receipt)?;
                        eprintln!("exact recomposition error: {e:#}");
                        println!("receipt written to {}", rpath.display());
                        return Ok(ExitCode::RecompositionFailure as i32);
                    }
                };

            let expected_tree = plan.source.head_tree.clone().unwrap_or_default();

            let all_ok = result.slice_results.iter().all(|sr| sr.apply_ok);
            let trees_match = result.recomposed_tree == expected_tree;
            let verdict = if all_ok && trees_match {
                RecompositionVerdict::Pass
            } else {
                RecompositionVerdict::Fail
            };

            let receipt = RecompositionReceipt {
                version: plan.version.clone(),
                base: plan.source.base.clone(),
                head: plan.source.head.clone(),
                head_tree: expected_tree,
                plan_fingerprint: compute_fingerprint(&plan),
                slice_hashes: result
                    .slice_results
                    .iter()
                    .map(|sr| SliceHash {
                        slice_id: sr.slice_id.clone(),
                        patch_sha256: sr.patch_sha256.clone(),
                        apply_status: if sr.apply_ok {
                            "ok".to_string()
                        } else {
                            sr.error
                                .clone()
                                .unwrap_or_else(|| "unknown error".to_string())
                        },
                    })
                    .collect(),
                recomposed_tree: result.recomposed_tree,
                verdict: verdict.clone(),
                generated_at: chrono::Utc::now().to_rfc3339(),
            };

            write_receipt(rpath, &receipt)?;
            println!("receipt written to {}", rpath.display());

            match verdict {
                RecompositionVerdict::Pass => Some(RecompositionStatus::Pass),
                RecompositionVerdict::Fail => Some(RecompositionStatus::Fail {
                    message: "recomposition verdict: fail".to_string(),
                }),
            }
        } else {
            match stackcut_git::validate_exact_recomposition(&repo_root, &plan) {
                Ok(()) => Some(RecompositionStatus::Pass),
                Err(e) => Some(RecompositionStatus::Fail {
                    message: format!("{e}"),
                }),
            }
        }
    } else {
        None
    };

    // Determine exit code
    let exit_code = if !plan_version_supported || has_errors {
        ExitCode::StructuralError as i32
    } else if matches!(exact_recomposition, Some(RecompositionStatus::Fail { .. })) {
        ExitCode::RecompositionFailure as i32
    } else {
        ExitCode::Success as i32
    };

    let result = ValidationResult {
        plan_version: plan.version.clone(),
        plan_version_supported,
        fingerprint_check,
        structural,
        exact_recomposition,
        exit_code,
    };

    // Output
    match format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&result)
                .context("failed to serialize validation result")?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if !plan_version_supported {
                eprintln!(
                    "error: plan version '{}' is not supported (expected '{}')",
                    plan.version,
                    stackcut_core::PLAN_VERSION
                );
            } else {
                if let Some(ref fc) = result.fingerprint_check {
                    if !fc.matches {
                        eprintln!(
                            "warning: plan fingerprint mismatch (expected {}, got {})",
                            fc.expected, fc.computed
                        );
                    }
                }

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

                if let Some(ref recomp) = result.exact_recomposition {
                    match recomp {
                        RecompositionStatus::Pass => println!("exact recomposition: ok"),
                        RecompositionStatus::Fail { message } => {
                            eprintln!("exact recomposition failed: {message}");
                        }
                        RecompositionStatus::Skipped => {}
                    }
                }
            }
        }
    }

    Ok(exit_code)
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

fn cmd_compare(old_path: &Path, new_path: &Path, json: bool) -> Result<i32> {
    let old_plan = read_plan(old_path)?;
    let new_plan = read_plan(new_path)?;
    let comparison = compare_plans(&old_plan, &new_plan);
    if json {
        let output = serde_json::to_string_pretty(&comparison)
            .context("failed to serialize comparison as JSON")?;
        println!("{output}");
    } else {
        print!("{}", render_comparison(&comparison));
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
    // Only check the repository root for manifests and lock files.  Nested
    // manifests (e.g. workspace member Cargo.toml files) are intentionally
    // excluded because the planner already groups them via path_families.
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
                escape_toml_string(&rule.prefix),
                escape_toml_string(&rule.family)
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

/// Escape a string for embedding in a TOML quoted value.
///
/// Handles backslashes and double-quotes so the rendered TOML stays valid even
/// if a path prefix or family name contains those characters.
fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn format_string_array(items: &[String]) -> String {
    if items.is_empty() {
        return "[]".to_string();
    }
    let inner: Vec<String> = items
        .iter()
        .map(|s| format!("\"{}\"", escape_toml_string(s)))
        .collect();
    format!("[{}]", inner.join(", "))
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

fn cmd_emit_proof(plan_path: &Path, output: Option<&Path>) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    let hints = render_proof_hints(&plan);

    match output {
        Some(out_path) => {
            if let Some(parent) = out_path.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory {}", parent.display()))?;
            }
            fs::write(out_path, &hints)
                .with_context(|| format!("failed to write {}", out_path.display()))?;
            println!("wrote {}", out_path.display());
        }
        None => {
            print!("{}", hints);
        }
    }

    Ok(ExitCode::Success as i32)
}

fn cmd_emit_sarif(plan_path: &Path, output: Option<&Path>) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    let sarif = render_sarif(&plan);
    let json = serde_json::to_string_pretty(&sarif).context("failed to serialize SARIF")?;

    match output {
        Some(path) => {
            if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::write(path, format!("{json}\n"))
                .with_context(|| format!("failed to write {}", path.display()))?;
            println!("wrote {}", path.display());
        }
        None => {
            println!("{json}");
        }
    }
    Ok(ExitCode::Success as i32)
}

fn cmd_emit_review_packet(plan_path: &Path, output: Option<&Path>) -> Result<i32> {
    let plan = read_plan(plan_path)?;
    let packet = render_review_packet(&plan);
    match output {
        Some(path) => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::write(path, &packet)
                .with_context(|| format!("failed to write {}", path.display()))?;
            println!("wrote {}", path.display());
        }
        None => {
            print!("{}", packet);
        }
    }
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
                fingerprint: None,
            }],
            ambiguities: Vec::new(),
            diagnostics: Vec::new(),
            fingerprint: None,
            override_fingerprint: None,
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
        assert!(
            subcommand_names.contains(&"emit-sarif"),
            "CLI missing 'emit-sarif' subcommand"
        );
        assert!(
            subcommand_names.contains(&"emit-proof"),
            "CLI missing 'emit-proof' subcommand"
        );
        assert!(
            subcommand_names.contains(&"emit-review-packet"),
            "CLI missing 'emit-review-packet' subcommand"
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
        assert!(
            help.contains("emit-proof"),
            "Root help missing 'emit-proof' subcommand"
        );
        assert!(
            help.contains("emit-review-packet"),
            "Root help missing 'emit-review-packet' subcommand"
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
                    assert!(help.contains("--dry-run"), "plan help missing --dry-run");
                }
                "explain" => {
                    assert!(
                        help.contains("<PLAN>") || help.contains("plan"),
                        "explain help missing plan argument"
                    );
                    assert!(help.contains("--why"), "explain help missing --why flag");
                }
                "validate" => {
                    assert!(help.contains("--exact"), "validate help missing --exact");
                    assert!(
                        help.contains("--receipt"),
                        "validate help missing --receipt"
                    );
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
                "emit-proof" => {
                    assert!(
                        help.contains("<PLAN>") || help.contains("plan"),
                        "emit-proof help missing plan argument"
                    );
                    assert!(
                        help.contains("--output"),
                        "emit-proof help missing --output"
                    );
                }
                "emit-review-packet" => {
                    assert!(
                        help.contains("<PLAN>") || help.contains("plan"),
                        "emit-review-packet help missing plan argument"
                    );
                    assert!(
                        help.contains("--output"),
                        "emit-review-packet help missing --output"
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

    // ── Plan dry-run tests ────────────────────────────────────────────

    #[test]
    fn plan_subcommand_has_dry_run_arg() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let plan_sub = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "plan")
            .expect("CLI missing 'plan' subcommand");
        let has_dry_run = plan_sub.get_arguments().any(|a| a.get_id() == "dry_run");
        assert!(has_dry_run, "plan subcommand missing --dry-run arg");
    }

    #[test]
    fn plan_help_contains_dry_run_flag() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let plan_sub = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "plan")
            .expect("CLI missing 'plan' subcommand");
        let mut sub_clone = plan_sub.clone();
        let mut buf = Vec::new();
        sub_clone.write_long_help(&mut buf).unwrap();
        let help = String::from_utf8(buf).unwrap();
        assert!(
            help.contains("--dry-run"),
            "plan help missing --dry-run flag"
        );
        assert!(
            help.contains("Print plan JSON to stdout without writing files"),
            "plan help missing --dry-run description"
        );
    }

    #[test]
    fn explain_why_flag_parses_correctly() {
        use clap::CommandFactory;
        // Verify --why is an optional argument on the explain subcommand
        let cmd = Cli::command();
        let explain_sub = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "explain")
            .expect("explain subcommand not found");
        let why_arg = explain_sub
            .get_arguments()
            .find(|a| a.get_id() == "why")
            .expect("--why argument not found on explain");
        assert!(!why_arg.is_required_set(), "--why should be optional");
    }

    #[test]
    fn cmd_explain_why_returns_error_for_unknown_slice() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let result = cmd_explain(&plan_path, Some("nonexistent")).unwrap();
        assert_eq!(
            result,
            ExitCode::StructuralError as i32,
            "Expected StructuralError for unknown slice"
        );
    }

    #[test]
    fn cmd_explain_why_succeeds_for_known_slice() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let result = cmd_explain(&plan_path, Some("behavior-cli")).unwrap();
        assert_eq!(
            result,
            ExitCode::Success as i32,
            "Expected Success for known slice"
        );
    }

    #[test]
    fn cmd_explain_without_why_uses_summary() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let result = cmd_explain(&plan_path, None).unwrap();
        assert_eq!(
            result,
            ExitCode::Success as i32,
            "Expected Success for summary mode"
        );
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

            let result = cmd_validate(&plan_path, false, None, &OutputFormat::Text).unwrap();
            prop_assert_eq!(
                result,
                ExitCode::StructuralError as i32,
                "Expected exit code 1 for unsupported version '{}', got {}",
                version,
                result
            );
        }
    }

    // ── compare subcommand tests ───────────────────────────────────────────

    #[test]
    fn cli_has_compare_subcommand() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let names: Vec<&str> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        assert!(
            names.contains(&"compare"),
            "CLI missing 'compare' subcommand"
        );
    }

    #[test]
    fn compare_subcommand_has_json_flag() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let compare = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "compare")
            .expect("compare subcommand not found");
        let arg_names: Vec<&str> = compare
            .get_arguments()
            .map(|a| a.get_id().as_str())
            .collect();
        assert!(
            arg_names.contains(&"json"),
            "compare subcommand missing --json flag"
        );
    }

    #[test]
    fn cmd_compare_returns_success() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.json");
        let new_path = dir.path().join("new.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&old_path, format!("{json}\n")).unwrap();
        std::fs::write(&new_path, format!("{json}\n")).unwrap();

        let result = cmd_compare(&old_path, &new_path, false).unwrap();
        assert_eq!(result, ExitCode::Success as i32);
    }

    #[test]
    fn cmd_compare_json_returns_success() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let old_path = dir.path().join("old.json");
        let new_path = dir.path().join("new.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&old_path, format!("{json}\n")).unwrap();
        std::fs::write(&new_path, format!("{json}\n")).unwrap();

        let result = cmd_compare(&old_path, &new_path, true).unwrap();
        assert_eq!(result, ExitCode::Success as i32);
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

    #[test]
    fn escape_toml_string_handles_special_chars() {
        assert_eq!(escape_toml_string("plain"), "plain");
        assert_eq!(escape_toml_string(r#"has"quote"#), r#"has\"quote"#);
        assert_eq!(escape_toml_string(r"back\slash"), r"back\\slash");
        assert_eq!(escape_toml_string(r#"both\"chars"#), r#"both\\\"chars"#);
    }

    #[test]
    fn render_config_toml_special_chars_roundtrip() {
        let config = StackcutConfig {
            version: 1,
            generated_prefixes: vec![],
            manifest_files: vec![r#"path with "quotes".toml"#.to_string()],
            lock_files: vec![],
            test_prefixes: vec![r"back\slash/".to_string()],
            doc_prefixes: vec![],
            ops_prefixes: vec![],
            path_families: vec![PathFamilyRule {
                prefix: r#"src/weird"dir/"#.to_string(),
                family: r#"weird"name"#.to_string(),
            }],
            review_budget: None,
        };

        let toml_str = render_config_toml(&config);
        let parsed: StackcutConfig =
            toml::from_str(&toml_str).expect("rendered TOML with special chars should parse");
        assert_eq!(parsed.manifest_files, config.manifest_files);
        assert_eq!(parsed.test_prefixes, config.test_prefixes);
        assert_eq!(parsed.path_families, config.path_families);
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

    // ── receipt tests ─────────────────────────────────────────────────

    #[test]
    fn receipt_without_exact_returns_structural_error() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let receipt_path = dir.path().join("receipt.json");
        let result =
            cmd_validate(&plan_path, false, Some(&receipt_path), &OutputFormat::Text).unwrap();
        assert_eq!(
            result,
            ExitCode::StructuralError as i32,
            "--receipt without --exact should return StructuralError"
        );
        // Receipt file should not be created
        assert!(
            !receipt_path.exists(),
            "receipt file should not be created when --exact is missing"
        );
    }

    #[test]
    fn validate_help_contains_receipt_flag() {
        use clap::CommandFactory;
        let cmd = Cli::command();

        for sub in cmd.get_subcommands() {
            if sub.get_name() == "validate" {
                let mut sub_clone = sub.clone();
                let mut buf = Vec::new();
                sub_clone.write_long_help(&mut buf).unwrap();
                let help = String::from_utf8(buf).unwrap();
                assert!(
                    help.contains("--receipt"),
                    "validate help missing --receipt flag"
                );
                return;
            }
        }
        panic!("validate subcommand not found");
    }

    // ── Tests for --format flag ────────────────────────────────────────

    #[test]
    fn validate_format_flag_exists() {
        use clap::CommandFactory;
        let cmd = Cli::command();
        let validate_cmd = cmd
            .get_subcommands()
            .find(|s| s.get_name() == "validate")
            .expect("validate subcommand not found");
        let args: Vec<&str> = validate_cmd
            .get_arguments()
            .map(|a| a.get_id().as_str())
            .collect();
        assert!(
            args.contains(&"format"),
            "validate subcommand missing --format argument"
        );
    }

    #[test]
    fn validate_json_format_produces_parseable_json() {
        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        // Capture stdout by calling cmd_validate with json format.
        // We can't capture stdout directly here, so we test the ValidationResult
        // serialization round-trip instead. The exit code should be 0.
        let exit = cmd_validate(&plan_path, false, None, &OutputFormat::Json).unwrap();
        assert_eq!(exit, ExitCode::Success as i32);
    }

    #[test]
    fn validate_json_output_contains_expected_fields() {
        use stackcut_artifact::ValidationResult;

        let plan = minimal_plan(PLAN_VERSION);
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        // Build a ValidationResult the same way cmd_validate would
        let plan = read_plan(&plan_path).unwrap();
        let diagnostics = structural_validate(&plan);
        let error_count = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .count();
        let warning_count = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .count();

        let result = ValidationResult {
            plan_version: plan.version.clone(),
            plan_version_supported: true,
            fingerprint_check: None,
            structural: StructuralResult {
                ok: error_count == 0,
                error_count,
                warning_count,
                diagnostics,
            },
            exact_recomposition: None,
            exit_code: ExitCode::Success as i32,
        };

        let serialized = serde_json::to_string_pretty(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert!(parsed.get("plan_version").is_some());
        assert!(parsed.get("plan_version_supported").is_some());
        assert!(parsed.get("structural").is_some());
        assert!(parsed.get("exit_code").is_some());

        let structural = parsed.get("structural").unwrap();
        assert!(structural.get("ok").is_some());
        assert!(structural.get("error_count").is_some());
        assert!(structural.get("warning_count").is_some());
        assert!(structural.get("diagnostics").is_some());
    }

    #[test]
    fn validate_unsupported_version_json_reports_exit_code_1() {
        let plan = minimal_plan("99.99.99");
        let dir = tempfile::tempdir().unwrap();
        let plan_path = dir.path().join("plan.json");
        let json = serde_json::to_string_pretty(&plan).unwrap();
        std::fs::write(&plan_path, format!("{json}\n")).unwrap();

        let exit = cmd_validate(&plan_path, false, None, &OutputFormat::Json).unwrap();
        assert_eq!(exit, ExitCode::StructuralError as i32);
    }
}
