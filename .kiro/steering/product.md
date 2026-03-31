# Product: stackcut

`stackcut` is a deterministic diff-to-stack compiler. It takes one oversized Git change, splits it into a reviewable stack of file-scoped slices, and emits portable artifacts (plan JSON, summary Markdown, diagnostics JSON, patch series).

## What it does (v0.1)

- Plans file-scoped stacks from a Git base/head range
- Groups changed files into ordered slices using deterministic rules
- Emits `plan.json`, `summary.md`, and `diagnostics.json`
- Materializes a patch series per slice
- Validates structural invariants and exact recomposition (applying patches to base must reproduce head)

## What it does not do (yet)

- Intra-file hunk splitting
- Semantic symbol slicing
- Stacked pull-request automation
- Workflow hosting or autonomous change application
- AI-driven decisions inside the trust boundary

## Trust boundary

Everything inside the trust boundary is deterministic and rule-based: edit normalization, path classification, slice planning, structural validation, exact recomposition. AI, semantic adapters, and downstream integrations live outside the trust boundary.

## Key design principles

- Deterministic rules over heuristics that cannot explain themselves
- Surface ambiguity explicitly rather than guessing silently
- Artifacts are contracts — `plan.json`, `summary.md`, `diagnostics.json`, and patches must remain stable
- Scenarios and fixtures are first-class; they pin planner semantics
- Override model (`override.toml`) is a release valve, not a hidden rule engine
