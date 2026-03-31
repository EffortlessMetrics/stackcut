# TESTING

`stackcut` uses a layered proof surface.

## Fast local loop

```bash
cargo run -p xtask -- ci-fast
```

That should cover formatting, linting, and core unit tests.

## Full loop

```bash
cargo run -p xtask -- ci-full
```

That is the repo-wide bar before merge for meaningful planner changes.

## Fixture-first planner changes

When planner behavior changes:

1. update or add a case under `fixtures/cases/`
2. update the scenario atlas if the case is new
3. update `expected.plan.json`
4. rerun the relevant tests

## What to prove

### `stackcut-core`

- no member loss
- no member duplication
- deterministic ordering
- stable dependency graph
- explicit ambiguity

### `stackcut-git`

- correct name-status ingest
- patch series written in plan order
- exact recomposition matches `head^{tree}`

### `stackcut-artifact`

- plan JSON round-trips
- summary remains legible and stable
- diagnostics serialization remains stable

### `stackcut-cli`

- help and exit codes
- plan output paths
- explain rendering
- materialize output
- validation modes

## Mutation

The intended policy is mutation-on-diff for semantic planner changes and scheduled deeper sweeps for critical planner logic.

## Snapshot discipline

Snapshot or golden changes should explain:

- what changed
- why the old artifact was wrong or incomplete
- what fixture now pins the new behavior
