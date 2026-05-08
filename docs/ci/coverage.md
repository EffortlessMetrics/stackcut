# Coverage

Codecov coverage is Rust execution-surface evidence for the `stackcut` repository.

## What Coverage answers

> Did tests execute this Rust surface?

## What Coverage does not answer

- Whether a proposed stack is correct
- Whether generated patches exactly recompose to the original head
- Whether patch materialization is correct
- Whether slice ordering is correct
- Whether overrides are applied correctly
- Whether diagnostics are complete
- Whether mutation adequacy is strong
- Whether fuzzing is sufficient
- Whether release readiness is proven

Those are separate proof lanes.

## Coverage workflow

The Coverage workflow runs on:
- Push to `main`
- `workflow_dispatch` (manual trigger)
- PRs labeled `coverage`, `full-ci`, or `ci:full`

## Codecov configuration

Codecov comments and annotations are disabled. Durable receipts are:
- `coverage.json` (structured coverage data)
- `coverage.txt` (human-readable summary)
- `lcov.info` (standard LCOV format)
- GitHub Actions coverage artifact (14-day retention)
- Codecov dashboard (persistent)
- `target/coverage/coverage-receipt.json` (claim boundary metadata)
