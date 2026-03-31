# Implementation Plan: stackcut-v01-completion

## Overview

Complete the stackcut v0.1 build by closing gaps between documented contracts and running code. Build order: contract drift → overrides → validation → planner deepening → git edge hardening → proof surface & artifacts. All 26 correctness properties get proptest-based tests. Dependency direction is preserved: no reverse edges into `stackcut-core`.

## Tasks

- [x] 1. Contract drift fixes (xtask alias, CI, config parser, docs-check)
  - [x] 1.1 Create `.cargo/config.toml` with `xtask` alias
    - Add `[alias] xtask = "run --package xtask --"` so `cargo xtask` works as documented
    - _Requirements: 1.1, 1.2_

  - [x] 1.2 Harden config parser in `stackcut-core`
    - Add `SUPPORTED_CONFIG_VERSION` constant
    - Implement `parse_config()` that returns `(StackcutConfig, Vec<Diagnostic>)`
    - Emit `unknown-config-key` warning for unrecognized keys
    - Reject `version` greater than supported with descriptive error
    - Add `review_budget: Option<u32>` field to `StackcutConfig`
    - _Requirements: 4.1, 4.2, 4.3, 15.2_

  - [x] 1.3 Write property tests for config parser
    - **Property 19: Unknown Config Keys Warning**
    - **Validates: Requirements 4.2**
    - **Property 20: Unsupported Config Version Rejection**
    - **Validates: Requirements 4.3**

  - [x] 1.4 Enhance `docs-check` in xtask
    - Implement `extract_path_references()` to find backtick-quoted and markdown-link paths in docs
    - Verify references in `README.md`, `AGENTS.md`, `TESTING.md`, `RELEASE.md`, `docs/ARCHITECTURE.md`
    - Exit non-zero on broken references
    - _Requirements: 5.1, 5.2_

  - [x] 1.5 Add CI workflow tool installation steps
    - Add `cargo install cargo-mutants` and `cargo install cargo-fuzz` with caching to `.github/workflows/ci.yml`
    - _Requirements: 2.1, 2.2, 2.3_

- [x] 2. Checkpoint — contract drift
  - Ensure all tests pass, ask the user if questions arise.

- [x] 3. Override engine — parsing and validation
  - [x] 3.1 Implement `validate_overrides()` in `stackcut-core`
    - Accept `Overrides`, `BTreeSet<String>` of unit IDs, `BTreeSet<String>` of slice IDs
    - Emit `unknown-override-member` for unknown member refs in `must_link` and `force_members`
    - Emit `unknown-override-slice` for unknown slice refs in `rename_slices` and `must_order`
    - Emit `must-link-too-few` for `must_link` groups with < 2 members
    - Wire `validate_overrides` into the `plan()` function after building slices
    - _Requirements: 6.2, 6.3, 6.4_

  - [x] 3.2 Write property test for override validation
    - **Property 21: Override Validation Warnings**
    - **Validates: Requirements 6.2, 6.3, 6.4**

- [x] 4. Override engine — application verbs
  - [x] 4.1 Add cycle detection to `must_order` in `apply_overrides()`
    - After adding each `must_order` edge, run `has_cycle()` on slices
    - If cycle detected, revert the edge and emit `override-cycle` error diagnostic
    - Only add `override-must-order` reason when edge is accepted
    - Return `Vec<Diagnostic>` from `apply_overrides` (change signature)
    - _Requirements: 10.1, 10.2, 10.3_

  - [x] 4.2 Write property tests for override application
    - **Property 8: Override Idempotence**
    - **Validates: Requirements 11.1**
    - **Property 9: must_link Consolidation**
    - **Validates: Requirements 7.1, 7.2, 7.4**
    - **Property 10: force_members Placement**
    - **Validates: Requirements 8.1, 8.3**
    - **Property 11: rename_slices Title Update**
    - **Validates: Requirements 9.1**
    - **Property 12: must_order Edge Addition**
    - **Validates: Requirements 10.1, 10.3**

- [x] 5. Checkpoint — override engine
  - Ensure all tests pass, ask the user if questions arise.

- [x] 6. Validate command promotion and exit codes
  - [x] 6.1 Add `ExitCode` enum and refactor CLI main to use stable exit codes
    - Define `ExitCode` enum in `stackcut-cli` (0=success, 1=structural, 2=recomposition, 3=override, 4=unsupported, 10=internal)
    - Refactor `main()` to wrap `run()` and map errors to exit codes
    - _Requirements: 3.4_

  - [x] 6.2 Promote `cmd_validate` with version/schema enforcement and fingerprint check
    - Check `plan.version` against `PLAN_VERSION`; exit 1 on mismatch
    - Verify fingerprint if present; emit warning on mismatch
    - Exit 1 on structural errors, skip exact recomposition
    - Exit 2 on recomposition failure
    - _Requirements: 3.1, 3.2, 3.3, 12.1, 12.2, 12.3, 12.4, 12.5, 13.1, 13.2_

  - [x] 6.3 Add `--dry-run` flag to `materialize` subcommand
    - Wire `--dry-run` through to `stackcut_git::materialize_patches`
    - _Requirements: 19.1_

  - [x] 6.4 Write property test for unsupported plan version rejection
    - **Property 22: Unsupported Plan Version Rejection**
    - **Validates: Requirements 13.1**

