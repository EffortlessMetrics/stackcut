# Requirements Document

## Introduction

stackcut v0.1 has a structurally correct scaffold — four crates, a CLI surface, fixture corpus, schemas, and documentation — but the load-bearing middle is operationally incomplete. This spec covers finishing the v0.1 build: fixing contract drift between docs and code, implementing the override engine end-to-end, promoting exact recomposition into the validate command, deepening the proof surface, and hardening the planner and Git edge for real-world inputs.

The build order is: contract drift → overrides → validation promotion → proof surface → planner deepening → Git edge hardening → artifact completion.

## Glossary

- **Planner**: The deterministic slice-solving engine in `stackcut-core` that assigns edit units to ordered slices.
- **Edit_Unit**: A single file-level change record with id, path, status, kind, and family.
- **Slice**: An ordered group of edit units that forms one reviewable PR-sized change.
- **Override_Engine**: The subsystem that parses `override.toml` and applies `must_link`, `force_members`, `rename_slices`, and `must_order` directives to a plan.
- **Exact_Recomposition**: The proof that applying the materialized patch series to the base revision produces a tree identical to the head revision.
- **Structural_Validation**: Checks that every unit is assigned exactly once, no slice is empty, dependencies are acyclic, and all referenced IDs exist.
- **CLI**: The `stackcut-cli` crate binary that exposes `plan`, `explain`, `validate`, and `materialize` subcommands.
- **Xtask**: The `xtask` crate that provides stable repo-ritual commands (`ci-fast`, `ci-full`, `smoke`, `golden`, `mutants`, `docs-check`, `release-check`).
- **Fixture**: A canonical test case under `fixtures/cases/` consisting of `input.units.json`, `expected.plan.json`, and `README.md`.
- **Ambiguity**: An explicit record emitted when the Planner cannot confidently assign an edit unit to a single slice.
- **Proof_Surface**: Metadata on each slice listing scenario IDs and expected verification commands.
- **Diagnostics**: Structured error/warning/note records emitted during planning and validation.
- **Config_Parser**: The subsystem that reads `stackcut.toml` into a `StackcutConfig` struct.
- **Override_Parser**: The subsystem that reads `override.toml` into an `Overrides` struct.
- **Unsupported_Surface**: Git changes that v0.1 cannot slice (binary files, submodules, symlinks, mode-only changes).
- **Review_Budget**: A configurable upper bound on the number of files per slice, used as a diagnostic signal.

## Requirements

### Requirement 1: Xtask Alias Configuration

**User Story:** As a developer, I want `cargo xtask` to work as documented in BUILD_NOTES.md, so that I can run repo rituals without remembering the `-p xtask` invocation.

#### Acceptance Criteria

1. THE CLI SHALL provide a `.cargo/config.toml` file that aliases `cargo xtask` to `cargo run --package xtask --`.
2. WHEN a developer runs `cargo xtask ci-fast`, THE Xtask SHALL execute the same sequence as `cargo run -p xtask -- ci-fast`.

### Requirement 2: CI Workflow Tool Installation

**User Story:** As a maintainer, I want the CI workflow to install required external tools before invoking them, so that `mutants` and `fuzz` xtask commands do not fail in CI.

#### Acceptance Criteria

1. WHEN the CI workflow runs the `mutants` task, THE CI_Workflow SHALL install `cargo-mutants` before execution.
2. WHEN the CI workflow runs fuzz targets, THE CI_Workflow SHALL install `cargo-fuzz` before execution.
3. THE CI_Workflow SHALL cache installed tools to avoid redundant downloads on subsequent runs.

### Requirement 3: CLI --exact Flag Contract

**User Story:** As a user, I want `stackcut validate --exact` to perform both structural validation and exact recomposition in a single command, so that the README contract is honored.

#### Acceptance Criteria

1. WHEN the `--exact` flag is passed to `validate`, THE CLI SHALL run structural validation followed by exact recomposition validation.
2. WHEN structural validation produces errors, THE CLI SHALL report the structural errors and skip exact recomposition.
3. WHEN exact recomposition fails, THE CLI SHALL exit with a distinct non-zero exit code.
4. THE CLI SHALL use stable exit codes: 0 for success, 1 for structural errors, 2 for recomposition failure, 3 for override conflict, 4 for unsupported Git surface, 10 for internal bug.

### Requirement 4: Config Parser Alignment

**User Story:** As a user, I want the `stackcut.toml` parser to accept the documented sample configuration without error, so that the README config example is a working contract.

#### Acceptance Criteria

