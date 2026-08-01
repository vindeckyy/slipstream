import { readFile, writeFile } from "node:fs/promises";

const source = new URL("../../api/openapi.json", import.meta.url);
const target = new URL("../public/openapi.json", import.meta.url);

await writeFile(target, await readFile(source));