- [x] 7. Planner deepening
  - [x] 7.1 Emit `prep-refactor` slice kind for rename-only mechanical slices
    - In the planner, after collecting mechanical IDs, check if all members are rename-only
    - Set `SliceKind::PrepRefactor` if all renames, keep `SliceKind::Mechanical` otherwise
    - _Requirements: 14.1, 14.2_

  - [x] 7.2 Write property test for prep-refactor vs mechanical kind
    - **Property 13: prep-refactor vs Mechanical Kind**
    - **Validates: Requirements 14.1, 14.2**

  - [x] 7.3 Add review budget diagnostic
    - After all slices are built, check each slice's member count against `review_budget` (default 15)
    - Emit `review-budget-exceeded` warning for oversized slices
    - _Requirements: 15.1, 15.3_

  - [x] 7.4 Write property test for review budget diagnostic
    - **Property 14: Review Budget Diagnostic**
    - **Validates: Requirements 15.1**

  - [x] 7.5 Strengthen ownership inference for docs/tests/generated files
    - Implement `infer_owner_by_path_segment()` for path-segment matching before fallback to ambiguity
    - Integrate into the attachable-units loop in `plan()`
    - _Requirements: 16.1, 16.2, 16.3, 16.4_

  - [x] 7.6 Write property test for ownership inference
    - **Property 15: Ownership Inference Attachment**
    - **Validates: Requirements 16.1, 16.2, 16.3**

  - [x] 7.7 Model unsupported Git surfaces in the planner
    - In `plan()`, detect units with `unsupported-*` notes and emit matching warning diagnostics
    - Assign unsupported units to a dedicated `misc` slice instead of dropping them
    - _Requirements: 17.1, 17.2, 17.3, 17.4, 17.5_

  - [x] 7.8 Write property test for unsupported surface handling
    - **Property 16: Unsupported Surface Handling**
    - **Validates: Requirements 17.1, 17.2, 17.3, 17.4, 17.5**

- [x] 8. Checkpoint — planner deepening
  - Ensure all tests pass, ask the user if questions arise.

- [x] 9. Git edge hardening
  - [x] 9.1 Handle rename-with-edit, copy, and type-change in `collect_edit_units`
    - Parse similarity index from rename status codes (e.g. `R095`)
    - Classify rename with similarity < 100% as `behavior` kind, not `mechanical`
    - Handle `T` (type-change) as `ChangeStatus::Unknown` with diagnostic
    - Tag binary, submodule, symlink, mode-only changes with `unsupported-*` notes
    - _Requirements: 18.1, 18.2, 18.3_

  - [x] 9.2 Write property tests for rename/copy/type-change classification
    - **Property 17: Rename Similarity Classification**
    - **Validates: Requirements 18.1**
    - **Property 18: Copy and Type-Change Classification**
    - **Validates: Requirements 18.2, 18.3**

  - [x] 9.3 Implement dry-run materialization with rollback
    - Add `dry_run: bool` parameter to `materialize_patches` in `stackcut-git`
    - In dry-run mode, write to a temp dir and verify patches apply with `git apply --check`
    - In normal mode, rollback (remove partially written patches) on failure
    - Report which patch failed and the Git error message
    - _Requirements: 19.1, 19.2, 19.3_

- [x] 10. Checkpoint — git edge hardening
  - Ensure all tests pass, ask the user if questions arise.

