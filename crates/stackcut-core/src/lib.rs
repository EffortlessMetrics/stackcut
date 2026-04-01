use anyhow::{bail, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PLAN_VERSION: &str = "0.1.0";
pub const SUPPORTED_CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ChangeStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum UnitKind {
    Manifest,
    Lockfile,
    Generated,
    Test,
    Documentation,
    OpsConfig,
    Mechanical,
    Behavior,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum SliceKind {
    PrepRefactor,
    Behavior,
    TestsDocs,
    Generated,
    Mechanical,
    OpsConfig,
    ApiSchema,
    Misc,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Note,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct EditUnit {
    pub id: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_path: Option<String>,
    pub status: ChangeStatus,
    pub kind: UnitKind,
    pub family: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct InclusionReason {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct ProofSurface {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenario_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub expected_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Slice {
    pub id: String,
    pub title: String,
    pub kind: SliceKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub families: Vec<String>,
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<InclusionReason>,
    #[serde(default)]
    pub proof_surface: ProofSurface,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Ambiguity {
    pub id: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_units: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub candidate_slices: Vec<String>,
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PlanSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_root: Option<String>,
    pub base: String,
    pub head: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_tree: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Plan {
    pub version: String,
    pub source: PlanSource,
    pub units: Vec<EditUnit>,
    pub slices: Vec<Slice>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ambiguities: Vec<Ambiguity>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub override_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PathFamilyRule {
    pub prefix: String,
    pub family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct StackcutConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub generated_prefixes: Vec<String>,
    #[serde(default)]
    pub manifest_files: Vec<String>,
    #[serde(default)]
    pub lock_files: Vec<String>,
    #[serde(default)]
    pub test_prefixes: Vec<String>,
    #[serde(default)]
    pub doc_prefixes: Vec<String>,
    #[serde(default)]
    pub ops_prefixes: Vec<String>,
    #[serde(default)]
    pub path_families: Vec<PathFamilyRule>,
    #[serde(default)]
    pub review_budget: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MustLinkOverride {
    pub members: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ForceMemberOverride {
    pub member: String,
    pub slice: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct RenameSliceOverride {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MustOrderOverride {
    pub before: String,
    pub after: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct Overrides {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub must_link: Vec<MustLinkOverride>,
    #[serde(default)]
    pub force_members: Vec<ForceMemberOverride>,
    #[serde(default)]
    pub rename_slices: Vec<RenameSliceOverride>,
    #[serde(default)]
    pub must_order: Vec<MustOrderOverride>,
}

fn default_version() -> u32 {
    1
}

impl Default for StackcutConfig {
    fn default() -> Self {
        Self {
            version: 1,
            generated_prefixes: vec![
                "generated/".to_string(),
                "dist/".to_string(),
                "fixtures/generated/".to_string(),
            ],
            manifest_files: vec![
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                "pyproject.toml".to_string(),
            ],
            lock_files: vec![
                "Cargo.lock".to_string(),
                "package-lock.json".to_string(),
                "pnpm-lock.yaml".to_string(),
            ],
            test_prefixes: vec!["tests/".to_string(), "specs/".to_string()],
            doc_prefixes: vec!["docs/".to_string(), "adr/".to_string()],
            ops_prefixes: vec![
                ".github/".to_string(),
                "ci/".to_string(),
                ".circleci/".to_string(),
            ],
            path_families: vec![
                PathFamilyRule {
                    prefix: "src/core/".to_string(),
                    family: "core".to_string(),
                },
                PathFamilyRule {
                    prefix: "src/git/".to_string(),
                    family: "git".to_string(),
                },
                PathFamilyRule {
                    prefix: "src/cli/".to_string(),
                    family: "cli".to_string(),
                },
            ],
            review_budget: None,
        }
    }
}

impl Default for Overrides {
    fn default() -> Self {
        Self {
            version: 1,
            must_link: Vec::new(),
            force_members: Vec::new(),
            rename_slices: Vec::new(),
            must_order: Vec::new(),
        }
    }
}

const KNOWN_CONFIG_KEYS: &[&str] = &[
    "version",
    "generated_prefixes",
    "manifest_files",
    "lock_files",
    "test_prefixes",
    "doc_prefixes",
    "ops_prefixes",
    "path_families",
    "review_budget",
];

/// Parse stackcut.toml with strict validation.
/// - Rejects version > SUPPORTED_CONFIG_VERSION
/// - Emits warning diagnostics for unknown keys
pub fn parse_config(contents: &str) -> Result<(StackcutConfig, Vec<Diagnostic>)> {
    let mut diagnostics = Vec::new();
    let raw: toml::Value = toml::from_str(contents)?;

    if let Some(table) = raw.as_table() {
        for key in table.keys() {
            if !KNOWN_CONFIG_KEYS.contains(&key.as_str()) {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: "unknown-config-key".to_string(),
                    message: format!("Unknown key '{}' in stackcut.toml", key),
                });
            }
        }
    }

    let config: StackcutConfig = toml::from_str(contents)?;
    if config.version > SUPPORTED_CONFIG_VERSION {
        bail!(
            "stackcut.toml version {} is not supported (max: {})",
            config.version,
            SUPPORTED_CONFIG_VERSION
        );
    }

    Ok((config, diagnostics))
}

impl Plan {
    pub fn unit_map(&self) -> BTreeMap<String, EditUnit> {
        self.units
            .iter()
            .cloned()
            .map(|unit| (unit.id.clone(), unit))
            .collect()
    }
}

pub fn classify_path(path: &str, status: &ChangeStatus, config: &StackcutConfig) -> UnitKind {
    let file_name = path.rsplit('/').next().unwrap_or(path);

    if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
        return UnitKind::Mechanical;
    }

    if config
        .manifest_files
        .iter()
        .any(|entry| entry == file_name || entry == path)
    {
        return UnitKind::Manifest;
    }

    if config
        .lock_files
        .iter()
        .any(|entry| entry == file_name || entry == path)
    {
        return UnitKind::Lockfile;
    }

    if config
        .generated_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || path.ends_with(".snap")
        || path.ends_with(".generated.rs")
    {
        return UnitKind::Generated;
    }

    if config
        .test_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".spec.ts")
    {
        return UnitKind::Test;
    }

    if config
        .doc_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || path.ends_with(".md")
        || path.ends_with(".mdx")
    {
        return UnitKind::Documentation;
    }

    if config
        .ops_prefixes
        .iter()
        .any(|prefix| path.starts_with(prefix))
        || file_name == "Dockerfile"
        || file_name == "docker-compose.yml"
    {
        return UnitKind::OpsConfig;
    }

    UnitKind::Behavior
}

pub fn infer_family(path: &str, config: &StackcutConfig) -> String {
    for mapping in &config.path_families {
        if path.starts_with(&mapping.prefix) {
            return mapping.family.clone();
        }
    }

    let parts: Vec<&str> = path.split('/').collect();

    if parts.is_empty() {
        return "root".to_string();
    }

    match parts.as_slice() {
        ["src", family, ..] => family.to_string(),
        ["crates", name, ..] => name.to_string(),
        ["tests", name, ..] => name.to_string(),
        ["docs", name, ..] => name.to_string(),
        [single] => {
            if single.ends_with(".md") {
                "root".to_string()
            } else {
                strip_extension(single)
            }
        }
        [first, ..] => strip_extension(first),
        _ => "root".to_string(),
    }
}

fn strip_extension(value: &str) -> String {
    value.split('.').next().unwrap_or(value).to_string()
}

fn infer_owner_by_path_segment(
    path: &str,
    family_to_slice: &BTreeMap<String, String>,
) -> Option<String> {
    let segments: Vec<&str> = path.split('/').collect();
    let stem = segments
        .last()
        .and_then(|s| {
            s.strip_suffix(".rs")
                .or_else(|| s.strip_suffix(".ts"))
                .or_else(|| s.strip_suffix(".js"))
                .or_else(|| s.strip_suffix(".md"))
                .or_else(|| s.strip_suffix(".json"))
        })
        .unwrap_or("");

    let mut candidates = Vec::new();
    for (family, slice_id) in family_to_slice {
        if segments.contains(&family.as_str()) || stem == family.as_str() {
            candidates.push(slice_id.clone());
        }
    }

    if candidates.len() == 1 {
        Some(candidates.into_iter().next().unwrap())
    } else {
        None // still ambiguous
    }
}

pub fn plan(
    source: PlanSource,
    mut units: Vec<EditUnit>,
    config: &StackcutConfig,
    overrides: &Overrides,
) -> Plan {
    units.sort_by(|left, right| left.path.cmp(&right.path));

    let mut slices: Vec<Slice> = Vec::new();
    let mut ambiguities: Vec<Ambiguity> = Vec::new();
    let mut assigned: BTreeSet<String> = BTreeSet::new();

    let manifest_ids = collect_ids(&units, &assigned, |unit| {
        matches!(unit.kind, UnitKind::Manifest | UnitKind::Lockfile)
    });
    if !manifest_ids.is_empty() {
        mark_assigned(&mut assigned, &manifest_ids);
        slices.push(new_slice(
            "api-schema-workspace",
            "Manifest and lockstep package metadata",
            SliceKind::ApiSchema,
            family_list_for_members(&units, &manifest_ids),
            manifest_ids,
            Vec::new(),
            vec![reason(
                "lockstep-metadata",
                "Manifest and lock files move together in v0.1.",
            )],
        ));
    }

    let ops_ids = collect_ids(&units, &assigned, |unit| unit.kind == UnitKind::OpsConfig);
    if !ops_ids.is_empty() {
        mark_assigned(&mut assigned, &ops_ids);
        slices.push(new_slice(
            "ops-config",
            "Ops and configuration",
            SliceKind::OpsConfig,
            family_list_for_members(&units, &ops_ids),
            ops_ids,
            Vec::new(),
            vec![reason(
                "ops-isolation",
                "Operational configuration is isolated from behavior changes.",
            )],
        ));
    }

    let mechanical_ids = collect_ids(&units, &assigned, |unit| unit.kind == UnitKind::Mechanical);
    if !mechanical_ids.is_empty() {
        mark_assigned(&mut assigned, &mechanical_ids);

        let all_renames = mechanical_ids.iter().all(|id| {
            units
                .iter()
                .find(|u| u.id == *id)
                .map(|u| u.status == ChangeStatus::Renamed)
                .unwrap_or(false)
        });
        let slice_kind = if all_renames {
            SliceKind::PrepRefactor
        } else {
            SliceKind::Mechanical
        };

        slices.push(new_slice(
            "mechanical-renames",
            "Mechanical rename-only changes",
            slice_kind,
            family_list_for_members(&units, &mechanical_ids),
            mechanical_ids,
            Vec::new(),
            vec![reason(
                "mechanical-split",
                "Rename-only changes peel off as a prep slice when independent.",
            )],
        ));
    }

    let mut behavior_by_family: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for unit in &units {
        if assigned.contains(&unit.id) {
            continue;
        }
        if unit.kind == UnitKind::Behavior {
            behavior_by_family
                .entry(unit.family.clone())
                .or_default()
                .push(unit.id.clone());
        }
    }

    let mut family_to_slice: BTreeMap<String, String> = BTreeMap::new();
    for (family, member_ids) in behavior_by_family {
        mark_assigned(&mut assigned, &member_ids);
        let slice_id = format!("behavior-{}", slugify(&family));
        let mut depends_on = Vec::new();
        if has_slice(&slices, "api-schema-workspace") {
            depends_on.push("api-schema-workspace".to_string());
        }
        if has_family_overlap(&units, &member_ids, &slices, "mechanical-renames") {
            depends_on.push("mechanical-renames".to_string());
        }

        slices.push(new_slice(
            &slice_id,
            &format!("Behavior: {}", family),
            SliceKind::Behavior,
            vec![family.clone()],
            member_ids,
            depends_on,
            vec![reason(
                "family-grouping",
                "Behavioral edits group by inferred path family.",
            )],
        ));
        family_to_slice.insert(family, slice_id);
    }

    let attachable_ids = collect_ids(&units, &assigned, |unit| {
        matches!(
            unit.kind,
            UnitKind::Generated | UnitKind::Test | UnitKind::Documentation
        )
    });
    let unit_lookup: BTreeMap<String, EditUnit> = units
        .iter()
        .cloned()
        .map(|unit| (unit.id.clone(), unit))
        .collect();

    let mut standalone_groups: BTreeMap<(SliceKind, String), Vec<String>> = BTreeMap::new();
    for unit_id in attachable_ids {
        let Some(unit) = unit_lookup.get(&unit_id) else {
            continue;
        };
        if let Some(slice_id) = family_to_slice.get(&unit.family) {
            attach_member(
                &mut slices,
                slice_id,
                unit,
                &format!(
                    "{} stays with the {} family when ownership is clear.",
                    describe_kind(&unit.kind),
                    unit.family
                ),
            );
            assigned.insert(unit.id.clone());
            continue;
        }

        if let Some(slice_id) = infer_owner_by_path_segment(&unit.path, &family_to_slice) {
            attach_member(
                &mut slices,
                &slice_id,
                unit,
                &format!(
                    "{} attached to {} via path-segment inference.",
                    describe_kind(&unit.kind),
                    unit.path
                ),
            );
            assigned.insert(unit.id.clone());
            continue;
        }

        if unit.family == "root" && family_to_slice.len() > 1 {
            ambiguities.push(Ambiguity {
                id: format!("ambiguity-{}", slugify(&unit.path)),
                message: format!(
                    "{} changed with multiple behavior families and cannot be attached confidently in v0.1.",
                    unit.path
                ),
                affected_units: vec![unit.id.clone()],
                candidate_slices: family_to_slice.values().cloned().collect(),
                resolution: "Left as a standalone docs/tests slice. Use override.toml to attach explicitly."
                    .to_string(),
            });
        }

        let standalone_kind = match unit.kind {
            UnitKind::Generated => SliceKind::Generated,
            UnitKind::Test | UnitKind::Documentation => SliceKind::TestsDocs,
            _ => SliceKind::Misc,
        };
        standalone_groups
            .entry((standalone_kind, unit.family.clone()))
            .or_default()
            .push(unit.id.clone());
        assigned.insert(unit.id.clone());
    }

    for ((slice_kind, family), member_ids) in standalone_groups {
        let slice_id = match slice_kind {
            SliceKind::Generated => format!("generated-{}", slugify(&family)),
            SliceKind::TestsDocs => format!("tests-docs-{}", slugify(&family)),
            _ => format!("misc-{}", slugify(&family)),
        };
        let title = match slice_kind {
            SliceKind::Generated => format!("Generated: {}", family),
            SliceKind::TestsDocs => format!("Docs/tests: {}", family),
            _ => format!("Misc: {}", family),
        };
        let mut depends_on = Vec::new();
        if let Some(slice_id_for_family) = family_to_slice.get(&family) {
            depends_on.push(slice_id_for_family.clone());
        }
        slices.push(new_slice(
            &slice_id,
            &title,
            slice_kind,
            family_list_for_members(&units, &member_ids),
            member_ids,
            depends_on,
            vec![reason(
                "standalone-attachment",
                "No single behavior owner was available, so the material stays explicit.",
            )],
        ));
    }

    // Collect any remaining unassigned units (including unsupported ones) into a misc slice
    let unassigned_ids: Vec<String> = units
        .iter()
        .filter(|u| !assigned.contains(&u.id))
        .map(|u| u.id.clone())
        .collect();
    if !unassigned_ids.is_empty() {
        mark_assigned(&mut assigned, &unassigned_ids);
        slices.push(new_slice(
            "misc-unassigned",
            "Misc: unassigned changes",
            SliceKind::Misc,
            family_list_for_members(&units, &unassigned_ids),
            unassigned_ids,
            Vec::new(),
            vec![reason(
                "misc-catchall",
                "Unassigned units collected into misc slice.",
            )],
        ));
    }

    // Validate overrides against known unit and slice IDs before applying
    let unit_ids: BTreeSet<String> = units.iter().map(|u| u.id.clone()).collect();
    let slice_ids: BTreeSet<String> = slices.iter().map(|s| s.id.clone()).collect();
    let override_diagnostics = validate_overrides(overrides, &unit_ids, &slice_ids);

    let apply_diagnostics = apply_overrides(&mut slices, overrides);

    let mut diagnostics = structural_validate(&Plan {
        version: PLAN_VERSION.to_string(),
        source: source.clone(),
        units: units.clone(),
        slices: slices.clone(),
        ambiguities: ambiguities.clone(),
        diagnostics: Vec::new(),
        fingerprint: None,
        override_fingerprint: None,
    });

    if !ambiguities.is_empty() {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "ambiguity-present".to_string(),
            message: "Plan contains one or more explicit ambiguities.".to_string(),
        });
    }

    diagnostics.extend(override_diagnostics);
    diagnostics.extend(apply_diagnostics);

    let budget = config.review_budget.unwrap_or(15) as usize;
    for slice in &slices {
        if slice.members.len() > budget {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "review-budget-exceeded".to_string(),
                message: format!(
                    "Slice '{}' has {} members (budget: {})",
                    slice.id,
                    slice.members.len(),
                    budget
                ),
            });
        }
    }

    // Emit warning diagnostics for units with unsupported notes
    for unit in &units {
        for note in &unit.notes {
            if note.starts_with("unsupported-") {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: note.clone(),
                    message: format!("{} is an unsupported change type in v0.1", unit.path),
                });
            }
        }
    }

    let override_fingerprint = {
        let has_overrides = !overrides.must_link.is_empty()
            || !overrides.force_members.is_empty()
            || !overrides.rename_slices.is_empty()
            || !overrides.must_order.is_empty();
        if has_overrides {
            use sha2::{Digest, Sha256};
            let json = serde_json::to_string(overrides).unwrap_or_default();
            Some(format!("{:x}", Sha256::digest(json.as_bytes())))
        } else {
            None
        }
    };

    Plan {
        version: PLAN_VERSION.to_string(),
        source,
        units,
        slices,
        ambiguities,
        diagnostics,
        fingerprint: None,
        override_fingerprint,
    }
}

pub fn structural_validate(plan: &Plan) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let known_slice_ids: BTreeSet<String> =
        plan.slices.iter().map(|slice| slice.id.clone()).collect();
    let mut member_counts: BTreeMap<String, usize> = BTreeMap::new();

    for slice in &plan.slices {
        if slice.members.is_empty() {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "empty-slice".to_string(),
                message: format!("Slice {} has no members.", slice.id),
            });
        }

        for dependency in &slice.depends_on {
            if dependency == &slice.id {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Error,
                    code: "self-dependency".to_string(),
                    message: format!("Slice {} depends on itself.", slice.id),
                });
            } else if !known_slice_ids.contains(dependency) {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Error,
                    code: "missing-dependency".to_string(),
                    message: format!(
                        "Slice {} depends on unknown slice {}.",
                        slice.id, dependency
                    ),
                });
            }
        }

        for member in &slice.members {
            *member_counts.entry(member.clone()).or_insert(0) += 1;
        }
    }

    for unit in &plan.units {
        match member_counts.get(&unit.id) {
            Some(1) => {}
            Some(count) => diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Error,
                code: "duplicate-member".to_string(),
                message: format!("Unit {} appears {} times across slices.", unit.id, count),
            }),
            None => diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Error,
                code: "missing-member".to_string(),
                message: format!("Unit {} is not assigned to any slice.", unit.id),
            }),
        }
    }

    if has_cycle(&plan.slices) {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Error,
            code: "cycle".to_string(),
            message: "Slice dependency graph contains a cycle.".to_string(),
        });
    }

    diagnostics.sort_by(|left, right| left.code.cmp(&right.code));
    diagnostics
}

