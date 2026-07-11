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
type a search query, and immediately see the matching result list.

The page title must stay at the top left of the header. The search field is the primary control and must appear directly
under that title, before catalog metadata. When the page loads, the search field must receive keyboard focus
automatically.

The browser tab must use a small Stark Parts favicon so the site is identifiable in tabs and bookmarks.

Catalog metadata must stay visible in the header but should be visually secondary to the title, search field, and bike
filters. It must include the catalog date, storefront source, feedback/contact address, and source-code link.

The bike filters must appear as a horizontal list of chip-style bike-name toggles immediately under the search field.
They must not take horizontal space away from the result list. When no bike is selected, the filters must show
`default: all bikes` to make the all-bikes default explicit. That text must disappear as soon as one or more bikes are
selected.

The site must be static at runtime. Loading and searching the catalog must not require a server, database, remote API
call, or any remote RPC. The catalog data used by the browser is committed in this repository and served as static site
content.

The supported catalog should include every Stark bike variant for which Stark exposes public parts catalog data.

Bike filtering is multi-select. If no bike variant is selected, that is equivalent to selecting all committed bike
variants. If one or more variants are selected, the result list shows only parts compatible with those variants.

The result list is flat. Each row represents one concrete searchable part result, not one catalog hierarchy node and not
one repeated bike-specific occurrence of the same result. The primary row text should be the human-readable article or
part name, and the secondary muted text should be the SKU when one exists. Internal catalog codes are available in the
detail card, but they should not be used as the primary scanning text in the result list.

The result list should offer default and compact density modes. Default mode should show thumbnails. Compact mode should
make rows shorter and hide thumbnails for faster scanning without changing the search query, bike filters, result count,
or detail-card behavior.

## Search Behavior

Search must update live as the user types. Each character typed into the search field must update the visible result
list without requiring a submit button and without making any remote request.

The entire app and committed catalog must be loaded during initial page load or app initialization. The page must show a
clear loading state until the catalog is ready. If the catalog cannot be loaded or parsed, the page must show a clear
failure state rather than an empty result list. Search and result list updates must be browser-local after
initialization.

An empty search query shows the full result list for the selected bike variants.

Non-empty search results should be ranked by match quality rather than raw catalog order. Direct part matches such as
SKU, article name, article code, variant code, and variant attributes should appear before matches found only in an
article description. Article-description matches should appear before matches found only through inherited product-group
wording, bike wording, or category context. Rows with the same match quality should keep their catalog-relative order.

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

Product group names and descriptions must be searchable for the parts inside that group. For example, a part whose own
name does not say "wiring harness" should still be eligible to match a "wiring harness" search when Stark presents it
inside a wiring-harness product group.

Search is case-insensitive. Punctuation in user input should not make common SKU searches fragile; for example,
searching with or without hyphens should still find the same SKU when the normalized text is otherwise the same.

Rows should show concise match feedback when the visible part name or SKU does not explain why the row matched. For
example, a row matched through inherited product-group wording should say which group matched.

When a row matches, its detail card must include enough context for the result to make sense, including bike
compatibility and category path when those fields exist in the committed catalog.

When no result matches the query and bike filters, the page must show a clear empty state. It should not leave the user
wondering whether the catalog failed to load.

## Result Details

Hovering a result row must show that row's detail card. Once shown, the card must stay visible when the pointer moves
elsewhere on the page. It may change only when the pointer enters another result row, or when the search results change
and the displayed row is no longer present in the current results. The row whose card is shown must stay visually
highlighted for as long as that card remains shown.

On wide screens, the card must appear to the right of the result list; on narrow screens, it may stack below the result
list so it remains usable. Each visible detail card should expose the data a user needs to identify the part before
going back to Stark or a dealer:

- display name, when available
- code, when available
- SKU, when available
- bike variant compatibility
- category path
- variant attributes, when available
- kit membership or kit contents, when available
- availability, when available in the committed data, with a stale-data warning
- price and currency, when available in the committed data, with a stale-data warning
- source image URL or rendered image at the top of the card, when available
- an immediately visible one-click link to the most specific canonical HTTPS Stark-owned page that can be determined for
  the part. If the committed data does not contain an article- or SKU-specific URL, the link should point to the Stark
  product-group page for that bike, category path, and product code. It must not fall back to the bike-level spare-parts
  overview for a part-level result.

Image frames should keep their layout stable when remote images are slow or fail to load, and failed images should show
a fallback state instead of a broken-image icon.

The detail card should present the image, title, compatibility subtitle, and Stark link before the detail fields. SKU,
price, and availability should be grouped as primary facts ahead of lower-level catalog metadata.

For broad searches, the page may render detail cards only on demand as rows are hovered. This is a rendering constraint,
not a search constraint: the full match count and result list still need to reflect the complete browser-local search
result.

The site should prefer human-readable display strings over localization keys. If a display string is missing, it must
fall back to stable codes or localization keys rather than hiding the result.

The catalog is US-storefront data for now. That storefront choice affects source pricing, currency, availability, and
other storefront-specific metadata. The site must not hide otherwise available catalog parts solely because source
metadata labels them as region-specific or non-US-specific.

## Static Runtime Constraints

The browser must not call Stark's catalog APIs at runtime.

The browser must not depend on a project-owned backend service at runtime.

The browser must not lazily fetch additional catalog data in response to search input, result hover, bike filter
changes, or result selection. Those interactions must use the catalog data already loaded during initial page load or
app initialization.

The browser may load ordinary static assets that are part of the site build.

Remote images may be lazy-loaded from HTTPS image hosts explicitly allowed by the implementation spec. Image loading
must not block the app's initial render, search behavior, result updates, or responsiveness to the next key press. A
slow or failed image load must not prevent a user from searching, hovering result rows, or using the Stark link for a
part.

## Catalog Freshness

The site shows the catalog state committed in the repository. It does not promise to reflect Stark's live catalog at
page-load time.

Catalog updates happen offline through the command line tool described in `IMPL_SPEC.md`. After an update, the generated
JSON5 catalog file is committed like any other source file.

The page should expose a "Parts data last updated" catalog date and enough source metadata for a user to understand how
stale the data may be. The date should reflect the last successful committed catalog update even when Stark returned the
same parts data as the previous update. The displayed date should omit time-of-day detail.

## Data Format Contract

The committed catalog state must be JSON5.

The JSON5 must be deterministically formatted. Given the same catalog state and metadata, formatting must produce the
same file bytes. The catalog update command intentionally refreshes freshness metadata after a successful crawl, so a
live update can change bytes even when Stark returns the same parts data.

The JSON5 must be text-reviewable in Git. Diffs should be useful when Stark adds, removes, renames, or reprices catalog
entries.

## Non-Goals

The site does not sell parts.

The site does not place orders.

The site does not authenticate with Stark.

The site does not provide live inventory guarantees.

The site does not replace Stark's official documentation, compatibility notes, or purchase flow.
