use anyhow::{bail, Context, Result};
use std::env;
use std::process::Command;

fn main() -> Result<()> {
    let task = env::args().nth(1).unwrap_or_else(|| "help".to_string());

    match task.as_str() {
        "ci-fast" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
            &["test", "-p", "stackcut-core"],
        ]),
        "ci-full" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
            &["test", "--workspace"],
        ]),
        "smoke" => run_sequence(&[
            &["test", "--workspace"],
        ]),
        "golden" => run_sequence(&[
            &["test", "-p", "stackcut-artifact"],
        ]),
        "mutants" => run_external("cargo-mutants", &["--workspace", "--timeout", "300"]),
        "docs-check" => run_sequence(&[
            &["test", "--doc", "--workspace"],
        ]),
        "release-check" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"],
            &["test", "--workspace"],
        ]),
        _ => {
            eprintln!("available tasks:");
            eprintln!("  ci-fast");
            eprintln!("  ci-full");
            eprintln!("  smoke");
            eprintln!("  golden");
            eprintln!("  mutants");
            eprintln!("  docs-check");
            eprintln!("  release-check");
            Ok(())
        }
    }
}

fn run_sequence(commands: &[&[&str]]) -> Result<()> {
    for args in commands {
        run_cargo(args)?;
    }
    Ok(())
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .status()
        .with_context(|| format!("failed to run cargo {}", args.join(" ")))?;

    if !status.success() {
        bail!("cargo {} failed", args.join(" "));
    }
    Ok(())
}

fn run_external(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to run {} {}", program, args.join(" ")))?;

    if !status.success() {
        bail!("{} {} failed", program, args.join(" "));
    }
    Ok(())
}
