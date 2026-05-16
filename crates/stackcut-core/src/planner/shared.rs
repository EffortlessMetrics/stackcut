//! Shared helpers used across planner phases.

use std::collections::{BTreeMap, BTreeSet};

use crate::{EditUnit, InclusionReason, ProofSurface, Slice, SliceKind, UnitKind};

pub(super) fn collect_ids<F>(
    units: &[EditUnit],
    assigned: &BTreeSet<String>,
    mut predicate: F,
) -> Vec<String>
where
    F: FnMut(&EditUnit) -> bool,
{
    units
        .iter()
        .filter(|unit| !assigned.contains(&unit.id) && predicate(unit))
        .map(|unit| unit.id.clone())
        .collect()
}

pub(super) fn family_list_for_members(units: &[EditUnit], member_ids: &[String]) -> Vec<String> {
    let families: BTreeSet<String> = units
        .iter()
        .filter(|unit| member_ids.iter().any(|member| member == &unit.id))
        .map(|unit| unit.family.clone())
        .collect();
    families.into_iter().collect()
}

pub(super) fn has_slice(slices: &[Slice], id: &str) -> bool {
    slices.iter().any(|slice| slice.id == id)
}

pub(super) fn has_family_overlap(
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

pub(super) fn mark_assigned(assigned: &mut BTreeSet<String>, member_ids: &[String]) {
    for member_id in member_ids {
        assigned.insert(member_id.clone());
    }
}

pub(crate) fn new_slice(
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
        fingerprint: None,
    };
    dedup_and_sort(&mut slice.families);
    dedup_and_sort(&mut slice.members);
    dedup_and_sort(&mut slice.depends_on);
    slice
}

pub(super) fn reason(code: &str, message: &str) -> InclusionReason {
    InclusionReason {
        code: code.to_string(),
        message: message.to_string(),
    }
}

pub(super) fn find_slice_for_member(slices: &[Slice], member: &str) -> Option<String> {
    slices
        .iter()
        .find(|slice| slice.members.iter().any(|candidate| candidate == member))
        .map(|slice| slice.id.clone())
}

pub(super) fn dedup_and_sort(values: &mut Vec<String>) {
    let set: BTreeSet<String> = values.drain(..).collect();
    values.extend(set);
}

pub(super) fn move_member(slices: &mut [Slice], member: &str, target_slice: &str) {
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

pub(super) fn attach_member(slices: &mut [Slice], slice_id: &str, unit: &EditUnit, message: &str) {
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

pub(super) fn describe_kind(kind: &UnitKind) -> &'static str {
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

pub(super) fn slugify(input: &str) -> String {
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

pub(super) fn infer_owner_by_path_segment(
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
        None
    }
}

pub(crate) fn has_cycle(slices: &[Slice]) -> bool {
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
