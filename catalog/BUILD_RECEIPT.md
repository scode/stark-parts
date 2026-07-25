# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-25 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-25T00:44:19.499221Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
f20d9c30bb6d65034251d067b8300387df4908f314b1c118596c826834ad5a7e  catalog/stark-parts.json5
```