1. THE Config_Parser SHALL parse the `stackcut.toml` format documented in README.md without error.
2. WHEN a `stackcut.toml` contains unknown keys, THE Config_Parser SHALL emit a warning diagnostic rather than silently ignoring the keys.
3. THE Config_Parser SHALL reject a `stackcut.toml` with `version` greater than the supported version and return a descriptive error.

### Requirement 5: Documentation Path Accuracy

**User Story:** As a developer or agent, I want all cross-references in documentation to point to files that exist, so that the repo is navigable without dead links.

#### Acceptance Criteria

1. THE Xtask `docs-check` command SHALL verify that every file path referenced in `README.md`, `AGENTS.md`, `TESTING.md`, `RELEASE.md`, and `docs/ARCHITECTURE.md` exists in the repository.
2. WHEN a referenced path does not exist, THE Xtask SHALL report the broken reference and exit with a non-zero code.

### Requirement 6: Override Engine — Parsing

**User Story:** As a user, I want to write an `override.toml` file that the system parses into a validated override model, so that I can control slice membership and ordering.

#### Acceptance Criteria

1. THE Override_Parser SHALL parse `override.toml` files conforming to `schema/stackcut.override.schema.json` into an `Overrides` struct.
2. WHEN an `override.toml` references a member ID that does not exist in the current plan's units, THE Override_Parser SHALL emit a warning diagnostic listing the unknown member IDs.
3. WHEN an `override.toml` references a slice ID that does not exist in the current plan, THE Override_Parser SHALL emit a warning diagnostic listing the unknown slice IDs.
4. WHEN an `override.toml` contains a `must_link` group with fewer than two members, THE Override_Parser SHALL emit a warning diagnostic.
5. FOR ALL valid Overrides structs, serializing to TOML then parsing back SHALL produce an equivalent Overrides struct (round-trip property).

### Requirement 7: Override Engine — must_link Application

**User Story:** As a user, I want `must_link` overrides to force specified members into the same slice, so that I can keep related changes together.

#### Acceptance Criteria

1. WHEN a `must_link` override lists members currently in different slices, THE Planner SHALL move all listed members into a single slice.
2. THE Planner SHALL choose the anchor slice as the slice containing the first listed member that is already assigned.
3. WHEN no listed member is currently assigned, THE Planner SHALL create a new override slice and assign all listed members to the new slice.
4. THE Planner SHALL record an `override-must-link` inclusion reason on the anchor slice with the user-provided reason or a default message.

### Requirement 8: Override Engine — force_members Application

**User Story:** As a user, I want `force_members` overrides to place a specific member into a named slice, so that I can resolve ambiguities explicitly.

#### Acceptance Criteria

1. WHEN a `force_members` override specifies a member and a target slice, THE Planner SHALL remove the member from its current slice and add the member to the target slice.
2. WHEN the target slice does not exist, THE Planner SHALL create the target slice before placing the member.
3. THE Planner SHALL record an `override-force-member` inclusion reason on the target slice.

### Requirement 9: Override Engine — rename_slices Application

**User Story:** As a user, I want `rename_slices` overrides to change the display title of a slice, so that I can customize PR titles.

#### Acceptance Criteria

1. WHEN a `rename_slices` override specifies a slice ID and a new title, THE Planner SHALL update the title of the matching slice.
2. WHEN the specified slice ID does not exist, THE Planner SHALL emit a warning diagnostic and skip the rename.

### Requirement 10: Override Engine — must_order Application

**User Story:** As a user, I want `must_order` overrides to add dependency edges between slices, so that I can enforce landing order.

#### Acceptance Criteria

1. WHEN a `must_order` override specifies a `before` and `after` slice, THE Planner SHALL add a dependency edge from `after` to `before`.
2. IF a `must_order` override would create a cycle in the dependency graph, THEN THE Planner SHALL reject the override and emit an error diagnostic identifying the cycle.
3. THE Planner SHALL record an `override-must-order` inclusion reason on the `after` slice.

### Requirement 11: Override Replay Stability

**User Story:** As a user, I want override application to be deterministic, so that the same inputs always produce the same plan.

#### Acceptance Criteria

1. FOR ALL combinations of a valid plan and valid overrides, applying overrides twice to the same input SHALL produce identical plans (idempotence property).
2. THE Override_Engine SHALL apply overrides in a fixed order: `must_link`, then `force_members`, then `rename_slices`, then `must_order`.

### Requirement 12: Validate Command — Structural and Exact Modes

**User Story:** As a user, I want the `validate` command to own both structural and exact recomposition validation, so that I have a single entry point for plan correctness.

#### Acceptance Criteria

