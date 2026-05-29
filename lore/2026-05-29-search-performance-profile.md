# Search Performance Profile

NOTE: This is a point-in-time profiling note from May 29, 2026. It is not a benchmark suite and it should not be read as
stable performance documentation. The goal was to identify the dominant source of sluggish typing in the current static
search UI.

## Setup

The app was built and served as the static site, not run through a debug build:

```sh
NO_COLOR=true trunk build --release
npx http-server dist -a 127.0.0.1 -p 1432 --silent
```

Chromium was driven through Playwright. The script opened the built app, blocked remote Stark S3 image requests, typed
representative search strings into `#catalog-search`, recorded per-input latency after two animation frames, and captured
a Chrome CPU profile plus a DevTools timeline trace.

The profiling artifacts were written outside the repository under `/tmp/stark-profile/`:

```text
search.cpuprofile
search-trace.json
timings.json
```

## Results

The slow cases were broad queries that matched many catalog rows and therefore rendered large result sets:

```text
query             elapsed     matches   tree nodes   detail cards   DOM nodes
S                3028.0 ms       1904         3878           1904      61589
SM               6127.8 ms       1448         2993           1448      47088
SMX              5825.1 ms       1006         2207           1006      32942
SMX1               52.4 ms       1006         2207           1006      32942
SMX1-             793.3 ms       1006         2207           1006      32942
SMX1-T            678.0 ms        984         2155            984      32220
SMX1-TO           508.3 ms        645         1449            645      21240
SMX1-TOOLBOX       48.8 ms          4           20              4        216
f                8274.8 ms       1737         3537           1737      56245
fr                975.7 ms       1115         2324           1115      36225
fro               752.1 ms        759         1628            759      24848
front             387.9 ms        448          960            448      14748
front brake       116.4 ms         85          229             85       2928
no result         118.1 ms          0            0              0         53
```

The trace pointed at layout and DOM work, not the string scan, as the main problem:

```text
54517.6 ms  RunTask
35999.8 ms  Layout
19825.1 ms  EventDispatch
 5954.9 ms  RunMicrotasks
 5704.0 ms  FunctionCall
 2202.7 ms  PrePaint
 1294.6 ms  UpdateLayoutTree
```

The top sampled frames were also dominated by browser and DOM operations such as `setAttribute`, `remove`,
`insertBefore`, `createElement`, and `createTextNode`. WASM frames were present, but they were not the top of the profile.

I also ran a quick A/B pass with `.results { display: none !important; }` injected after the app mounted. That still ran
the search and reactive update path, but removed most visible layout and paint work:

```text
query   visible results   hidden results
S            885.6 ms          110.0 ms
SM          1019.9 ms          158.6 ms
SMX          901.8 ms          120.3 ms
front        441.3 ms           60.2 ms
```

The numbers vary between runs, but the shape is clear: broad searches create and lay out far too much DOM on every
keystroke.

## Current Mechanics

The search model is simple and browser-local. At startup, the app parses the committed JSON5 catalog and builds a
`SearchIndex`. The index has one row per article variant/SKU-ish leaf. Each row stores a lowercased normalized text blob
and a punctuation-stripped compact text blob.

On every input event, the app scans all rows, checks substring matches, projects matching rows back into an ancestor tree,
flattens that tree for rendering, and renders every matching detail card. This is acceptable for specific queries, but it
is a bad shape for one-character and two-character queries because those queries match a very large fraction of the
catalog.

## Suggested Improvements

The first fix should be to stop rendering every detail card for broad searches. The detail cards are expensive because
each one contains multiple nested elements, optional attributes, stale-data text, image tags, and links. For a query like
`S`, the app currently renders about 1900 cards. That is not useful to a person and it is costly to the browser. A better
shape is to show the tree and a concise summary first, then show details only for a selected row, a small initial slice,
or an explicit "show details" action. If broad queries still need detail previews, cap them with clear UI text such as
"Showing the first 50 details; narrow the search to see more." That cap should be on details, not on search results, so
the tree can still communicate the breadth of the match.

The second fix should be to virtualize the visible tree and detail list if the UI is meant to show hundreds or thousands
of rows. The current code rebuilds and lays out a full `<ol>` plus all result cards after each input. Virtualization
would keep the logical result set intact while only mounting the rows near the viewport. That reduces DOM size, layout
cost, and garbage collection pressure. This is especially relevant for the tree because broad searches can produce
thousands of flattened tree nodes even before details are considered.

The third fix should be to avoid doing expensive work synchronously for every keystroke. A small debounce can help, but
it should not be the main fix because it only hides the cost until the user pauses. A better version is to update the
input value immediately, schedule search/render work with a short delay or `requestAnimationFrame`, and cancel obsolete
work when a newer keystroke arrives. This matters most when someone types quickly through broad intermediate states like
`S`, `SM`, and `SMX` on the way to a specific SKU.

The fourth fix is to split search computation from rendering enough to measure them independently. The current profile
already says rendering dominates, but future changes should add cheap instrumentation around `search_index.search`,
tree projection, tree flattening, and detail rendering. This does not need to be permanent user-visible code. Even a
developer-only profiling harness would make it much harder to misdiagnose a future slowdown as "search is slow" when the
actual problem is layout.

Only after those rendering changes would I spend much time optimizing the string search itself. A more sophisticated
index could help eventually: token-to-row indexes, prefix indexes for SKU-like strings, or precomputed result sets for
common short tokens. But the current evidence says a faster scan will not solve the worst user-visible pauses while the
UI still tries to mount tens of thousands of DOM nodes for broad queries.
