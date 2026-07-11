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
The static deployment serves that committed file alongside the app, which loads it once during startup. Search remains
local to the browser after initialization.

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

`npm run build` separates the application build from catalog assembly:

1. Turborepo runs or restores the `build:app` task. Its inputs exclude the generated catalog and build receipt, and its
   cached outputs exclude the deployed catalog file.
2. `npm run assemble:catalog` always copies the current committed catalog into `dist/`, whether the application came
   from a cache hit or a real Trunk build.
3. Vercel serves the completed `dist/` directory as a static deployment.

Vercel Remote Cache is enabled automatically for the Turborepo task; this project does not need cache credentials or
runtime environment variables. A catalog-only deployment should restore the unchanged application and skip Rust, Trunk,
and WASM compilation. On a cache miss, `scripts/build-app.sh` installs the pinned Rust toolchain and Trunk version
before running the normal release build. Cache availability is only an optimization: a miss must make the deployment
slower, not change its output or cause it to fail.

Vercel currently expires Remote Cache artifacts seven days after upload. Expect an occasional full build even when only
the catalog has changed. Application changes, build-input changes, deliberate cache clearing, and expired artifacts also
produce normal cache misses.

### Build troubleshooting

The Turborepo summary is the first place to look:

- `cache hit` means the application task was restored. Turborepo replays the cached task logs, including their original
  timestamps; those lines do not mean the compiler ran again.
- `cache miss` means `scripts/build-app.sh` should install any missing tools and run Trunk.
- A successful application task followed by a bad or stale catalog points to `scripts/assemble-catalog.mjs`, not the
  cache. `npm run test:build-scripts` verifies that assembly replaces stale output with the committed bytes.

Local Turborepo artifacts live in `.turbo/`. Removing that ignored directory forces a local cold build without affecting
Vercel's Remote Cache.

## Continuous integration

GitHub Actions classifies changed paths before starting the main jobs. A change limited to `catalog/stark-parts.json5`
and `catalog/BUILD_RECEIPT.md` runs catalog validation and generated-file formatting without compiling the web
application. Any other changed path—including a catalog update mixed with code or configuration—runs the complete
formatting, lint, test, static-build, and browser-test suite.

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
