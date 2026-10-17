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

`stark-parts catalog update` must refresh the catalog `generated_at` metadata after a successful crawl, even when Stark
returns the same parts data as the existing committed catalog. That timestamp feeds the user-visible freshness date.

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

The app should be built as a static client-side site. The committed catalog is deployed as a separate static asset and
loaded once during app initialization. The app must validate the complete response before constructing the search index,
then perform filtering and result-list updates in the browser without further catalog requests. The catalog must not be
compiled into the application binary; keeping code and catalog artifacts separate allows catalog-only deployments to
reuse an unchanged application build.

Vercel deployments should use the checked-in Vercel project configuration at the repository root. The project is a
static "Other" framework deployment: Vercel obtains the application output through the cached build task described
below, assembles the current catalog into it, and serves only the resulting `dist/` directory. That deployment path must
not introduce Vercel Functions, runtime environment variables, or a backend dependency for the web app.

The Trunk application build should run as a Turborepo task backed by Vercel Remote Cache. The task hash must exclude the
generated catalog and its build receipt, and the cached outputs must exclude the deployed catalog asset. A separate,
uncached assembly step copies the current committed catalog into `dist/` after either a cache hit or a real Trunk build.
Cache misses must fall back to the ordinary release build without requiring manual intervention or external artifact
storage. Vercel's install phase should install only npm dependencies; Rust and Trunk setup belongs inside the cached
task so a cache hit does not pay the application toolchain setup cost. Remote Cache artifacts currently expire seven
days after upload, so an expired artifact must behave as an ordinary cache miss and rebuild the application.

Vercel Web Analytics may be loaded from the static HTML entrypoint with Vercel's hosted analytics script. Basic
page-view analytics should not add React, Next.js, or npm analytics package integration to the Leptos app.

Continuous integration should classify changes before starting compilation. A change limited to the generated catalog
and its build receipt validates the catalog crate and generated-file formatting without compiling or testing the web
application. Any change outside those two generated files, including a change that also updates the catalog, runs the
complete formatting, lint, test, static-build, and browser-test suite.

The search index should be derived from the committed catalog state, not from a live Stark endpoint. The implementation
may precompute normalized search text during the offline catalog update if that keeps browser code simpler and makes
search behavior deterministic.

The app should encode search state in the URL, including the query and selected bike variants. Use stable bike
identifiers in the URL, not display names.

## Source Data

The crawler uses the API consumed by Stark's current storefront:

```text
https://api.starkfuture.com/v3/product-catalog
```

These endpoints were observed to accept unauthenticated GET requests. This is not a claim that Stark has published a
third-party API contract or guaranteed their stability.

```text
GET categories/tree?language=en
GET categories/by-slug/parts/products?country=US&language=en&page={page}&limit=100
GET products/{slug}?country=US&language=en
GET products/{slug}/pack-composition?country=US&language=en&variantValueId={value}
```

The unfiltered Parts listing defines snapshot membership and includes mounting parts. Do not request only
`spareParts=false`, which excludes mounting hardware. Follow every page and check page numbers, totals, unique product
IDs, and the final item count; reject an inconsistent or incomplete crawl before writing the catalog. An API-level
failure inside an HTTP 200 response is still a failed crawl.

The first catalog should use the US storefront for storefront-specific data such as prices, currency, and availability.
That choice should not become a compatibility filter; preserve parts that Stark exposes in the crawled catalog even when
source metadata suggests regional specificity.

Discover models from category-tree `bikeSlug` values and structured product fitments. Preserve the existing four bike
filter IDs when mapping the source slugs so bookmarked searches still work. Discover categories from the shared Parts
tree, retaining all paths and bounding tree depth. Do not scrape HTML cards or execute upstream JavaScript.

Fetch each listed product's details once. Keep structured fitment conditions separately from prose; never broaden them
from a description. Prefer SKU-specific stock selections over product totals, and do not substitute product-level
availability for an individual SKU. Prices are integer minor-unit strings with an explicit currency. Build canonical
product links from safe slugs and URL-encoded variant IDs. Fetch kit composition for each concrete kit SKU's selected
value and store it on that variant, retaining quantities and preassigned component options except the internal `price`
attribute. Preserve variant descriptions and localization keys when supplied; variant wording overrides the product's
default description for search and display. If a product description exactly matches one of its variant descriptions,
treat it as the storefront's selected variant default and omit it from article-level fallback. A sibling SKU without
translated wording must not inherit another variant's description.

## Committed Catalog Schema

The committed JSON5 schema should be project-owned, not a raw dump of Stark API responses.

The schema should preserve enough source data to support search, display, and future recrawls:

