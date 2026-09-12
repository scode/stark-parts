# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-09-12 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-09-12T23:16:54.675945Z  INFO run_with{repo_root=/tmp/hermes-codex-chores-egmcvuxa/run-stark-parts-catalog}: stark_parts: catalog written path=/tmp/hermes-codex-chores-egmcvuxa/run-stark-parts-catalog/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
df83386e56e3203c78fa7b4aed2e9599c224e34450b0739c2a0b33219cb42012  catalog/stark-parts.json5
```
