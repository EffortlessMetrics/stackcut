default:
    @just --list

ci-fast:
    cargo run -p xtask -- ci-fast

ci-full:
    cargo run -p xtask -- ci-full

smoke:
    cargo run -p xtask -- smoke

golden:
    cargo run -p xtask -- golden

mutants:
    cargo run -p xtask -- mutants

docs-check:
    cargo run -p xtask -- docs-check

release-check:
    cargo run -p xtask -- release-check
