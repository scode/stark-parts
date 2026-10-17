# Catalog fixtures

`interaction-catalog.json` is a small, frozen legacy-format snapshot for UI interaction tests. Its products are
examples, not inputs to catalog refreshes. Keeping them fixed lets tests distinguish a removed upstream product from a
change to search, hover persistence, bike filtering, or rendering.

`shared-catalog.json` is a frozen schema-2 kit with colour-specific descriptions and components, a conditional fitment,
and selected-variant links. It also covers an explicit empty component list and legacy article-level fallback.

The deployed-catalog browser test and committed-catalog search test discover a SKU from the real generated snapshot.
They must not require a particular upstream SKU or a fixed number of models to remain available.
