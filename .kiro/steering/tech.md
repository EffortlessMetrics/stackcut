# Tech Stack

## Language & Toolchain

- Rust (edition 2024, MSRV 1.92)
- Toolchain: stable channel via `rust-toolchain.toml`
- Required components: clippy, rustfmt

## Workspace

Cargo workspace with resolver v2. Workspace version: `0.1.0`.

## Key Dependencies

| Crate | Purpose |
|---|---|
| `serde` / `serde_json` / `toml` | Serialization for config, plan, overrides |
| `schemars` | JSON Schema generation for plan and override formats |
| `clap` (derive) | CLI argument parsing |
| `anyhow` | Error handling in git, artifact, and CLI crates |
| `tempfile` | Temporary directories for recomposition validation |

## Build & Test Commands

All repo rituals go through `xtask`. Use these commands:

```bash
# Fast local loop (fmt + clippy + core tests)
cargo run -p xtask -- ci-fast

# Full CI bar (fmt + clippy + all workspace tests)
cargo run -p xtask -- ci-full

# Run all workspace tests
cargo run -p xtask -- smoke

# Run artifact tests (golden/snapshot)
cargo run -p xtask -- golden

# Mutation testing
cargo run -p xtask -- mutants

# Doc tests
cargo run -p xtask -- docs-check

# Pre-release checks
cargo run -p xtask -- release-check
```

A `justfile` wraps these same commands for convenience.

## CLI Usage

```bash
stackcut plan --base <rev> --head <rev>
stackcut explain .stackcut/plan.json
stackcut validate .stackcut/plan.json --exact
stackcut materialize .stackcut/plan.json --out .stackcut/patches
```

## Schemas

- `schema/stackcut.plan.schema.json` — plan artifact format
- `schema/stackcut.override.schema.json` — override file format

These are contracts. Changes require versioning.
