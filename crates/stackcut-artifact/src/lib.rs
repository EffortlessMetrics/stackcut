use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use stackcut_core::{Diagnostic, DiagnosticLevel, Plan};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsEnvelope {
    pub source_base: String,
    pub source_head: String,
    pub generated_at: String,
    pub counts: DiagnosticCounts,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticCounts {
    pub errors: usize,
    pub warnings: usize,
    pub notes: usize,
}

pub fn compute_fingerprint(plan: &Plan) -> String {
    let mut plan_copy = plan.clone();
    plan_copy.fingerprint = None;
    let json = serde_json::to_string(&plan_copy).unwrap_or_default();
    let hash = Sha256::digest(json.as_bytes());
    format!("{:x}", hash)
}

pub fn read_plan(path: &Path) -> Result<Plan> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let plan = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(plan)
}

pub fn write_plan(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut plan_with_fp = plan.clone();
    plan_with_fp.fingerprint = Some(compute_fingerprint(plan));
    let json = serde_json::to_string_pretty(&plan_with_fp).context("failed to serialize plan")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn write_diagnostics(path: &Path, diagnostics: &[Diagnostic]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json =
        serde_json::to_string_pretty(diagnostics).context("failed to serialize diagnostics")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn write_diagnostics_envelope(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let envelope = DiagnosticsEnvelope {
        source_base: plan.source.base.clone(),
        source_head: plan.source.head.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        counts: DiagnosticCounts {
            errors: plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Error)
                .count(),
            warnings: plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Warning)
                .count(),
            notes: plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Note)
                .count(),
        },
        diagnostics: plan.diagnostics.clone(),
    };
    let json = serde_json::to_string_pretty(&envelope)
        .context("failed to serialize diagnostics envelope")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn write_summary(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, render_summary(plan))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn render_summary(plan: &Plan) -> String {
    let mut output = String::new();

    output.push_str("# stackcut summary\n\n");
    output.push_str(&format!(
        "- base: `{}`\n- head: `{}`\n- slices: `{}`\n- ambiguities: `{}`\n\n",
        plan.source.base,
        plan.source.head,
        plan.slices.len(),
        plan.ambiguities.len()
    ));

    output.push_str("## slices\n\n");
    for slice in &plan.slices {
        output.push_str(&format!(
            "### `{}` — {} ({:?})\n\n",
            slice.id, slice.title, slice.kind
        ));
        if !slice.families.is_empty() {
            output.push_str(&format!("- families: {}\n", slice.families.join(", ")));
        }
        if !slice.depends_on.is_empty() {
            output.push_str(&format!("- depends on: {}\n", slice.depends_on.join(", ")));
        }
        output.push_str("- members:\n");
        for member in &slice.members {
            output.push_str(&format!("  - `{}`\n", member));
        }
        if !slice.reasons.is_empty() {
            output.push_str("- reasons:\n");
            for reason in &slice.reasons {
                output.push_str(&format!("  - `{}`: {}\n", reason.code, reason.message));
            }
        }
        output.push('\n');
    }

    if !plan.ambiguities.is_empty() {
        output.push_str("## ambiguities\n\n");
        for ambiguity in &plan.ambiguities {
            output.push_str(&format!("### `{}`\n\n", ambiguity.id));
            output.push_str(&format!("{}\n\n", ambiguity.message));
            if !ambiguity.affected_units.is_empty() {
                output.push_str(&format!(
                    "- affected: {}\n",
                    ambiguity.affected_units.join(", ")
                ));
            }
            if !ambiguity.candidate_slices.is_empty() {
                output.push_str(&format!(
                    "- candidates: {}\n",
                    ambiguity.candidate_slices.join(", ")
                ));
            }
            output.push_str(&format!("- resolution: {}\n\n", ambiguity.resolution));
        }
    }

    if !plan.diagnostics.is_empty() {
        output.push_str("## diagnostics\n\n");
        for diagnostic in &plan.diagnostics {
            output.push_str(&format!(
                "- `{:?}` `{}`: {}\n",
                diagnostic.level, diagnostic.code, diagnostic.message
            ));
        }
        output.push('\n');
    }

    output
}

