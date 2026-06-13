# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-06-13 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-06-13T00:29:19.960546Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
42f8c3ecabec4a4820037e1207652a7d5335230b0fafcf7a5e1b17c45116f511  catalog/stark-parts.json5
```
