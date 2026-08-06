# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-06 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-06T02:52:28.049399Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
e17f7bf1d700c312ff6c2646dd0e76053f746654f1a5137662d13097ac805d71  catalog/stark-parts.json5
```
