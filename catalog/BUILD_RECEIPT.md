# Stark Catalog Build Receipt

This catalog was generated from Stark's current US Parts storefront on 2026-10-17 UTC, using its unauthenticated v3
catalog API. It contains 449 products and 537 distinct SKUs, including mounting hardware.

The snapshot preserves each kit SKU's own composition and descriptions. The original MX model is labeled MX 1.0.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-10-17T23:09:54.508371Z  INFO run_with{repo_root=/home/scode/git/stark-parts-1}: stark_parts: catalog written path=/home/scode/git/stark-parts-1/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
656730b8d081fd09c04209590c70941a3d8446764f48ae782557c50d7ff06832  catalog/stark-parts.json5
```
