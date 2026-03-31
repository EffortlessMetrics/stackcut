# Project Structure

## Crate Map

```
crates/
  stackcut-core/      # Pure IR, config, planner, structural validation (no local deps)
  stackcut-git/       # Git ingest, patch materialization, exact recomposition (depends on core)
  stackcut-artifact/  # Plan JSON IO, summary rendering, diagnostics (depends on core)
  stackcut-cli/       # CLI surface, orchestrates all three crates above
xtask/                # Repo ritual runner (ci-fast, ci-full, smoke, golden, etc.)
```

## Dependency Direction (strict)

```
stackcut-core       → (no local crate)
stackcut-git        → stackcut-core
stackcut-artifact   → stackcut-core
stackcut-cli        → stackcut-core, stackcut-git, stackcut-artifact
xtask               → (no local crate)
```

Do not introduce reverse edges into `stackcut-core`. If a change requires a new dependency direction, stop and escalate.

## Other Directories

```
docs/               # Architecture docs, ADRs, scenario atlas, roadmap
schema/             # JSON schemas for plan and override formats (contracts)
fixtures/cases/     # Canonical planner test cases with input.units.json + expected.plan.json
examples/           # Example config files (override.toml)
```

## Config Files

- `stackcut.toml` — repo-level planner configuration (path families, prefixes)
- `.stackcut/override.toml` — optional human overrides (must_link, force_members, rename_slices, must_order)

## Fixture Convention

Each fixture case lives in `fixtures/cases/{name}/` with:
- `input.units.json` — input edit units
- `expected.plan.json` — expected planner output
- `README.md` — problem statement

When planner behavior changes, update or add fixtures and update `docs/SCENARIO_ATLAS.md`.

## Escalation Triggers

Stop and leave a note when:
- A change would require intra-file hunk slicing
- Exact recomposition no longer holds
- A new dependency direction is needed
- A schema change breaks the plan format
- Behavior cannot be made deterministic
