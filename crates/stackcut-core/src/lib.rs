use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PLAN_VERSION: &str = "0.1.0";

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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

    if config.generated_prefixes.iter().any(|prefix| path.starts_with(prefix))
        || path.ends_with(".snap")
        || path.ends_with(".generated.rs")
    {
        return UnitKind::Generated;
    }

    if config.test_prefixes.iter().any(|prefix| path.starts_with(prefix))
        || path.contains("/tests/")
        || path.ends_with("_test.rs")
        || path.ends_with(".spec.ts")
    {
        return UnitKind::Test;
    }

    if config.doc_prefixes.iter().any(|prefix| path.starts_with(prefix))
        || path.ends_with(".md")
        || path.ends_with(".mdx")
    {
        return UnitKind::Documentation;
    }

    if config.ops_prefixes.iter().any(|prefix| path.starts_with(prefix))
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

pub fn plan(
    source: PlanSource,
    mut units: Vec<EditUnit>,
    _config: &StackcutConfig,
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
        slices.push(new_slice(
            "mechanical-renames",
            "Mechanical rename-only changes",
            SliceKind::Mechanical,
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

    apply_overrides(&mut slices, overrides);

    let mut diagnostics = structural_validate(&Plan {
        version: PLAN_VERSION.to_string(),
        source: source.clone(),
        units: units.clone(),
        slices: slices.clone(),
        ambiguities: ambiguities.clone(),
        diagnostics: Vec::new(),
    });

    if !ambiguities.is_empty() {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "ambiguity-present".to_string(),
            message: "Plan contains one or more explicit ambiguities.".to_string(),
        });
    }

    Plan {
        version: PLAN_VERSION.to_string(),
        source,
        units,
        slices,
        ambiguities,
        diagnostics,
    }
}

pub fn structural_validate(plan: &Plan) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let known_slice_ids: BTreeSet<String> = plan.slices.iter().map(|slice| slice.id.clone()).collect();
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
                message: format!(
                    "Unit {} appears {} times across slices.",
                    unit.id, count
                ),
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
        .filter_map(|(node, count)| if *count == 0 { Some(node.clone()) } else { None })
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

fn apply_overrides(slices: &mut Vec<Slice>, overrides: &Overrides) {
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
                    vec![reason(
                        "override",
                        "Created to satisfy must_link override.",
                    )],
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
        if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.after) {
            slice.depends_on.push(rule.before.clone());
            dedup_and_sort(&mut slice.depends_on);
            slice.reasons.push(reason(
                "override-must-order",
                rule.reason
                    .as_deref()
                    .unwrap_or("Ordering edge added by override."),
            ));
        }
    }
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
            unit("path:src/core/planner.rs", "src/core/planner.rs", UnitKind::Behavior, "core"),
            unit("path:tests/planner.rs", "tests/planner.rs", UnitKind::Test, "core"),
            unit("path:docs/planner.md", "docs/planner.md", UnitKind::Documentation, "core"),
        ];

        let plan = plan(source, units, &StackcutConfig::default(), &Overrides::default());
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
            unit("path:src/core/a.rs", "src/core/a.rs", UnitKind::Behavior, "core"),
            unit("path:src/git/b.rs", "src/git/b.rs", UnitKind::Behavior, "git"),
            unit("path:README.md", "README.md", UnitKind::Documentation, "root"),
        ];

        let plan = plan(source, units, &StackcutConfig::default(), &Overrides::default());
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
        };

        let diagnostics = structural_validate(&plan);
        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "duplicate-member"));
    }
}
