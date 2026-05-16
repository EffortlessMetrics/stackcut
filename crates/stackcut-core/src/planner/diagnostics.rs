//! Gathers plan-level diagnostics (override validation, structural checks,
//! ambiguity, review budget, unsupported notes).

use std::collections::BTreeSet;

use crate::{
    structural_validate, validate_overrides, Ambiguity, Diagnostic, DiagnosticLevel, EditUnit,
    Overrides, Plan, PlanSource, Slice, StackcutConfig, PLAN_VERSION,
};

use super::overrides::apply as apply_overrides;

pub(super) fn gather(
    source: &PlanSource,
    units: &[EditUnit],
    slices: &mut Vec<Slice>,
    ambiguities: &[Ambiguity],
    overrides_cfg: &Overrides,
    config: &StackcutConfig,
) -> Vec<Diagnostic> {
    let unit_ids: BTreeSet<String> = units.iter().map(|u| u.id.clone()).collect();
    let slice_ids: BTreeSet<String> = slices.iter().map(|s| s.id.clone()).collect();
    let override_diagnostics = validate_overrides(overrides_cfg, &unit_ids, &slice_ids);

    let apply_diagnostics = apply_overrides(slices, overrides_cfg);

    let mut diagnostics = structural_validate(&Plan {
        version: PLAN_VERSION.to_string(),
        source: source.clone(),
        units: units.to_vec(),
        slices: slices.clone(),
        ambiguities: ambiguities.to_vec(),
        diagnostics: Vec::new(),
        fingerprint: None,
        override_fingerprint: None,
    });

    if !ambiguities.is_empty() {
        diagnostics.push(Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "ambiguity-present".to_string(),
            message: "Plan contains one or more explicit ambiguities.".to_string(),
        });
    }

    diagnostics.extend(override_diagnostics);
    diagnostics.extend(apply_diagnostics);

    diagnostics.extend(review_budget_warnings(slices, config));
    diagnostics.extend(unsupported_note_warnings(units));

    diagnostics
}

fn review_budget_warnings(slices: &[Slice], config: &StackcutConfig) -> Vec<Diagnostic> {
    let budget = config.review_budget.unwrap_or(15) as usize;
    slices
        .iter()
        .filter(|slice| slice.members.len() > budget)
        .map(|slice| Diagnostic {
            level: DiagnosticLevel::Warning,
            code: "review-budget-exceeded".to_string(),
            message: format!(
                "Slice '{}' has {} members (budget: {})",
                slice.id,
                slice.members.len(),
                budget
            ),
        })
        .collect()
}

fn unsupported_note_warnings(units: &[EditUnit]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for unit in units {
        for note in &unit.notes {
            if note.starts_with("unsupported-") {
                diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    code: note.clone(),
                    message: format!("{} is an unsupported change type in v0.1", unit.path),
                });
            }
        }
    }
    diagnostics
}
