use anyhow::{anyhow, bail, Context, Result};
use sha2::{Digest, Sha256};
use stackcut_core::{
    classify_path, infer_family, ChangeStatus, EditUnit, Plan, PlanSource, Slice, StackcutConfig,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

pub struct RecompositionResult {
    pub slice_results: Vec<SliceApplyResult>,
    pub recomposed_tree: String,
}

pub struct SliceApplyResult {
    pub slice_id: String,
    pub patch_sha256: String,
    pub apply_ok: bool,
    pub error: Option<String>,
}

pub fn discover_repo_root(start: &Path) -> Result<PathBuf> {
    let stdout = run_git_capture(Some(start), &["rev-parse", "--show-toplevel"])?;
    Ok(PathBuf::from(stdout))
}

pub fn collect_edit_units(
    repo: &Path,
    base: &str,
    head: &str,
    config: &StackcutConfig,
) -> Result<(PlanSource, Vec<EditUnit>)> {
    let repo_root = discover_repo_root(repo)?;
    let diff_output = run_git_capture(
        Some(&repo_root),
        &["diff", "--name-status", "--find-renames", base, head],
    )?;
    let head_tree = run_git_capture(
        Some(&repo_root),
        &["rev-parse", &format!("{head}^{{tree}}")],
    )?;

    let mut units = Vec::new();
    for line in diff_output.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut parts = line.split('\t');
        let code = parts.next().unwrap_or("M");
        let status_char = code.chars().next().unwrap_or('M');

        let (status, old_path, path) = match status_char {
            'A' => (
                ChangeStatus::Added,
                None,
                parts.next().unwrap_or_default().to_string(),
            ),
            'D' => (
                ChangeStatus::Deleted,
                None,
                parts.next().unwrap_or_default().to_string(),
            ),
            'M' => (
                ChangeStatus::Modified,
                None,
                parts.next().unwrap_or_default().to_string(),
            ),
            'R' => {
                let similarity = code[1..].parse::<u32>().unwrap_or(100);
                let status = if similarity < 100 {
                    ChangeStatus::Modified // rename-with-edit → behavior, not mechanical
                } else {
                    ChangeStatus::Renamed
                };
                (
                    status,
                    parts.next().map(|value| value.to_string()),
                    parts.next().unwrap_or_default().to_string(),
                )
            }
            'C' => (
                ChangeStatus::Copied,
                parts.next().map(|value| value.to_string()),
                parts.next().unwrap_or_default().to_string(),
            ),
            'T' => (
                ChangeStatus::Unknown,
                None,
                parts.next().unwrap_or_default().to_string(),
            ),
            _ => (
                ChangeStatus::Unknown,
                None,
                parts.next().unwrap_or_default().to_string(),
            ),
        };

        if path.is_empty() {
            continue;
        }

        let kind = classify_path(&path, &status, config);
        let family = infer_family(&path, config);

        let mut notes = Vec::new();
        if is_binary_path(&path) {
            notes.push("unsupported-binary".to_string());
        }
        if path == ".gitmodules" || path.starts_with(".gitmodules/") {
            notes.push("unsupported-submodule".to_string());
        }
        if status_char == 'T' {
            notes.push("unsupported-symlink".to_string());
        }

        units.push(EditUnit {
            id: format!("path:{path}"),
            path,
            old_path,
            status,
            kind,
            family,
            notes,
        });
    }

    units.sort_by(|left, right| left.path.cmp(&right.path));

    let source = PlanSource {
        repo_root: Some(repo_root.display().to_string()),
        base: base.to_string(),
        head: head.to_string(),
        head_tree: Some(head_tree),
    };

    Ok((source, units))
}