1. WHEN `validate` is called without `--exact`, THE CLI SHALL perform structural validation only and report diagnostics.
2. WHEN `validate` is called with `--exact`, THE CLI SHALL perform structural validation first, then exact recomposition validation.
3. WHEN structural validation finds errors, THE CLI SHALL exit with code 1 and skip exact recomposition.
4. WHEN exact recomposition fails, THE CLI SHALL exit with code 2 and report the tree mismatch.
5. WHEN validation succeeds, THE CLI SHALL exit with code 0.

### Requirement 13: Validate Command — Schema and Version Enforcement

**User Story:** As a user, I want the validate command to reject plans with unsupported versions, so that I do not silently consume incompatible artifacts.

#### Acceptance Criteria

1. WHEN a plan's `version` field does not match the supported version, THE CLI SHALL exit with a descriptive error and a non-zero exit code.
2. THE CLI SHALL validate that all required fields defined in `schema/stackcut.plan.schema.json` are present in the plan.

### Requirement 14: Planner — prep-refactor Slice Emission

**User Story:** As a developer, I want the planner to emit `prep-refactor` slices for rename-only mechanical changes, so that the slice kind accurately reflects the intent.

#### Acceptance Criteria

1. WHEN all members of a mechanical slice are rename-only changes, THE Planner SHALL emit the slice with kind `prep-refactor` instead of `mechanical`.
2. WHEN a mechanical slice contains a mix of renames and other changes, THE Planner SHALL keep the slice kind as `mechanical`.

### Requirement 15: Planner — Review Budget Diagnostic

**User Story:** As a user, I want the planner to warn when a slice exceeds a configurable file count, so that I can identify slices that may be too large for review.

#### Acceptance Criteria

1. WHEN a slice contains more members than the configured `review_budget` threshold, THE Planner SHALL emit a warning diagnostic with code `review-budget-exceeded`.
2. THE Config_Parser SHALL accept an optional `review_budget` field in `stackcut.toml` with a positive integer value.
3. WHEN `review_budget` is not configured, THE Planner SHALL use a default threshold of 15 files.

### Requirement 16: Planner — Stronger Ownership Inference for Docs, Tests, and Generated Files

**User Story:** As a user, I want docs, tests, and generated files to attach to their owning behavior family more reliably, so that fewer ambiguities are emitted.

#### Acceptance Criteria

1. WHEN a test file's path contains a segment matching a single behavior family name, THE Planner SHALL attach the test to that family's behavior slice.
2. WHEN a doc file resides under a path prefix that maps to a single behavior family, THE Planner SHALL attach the doc to that family's behavior slice.
3. WHEN a generated file's family matches exactly one behavior slice's family, THE Planner SHALL attach the generated file to that behavior slice.
4. WHEN ownership remains ambiguous after inference, THE Planner SHALL emit an Ambiguity record and leave the file in a standalone slice.

### Requirement 17: Planner — Unsupported Surface Modeling

**User Story:** As a user, I want the planner to explicitly flag Git changes it cannot slice, so that I know which files were excluded and why.

#### Acceptance Criteria

1. WHEN a diff contains a binary file change, THE Planner SHALL emit a warning diagnostic with code `unsupported-binary`.
2. WHEN a diff contains a submodule change, THE Planner SHALL emit a warning diagnostic with code `unsupported-submodule`.
3. WHEN a diff contains a symlink change, THE Planner SHALL emit a warning diagnostic with code `unsupported-symlink`.
4. WHEN a diff contains a mode-only change (no content diff), THE Planner SHALL emit a warning diagnostic with code `unsupported-mode-only`.
5. THE Planner SHALL assign unsupported units to a dedicated `misc` slice rather than silently dropping the units.

### Requirement 18: Git Edge — Rename, Copy, and Type-Change Handling

**User Story:** As a user, I want the Git ingest layer to correctly handle renames, copies, rename-plus-edit, and type changes, so that the planner receives accurate edit units.

#### Acceptance Criteria

1. WHEN Git reports a rename with similarity index below 100%, THE Git_Ingest SHALL classify the unit as `behavior` rather than `mechanical`.
2. WHEN Git reports a copy, THE Git_Ingest SHALL create an edit unit with status `copied` and record the old path.
3. WHEN Git reports a type change (file to symlink or vice versa), THE Git_Ingest SHALL classify the unit as `unknown` and emit a diagnostic.

### Requirement 19: Git Edge — Safer Branch Materialization

**User Story:** As a user, I want patch materialization to support dry-run mode and clean up on partial failure, so that my working tree is not left in a broken state.

#### Acceptance Criteria

