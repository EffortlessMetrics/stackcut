use anyhow::{Context, Result};
use stackcut_core::{Diagnostic, Plan};
use std::fs;
use std::path::Path;

pub fn read_plan(path: &Path) -> Result<Plan> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let plan = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(plan)
}

pub fn write_plan(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(plan).context("failed to serialize plan")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn write_diagnostics(path: &Path, diagnostics: &[Diagnostic]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(diagnostics)
        .context("failed to serialize diagnostics")?;
    fs::write(path, format!("{json}\n"))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn write_summary(path: &Path, plan: &Plan) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, render_summary(plan))
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

pub fn render_summary(plan: &Plan) -> String {
    let mut output = String::new();

    output.push_str("# stackcut summary\n\n");
    output.push_str(&format!(
        "- base: `{}`\n- head: `{}`\n- slices: `{}`\n- ambiguities: `{}`\n\n",
        plan.source.base,
        plan.source.head,
        plan.slices.len(),
        plan.ambiguities.len()
    ));

    output.push_str("## slices\n\n");
    for slice in &plan.slices {
        output.push_str(&format!(
            "### `{}` — {} ({:?})\n\n",
            slice.id, slice.title, slice.kind
        ));
        if !slice.families.is_empty() {
            output.push_str(&format!(
                "- families: {}\n",
                slice.families.join(", ")
            ));
        }
        if !slice.depends_on.is_empty() {
            output.push_str(&format!(
                "- depends on: {}\n",
                slice.depends_on.join(", ")
            ));
        }
        output.push_str("- members:\n");
        for member in &slice.members {
            output.push_str(&format!("  - `{}`\n", member));
        }
        if !slice.reasons.is_empty() {
            output.push_str("- reasons:\n");
            for reason in &slice.reasons {
                output.push_str(&format!("  - `{}`: {}\n", reason.code, reason.message));
            }
        }
        output.push('\n');
    }

    if !plan.ambiguities.is_empty() {
        output.push_str("## ambiguities\n\n");
        for ambiguity in &plan.ambiguities {
            output.push_str(&format!("### `{}`\n\n", ambiguity.id));
            output.push_str(&format!("{}\n\n", ambiguity.message));
            if !ambiguity.affected_units.is_empty() {
                output.push_str(&format!(
                    "- affected: {}\n",
                    ambiguity.affected_units.join(", ")
                ));
            }
            if !ambiguity.candidate_slices.is_empty() {
                output.push_str(&format!(
                    "- candidates: {}\n",
                    ambiguity.candidate_slices.join(", ")
                ));
            }
            output.push_str(&format!("- resolution: {}\n\n", ambiguity.resolution));
        }
    }

    if !plan.diagnostics.is_empty() {
        output.push_str("## diagnostics\n\n");
        for diagnostic in &plan.diagnostics {
            output.push_str(&format!(
                "- `{:?}` `{}`: {}\n",
                diagnostic.level, diagnostic.code, diagnostic.message
            ));
        }
        output.push('\n');
    }

    output
}


#[cfg(test)]
mod tests {
    use super::*;
    use stackcut_core::{
        ChangeStatus, Diagnostic, DiagnosticLevel, EditUnit, Plan, PlanSource, ProofSurface,
        Slice, SliceKind,
    };

    #[test]
    fn summary_includes_slice_ids_and_diagnostics() {
        let plan = Plan {
            version: "0.1.0".to_string(),
            source: PlanSource {
                repo_root: None,
                base: "base".to_string(),
                head: "head".to_string(),
                head_tree: None,
            },
            units: vec![EditUnit {
                id: "path:src/core/planner.rs".to_string(),
                path: "src/core/planner.rs".to_string(),
                old_path: None,
                status: ChangeStatus::Modified,
                kind: stackcut_core::UnitKind::Behavior,
                family: "core".to_string(),
                notes: Vec::new(),
            }],
            slices: vec![Slice {
                id: "behavior-core".to_string(),
                title: "Behavior: core".to_string(),
                kind: SliceKind::Behavior,
                families: vec!["core".to_string()],
                members: vec!["path:src/core/planner.rs".to_string()],
                depends_on: Vec::new(),
                reasons: Vec::new(),
                proof_surface: ProofSurface::default(),
            }],
            ambiguities: Vec::new(),
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Note,
                code: "ok".to_string(),
                message: "summary smoke".to_string(),
            }],
        };

        let rendered = render_summary(&plan);
        assert!(rendered.contains("behavior-core"));
        assert!(rendered.contains("summary smoke"));
    }
}
