//! Builds per-family behavior slices and records the family-to-slice mapping.

use std::collections::{BTreeMap, BTreeSet};

use crate::{EditUnit, Slice, SliceKind, UnitKind};

use super::shared::{has_family_overlap, has_slice, new_slice, reason, slugify};

pub(super) struct BehaviorOutcome {
    pub slices: Vec<Slice>,
    pub assigned_ids: Vec<String>,
    pub family_to_slice: BTreeMap<String, String>,
}

pub(super) fn build(
    units: &[EditUnit],
    existing_slices: &[Slice],
    assigned: &BTreeSet<String>,
) -> BehaviorOutcome {
    let mut behavior_by_family: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for unit in units {
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

    let mut new_slices = Vec::new();
    let mut assigned_ids = Vec::new();
    let mut family_to_slice = BTreeMap::new();

    for (family, member_ids) in behavior_by_family {
        assigned_ids.extend(member_ids.iter().cloned());

        let slice_id = format!("behavior-{}", slugify(&family));
        let mut depends_on = Vec::new();
        if has_slice(existing_slices, "api-schema-workspace") {
            depends_on.push("api-schema-workspace".to_string());
        }
        if has_family_overlap(units, &member_ids, existing_slices, "mechanical-renames") {
            depends_on.push("mechanical-renames".to_string());
        }

        new_slices.push(new_slice(
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

    BehaviorOutcome {
        slices: new_slices,
        assigned_ids,
        family_to_slice,
    }
}