1. WHEN `materialize` is called with a `--dry-run` flag, THE CLI SHALL verify that all patches apply cleanly without writing output files.
2. IF a patch fails to apply during materialization, THEN THE CLI SHALL remove any partially written patches from the output directory and exit with a non-zero code.
3. THE CLI SHALL report which patch failed and the Git error message.

### Requirement 20: Proof Surface — Fixture-Driven Golden Tests

**User Story:** As a developer, I want golden tests that load each fixture's `input.units.json`, run the planner, and compare the output to `expected.plan.json`, so that planner regressions are caught automatically.

#### Acceptance Criteria

1. FOR ALL fixture cases under `fixtures/cases/`, THE Test_Suite SHALL load `input.units.json`, run the Planner with default config and empty overrides, and compare the resulting plan to `expected.plan.json`.
2. WHEN the planner output differs from the expected plan, THE Test_Suite SHALL report the diff and fail.
3. THE Test_Suite SHALL compare exact member IDs, slice IDs, dependency edges, slice kinds, and ambiguity records.

### Requirement 21: Proof Surface — Property Tests for Planner Invariants

**User Story:** As a developer, I want property-based tests that verify planner invariants hold for arbitrary inputs, so that edge cases are discovered automatically.

#### Acceptance Criteria

1. FOR ALL valid sets of edit units, THE Property_Tests SHALL verify that every unit ID appears in exactly one slice (no loss, no duplication).
2. FOR ALL valid sets of edit units, THE Property_Tests SHALL verify that the slice dependency graph is acyclic.
3. FOR ALL valid sets of edit units, THE Property_Tests SHALL verify that the planner output is deterministic (same input produces identical output).
4. FOR ALL valid sets of edit units and valid overrides, THE Property_Tests SHALL verify that override application preserves the no-loss and no-duplication invariants.

### Requirement 22: Proof Surface — Artifact Snapshot Tests

**User Story:** As a developer, I want snapshot tests for rendered artifacts, so that summary.md, diagnostics.json, and CLI help output remain stable across changes.

#### Acceptance Criteria

1. THE Test_Suite SHALL snapshot-test the `render_summary` output for each fixture case.
2. THE Test_Suite SHALL snapshot-test the serialized diagnostics output for each fixture case.
3. THE Test_Suite SHALL snapshot-test the CLI `--help` output for each subcommand.

### Requirement 23: Proof Surface — Temp-Repo Integration Tests

**User Story:** As a developer, I want integration tests that create temporary Git repos, run the full plan/materialize/validate pipeline, and verify exact recomposition, so that the end-to-end contract is tested without relying on external repos.

#### Acceptance Criteria

1. THE Integration_Tests SHALL create a temporary Git repository with a known base commit and a known head commit.
2. THE Integration_Tests SHALL run `plan`, `materialize`, and `validate --exact` against the temporary repository.
3. WHEN the pipeline completes, THE Integration_Tests SHALL verify that exact recomposition succeeds (applied patches produce the head tree).
4. THE Integration_Tests SHALL cover at least: a simple add, a simple modify, a rename, and a multi-family split.

### Requirement 24: Artifact — Plan Fingerprint

**User Story:** As a user, I want each plan to include a self-hash fingerprint, so that I can detect if a plan file has been tampered with or corrupted.

#### Acceptance Criteria

1. THE Artifact_Writer SHALL compute a SHA-256 hash of the plan content (excluding the fingerprint field itself) and include the hash in a `fingerprint` field.
2. WHEN `validate` loads a plan with a `fingerprint` field, THE CLI SHALL verify the fingerprint matches the computed hash and emit a warning if the fingerprint is invalid.

### Requirement 25: Artifact — Richer Diagnostics

**User Story:** As a user, I want diagnostics.json to include the plan source metadata and a summary count, so that diagnostics are self-contained and useful in CI.

#### Acceptance Criteria

1. THE Diagnostics_Writer SHALL include `source.base`, `source.head`, and a count of errors, warnings, and notes in the diagnostics output.
2. THE Diagnostics_Writer SHALL include a `generated_at` ISO-8601 timestamp in the diagnostics output.

### Requirement 26: Plan JSON Round-Trip

**User Story:** As a developer, I want plan serialization and deserialization to be lossless, so that reading a written plan produces an identical struct.

#### Acceptance Criteria

1. FOR ALL valid Plan structs, writing to JSON then reading back SHALL produce an equivalent Plan struct (round-trip property).
2. FOR ALL valid Overrides structs, writing to TOML then reading back SHALL produce an equivalent Overrides struct (round-trip property).
3. FOR ALL valid StackcutConfig structs, writing to TOML then reading back SHALL produce an equivalent StackcutConfig struct (round-trip property).
