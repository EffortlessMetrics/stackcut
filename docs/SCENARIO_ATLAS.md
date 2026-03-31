# Scenario atlas

The first five canonical fixture cases pin the planner's intended behavior.

| Case | Problem | Expected cut | Artifact |
|---|---|---|---|
| `01-feature-plus-refactor` | Prep refactor mixed with behavior and tests | rename/config before behavior | `expected.plan.json` |
| `02-generated-follows-source` | Generated outputs changed with their source | generated attached to source family | `expected.plan.json` |
| `03-manifest-lockfile` | Package metadata changed together | manifest + lock in one slice | `expected.plan.json` |
| `04-docs-and-tests-attach` | Docs/tests mixed with implementation | docs/tests attach to the implementation family | `expected.plan.json` |
| `05-ambiguous-root-doc` | Root docs changed with multiple behavior families | ambiguity surfaced, standalone docs slice | `expected.plan.json` |

## How to add a scenario

1. Create a new folder under `fixtures/cases/`.
2. Add `input.units.json`.
3. Add `expected.plan.json`.
4. Add a short `README.md` with the problem statement.
5. Update this atlas.
6. Add or update tests that pin the behavior.

## Why this matters

The scenario atlas is the semantic index of the planner:

- humans can see what kinds of cuts are intentional
- agents can discover the closest existing case
- regressions become concrete instead of narrative
