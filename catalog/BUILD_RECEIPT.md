# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-06-25 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-06-25T02:13:51.401451Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
7f776e74ef144ee5ad2a5f838ef28a919080a9f8d3f863184aa0afb5f963f4ae  catalog/stark-parts.json5
```