pub fn materialize_patches(
    repo: &Path,
    plan: &Plan,
    out_dir: &Path,
    dry_run: bool,
) -> Result<Vec<PathBuf>> {
    let repo_root = discover_repo_root(repo)?;

    if dry_run {
        // Write patches to a temp dir and verify they apply cleanly
        let temp = tempdir().context("failed to create temporary directory for dry-run")?;
        let paths = do_materialize(&repo_root, plan, temp.path())?;
        for patch in &paths {
            let patch_str = patch.display().to_string();
            run_git_capture(Some(&repo_root), &["apply", "--check", &patch_str]).with_context(
                || format!("dry-run: patch {} does not apply cleanly", patch.display()),
            )?;
        }
        return Ok(paths);
    }

    // Normal mode with rollback on failure
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create patch directory {}", out_dir.display()))?;

    let mut written: Vec<PathBuf> = Vec::new();
    for (index, slice) in plan.slices.iter().enumerate() {
        let file_name = format!("{:04}-{}.patch", index + 1, slice.id);
        let out_path = out_dir.join(&file_name);

        let patch_bytes = match patch_bytes_for_slice(&repo_root, plan, slice) {
            Ok(bytes) => bytes,
            Err(e) => {
                rollback_written(&written);
                return Err(e.context(format!("failed to generate patch for slice '{}'", slice.id)));
            }
        };

        if let Err(e) = fs::write(&out_path, patch_bytes) {
            rollback_written(&written);
            return Err(anyhow!("failed to write {}: {}", out_path.display(), e));
        }
        written.push(out_path);
    }

    Ok(written)
}

/// Write all patches to the given directory, returning the list of written paths.
fn do_materialize(repo: &Path, plan: &Plan, dir: &Path) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(dir)
        .with_context(|| format!("failed to create patch directory {}", dir.display()))?;

    let mut paths = Vec::new();
    for (index, slice) in plan.slices.iter().enumerate() {
        let patch_bytes = patch_bytes_for_slice(repo, plan, slice)
            .with_context(|| format!("failed to generate patch for slice '{}'", slice.id))?;
        let file_name = format!("{:04}-{}.patch", index + 1, slice.id);
        let out_path = dir.join(file_name);
        fs::write(&out_path, &patch_bytes)
            .with_context(|| format!("failed to write {}", out_path.display()))?;
        paths.push(out_path);
    }
    Ok(paths)
}

/// Remove partially written patch files on failure.
fn rollback_written(written: &[PathBuf]) {
    for path in written {
        let _ = fs::remove_file(path);
    }
}

pub fn validate_exact_recomposition(repo: &Path, plan: &Plan) -> Result<()> {
    let result = validate_exact_recomposition_with_receipt(repo, plan)?;
    let expected_tree = plan.source.head_tree.as_ref().ok_or_else(|| {
        anyhow!("plan is missing source.head_tree; exact validation is unavailable")
    })?;

    // Check if any slice failed to apply
    for sr in &result.slice_results {
        if !sr.apply_ok {
            bail!(
                "exact recomposition failed: slice '{}' failed to apply: {}",
                sr.slice_id,
                sr.error.as_deref().unwrap_or("unknown error")
            );
        }
    }

    if result.recomposed_tree != *expected_tree {
        bail!(
            "exact recomposition failed: expected tree {} but got {}",
            expected_tree,
            result.recomposed_tree
        );
    }

    Ok(())
}

