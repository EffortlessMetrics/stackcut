# Build notes

This bundle was assembled and sanity-checked in an environment without a Rust toolchain.

Checks completed:

- all `*.toml` files parse
- all `*.json` files parse
- fixture plans and schemas are present
- no `unwrap`/`expect` calls remain in the starter code

Checks not completed here:

- `cargo fmt`
- `cargo clippy`
- `cargo test`
- end-to-end Git-backed plan/materialize/validate runs

Run these first after download:

```bash
cargo run -p xtask -- ci-fast
cargo run -p xtask -- ci-full
```
