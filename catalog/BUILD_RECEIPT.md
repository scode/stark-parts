# Stark Catalog Build Receipt

This catalog was generated from Stark's public US storefront on 2026-07-18 UTC.

Build command:

```sh
RUST_LOG=stark_parts=info,stark_parts_catalog=warn cargo run -p stark-parts-cli -- catalog update
```

Relevant output:

```text
2026-07-18T14:06:43.681517Z  INFO run_with{repo_root=/home/scode/git/stark-parts}: stark_parts: catalog written path=/home/scode/git/stark-parts/catalog/stark-parts.json5
catalog written: catalog/stark-parts.json5
```

Generated catalog hash:

```sh
sha256sum catalog/stark-parts.json5
```

Relevant output:

```text
6ed65afd1937618ea7204733555d0518f0e9dd15eaf69fb214ecadaa449b886c  catalog/stark-parts.json5
```
