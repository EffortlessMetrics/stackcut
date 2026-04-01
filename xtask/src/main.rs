use anyhow::{bail, Context, Result};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> Result<()> {
    let task = env::args().nth(1).unwrap_or_else(|| "help".to_string());

    match task.as_str() {
        "ci-fast" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            &["test", "-p", "stackcut-core"],
        ]),
        "ci-full" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            &["test", "--workspace"],
        ]),
        "smoke" => run_sequence(&[&["test", "--workspace"]]),
        "golden" => run_sequence(&[&["test", "-p", "stackcut-artifact"]]),
        "mutants" => run_cargo(&["mutants", "--workspace", "--timeout", "300"]),
        "fuzz" => {
            eprintln!("fuzz: no fuzz targets defined yet, skipping");
            Ok(())
        }
        "docs-check" => docs_check(),
        "release-check" => run_sequence(&[
            &["fmt", "--all", "--check"],
            &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
            &["test", "--workspace"],
        ]),
        _ => {
            eprintln!("available tasks:");
            eprintln!("  ci-fast");
            eprintln!("  ci-full");
            eprintln!("  smoke");
            eprintln!("  golden");
            eprintln!("  mutants");
            eprintln!("  fuzz");
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

/// Check that file path references in documentation files actually exist.
fn docs_check() -> Result<()> {
    let doc_files = &[
        "README.md",
        "AGENTS.md",
        "TESTING.md",
        "RELEASE.md",
        "docs/ARCHITECTURE.md",
    ];
    let mut broken = Vec::new();

    for doc_file in doc_files {
        let contents =
            fs::read_to_string(doc_file).with_context(|| format!("failed to read {}", doc_file))?;
        for path_ref in extract_path_references(&contents) {
            if !Path::new(&path_ref).exists() {
                broken.push((doc_file.to_string(), path_ref));
            }
        }
    }

    if broken.is_empty() {
        println!("docs-check: all references valid");
        Ok(())
    } else {
        for (doc, path) in &broken {
            eprintln!("broken reference in {}: {}", doc, path);
        }
        bail!("docs-check found {} broken references", broken.len());
    }
}

/// Extract file path references from markdown content.
///
/// Finds:
/// 1. Backtick-quoted paths that look like file/directory paths
/// 2. Markdown link targets `[text](path)` that look like local file paths
///
/// Filters out:
/// - URLs (http://, https://)
/// - Anchors (#section)
/// - Bare words without path separators or file extensions
/// - Code block contents (fenced with ```)
/// - Inline code that looks like shell commands, code snippets, or config values
fn extract_path_references(contents: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut in_code_block = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        // Track fenced code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Extract backtick-quoted paths
        extract_backtick_paths(line, &mut refs);

        // Extract markdown link targets
        extract_markdown_link_paths(line, &mut refs);
    }

    refs.sort();
    refs.dedup();
    refs
}

/// Extract paths from backtick-quoted text like `path/to/file.rs`.
fn extract_backtick_paths(line: &str, refs: &mut Vec<String>) {
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        rest = &rest[start + 1..];
        if let Some(end) = rest.find('`') {
            let candidate = &rest[..end];
            rest = &rest[end + 1..];
            if looks_like_path(candidate) {
                // Strip trailing `/` for directory references
                let path = candidate.trim_end_matches('/');
                refs.push(path.to_string());
            }
        } else {
            break;
        }
    }
}

/// Extract paths from markdown links like [text](path).
fn extract_markdown_link_paths(line: &str, refs: &mut Vec<String>) {
    let mut rest = line;
    while let Some(bracket_start) = rest.find("](") {
        rest = &rest[bracket_start + 2..];
        if let Some(paren_end) = rest.find(')') {
            let candidate = &rest[..paren_end];
            rest = &rest[paren_end + 1..];
            if looks_like_local_link(candidate) {
                refs.push(candidate.to_string());
            }
        } else {
            break;
        }
    }
}

