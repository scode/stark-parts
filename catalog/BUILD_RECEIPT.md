# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-03 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-03T00:29:21.382757Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
07ac7536ae032d85668617d2097e73bb9241b93bebb8674328e2a96b99e8eacd  catalog/stark-parts.json5
```
