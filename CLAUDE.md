# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

Stackcut is a deterministic diff-to-stack compiler. It turns one oversized change into a reviewable stack of slices with portable artifacts (plan.json, summary.md, diagnostics.json, patch series). It is file-scoped (v0.1) — no intra-file hunk slicing.

## Commands

All stable commands go through xtask:

```bash
cargo run -p xtask -- ci-fast       # fmt + clippy + core tests (local quick bar)
cargo run -p xtask -- ci-full       # fmt + clippy + all tests (merge bar)
cargo run -p xtask -- smoke         # all workspace tests
cargo run -p xtask -- golden        # artifact/fixture tests
cargo run -p xtask -- mutants       # cargo-mutants (300s timeout)
cargo run -p xtask -- docs-check    # documentation validation
cargo run -p xtask -- release-check # full validation pre-release
```

Run a single crate's tests: `cargo test -p stackcut-core`

Run a single test: `cargo test -p stackcut-core -- test_name`

A `justfile` also aliases these xtask commands.

## Workspace structure

Five crates with strict dependency direction — **no reverse edges into stackcut-core**:

```
stackcut-core    (pure IR, config, planner, validation — no local deps)
  ↑         ↑
stackcut-git    stackcut-artifact
  (Git ingest,    (plan JSON IO,
   patches,        markdown render,
   recomposition)  diagnostics)
       ↑    ↑       ↑
        stackcut-cli
        (commands, orchestration, exit codes)

xtask (repo rituals — no local deps)
```

All crates under `crates/` except xtask at root. Each crate currently uses a single `lib.rs` (or `main.rs` for cli).

## Data flow

```
git worktree → stackcut-git (collect_edit_units)
            → stackcut-core (plan solver)
            → stackcut-artifact (render artifacts)
            → stackcut-git (materialize_patches, validate_exact_recomposition)
```

## Key design constraints

- **Deterministic**: Same input always produces same output. No hidden nondeterminism, no timestamps in plan artifacts.
- **Planner rules**: Every unit appears exactly once. Slices form acyclic DAG. Manifest+lock move together. Tests/docs attach to code family. Generated files follow source family. Ambiguous roots stay explicit — do not guess silently.
- **Override model**: `must_link`, `force_members`, `rename_slices`, `must_order` in override.toml. Overrides are replayable, not a hidden rule engine.
- **Trust boundary**: Normalized edits, classification, rule-based planning, structural validation, and exact recomposition are inside. AI suggestions and semantic adapters are outside.
- **Schemas are contracts**: `schema/stackcut.plan.schema.json` and `schema/stackcut.override.schema.json` are versioned contracts.

## Fixture-first workflow

When planner behavior changes:
1. Update or add a case under `fixtures/cases/`
2. Update `docs/SCENARIO_ATLAS.md` if the case is new
3. Update `expected.plan.json`
4. Rerun tests

Five canonical cases in `fixtures/cases/`: feature-plus-refactor, generated-follows-source, manifest-lockfile, docs-and-tests-attach, ambiguous-root-doc.

## Merge bars

- **Normal change**: `ci-fast`, affected docs/fixtures updated, no unexplained snapshot drift
- **Behavior change**: `ci-full`, fixture case added/updated, ambiguity reasoning documented
- **Release candidate**: `release-check`, manual dry-run against real repo, schema review

## When to stop and escalate

- A change would require intra-file hunk slicing
- Exact recomposition no longer holds
- A new dependency direction is needed
- A schema change breaks the current plan format
- Behavior cannot be made deterministic
