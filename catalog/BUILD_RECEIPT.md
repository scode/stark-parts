# Stark Catalog Build Receipt

This catalog was generated from Stark's current US Parts storefront on 2026-11-03 UTC, using its unauthenticated v3
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
cddea3e9274db27d8f56839a7f31638aedf20a0ee47a108fb1d23b2e65157d80  catalog/stark-parts.json5
```
