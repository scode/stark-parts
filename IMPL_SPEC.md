# Stark Parts Implementation Spec

NOTE: This is not the user-visible behavior contract. `SPEC.md` describes what the user should experience. This file
records implementation choices, implementation constraints, and internal contracts that should guide the first build.

## Repository Shape

The project has two major pieces:

- a Rust command line tool that crawls and refreshes the committed catalog data
- a Rust/Leptos web app that renders and searches the committed catalog data as a static site

The web app must be able to run from static build output. It should not require a server process once built.

## Catalog Commands

The command line tool is named `stark-parts`.

It must provide these subcommands:

```text
stark-parts catalog init
stark-parts catalog update
```

`stark-parts catalog init` creates the initial committed catalog JSON5 file. It must be run from the repository root so
paths and generated metadata are predictable.

`stark-parts catalog update` refreshes an existing committed catalog JSON5 file. It must also be run from the repository
root.

Both commands require network access. Runtime use of the website does not.

Both commands must emit fully formatted JSON5 using the same deterministic formatter. The formatter is part of the
contract: no command should write ad hoc JSON5 that happens to parse but produces noisy diffs.

## Rust Tooling

The catalog tooling is implemented in Rust.

Command line parsing uses the `clap` crate, with derive-based parsing unless there is a concrete reason not to. Use the
current stable `clap` release when the project is scaffolded rather than pinning an old version from memory.

Logging uses `tracing`. The CLI should emit useful crawl progress, source URLs, item counts, and write decisions without
dumping full catalog payloads by default.

The crawler should model Stark API responses with typed Rust structs at the boundary. The committed catalog format can
be a separate internal schema so the public data file is stable even if Stark's API shape shifts.

Network access to Stark's upstream catalog must be isolated behind a small trait. The crawler should depend on that
trait rather than on `reqwest` or another concrete HTTP client directly. Tests should be able to provide canned
category, product-list, product-detail, and localization responses without making network calls or mutating process-wide
state.

## Web App Tooling

The web app is implemented in Rust using Leptos.

The app should be built as a static client-side site. It loads the entire committed catalog data set as a static asset
or compiled-in data during initial page load or app initialization, then performs filtering and tree updates in the
browser.

The search index should be derived from the committed catalog state, not from a live Stark endpoint. The implementation
may precompute normalized search text during the offline catalog update if that keeps browser code simpler and makes
search behavior deterministic.

The app should encode search state in the URL, including the query and selected bike variants. Use stable bike
identifiers in the URL, not display names.

## Source Data

The current crawl research lives in `lore/2026-05-26-stark-parts-crawl.md`.

The first known Stark API base from that report is:

```text
https://api.starkfuture.com/v2
```

Known useful endpoints from the report:

```text
GET store/categories
GET store/categories/{code}
GET store/products
GET store/products/{code}
GET store/articles/suggestions
```

The crawler should use the category and product endpoints for full ingestion. The suggestions endpoint is useful for
comparison, but it is query-driven and should not be the main crawl source.

The first catalog should use the US storefront for storefront-specific data such as prices, currency, and availability.
That choice should not become a compatibility filter; preserve parts that Stark exposes in the crawled catalog even when
source metadata suggests regional specificity.

For `varg-ex`, the report identifies this traversal:

```text
GET /store/categories?product_tag=varg-ex&path=SP
GET /store/categories?product_tag=varg-ex&path={parent_path}/{category_code}
GET /store/products?category={leaf_category_code}&tags=varg-ex
GET /store/products/{product_code}?tags=varg-ex&country=US
```

Other Stark bike variant tags must be discovered from Stark's public catalog surface before the full catalog refresh.
The implementation should treat `varg-ex` as the known starting point from the research report, not as the complete
variant set.

The crawler must carry its own traversal path. The report notes that Stark's `path` field is a parent path and is not
reliable enough by itself to reconstruct hierarchy after the fact.

## Committed Catalog Schema

The committed JSON5 schema should be project-owned, not a raw dump of Stark API responses.

The schema should preserve enough source data to support search, display, and future recrawls:

- catalog generation metadata
- source endpoint metadata
- country and language assumptions
- supported bike variants
- bike filter identifiers suitable for URL state
- category tree per bike variant
- product groups
- articles or parts
- variants and SKUs
- attributes and options
- prices and availability, if included
- image URLs, if included
- Stark website URLs, or enough stable source fields to deterministically build them
- localization keys and resolved display strings, when available

