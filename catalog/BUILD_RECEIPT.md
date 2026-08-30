# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-30 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-30T15:39:40.022434Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
05b6de20b0619be117c824b0de6ac9010b11594f04b56b2f443e80edb85f5cb6  catalog/stark-parts.json5
```