- [x] 11. Artifact fingerprint and richer diagnostics
  - [x] 11.1 Add `fingerprint` field to `Plan` struct
    - Add `fingerprint: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
    - Update `schema/stackcut.plan.schema.json` to include the `fingerprint` property
    - _Requirements: 24.1_

  - [x] 11.2 Implement `compute_fingerprint()` in `stackcut-artifact`
    - Add `sha2` dependency to `stackcut-artifact`
    - Compute SHA-256 of plan JSON with `fingerprint` field set to `None`
    - Wire fingerprint computation into `write_plan`
    - _Requirements: 24.1_

  - [x] 11.3 Write property test for fingerprint verification
    - **Property 24: Fingerprint Verification**
    - **Validates: Requirements 24.1**

  - [x] 11.4 Implement `DiagnosticsEnvelope` and `write_diagnostics_envelope()` in `stackcut-artifact`
    - Define `DiagnosticsEnvelope` with `source_base`, `source_head`, `generated_at` (ISO-8601), `counts`, `diagnostics`
    - Add `chrono` dependency to `stackcut-artifact`
    - Replace `write_diagnostics` call in CLI with `write_diagnostics_envelope`
    - _Requirements: 25.1, 25.2_

  - [x] 11.5 Write property test for diagnostics envelope completeness
    - **Property 23: Diagnostics Envelope Completeness**
    - **Validates: Requirements 25.1, 25.2**

- [x] 12. Checkpoint — artifacts
  - Ensure all tests pass, ask the user if questions arise.

- [x] 13. Proof surface — round-trip properties and planner invariants
  - [x] 13.1 Add `proptest` dev-dependency to `stackcut-core`, `stackcut-artifact`, `stackcut-git`, `stackcut-cli`
    - Add `proptest` to `[workspace.dependencies]` and each crate's `[dev-dependencies]`
    - _Requirements: 21.1_

  - [x] 13.2 Implement proptest `Arbitrary` generators in `stackcut-core`
    - `arb_edit_unit()` — random EditUnit with valid id, path, status, kind, family
    - `arb_edit_units()` — Vec of 1–50 random EditUnits with unique IDs
    - `arb_overrides(units, slices)` — random Overrides referencing existing unit/slice IDs
    - `arb_config()` — random StackcutConfig with valid fields
    - Place generators in a `testutil` module gated behind `#[cfg(test)]` or a shared test helper
    - _Requirements: 21.1, 21.2, 21.3, 21.4_

  - [x] 13.3 Write property tests for planner invariants in `stackcut-core`
    - **Property 4: No-Loss No-Duplication Invariant**
    - **Validates: Requirements 21.1**
    - **Property 5: Acyclic Dependency Graph**
    - **Validates: Requirements 21.2**
    - **Property 6: Planner Determinism**
    - **Validates: Requirements 21.3**
    - **Property 7: Override Preserves No-Loss No-Duplication**
    - **Validates: Requirements 21.4**

  - [x] 13.4 Write round-trip property tests in `stackcut-artifact`
    - **Property 1: Plan JSON Round-Trip**
    - **Validates: Requirements 26.1**
    - **Property 2: Overrides TOML Round-Trip**
    - **Validates: Requirements 6.5, 26.2**
    - **Property 3: StackcutConfig TOML Round-Trip**
    - **Validates: Requirements 26.3**

- [x] 14. Checkpoint — property tests
  - Ensure all tests pass, ask the user if questions arise.

- [x] 15. Proof surface — golden tests, snapshot tests, and integration tests
  - [x] 15.1 Implement fixture-driven golden tests
    - Iterate `fixtures/cases/*/`, load `input.units.json`, run planner with default config and empty overrides
    - Compare resulting plan's slices, members, dependencies, kinds, and ambiguities to `expected.plan.json`
    - Report diff on mismatch
    - _Requirements: 20.1, 20.2, 20.3_

  - [x] 15.2 Write property test for golden fixture match
    - **Property 25: Golden Fixture Match**
    - **Validates: Requirements 20.1**

  - [x] 15.3 Write snapshot tests for artifacts
    - Snapshot-test `render_summary` output for each fixture case
    - Snapshot-test serialized diagnostics output for each fixture case
    - Snapshot-test CLI `--help` output for each subcommand
    - _Requirements: 22.1, 22.2, 22.3_

  - [x] 15.4 Implement temp-repo integration tests
    - Create temporary Git repos with known base and head commits
    - Run full `plan → materialize → validate --exact` pipeline
    - Cover scenarios: simple add, simple modify, rename, multi-family split
    - _Requirements: 23.1, 23.2, 23.3, 23.4_

  - [x] 15.5 Write property test for end-to-end recomposition
    - **Property 26: End-to-End Recomposition**
    - **Validates: Requirements 23.3**

- [x] 16. Final wiring and integration
  - [x] 16.1 Wire `parse_config` into CLI `cmd_plan`
    - Replace `load_toml_or_default::<StackcutConfig>` with `parse_config` call
    - Print config diagnostics to stderr
    - _Requirements: 4.1, 4.2, 4.3_

  - [x] 16.2 Wire fingerprint verification into `cmd_validate`
    - After loading plan, verify fingerprint if present
    - Emit warning on mismatch
    - _Requirements: 24.2_

  - [x] 16.3 Wire diagnostics envelope into `cmd_plan`
    - Replace `write_diagnostics` with `write_diagnostics_envelope`
    - _Requirements: 25.1, 25.2_

  - [x] 16.4 Update fixture `expected.plan.json` files for new `fingerprint` field
    - Regenerate expected plans for all fixtures under `fixtures/cases/`
    - _Requirements: 20.1_

- [x] 17. Final checkpoint — Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP
- Each task references specific requirements for traceability
- Checkpoints ensure incremental validation
- Property tests use `proptest` with minimum 100 iterations per property
- Each property test is tagged: `Feature: stackcut-v01-completion, Property {N}: {text}`
- Build order: contract drift → overrides → validation → planner → git edge → proof surface → artifacts
- Dependency direction preserved: `stackcut-core` depends on no local crate, no reverse edges
