# Stark Parts Behavior Spec

NOTE: This is a user-visible behavior spec. It describes what the site must do for a person trying to find parts, not
how the catalog is crawled or how the web app is built. Implementation choices belong in `IMPL_SPEC.md`, and the
implementation must conform to `IMPL_SPEC.md`.

## Problem

Stark's official parts site is hard to navigate when you already have a part, SKU, assembly name, or rough subsystem in
mind. This project provides a small static website for searching the Stark parts catalog without needing to click
through Stark's own catalog UI.

The site is not an official Stark site and must not imply that it is. It is a locally maintained catalog view generated
from public catalog data.

## Core Experience

At the top of the page, the site must show a persistent, visually distinct banner explaining that the site is
unofficial, is not endorsed by Stark, may contain errors, and that Stark's own website is the authoritative source. The
banner must be visible on initial page load without hiding the search controls.

The first screen must be the search experience. A user should be able to load the page, optionally choose bike variants,
type a search query, and immediately see the matching catalog tree.

The page title must stay at the top left of the header. The search field is the primary control and must appear directly
under that title, before catalog metadata. When the page loads, the search field must receive keyboard focus
automatically.

The site must be static at runtime. Loading and searching the catalog must not require a server, database, remote API
call, or any remote RPC. The catalog data used by the browser is committed in this repository and served as static site
content.

The supported catalog should include every Stark bike variant for which Stark exposes public parts catalog data.

The catalog tree is rooted by bike variant. Bike filtering is multi-select. If no bike variant is selected, that is
equivalent to selecting all committed bike variants. If one or more variants are selected, the tree shows only those
variants.

Within each bike root, the tree must reflect the parts catalog hierarchy:

- bike variant
- catalog category
- catalog subcategory, when present
- product group
- article or part
- variant/SKU, when present

The tree may omit empty hierarchy levels when the source data does not provide them, but it must preserve the catalog
structure that exists in the committed data.

## Search Behavior

Search must update live as the user types. Each character typed into the search field must update the visible tree
without requiring a submit button and without making any remote request.

The entire app and committed catalog must be loaded during initial page load or app initialization. Search and tree
updates must be browser-local after that point.

An empty search query shows the full tree for the selected bike variants.

The current search state must be reflected in the page URL so searches can be bookmarked or shared. Loading a URL with
search state must restore the query and selected bike variants.

Search must match at least these fields when they exist in the committed catalog data:

- bike variant code and display name
- category code and display name
- product group code, display name, and description
- article code, display name, and description
- variant code
- SKU
- attribute code, option code, and option display name
- kit contents

Search is case-insensitive. Punctuation in user input should not make common SKU searches fragile; for example,
searching with or without hyphens should still find the same SKU when the normalized text is otherwise the same.

When a node matches, the tree must include enough ancestors for the result to make sense. For example, a matching SKU
must still be shown under its bike, category path, product group, and article. A category match may show the category
and its matching descendants; whether it expands every descendant by default is an open UI decision.

When no result matches the query and bike filters, the page must show a clear empty state. It should not leave the user
wondering whether the catalog failed to load.

## Result Details

Each visible part-level result should expose the data a user needs to identify the part before going back to Stark or a
dealer:

- display name, when available
- code, when available
- SKU, when available
- bike variant compatibility
- category path
- variant attributes, when available
- kit membership or kit contents, when available
- availability, when available in the committed data, with a stale-data warning
- price and currency, when available in the committed data, with a stale-data warning
- source image URL or rendered image, when available
- an immediately visible one-click link to the most specific canonical HTTPS Stark-owned page that can be determined for
  the part. If the committed data does not contain an article- or SKU-specific URL, the link should point to the Stark
  product-group page for that bike, category path, and product code. It must not fall back to the bike-level spare-parts
  overview for a part-level result.

For broad searches, the page may cap the number of rendered detail cards as long as it keeps the full match count and
catalog tree visible, explains that only the first details are being shown, and tells the user to narrow the search to
inspect the rest. This cap is a rendering constraint, not a search constraint.

The site should prefer human-readable display strings over localization keys. If a display string is missing, it must
fall back to stable codes or localization keys rather than hiding the result.

The catalog is US-storefront data for now. That storefront choice affects source pricing, currency, availability, and
other storefront-specific metadata. The site must not hide otherwise available catalog parts solely because source
metadata labels them as region-specific or non-US-specific.

## Static Runtime Constraints

The browser must not call Stark's catalog APIs at runtime.

The browser must not depend on a project-owned backend service at runtime.

The browser must not lazily fetch additional catalog data in response to search input, tree expansion, bike filter
changes, or result selection. Those interactions must use the catalog data already loaded during initial page load or
app initialization.

The browser may load ordinary static assets that are part of the site build.

Remote images may be lazy-loaded from HTTPS image hosts explicitly allowed by the implementation spec. Image loading
must not block the app's initial render, search behavior, tree updates, or responsiveness to the next key press. A slow
or failed image load must not prevent a user from searching, expanding results, or using the Stark link for a part.

## Catalog Freshness

The site shows the catalog state committed in the repository. It does not promise to reflect Stark's live catalog at
page-load time.

Catalog updates happen offline through the command line tool described in `IMPL_SPEC.md`. After an update, the generated
JSON5 catalog file is committed like any other source file.

The page should expose the catalog generation timestamp and enough source metadata for a user to understand how stale
the data may be.

## Data Format Contract

The committed catalog state must be JSON5.

The JSON5 must be deterministically formatted. Re-running the catalog update command against unchanged source data must
produce the same file bytes.

The JSON5 must be text-reviewable in Git. Diffs should be useful when Stark adds, removes, renames, or reprices catalog
entries.

## Non-Goals

The site does not sell parts.

The site does not place orders.

The site does not authenticate with Stark.

The site does not provide live inventory guarantees.

The site does not replace Stark's official documentation, compatibility notes, or purchase flow.
