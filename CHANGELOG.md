# Changelog

All notable changes to stackcut are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-06-20

First release. stackcut is a deterministic, file-scoped diff-to-stack compiler:
it turns one oversized Git change into a reviewable stack of slices with
portable artifacts, and proves the stack reproduces the original change exactly.

### Planner

- Deterministic, file-scoped slice solver — same input always yields the same
  plan; no timestamps or hidden nondeterminism in artifacts.
- Transparent grouping rules: manifest and lock files move together, generated
  files follow their source family, tests and docs attach to the code family
  they validate, ops/config changes stay isolated, and rename-only changes can
  peel off as `prep-refactor` slices.
- Ambiguous root-level docs/tests are surfaced explicitly rather than guessed,
  with a review-budget diagnostic for oversized slices.
- Slices form an acyclic DAG; every edit unit appears exactly once.

### Git ingest and recomposition

- `collect_edit_units` over a base/head range, handling adds, modifies, pure
  renames, rename-with-edit, copies, and type-changes.
- Patch-series materialization per slice, with dry-run application and rollback.
- Exact recomposition validation: applying the generated patches to the base
  revision reproduces the head tree, verified by tree hash.

### Overrides

- `override.toml` model with `must_link`, `force_members`, `rename_slices`, and
  `must_order`, including cycle detection. Overrides are replayable inputs, not
  a hidden rule engine.

### Artifacts and validation

- Portable artifacts: `plan.json`, `summary.md`, `diagnostics.json`, and a
  per-slice patch series.
- Plan and per-slice SHA-256 fingerprints plus an override fingerprint.
- Recomposition receipts and a structured diagnostics envelope.
- Derived outputs: SARIF 2.1.0 (`emit-sarif`), proof-surface hints
  (`emit-proof`), a PR-ready review packet (`emit-review-packet`), and an
  override scaffold (`scaffold-overrides`).
- Versioned schema contracts: `schema/stackcut.plan.schema.json` and
  `schema/stackcut.override.schema.json`.

### CLI

- Eleven subcommands: `plan`, `explain` (with `--why`), `validate` (with
  `--exact`, `--receipt`, `--format`), `materialize`, `doctor`, `compare`,
  `init`, `scaffold-overrides`, `emit-sarif`, `emit-proof`, and
  `emit-review-packet`.
- Stable exit-code contract: `0` success, `1` structural error, `2`
  recomposition failure, `3` override conflict, `4` unsupported surface,
  `10` internal bug.

### Trust boundary

- Edit normalization, classification, rule-based planning, structural
  validation, and exact recomposition are inside the trust boundary and fully
  deterministic. AI suggestions and semantic adapters are explicitly outside.

### Tooling and proof surface

- Canonical fixture corpus with golden plans and a scenario atlas/index.
- Property tests (proptest), golden tests, snapshot tests, and temp-repo
  integration tests covering the plan → materialize → validate pipeline.
- `xtask` ritual commands (`ci-fast`, `ci-full`, `smoke`, `golden`, `mutants`,
  `docs-check`, `release-check`) with CI and coverage lanes.

[0.1.0]: https://github.com/EffortlessMetrics/stackcut/releases/tag/v0.1.0
