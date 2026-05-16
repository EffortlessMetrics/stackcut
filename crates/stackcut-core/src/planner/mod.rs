//! Plan construction pipeline.
//!
//! `plan` orchestrates the SRP-focused submodules in this directory, each of
//! which owns one phase of the conversion from a sorted [`EditUnit`] stream
//! into a finished [`Plan`]:
//!
//! - [`manifest`]: manifest + lockfile slice
//! - [`ops`]: ops/config slice
//! - [`mechanical`]: rename-only / mechanical slice
//! - [`behavior`]: per-family behavior slices
//! - [`attachment`]: hangs generated/test/doc units onto behavior slices
//! - [`standalone`]: standalone slices for orphaned units + misc catch-all
//! - [`overrides`]: applies override rules over the deterministic output
//! - [`diagnostics`]: gathers structural + override + budget warnings
//! - [`fingerprint`]: computes the override fingerprint
//! - [`shared`]: small helpers used across phases

use std::collections::BTreeSet;

use crate::{EditUnit, Overrides, Plan, PlanSource, StackcutConfig, PLAN_VERSION};

mod attachment;
mod behavior;
mod diagnostics;
mod fingerprint;
mod manifest;
mod mechanical;
mod ops;
pub(crate) mod overrides;
pub(crate) mod shared;
mod standalone;

use shared::mark_assigned;

pub fn plan(
    source: PlanSource,
    mut units: Vec<EditUnit>,
    config: &StackcutConfig,
    overrides_cfg: &Overrides,
) -> Plan {
    units.sort_by(|left, right| left.path.cmp(&right.path));

    let mut slices = Vec::new();
    let mut assigned: BTreeSet<String> = BTreeSet::new();

    if let Some((slice, ids)) = manifest::build(&units, &assigned) {
        mark_assigned(&mut assigned, &ids);
        slices.push(slice);
    }

    if let Some((slice, ids)) = ops::build(&units, &assigned) {
        mark_assigned(&mut assigned, &ids);
        slices.push(slice);
    }

    if let Some((slice, ids)) = mechanical::build(&units, &assigned) {
        mark_assigned(&mut assigned, &ids);
        slices.push(slice);
    }

    let behavior_outcome = behavior::build(&units, &slices, &assigned);
    mark_assigned(&mut assigned, &behavior_outcome.assigned_ids);
    slices.extend(behavior_outcome.slices);
    let family_to_slice = behavior_outcome.family_to_slice;

    let attachment_outcome = attachment::run(&units, &mut slices, &assigned, &family_to_slice);
    mark_assigned(&mut assigned, &attachment_outcome.assigned_ids);
    let ambiguities = attachment_outcome.ambiguities;

    let standalone_slices = standalone::build_standalone_slices(
        &units,
        attachment_outcome.standalone_groups,
        &family_to_slice,
    );
    slices.extend(standalone_slices);

    if let Some((slice, ids)) = standalone::build_misc_unassigned(&units, &assigned) {
        mark_assigned(&mut assigned, &ids);
        slices.push(slice);
    }

    let diagnostics = diagnostics::gather(
        &source,
        &units,
        &mut slices,
        &ambiguities,
        overrides_cfg,
        config,
    );

    let override_fingerprint = fingerprint::compute_override_fingerprint(overrides_cfg);

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
