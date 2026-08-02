# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-08-02 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-08-02T00:03:25.223375Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
b53861250e00380b77f3dfdafa1b01b48b6b894c091b4cbfa47a2ff22eaa87a1  catalog/stark-parts.json5
```
