//! Attaches generated/test/doc units to existing behavior slices or surfaces
//! them for standalone grouping.

use std::collections::{BTreeMap, BTreeSet};

use crate::{Ambiguity, EditUnit, Slice, SliceKind, UnitKind};

use super::shared::{
    attach_member, collect_ids, describe_kind, infer_owner_by_path_segment, slugify,
};

pub(super) type StandaloneGroups = BTreeMap<(SliceKind, String), Vec<String>>;

pub(super) struct AttachmentOutcome {
    pub assigned_ids: Vec<String>,
    pub ambiguities: Vec<Ambiguity>,
    pub standalone_groups: StandaloneGroups,
}

pub(super) fn run(
    units: &[EditUnit],
    slices: &mut [Slice],
    assigned: &BTreeSet<String>,
    family_to_slice: &BTreeMap<String, String>,
) -> AttachmentOutcome {
    let attachable_ids = collect_ids(units, assigned, |unit| {
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

    let mut assigned_ids = Vec::new();
    let mut ambiguities = Vec::new();
    let mut standalone_groups: StandaloneGroups = BTreeMap::new();

    for unit_id in attachable_ids {
        let Some(unit) = unit_lookup.get(&unit_id) else {
            continue;
        };
        if let Some(slice_id) = family_to_slice.get(&unit.family) {
            attach_member(
                slices,
                slice_id,
                unit,
                &format!(
                    "{} stays with the {} family when ownership is clear.",
                    describe_kind(&unit.kind),
                    unit.family
                ),
            );
            assigned_ids.push(unit.id.clone());
            continue;
        }

        if let Some(slice_id) = infer_owner_by_path_segment(&unit.path, family_to_slice) {
            attach_member(
                slices,
                &slice_id,
                unit,
                &format!(
                    "{} attached to {} via path-segment inference.",
                    describe_kind(&unit.kind),
                    unit.path
                ),
            );
            assigned_ids.push(unit.id.clone());
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
        assigned_ids.push(unit.id.clone());
    }

    AttachmentOutcome {
        assigned_ids,
        ambiguities,
        standalone_groups,
    }
}
