//! Integration tests that create temporary Git repos and run the full
//! plan → materialize → validate --exact pipeline.
//!
//! These tests require `git` to be available on PATH.
//!
//! **Validates: Requirements 23.1, 23.2, 23.3, 23.4**

use std::path::Path;
use std::process::Command;
use tempfile::tempdir;

use stackcut_core::{plan, Overrides, PlanSource, StackcutConfig, PLAN_VERSION};
use stackcut_git::{
    collect_edit_units, materialize_patches, validate_exact_recomposition,
    validate_exact_recomposition_with_receipt,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("failed to run git {:?}: {}", args, e));
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

fn init_repo(dir: &Path) {
    git(dir, &["init", "-b", "main"]);
    git(dir, &["config", "user.email", "test@test.com"]);
    git(dir, &["config", "user.name", "Test"]);
    git(dir, &["config", "commit.gpgsign", "false"]);
}

fn write_file(repo: &Path, relative: &str, content: &str) {
    let full = repo.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&full, content).unwrap();
}

fn commit_all(repo: &Path, message: &str) {
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", message, "--allow-empty"]);
}

/// Run the full pipeline: collect → plan → materialize → validate --exact.
fn run_pipeline(repo: &Path, base: &str, head: &str) {
    let config = StackcutConfig::default();
    let overrides = Overrides::default();

    let (source, units) =
        collect_edit_units(repo, base, head, &config).expect("collect_edit_units failed");

    assert!(!units.is_empty(), "expected at least one edit unit");

    let the_plan = plan(source, units, &config, &overrides);

    // Structural: no errors
    let errors: Vec<_> = the_plan
        .diagnostics
        .iter()
        .filter(|d| d.level == stackcut_core::DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "plan has structural errors: {:?}",
        errors
    );

    // Materialize patches to a temp dir
    let patch_dir = tempdir().expect("failed to create patch temp dir");
    let patches = materialize_patches(repo, &the_plan, patch_dir.path(), false)
        .expect("materialize_patches failed");
    assert!(
        !patches.is_empty(),
        "expected at least one patch to be materialized"
    );

    // Exact recomposition
    validate_exact_recomposition(repo, &the_plan).expect("validate_exact_recomposition failed");
}

// ---------------------------------------------------------------------------
// Scenario: Simple Add
// ---------------------------------------------------------------------------

#[test]
fn integration_simple_add() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit: one file
    write_file(repo, "src/core/existing.rs", "fn existing() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: add a new file
    write_file(repo, "src/core/added.rs", "fn added() {}\n");
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    run_pipeline(repo, &base, &head);
}

// ---------------------------------------------------------------------------
// Scenario: Simple Modify
// ---------------------------------------------------------------------------

#[test]
fn integration_simple_modify() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit
    write_file(repo, "src/core/module.rs", "fn original() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: modify the file
    write_file(
        repo,
        "src/core/module.rs",
        "fn modified() { /* changed */ }\n",
    );
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    run_pipeline(repo, &base, &head);
}

// ---------------------------------------------------------------------------
// Scenario: Rename
// ---------------------------------------------------------------------------

#[test]
fn integration_rename() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit
    write_file(repo, "src/core/old_name.rs", "fn hello() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: rename the file (pure rename, no content change)
    git(
        repo,
        &["mv", "src/core/old_name.rs", "src/core/new_name.rs"],
    );
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    run_pipeline(repo, &base, &head);
}

// ---------------------------------------------------------------------------
// Scenario: Multi-Family Split
// ---------------------------------------------------------------------------

#[test]
fn integration_multi_family_split() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit: files in two families
    write_file(repo, "src/core/engine.rs", "fn engine_v1() {}\n");
    write_file(repo, "src/git/ingest.rs", "fn ingest_v1() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: modify files in both families
    write_file(
        repo,
        "src/core/engine.rs",
        "fn engine_v2() { /* updated */ }\n",
    );
    write_file(
        repo,
        "src/git/ingest.rs",
        "fn ingest_v2() { /* updated */ }\n",
    );
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    // Run pipeline
    let config = StackcutConfig::default();
    let overrides = Overrides::default();

    let (source, units) =
        collect_edit_units(repo, &base, &head, &config).expect("collect_edit_units failed");

    // Should have units from both families
    assert!(
        units.len() >= 2,
        "expected at least 2 edit units, got {}",
        units.len()
    );

    let families: std::collections::BTreeSet<String> =
        units.iter().map(|u| u.family.clone()).collect();
    assert!(
        families.contains("core"),
        "expected 'core' family in units, got {:?}",
        families
    );
    assert!(
        families.contains("git"),
        "expected 'git' family in units, got {:?}",
        families
    );

    let the_plan = plan(source, units, &config, &overrides);

    // Should produce multiple behavior slices (one per family)
    let behavior_slices: Vec<_> = the_plan
        .slices
        .iter()
        .filter(|s| s.kind == stackcut_core::SliceKind::Behavior)
        .collect();
    assert!(
        behavior_slices.len() >= 2,
        "expected at least 2 behavior slices for multi-family split, got {}",
        behavior_slices.len()
    );

    // No structural errors
    let errors: Vec<_> = the_plan
        .diagnostics
        .iter()
        .filter(|d| d.level == stackcut_core::DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "plan has structural errors: {:?}",
        errors
    );

    // Materialize and validate exact recomposition
    let patch_dir = tempdir().expect("failed to create patch temp dir");
    let patches = materialize_patches(repo, &the_plan, patch_dir.path(), false)
        .expect("materialize_patches failed");
    assert!(!patches.is_empty());

    validate_exact_recomposition(repo, &the_plan).expect("validate_exact_recomposition failed");
}

