# stark-parts

Unofficial static search for the committed Stark parts catalog.

## Quickstart

```sh
cargo install trunk --version 0.21.14 --locked
NO_COLOR=true trunk serve --release --port 1420
```

Then open:

```text
http://127.0.0.1:1420
```

## Architecture and Design

This is a static Rust/Leptos web app backed by a committed JSON5 catalog. Runtime search does not call Stark's APIs, and
there is no project-owned backend service. The browser loads the app and catalog snapshot, builds a local search index,
and updates the tree/details view as the user types or changes bike filters.

The important pieces are:

- `catalog/stark-parts.json5`: the committed catalog snapshot. It is generated from Stark's public catalog data and is
  meant to be reviewed in Git.
- `crates/stark-parts-catalog`: the catalog schema, deterministic JSON5 formatter/parser, validation, crawler core, and
  Stark HTTP client boundary.
- `crates/stark-parts-cli`: the `stark-parts catalog init` and `stark-parts catalog update` commands. These are the only
  parts of the project that should talk to Stark's catalog APIs.
- `crates/stark-parts-web`: the Leptos app and browser-local search model.
- `index.html` and `Trunk.toml`: the static site entrypoint used by Trunk.
- `tests/static-smoke.spec.mjs`: a Playwright smoke test that serves the built `dist/` output and drives Chromium
  against it.

Catalog updates are offline work. Run them from the repository root:

```sh
cargo run -p stark-parts-cli -- catalog update
```

That command may call Stark's public API, refresh `catalog/stark-parts.json5`, and preserve deterministic formatting.
After that, the web app consumes the committed file with `include_str!`, so search remains local to the browser.

The search model is intentionally simple. It indexes article/variant rows with denormalized ancestor text: bike variant,
category path, product group, article, SKU, attributes, and kit data. Matching rows are projected back into a catalog
tree so a SKU result still appears under the bike, category, product group, and article that make it understandable.

The UI exposes the catalog generation/source metadata, a persistent unofficial-site warning, multi-select bike filters,
URL-restorable search state, result details, stale price/availability warnings, lazy remote images, and Stark links when
a safe URL can be derived. Catalog strings are rendered as text, not raw HTML.

Useful local checks:

```sh
cargo fmt --all -- --check
cargo clippy
cargo test
dprint check
npm ci
NO_COLOR=true trunk build --release
npx playwright install --with-deps chromium
npx playwright test
```
