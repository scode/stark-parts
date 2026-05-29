# Implementation Plan

NOTE: This file is both the implementation plan and the status tracker. Keep the steps in order. When a step has passed
its required gates and is ready to become its PR, change its marker from `[ ]` to `[x]` in that same reviewable change.

Each step is its own reviewable `$jjstack` PR. Build the stack bottom-up: complete the step, add or update good test
coverage for the behavior or implementation surface introduced by that step, run those tests, run `$pre-pr-review-swarm`
against the finished diff, address all feedback that is unambiguously an improvement, rerun the checks affected by those
fixes, rerun the swarm if the fixes materially change the diff, mark the step `[x]`, then commit that step and create
the PR. The next step builds on top of the previous step's bookmark.

Creating or updating a PR is not a stopping point for the overall plan. Agents should keep working through later plan
steps and return with the full set of created PRs unless blocked by a real dependency or by the user's current
instruction.

Merging is different. "Ready for review", "ready to merge", passing checks, approved reviews, a completed plan step, or
the need to build later PRs on top of earlier PRs are not permission to merge. Do not merge a PR, close a PR as merged,
enable auto-merge, or ask another tool or service to merge a PR unless the user's current instruction explicitly says to
do that.

If a decision is unclear but forward progress is still reasonable, record it in `AMBIGUITY.md` and continue. Only stop
for help when the decision is existential enough that building either direction would likely waste the stack.

## Stack

