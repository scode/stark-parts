import { copyFile, mkdir } from "node:fs/promises";

// This step intentionally runs outside Turborepo's cached application task. A cache hit may restore an app built days
// earlier, so the current checkout's catalog must be copied afterward and remain the authoritative deployment data.
const outputDirectory = new URL("../dist/", import.meta.url);
const sourceCatalog = new URL("../catalog/stark-parts.json5", import.meta.url);
const outputCatalog = new URL("stark-parts.json5", outputDirectory);

await mkdir(outputDirectory, { recursive: true });
await copyFile(sourceCatalog, outputCatalog);