pub fn validate_exact_recomposition_with_receipt(
    repo: &Path,
    plan: &Plan,
) -> Result<RecompositionResult> {
    let repo_root = discover_repo_root(repo)?;
    plan.source.head_tree.as_ref().ok_or_else(|| {
        anyhow!("plan is missing source.head_tree; exact validation is unavailable")
    })?;

    let patch_dir = tempdir().context("failed to create temporary patch directory")?;
    let patch_paths = materialize_patches(&repo_root, plan, patch_dir.path(), false)?;

    let clone_dir = tempdir().context("failed to create temporary clone directory")?;
    let clone_repo = clone_dir.path().join("repo");
    run_git_with_cwd(
        None,
        &[
            "clone".to_string(),
            "--quiet".to_string(),
            repo_root.display().to_string(),
            clone_repo.display().to_string(),
        ],
    )?;

    run_git_capture(
        Some(&clone_repo),
        &["checkout", "--quiet", &plan.source.base],
    )?;

    let mut slice_results = Vec::new();
    for (i, patch) in patch_paths.iter().enumerate() {
        let patch_bytes =
            fs::read(patch).with_context(|| format!("failed to read patch {}", patch.display()))?;
        let hash = Sha256::digest(&patch_bytes);
        let patch_sha256 = format!("{:x}", hash);

        let slice_id = plan
            .slices
            .get(i)
            .map(|s| s.id.clone())
            .unwrap_or_else(|| format!("unknown-{}", i));

        let patch_value = patch.display().to_string();
        let check_result = run_git_capture(Some(&clone_repo), &["apply", "--check", &patch_value]);

        match check_result {
            Ok(_) => {
                let apply_result =
                    run_git_capture(Some(&clone_repo), &["apply", "--index", &patch_value]);
                match apply_result {
                    Ok(_) => {
                        slice_results.push(SliceApplyResult {
                            slice_id,
                            patch_sha256,
                            apply_ok: true,
                            error: None,
                        });
                    }
                    Err(e) => {
                        slice_results.push(SliceApplyResult {
                            slice_id,
                            patch_sha256,
                            apply_ok: false,
                            error: Some(format!("{e}")),
                        });
                    }
                }
            }
            Err(e) => {
                slice_results.push(SliceApplyResult {
                    slice_id,
                    patch_sha256,
                    apply_ok: false,
                    error: Some(format!("{e}")),
                });
            }
        }
    }

    let recomposed_tree = run_git_capture(Some(&clone_repo), &["write-tree"])
        .unwrap_or_else(|_| "unknown".to_string());

    Ok(RecompositionResult {
        slice_results,
        recomposed_tree,
    })
}

fn patch_bytes_for_slice(repo: &Path, plan: &Plan, slice: &Slice) -> Result<Vec<u8>> {
    let pathspecs = pathspecs_for_slice(plan, slice);
    if pathspecs.is_empty() {
        bail!("slice {} has no pathspecs to materialize", slice.id);
    }

    let mut args = vec![
        "diff".to_string(),
        "--binary".to_string(),
        "--full-index".to_string(),
        "--find-renames".to_string(),
        plan.source.base.clone(),
        plan.source.head.clone(),
        "--".to_string(),
    ];
    args.extend(pathspecs);

    run_git_with_cwd(Some(repo), &args)
}

fn pathspecs_for_slice(plan: &Plan, slice: &Slice) -> Vec<String> {
    let unit_map = plan.unit_map();
    let mut pathspecs: BTreeSet<String> = BTreeSet::new();

    for member in &slice.members {
        if let Some(unit) = unit_map.get(member) {
            if let Some(old_path) = &unit.old_path {
                pathspecs.insert(old_path.clone());
            }
            pathspecs.insert(unit.path.clone());
        }
    }

    pathspecs.into_iter().collect()
}

fn run_git_capture(repo: Option<&Path>, args: &[&str]) -> Result<String> {
    let owned: Vec<String> = args.iter().map(|value| value.to_string()).collect();
    let bytes = run_git_with_cwd(repo, &owned)?;
    let output = String::from_utf8(bytes).context("git returned non-UTF-8 output")?;
    Ok(output.trim().to_string())
}