// ---------------------------------------------------------------------------
// Scenario: Simple Delete
// ---------------------------------------------------------------------------

#[test]
fn integration_simple_delete() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit: one file
    write_file(repo, "src/core/to_delete.rs", "fn to_delete() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: delete the file
    git(repo, &["rm", "src/core/to_delete.rs"]);
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    let config = StackcutConfig::default();
    let overrides = stackcut_core::Overrides::default();

    let (source, units) =
        collect_edit_units(repo, &base, &head, &config).expect("collect_edit_units failed");

    // Verify the deleted file produces a Deleted edit unit
    assert!(!units.is_empty(), "expected at least one edit unit");
    let deleted_unit = units
        .iter()
        .find(|u| u.path == "src/core/to_delete.rs")
        .expect("expected a unit for the deleted file");
    assert_eq!(
        deleted_unit.status,
        stackcut_core::ChangeStatus::Deleted,
        "expected Deleted status for removed file"
    );

    // Run the full pipeline: plan → materialize → validate
    let the_plan = plan(source, units, &config, &overrides);
    let errors: Vec<_> = the_plan
        .diagnostics
        .iter()
        .filter(|d| d.level == stackcut_core::DiagnosticLevel::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "plan has structural errors: {:?}",
        errors
    );

    let patch_dir = tempdir().expect("failed to create patch temp dir");
    let patches = materialize_patches(repo, &the_plan, patch_dir.path(), false)
        .expect("materialize_patches failed");
    assert!(!patches.is_empty(), "expected at least one patch");

    validate_exact_recomposition(repo, &the_plan).expect("validate_exact_recomposition failed");
}

// ---------------------------------------------------------------------------
// Scenario: Materialize dry-run
// ---------------------------------------------------------------------------

#[test]
fn integration_materialize_dry_run() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit: one file
    write_file(repo, "src/core/original.rs", "fn original() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: modify the file
    write_file(
        repo,
        "src/core/original.rs",
        "fn original() { /* updated */ }\n",
    );
    commit_all(repo, "head");
    let head = git(repo, &["rev-parse", "HEAD"]);

    let config = StackcutConfig::default();
    let overrides = stackcut_core::Overrides::default();

    let (source, units) =
        collect_edit_units(repo, &base, &head, &config).expect("collect_edit_units failed");
    assert!(!units.is_empty(), "expected at least one edit unit");

    let the_plan = plan(source, units, &config, &overrides);

    // The dry-run path calls `git apply --check` against the working tree. For
    // that check to succeed, the working tree must be at the BASE state so the
    // patch (base→head) can be applied cleanly. Checkout the base commit first.
    git(repo, &["checkout", &base]);

    // Use a real output directory — in dry-run mode materialize_patches should
    // NOT write anything to it (patches go to an internal tempdir).
    let dry_run_out_dir = tempdir().expect("failed to create dry-run output dir");

    let paths = materialize_patches(repo, &the_plan, dry_run_out_dir.path(), true)
        .expect("materialize_patches dry-run failed");

    // Must return at least one path
    assert!(
        !paths.is_empty(),
        "expected at least one path returned from dry-run"
    );

    // The output directory must be empty — dry-run writes to an internal tempdir
    let entries: Vec<_> = std::fs::read_dir(dry_run_out_dir.path()).unwrap().collect();
    assert!(
        entries.is_empty(),
        "dry-run must not write files to the provided output directory"
    );
}

// ---------------------------------------------------------------------------
// Scenario: Type-change (file → symlink) produces Unknown status with note
// ---------------------------------------------------------------------------

