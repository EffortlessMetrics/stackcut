//! Builds standalone slices (generated/tests-docs/misc) and the catch-all
//! `misc-unassigned` slice for any remaining units.

use std::collections::BTreeSet;

use crate::{EditUnit, Slice, SliceKind};

use super::attachment::StandaloneGroups;
use super::shared::{collect_ids, family_list_for_members, new_slice, reason, slugify};

pub(super) fn build_standalone_slices(
    units: &[EditUnit],
    groups: StandaloneGroups,
    family_to_slice: &std::collections::BTreeMap<String, String>,
) -> Vec<Slice> {
    let mut slices = Vec::new();

    for ((slice_kind, family), member_ids) in groups {
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
            family_list_for_members(units, &member_ids),
            member_ids,
            depends_on,
            vec![reason(
                "standalone-attachment",
                "No single behavior owner was available, so the material stays explicit.",
            )],
        ));
    }

    slices
}

pub(super) fn build_misc_unassigned(
    units: &[EditUnit],
    assigned: &BTreeSet<String>,
) -> Option<(Slice, Vec<String>)> {
    let unassigned_ids = collect_ids(units, assigned, |_| true);
    if unassigned_ids.is_empty() {
        return None;
    }
    let slice = new_slice(
        "misc-unassigned",
        "Misc: unassigned changes",
        SliceKind::Misc,
        family_list_for_members(units, &unassigned_ids),
        unassigned_ids.clone(),
        Vec::new(),
        vec![reason(
            "misc-catchall",
            "Unassigned units collected into misc slice.",
        )],
    );
    Some((slice, unassigned_ids))
}
