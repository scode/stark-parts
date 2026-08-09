# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-09 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-09T02:37:47.302149Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
b976dbc97d3a10d85bc802adedec99e29e3af5e1a98ea4adbe35d8b6521e3433  catalog/stark-parts.json5
```
