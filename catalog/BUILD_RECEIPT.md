# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-20 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-20T02:52:34.686912Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
014bb436fffef22f114e612f23531d1498cc5c98c36aa35b4ef5a6b363c1d90e  catalog/stark-parts.json5
```
