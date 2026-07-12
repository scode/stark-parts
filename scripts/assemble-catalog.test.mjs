import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import test from "node:test";

const sourceCatalog = new URL("../catalog/stark-parts.json5", import.meta.url);
const outputCatalog = new URL("../dist/stark-parts.json5", import.meta.url);

test("assembly replaces a stale cached catalog with the committed catalog", async () => {
  // Model the only dangerous cache hit: the app is reusable, but its output directory contains yesterday's catalog.
  await writeFile(outputCatalog, "stale catalog data");

  await import("./assemble-catalog.mjs");

  assert.deepEqual(await readFile(outputCatalog), await readFile(sourceCatalog));
});
