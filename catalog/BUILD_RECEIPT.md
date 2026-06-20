# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-06-20 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-06-20T15:08:34.340550Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
3ab275405b4e7aee7ed3e35bdd3a73c04b742b17e0f856fc943aa1433f3ed148  catalog/stark-parts.json5
```
