# Stark Catalog Build Receipt

This catalog was generated from Stark's current US Parts storefront on 2026-11-14 UTC, using its unauthenticated v3
catalog API, including mounting hardware.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
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
75a6590b667164ec2e2fdea32a6bacb072f7f1bc1bd6431e1bdcb7d3653a46b8  catalog/stark-parts.json5
```
