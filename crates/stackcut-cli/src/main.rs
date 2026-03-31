use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use stackcut_artifact::{
    compute_fingerprint, read_plan, render_summary, write_diagnostics_envelope, write_plan,
    write_summary,
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
}
