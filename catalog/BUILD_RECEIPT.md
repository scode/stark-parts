# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-06-08 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-06-08T23:44:14.285971Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
72b6401c78054c1900f1a6c7f69e51dc8ca6c8964142401932a076f4eba00c02  catalog/stark-parts.json5
```