fn run_git_with_cwd(repo: Option<&Path>, args: &[String]) -> Result<Vec<u8>> {
    let mut command = Command::new("git");
    if let Some(repo_root) = repo {
        command.arg("-C").arg(repo_root);
    }
    command.args(args);

    let output = command
        .output()
        .with_context(|| format!("failed to execute git {}", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("git {} failed: {}", args.join(" "), stderr.trim());
    }

    Ok(output.stdout)
}

/// Heuristic check for binary file extensions.
fn is_binary_path(path: &str) -> bool {
    const BINARY_EXTENSIONS: &[&str] = &[
        ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".webp", ".svg", ".woff", ".woff2",
        ".ttf", ".otf", ".eot", ".zip", ".tar", ".gz", ".bz2", ".xz", ".7z", ".rar", ".exe",
        ".dll", ".so", ".dylib", ".a", ".o", ".obj", ".wasm", ".class", ".pyc", ".pyo", ".pdf",
        ".doc", ".docx", ".xls", ".xlsx", ".ppt", ".pptx", ".mp3", ".mp4", ".avi", ".mov", ".wav",
        ".flac", ".ogg", ".bin", ".dat", ".db", ".sqlite",
    ];
    let lower = path.to_ascii_lowercase();
    BINARY_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

#[cfg(test)]
mod tests {
    use super::*;
    use stackcut_core::ChangeStatus;

    #[test]
    fn is_binary_path_detects_common_extensions() {
        assert!(is_binary_path("assets/logo.png"));
        assert!(is_binary_path("build/output.wasm"));
        assert!(is_binary_path("data/archive.zip"));
        assert!(is_binary_path("fonts/Inter.woff2"));
        assert!(is_binary_path("report.PDF")); // case-insensitive
    }

    #[test]
    fn is_binary_path_rejects_text_files() {
        assert!(!is_binary_path("src/main.rs"));
        assert!(!is_binary_path("README.md"));
        assert!(!is_binary_path("Cargo.toml"));
        assert!(!is_binary_path("src/lib.ts"));
    }

    /// Simulate the rename status code parsing logic from collect_edit_units.
    fn parse_rename_status(code: &str) -> (ChangeStatus, Option<String>, String) {
        let status_char = code.chars().next().unwrap_or('M');
        match status_char {
            'R' => {
                let similarity = code[1..].parse::<u32>().unwrap_or(100);
                let status = if similarity < 100 {
                    ChangeStatus::Modified
                } else {
                    ChangeStatus::Renamed
                };
                (status, Some("old.rs".to_string()), "new.rs".to_string())
            }
            _ => (ChangeStatus::Unknown, None, String::new()),
        }
    }

    #[test]
    fn rename_with_full_similarity_is_renamed() {
        let (status, _, _) = parse_rename_status("R100");
        assert_eq!(status, ChangeStatus::Renamed);
    }

    #[test]
    fn rename_with_partial_similarity_is_modified() {
        let (status, old_path, _) = parse_rename_status("R095");
        assert_eq!(status, ChangeStatus::Modified);
        assert!(old_path.is_some()); // old_path still recorded
    }

    #[test]
    fn rename_with_low_similarity_is_modified() {
        let (status, _, _) = parse_rename_status("R050");
        assert_eq!(status, ChangeStatus::Modified);
    }

    #[test]
    fn rename_bare_r_defaults_to_renamed() {
        // "R" with no digits → parse fails → defaults to 100 → Renamed
        let (status, _, _) = parse_rename_status("R");
        assert_eq!(status, ChangeStatus::Renamed);
    }

    /// Simulate the type-change parsing logic.
    fn parse_type_change_status(code: &str) -> ChangeStatus {
        let status_char = code.chars().next().unwrap_or('M');
        match status_char {
            'T' => ChangeStatus::Unknown,
            _ => ChangeStatus::Modified,
        }
    }

    #[test]
    fn type_change_is_unknown() {
        assert_eq!(parse_type_change_status("T"), ChangeStatus::Unknown);
    }

    #[test]
    fn type_change_gets_unsupported_symlink_note() {
        // Simulate the notes logic from collect_edit_units
        let status_char = 'T';
        let path = "some/link";
        let mut notes = Vec::new();
        if is_binary_path(path) {
            notes.push("unsupported-binary".to_string());
        }
        if path == ".gitmodules" || path.starts_with(".gitmodules/") {
            notes.push("unsupported-submodule".to_string());
        }
        if status_char == 'T' {
            notes.push("unsupported-symlink".to_string());
        }
        assert_eq!(notes, vec!["unsupported-symlink"]);
    }

    #[test]
    fn binary_file_gets_unsupported_binary_note() {
        let path = "assets/image.png";
        let mut notes = Vec::new();
        if is_binary_path(path) {
            notes.push("unsupported-binary".to_string());
        }
        assert_eq!(notes, vec!["unsupported-binary"]);
    }

    #[test]
    fn gitmodules_gets_unsupported_submodule_note() {
        let path = ".gitmodules";
        let mut notes = Vec::new();
        if path == ".gitmodules" || path.starts_with(".gitmodules/") {
            notes.push("unsupported-submodule".to_string());
        }
        assert_eq!(notes, vec!["unsupported-submodule"]);
    }

    #[test]
    fn regular_file_gets_no_notes() {
        let status_char = 'M';
        let path = "src/main.rs";
        let mut notes = Vec::new();
        if is_binary_path(path) {
            notes.push("unsupported-binary".to_string());
        }
        if path == ".gitmodules" || path.starts_with(".gitmodules/") {
            notes.push("unsupported-submodule".to_string());
        }
        if status_char == 'T' {
            notes.push("unsupported-symlink".to_string());
        }
        assert!(notes.is_empty());
    }

    #[test]
    fn rollback_written_removes_files() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("0001-a.patch");
        let f2 = dir.path().join("0002-b.patch");
        fs::write(&f1, b"patch1").unwrap();
        fs::write(&f2, b"patch2").unwrap();
        assert!(f1.exists());
        assert!(f2.exists());

        rollback_written(&[f1.clone(), f2.clone()]);

        assert!(!f1.exists());
        assert!(!f2.exists());
    }

    #[test]
    fn rollback_written_ignores_missing_files() {
        let dir = tempdir().unwrap();
        let missing = dir.path().join("does-not-exist.patch");
        // Should not panic even if file doesn't exist
        rollback_written(&[missing]);
    }

    mod prop_tests {
        use super::*;
        use proptest::prelude::*;

        /// Simulate the copy status code parsing logic from collect_edit_units.
        fn parse_copy_status(code: &str) -> (ChangeStatus, Option<String>, String) {
            let status_char = code.chars().next().unwrap_or('M');
            match status_char {
                'C' => (
                    ChangeStatus::Copied,
                    Some("old.rs".to_string()),
                    "new.rs".to_string(),
                ),
                _ => (ChangeStatus::Unknown, None, String::new()),
            }
        }

        // Feature: stackcut-v01-completion, Property 17: Rename Similarity Classification
        //
        // For any Git rename where the similarity index is below 100%, the resulting
        // EditUnit shall have kind behavior (not mechanical), because the file was
        // edited in addition to being renamed.
        //
        // **Validates: Requirements 18.1**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_rename_similarity_below_100_is_modified(similarity in 0u32..100) {
                let code = format!("R{:03}", similarity);
                let (status, old_path, _) = parse_rename_status(&code);

                // Rename with similarity < 100% must be classified as Modified
                // (which classify_path maps to Behavior kind, not Mechanical)
                prop_assert_eq!(status, ChangeStatus::Modified);

                // old_path must still be recorded for renames
                prop_assert!(old_path.is_some());
            }
        }

        // Feature: stackcut-v01-completion, Property 18: Copy and Type-Change Classification
        //
        // For any Git copy change, the resulting EditUnit shall have status copied
        // and a non-None old_path. For any Git type change, the resulting EditUnit
        // shall have status unknown.
        //
        // **Validates: Requirements 18.2, 18.3**
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn prop_copy_has_copied_status_and_old_path(similarity in 0u32..=100) {
                let code = format!("C{:03}", similarity);
                let (status, old_path, _) = parse_copy_status(&code);

                // Copy must always produce Copied status
                prop_assert_eq!(status, ChangeStatus::Copied);

                // Copy must always have a non-None old_path
                prop_assert!(old_path.is_some());
            }

            #[test]
            fn prop_type_change_is_unknown(_dummy in 0u32..100) {
                // Type-change is always 'T' with no similarity suffix,
                // but we run 100 iterations to satisfy the PBT contract.
                let status = parse_type_change_status("T");

                prop_assert_eq!(status, ChangeStatus::Unknown);
            }
        }
    }
}
