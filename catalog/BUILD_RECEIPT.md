# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-11 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-11T01:44:06.062937Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
c6187a2df03516124d538933ce3ef40795943e1daa33d886e9d8b2eda8054dca  catalog/stark-parts.json5
```