// Unix-only: relies on `std::os::unix::fs::symlink` and a `/dev/null` symlink
// target to produce a Git type-change ('T'). Skipped on Windows.
#[cfg(unix)]
#[test]
fn integration_type_change_produces_unknown_status() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    // Base commit: a regular file
    write_file(repo, "src/core/link_target.rs", "fn placeholder() {}\n");
    write_file(repo, "src/core/changed.rs", "fn original() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    // Head commit: replace the regular file with a symlink (type change 'T')
    let changed_path = repo.join("src/core/changed.rs");
    std::fs::remove_file(&changed_path).unwrap();
    std::os::unix::fs::symlink("/dev/null", &changed_path).unwrap();
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-m", "head"]);
    let head = git(repo, &["rev-parse", "HEAD"]);

    let config = StackcutConfig::default();

    let (_, units) =
        collect_edit_units(repo, &base, &head, &config).expect("collect_edit_units failed");

    // The type-changed file must produce an EditUnit with Unknown status and
    // the "unsupported-symlink" note.
    let type_changed = units
        .iter()
        .find(|u| u.path == "src/core/changed.rs")
        .expect("expected an edit unit for the type-changed file");

    assert_eq!(
        type_changed.status,
        stackcut_core::ChangeStatus::Unknown,
        "type-change must produce Unknown status"
    );
    assert!(
        type_changed
            .notes
            .iter()
            .any(|n| n == "unsupported-symlink"),
        "type-change must carry 'unsupported-symlink' note, got: {:?}",
        type_changed.notes
    );
}

// ---------------------------------------------------------------------------
// Scenario: validate_exact_recomposition rejects plan with missing head_tree
// ---------------------------------------------------------------------------

#[test]
fn integration_validate_exact_recomposition_missing_head_tree() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    write_file(repo, "src/core/file.rs", "fn foo() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);
    let head = base.clone(); // same commit, doesn't matter — we bail before touching git

    // Construct a plan with head_tree deliberately set to None
    let source = PlanSource {
        repo_root: None,
        base: base.clone(),
        head: head.clone(),
        head_tree: None, // missing!
    };
    let minimal_plan = stackcut_core::Plan {
        version: PLAN_VERSION.to_string(),
        source,
        units: vec![],
        slices: vec![],
        ambiguities: vec![],
        diagnostics: vec![],
        fingerprint: None,
        override_fingerprint: None,
    };

    // validate_exact_recomposition must fail because head_tree is None
    let result = validate_exact_recomposition(repo, &minimal_plan);
    assert!(
        result.is_err(),
        "expected error for plan with missing head_tree"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("head_tree"),
        "error should mention head_tree, got: {msg}"
    );
}

