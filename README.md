# stackcut

[![CI](https://github.com/EffortlessMetrics/stackcut/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/EffortlessMetrics/stackcut/actions/workflows/ci.yml)
[![Coverage](https://github.com/EffortlessMetrics/stackcut/actions/workflows/coverage.yml/badge.svg?branch=main)](https://github.com/EffortlessMetrics/stackcut/actions/workflows/coverage.yml)
[![Codecov](https://codecov.io/gh/EffortlessMetrics/stackcut/branch/main/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/stackcut)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-blue.svg)](Cargo.toml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE)

`stackcut` is a deterministic **diff-to-stack compiler**.

It takes one oversized change, turns it into a reviewable stack, proves the stack covers the original change exactly at the selected granularity, and emits portable artifacts a human, CI job, or agent can trust.

## What v0.1 does

This starter build is intentionally narrow:

- plans **file-scoped** stacks from a Git base/head range
- groups changed files into ordered slices
- emits a portable `plan.json`, `summary.md`, and `diagnostics.json`
- materializes a patch series for each slice
- validates structural invariants and, when possible, exact recomposition by applying the generated patches to the base revision and comparing the resulting tree to `head`

Codecov is Rust execution-surface telemetry only; see [Coverage](docs/ci/coverage.md) for what the badge does and does not claim.

## What v0.1 does not do

Not yet:

- intra-file hunk splitting
- semantic symbol slicing
- stacked pull-request automation
- workflow hosting
- autonomous change application

That is deliberate. The first trust bar is honesty and exactness, not semantic magic.

## Design center

`stackcut` is built as a compiler pipeline:

```text
git/worktree + repo rules + overrides
          ↓
     edit normalization
          ↓
 classification + constraints
          ↓
       slice solving
          ↓
  explain + validate + diagnose
          ↓
  artifacts + patch materialization
```

The current planner is deterministic, file-scoped, and override-friendly. It is designed to grow into deeper semantic slicing without changing the trust boundary.

## Workspace layout

```text
crates/
  stackcut-core       # IR, config, planner, structural validation
  stackcut-git        # git ingest, patch materialization, exact recomposition
  stackcut-artifact   # plan IO, markdown summaries, diagnostics
  stackcut-cli        # command surface
xtask/                # repo rituals
docs/                 # architecture, ADRs, scenario atlas, roadmap
schema/               # plan and override schemas
fixtures/             # canonical cases and expected plans
```

## CLI

The core pipeline is four commands:

```bash
stackcut plan --base <rev> --head <rev>
stackcut explain .stackcut/plan.json
stackcut validate .stackcut/plan.json --exact
stackcut materialize .stackcut/plan.json --out-dir .stackcut/patches
```

`plan` writes three artifacts by default:

- `.stackcut/plan.json`
- `.stackcut/summary.md`
- `.stackcut/diagnostics.json`

### Command reference

| Command | Key flags | What it does |
| --- | --- | --- |
| `plan` | `--base <rev> --head <rev>` `[--repo .]` `[--out-dir .stackcut]` `[--config <path>]` `[--overrides <path>]` `[--dry-run]` | Plan a file-scoped stack from a git range. `--dry-run` prints plan JSON to stdout without writing files. |
| `explain` | `<plan>` `[--why <slice>]` | Render a stored plan as Markdown. `--why` focuses one slice. |
| `validate` | `<plan>` `[--exact]` `[--receipt <path>]` `[--format text\|json]` | Structural validation; `--exact` also verifies exact recomposition. `--receipt` writes a recomposition receipt. |
| `materialize` | `<plan>` `[--out-dir .stackcut/patches]` `[--dry-run]` | Emit a patch series per slice. `--dry-run` validates application with rollback, writing nothing. |
| `doctor` | `[--repo .]` | Check repo readiness (git availability, clean state, config). |
| `compare` | `<old> <new>` `[--json]` | Diff two plans and report what changed. |
| `init` | `[--repo .]` `[--force]` | Scaffold a starter `stackcut.toml`. |
| `scaffold-overrides` | `<plan>` `[--output .stackcut/override.toml]` `[--force]` | Generate an `override.toml` skeleton from a plan's ambiguities. |
| `emit-sarif` | `<plan>` `[-o <path>]` | Emit diagnostics as SARIF 2.1.0 JSON for CI code-scanning. |
| `emit-proof` | `<plan>` `[-o <path>]` | Emit per-slice proof-surface hints. |
| `emit-review-packet` | `<plan>` `[-o <path>]` | Emit a PR-ready review packet (Markdown). |

### Exit codes

Every command resolves to exactly one stable exit code:

| Code | Meaning |
| --- | --- |
| `0` | Success |
| `1` | Structural error (plan failed structural validation) |
| `2` | Recomposition failure (`validate --exact` mismatch) |
| `3` | Override conflict |
| `4` | Unsupported git surface |
| `10` | Internal bug (unexpected error) |

## Quick start

1. Install Rust stable.
2. Run the fast repo checks:

   ```bash
   cargo run -p xtask -- ci-fast
   ```

3. Create a plan from a real repo range:

   ```bash
   cargo run -p stackcut-cli -- plan --base HEAD~1 --head HEAD
   ```

4. Inspect the stack:

   ```bash
   cargo run -p stackcut-cli -- explain .stackcut/plan.json
   ```

5. Materialize patches:

   ```bash
   cargo run -p stackcut-cli -- materialize .stackcut/plan.json --out .stackcut/patches
   ```

6. Validate structural invariants and exact recomposition:

   ```bash
   cargo run -p stackcut-cli -- validate .stackcut/plan.json --exact
   ```

## Current planning rules

The current planner enforces a small set of transparent rules:

- manifest and lock files move together
- generated files follow their family when possible
- tests and docs attach to the code family they validate when possible
- ops and config changes are isolated
- mechanical rename-only changes can peel off
- ambiguous root-level docs/tests are left explicit and overrideable

That gives a clean, reviewable v0.1 without pretending to solve all semantic slicing.

## Config

The repo root can include `stackcut.toml`:

```toml
version = 1
generated_prefixes = ["dist/", "generated/", "fixtures/generated/"]
manifest_files = ["Cargo.toml", "package.json", "pyproject.toml"]
lock_files = ["Cargo.lock", "package-lock.json", "pnpm-lock.yaml"]
test_prefixes = ["tests/", "specs/"]
doc_prefixes = ["docs/", "adr/"]
ops_prefixes = [".github/", "ci/", ".circleci/"]

[[path_families]]
prefix = "src/core/"
family = "core"

[[path_families]]
prefix = "src/git/"
family = "git"
```

Optional `.stackcut/override.toml` lets a human pin members together, force a member into a slice, rename a slice, or add an ordering edge.

## Why the repo looks this way

The repo is artifact-first and delegation-aware:

- scenarios and fixtures are first-class
- outputs are stable contracts
- local commands are part of the architecture
- documentation teaches the system, not just the commands
- the planner is pure enough to test hard, while Git stays at the edges

See:

- `docs/ARCHITECTURE.md`
- `docs/SCENARIO_ATLAS.md`
- `AGENTS.md`
- `TESTING.md`
- `RELEASE.md`

## Roadmap

The next meaningful expansions are:

1. hunk-scoped edit units
2. semantic language adapters
3. richer ambiguity modeling
4. branch-stack materialization
5. downstream exporters for review workflows
6. AI sidecars for titles and alternate cuts, kept outside the trust boundary
