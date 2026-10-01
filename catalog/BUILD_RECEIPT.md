# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-10-01 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
catalog written: catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
65c5c9f4c25bfa8c4654af022b3419c740d730684534bc888b927820a8a6287d  catalog/stark-parts.json5
```