#[test]
fn integration_validate_exact_recomposition_with_receipt_missing_head_tree() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path();
    init_repo(repo);

    write_file(repo, "src/core/file.rs", "fn foo() {}\n");
    commit_all(repo, "base");
    let base = git(repo, &["rev-parse", "HEAD"]);

    let source = PlanSource {
        repo_root: None,
        base: base.clone(),
        head: base.clone(),
        head_tree: None,
    };
    let minimal_plan = stackcut_core::Plan {
        version: PLAN_VERSION.to_string(),
        source,
        units: vec![],
        slices: vec![],
        ambiguities: vec![],
        diagnostics: vec![],
        fingerprint: None,
        override_fingerprint: None,
    };

    let result = validate_exact_recomposition_with_receipt(repo, &minimal_plan);
    assert!(
        result.is_err(),
        "expected error for plan with missing head_tree"
    );
    let msg = format!("{}", result.err().unwrap());
    assert!(
        msg.contains("head_tree"),
        "error should mention head_tree, got: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Property 26: End-to-End Recomposition (proptest)
// ---------------------------------------------------------------------------

mod prop_end_to_end {
    use super::*;
    use proptest::prelude::*;

    /// A single file operation to apply between base and head commits.
    #[derive(Debug, Clone)]
    enum FileOp {
        /// Add a new file with the given relative path and content.
        Add { path: String, content: String },
        /// Modify an existing file (identified by index into base files).
        Modify { base_index: usize, content: String },
        /// Rename an existing file (identified by index into base files).
        Rename { base_index: usize, new_name: String },
    }

    /// Generate a safe file name component (lowercase alpha, 3-8 chars).
    fn arb_name_component() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-z]{3,8}").unwrap()
    }

    /// Generate random file content (1-4 lines of alphanumeric text).
    fn arb_content() -> impl Strategy<Value = String> {
        prop::collection::vec(
            proptest::string::string_regex("[a-zA-Z0-9_ ]{5,40}").unwrap(),
            1..=4,
        )
        .prop_map(|lines| lines.join("\n") + "\n")
    }

    /// Generate a family name from a small set to encourage multi-family splits.
    fn arb_family() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("core".to_string()),
            Just("git".to_string()),
            Just("artifact".to_string()),
        ]
    }

    /// Generate a single file operation.
    /// `base_file_count` is the number of files in the base commit (for Modify/Rename indices).
    fn arb_file_op(base_file_count: usize) -> impl Strategy<Value = FileOp> {
        let add = (arb_family(), arb_name_component(), arb_content()).prop_map(
            |(family, name, content)| FileOp::Add {
                path: format!("src/{}/{}.rs", family, name),
                content,
            },
        );

        if base_file_count == 0 {
            // No base files to modify or rename — only adds are possible
            return add.boxed();
        }

        let modify =
            (0..base_file_count, arb_content()).prop_map(|(idx, content)| FileOp::Modify {
                base_index: idx,
                content,
            });

        let rename =
            (0..base_file_count, arb_name_component()).prop_map(|(idx, new_name)| FileOp::Rename {
                base_index: idx,
                new_name,
            });

        prop_oneof![
            3 => add,
            2 => modify,
            1 => rename,
        ]
        .boxed()
    }

    /// Generate a vec of 1-5 file operations.
    fn arb_ops() -> impl Strategy<Value = Vec<FileOp>> {
        // We always start with 2-4 base files, then generate 1-5 operations.
        // Base file count is fixed at 3 so Modify/Rename indices are valid.
        prop::collection::vec(arb_file_op(3), 1..=5)
    }

    /// Apply the generated operations to a repo, returning (base_sha, head_sha).
    /// Creates 3 base files in different families, then applies the ops.
    fn setup_repo(repo: &Path, ops: &[FileOp]) -> (String, String) {
        init_repo(repo);

        // Base commit: 3 files across different families
        let base_files = [
            ("src/core/base_alpha.rs", "fn alpha() {}\n"),
            ("src/git/base_beta.rs", "fn beta() {}\n"),
            ("src/artifact/base_gamma.rs", "fn gamma() {}\n"),
        ];
        for (path, content) in &base_files {
            write_file(repo, path, content);
        }
        commit_all(repo, "base");
        let base = git(repo, &["rev-parse", "HEAD"]);

        // Track which base files are still available (not yet renamed)
        let mut current_paths: Vec<String> =
            base_files.iter().map(|(p, _)| p.to_string()).collect();
        let mut renamed: Vec<bool> = vec![false; base_files.len()];
        let mut modified: Vec<bool> = vec![false; base_files.len()];
        let mut added_paths: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();

        for op in ops {
            match op {
                FileOp::Add { path, content } => {
                    // Skip if path collides with an existing file
                    if !added_paths.contains(path) && !current_paths.contains(path) {
                        write_file(repo, path, content);
                        added_paths.insert(path.clone());
                    }
                }
                FileOp::Modify {
                    base_index,
                    content,
                } => {
                    let idx = *base_index;
                    if idx < current_paths.len() && !renamed[idx] {
                        write_file(repo, &current_paths[idx], content);
                        modified[idx] = true;
                    }
                }
                FileOp::Rename {
                    base_index,
                    new_name,
                } => {
                    let idx = *base_index;
                    if idx < current_paths.len() && !renamed[idx] {
                        let old = &current_paths[idx];
                        // Derive new path: same directory, new file name
                        let parent = Path::new(old).parent().unwrap().to_str().unwrap();
                        let new_path = format!("{}/{}.rs", parent, new_name);
                        if new_path != *old
                            && !added_paths.contains(&new_path)
                            && !current_paths.contains(&new_path)
                        {
                            git(repo, &["mv", old, &new_path]);
                            current_paths[idx] = new_path;
                            renamed[idx] = true;
                        }
                    }
                }
            }
        }

        // Only commit if there are actual changes
        let status = git(repo, &["status", "--porcelain"]);
        if status.is_empty() {
            // Force at least one change so the pipeline has something to work with
            write_file(repo, "src/core/fallback.rs", "fn fallback() {}\n");
        }
        commit_all(repo, "head");
        let head = git(repo, &["rev-parse", "HEAD"]);

        (base, head)
    }

    // Feature: stackcut-v01-completion, Property 26: End-to-End Recomposition
    //
    // For any temporary Git repository with a known base and head commit
    // containing file additions, modifications, renames, and multi-family
    // splits, running the full plan → materialize → validate --exact pipeline
    // shall succeed — the applied patch series produces a tree identical to
    // the head tree.
    //
    // **Validates: Requirements 23.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(20))]

        #[test]
        fn prop_end_to_end_recomposition(ops in arb_ops()) {
            let tmp = tempdir().unwrap();
            let repo = tmp.path();
            let (base, head) = setup_repo(repo, &ops);
            run_pipeline(repo, &base, &head);
        }
    }
}
