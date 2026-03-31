# MAINTAINERS

## Review posture

A good change here carries proof:

- fixture or scenario updates when semantics move
- structural validation remains clean
- exact recomposition still passes for Git-backed plans
- user-visible artifacts stay legible
- schema changes are explicit and versioned

## Merge bars

### Normal change

- `ci-fast`
- affected docs and fixtures updated
- no unexplained snapshot drift

### Behavior change

- `ci-full`
- fixture case added or updated
- ambiguity reasoning in docs or ADR if new class of cut is introduced

### Release candidate

- `release-check`
- manual dry-run against a real repo
- schema review
- patch materialization smoke run

## Banned shortcuts

- hidden nondeterminism
- silent fallback that changes slice membership
- reverse dependencies into `stackcut-core`
- embedding timestamps into plan artifacts
- machine-generated explanations without deterministic backing