// ── Plan comparison ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanComparison {
    pub old_fingerprint: Option<String>,
    pub new_fingerprint: Option<String>,
    pub slices_added: Vec<String>,
    pub slices_removed: Vec<String>,
    pub slices_unchanged: Vec<String>,
    pub slices_modified: Vec<SliceDiff>,
    pub units_moved: Vec<UnitMove>,
    pub source_changed: bool,
    pub equivalence: Equivalence,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceDiff {
    pub slice_id: String,
    pub members_added: Vec<String>,
    pub members_removed: Vec<String>,
    pub title_changed: bool,
    pub old_title: Option<String>,
    pub new_title: Option<String>,
    pub depends_on_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnitMove {
    pub unit_id: String,
    pub old_slice: String,
    pub new_slice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Equivalence {
    /// Both plans cover exactly the same set of unit IDs.
    UnitEquivalent,
    /// The unit sets differ.
    UnitsDiffer,
}

/// Compare two plans and produce a structured comparison.
pub fn compare_plans(old: &Plan, new: &Plan) -> PlanComparison {
    let old_fp = Some(compute_fingerprint(old));
    let new_fp = Some(compute_fingerprint(new));

    // Index slices by ID for both plans, keeping the first occurrence when
    // duplicates exist (malformed plans). Warn on stderr about any duplicates.
    let mut old_slices: BTreeMap<&str, &stackcut_core::Slice> = BTreeMap::new();
    for s in &old.slices {
        if old_slices.contains_key(s.id.as_str()) {
            eprintln!(
                "warning: duplicate slice ID '{}' in old plan; using first occurrence",
                s.id
            );
        } else {
            old_slices.insert(s.id.as_str(), s);
        }
    }
    let mut new_slices: BTreeMap<&str, &stackcut_core::Slice> = BTreeMap::new();
    for s in &new.slices {
        if new_slices.contains_key(s.id.as_str()) {
            eprintln!(
                "warning: duplicate slice ID '{}' in new plan; using first occurrence",
                s.id
            );
        } else {
            new_slices.insert(s.id.as_str(), s);
        }
    }

    let old_ids: BTreeSet<&str> = old_slices.keys().copied().collect();
    let new_ids: BTreeSet<&str> = new_slices.keys().copied().collect();

    let slices_added: Vec<String> = new_ids
        .difference(&old_ids)
        .map(|s| s.to_string())
        .collect();
    let slices_removed: Vec<String> = old_ids
        .difference(&new_ids)
        .map(|s| s.to_string())
        .collect();

    let common_ids: BTreeSet<&str> = old_ids.intersection(&new_ids).copied().collect();

    let mut slices_unchanged = Vec::new();
    let mut slices_modified = Vec::new();

    for id in &common_ids {
        let old_slice = old_slices[id];
        let new_slice = new_slices[id];

        let old_members: BTreeSet<&str> = old_slice.members.iter().map(|s| s.as_str()).collect();
        let new_members: BTreeSet<&str> = new_slice.members.iter().map(|s| s.as_str()).collect();

        let title_changed = old_slice.title != new_slice.title;
        let members_same = old_members == new_members;
        let depends_on_changed = old_slice.depends_on != new_slice.depends_on;

        if members_same && !title_changed && !depends_on_changed {
            slices_unchanged.push(id.to_string());
        } else {
            let members_added: Vec<String> = new_members
                .difference(&old_members)
                .map(|s| s.to_string())
                .collect();
            let members_removed: Vec<String> = old_members
                .difference(&new_members)
                .map(|s| s.to_string())
                .collect();
            slices_modified.push(SliceDiff {
                slice_id: id.to_string(),
                members_added,
                members_removed,
                title_changed,
                old_title: if title_changed {
                    Some(old_slice.title.clone())
                } else {
                    None
                },
                new_title: if title_changed {
                    Some(new_slice.title.clone())
                } else {
                    None
                },
                depends_on_changed,
            });
        }
    }

    // Build unit → owning-slice maps for both plans.
    let old_unit_slice: BTreeMap<&str, &str> = old
        .slices
        .iter()
        .flat_map(|s| s.members.iter().map(move |m| (m.as_str(), s.id.as_str())))
        .collect();
    let new_unit_slice: BTreeMap<&str, &str> = new
        .slices
        .iter()
        .flat_map(|s| s.members.iter().map(move |m| (m.as_str(), s.id.as_str())))
        .collect();

    let mut units_moved = Vec::new();
    for (unit_id, &old_sid) in &old_unit_slice {
        if let Some(&new_sid) = new_unit_slice.get(unit_id) {
            if old_sid != new_sid {
                units_moved.push(UnitMove {
                    unit_id: unit_id.to_string(),
                    old_slice: old_sid.to_string(),
                    new_slice: new_sid.to_string(),
                });
            }
        }
    }

    // Check unit-set equivalence.
    let old_unit_ids: BTreeSet<&str> = old.units.iter().map(|u| u.id.as_str()).collect();
    let new_unit_ids: BTreeSet<&str> = new.units.iter().map(|u| u.id.as_str()).collect();
    let equivalence = if old_unit_ids == new_unit_ids {
        Equivalence::UnitEquivalent
    } else {
        Equivalence::UnitsDiffer
    };

    let source_changed = old.source != new.source;

    PlanComparison {
        old_fingerprint: old_fp,
        new_fingerprint: new_fp,
        slices_added,
        slices_removed,
        slices_unchanged,
        slices_modified,
        units_moved,
        source_changed,
        equivalence,
    }
}

/// Render a human-readable Markdown summary of a plan comparison.
pub fn render_comparison(comparison: &PlanComparison) -> String {
    let mut out = String::new();

    out.push_str("# stackcut plan comparison\n\n");

    let old_fp = comparison.old_fingerprint.as_deref().unwrap_or("(none)");
    let new_fp = comparison.new_fingerprint.as_deref().unwrap_or("(none)");
    out.push_str(&format!("- fingerprint: `{old_fp}` → `{new_fp}`\n"));

    let equiv_label = match comparison.equivalence {
        Equivalence::UnitEquivalent => "unit-equivalent",
        Equivalence::UnitsDiffer => "units-differ",
    };
    out.push_str(&format!("- equivalence: {equiv_label}\n"));

    if comparison.source_changed {
        out.push_str("- source: changed\n");
    }

    out.push_str(&format!(
        "\n## slices added ({})\n",
        comparison.slices_added.len()
    ));
    for id in &comparison.slices_added {
        out.push_str(&format!("- `{id}`\n"));
    }

    out.push_str(&format!(
        "\n## slices removed ({})\n",
        comparison.slices_removed.len()
    ));
    for id in &comparison.slices_removed {
        out.push_str(&format!("- `{id}`\n"));
    }

    out.push_str(&format!(
        "\n## slices modified ({})\n",
        comparison.slices_modified.len()
    ));
    for diff in &comparison.slices_modified {
        out.push_str(&format!("### `{}`\n", diff.slice_id));
        if diff.title_changed {
            let old_t = diff.old_title.as_deref().unwrap_or("(none)");
            let new_t = diff.new_title.as_deref().unwrap_or("(none)");
            out.push_str(&format!("  - title: `{old_t}` → `{new_t}`\n"));
        }
        if diff.depends_on_changed {
            out.push_str("  - depends_on: changed\n");
        }
        for m in &diff.members_added {
            out.push_str(&format!("  - added: `{m}`\n"));
        }
        for m in &diff.members_removed {
            out.push_str(&format!("  - removed: `{m}`\n"));
        }
    }

    out.push_str(&format!(
        "\n## units moved ({})\n",
        comparison.units_moved.len()
    ));
    for mv in &comparison.units_moved {
        out.push_str(&format!(
            "- `{}`: `{}` → `{}`\n",
            mv.unit_id, mv.old_slice, mv.new_slice
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use stackcut_core::{
        ChangeStatus, Diagnostic, DiagnosticLevel, EditUnit, ForceMemberOverride, InclusionReason,
        MustLinkOverride, MustOrderOverride, Overrides, PathFamilyRule, Plan, PlanSource,
        ProofSurface, RenameSliceOverride, Slice, SliceKind, StackcutConfig, UnitKind,
    };

    #[test]
    fn summary_includes_slice_ids_and_diagnostics() {
        let plan = Plan {
            version: "0.1.0".to_string(),
            source: PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            },
            units: vec![EditUnit {
                id: "path:src/core/planner.rs".to_string(),
                path: "src/core/planner.rs".to_string(),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: stackcut_core::UnitKind::Behavior,
                family: "core".to_string(),
                notes: Vec::new(),
            }],
            slices: vec![Slice {
                id: "behavior-core".to_string(),
                title: "Behavior: core".to_string(),
                kind: SliceKind::Behavior,
                families: vec!["core".to_string()],
                members: vec!["path:src/core/planner.rs".to_string()],
                depends_on: Vec::new(),
                reasons: Vec::new(),
                proof_surface: ProofSurface::default(),
            }],
            ambiguities: Vec::new(),
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Note,
                code: "ok".to_string(),
                message: "summary smoke".to_string(),
            }],
            fingerprint: None,
        };

        let rendered = render_summary(&plan);
        assert!(rendered.contains("behavior-core"));
        assert!(rendered.contains("summary smoke"));
    }

    fn arb_change_status() -> impl Strategy<Value = ChangeStatus> {
        prop_oneof![
            Just(ChangeStatus::Added),
            Just(ChangeStatus::Modified),
            Just(ChangeStatus::Deleted),
            Just(ChangeStatus::Renamed),
            Just(ChangeStatus::Copied),
            Just(ChangeStatus::Unknown),
        ]
    }

    fn arb_unit_kind() -> impl Strategy<Value = UnitKind> {
        prop_oneof![
            Just(UnitKind::Behavior),
            Just(UnitKind::Mechanical),
            Just(UnitKind::Test),
            Just(UnitKind::Documentation),
            Just(UnitKind::Generated),
            Just(UnitKind::Manifest),
            Just(UnitKind::Lockfile),
            Just(UnitKind::OpsConfig),
        ]
    }

    fn arb_slice_kind() -> impl Strategy<Value = SliceKind> {
        prop_oneof![
            Just(SliceKind::Behavior),
            Just(SliceKind::Mechanical),
            Just(SliceKind::PrepRefactor),
            Just(SliceKind::ApiSchema),
            Just(SliceKind::OpsConfig),
            Just(SliceKind::Generated),
            Just(SliceKind::TestsDocs),
            Just(SliceKind::Misc),
        ]
    }

    fn arb_diagnostic_level() -> impl Strategy<Value = DiagnosticLevel> {
        prop_oneof![
            Just(DiagnosticLevel::Error),
            Just(DiagnosticLevel::Warning),
            Just(DiagnosticLevel::Note),
        ]
    }

    fn arb_edit_unit(index: usize) -> impl Strategy<Value = EditUnit> {
        (arb_change_status(), arb_unit_kind(), "[a-z]{1,8}").prop_map(
            move |(status, kind, family)| EditUnit {
                id: format!("path:src/{}/file{}.rs", family, index),
                path: format!("src/{}/file{}.rs", family, index),
                old_path: None,
                status,
                kind,
                family,
                notes: Vec::new(),
            },
        )
    }

    fn arb_slice(index: usize, unit_count: usize) -> impl Strategy<Value = Slice> {
        (arb_slice_kind(), "[a-z]{1,8}").prop_map(move |(kind, family)| {
            let members: Vec<String> = (0..unit_count)
                .map(|j| format!("path:src/{}/file{}.rs", family, j))
                .collect();
            Slice {
                id: format!("slice-{}", index),
                title: format!("Slice {}", index),
                kind,
                families: vec![family],
                members,
                depends_on: Vec::new(),
                reasons: vec![InclusionReason {
                    code: "test".to_string(),
                    message: "test reason".to_string(),
                }],
                proof_surface: ProofSurface::default(),
            }
        })
    }

    fn arb_plan() -> impl Strategy<Value = Plan> {
        let unit_count = 1..=5usize;
        unit_count
            .prop_flat_map(|n| {
                let units = proptest::collection::vec(arb_edit_unit(0), n..=n);
                let slices = proptest::collection::vec(arb_slice(0, 1), 1..=3);
                let diagnostics = proptest::collection::vec(
                    (arb_diagnostic_level(), "[a-z-]{1,12}", ".{1,30}").prop_map(
                        |(level, code, message)| Diagnostic {
                            level,
                            code,
                            message,
                        },
                    ),
                    0..=3,
                );
                let base = "[a-f0-9]{8}";
                let head = "[a-f0-9]{8}";
                let version = Just("0.1.0".to_string());
                (units, slices, diagnostics, base, head, version)
            })
            .prop_map(|(units, slices, diagnostics, base, head, version)| Plan {
                version,
                source: PlanSource {
                    repo_root: None,
                    base,
                    head,
                    head_tree: None,
                },
                units,
                slices,
                ambiguities: Vec::new(),
                diagnostics,
                fingerprint: None,
            })
    }

    // Feature: stackcut-v01-completion, Property 23: Diagnostics Envelope Completeness
    // **Validates: Requirements 25.1, 25.2**
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        #[test]
        fn diagnostics_envelope_completeness(plan in arb_plan()) {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("diagnostics.json");

            // Write the diagnostics envelope
            write_diagnostics_envelope(&path, &plan).unwrap();

            // Read back and parse
            let contents = std::fs::read_to_string(&path).unwrap();
            let envelope: DiagnosticsEnvelope = serde_json::from_str(&contents).unwrap();

            // Assert source fields match
            prop_assert_eq!(&envelope.source_base, &plan.source.base);
            prop_assert_eq!(&envelope.source_head, &plan.source.head);

            // Assert counts match
            let expected_errors = plan.diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Error).count();
            let expected_warnings = plan.diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Warning).count();
            let expected_notes = plan.diagnostics.iter().filter(|d| d.level == DiagnosticLevel::Note).count();
            prop_assert_eq!(envelope.counts.errors, expected_errors);
            prop_assert_eq!(envelope.counts.warnings, expected_warnings);
            prop_assert_eq!(envelope.counts.notes, expected_notes);

            // Assert diagnostics length matches
            prop_assert_eq!(envelope.diagnostics.len(), plan.diagnostics.len());

            // Assert generated_at parses as valid ISO-8601 / RFC-3339
            prop_assert!(
                chrono::DateTime::parse_from_rfc3339(&envelope.generated_at).is_ok(),
                "generated_at '{}' is not a valid RFC-3339 timestamp", envelope.generated_at
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 24: Fingerprint Verification
    // **Validates: Requirements 24.1**
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        #[test]
        fn fingerprint_survives_serialize_deserialize_round_trip(plan in arb_plan()) {
            // 1. Compute fingerprint on the original plan
            let fingerprint = compute_fingerprint(&plan);

            // 2. Store fingerprint in the plan
            let mut plan_with_fp = plan.clone();
            plan_with_fp.fingerprint = Some(fingerprint.clone());

            // 3. Serialize to JSON
            let json = serde_json::to_string_pretty(&plan_with_fp).unwrap();

            // 4. Deserialize back
            let deserialized: Plan = serde_json::from_str(&json).unwrap();

            // 5. Recompute fingerprint on the deserialized plan
            let recomputed = compute_fingerprint(&deserialized);

            // 6. Assert the recomputed fingerprint equals the original
            prop_assert_eq!(fingerprint, recomputed);
        }
    }

    fn arb_must_link_override() -> impl Strategy<Value = MustLinkOverride> {
        (
            proptest::collection::vec("[a-z]{2,6}", 2..=3),
            proptest::option::of("[a-z ]{3,15}"),
        )
            .prop_map(|(members, reason)| MustLinkOverride { members, reason })
    }

    fn arb_force_member_override() -> impl Strategy<Value = ForceMemberOverride> {
        (
            "[a-z]{2,6}",
            "[a-z-]{3,10}",
            proptest::option::of("[a-z ]{3,15}"),
        )
            .prop_map(|(member, slice, reason)| ForceMemberOverride {
                member,
                slice,
                reason,
            })
    }

    fn arb_rename_slice_override() -> impl Strategy<Value = RenameSliceOverride> {
        ("[a-z-]{3,10}", "[A-Za-z ]{3,15}")
            .prop_map(|(id, title)| RenameSliceOverride { id, title })
    }

    fn arb_must_order_override() -> impl Strategy<Value = MustOrderOverride> {
        (
            "[a-z-]{3,10}",
            "[a-z-]{3,10}",
            proptest::option::of("[a-z ]{3,15}"),
        )
            .prop_map(|(before, after, reason)| MustOrderOverride {
                before,
                after,
                reason,
            })
    }

    fn arb_overrides() -> impl Strategy<Value = Overrides> {
        (
            proptest::collection::vec(arb_must_link_override(), 0..=2),
            proptest::collection::vec(arb_force_member_override(), 0..=2),
            proptest::collection::vec(arb_rename_slice_override(), 0..=2),
            proptest::collection::vec(arb_must_order_override(), 0..=2),
        )
            .prop_map(
                |(must_link, force_members, rename_slices, must_order)| Overrides {
                    version: 1,
                    must_link,
                    force_members,
                    rename_slices,
                    must_order,
                },
            )
    }

    fn arb_path_family_rule() -> impl Strategy<Value = PathFamilyRule> {
        ("[a-z]{2,6}/", "[a-z]{2,6}").prop_map(|(prefix, family)| PathFamilyRule { prefix, family })
    }

    fn arb_stackcut_config() -> impl Strategy<Value = StackcutConfig> {
        (
            proptest::collection::vec("[a-z]{2,6}/", 0..=2),
            proptest::collection::vec("[a-z]{2,8}\\.[a-z]{2,4}", 0..=2),
            proptest::collection::vec("[a-z]{2,8}\\.lock", 0..=2),
            proptest::collection::vec("[a-z]{2,6}/", 0..=2),
            proptest::collection::vec("[a-z]{2,6}/", 0..=2),
            proptest::collection::vec("[a-z]{2,6}/", 0..=2),
            proptest::collection::vec(arb_path_family_rule(), 0..=2),
            proptest::option::of(1..=50u32),
        )
            .prop_map(
                |(
                    generated_prefixes,
                    manifest_files,
                    lock_files,
                    test_prefixes,
                    doc_prefixes,
                    ops_prefixes,
                    path_families,
                    review_budget,
                )| {
                    StackcutConfig {
                        version: 1,
                        generated_prefixes,
                        manifest_files,
                        lock_files,
                        test_prefixes,
                        doc_prefixes,
                        ops_prefixes,
                        path_families,
                        review_budget,
                    }
                },
            )
    }

    // ── Fixture helpers for snapshot tests (Task 15.3) ──────────────────

    fn fixture_case_dirs() -> Vec<std::path::PathBuf> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();
        let cases_dir = workspace_root.join("fixtures/cases");
        let mut dirs: Vec<std::path::PathBuf> = std::fs::read_dir(&cases_dir)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", cases_dir.display(), e))
            .filter_map(|entry| {
                let entry = entry.ok()?;
                if entry.file_type().ok()?.is_dir() {
                    Some(entry.path())
                } else {
                    None
                }
            })
            .collect();
        dirs.sort();
        dirs
    }

    fn load_fixture_plan(case_dir: &std::path::Path) -> Plan {
        let input_path = case_dir.join("input.units.json");
        let input_json = std::fs::read_to_string(&input_path)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", input_path, e));
        let units: Vec<EditUnit> = serde_json::from_str(&input_json)
            .unwrap_or_else(|e| panic!("Failed to parse {:?}: {}", input_path, e));

        let source = PlanSource {
            repo_root: None,
            base: "fixture-base".to_string(),
            head: "fixture-head".to_string(),
            head_tree: None,
        };
        stackcut_core::plan(
            source,
            units,
            &StackcutConfig::default(),
            &stackcut_core::Overrides::default(),
        )
    }

    // ── Snapshot tests: render_summary (Task 15.3) ──────────────────────
    // Validates: Requirements 22.1

    #[test]
    fn snapshot_render_summary_all_fixtures() {
        for case_dir in fixture_case_dirs() {
            let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();
            let plan = load_fixture_plan(&case_dir);
            let summary = render_summary(&plan);

            // Summary must be non-empty
            assert!(
                !summary.is_empty(),
                "[{}] render_summary produced empty output",
                case_name
            );

            // Summary must contain base and head
            assert!(
                summary.contains("fixture-base"),
                "[{}] summary missing base ref",
                case_name
            );
            assert!(
                summary.contains("fixture-head"),
                "[{}] summary missing head ref",
                case_name
            );

            // Summary must contain all slice IDs
            for slice in &plan.slices {
                assert!(
                    summary.contains(&slice.id),
                    "[{}] summary missing slice ID '{}'",
                    case_name,
                    slice.id
                );
            }

            // Summary must contain the slice count
            assert!(
                summary.contains(&format!("`{}`", plan.slices.len())),
                "[{}] summary missing slice count",
                case_name
            );

            // Stability: rendering twice produces identical output
            let summary2 = render_summary(&plan);
            assert_eq!(
                summary, summary2,
                "[{}] render_summary is not stable across calls",
                case_name
            );
        }
    }

    // ── Snapshot tests: diagnostics serialization (Task 15.3) ────────────
    // Validates: Requirements 22.2

    #[test]
    fn snapshot_diagnostics_all_fixtures() {
        for case_dir in fixture_case_dirs() {
            let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();
            let plan = load_fixture_plan(&case_dir);

            // Serialize diagnostics as JSON
            let json = serde_json::to_string_pretty(&plan.diagnostics).unwrap_or_else(|e| {
                panic!("[{}] Failed to serialize diagnostics: {}", case_name, e)
            });

            // Must be valid JSON (parse back)
            let parsed: Vec<Diagnostic> = serde_json::from_str(&json).unwrap_or_else(|e| {
                panic!(
                    "[{}] Diagnostics JSON failed to round-trip: {}",
                    case_name, e
                )
            });

            // Round-trip must preserve count
            assert_eq!(
                parsed.len(),
                plan.diagnostics.len(),
                "[{}] diagnostics count mismatch after round-trip",
                case_name
            );

            // Each diagnostic must have valid structure
            for diag in &parsed {
                assert!(
                    !diag.code.is_empty(),
                    "[{}] diagnostic has empty code",
                    case_name
                );
                assert!(
                    !diag.message.is_empty(),
                    "[{}] diagnostic has empty message",
                    case_name
                );
            }

            // Stability: serializing twice produces identical JSON
            let json2 = serde_json::to_string_pretty(&plan.diagnostics).unwrap();
            assert_eq!(
                json, json2,
                "[{}] diagnostics serialization is not stable",
                case_name
            );
        }
    }

    // ── Snapshot tests: diagnostics envelope per fixture (Task 15.3) ─────
    // Validates: Requirements 22.2

    #[test]
    fn snapshot_diagnostics_envelope_all_fixtures() {
        for case_dir in fixture_case_dirs() {
            let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();
            let plan = load_fixture_plan(&case_dir);

            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("diagnostics.json");
            write_diagnostics_envelope(&path, &plan).unwrap();

            let contents = std::fs::read_to_string(&path).unwrap();
            let envelope: DiagnosticsEnvelope =
                serde_json::from_str(&contents).unwrap_or_else(|e| {
                    panic!(
                        "[{}] Failed to parse diagnostics envelope: {}",
                        case_name, e
                    )
                });

            // Source fields must match
            assert_eq!(
                envelope.source_base, "fixture-base",
                "[{}] envelope source_base mismatch",
                case_name
            );
            assert_eq!(
                envelope.source_head, "fixture-head",
                "[{}] envelope source_head mismatch",
                case_name
            );

            // Counts must be consistent
            let expected_errors = plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Error)
                .count();
            let expected_warnings = plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Warning)
                .count();
            let expected_notes = plan
                .diagnostics
                .iter()
                .filter(|d| d.level == DiagnosticLevel::Note)
                .count();
            assert_eq!(
                envelope.counts.errors, expected_errors,
                "[{}] error count mismatch",
                case_name
            );
            assert_eq!(
                envelope.counts.warnings, expected_warnings,
                "[{}] warning count mismatch",
                case_name
            );
            assert_eq!(
                envelope.counts.notes, expected_notes,
                "[{}] note count mismatch",
                case_name
            );

            // generated_at must be valid RFC-3339
            assert!(
                chrono::DateTime::parse_from_rfc3339(&envelope.generated_at).is_ok(),
                "[{}] generated_at is not valid RFC-3339: {}",
                case_name,
                envelope.generated_at
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 1: Plan JSON Round-Trip
    // **Validates: Requirements 26.1**
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        #[test]
        fn plan_json_round_trip(plan in arb_plan()) {
            let json = serde_json::to_string_pretty(&plan).unwrap();
            let deserialized: Plan = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(plan, deserialized);
        }
    }

    // Feature: stackcut-v01-completion, Property 2: Overrides TOML Round-Trip
    // **Validates: Requirements 6.5, 26.2**
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        #[test]
        fn overrides_toml_round_trip(overrides in arb_overrides()) {
            let toml_str = toml::to_string(&overrides).unwrap();
            let deserialized: Overrides = toml::from_str(&toml_str).unwrap();
            prop_assert_eq!(overrides, deserialized);
        }
    }

    // Feature: stackcut-v01-completion, Property 3: StackcutConfig TOML Round-Trip
    // **Validates: Requirements 26.3**
    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(100))]

        #[test]
        fn stackcut_config_toml_round_trip(config in arb_stackcut_config()) {
            let toml_str = toml::to_string(&config).unwrap();
            let deserialized: StackcutConfig = toml::from_str(&toml_str).unwrap();
            prop_assert_eq!(config, deserialized);
        }
    }

    // ── compare_plans tests ────────────────────────────────────────────────

    fn make_plan(base: &str, head: &str, units: Vec<EditUnit>, slices: Vec<Slice>) -> Plan {
        Plan {
            version: "0.1.0".to_string(),
            source: PlanSource {
                repo_root: None,
                base: base.to_string(),
                head: head.to_string(),
                head_tree: None,
            },
            units,
            slices,
            ambiguities: Vec::new(),
            diagnostics: Vec::new(),
            fingerprint: None,
        }
    }

    fn make_unit(id: &str, family: &str) -> EditUnit {
        EditUnit {
            id: id.to_string(),
            path: id.strip_prefix("path:").unwrap_or(id).to_string(),
            old_path: None,
            status: ChangeStatus::Modified,
            kind: UnitKind::Behavior,
            family: family.to_string(),
            notes: Vec::new(),
        }
    }

    fn make_slice(id: &str, title: &str, members: &[&str]) -> Slice {
        Slice {
            id: id.to_string(),
            title: title.to_string(),
            kind: SliceKind::Behavior,
            families: Vec::new(),
            members: members.iter().map(|s| s.to_string()).collect(),
            depends_on: Vec::new(),
            reasons: Vec::new(),
            proof_surface: ProofSurface::default(),
        }
    }

    #[test]
    fn compare_identical_plans() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let slices = vec![make_slice("behavior-core", "Core", &["path:src/a.rs"])];
        let plan = make_plan("aaa", "bbb", units, slices);

        let cmp = compare_plans(&plan, &plan);
        assert!(cmp.slices_added.is_empty());
        assert!(cmp.slices_removed.is_empty());
        assert!(cmp.slices_modified.is_empty());
        assert!(cmp.units_moved.is_empty());
        assert_eq!(cmp.slices_unchanged, vec!["behavior-core"]);
        assert_eq!(cmp.equivalence, Equivalence::UnitEquivalent);
        assert!(!cmp.source_changed);
    }

    #[test]
    fn compare_added_and_removed_slices() {
        let old_units = vec![make_unit("path:src/a.rs", "core")];
        let old_slices = vec![make_slice("slice-old", "Old", &["path:src/a.rs"])];
        let old = make_plan("aaa", "bbb", old_units, old_slices);

        let new_units = vec![make_unit("path:src/b.rs", "auth")];
        let new_slices = vec![make_slice("slice-new", "New", &["path:src/b.rs"])];
        let new = make_plan("aaa", "bbb", new_units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert_eq!(cmp.slices_added, vec!["slice-new"]);
        assert_eq!(cmp.slices_removed, vec!["slice-old"]);
        assert!(cmp.slices_unchanged.is_empty());
        assert_eq!(cmp.equivalence, Equivalence::UnitsDiffer);
    }

    #[test]
    fn compare_units_moved_between_slices() {
        let units = vec![
            make_unit("path:src/shared.rs", "shared"),
            make_unit("path:src/other.rs", "other"),
        ];

        let old_slices = vec![
            make_slice("slice-a", "A", &["path:src/shared.rs"]),
            make_slice("slice-b", "B", &["path:src/other.rs"]),
        ];
        let old = make_plan("aaa", "bbb", units.clone(), old_slices);

        let new_slices = vec![
            make_slice("slice-a", "A", &["path:src/other.rs"]),
            make_slice("slice-b", "B", &["path:src/shared.rs"]),
        ];
        let new = make_plan("aaa", "bbb", units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert_eq!(cmp.units_moved.len(), 2);
        assert_eq!(cmp.equivalence, Equivalence::UnitEquivalent);
    }

    #[test]
    fn compare_modified_slice_members() {
        let old_units = vec![
            make_unit("path:src/a.rs", "core"),
            make_unit("path:src/old.rs", "core"),
        ];
        let old_slices = vec![make_slice(
            "behavior-core",
            "Core",
            &["path:src/a.rs", "path:src/old.rs"],
        )];
        let old = make_plan("aaa", "bbb", old_units, old_slices);

        let new_units = vec![
            make_unit("path:src/a.rs", "core"),
            make_unit("path:src/new.rs", "core"),
        ];
        let new_slices = vec![make_slice(
            "behavior-core",
            "Core",
            &["path:src/a.rs", "path:src/new.rs"],
        )];
        let new = make_plan("aaa", "bbb", new_units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert!(cmp.slices_unchanged.is_empty());
        assert_eq!(cmp.slices_modified.len(), 1);
        let diff = &cmp.slices_modified[0];
        assert_eq!(diff.slice_id, "behavior-core");
        assert_eq!(diff.members_added, vec!["path:src/new.rs"]);
        assert_eq!(diff.members_removed, vec!["path:src/old.rs"]);
        assert!(!diff.title_changed);
    }

    #[test]
    fn compare_source_changed() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let slices = vec![make_slice("s", "S", &["path:src/a.rs"])];
        let old = make_plan("aaa", "bbb", units.clone(), slices.clone());
        let new = make_plan("aaa", "ccc", units, slices);

        let cmp = compare_plans(&old, &new);
        assert!(cmp.source_changed);
    }

    #[test]
    fn compare_title_changed() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let old_slices = vec![make_slice("s", "Old Title", &["path:src/a.rs"])];
        let new_slices = vec![make_slice("s", "New Title", &["path:src/a.rs"])];
        let old = make_plan("aaa", "bbb", units.clone(), old_slices);
        let new = make_plan("aaa", "bbb", units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert_eq!(cmp.slices_modified.len(), 1);
        let diff = &cmp.slices_modified[0];
        assert!(diff.title_changed);
        assert_eq!(diff.old_title.as_deref(), Some("Old Title"));
        assert_eq!(diff.new_title.as_deref(), Some("New Title"));
    }

    fn make_slice_with_deps(id: &str, title: &str, members: &[&str], deps: &[&str]) -> Slice {
        let mut s = make_slice(id, title, members);
        s.depends_on = deps.iter().map(|d| d.to_string()).collect();
        s
    }

    #[test]
    fn compare_depends_on_changed() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let old_slices = vec![make_slice("s", "S", &["path:src/a.rs"])];
        let new_slices = vec![make_slice_with_deps(
            "s",
            "S",
            &["path:src/a.rs"],
            &["other"],
        )];
        let old = make_plan("aaa", "bbb", units.clone(), old_slices);
        let new = make_plan("aaa", "bbb", units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert!(cmp.slices_unchanged.is_empty());
        assert_eq!(cmp.slices_modified.len(), 1);
        let diff = &cmp.slices_modified[0];
        assert!(diff.depends_on_changed);
        assert!(!diff.title_changed);
        assert!(diff.members_added.is_empty());
        assert!(diff.members_removed.is_empty());
    }

    #[test]
    fn compare_depends_on_unchanged() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let old_slices = vec![make_slice_with_deps("s", "S", &["path:src/a.rs"], &["dep"])];
        let new_slices = vec![make_slice_with_deps("s", "S", &["path:src/a.rs"], &["dep"])];
        let old = make_plan("aaa", "bbb", units.clone(), old_slices);
        let new = make_plan("aaa", "bbb", units, new_slices);

        let cmp = compare_plans(&old, &new);
        assert_eq!(cmp.slices_unchanged, vec!["s"]);
        assert!(cmp.slices_modified.is_empty());
    }

    #[test]
    fn compare_source_changed_via_repo_root() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let slices = vec![make_slice("s", "S", &["path:src/a.rs"])];
        let mut old = make_plan("aaa", "bbb", units.clone(), slices.clone());
        let new = make_plan("aaa", "bbb", units, slices);
        old.source.repo_root = Some("/old/root".to_string());

        let cmp = compare_plans(&old, &new);
        assert!(cmp.source_changed);
    }

    #[test]
    fn compare_duplicate_slice_ids_uses_first() {
        let units = vec![make_unit("path:src/a.rs", "core")];
        let old_slices = vec![
            make_slice("dup", "First", &["path:src/a.rs"]),
            make_slice("dup", "Second", &["path:src/a.rs"]),
        ];
        let new_slices = vec![make_slice("dup", "First", &["path:src/a.rs"])];
        let old = make_plan("aaa", "bbb", units.clone(), old_slices);
        let new = make_plan("aaa", "bbb", units, new_slices);

        let cmp = compare_plans(&old, &new);
        // First occurrence ("First") matches new ("First") => unchanged
        assert_eq!(cmp.slices_unchanged, vec!["dup"]);
    }

    #[test]
    fn render_comparison_contains_expected_sections() {
        let old_units = vec![
            make_unit("path:src/a.rs", "core"),
            make_unit("path:src/shared.rs", "shared"),
        ];
        let old_slices = vec![make_slice(
            "behavior-core",
            "Core",
            &["path:src/a.rs", "path:src/shared.rs"],
        )];
        let old = make_plan("aaa", "bbb", old_units, old_slices);

        let new_units = vec![
            make_unit("path:src/a.rs", "core"),
            make_unit("path:src/shared.rs", "shared"),
        ];
        let new_slices = vec![
            make_slice("behavior-core", "Core", &["path:src/a.rs"]),
            make_slice("behavior-auth", "Auth", &["path:src/shared.rs"]),
        ];
        let new = make_plan("aaa", "bbb", new_units, new_slices);

        let cmp = compare_plans(&old, &new);
        let rendered = render_comparison(&cmp);

        assert!(rendered.contains("# stackcut plan comparison"));
        assert!(rendered.contains("fingerprint:"));
        assert!(rendered.contains("equivalence:"));
        assert!(rendered.contains("## slices added"));
        assert!(rendered.contains("`behavior-auth`"));
        assert!(rendered.contains("## slices removed"));
        assert!(rendered.contains("## slices modified"));
        assert!(rendered.contains("## units moved"));
    }
}
