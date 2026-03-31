use anyhow::{anyhow, bail, Context, Result};
use stackcut_core::{
    classify_path, infer_family, ChangeStatus, EditUnit, Plan, PlanSource, Slice, StackcutConfig,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::tempdir;

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
            'R' => (
                ChangeStatus::Renamed,
                parts.next().map(|value| value.to_string()),
                parts.next().unwrap_or_default().to_string(),
            ),
            'C' => (
                ChangeStatus::Copied,
                parts.next().map(|value| value.to_string()),
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
        units.push(EditUnit {
            id: format!("path:{path}"),
            path,
            old_path,
            status,
            kind,
            family,
            notes: Vec::new(),
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

pub fn materialize_patches(repo: &Path, plan: &Plan, out_dir: &Path) -> Result<Vec<PathBuf>> {
    let repo_root = discover_repo_root(repo)?;
    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create patch directory {}", out_dir.display()))?;

    let mut written = Vec::new();
    for (index, slice) in plan.slices.iter().enumerate() {
        let patch_bytes = patch_bytes_for_slice(&repo_root, plan, slice)?;
        let file_name = format!("{:04}-{}.patch", index + 1, slice.id);
        let out_path = out_dir.join(file_name);
        fs::write(&out_path, patch_bytes)
            .with_context(|| format!("failed to write {}", out_path.display()))?;
        written.push(out_path);
    }

    Ok(written)
}

pub fn validate_exact_recomposition(repo: &Path, plan: &Plan) -> Result<()> {
    let repo_root = discover_repo_root(repo)?;
    let expected_tree = plan
        .source
        .head_tree
        .as_ref()
        .ok_or_else(|| anyhow!("plan is missing source.head_tree; exact validation is unavailable"))?
        .clone();

    let patch_dir = tempdir().context("failed to create temporary patch directory")?;
    let patch_paths = materialize_patches(&repo_root, plan, patch_dir.path())?;

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

    for patch in &patch_paths {
        let patch_value = patch.display().to_string();
        run_git_capture(Some(&clone_repo), &["apply", "--check", &patch_value])
            .with_context(|| format!("patch {} does not apply cleanly", patch.display()))?;
        run_git_capture(Some(&clone_repo), &["apply", "--index", &patch_value])
            .with_context(|| format!("failed to apply patch {}", patch.display()))?;
    }

    let actual_tree = run_git_capture(Some(&clone_repo), &["write-tree"])?;
    if actual_tree != expected_tree {
        bail!(
            "exact recomposition failed: expected tree {} but got {}",
            expected_tree,
            actual_tree
        );
    }

    Ok(())
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