fn has_cycle(slices: &[Slice]) -> bool {
    let mut incoming: BTreeMap<String, usize> = slices
        .iter()
        .map(|slice| (slice.id.clone(), 0usize))
        .collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for slice in slices {
        for dependency in &slice.depends_on {
            *incoming.entry(slice.id.clone()).or_insert(0) += 1;
            outgoing
                .entry(dependency.clone())
                .or_default()
                .push(slice.id.clone());
        }
    }

    let mut ready: Vec<String> = incoming
        .iter()
        .filter_map(|(node, count)| {
            if *count == 0 {
                Some(node.clone())
            } else {
                None
            }
        })
        .collect();
    ready.sort();

    let mut visited = 0usize;
    while let Some(node) = ready.pop() {
        visited += 1;
        if let Some(children) = outgoing.get(&node) {
            for child in children {
                if let Some(count) = incoming.get_mut(child) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push(child.clone());
                        ready.sort();
                    }
                }
            }
        }
    }

    visited != slices.len()
}

fn apply_overrides(slices: &mut Vec<Slice>, overrides: &Overrides) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for rule in &overrides.must_link {
        if rule.members.is_empty() {
            continue;
        }

        let anchor_slice_id = rule
            .members
            .iter()
            .find_map(|member| find_slice_for_member(slices, member))
            .unwrap_or_else(|| {
                let new_id = format!("override-{}", slugify(&rule.members[0]));
                slices.push(new_slice(
                    &new_id,
                    "Override bundle",
                    SliceKind::Misc,
                    vec!["override".to_string()],
                    Vec::new(),
                    Vec::new(),
                    vec![reason("override", "Created to satisfy must_link override.")],
                ));
                new_id
            });

        for member in &rule.members {
            move_member(slices, member, &anchor_slice_id);
        }

        if let Some(slice) = slices.iter_mut().find(|slice| slice.id == anchor_slice_id) {
            slice.reasons.push(reason(
                "override-must-link",
                rule.reason
                    .as_deref()
                    .unwrap_or("Members were forced to stay together by override."),
            ));
            dedup_and_sort(&mut slice.members);
        }
    }

    for rule in &overrides.force_members {
        if !slices.iter().any(|slice| slice.id == rule.slice) {
            slices.push(new_slice(
                &rule.slice,
                &rule.slice,
                SliceKind::Misc,
                vec!["override".to_string()],
                Vec::new(),
                Vec::new(),
                vec![reason(
                    "override",
                    "Created to satisfy force_members override.",
                )],
            ));
        }

        move_member(slices, &rule.member, &rule.slice);

        if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.slice) {
            slice.reasons.push(reason(
                "override-force-member",
                rule.reason
                    .as_deref()
                    .unwrap_or("Member was forced into this slice by override."),
            ));
            dedup_and_sort(&mut slice.members);
        }
    }

    for rule in &overrides.rename_slices {
        if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.id) {
            slice.title = rule.title.clone();
        }
    }

    for rule in &overrides.must_order {
        // Add the edge first
        let found = if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.after) {
            slice.depends_on.push(rule.before.clone());
            dedup_and_sort(&mut slice.depends_on);
            true
        } else {
            false
        };

        if !found {
            continue;
        }

        // Check for cycle (no outstanding mutable borrow)
        if has_cycle(slices) {
            // Revert the edge
            if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.after) {
                slice.depends_on.retain(|d| d != &rule.before);
            }
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Error,
                code: "override-cycle".to_string(),
                message: format!(
                    "must_order '{} -> {}' would create a cycle; edge rejected",
                    rule.before, rule.after
                ),
            });
        } else {
            if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.after) {
                slice.reasons.push(reason(
                    "override-must-order",
                    rule.reason
                        .as_deref()
                        .unwrap_or("Ordering edge added by override."),
                ));
            }
        }
    }

    diagnostics
}

fn move_member(slices: &mut [Slice], member: &str, target_slice: &str) {
    for slice in slices.iter_mut() {
        if slice.id != target_slice {
            slice.members.retain(|candidate| candidate != member);
        }
    }

    if let Some(target) = slices.iter_mut().find(|slice| slice.id == target_slice) {
        if !target.members.iter().any(|candidate| candidate == member) {
            target.members.push(member.to_string());
        }
        dedup_and_sort(&mut target.members);
    }
}

fn attach_member(slices: &mut [Slice], slice_id: &str, unit: &EditUnit, message: &str) {
    if let Some(slice) = slices.iter_mut().find(|slice| slice.id == slice_id) {
        slice.members.push(unit.id.clone());
        slice.reasons.push(reason("family-attachment", message));
        if !slice.families.iter().any(|family| family == &unit.family) {
            slice.families.push(unit.family.clone());
            dedup_and_sort(&mut slice.families);
        }
        dedup_and_sort(&mut slice.members);
    }
}

fn collect_ids<F>(units: &[EditUnit], assigned: &BTreeSet<String>, mut predicate: F) -> Vec<String>
where
    F: FnMut(&EditUnit) -> bool,
{
    units
        .iter()
        .filter(|unit| !assigned.contains(&unit.id) && predicate(unit))
        .map(|unit| unit.id.clone())
        .collect()
}

fn family_list_for_members(units: &[EditUnit], member_ids: &[String]) -> Vec<String> {
    let families: BTreeSet<String> = units
        .iter()
        .filter(|unit| member_ids.iter().any(|member| member == &unit.id))
        .map(|unit| unit.family.clone())
        .collect();
    families.into_iter().collect()
}

fn has_slice(slices: &[Slice], id: &str) -> bool {
    slices.iter().any(|slice| slice.id == id)
}

fn has_family_overlap(
    units: &[EditUnit],
    member_ids: &[String],
    slices: &[Slice],
    slice_id: &str,
) -> bool {
    let member_families = family_list_for_members(units, member_ids);
    slices
        .iter()
        .find(|slice| slice.id == slice_id)
        .map(|slice| {
            slice
                .families
                .iter()
                .any(|family| member_families.iter().any(|candidate| candidate == family))
        })
        .unwrap_or(false)
}

fn mark_assigned(assigned: &mut BTreeSet<String>, member_ids: &[String]) {
    for member_id in member_ids {
        assigned.insert(member_id.clone());
    }
}

fn new_slice(
    id: &str,
    title: &str,
    kind: SliceKind,
    families: Vec<String>,
    members: Vec<String>,
    depends_on: Vec<String>,
    reasons: Vec<InclusionReason>,
) -> Slice {
    let mut slice = Slice {
        id: id.to_string(),
        title: title.to_string(),
        kind,
        families,
        members,
        depends_on,
        reasons,
        proof_surface: ProofSurface {
            scenario_ids: Vec::new(),
            expected_commands: vec!["cargo test --workspace".to_string()],
        },
    };
    dedup_and_sort(&mut slice.families);
    dedup_and_sort(&mut slice.members);
    dedup_and_sort(&mut slice.depends_on);
    slice
}

fn reason(code: &str, message: &str) -> InclusionReason {
    InclusionReason {
        code: code.to_string(),
        message: message.to_string(),
    }
}

fn find_slice_for_member(slices: &[Slice], member: &str) -> Option<String> {
    slices
        .iter()
        .find(|slice| slice.members.iter().any(|candidate| candidate == member))
        .map(|slice| slice.id.clone())
}

