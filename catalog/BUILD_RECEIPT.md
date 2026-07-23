# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-23 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-23T04:39:53.365544Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
7ecc7abc56c2160c52b2f792936df7929773494aba6548395e4619c11ab975c6  catalog/stark-parts.json5
```
