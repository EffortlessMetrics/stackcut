//! Builds the manifest + lockfile slice (`api-schema-workspace`).

use std::collections::BTreeSet;

use crate::{EditUnit, Slice, SliceKind, UnitKind};

use super::shared::{collect_ids, family_list_for_members, new_slice, reason};

pub(super) fn build(
    units: &[EditUnit],
    assigned: &BTreeSet<String>,
) -> Option<(Slice, Vec<String>)> {
    let manifest_ids = collect_ids(units, assigned, |unit| {
        matches!(unit.kind, UnitKind::Manifest | UnitKind::Lockfile)
    });
    if manifest_ids.is_empty() {
        return None;
    }
    let slice = new_slice(
        "api-schema-workspace",
        "Manifest and lockstep package metadata",
        SliceKind::ApiSchema,
        family_list_for_members(units, &manifest_ids),
        manifest_ids.clone(),
        Vec::new(),
        vec![reason(
            "lockstep-metadata",
            "Manifest and lock files move together in v0.1.",
        )],
    );
    Some((slice, manifest_ids))
}
