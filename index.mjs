import { createRequire } from "module";
const require = createRequire(import.meta.url);
const peisar = require("./index.cjs");

export const Peisar = peisar.Peisar;
export const PeisarCache = peisar.PeisarCache;
export const frontmatter = peisar.frontmatter;
export const yamlParser = peisar.yamlParser;
