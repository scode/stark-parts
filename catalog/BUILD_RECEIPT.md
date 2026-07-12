# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-12 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-12T02:07:52.234665Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
5f77ee6f7108a666b718352f57182098fe0b80cdac7dd875e652b6ca7d2e2f76  catalog/stark-parts.json5
```
