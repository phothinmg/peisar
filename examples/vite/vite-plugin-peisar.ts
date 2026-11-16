import { Peisar, type PeisarOptions } from "@peisar/peisar-wasm32-wasi";
import type { Plugin } from "vite";

export function vitePluginPeisar(options?: PeisarOptions): Plugin {
  const extensions = [
    ".md",
    ".markdown",
    ".mdown",
    ".mkdn",
    ".mkd",
    ".mdwn",
    ".mkdown",
    ".ron",
  ];
  const isolationHeaders = {
    "Cross-Origin-Opener-Policy": "same-origin",
    "Cross-Origin-Embedder-Policy": "require-corp",
  };
  return {
    name: "vite-plugin-peisar",
    enforce: "pre",

    config() {
      return {
        server: {
          headers: isolationHeaders,
        },
        optimizeDeps: {
          exclude: ["@peisar/peisar-wasm32-wasi"],
        },
      };
    },
    // 2. Apply the same isolation headers to `vite preview`
    configurePreviewServer(server) {
      server.middlewares.use((_req, res, next) => {
        res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
        res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
        next();
      });
    },

    transform(code, id) {
      if (!extensions.some((ext) => id.endsWith(ext))) return;

      const peisar = new Peisar(code, options);
      const html = peisar.html;

      return {
        code: `
          import React from "react";

          export const html = ${JSON.stringify(html)};
          export const raw = ${JSON.stringify(code)};

          export default function MarkdownContent(props) {
            return React.createElement("div", {
              ...props,
              dangerouslySetInnerHTML: { __html: html },
            });
          }
        `,
        map: null,
      };
    },
  };
}
