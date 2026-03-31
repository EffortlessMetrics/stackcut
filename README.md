# stackcut

`stackcut` is a deterministic **diff-to-stack compiler**.

It takes one oversized change, turns it into a reviewable stack, proves the stack covers the original change exactly at the selected granularity, and emits portable artifacts a human, CI job, or agent can trust.

## What v0.1 does

This starter build is intentionally narrow:

- plans **file-scoped** stacks from a Git base/head range
- groups changed files into ordered slices
- emits a portable `plan.json`, `summary.md`, and `diagnostics.json`
- materializes a patch series for each slice
- validates structural invariants and, when possible, exact recomposition by applying the generated patches to the base revision and comparing the resulting tree to `head`

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

```bash
stackcut plan --base <rev> --head <rev>
stackcut explain .stackcut/plan.json
stackcut validate .stackcut/plan.json --exact
stackcut materialize .stackcut/plan.json --out .stackcut/patches
```

`plan` writes three artifacts by default:

- `.stackcut/plan.json`
- `.stackcut/summary.md`
- `.stackcut/diagnostics.json`

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