- 1. [x] Project scaffold and CI baseline.

  Create the Rust workspace, basic crate layout, formatting/lint/test commands, and CI needed for later PRs to have
  useful gates. Include a minimal CLI binary and a minimal Leptos app shell only far enough to prove the workspace
  builds. Do not implement catalog behavior yet.

  This step must apply the Rust-relevant `$scode-modernize` baseline while scaffolding the project:

  - add separate GitHub Actions jobs for `cargo fmt --all -- --check`, `cargo clippy`, and `cargo test`
  - add cargo caching to every CI job that compiles Rust code
  - avoid deprecated `actions-rs/*` actions
  - add `dprint` formatting for Markdown, TOML, and JSON, including a CI check
  - keep agent finish-work commands in sync with required CI commands
  - keep `AGENTS.md` canonical and `CLAUDE.md` as a symlink to it
  - add Conventional Commit guidance to the agent instructions
  - use `tracing`/`tracing-subscriber` instead of `log`-ecosystem logging when logging is introduced

  Expected coverage: workspace build tests or smoke tests, CI command coverage, dprint coverage, and enough checks to
  catch broken formatting, clippy failures, and test failures.

  Coverage gate before swarm: add or update tests or checks that prove the scaffold, build checks, dprint check, and CI
  gates catch the failures this step is meant to catch.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 2. [x] Catalog schema and deterministic JSON5 formatting.

  Define the project-owned committed catalog schema, including metadata, bike variants, category tree nodes, products,
  articles, variants, SKUs, attributes, prices, availability, image URLs, Stark website links or link-building fields,
  localization keys, display strings, and search-supporting identifiers. Implement deterministic JSON5 serialization and
  parsing for that schema.

  Expected coverage: round-trip parsing, deterministic byte-for-byte formatting, schema key ordering, stable treatment
  of timestamps, allowed image-host validation, Stark-link validation, and representative fixture coverage for prices,
  availability, image URLs, Stark links, and multiple bike variants.

  Coverage gate before swarm: add or update tests that prove schema parsing, formatting, ordering, allowed image-host
  validation, Stark-link validation, and fixture coverage for representative catalog data.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 3. [x] Upstream catalog client trait and fixture-backed crawler core.

  Add the trait boundary for Stark upstream catalog access and implement crawler traversal against that trait using
  fixture responses. The crawler core should handle variant tag discovery, category recursion, product group fetches,
  product detail fetches, storefront parameters, and path handling without making real network calls in tests.

  Expected coverage: mocked upstream responses, variant tag discovery, branch and leaf category traversal, parent-path
  handling, US storefront parameter use, no filtering of region-labeled parts, error handling, and transformation into
  the committed schema.

  Coverage gate before swarm: add or update tests that prove variant tag discovery and crawler traversal work through
  the upstream trait using fixtures and do not require live Stark network access.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 4. [x] Real Stark HTTP client and catalog commands.

  Implement the concrete network client behind the upstream trait and wire `stark-parts catalog init` and
  `stark-parts catalog update`. Both commands must run from the repository root, emit deterministic JSON5, use
  `tracing`, and keep network behavior isolated from the rest of the code.

  Expected coverage: CLI argument parsing, repository-root enforcement, init/update write behavior, deterministic output
  after unchanged fixture data, logging setup, and network-client unit tests that do not require live Stark calls.

  Coverage gate before swarm: add or update tests that prove CLI behavior, repository-root checks, deterministic writes,
  logging setup, and HTTP-client boundaries without live Stark calls.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 5. [x] Real Stark catalog build and crawler hardening.

  Run `stark-parts catalog init` or `stark-parts catalog update` against Stark's real public website, debug and fix
  crawler or schema problems discovered by the live crawl, and commit the resulting deterministic JSON5 catalog
  database. This step must happen before the browser UI is built so later search and UI work compose on top of real
  committed catalog data, not only fixtures.

  Expected coverage: live crawl receipt or equivalent captured command output, committed catalog diff review, generated
  metadata sanity checks, all-public-variant coverage checks, deterministic rerun check, and regression tests for every
  crawler/schema issue discovered while applying the catalog builder to Stark's real site.

  Coverage gate before swarm: add or update tests or scripted checks that prove the generated catalog includes every
  discovered public bike variant, is deterministic, and covers any real-site crawler failures fixed in this step.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 6. [x] Search index and tree projection.

  Build the browser-local search data model from the committed catalog. Implement normalization, matching, bike
  filtering, URL-state encoding/decoding, and tree projection from matching rows and ancestors without any UI
  dependency.

  Expected coverage: required search fields from `SPEC.md`, case-insensitive matching, punctuation and SKU hyphen
  normalization, empty query behavior, multi-select bike filters, none-selected-means-all behavior, URL restore/share
  behavior, no-result behavior, and ancestor-preserving tree pruning.

  Coverage gate before swarm: add or update tests that prove every required search field is matched, along with
  normalization, filter semantics, URL state, empty states, and ancestor-preserving tree projection.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 7. [ ] Static Leptos search UI.

  Implement the actual browser experience: unofficial warning banner, bike filter controls, search box, live-updating
  tree, result details, visible catalog generation/source metadata, stale-data warnings for price and availability,
  one-click Stark links, and lazy remote images that never block search or tree responsiveness.

  Expected coverage: component tests or browser tests for initial load, visible catalog freshness/source metadata,
  URL-state restoration, search-as-you-type behavior, no query-time catalog fetches, result details, display-string
  fallback, text rendering or sanitization for external catalog strings, stale warnings, safe Stark links, image failure
  behavior, and responsive behavior while image loads are pending.

  Coverage gate before swarm: add or update tests that prove the user-facing search UI, warning banner, details,
  display-string fallback, safe external-string rendering, safe Stark links, lazy images, and no-query-time-fetch
  behavior.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.

- 8. [ ] Static build verification with committed catalog.

  Verify the static app build consumes the committed JSON5 catalog and still performs search locally after initial load.
  This step should not be the first real catalog build; it is the end-to-end verification pass after the catalog, search
  model, and UI have all landed in the stack.

  Expected coverage: static build, browser smoke test against the built app, URL-state restore, browser-local search
  over the committed catalog, visible catalog freshness/source metadata, lazy image behavior, safe Stark links, and
  checks that runtime Stark API calls are absent.

  Coverage gate before swarm: add or update tests or scripted checks that prove the static app consumes the committed
  catalog and does not trigger runtime Stark API calls.

  Final gate before PR: checks, `$pre-pr-review-swarm`, unambiguous fixes, `[x]`, stacked PR.
