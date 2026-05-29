# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-05-28.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog init
```

Relevant output:

```text
catalog written: catalog/stark-parts.json5
```

Deterministic rerun check:

```sh
sha256sum catalog/stark-parts.json5
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
885b528f5c3246ba331efd8fc55afd46460a45cfdf70bd202fe3f2a3eb20ad39  catalog/stark-parts.json5
catalog unchanged: catalog/stark-parts.json5
885b528f5c3246ba331efd8fc55afd46460a45cfdf70bd202fe3f2a3eb20ad39  catalog/stark-parts.json5
```
