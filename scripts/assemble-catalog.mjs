import { copyFile, mkdir } from "node:fs/promises";

const outputDirectory = new URL("../dist/", import.meta.url);
const sourceCatalog = new URL("../catalog/stark-parts.json5", import.meta.url);
const outputCatalog = new URL("stark-parts.json5", outputDirectory);

await mkdir(outputDirectory, { recursive: true });
await copyFile(sourceCatalog, outputCatalog);
