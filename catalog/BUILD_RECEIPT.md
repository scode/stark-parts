# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-30 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-30T17:07:51.488006Z  INFO run_with{repo_root=/tmp/hermes-codex-chores-_fzih3eq/stark-parts}: stark_parts: catalog written path=/tmp/hermes-codex-chores-_fzih3eq/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
9736b2f0d757c589d3a034e66cf4cd41fd60068b568bc9da4f23e0eed098de69  catalog/stark-parts.json5
```
