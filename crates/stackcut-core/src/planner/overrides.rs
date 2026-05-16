//! Applies override rules to mutate the slice set after the deterministic
//! phases have produced their assignments.

use crate::{Diagnostic, DiagnosticLevel, Overrides, Slice, SliceKind};

use super::shared::{
    dedup_and_sort, find_slice_for_member, has_cycle, move_member, new_slice, reason, slugify,
};

pub(crate) fn apply(slices: &mut Vec<Slice>, overrides: &Overrides) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    apply_must_link(slices, overrides);
    apply_force_members(slices, overrides);
    apply_rename_slices(slices, overrides);
    diagnostics.extend(apply_must_order(slices, overrides));

    diagnostics
}

fn apply_must_link(slices: &mut Vec<Slice>, overrides: &Overrides) {
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
}

fn apply_force_members(slices: &mut Vec<Slice>, overrides: &Overrides) {
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
}

fn apply_rename_slices(slices: &mut [Slice], overrides: &Overrides) {
    for rule in &overrides.rename_slices {
        if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.id) {
            slice.title = rule.title.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ForceMemberOverride;

    use super::super::shared::new_slice;

    fn make_slice(id: &str, members: Vec<&str>) -> Slice {
        new_slice(
            id,
            id,
            SliceKind::Behavior,
            vec!["test-family".to_string()],
            members.into_iter().map(|s| s.to_string()).collect(),
            Vec::new(),
            Vec::new(),
        )
    }

    #[test]
    fn apply_force_members_creates_new_slice_when_target_missing() {
        // Two slices exist; the override targets a slice ID that does not exist yet.
        let mut slices = vec![
            make_slice("slice-alpha", vec!["unit-a", "unit-b"]),
            make_slice("slice-beta", vec!["unit-c"]),
        ];

        let overrides = Overrides {
            force_members: vec![ForceMemberOverride {
                member: "unit-b".to_string(),
                slice: "nonexistent-slice".to_string(),
                reason: None,
            }],
            ..Default::default()
        };

        apply_force_members(&mut slices, &overrides);

        // A new slice must have been created.
        let new_slice = slices
            .iter()
            .find(|s| s.id == "nonexistent-slice")
            .expect("new slice 'nonexistent-slice' must be created");

        // The member must have moved into the new slice.
        assert!(
            new_slice.members.contains(&"unit-b".to_string()),
            "unit-b should be in the newly-created slice"
        );

        // The creation reason must be present.
        assert!(
            new_slice
                .reasons
                .iter()
                .any(|r| r.message == "Created to satisfy force_members override."),
            "new slice must carry the creation reason; reasons: {:?}",
            new_slice.reasons
        );

        // The member must no longer appear in the original slice.
        let alpha = slices
            .iter()
            .find(|s| s.id == "slice-alpha")
            .expect("slice-alpha must still exist");
        assert!(
            !alpha.members.contains(&"unit-b".to_string()),
            "unit-b should have been moved out of slice-alpha"
        );
    }
}

fn apply_must_order(slices: &mut [Slice], overrides: &Overrides) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for rule in &overrides.must_order {
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

        if has_cycle(slices) {
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
        } else if let Some(slice) = slices.iter_mut().find(|slice| slice.id == rule.after) {
            slice.reasons.push(reason(
                "override-must-order",
                rule.reason
                    .as_deref()
                    .unwrap_or("Ordering edge added by override."),
            ));
        }
    }
    diagnostics
}
