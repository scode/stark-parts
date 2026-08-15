# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-15 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-15T02:00:41.377430Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
72cb6378521b6906e5ace542cc71c0456d8878c74e021549c613fceaee838c98  catalog/stark-parts.json5
```