The schema should keep stable identifiers close to each node. Display strings can change; codes and SKUs are more useful
for deterministic diffing and exact search.

Stark website links must be built from trusted catalog identifiers when possible. If the catalog stores a URL, it must
be a canonical HTTPS URL with no username, password, or fragment, and its host must be in this allowlist:

```text
starkfuture.com
www.starkfuture.com
```

Reject `javascript:`, `data:`, non-HTTPS, credentialed, and non-allowlisted URLs before they can be rendered as anchors.

## Deterministic Formatting

Catalog output must be deterministic at the byte level when source data is unchanged.

Practical requirements:

- sort object keys according to the schema, not hash-map iteration order
- sort arrays only where order is not semantically meaningful
- preserve catalog tree order where Stark's ordering appears user-visible
- use stable indentation
- use stable escaping
- include generated timestamps only when the crawl actually writes changed catalog data, or place volatile timestamps
  where they do not cause unrelated churn

If the chosen JSON5 serializer cannot guarantee this directly, the project should add a formatting layer rather than
accepting noisy generated output.

## Localization

Stark API responses return localization keys for many names and descriptions.

The first implementation should attempt to extract English display strings from Stark's Next.js page payload. Parse that
payload as data only; do not execute upstream JavaScript or use `eval`-style script evaluation. If extraction fails or a
key is missing, keep the raw code or localization-key fallback and record enough metadata to debug the failure.

Resolved display strings and descriptions are external data. Render them as text, or sanitize them before any HTML
insertion. The UI should not use raw HTML insertion for catalog names, descriptions, localization strings, codes, SKUs,
attributes, or kit contents.

If localization extraction is implemented, the catalog file should keep both the resolved display string and the
original localization key. The key is useful for debugging upstream changes.

## Search Index

Search should be simple and deterministic before it is clever.

A practical first index is one row per article variant, with SKUs kept as an array. That matches the crawl report's
recommendation and keeps exact SKU lookup straightforward without losing the relationship between variant attributes and
part data.

Each index row should include denormalized ancestor text:

- bike variant
- category path
- product group
- article
- variant
- SKUs
- attributes
- kit data

The renderer can then rebuild the visible tree from matching rows and their ancestors.

Remote images may be loaded by the browser as images, but they must stay out of the catalog data path. Only keep HTTPS
image URLs whose canonical host is in an explicit allowlist. The initial allowlist is:

```text
s3-stark-prod.s3.eu-central-1.amazonaws.com
s3-stark-production.s3.eu-west-1.amazonaws.com
```

Adding another image host requires updating this implementation spec. Configure the static app so image requests do not
leak more referrer information than needed. Do not gate initial render, search indexing, tree updates, or input handling
on image fetches or image decode completion.

## Network Behavior

Only the catalog commands should call Stark's APIs.

The web app must not call:

```text
https://api.starkfuture.com/v2/store/*
```

That constraint should be easy to test. The app code should not contain Stark API client logic, and browser tests should
be able to fail if runtime fetches are made to Stark catalog endpoints.

The web app also should not perform query-time catalog fetches from any other endpoint. Tests should be able to assert
that search input, tree expansion, bike filter changes, and result selection do not trigger network requests for more
catalog data.

## Validation

The first implementation should include tests for the parts that are easiest to get subtly wrong:

- deterministic JSON5 output
- repository-root enforcement for `catalog init` and `catalog update`
- category traversal path handling
- parsing representative Stark category, product, article, variant, price, and availability payloads
- crawler behavior through a mocked upstream catalog client
- search normalization, especially case, punctuation, and SKU hyphen behavior
- tree pruning from search results

Tests must not mutate process-wide environment variables. Prefer dependency injection for paths, network clients,
clocks, and logging sinks.

## Open Implementation Decisions

These should be decided before the first substantial implementation pass that needs them:

- Whether the catalog schema lives in its own workspace crate or stays inside one of the existing crates until more
  sharing pressure appears.
- Exact path of the committed JSON5 catalog file.
- Whether catalog data is loaded by the web app through a static fetch, embedded at compile time, or generated into Rust
  data during the build.
- Whether `catalog update` should preserve manually disabled variants or always regenerate the complete file from
  source.
