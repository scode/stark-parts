# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-15 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-15T02:46:16.697017Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
013187c7eb5e67d4986b334521dc88ce2daea46c37403ea8eae351e59e34cf78  catalog/stark-parts.json5
```