- catalog generation metadata
- source endpoint metadata
- country and language assumptions
- supported bike variants
- bike filter identifiers suitable for URL state
- shared category tree and every product's category paths
- product groups
- articles or parts
- variants and SKUs
- structured model fitments and configuration restrictions
- attributes and options
- prices and availability, if included
- image URLs, if included
- Stark website URLs, or enough stable source fields to deterministically build them
- localization keys and resolved display strings, when available

The schema should keep stable identifiers close to each node. Display strings can change; codes and SKUs are more useful
for deterministic diffing and exact search.

Schema version 2 stores products once in the top-level `products` collection, independently of model and category
membership. The parser continues to accept version 1 `catalog_trees` snapshots so the application migration can be
reviewed and deployed before the generated-data refresh. Empty additive fields are omitted by the formatter to preserve
legacy snapshot bytes.

Variant `kit_contents` is an optional override: absence inherits article-level contents for legacy snapshots, while an
explicit empty array means that SKU has no components. Preserve that distinction in parsing and formatting; combining
the component lists across a product's colours would contaminate each SKU's search text and detail card.

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
- keep deterministic-formatting tests focused on formatter and schema behavior; do not depend on repeated live Stark
  crawls to prove byte stability

If the chosen JSON5 serializer cannot guarantee this directly, the project should add a formatting layer rather than
accepting noisy generated output.

## Localization

Stark API responses return localization keys for many names and descriptions.

Request English strings through the API's `language=en` parameter. Preserve both resolved strings and localization keys.
If a resolved string is absent, keep the raw code or localization-key fallback. Localization must not depend on Next.js
page payloads or script execution.

Resolved display strings and descriptions are external data. Render them as text, or sanitize them before any HTML
insertion. The UI should not use raw HTML insertion for catalog names, descriptions, localization strings, codes, SKUs,
attributes, or kit contents.

If localization extraction is implemented, the catalog file should keep both the resolved display string and the
original localization key. The key is useful for debugging upstream changes.

## Search Index

Search should be simple and deterministic before it is clever.

The index expands each concrete SKU into its model and category contexts, then merges matching contexts into one visible
result. Shared product identity must survive overlapping categories. Products with an empty fitment list have an
unassigned model context; they are visible in unfiltered search and excluded by an explicit bike selection.

Each index row should include denormalized ancestor text:

- bike variant
- category path
- product group
- article
- variant
- SKUs
- attributes
- kit data

The search index should keep text buckets separate by source. Exact article or variant wording should remain distinct
from article descriptions, inherited product-group wording, and broader context such as bike variant and category path.
Non-empty searches should rank those buckets in that order while preserving catalog order inside each rank. Empty
searches should remain catalog ordered.

A complete normalized SKU match has its own highest rank. UUID variant identifiers and short SKU fragments can otherwise
match many source fields and push the requested SKU below the visible window.

The renderer should show matching rows as a flat virtualized result list. Repeated occurrences of the same article or
variant across bike catalog trees should merge into one visible result with bike compatibility attached. Ancestor fields
remain denormalized into each row so the detail card can show bike, category, product group, article, and SKU context
without rebuilding a visible catalog tree.

Remote images may be loaded by the browser as images, but they must stay out of the catalog data path. Only keep HTTPS
image URLs whose canonical host is in an explicit allowlist. The initial allowlist is:

```text
assets.starkfuture.com
s3-stark-prod.s3.eu-central-1.amazonaws.com
s3-stark-production.s3.eu-west-1.amazonaws.com
```

Adding another image host requires updating this implementation spec. Configure the static app so image requests do not
leak more referrer information than needed. Do not gate initial render, search indexing, result-list updates, or input
handling on image fetches or image decode completion.

## Network Behavior

Only the catalog commands should call Stark's APIs.

The web app must not call:

```text
https://api.starkfuture.com/*
```

That constraint should be easy to test. The app code should not contain Stark API client logic, and browser tests should
be able to fail if runtime fetches are made to Stark catalog endpoints.

The web app also should not perform query-time catalog fetches from any other endpoint. Tests should be able to assert
that search input, result hover, bike filter changes, and result selection do not trigger network requests for more
catalog data.

## Validation

The first implementation should include tests for the parts that are easiest to get subtly wrong:

- deterministic JSON5 output
- repository-root enforcement for `catalog init` and `catalog update`
- category traversal path handling
- parsing representative Stark category, product, article, variant, price, and availability payloads
- crawler behavior through a mocked upstream catalog client
- search normalization, especially case, punctuation, and SKU hyphen behavior
- flat result-list filtering from search rows

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
