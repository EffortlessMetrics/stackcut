# RELEASE

## Release checklist

1. run `cargo run -p xtask -- release-check`
2. verify `schema/stackcut.plan.schema.json`
3. verify `schema/stackcut.override.schema.json`
4. materialize patches from a real repo range
5. run exact recomposition validation
6. review `docs/ROADMAP.md` for any newly opened gaps
7. tag and publish only after artifacts are stable

## Versioning

- bump the workspace version for user-visible CLI changes
- bump schema versions when artifact structure changes incompatibly
- avoid silent changes to plan semantics

## Public artifact contract

These are the user-facing contracts for v0.1:

- `plan.json`
- `summary.md`
- `diagnostics.json`
- patch files emitted by `materialize`

Those should remain stable unless a versioned change is intentional.
