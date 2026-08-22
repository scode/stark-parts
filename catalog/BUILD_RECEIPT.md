# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-22 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-22T04:06:09.266769Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
b5c182ce6f6c05f7d995da2e288c1ee61478ef709b76d33436c394799f9e3129  catalog/stark-parts.json5
```
