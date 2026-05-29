# stark-parts

This is a scaffold for a static Stark parts catalog search site. The committed catalog schema and fixture-backed crawler
core exist. The real HTTP client, catalog commands, committed catalog data, search model, and usable web UI are not
implemented yet.

Current useful commands:

```sh
cargo test
cargo clippy
cargo fmt --all -- --check
dprint check
```

`dprint` is required for local formatting checks. Install it with the upstream `dprint` installer or run the CI job if
you only need remote verification.

The `stark-parts catalog init` and `stark-parts catalog update` command shapes exist, but they intentionally print
not-implemented messages and exit non-zero until the real HTTP client and catalog command wiring land. From a fresh
checkout, run the scaffolded binary through Cargo:

```sh
cargo run -p stark-parts-cli -- catalog init
cargo run -p stark-parts-cli -- catalog update
```
