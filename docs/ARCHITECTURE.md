# Architecture

`stackcut` is a **repo-ops compiler**.

It does not replace stacked-PR tools, smartlogs, or review hosting. It sits upstream of them and turns one overloaded change into a deterministic, reviewable stack with portable artifacts.

## Product boundary

`stackcut` owns:

- diff ingest
- deterministic slice planning
- explanation
- validation
- patch materialization

It does not own:

- review hosting
- merge queues
- PR stack management
- source control replacement

## Compiler shape

```text
git range / worktree
    ↓
edit normalization
    ↓
classification
    ↓
constraint building
    ↓
slice solving
    ↓
explanation + validation
    ↓
artifacts + patches
```

## Trust boundary

The trust boundary is deterministic.

Inside the trust boundary:

- normalized edit units
- path/family classification
- rule-based slice planning
- structural validation
- exact recomposition

Outside the trust boundary:

- future semantic language adapters
- future AI-generated titles or alternate suggestions
- downstream review workflow integrations

## Current granularity

v0.1 is intentionally **file-scoped**.

That buys four things:

1. a clean, explainable planner
2. exact recomposition by patch application
3. stable portable artifacts
4. bounded blast radius for overrides

It does not solve intra-file mixed concerns. That is a future expansion.

## Crate roles

### `stackcut-core`

Pure planning core.

Responsibilities:

- plan IR
- config and override models
- path classification
- family inference
- slice planning
- structural validation

### `stackcut-git`

Translation edge to Git.

Responsibilities:

- discover repo root
- collect changed paths
- map Git changes into `EditUnit`
- materialize patch series
- validate exact recomposition

### `stackcut-artifact`

Artifact surface.

Responsibilities:

- plan JSON IO
- diagnostics IO
- Markdown rendering
- summary stability

### `stackcut-cli`

Operator surface.

Responsibilities:

- parse commands
- resolve config and overrides
- orchestrate planning and output
- expose dry-run friendly flows

### `xtask`

Repo ritual control plane.

Responsibilities:

- stable local commands
- CI parity
- doc and release checks

## Planner rules

The planner is intentionally small and explicit.

### Hard rules

- every unit appears exactly once
- slices form an acyclic dependency graph
- manifest and lock files move together
- tests and docs attach to their code family when possible
- generated outputs follow their family when possible
- ambiguous unattached roots stay explicit

### Soft goals

- keep slices reviewable
- keep dependencies shallow
- prefer prep/config before behavior
- preserve stable ordering

## Override model

Overrides are a release valve, not a hidden rule engine.

v0.1 supports:

- `must_link`
- `force_members`
- `rename_slices`
- `must_order`

Overrides are replayable and live in `override.toml`.

## Evidence surface

Artifacts are first-class:

- `plan.json`
- `summary.md`
- `diagnostics.json`
- patch series

These are meant to be reviewed and shared in CI and handoffs.

## Why the repo is documentation-heavy

The repo is structured to be teachable:

- `AGENTS.md` gives machine-facing guidance
- `TESTING.md` and `RELEASE.md` pin proof surfaces
- fixtures and the scenario atlas make planner semantics discoverable
- ADRs explain non-obvious trade-offs

That is not support material. It is part of the architecture.
