# Stark Catalog Build Receipt

This catalog was generated from Stark's current US Parts storefront on 2026-11-09 UTC, using its unauthenticated v3
catalog API, including mounting hardware.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
8481b5cae98766bc75125506566acbc9780aa2f9ab790b602c6776687b479fb2  catalog/stark-parts.json5
```
