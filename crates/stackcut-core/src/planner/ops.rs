//! Builds the ops/configuration slice (`ops-config`).

use std::collections::BTreeSet;

use crate::{EditUnit, Slice, SliceKind, UnitKind};

use super::shared::{collect_ids, family_list_for_members, new_slice, reason};

pub(super) fn build(
    units: &[EditUnit],
    assigned: &BTreeSet<String>,
) -> Option<(Slice, Vec<String>)> {
    let ops_ids = collect_ids(units, assigned, |unit| unit.kind == UnitKind::OpsConfig);
    if ops_ids.is_empty() {
        return None;
    }
    let slice = new_slice(
        "ops-config",
        "Ops and configuration",
        SliceKind::OpsConfig,
        family_list_for_members(units, &ops_ids),
        ops_ids.clone(),
        Vec::new(),
        vec![reason(
            "ops-isolation",
            "Operational configuration is isolated from behavior changes.",
        )],
    );
    Some((slice, ops_ids))
}
