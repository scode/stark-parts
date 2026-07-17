# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-17 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-17T02:34:44.284730Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
970b0a481fb569eb7daa143ff8a9df9387ecc8d21e06b6ea360b89c711bfa1fa  catalog/stark-parts.json5
```
