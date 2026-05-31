# stark-parts

Unofficial static search for the committed Stark parts catalog.

## Quickstart

```sh
rustup toolchain install
cargo install trunk --version 0.21.14 --locked
npm run dev
```

Then open:

```text
http://127.0.0.1:1420
```

## Architecture and Design

This is a static Rust/Leptos web app backed by a committed JSON5 catalog. Runtime search does not call Stark's APIs, and
there is no project-owned backend service. The browser loads the app and catalog snapshot, builds a local search index,
and updates the result list/details view as the user types or changes bike filters.

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
category path, product group, article, SKU, attributes, and kit data. Matching rows are shown as a flat list whose
primary text is the human-readable article or part name, with SKU shown as the secondary scanning text when it exists.
Repeated occurrences of the same article or variant across bike catalog trees are merged into one visible result with
bike compatibility kept on the detail card.

The UI exposes the catalog generation/source metadata, a persistent unofficial-site warning, multi-select bike filters,
URL-restorable search state, hover detail cards for result rows, stale price/availability warnings, lazy remote images,
and Stark links when a safe URL can be derived. Catalog strings are rendered as text, not raw HTML.

## Vercel

This repo is set up for Vercel Git deployments as a static site. Import the repository in Vercel, use the repo root as
the project root, and select the "Other" framework preset. The build, dev, and output settings are checked in through
`vercel.json`.

The Vercel build installs Rust with `rustup`, uses the pinned toolchain in `rust-toolchain.toml`, installs the pinned
Trunk version used by CI, runs `npm run build`, and serves the generated `dist/` directory. No Vercel environment
variables are required for the static site.

Useful local checks:

```sh
cargo fmt --all -- --check
cargo clippy
cargo test
dprint check
npm ci
npm run build
vercel build
npx playwright install --with-deps chromium
npx playwright test
```
