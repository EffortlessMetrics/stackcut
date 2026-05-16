//! Builds the mechanical/rename slice (`mechanical-renames` or `prep-refactor` when all renames).

use std::collections::BTreeSet;

use crate::{ChangeStatus, EditUnit, Slice, SliceKind, UnitKind};

use super::shared::{collect_ids, family_list_for_members, new_slice, reason};

pub(super) fn build(
    units: &[EditUnit],
    assigned: &BTreeSet<String>,
) -> Option<(Slice, Vec<String>)> {
    let mechanical_ids = collect_ids(units, assigned, |unit| unit.kind == UnitKind::Mechanical);
    if mechanical_ids.is_empty() {
        return None;
    }

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

    let slice = new_slice(
        "mechanical-renames",
        "Mechanical rename-only changes",
        slice_kind,
        family_list_for_members(units, &mechanical_ids),
        mechanical_ids.clone(),
        Vec::new(),
        vec![reason(
            "mechanical-split",
            "Rename-only changes peel off as a prep slice when independent.",
        )],
    );
    Some((slice, mechanical_ids))
}