fn dedup_and_sort(values: &mut Vec<String>) {
    let set: BTreeSet<String> = values.drain(..).collect();
    values.extend(set);
}

/// Validate parsed overrides against a plan's unit and slice IDs.
/// Returns warning diagnostics for unknown references and malformed rules.
pub fn validate_overrides(
    overrides: &Overrides,
    unit_ids: &BTreeSet<String>,
    slice_ids: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for rule in &overrides.must_link {
        if rule.members.len() < 2 {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "must-link-too-few".to_string(),
                message: "must_link group has fewer than 2 members".to_string(),
            });
        }
        for member in &rule.members {
            if !unit_ids.contains(member) {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: "unknown-override-member".to_string(),
                    message: format!("must_link references unknown member '{}'", member),
                });
            }
        }
    }

    for rule in &overrides.force_members {
        if !unit_ids.contains(&rule.member) {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "unknown-override-member".to_string(),
                message: format!("force_members references unknown member '{}'", rule.member),
            });
        }
    }

    for rule in &overrides.rename_slices {
        if !slice_ids.contains(&rule.id) {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "unknown-override-slice".to_string(),
                message: format!("rename_slices references unknown slice '{}'", rule.id),
            });
        }
    }

    for rule in &overrides.must_order {
        if !slice_ids.contains(&rule.before) {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "unknown-override-slice".to_string(),
                message: format!(
                    "must_order references unknown 'before' slice '{}'",
                    rule.before
                ),
            });
        }
        if !slice_ids.contains(&rule.after) {
            diagnostics.push(Diagnostic {
                level: DiagnosticLevel::Warning,
                code: "unknown-override-slice".to_string(),
                message: format!(
                    "must_order references unknown 'after' slice '{}'",
                    rule.after
                ),
            });
        }
    }

    diagnostics
}

fn describe_kind(kind: &UnitKind) -> &'static str {
    match kind {
        UnitKind::Generated => "generated output",
        UnitKind::Test => "test",
        UnitKind::Documentation => "documentation",
        UnitKind::Manifest => "manifest",
        UnitKind::Lockfile => "lock file",
        UnitKind::OpsConfig => "ops config",
        UnitKind::Mechanical => "mechanical change",
        UnitKind::Behavior => "behavior change",
    }
}

fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }

    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "root".to_string()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use proptest::strategy::ValueTree;

    fn unit(id: &str, path: &str, kind: UnitKind, family: &str) -> EditUnit {
        EditUnit {
            id: id.to_string(),
            path: path.to_string(),
            old_path: None,
            status: ChangeStatus::Modified,
            kind,
            family: family.to_string(),
            notes: Vec::new(),
        }
    }

    #[test]
    fn docs_and_tests_attach_to_behavior_family() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![
            unit(
                "path:src/core/planner.rs",
                "src/core/planner.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:tests/planner.rs",
                "tests/planner.rs",
                UnitKind::Test,
                "core",
            ),
            unit(
                "path:docs/planner.md",
                "docs/planner.md",
                UnitKind::Documentation,
                "core",
            ),
        ];

        let plan = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );
        assert_eq!(plan.slices.len(), 1);
        assert_eq!(plan.slices[0].id, "behavior-core");
        assert_eq!(plan.slices[0].members.len(), 3);
    }

    #[test]
    fn ambiguity_is_surface_for_root_docs() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![
            unit(
                "path:src/core/a.rs",
                "src/core/a.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:src/git/b.rs",
                "src/git/b.rs",
                UnitKind::Behavior,
                "git",
            ),
            unit(
                "path:README.md",
                "README.md",
                UnitKind::Documentation,
                "root",
            ),
        ];

        let plan = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );
        assert_eq!(plan.ambiguities.len(), 1);
        assert!(plan
            .slices
            .iter()
            .any(|slice| slice.id == "tests-docs-root"));
    }

    #[test]
    fn structural_validation_catches_duplicate_member() {
        let plan = Plan {
            version: PLAN_VERSION.to_string(),
            source: PlanSource {
                repo_root: None,
                base: "a".to_string(),
                head: "b".to_string(),
                head_tree: None,
            },
            units: vec![unit("path:x", "x", UnitKind::Behavior, "root")],
            slices: vec![
                new_slice(
                    "behavior-root",
                    "Behavior: root",
                    SliceKind::Behavior,
                    vec!["root".to_string()],
                    vec!["path:x".to_string()],
                    Vec::new(),
                    Vec::new(),
                ),
                new_slice(
                    "tests-docs-root",
                    "Docs/tests: root",
                    SliceKind::TestsDocs,
                    vec!["root".to_string()],
                    vec!["path:x".to_string()],
                    Vec::new(),
                    Vec::new(),
                ),
            ],
            ambiguities: Vec::new(),
            diagnostics: Vec::new(),
            fingerprint: None,
            override_fingerprint: None,
        };

        let diagnostics = structural_validate(&plan);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "duplicate-member"));
    }

    #[test]
    fn parse_config_accepts_documented_sample() {
        let toml = r#"
version = 1
generated_prefixes = ["generated/"]
manifest_files = ["Cargo.toml"]
lock_files = ["Cargo.lock"]
test_prefixes = ["tests/"]
doc_prefixes = ["docs/"]
ops_prefixes = [".github/"]
review_budget = 20

[[path_families]]
prefix = "src/core/"
family = "core"
"#;
        let (config, diagnostics) = parse_config(toml).unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(config.version, 1);
        assert_eq!(config.review_budget, Some(20));
        assert_eq!(config.generated_prefixes, vec!["generated/"]);
    }

    #[test]
    fn parse_config_warns_on_unknown_keys() {
        let toml = r#"
version = 1
unknown_key = "hello"
another_bad_key = 42
"#;
        let (config, diagnostics) = parse_config(toml).unwrap();
        assert_eq!(config.version, 1);
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().all(|d| d.code == "unknown-config-key"));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("unknown_key")));
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("another_bad_key")));
    }

    #[test]
    fn parse_config_rejects_unsupported_version() {
        let toml = "version = 99\n";
        let result = parse_config(toml);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("99"));
        assert!(err_msg.contains("not supported"));
    }

    #[test]
    fn parse_config_review_budget_defaults_to_none() {
        let toml = "version = 1\n";
        let (config, diagnostics) = parse_config(toml).unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(config.review_budget, None);
    }

    #[test]
    fn parse_config_empty_input_uses_defaults() {
        let toml = "";
        let (config, diagnostics) = parse_config(toml).unwrap();
        assert!(diagnostics.is_empty());
        assert_eq!(config.version, 1);
        assert_eq!(config.review_budget, None);
    }

    #[test]
    fn validate_overrides_unknown_must_link_member() {
        let unit_ids: BTreeSet<String> = ["path:a".to_string()].into_iter().collect();
        let slice_ids: BTreeSet<String> = ["behavior-core".to_string()].into_iter().collect();
        let overrides = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:a".to_string(), "path:nonexistent".to_string()],
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == "unknown-override-member")
                .count(),
            1
        );
        assert!(diags[0].message.contains("nonexistent"));
    }

    #[test]
    fn validate_overrides_must_link_too_few() {
        let unit_ids: BTreeSet<String> = ["path:a".to_string()].into_iter().collect();
        let slice_ids: BTreeSet<String> = BTreeSet::new();
        let overrides = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:a".to_string()],
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert!(diags.iter().any(|d| d.code == "must-link-too-few"));
    }

    #[test]
    fn validate_overrides_unknown_force_member() {
        let unit_ids: BTreeSet<String> = BTreeSet::new();
        let slice_ids: BTreeSet<String> = ["s1".to_string()].into_iter().collect();
        let overrides = Overrides {
            force_members: vec![ForceMemberOverride {
                member: "path:ghost".to_string(),
                slice: "s1".to_string(),
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == "unknown-override-member")
                .count(),
            1
        );
    }

    #[test]
    fn validate_overrides_unknown_rename_slice() {
        let unit_ids: BTreeSet<String> = BTreeSet::new();
        let slice_ids: BTreeSet<String> = BTreeSet::new();
        let overrides = Overrides {
            rename_slices: vec![RenameSliceOverride {
                id: "no-such-slice".to_string(),
                title: "New Title".to_string(),
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == "unknown-override-slice")
                .count(),
            1
        );
    }

    #[test]
    fn validate_overrides_unknown_must_order_slices() {
        let unit_ids: BTreeSet<String> = BTreeSet::new();
        let slice_ids: BTreeSet<String> = ["s1".to_string()].into_iter().collect();
        let overrides = Overrides {
            must_order: vec![MustOrderOverride {
                before: "s1".to_string(),
                after: "s-missing".to_string(),
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert_eq!(
            diags
                .iter()
                .filter(|d| d.code == "unknown-override-slice")
                .count(),
            1
        );
        assert!(diags[0].message.contains("s-missing"));
    }

    #[test]
    fn validate_overrides_clean_when_all_refs_valid() {
        let unit_ids: BTreeSet<String> = ["path:a".to_string(), "path:b".to_string()]
            .into_iter()
            .collect();
        let slice_ids: BTreeSet<String> =
            ["s1".to_string(), "s2".to_string()].into_iter().collect();
        let overrides = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:a".to_string(), "path:b".to_string()],
                reason: None,
            }],
            force_members: vec![ForceMemberOverride {
                member: "path:a".to_string(),
                slice: "s1".to_string(),
                reason: None,
            }],
            rename_slices: vec![RenameSliceOverride {
                id: "s1".to_string(),
                title: "New".to_string(),
            }],
            must_order: vec![MustOrderOverride {
                before: "s1".to_string(),
                after: "s2".to_string(),
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = validate_overrides(&overrides, &unit_ids, &slice_ids);
        assert!(
            diags.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diags
        );
    }

    #[test]
    fn validate_overrides_wired_into_plan() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![unit(
            "path:src/core/a.rs",
            "src/core/a.rs",
            UnitKind::Behavior,
            "core",
        )];
        let overrides = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:ghost".to_string()],
                reason: None,
            }],
            rename_slices: vec![RenameSliceOverride {
                id: "no-such-slice".to_string(),
                title: "X".to_string(),
            }],
            ..Overrides::default()
        };
        let result = plan(source, units, &StackcutConfig::default(), &overrides);
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "unknown-override-member"));
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "must-link-too-few"));
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "unknown-override-slice"));
    }

    #[test]
    fn review_budget_exceeded_emits_warning() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        // Create 3 behavior units in the same family → one slice with 3 members
        let units = vec![
            unit(
                "path:src/core/a.rs",
                "src/core/a.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:src/core/b.rs",
                "src/core/b.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:src/core/c.rs",
                "src/core/c.rs",
                UnitKind::Behavior,
                "core",
            ),
        ];
        let config = StackcutConfig {
            review_budget: Some(2), // budget of 2, slice has 3
            ..StackcutConfig::default()
        };

        let result = plan(source, units, &config, &Overrides::default());
        let budget_diags: Vec<_> = result
            .diagnostics
            .iter()
            .filter(|d| d.code == "review-budget-exceeded")
            .collect();
        assert_eq!(budget_diags.len(), 1);
        assert!(budget_diags[0].message.contains("behavior-core"));
        assert!(budget_diags[0].message.contains("3 members"));
        assert!(budget_diags[0].message.contains("budget: 2"));
    }

    #[test]
    fn review_budget_default_is_15() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        // Create 16 behavior units in the same family → exceeds default budget of 15
        let units: Vec<EditUnit> = (0..16)
            .map(|i| {
                unit(
                    &format!("path:src/core/{}.rs", i),
                    &format!("src/core/{}.rs", i),
                    UnitKind::Behavior,
                    "core",
                )
            })
            .collect();

        let result = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "review-budget-exceeded"));
    }

    #[test]
    fn review_budget_no_warning_when_within_budget() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![
            unit(
                "path:src/core/a.rs",
                "src/core/a.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:src/core/b.rs",
                "src/core/b.rs",
                UnitKind::Behavior,
                "core",
            ),
        ];
        let config = StackcutConfig {
            review_budget: Some(5),
            ..StackcutConfig::default()
        };

        let result = plan(source, units, &config, &Overrides::default());
        assert!(!result
            .diagnostics
            .iter()
            .any(|d| d.code == "review-budget-exceeded"));
    }

    /// Generate an alphanumeric key name that is NOT in KNOWN_CONFIG_KEYS.
    fn arb_unknown_key() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{2,15}".prop_filter("must not be a known config key", |key| {
            !KNOWN_CONFIG_KEYS.contains(&key.as_str())
        })
    }

    // Feature: stackcut-v01-completion, Property 19: Unknown Config Keys Warning
    // **Validates: Requirements 4.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unknown_config_keys_warning(
            unknown_keys in prop::collection::hash_set(arb_unknown_key(), 1..=5)
        ) {
            // Build a TOML string with version = 1 plus the unknown keys
            let mut toml_str = "version = 1\n".to_string();
            for key in &unknown_keys {
                toml_str.push_str(&format!("{} = true\n", key));
            }

            let result = parse_config(&toml_str);
            prop_assert!(result.is_ok(), "parse_config should succeed for valid TOML with unknown keys");

            let (_config, diagnostics) = result.unwrap();

            // There should be exactly one "unknown-config-key" diagnostic per unknown key
            let unknown_diags: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.code == "unknown-config-key")
                .collect();
            prop_assert_eq!(
                unknown_diags.len(),
                unknown_keys.len(),
                "Expected one unknown-config-key diagnostic per unknown key, got {} for {} keys",
                unknown_diags.len(),
                unknown_keys.len()
            );

            // Each unknown key should be mentioned in a diagnostic message
            for key in &unknown_keys {
                prop_assert!(
                    unknown_diags.iter().any(|d| d.message.contains(key)),
                    "Expected diagnostic mentioning unknown key '{}'",
                    key
                );
            }

            // All diagnostics should be warnings
            for diag in &unknown_diags {
                prop_assert!(
                    diag.level == DiagnosticLevel::Warning,
                    "Expected warning level diagnostic"
                );
            }
        }
    }

    // Feature: stackcut-v01-completion, Property 20: Unsupported Config Version Rejection
    // **Validates: Requirements 4.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unsupported_config_version_rejection(
            version in (SUPPORTED_CONFIG_VERSION + 1)..=u32::MAX
        ) {
            let toml_str = format!("version = {}\n", version);
            let result = parse_config(&toml_str);
            prop_assert!(
                result.is_err(),
                "parse_config should reject version {} (supported max: {})",
                version,
                SUPPORTED_CONFIG_VERSION
            );
        }
    }

    /// Generate a simple identifier string for use as unit/slice IDs.
    fn arb_id() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{1,12}".prop_map(|s| format!("id:{}", s))
    }

    /// Generate an Overrides struct that mixes valid and invalid references
    /// against the given known unit and slice ID sets.
    fn arb_overrides_with_invalids(
        known_unit_ids: BTreeSet<String>,
        known_slice_ids: BTreeSet<String>,
    ) -> impl Strategy<Value = Overrides> {
        let known_units_vec: Vec<String> = known_unit_ids.into_iter().collect();
        let known_slices_vec: Vec<String> = known_slice_ids.into_iter().collect();

        let ku = known_units_vec.clone();
        let ks = known_slices_vec.clone();

        // Generate unknown IDs that are guaranteed not in the known sets
        let unknown_member_strat = "[a-z]{3,8}".prop_map(|s| format!("unknown-member:{}", s));
        let unknown_slice_strat = "[a-z]{3,8}".prop_map(|s| format!("unknown-slice:{}", s));

        let must_link_strat = {
            let ku2 = ku.clone();
            let um = unknown_member_strat.clone();
            prop::collection::vec(
                (
                    prop::collection::vec(
                        prop::strategy::Union::new(vec![
                            um.clone().boxed(),
                            if ku2.is_empty() {
                                um.clone().boxed()
                            } else {
                                prop::sample::select(ku2.clone()).boxed()
                            },
                        ]),
                        0..=4,
                    ),
                    prop::option::of("[a-z ]{3,20}"),
                ),
                0..=3,
            )
            .prop_map(|groups| {
                groups
                    .into_iter()
                    .map(|(members, reason)| MustLinkOverride { members, reason })
                    .collect::<Vec<_>>()
            })
        };

        let force_members_strat = {
            let ku3 = ku.clone();
            let um2 = unknown_member_strat.clone();
            let ks2 = ks.clone();
            let us2 = unknown_slice_strat.clone();
            prop::collection::vec(
                (
                    prop::strategy::Union::new(vec![
                        um2.clone().boxed(),
                        if ku3.is_empty() {
                            um2.clone().boxed()
                        } else {
                            prop::sample::select(ku3.clone()).boxed()
                        },
                    ]),
                    // force_members target slice — we don't warn on unknown target slices
                    // (they get created), so just use any string
                    if ks2.is_empty() {
                        us2.clone().boxed()
                    } else {
                        prop::strategy::Union::new(vec![
                            us2.clone().boxed(),
                            prop::sample::select(ks2.clone()).boxed(),
                        ])
                        .boxed()
                    },
                    prop::option::of("[a-z ]{3,20}"),
                ),
                0..=3,
            )
            .prop_map(|entries| {
                entries
                    .into_iter()
                    .map(|(member, slice, reason)| ForceMemberOverride {
                        member,
                        slice,
                        reason,
                    })
                    .collect::<Vec<_>>()
            })
        };

        let rename_slices_strat = {
            let ks3 = ks.clone();
            let us3 = unknown_slice_strat.clone();
            prop::collection::vec(
                (
                    prop::strategy::Union::new(vec![
                        us3.clone().boxed(),
                        if ks3.is_empty() {
                            us3.clone().boxed()
                        } else {
                            prop::sample::select(ks3.clone()).boxed()
                        },
                    ]),
                    "[A-Z][a-z ]{2,15}",
                ),
                0..=3,
            )
            .prop_map(|entries| {
                entries
                    .into_iter()
                    .map(|(id, title)| RenameSliceOverride { id, title })
                    .collect::<Vec<_>>()
            })
        };

        let must_order_strat = {
            let ks4 = ks.clone();
            let us4 = unknown_slice_strat.clone();
            let any_slice = prop::strategy::Union::new(vec![
                us4.clone().boxed(),
                if ks4.is_empty() {
                    us4.clone().boxed()
                } else {
                    prop::sample::select(ks4.clone()).boxed()
                },
            ]);
            prop::collection::vec(
                (
                    any_slice.clone(),
                    any_slice.clone(),
                    prop::option::of("[a-z ]{3,20}"),
                ),
                0..=3,
            )
            .prop_map(|entries| {
                entries
                    .into_iter()
                    .map(|(before, after, reason)| MustOrderOverride {
                        before,
                        after,
                        reason,
                    })
                    .collect::<Vec<_>>()
            })
        };

        (
            must_link_strat,
            force_members_strat,
            rename_slices_strat,
            must_order_strat,
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

    // Feature: stackcut-v01-completion, Property 21: Override Validation Warnings
    // **Validates: Requirements 6.2, 6.3, 6.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_override_validation_warnings(
            known_unit_ids in prop::collection::btree_set(arb_id(), 1..=10),
            known_slice_ids in prop::collection::btree_set(arb_id(), 1..=5),
        ) {
            // Use a seeded strategy to generate overrides based on the known IDs
            let unit_ids = known_unit_ids.clone();
            let slice_ids = known_slice_ids.clone();

            // We need to run the inner strategy — use a TestRunner for the nested generation
            let mut runner = proptest::test_runner::TestRunner::default();
            let overrides_strategy = arb_overrides_with_invalids(
                known_unit_ids.clone(),
                known_slice_ids.clone(),
            );
            let overrides = overrides_strategy
                .new_tree(&mut runner)
                .unwrap()
                .current();

            let diagnostics = validate_overrides(&overrides, &unit_ids, &slice_ids);

            // Compute expected counts manually

            // 1. Count unknown member refs in must_link
            let mut expected_unknown_members = 0usize;
            for rule in &overrides.must_link {
                for member in &rule.members {
                    if !unit_ids.contains(member) {
                        expected_unknown_members += 1;
                    }
                }
            }

            // 2. Count unknown member refs in force_members
            for rule in &overrides.force_members {
                if !unit_ids.contains(&rule.member) {
                    expected_unknown_members += 1;
                }
            }

            // 3. Count unknown slice refs in rename_slices
            let mut expected_unknown_slices = 0usize;
            for rule in &overrides.rename_slices {
                if !slice_ids.contains(&rule.id) {
                    expected_unknown_slices += 1;
                }
            }

            // 4. Count unknown slice refs in must_order
            for rule in &overrides.must_order {
                if !slice_ids.contains(&rule.before) {
                    expected_unknown_slices += 1;
                }
                if !slice_ids.contains(&rule.after) {
                    expected_unknown_slices += 1;
                }
            }

            // 5. Count must_link groups with < 2 members
            let expected_too_few = overrides
                .must_link
                .iter()
                .filter(|rule| rule.members.len() < 2)
                .count();

            // Verify diagnostic counts
            let actual_unknown_members = diagnostics
                .iter()
                .filter(|d| d.code == "unknown-override-member")
                .count();
            let actual_unknown_slices = diagnostics
                .iter()
                .filter(|d| d.code == "unknown-override-slice")
                .count();
            let actual_too_few = diagnostics
                .iter()
                .filter(|d| d.code == "must-link-too-few")
                .count();

            prop_assert_eq!(
                actual_unknown_members,
                expected_unknown_members,
                "unknown-override-member count mismatch"
            );
            prop_assert_eq!(
                actual_unknown_slices,
                expected_unknown_slices,
                "unknown-override-slice count mismatch"
            );
            prop_assert_eq!(
                actual_too_few,
                expected_too_few,
                "must-link-too-few count mismatch"
            );

            // All diagnostics should be warnings
            for diag in &diagnostics {
                prop_assert_eq!(
                    &diag.level,
                    &DiagnosticLevel::Warning,
                    "All override validation diagnostics should be warnings, got {:?} for code {}",
                    diag.level,
                    diag.code
                );
            }

            // Total diagnostics should match sum of all expected
            let expected_total = expected_unknown_members + expected_unknown_slices + expected_too_few;
            prop_assert_eq!(
                diagnostics.len(),
                expected_total,
                "Total diagnostic count mismatch"
            );
        }
    }

    #[test]
    fn must_order_cycle_is_rejected() {
        // Create two slices: A depends on B, then try to add B depends on A
        let mut slices = vec![
            new_slice(
                "a",
                "Slice A",
                SliceKind::Behavior,
                vec![],
                vec!["path:a".to_string()],
                vec!["b".to_string()],
                Vec::new(),
            ),
            new_slice(
                "b",
                "Slice B",
                SliceKind::Behavior,
                vec![],
                vec!["path:b".to_string()],
                Vec::new(),
                Vec::new(),
            ),
        ];
        let overrides = Overrides {
            must_order: vec![MustOrderOverride {
                before: "a".to_string(),
                after: "b".to_string(),
                reason: Some("test cycle".to_string()),
            }],
            ..Overrides::default()
        };
        let diags = apply_overrides(&mut slices, &overrides);
        // The edge should be rejected
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "override-cycle");
        assert!(diags[0].message.contains("a -> b"));
        // The edge should NOT be in depends_on
        let slice_b = slices.iter().find(|s| s.id == "b").unwrap();
        assert!(!slice_b.depends_on.contains(&"a".to_string()));
        // No override-must-order reason should be added
        assert!(!slice_b
            .reasons
            .iter()
            .any(|r| r.code == "override-must-order"));
    }

    #[test]
    fn must_order_non_cycle_is_accepted() {
        let mut slices = vec![
            new_slice(
                "a",
                "Slice A",
                SliceKind::Behavior,
                vec![],
                vec!["path:a".to_string()],
                Vec::new(),
                Vec::new(),
            ),
            new_slice(
                "b",
                "Slice B",
                SliceKind::Behavior,
                vec![],
                vec!["path:b".to_string()],
                Vec::new(),
                Vec::new(),
            ),
        ];
        let overrides = Overrides {
            must_order: vec![MustOrderOverride {
                before: "a".to_string(),
                after: "b".to_string(),
                reason: None,
            }],
            ..Overrides::default()
        };
        let diags = apply_overrides(&mut slices, &overrides);
        assert!(diags.is_empty());
        let slice_b = slices.iter().find(|s| s.id == "b").unwrap();
        assert!(slice_b.depends_on.contains(&"a".to_string()));
        assert!(slice_b
            .reasons
            .iter()
            .any(|r| r.code == "override-must-order"));
    }

    #[test]
    fn must_order_cycle_in_plan_emits_diagnostic() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![
            unit(
                "path:src/core/a.rs",
                "src/core/a.rs",
                UnitKind::Behavior,
                "core",
            ),
            unit(
                "path:src/git/b.rs",
                "src/git/b.rs",
                UnitKind::Behavior,
                "git",
            ),
        ];
        // The planner will create behavior-core and behavior-git.
        // Add a must_order that creates a cycle: behavior-git -> behavior-core -> behavior-git
        let overrides = Overrides {
            must_order: vec![
                MustOrderOverride {
                    before: "behavior-git".to_string(),
                    after: "behavior-core".to_string(),
                    reason: None,
                },
                MustOrderOverride {
                    before: "behavior-core".to_string(),
                    after: "behavior-git".to_string(),
                    reason: None,
                },
            ],
            ..Overrides::default()
        };
        let result = plan(source, units, &StackcutConfig::default(), &overrides);
        // The second edge should be rejected as a cycle
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "override-cycle"),
            "Expected override-cycle diagnostic, got: {:?}",
            result.diagnostics
        );
        // The first edge should be accepted
        let core_slice = result
            .slices
            .iter()
            .find(|s| s.id == "behavior-core")
            .unwrap();
        assert!(core_slice.depends_on.contains(&"behavior-git".to_string()));
        // The second edge should NOT be present
        let git_slice = result
            .slices
            .iter()
            .find(|s| s.id == "behavior-git")
            .unwrap();
        assert!(!git_slice.depends_on.contains(&"behavior-core".to_string()));
    }

    #[test]
    fn unsupported_notes_emit_warning_diagnostics() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![
            EditUnit {
                id: "path:image.png".to_string(),
                path: "image.png".to_string(),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: UnitKind::Behavior,
                family: "root".to_string(),
                notes: vec!["unsupported-binary".to_string()],
            },
            EditUnit {
                id: "path:.gitmodules".to_string(),
                path: ".gitmodules".to_string(),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: UnitKind::Behavior,
                family: "root".to_string(),
                notes: vec!["unsupported-submodule".to_string()],
            },
        ];
        let result = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );

        // Both unsupported notes should produce warning diagnostics
        let unsupported_diags: Vec<&Diagnostic> = result
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with("unsupported-"))
            .collect();
        assert_eq!(unsupported_diags.len(), 2);
        assert!(unsupported_diags
            .iter()
            .any(|d| d.code == "unsupported-binary"));
        assert!(unsupported_diags
            .iter()
            .any(|d| d.code == "unsupported-submodule"));

        // All units should be assigned to some slice (not dropped)
        let all_members: Vec<String> = result
            .slices
            .iter()
            .flat_map(|s| s.members.clone())
            .collect();
        assert!(all_members.contains(&"path:image.png".to_string()));
        assert!(all_members.contains(&"path:.gitmodules".to_string()));
    }

    #[test]
    fn unsupported_unit_assigned_to_behavior_still_gets_diagnostic() {
        let source = PlanSource {
            repo_root: None,
            base: "base".to_string(),
            head: "head".to_string(),
            head_tree: None,
        };
        let units = vec![EditUnit {
            id: "path:src/core/lib.rs".to_string(),
            path: "src/core/lib.rs".to_string(),
            old_path: None,
            status: ChangeStatus::Modified,
            kind: UnitKind::Behavior,
            family: "core".to_string(),
            notes: vec!["unsupported-symlink".to_string()],
        }];
        let result = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );

        // The unit should be in a behavior slice
        let behavior_slice = result.slices.iter().find(|s| s.id == "behavior-core");
        assert!(behavior_slice.is_some());
        assert!(behavior_slice
            .unwrap()
            .members
            .contains(&"path:src/core/lib.rs".to_string()));

        // The unsupported diagnostic should still be emitted
        assert!(result
            .diagnostics
            .iter()
            .any(|d| d.code == "unsupported-symlink"));
    }

    // ---------------------------------------------------------------
    // Helper: build a plan from behavior units across distinct families
    // so that we get predictable slices to test overrides against.
    // ---------------------------------------------------------------

    /// Create a minimal plan with N behavior families, each having one unit.
    /// Returns (plan_produced_by_planner, unit_ids, slice_ids).
    fn make_plan_with_families(family_count: usize) -> (Plan, Vec<String>, Vec<String>) {
        let families: Vec<String> = (0..family_count).map(|i| format!("fam{}", i)).collect();
        let units: Vec<EditUnit> = families
            .iter()
            .map(|fam| EditUnit {
                id: format!("path:src/{}/mod.rs", fam),
                path: format!("src/{}/mod.rs", fam),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: UnitKind::Behavior,
                family: fam.clone(),
                notes: Vec::new(),
            })
            .collect();
        let source = PlanSource {
            repo_root: None,
            base: "aaa".to_string(),
            head: "bbb".to_string(),
            head_tree: None,
        };
        let p = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );
        let unit_ids: Vec<String> = p.units.iter().map(|u| u.id.clone()).collect();
        let slice_ids: Vec<String> = p.slices.iter().map(|s| s.id.clone()).collect();
        (p, unit_ids, slice_ids)
    }

    /// Generate a family count between 2 and 6 for property tests.
    fn arb_family_count() -> impl Strategy<Value = usize> {
        2..=6usize
    }

    // Feature: stackcut-v01-completion, Property 8: Override Idempotence
    // **Validates: Requirements 11.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_override_idempotence(family_count in arb_family_count()) {
            let (base_plan, unit_ids, slice_ids) = make_plan_with_families(family_count);

            // Build overrides that reference existing members/slices
            let first_unit = &unit_ids[0];
            let second_unit = &unit_ids[1];
            let first_slice = &slice_ids[0];
            let second_slice = if slice_ids.len() > 1 { &slice_ids[1] } else { &slice_ids[0] };

            let overrides = Overrides {
                must_link: vec![MustLinkOverride {
                    members: vec![first_unit.clone(), second_unit.clone()],
                    reason: Some("link them".to_string()),
                }],
                force_members: vec![],
                rename_slices: vec![RenameSliceOverride {
                    id: first_slice.clone(),
                    title: "Renamed Slice".to_string(),
                }],
                must_order: if first_slice != second_slice {
                    vec![MustOrderOverride {
                        before: first_slice.clone(),
                        after: second_slice.clone(),
                        reason: None,
                    }]
                } else {
                    vec![]
                },
                ..Overrides::default()
            };

            // Apply once
            let mut slices_once = base_plan.slices.clone();
            let _diags1 = apply_overrides(&mut slices_once, &overrides);

            // Apply twice (on the result of the first application)
            let mut slices_twice = slices_once.clone();
            let _diags2 = apply_overrides(&mut slices_twice, &overrides);

            // Compare: after second application, slices should be identical
            prop_assert_eq!(
                slices_once.len(),
                slices_twice.len(),
                "Slice count differs after second override application"
            );
            for (s1, s2) in slices_once.iter().zip(slices_twice.iter()) {
                prop_assert_eq!(&s1.id, &s2.id, "Slice ID mismatch");
                prop_assert_eq!(&s1.title, &s2.title, "Title mismatch for slice {}", s1.id);
                prop_assert_eq!(&s1.members, &s2.members, "Members mismatch for slice {}", s1.id);
                prop_assert_eq!(&s1.depends_on, &s2.depends_on, "depends_on mismatch for slice {}", s1.id);
            }
        }
    }

    // Feature: stackcut-v01-completion, Property 9: must_link Consolidation
    // **Validates: Requirements 7.1, 7.2, 7.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_must_link_consolidation(family_count in arb_family_count()) {
            let (base_plan, unit_ids, _slice_ids) = make_plan_with_families(family_count);

            // Pick the first two units (which are in different slices since they have different families)
            let member_a = &unit_ids[0];
            let member_b = &unit_ids[1];

            let overrides = Overrides {
                must_link: vec![MustLinkOverride {
                    members: vec![member_a.clone(), member_b.clone()],
                    reason: Some("consolidation test".to_string()),
                }],
                ..Overrides::default()
            };

            let mut slices = base_plan.slices.clone();
            let _diags = apply_overrides(&mut slices, &overrides);

            // Find which slice contains member_a
            let slice_for_a = slices
                .iter()
                .find(|s| s.members.contains(member_a))
                .map(|s| s.id.clone());
            // Find which slice contains member_b
            let slice_for_b = slices
                .iter()
                .find(|s| s.members.contains(member_b))
                .map(|s| s.id.clone());

            prop_assert!(slice_for_a.is_some(), "member_a should be in some slice");
            prop_assert!(slice_for_b.is_some(), "member_b should be in some slice");
            prop_assert_eq!(
                slice_for_a.as_ref().unwrap(),
                slice_for_b.as_ref().unwrap(),
                "Both must_link members should be in the same slice"
            );

            // The anchor slice should have an override-must-link reason
            let anchor = slices
                .iter()
                .find(|s| s.id == *slice_for_a.as_ref().unwrap())
                .unwrap();
            prop_assert!(
                anchor.reasons.iter().any(|r| r.code == "override-must-link"),
                "Anchor slice should have override-must-link reason, got: {:?}",
                anchor.reasons
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 10: force_members Placement
    // **Validates: Requirements 8.1, 8.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_force_members_placement(family_count in arb_family_count()) {
            let (base_plan, unit_ids, slice_ids) = make_plan_with_families(family_count);

            // Pick a unit and force it into a different slice
            let member = &unit_ids[0];
            // Find the slice that currently holds this member
            let current_slice = base_plan
                .slices
                .iter()
                .find(|s| s.members.contains(member))
                .map(|s| s.id.clone())
                .unwrap();
            // Pick a different target slice
            let target_slice = slice_ids
                .iter()
                .find(|s| **s != current_slice)
                .unwrap_or(&current_slice)
                .clone();

            let overrides = Overrides {
                force_members: vec![ForceMemberOverride {
                    member: member.clone(),
                    slice: target_slice.clone(),
                    reason: Some("force test".to_string()),
                }],
                ..Overrides::default()
            };

            let mut slices = base_plan.slices.clone();
            let _diags = apply_overrides(&mut slices, &overrides);

            // The member should appear in the target slice
            let target = slices.iter().find(|s| s.id == target_slice).unwrap();
            prop_assert!(
                target.members.contains(member),
                "Member {} should be in target slice {}",
                member,
                target_slice
            );

            // The member should NOT appear in any other slice
            for s in &slices {
                if s.id != target_slice {
                    prop_assert!(
                        !s.members.contains(member),
                        "Member {} should not be in slice {} (only in {})",
                        member,
                        s.id,
                        target_slice
                    );
                }
            }

            // The target slice should have an override-force-member reason
            prop_assert!(
                target.reasons.iter().any(|r| r.code == "override-force-member"),
                "Target slice should have override-force-member reason"
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 11: rename_slices Title Update
    // **Validates: Requirements 9.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_rename_slices_title_update(
            family_count in arb_family_count(),
            new_title in "[A-Z][a-z ]{2,20}"
        ) {
            let (base_plan, _unit_ids, slice_ids) = make_plan_with_families(family_count);

            // Pick the first slice to rename
            let target_id = &slice_ids[0];

            let overrides = Overrides {
                rename_slices: vec![RenameSliceOverride {
                    id: target_id.clone(),
                    title: new_title.clone(),
                }],
                ..Overrides::default()
            };

            let mut slices = base_plan.slices.clone();
            let _diags = apply_overrides(&mut slices, &overrides);

            let renamed = slices.iter().find(|s| s.id == *target_id).unwrap();
            prop_assert_eq!(
                &renamed.title,
                &new_title,
                "Slice title should be updated to the override value"
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 12: must_order Edge Addition
    // **Validates: Requirements 10.1, 10.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_must_order_edge_addition(family_count in 2..=6usize) {
            let (base_plan, _unit_ids, slice_ids) = make_plan_with_families(family_count);

            // Pick two distinct slices where adding before -> after won't create a cycle
            let before_id = &slice_ids[0];
            let after_id = &slice_ids[1];

            // Verify no pre-existing edge from after to before (which would make our
            // edge cyclic). The planner with distinct families produces independent slices
            // so this should be safe.
            let after_slice_before = base_plan
                .slices
                .iter()
                .find(|s| s.id == *after_id)
                .unwrap();
            let would_cycle = after_slice_before.depends_on.contains(before_id)
                || {
                    // Quick check: if before already depends on after, adding after->before
                    // would create a cycle. But we're adding after depends_on before, so
                    // check if before depends on after.
                    let before_slice = base_plan
                        .slices
                        .iter()
                        .find(|s| s.id == *before_id)
                        .unwrap();
                    before_slice.depends_on.contains(after_id)
                };

            // Skip if it would cycle (shouldn't happen with independent families)
            prop_assume!(!would_cycle);

            let overrides = Overrides {
                must_order: vec![MustOrderOverride {
                    before: before_id.clone(),
                    after: after_id.clone(),
                    reason: Some("ordering test".to_string()),
                }],
                ..Overrides::default()
            };

            let mut slices = base_plan.slices.clone();
            let diags = apply_overrides(&mut slices, &overrides);

            // No cycle diagnostic should be emitted
            prop_assert!(
                !diags.iter().any(|d| d.code == "override-cycle"),
                "Should not produce a cycle diagnostic for a valid edge"
            );

            // The after slice should now depend on the before slice
            let after_slice = slices.iter().find(|s| s.id == *after_id).unwrap();
            prop_assert!(
                after_slice.depends_on.contains(before_id),
                "after slice should depend on before slice"
            );

            // The after slice should have an override-must-order reason
            prop_assert!(
                after_slice.reasons.iter().any(|r| r.code == "override-must-order"),
                "after slice should have override-must-order reason"
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 13: prep-refactor vs Mechanical Kind
    // **Validates: Requirements 14.1, 14.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_prep_refactor_vs_mechanical_kind(
            all_renames in proptest::bool::ANY,
            mech_count in 1..=5usize,
        ) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            let mut units: Vec<EditUnit> = Vec::new();

            // Always include at least one Behavior unit so the plan has a behavior slice
            units.push(EditUnit {
                id: "path:src/core/main.rs".to_string(),
                path: "src/core/main.rs".to_string(),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: UnitKind::Behavior,
                family: "core".to_string(),
                notes: Vec::new(),
            });

            if all_renames {
                // All mechanical units are renames
                for i in 0..mech_count {
                    units.push(EditUnit {
                        id: format!("path:src/core/old{}.rs", i),
                        path: format!("src/core/new{}.rs", i),
                        old_path: Some(format!("src/core/old{}.rs", i)),
                        status: ChangeStatus::Renamed,
                        kind: UnitKind::Mechanical,
                        family: "core".to_string(),
                        notes: Vec::new(),
                    });
                }
            } else {
                // At least one Copied unit, rest are Renamed
                units.push(EditUnit {
                    id: "path:src/core/copied.rs".to_string(),
                    path: "src/core/copied.rs".to_string(),
                    old_path: Some("src/core/original.rs".to_string()),
                    status: ChangeStatus::Copied,
                    kind: UnitKind::Mechanical,
                    family: "core".to_string(),
                    notes: Vec::new(),
                });
                for i in 0..(mech_count.saturating_sub(1)) {
                    units.push(EditUnit {
                        id: format!("path:src/core/rold{}.rs", i),
                        path: format!("src/core/rnew{}.rs", i),
                        old_path: Some(format!("src/core/rold{}.rs", i)),
                        status: ChangeStatus::Renamed,
                        kind: UnitKind::Mechanical,
                        family: "core".to_string(),
                        notes: Vec::new(),
                    });
                }
            }

            let result = plan(
                source,
                units,
                &StackcutConfig::default(),
                &Overrides::default(),
            );

            // Find the mechanical-renames slice
            let mech_slice = result
                .slices
                .iter()
                .find(|s| s.id == "mechanical-renames");

            prop_assert!(
                mech_slice.is_some(),
                "Plan should contain a mechanical-renames slice"
            );
            let mech_slice = mech_slice.unwrap();

            if all_renames {
                prop_assert_eq!(
                    &mech_slice.kind,
                    &SliceKind::PrepRefactor,
                    "When all mechanical members are renames, kind should be PrepRefactor"
                );
            } else {
                prop_assert_eq!(
                    &mech_slice.kind,
                    &SliceKind::Mechanical,
                    "When any mechanical member is not a rename, kind should be Mechanical"
                );
            }
        }
    }

    // Feature: stackcut-v01-completion, Property 14: Review Budget Diagnostic
    // **Validates: Requirements 15.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_review_budget_diagnostic(
            budget in 1u32..=5,
            extra in 1u32..=10,
        ) {
            let member_count = (budget + extra) as usize;

            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            // Generate `member_count` behavior units in the same family
            let units: Vec<EditUnit> = (0..member_count)
                .map(|i| EditUnit {
                    id: format!("path:src/core/file{}.rs", i),
                    path: format!("src/core/file{}.rs", i),
                    old_path: None,
                    status: ChangeStatus::Modified,
                    kind: UnitKind::Behavior,
                    family: "core".to_string(),
                    notes: Vec::new(),
                })
                .collect();

            let config = StackcutConfig {
                review_budget: Some(budget),
                ..StackcutConfig::default()
            };

            let result = plan(source, units, &config, &Overrides::default());

            // The single behavior slice has `member_count` members which exceeds `budget`
            let budget_diags: Vec<_> = result
                .diagnostics
                .iter()
                .filter(|d| d.code == "review-budget-exceeded")
                .collect();

            prop_assert!(
                !budget_diags.is_empty(),
                "Expected at least one review-budget-exceeded diagnostic for {} members with budget {}",
                member_count,
                budget
            );

            // The diagnostic should be a warning
            for diag in &budget_diags {
                prop_assert_eq!(
                    &diag.level,
                    &DiagnosticLevel::Warning,
                    "review-budget-exceeded should be a warning"
                );
            }

            // The diagnostic should name the oversized slice
            let behavior_slice = result
                .slices
                .iter()
                .find(|s| s.id == "behavior-core");
            prop_assert!(behavior_slice.is_some(), "Should have a behavior-core slice");

            prop_assert!(
                budget_diags.iter().any(|d| d.message.contains("behavior-core")),
                "Diagnostic should name the oversized slice 'behavior-core'"
            );
        }
    }

    /// Strategy that picks one of the three attachable UnitKind values.
    fn arb_attachable_kind() -> impl Strategy<Value = UnitKind> {
        prop_oneof![
            Just(UnitKind::Test),
            Just(UnitKind::Documentation),
            Just(UnitKind::Generated),
        ]
    }

    /// Build a path for an attachable unit that:
    ///  - classifies as the given `kind` under the default config, AND
    ///  - contains `family_name` as a path segment, BUT
    ///  - `infer_family` returns something OTHER than `family_name`.
    ///
    /// This forces the planner through the `infer_owner_by_path_segment` path.
    fn attachable_path_for(kind: &UnitKind, family_name: &str) -> String {
        match kind {
            // generated/<family>/output.snap → Generated kind, family = "generated"
            UnitKind::Generated => format!("generated/{}/output.snap", family_name),
            // vendor/<family>/tests/check.rs → Test kind (contains /tests/), family = "vendor"
            UnitKind::Test => format!("vendor/{}/tests/check.rs", family_name),
            // vendor/<family>/notes/guide.md → Documentation kind (.md), family = "vendor"
            UnitKind::Documentation => format!("vendor/{}/notes/guide.md", family_name),
            _ => unreachable!(),
        }
    }

    // Feature: stackcut-v01-completion, Property 15: Ownership Inference Attachment
    // **Validates: Requirements 16.1, 16.2, 16.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_ownership_inference_attachment(
            family_name in "[a-z]{4,8}",
            attachable_kind in arb_attachable_kind(),
        ) {
            let config = StackcutConfig::default();

            // 1. Create a behavior unit in the generated family
            let behavior_path = format!("src/{}/main.rs", family_name);
            let behavior_id = format!("path:{}", behavior_path);
            let behavior_family = infer_family(&behavior_path, &config);
            // Sanity: behavior_family should equal family_name via ["src", family, ..] pattern
            prop_assert_eq!(
                &behavior_family, &family_name,
                "Behavior unit family should match the generated family name"
            );

            let behavior_unit = EditUnit {
                id: behavior_id.clone(),
                path: behavior_path,
                old_path: None,
                status: ChangeStatus::Modified,
                kind: UnitKind::Behavior,
                family: behavior_family,
                notes: Vec::new(),
            };

            // 2. Create an attachable unit whose path contains family_name as a segment
            //    but whose inferred family is different
            let attach_path = attachable_path_for(&attachable_kind, &family_name);
            let attach_id = format!("path:{}", attach_path);
            let attach_family = infer_family(&attach_path, &config);
            let attach_kind = classify_path(&attach_path, &ChangeStatus::Modified, &config);

            // Precondition: the attachable unit's family must NOT equal family_name
            // (otherwise it would be attached via direct family lookup, not path-segment inference)
            prop_assume!(attach_family != family_name);

            // Precondition: the unit must classify as the expected attachable kind
            prop_assert!(
                matches!(attach_kind, UnitKind::Test | UnitKind::Documentation | UnitKind::Generated),
                "Attachable path should classify as test/doc/generated, got {:?}",
                attach_kind
            );

            let attachable_unit = EditUnit {
                id: attach_id.clone(),
                path: attach_path,
                old_path: None,
                status: ChangeStatus::Modified,
                kind: attach_kind,
                family: attach_family,
                notes: Vec::new(),
            };

            // 3. Run the planner
            let source = PlanSource {
                repo_root: None,
                base: "aaa".to_string(),
                head: "bbb".to_string(),
                head_tree: None,
            };
            let result = plan(
                source,
                vec![behavior_unit, attachable_unit],
                &config,
                &Overrides::default(),
            );

            // 4. The attachable unit should be in the behavior slice, not standalone
            let behavior_slice_id = format!("behavior-{}", family_name);
            let behavior_slice = result
                .slices
                .iter()
                .find(|s| s.id == behavior_slice_id);

            prop_assert!(
                behavior_slice.is_some(),
                "Plan should contain a behavior slice for family '{}'",
                family_name
            );
            let behavior_slice = behavior_slice.unwrap();

            prop_assert!(
                behavior_slice.members.contains(&attach_id),
                "Attachable unit '{}' should be in behavior slice '{}', but members are: {:?}",
                attach_id,
                behavior_slice_id,
                behavior_slice.members
            );

            // 5. The attachable unit should NOT be in any standalone slice
            for slice in &result.slices {
                if slice.id != behavior_slice_id {
                    prop_assert!(
                        !slice.members.contains(&attach_id),
                        "Attachable unit '{}' should not be in standalone slice '{}', only in '{}'",
                        attach_id,
                        slice.id,
                        behavior_slice_id
                    );
                }
            }
        }
    }

    fn arb_unsupported_notes() -> impl Strategy<Value = Vec<String>> {
        let all_notes = vec![
            "unsupported-binary".to_string(),
            "unsupported-submodule".to_string(),
            "unsupported-symlink".to_string(),
            "unsupported-mode-only".to_string(),
        ];
        proptest::sample::subsequence(all_notes, 1..=4)
    }

    // ---------------------------------------------------------------
    // Proptest Arbitrary generators for planner invariant tests
    // (Task 13.2)
    // ---------------------------------------------------------------

    /// Strategy for a random ChangeStatus.
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

    /// Strategy for a random UnitKind.
    fn arb_unit_kind() -> impl Strategy<Value = UnitKind> {
        prop_oneof![
            Just(UnitKind::Manifest),
            Just(UnitKind::Lockfile),
            Just(UnitKind::Generated),
            Just(UnitKind::Test),
            Just(UnitKind::Documentation),
            Just(UnitKind::OpsConfig),
            Just(UnitKind::Mechanical),
            Just(UnitKind::Behavior),
        ]
    }

    /// Strategy for a random family name (short lowercase identifier).
    fn arb_family() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("core".to_string()),
            Just("git".to_string()),
            Just("cli".to_string()),
            Just("artifact".to_string()),
            Just("auth".to_string()),
            Just("api".to_string()),
            Just("root".to_string()),
        ]
    }

    /// Generate a single random EditUnit with a given index for uniqueness.
    /// The id and path are derived from the index and family to guarantee uniqueness
    /// when used with `arb_edit_units`.
    pub(crate) fn arb_edit_unit() -> impl Strategy<Value = EditUnit> {
        (
            arb_change_status(),
            arb_unit_kind(),
            arb_family(),
            0..1000u32,
        )
            .prop_map(|(status, kind, family, idx)| {
                let path = format!("src/{}/file{}.rs", family, idx);
                let id = format!("path:{}", path);
                let old_path = if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
                    Some(format!("src/{}/old_file{}.rs", family, idx))
                } else {
                    None
                };
                // Ensure kind is consistent with status for mechanical units
                let kind = if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
                    UnitKind::Mechanical
                } else {
                    kind
                };
                EditUnit {
                    id,
                    path,
                    old_path,
                    status,
                    kind,
                    family,
                    notes: Vec::new(),
                }
            })
    }

    /// Generate a Vec of 1–50 random EditUnits with unique IDs.
    /// Uses index-based IDs to guarantee uniqueness.
    pub(crate) fn arb_edit_units() -> impl Strategy<Value = Vec<EditUnit>> {
        (1..=50usize).prop_flat_map(|count| {
            proptest::collection::vec(
                (arb_change_status(), arb_unit_kind(), arb_family()),
                count..=count,
            )
            .prop_map(|items| {
                items
                    .into_iter()
                    .enumerate()
                    .map(|(i, (status, kind, family))| {
                        let path = format!("src/{}/file{}.rs", family, i);
                        let id = format!("path:{}", path);
                        let old_path =
                            if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied) {
                                Some(format!("src/{}/old_file{}.rs", family, i))
                            } else {
                                None
                            };
                        let kind = if matches!(status, ChangeStatus::Renamed | ChangeStatus::Copied)
                        {
                            UnitKind::Mechanical
                        } else {
                            kind
                        };
                        EditUnit {
                            id,
                            path,
                            old_path,
                            status,
                            kind,
                            family,
                            notes: Vec::new(),
                        }
                    })
                    .collect()
            })
        })
    }

    /// Generate random Overrides that reference existing unit and slice IDs.
    /// This produces valid overrides suitable for planner invariant tests.
    pub(crate) fn arb_overrides(
        unit_ids: Vec<String>,
        slice_ids: Vec<String>,
    ) -> impl Strategy<Value = Overrides> {
        let ui = unit_ids.clone();
        let si = slice_ids.clone();

        let must_link_strat = if unit_ids.len() >= 2 {
            let ui2 = unit_ids.clone();
            prop::collection::vec(
                proptest::sample::subsequence(ui2.clone(), 2..=ui2.len().min(4)).prop_map(
                    |members| MustLinkOverride {
                        members,
                        reason: Some("arb must_link".to_string()),
                    },
                ),
                0..=2,
            )
            .boxed()
        } else {
            Just(Vec::new()).boxed()
        };

        let force_strat = if !ui.is_empty() && !si.is_empty() {
            let ui3 = ui.clone();
            let si3 = si.clone();
            prop::collection::vec(
                (proptest::sample::select(ui3), proptest::sample::select(si3)).prop_map(
                    |(member, slice)| ForceMemberOverride {
                        member,
                        slice,
                        reason: Some("arb force".to_string()),
                    },
                ),
                0..=2,
            )
            .boxed()
        } else {
            Just(Vec::new()).boxed()
        };

        let rename_strat = if !si.is_empty() {
            let si4 = si.clone();
            prop::collection::vec(
                (proptest::sample::select(si4), "[A-Z][a-z ]{2,12}")
                    .prop_map(|(id, title)| RenameSliceOverride { id, title }),
                0..=2,
            )
            .boxed()
        } else {
            Just(Vec::new()).boxed()
        };

        let must_order_strat = if slice_ids.len() >= 2 {
            let si5 = slice_ids.clone();
            prop::collection::vec(
                (
                    proptest::sample::select(si5.clone()),
                    proptest::sample::select(si5),
                )
                    .prop_filter("before != after", |(a, b)| a != b)
                    .prop_map(|(before, after)| MustOrderOverride {
                        before,
                        after,
                        reason: Some("arb order".to_string()),
                    }),
                0..=2,
            )
            .boxed()
        } else {
            Just(Vec::new()).boxed()
        };

        (must_link_strat, force_strat, rename_strat, must_order_strat).prop_map(
            |(must_link, force_members, rename_slices, must_order)| Overrides {
                version: 1,
                must_link,
                force_members,
                rename_slices,
                must_order,
            },
        )
    }

    /// Generate a random StackcutConfig with valid fields.
    pub(crate) fn arb_config() -> impl Strategy<Value = StackcutConfig> {
        (
            // review_budget: None or 1..=50
            prop::option::of(1u32..=50),
            // generated_prefixes: 0-3 entries
            prop::collection::vec("(generated|dist|build|out)/", 0..=3),
            // manifest_files: 0-3 entries
            prop::collection::vec(
                prop_oneof![
                    Just("Cargo.toml".to_string()),
                    Just("package.json".to_string()),
                    Just("pyproject.toml".to_string()),
                ],
                0..=3,
            ),
            // lock_files: 0-3 entries
            prop::collection::vec(
                prop_oneof![
                    Just("Cargo.lock".to_string()),
                    Just("package-lock.json".to_string()),
                    Just("pnpm-lock.yaml".to_string()),
                ],
                0..=3,
            ),
            // test_prefixes: 0-2 entries
            prop::collection::vec(
                prop_oneof![Just("tests/".to_string()), Just("specs/".to_string()),],
                0..=2,
            ),
            // doc_prefixes: 0-2 entries
            prop::collection::vec(
                prop_oneof![Just("docs/".to_string()), Just("adr/".to_string()),],
                0..=2,
            ),
            // ops_prefixes: 0-2 entries
            prop::collection::vec(
                prop_oneof![
                    Just(".github/".to_string()),
                    Just("ci/".to_string()),
                    Just(".circleci/".to_string()),
                ],
                0..=2,
            ),
        )
            .prop_map(
                |(
                    review_budget,
                    generated_prefixes,
                    manifest_files,
                    lock_files,
                    test_prefixes,
                    doc_prefixes,
                    ops_prefixes,
                )| {
                    StackcutConfig {
                        version: 1,
                        generated_prefixes,
                        manifest_files,
                        lock_files,
                        test_prefixes,
                        doc_prefixes,
                        ops_prefixes,
                        path_families: vec![],
                        review_budget,
                    }
                },
            )
    }

    #[test]
    fn arb_generators_smoke_test() {
        // Verify each generator produces valid data via a quick proptest run
        let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig::with_cases(5));

        // arb_edit_unit produces a unit with non-empty id and path
        runner
            .run(&arb_edit_unit(), |unit| {
                prop_assert!(!unit.id.is_empty());
                prop_assert!(!unit.path.is_empty());
                prop_assert!(!unit.family.is_empty());
                if matches!(unit.status, ChangeStatus::Renamed | ChangeStatus::Copied) {
                    prop_assert!(unit.old_path.is_some());
                    prop_assert_eq!(unit.kind, UnitKind::Mechanical);
                }
                Ok(())
            })
            .unwrap();

        // arb_edit_units produces units with unique IDs
        runner
            .run(&arb_edit_units(), |units| {
                let ids: BTreeSet<String> = units.iter().map(|u| u.id.clone()).collect();
                prop_assert_eq!(ids.len(), units.len(), "IDs must be unique");
                prop_assert!(!units.is_empty());
                prop_assert!(units.len() <= 50);
                Ok(())
            })
            .unwrap();

        // arb_config produces a config with version 1
        runner
            .run(&arb_config(), |config| {
                prop_assert_eq!(config.version, 1);
                Ok(())
            })
            .unwrap();

        // arb_overrides with known IDs produces valid overrides
        let unit_ids = vec![
            "path:a".to_string(),
            "path:b".to_string(),
            "path:c".to_string(),
        ];
        let slice_ids = vec!["s1".to_string(), "s2".to_string()];
        runner
            .run(&arb_overrides(unit_ids, slice_ids), |ovr| {
                prop_assert_eq!(ovr.version, 1);
                Ok(())
            })
            .unwrap();
    }

    // Feature: stackcut-v01-completion, Property 16: Unsupported Surface Handling
    // **Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_unsupported_surface_handling(
            note_sets in proptest::collection::vec(arb_unsupported_notes(), 1..=4),
        ) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            // Build units: each gets a unique subset of unsupported notes
            let mut units: Vec<EditUnit> = Vec::new();
            let mut all_expected_notes: Vec<String> = Vec::new();

            for (i, notes) in note_sets.iter().enumerate() {
                let path = format!("src/unsupported_{}.bin", i);
                let id = format!("path:{}", path);
                all_expected_notes.extend(notes.clone());
                units.push(EditUnit {
                    id,
                    path,
                    old_path: None,
                    status: ChangeStatus::Modified,
                    kind: UnitKind::Behavior,
                    family: "core".to_string(),
                    notes: notes.clone(),
                });
            }

            let result = plan(
                source,
                units.clone(),
                &StackcutConfig::default(),
                &Overrides::default(),
            );

            // Assert 1: For each unsupported note on each unit, there's a matching warning diagnostic
            for unit in &units {
                for note in &unit.notes {
                    let has_diag = result.diagnostics.iter().any(|d| {
                        d.level == DiagnosticLevel::Warning
                            && d.code == *note
                            && d.message.contains(&unit.path)
                    });
                    prop_assert!(
                        has_diag,
                        "Expected warning diagnostic with code '{}' for path '{}', diagnostics: {:?}",
                        note,
                        unit.path,
                        result.diagnostics
                    );
                }
            }

            // Assert 2: Every unit is assigned to some slice (not dropped)
            let all_assigned_members: BTreeSet<String> = result
                .slices
                .iter()
                .flat_map(|s| s.members.iter().cloned())
                .collect();

            for unit in &units {
                prop_assert!(
                    all_assigned_members.contains(&unit.id),
                    "Unit '{}' should be assigned to a slice, but was not found in any slice members",
                    unit.id
                );
            }
        }
    }

    // Feature: stackcut-v01-completion, Property 4: No-Loss No-Duplication Invariant
    // **Validates: Requirements 21.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_no_loss_no_duplication(units in arb_edit_units()) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            let result = plan(
                source,
                units.clone(),
                &StackcutConfig::default(),
                &Overrides::default(),
            );

            let unit_ids: BTreeSet<String> = units.iter().map(|u| u.id.clone()).collect();

            // Count how many times each unit ID appears across all slices
            let mut member_counts: BTreeMap<String, usize> = BTreeMap::new();
            for slice in &result.slices {
                for member in &slice.members {
                    *member_counts.entry(member.clone()).or_insert(0) += 1;
                }
            }

            // Every unit ID must appear exactly once
            for uid in &unit_ids {
                let count = member_counts.get(uid).copied().unwrap_or(0);
                prop_assert_eq!(
                    count, 1,
                    "Unit '{}' should appear in exactly 1 slice, but appears in {}",
                    uid, count
                );
            }

            // No extra IDs in slices that aren't in the input units
            for member in member_counts.keys() {
                prop_assert!(
                    unit_ids.contains(member),
                    "Slice member '{}' is not in the input units",
                    member
                );
            }
        }
    }

    // Feature: stackcut-v01-completion, Property 5: Acyclic Dependency Graph
    // **Validates: Requirements 21.2**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_acyclic_dependency_graph(units in arb_edit_units()) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            let result = plan(
                source,
                units,
                &StackcutConfig::default(),
                &Overrides::default(),
            );

            // Topological sort via Kahn's algorithm — same approach as has_cycle
            let slice_count = result.slices.len();
            let mut incoming: BTreeMap<String, usize> = result
                .slices
                .iter()
                .map(|s| (s.id.clone(), 0usize))
                .collect();
            let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();

            for slice in &result.slices {
                for dep in &slice.depends_on {
                    *incoming.entry(slice.id.clone()).or_insert(0) += 1;
                    outgoing
                        .entry(dep.clone())
                        .or_default()
                        .push(slice.id.clone());
                }
            }

            let mut ready: Vec<String> = incoming
                .iter()
                .filter_map(|(node, count)| {
                    if *count == 0 { Some(node.clone()) } else { None }
                })
                .collect();
            ready.sort();

            let mut visited = 0usize;
            while let Some(node) = ready.pop() {
                visited += 1;
                if let Some(children) = outgoing.get(&node) {
                    for child in children {
                        if let Some(count) = incoming.get_mut(child) {
                            *count -= 1;
                            if *count == 0 {
                                ready.push(child.clone());
                                ready.sort();
                            }
                        }
                    }
                }
            }

            prop_assert_eq!(
                visited, slice_count,
                "Topological sort visited {} of {} slices — cycle detected",
                visited, slice_count
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 6: Planner Determinism
    // **Validates: Requirements 21.3**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_planner_determinism(
            units in arb_edit_units(),
            config in arb_config(),
        ) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };
            let overrides = Overrides::default();

            let plan1 = plan(
                source.clone(),
                units.clone(),
                &config,
                &overrides,
            );
            let plan2 = plan(
                source,
                units,
                &config,
                &overrides,
            );

            prop_assert_eq!(
                plan1, plan2,
                "Running the planner twice on the same input must produce identical Plans"
            );
        }
    }

    // Feature: stackcut-v01-completion, Property 7: Override Preserves No-Loss No-Duplication
    // **Validates: Requirements 21.4**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_override_preserves_no_loss_no_dup(units in arb_edit_units()) {
            let source = PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            };

            // First run with no overrides to get the plan's unit/slice IDs
            let initial = plan(
                source.clone(),
                units.clone(),
                &StackcutConfig::default(),
                &Overrides::default(),
            );

            let unit_ids: Vec<String> = initial.units.iter().map(|u| u.id.clone()).collect();
            let slice_ids: Vec<String> = initial.slices.iter().map(|s| s.id.clone()).collect();

            // Generate overrides referencing the plan's actual IDs
            let mut runner = proptest::test_runner::TestRunner::default();
            let overrides = arb_overrides(unit_ids.clone(), slice_ids)
                .new_tree(&mut runner)
                .unwrap()
                .current();

            // Run planner again with the generated overrides
            let result = plan(
                source,
                units,
                &StackcutConfig::default(),
                &overrides,
            );

            let unit_id_set: BTreeSet<String> = unit_ids.into_iter().collect();

            // Count how many times each unit ID appears across all slices
            let mut member_counts: BTreeMap<String, usize> = BTreeMap::new();
            for slice in &result.slices {
                for member in &slice.members {
                    *member_counts.entry(member.clone()).or_insert(0) += 1;
                }
            }

            // Every unit ID must appear exactly once
            for uid in &unit_id_set {
                let count = member_counts.get(uid).copied().unwrap_or(0);
                prop_assert_eq!(
                    count, 1,
                    "After overrides, unit '{}' should appear in exactly 1 slice, but appears in {}",
                    uid, count
                );
            }

            // No extra IDs in slices that aren't in the input units
            for member in member_counts.keys() {
                prop_assert!(
                    unit_id_set.contains(member),
                    "After overrides, slice member '{}' is not in the input units",
                    member
                );
            }
        }
    }

    // ── Fixture-driven golden tests (Task 15.1) ──────────────────────────
    // Validates: Requirements 20.1, 20.2, 20.3

    /// Compact representation of a slice for comparison (ignoring reasons/proof_surface).
    #[derive(Debug, PartialEq, Eq)]
    struct SliceSummary {
        id: String,
        kind: SliceKind,
        members: Vec<String>,
        depends_on: Vec<String>,
    }

    impl SliceSummary {
        fn from_slice(s: &Slice) -> Self {
            let mut members = s.members.clone();
            members.sort();
            let mut depends_on = s.depends_on.clone();
            depends_on.sort();
            Self {
                id: s.id.clone(),
                kind: s.kind.clone(),
                members,
                depends_on,
            }
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    struct AmbiguitySummary {
        id: String,
        affected_units: Vec<String>,
        candidate_slices: Vec<String>,
    }

    impl AmbiguitySummary {
        fn from_ambiguity(a: &Ambiguity) -> Self {
            let mut affected_units = a.affected_units.clone();
            affected_units.sort();
            let mut candidate_slices = a.candidate_slices.clone();
            candidate_slices.sort();
            Self {
                id: a.id.clone(),
                affected_units,
                candidate_slices,
            }
        }
    }

    fn run_golden_fixture(case_dir: &std::path::Path) {
        let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();

        // Load input units
        let input_path = case_dir.join("input.units.json");
        let input_json = std::fs::read_to_string(&input_path)
            .unwrap_or_else(|e| panic!("[{}] Failed to read {:?}: {}", case_name, input_path, e));
        let units: Vec<EditUnit> = serde_json::from_str(&input_json)
            .unwrap_or_else(|e| panic!("[{}] Failed to parse input.units.json: {}", case_name, e));

        // Load expected plan
        let expected_path = case_dir.join("expected.plan.json");
        let expected_json = std::fs::read_to_string(&expected_path).unwrap_or_else(|e| {
            panic!("[{}] Failed to read {:?}: {}", case_name, expected_path, e)
        });
        let expected_plan: Plan = serde_json::from_str(&expected_json).unwrap_or_else(|e| {
            panic!("[{}] Failed to parse expected.plan.json: {}", case_name, e)
        });

        // Run planner with default config and empty overrides
        let source = PlanSource {
            repo_root: None,
            base: "fixture-base".to_string(),
            head: "fixture-head".to_string(),
            head_tree: None,
        };
        let actual_plan = plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        );

        // Compare slice count
        assert_eq!(
            actual_plan.slices.len(),
            expected_plan.slices.len(),
            "[{}] Slice count mismatch: actual {} vs expected {}",
            case_name,
            actual_plan.slices.len(),
            expected_plan.slices.len()
        );

        // Compare slice summaries (id, kind, members, depends_on)
        let mut actual_summaries: Vec<SliceSummary> = actual_plan
            .slices
            .iter()
            .map(SliceSummary::from_slice)
            .collect();
        actual_summaries.sort_by(|a, b| a.id.cmp(&b.id));

        let mut expected_summaries: Vec<SliceSummary> = expected_plan
            .slices
            .iter()
            .map(SliceSummary::from_slice)
            .collect();
        expected_summaries.sort_by(|a, b| a.id.cmp(&b.id));

        for (actual, expected) in actual_summaries.iter().zip(expected_summaries.iter()) {
            assert_eq!(
                actual.id, expected.id,
                "[{}] Slice ID mismatch: actual {:?} vs expected {:?}",
                case_name, actual.id, expected.id
            );
            assert_eq!(
                actual.kind, expected.kind,
                "[{}] Slice '{}' kind mismatch: actual {:?} vs expected {:?}",
                case_name, actual.id, actual.kind, expected.kind
            );
            assert_eq!(
                actual.members, expected.members,
                "[{}] Slice '{}' members mismatch:\n  actual:   {:?}\n  expected: {:?}",
                case_name, actual.id, actual.members, expected.members
            );
            assert_eq!(
                actual.depends_on, expected.depends_on,
                "[{}] Slice '{}' depends_on mismatch:\n  actual:   {:?}\n  expected: {:?}",
                case_name, actual.id, actual.depends_on, expected.depends_on
            );
        }

        // Compare ambiguity summaries
        let mut actual_ambiguities: Vec<AmbiguitySummary> = actual_plan
            .ambiguities
            .iter()
            .map(AmbiguitySummary::from_ambiguity)
            .collect();
        actual_ambiguities.sort_by(|a, b| a.id.cmp(&b.id));

        let mut expected_ambiguities: Vec<AmbiguitySummary> = expected_plan
            .ambiguities
            .iter()
            .map(AmbiguitySummary::from_ambiguity)
            .collect();
        expected_ambiguities.sort_by(|a, b| a.id.cmp(&b.id));

        assert_eq!(
            actual_ambiguities.len(),
            expected_ambiguities.len(),
            "[{}] Ambiguity count mismatch: actual {} vs expected {}",
            case_name,
            actual_ambiguities.len(),
            expected_ambiguities.len()
        );

        for (actual, expected) in actual_ambiguities.iter().zip(expected_ambiguities.iter()) {
            assert_eq!(
                actual.id, expected.id,
                "[{}] Ambiguity ID mismatch: actual {:?} vs expected {:?}",
                case_name, actual.id, expected.id
            );
            assert_eq!(
                actual.affected_units, expected.affected_units,
                "[{}] Ambiguity '{}' affected_units mismatch:\n  actual:   {:?}\n  expected: {:?}",
                case_name, actual.id, actual.affected_units, expected.affected_units
            );
            assert_eq!(
                actual.candidate_slices, expected.candidate_slices,
                "[{}] Ambiguity '{}' candidate_slices mismatch:\n  actual:   {:?}\n  expected: {:?}",
                case_name, actual.id, actual.candidate_slices, expected.candidate_slices
            );
        }
    }

    #[test]
    fn golden_fixture_tests() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // workspace root is two levels up from crates/stackcut-core/
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root");
        let fixtures_dir = workspace_root.join("fixtures").join("cases");

        assert!(
            fixtures_dir.exists(),
            "Fixtures directory not found: {:?}",
            fixtures_dir
        );

        let mut case_dirs: Vec<std::path::PathBuf> = std::fs::read_dir(&fixtures_dir)
            .expect("Failed to read fixtures/cases/")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir()
                    && path.join("input.units.json").exists()
                    && path.join("expected.plan.json").exists()
                {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        case_dirs.sort();

        assert!(
            !case_dirs.is_empty(),
            "No fixture cases found in {:?}",
            fixtures_dir
        );

        let mut failures: Vec<String> = Vec::new();
        for case_dir in &case_dirs {
            let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();
            let result = std::panic::catch_unwind(|| {
                run_golden_fixture(case_dir);
            });
            if let Err(e) = result {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };
                failures.push(format!("[{}] {}", case_name, msg));
            }
        }

        if !failures.is_empty() {
            panic!(
                "Golden fixture failures ({}/{}):\n{}",
                failures.len(),
                case_dirs.len(),
                failures.join("\n\n")
            );
        }
    }

    #[test]
    fn empty_overrides_produce_no_override_fingerprint() {
        let source = PlanSource {
            repo_root: None,
            base: "aaa".to_string(),
            head: "bbb".to_string(),
            head_tree: None,
        };
        let units = vec![unit(
            "path:src/main.rs",
            "src/main.rs",
            UnitKind::Behavior,
            "app",
        )];
        let config = StackcutConfig::default();
        let overrides = Overrides::default();

        let result = plan(source, units, &config, &overrides);
        assert_eq!(result.override_fingerprint, None);
    }

    #[test]
    fn non_empty_overrides_produce_some_override_fingerprint() {
        let source = PlanSource {
            repo_root: None,
            base: "aaa".to_string(),
            head: "bbb".to_string(),
            head_tree: None,
        };
        let units = vec![unit(
            "path:src/main.rs",
            "src/main.rs",
            UnitKind::Behavior,
            "app",
        )];
        let config = StackcutConfig::default();
        let overrides = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:a".to_string(), "path:b".to_string()],
                reason: None,
            }],
            ..Overrides::default()
        };

        let result = plan(source, units, &config, &overrides);
        assert!(result.override_fingerprint.is_some());
        assert!(!result.override_fingerprint.as_ref().unwrap().is_empty());
    }

    #[test]
    fn override_fingerprint_is_deterministic() {
        let make_plan = || {
            let source = PlanSource {
                repo_root: None,
                base: "aaa".to_string(),
                head: "bbb".to_string(),
                head_tree: None,
            };
            let units = vec![unit(
                "path:src/main.rs",
                "src/main.rs",
                UnitKind::Behavior,
                "app",
            )];
            let config = StackcutConfig::default();
            let overrides = Overrides {
                force_members: vec![ForceMemberOverride {
                    member: "path:src/main.rs".to_string(),
                    slice: "behavior-app".to_string(),
                    reason: Some("test".to_string()),
                }],
                ..Overrides::default()
            };
            plan(source, units, &config, &overrides)
        };

        let fp1 = make_plan().override_fingerprint;
        let fp2 = make_plan().override_fingerprint;
        assert_eq!(fp1, fp2);
    }

    #[test]
    fn different_overrides_produce_different_fingerprints() {
        let source = || PlanSource {
            repo_root: None,
            base: "aaa".to_string(),
            head: "bbb".to_string(),
            head_tree: None,
        };
        let units = || {
            vec![unit(
                "path:src/main.rs",
                "src/main.rs",
                UnitKind::Behavior,
                "app",
            )]
        };
        let config = StackcutConfig::default();

        let overrides_a = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:a".to_string(), "path:b".to_string()],
                reason: None,
            }],
            ..Overrides::default()
        };
        let overrides_b = Overrides {
            must_link: vec![MustLinkOverride {
                members: vec!["path:c".to_string(), "path:d".to_string()],
                reason: None,
            }],
            ..Overrides::default()
        };

        let plan_a = plan(source(), units(), &config, &overrides_a);
        let plan_b = plan(source(), units(), &config, &overrides_b);
        assert_ne!(plan_a.override_fingerprint, plan_b.override_fingerprint);
    }

    #[test]
    fn override_fingerprint_roundtrips_through_json() {
        let source = PlanSource {
            repo_root: None,
            base: "aaa".to_string(),
            head: "bbb".to_string(),
            head_tree: None,
        };
        let units = vec![unit(
            "path:src/main.rs",
            "src/main.rs",
            UnitKind::Behavior,
            "app",
        )];
        let config = StackcutConfig::default();
        let overrides = Overrides {
            rename_slices: vec![RenameSliceOverride {
                id: "behavior-app".to_string(),
                title: "App logic".to_string(),
            }],
            ..Overrides::default()
        };

        let original = plan(source, units, &config, &overrides);
        assert!(original.override_fingerprint.is_some());

        let json = serde_json::to_string(&original).unwrap();
        let deserialized: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(
            original.override_fingerprint,
            deserialized.override_fingerprint
        );
    }

    /// Returns sorted list of fixture case directories under fixtures/cases/.
    fn fixture_case_dirs() -> Vec<std::path::PathBuf> {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("Could not find workspace root");
        let fixtures_dir = workspace_root.join("fixtures").join("cases");

        let mut case_dirs: Vec<std::path::PathBuf> = std::fs::read_dir(&fixtures_dir)
            .expect("Failed to read fixtures/cases/")
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir()
                    && path.join("input.units.json").exists()
                    && path.join("expected.plan.json").exists()
                {
                    Some(path)
                } else {
                    None
                }
            })
            .collect();

        case_dirs.sort();
        case_dirs
    }

    /// Load a fixture's input units and run the planner, returning the plan.
    fn run_fixture_planner(case_dir: &std::path::Path) -> Plan {
        let input_path = case_dir.join("input.units.json");
        let input_json =
            std::fs::read_to_string(&input_path).expect("Failed to read input.units.json");
        let units: Vec<EditUnit> =
            serde_json::from_str(&input_json).expect("Failed to parse input.units.json");

        let source = PlanSource {
            repo_root: None,
            base: "fixture-base".to_string(),
            head: "fixture-head".to_string(),
            head_tree: None,
        };
        plan(
            source,
            units,
            &StackcutConfig::default(),
            &Overrides::default(),
        )
    }

    // Feature: stackcut-v01-completion, Property 25: Golden Fixture Match
    // **Validates: Requirements 20.1**
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_golden_fixture_determinism(fixture_idx in 0..5usize) {
            let case_dirs = fixture_case_dirs();
            prop_assert!(fixture_idx < case_dirs.len(),
                "fixture_idx {} out of range (only {} fixtures)", fixture_idx, case_dirs.len());

            let case_dir = &case_dirs[fixture_idx];
            let case_name = case_dir.file_name().unwrap().to_string_lossy().to_string();

            // Run planner twice on the same fixture
            let plan_a = run_fixture_planner(case_dir);
            let plan_b = run_fixture_planner(case_dir);

            // Compare slices
            let summaries_a: Vec<SliceSummary> = plan_a.slices.iter().map(SliceSummary::from_slice).collect();
            let summaries_b: Vec<SliceSummary> = plan_b.slices.iter().map(SliceSummary::from_slice).collect();
            prop_assert_eq!(&summaries_a, &summaries_b,
                "[{}] Planner produced different slices on two runs", case_name);

            // Compare ambiguities
            let ambiguities_a: Vec<AmbiguitySummary> = plan_a.ambiguities.iter().map(AmbiguitySummary::from_ambiguity).collect();
            let ambiguities_b: Vec<AmbiguitySummary> = plan_b.ambiguities.iter().map(AmbiguitySummary::from_ambiguity).collect();
            prop_assert_eq!(&ambiguities_a, &ambiguities_b,
                "[{}] Planner produced different ambiguities on two runs", case_name);

            // Also verify against expected.plan.json for golden match
            let expected_path = case_dir.join("expected.plan.json");
            let expected_json = std::fs::read_to_string(&expected_path)
                .expect("Failed to read expected.plan.json");
            let expected_plan: Plan = serde_json::from_str(&expected_json)
                .expect("Failed to parse expected.plan.json");

            let mut expected_summaries: Vec<SliceSummary> = expected_plan.slices.iter().map(SliceSummary::from_slice).collect();
            expected_summaries.sort_by(|a, b| a.id.cmp(&b.id));
            let mut actual_summaries: Vec<SliceSummary> = summaries_a;
            actual_summaries.sort_by(|a, b| a.id.cmp(&b.id));

            prop_assert_eq!(actual_summaries.len(), expected_summaries.len(),
                "[{}] Slice count mismatch: actual {} vs expected {}", case_name, actual_summaries.len(), expected_summaries.len());

            for (actual, expected) in actual_summaries.iter().zip(expected_summaries.iter()) {
                prop_assert_eq!(&actual.id, &expected.id,
                    "[{}] Slice ID mismatch", case_name);
                prop_assert_eq!(&actual.kind, &expected.kind,
                    "[{}] Slice '{}' kind mismatch", case_name, actual.id);
                prop_assert_eq!(&actual.members, &expected.members,
                    "[{}] Slice '{}' members mismatch", case_name, actual.id);
                prop_assert_eq!(&actual.depends_on, &expected.depends_on,
                    "[{}] Slice '{}' depends_on mismatch", case_name, actual.id);
            }

            let mut expected_ambiguities: Vec<AmbiguitySummary> = expected_plan.ambiguities.iter().map(AmbiguitySummary::from_ambiguity).collect();
            expected_ambiguities.sort_by(|a, b| a.id.cmp(&b.id));
            let mut actual_ambiguities: Vec<AmbiguitySummary> = ambiguities_a;
            actual_ambiguities.sort_by(|a, b| a.id.cmp(&b.id));

            prop_assert_eq!(actual_ambiguities.len(), expected_ambiguities.len(),
                "[{}] Ambiguity count mismatch", case_name);

            for (actual, expected) in actual_ambiguities.iter().zip(expected_ambiguities.iter()) {
                prop_assert_eq!(&actual.id, &expected.id,
                    "[{}] Ambiguity ID mismatch", case_name);
                prop_assert_eq!(&actual.affected_units, &expected.affected_units,
                    "[{}] Ambiguity '{}' affected_units mismatch", case_name, actual.id);
                prop_assert_eq!(&actual.candidate_slices, &expected.candidate_slices,
                    "[{}] Ambiguity '{}' candidate_slices mismatch", case_name, actual.id);
            }
        }
    }
}
