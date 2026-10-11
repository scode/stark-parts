# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-10-11 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli --target-dir /home/scode/.cache/chores/catalog-build-12bb1e4254204a8f823eb75dae1f48a3 -- catalog update
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
eb3c3c4f2f809c900704d6ddc1237882f184d34f467ea8cc191324f2b40b0704  catalog/stark-parts.json5
```