/// Determine if a backtick-quoted string looks like a file or directory path
/// that should exist in the repository.
fn looks_like_path(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    // Filter out URLs
    if s.starts_with("http://") || s.starts_with("https://") {
        return false;
    }

    // Filter out anchors
    if s.starts_with('#') {
        return false;
    }

    // Filter out things that look like shell commands
    if s.starts_with("cargo ")
        || s.starts_with("git ")
        || s.starts_with("npm ")
        || s.starts_with("--")
    {
        return false;
    }

    // Filter out things with spaces (likely prose or commands, not paths)
    if s.contains(' ') {
        return false;
    }

    // Filter out things that look like code identifiers or config values
    if s.contains('(') || s.contains(')') || s.contains('{') || s.contains('}') {
        return false;
    }

    // Filter out version-like strings (e.g., `v0.1`)
    if s.starts_with('v') && s.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
        return false;
    }

    // Filter out comparison operators and expressions
    if s.contains("==") || s.contains("!=") || s.contains(">=") || s.contains("<=") {
        return false;
    }

    // Filter out runtime output paths (e.g., `.stackcut/plan.json`)
    if s.starts_with(".stackcut/") {
        return false;
    }

    // Filter out wildcard/glob patterns (e.g., `fixtures/cases/*/`)
    if s.contains('*') {
        return false;
    }

    // Must contain a `/` (path separator) or a `.` with a plausible file extension
    let has_slash = s.contains('/');

    if has_slash {
        return true;
    }

    // For bare filenames (no `/`), only treat them as repo paths if they look
    // like top-level repo files. Bare artifact names like `plan.json` or
    // `summary.md` are conceptual references, not repo paths.
    if let Some(dot_pos) = s.rfind('.') {
        let ext = &s[dot_pos + 1..];
        let path_extensions = [
            "md", "toml", "json", "rs", "ts", "js", "yaml", "yml", "txt", "lock", "sh", "py",
        ];
        if !path_extensions.iter().any(|e| ext.eq_ignore_ascii_case(e)) {
            return false;
        }
        // Only consider bare filenames that start with an uppercase letter
        // (like `README.md`, `TESTING.md`, `Cargo.toml`) as repo paths.
        // Lowercase bare names like `plan.json`, `summary.md`, `override.toml`
        // are typically artifact/concept names in docs.
        s.starts_with(|c: char| c.is_ascii_uppercase())
    } else {
        false
    }
}

/// Determine if a markdown link target looks like a local file path (not a URL or anchor).
fn looks_like_local_link(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        return false;
    }
    if s.starts_with('#') {
        return false;
    }
    if s.starts_with("mailto:") {
        return false;
    }
    // Must look like a relative path
    s.contains('/') || s.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_backtick_paths_with_slash() {
        let content = "See `docs/ARCHITECTURE.md` for details.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["docs/ARCHITECTURE.md"]);
    }

    #[test]
    fn extracts_backtick_paths_with_extension() {
        let content = "Edit `Cargo.toml` to configure.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["Cargo.toml"]);
    }

    #[test]
    fn extracts_markdown_link_paths() {
        let content = "See [architecture](docs/ARCHITECTURE.md) for more.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["docs/ARCHITECTURE.md"]);
    }

    #[test]
    fn filters_urls() {
        let content = "Visit [site](https://example.com) and `https://foo.bar`.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn filters_anchors() {
        let content = "See [section](#overview) and `#heading`.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn filters_code_identifiers() {
        let content = "Use `must_link` and `force_members` overrides.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn filters_shell_commands() {
        let content = "Run `cargo run -p xtask -- ci-fast` to check.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn skips_fenced_code_blocks() {
        let content = "Before\n```\nsome/path.rs\n`another/path.md`\n```\nAfter `real/path.md`.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["real/path.md"]);
    }

    #[test]
    fn strips_trailing_slash_from_directories() {
        let content = "Files under `fixtures/cases/`.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["fixtures/cases"]);
    }

    #[test]
    fn deduplicates_references() {
        let content = "See `README.md` and also `README.md` again.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["README.md"]);
    }

    #[test]
    fn filters_version_strings() {
        let content = "This is `v0.1` of the tool.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn extracts_multiple_from_one_line() {
        let content = "See `README.md`, `TESTING.md`, and `RELEASE.md`.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["README.md", "RELEASE.md", "TESTING.md"]);
    }

    #[test]
    fn filters_function_calls() {
        let content = "Call `plan()` and `validate()`.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn filters_runtime_output_paths() {
        let content = "Writes to `.stackcut/plan.json` and `.stackcut/summary.md`.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn filters_lowercase_bare_artifact_names() {
        let content = "Emits `plan.json`, `summary.md`, and `diagnostics.json`.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }

    #[test]
    fn keeps_uppercase_bare_filenames() {
        let content = "See `README.md` and `TESTING.md`.";
        let refs = extract_path_references(content);
        assert_eq!(refs, vec!["README.md", "TESTING.md"]);
    }

    #[test]
    fn filters_glob_patterns() {
        let content = "Iterate `fixtures/cases/*/` for all cases.";
        let refs = extract_path_references(content);
        assert!(refs.is_empty());
    }
}
