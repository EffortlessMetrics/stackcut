# AGENTS

This repo is built as a **delegation-aware diff-to-stack compiler**. Work from artifacts and scenario surfaces first.

## First read

1. `README.md`
2. `docs/ARCHITECTURE.md`
3. `docs/SCENARIO_ATLAS.md`
4. `TESTING.md`
5. `RELEASE.md`

## Crate map

- `stackcut-core`: pure IR, config, planner, structural validation
- `stackcut-git`: Git ingest, patch generation, exact recomposition
- `stackcut-artifact`: stable output and summary rendering
- `stackcut-cli`: operator surface
- `xtask`: repo rituals

## Dependency direction

- `stackcut-core` depends on no local crate
- `stackcut-git` depends on `stackcut-core`
- `stackcut-artifact` depends on `stackcut-core`
- `stackcut-cli` depends on all three
- `xtask` depends on no local crate

Do not introduce reverse edges into `stackcut-core`.

## Required commands

Use the stable command surface (`cargo xtask` is an alias defined in `.cargo/config.toml`):

- `cargo xtask ci-fast`
- `cargo xtask ci-full`
- `cargo xtask smoke`
- `cargo xtask golden`
- `cargo xtask mutants`
- `cargo xtask fuzz`
- `cargo xtask docs-check`
- `cargo xtask release-check`

## How to work

- Prefer deterministic rules to heuristics that cannot explain themselves.
- Keep Git plumbing at the edges.
- Add or update fixtures whenever the planner changes.
- Treat `schema/` and Markdown artifacts as contracts.
- When the planner is uncertain, surface ambiguity. Do not guess silently.
- Keep AI outside the trust boundary.

## When to stop and escalate

Stop and leave a note when:

- a change would require intra-file hunk slicing instead of file-scoped slicing
- exact recomposition no longer holds
- a new dependency direction is needed
- a schema change breaks the current plan format
- behavior cannot be made deterministic
