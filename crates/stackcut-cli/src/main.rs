use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::de::DeserializeOwned;
use stackcut_artifact::{read_plan, render_summary, write_diagnostics, write_plan, write_summary};
use stackcut_core::{plan as build_plan, structural_validate, Overrides, StackcutConfig};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "stackcut", version, about = "Deterministic diff-to-stack compiler")]
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
    Explain {
        plan: PathBuf,
    },
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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Plan {
            base,
            head,
            repo,
            out_dir,
            config,
            overrides,
        } => cmd_plan(&repo, &base, &head, &out_dir, config.as_deref(), overrides.as_deref()),
        Commands::Explain { plan } => cmd_explain(&plan),
        Commands::Validate { plan, exact } => cmd_validate(&plan, exact),
        Commands::Materialize { plan, out_dir } => cmd_materialize(&plan, &out_dir),
    }
}

fn cmd_plan(
    repo: &Path,
    base: &str,
    head: &str,
    out_dir: &Path,
    config_path: Option<&Path>,
    override_path: Option<&Path>,
) -> Result<()> {
    let repo_root = stackcut_git::discover_repo_root(repo)
        .with_context(|| format!("failed to discover git repo from {}", repo.display()))?;

    let default_config = existing_path(repo_root.join("stackcut.toml"));
    let default_overrides = existing_path(repo_root.join(".stackcut/override.toml"));

    let config = load_toml_or_default::<StackcutConfig>(
        config_path.or(default_config.as_deref()),
    )?;
    let overrides = load_toml_or_default::<Overrides>(
        override_path.or(default_overrides.as_deref()),
    )?;

    let (source, units) = stackcut_git::collect_edit_units(&repo_root, base, head, &config)?;
    let plan = build_plan(source, units, &config, &overrides);

    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create {}", out_dir.display()))?;

    let plan_path = out_dir.join("plan.json");
    let summary_path = out_dir.join("summary.md");
    let diagnostics_path = out_dir.join("diagnostics.json");

    write_plan(&plan_path, &plan)?;
    write_summary(&summary_path, &plan)?;
    write_diagnostics(&diagnostics_path, &plan.diagnostics)?;

    println!("wrote {}", plan_path.display());
    println!("wrote {}", summary_path.display());
    println!("wrote {}", diagnostics_path.display());
    Ok(())
}

fn cmd_explain(plan_path: &Path) -> Result<()> {
    let plan = read_plan(plan_path)?;
    print!("{}", render_summary(&plan));
    Ok(())
}

fn cmd_validate(plan_path: &Path, exact: bool) -> Result<()> {
    let plan = read_plan(plan_path)?;
    let diagnostics = structural_validate(&plan);

    if diagnostics.is_empty() {
        println!("structural validation: ok");
    } else {
        println!("structural validation:");
        for diagnostic in &diagnostics {
            println!("- {:?} {}: {}", diagnostic.level, diagnostic.code, diagnostic.message);
        }
    }

    if exact {
        let repo_root = plan
            .source
            .repo_root
            .as_ref()
            .map(PathBuf::from)
            .context("plan is missing source.repo_root; exact validation is unavailable")?;
        stackcut_git::validate_exact_recomposition(&repo_root, &plan)?;
        println!("exact recomposition: ok");
    }

    Ok(())
}

fn cmd_materialize(plan_path: &Path, out_dir: &Path) -> Result<()> {
    let plan = read_plan(plan_path)?;
    let repo_root = plan
        .source
        .repo_root
        .as_ref()
        .map(PathBuf::from)
        .context("plan is missing source.repo_root; cannot materialize patches")?;
    let written = stackcut_git::materialize_patches(&repo_root, &plan, out_dir)?;
    for path in written {
        println!("{}", path.display());
    }
    Ok(())
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
