# Stark Catalog Build Receipt

This catalog was generated from Stark's current US Parts storefront on 2026-11-19 UTC, using its unauthenticated v3
catalog API, including mounting hardware.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run --locked -p stark-parts-cli -- catalog update
```

Relevant output:

```text
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
75649f0adeb7eabee0d0d687320f741188a8d27dde8db967f9044223d360b045  catalog/stark-parts.json5
```
