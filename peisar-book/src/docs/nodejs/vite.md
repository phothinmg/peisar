# Node.js usage: Vite

For direct browser-side parsing, Vite applications should use Peisar's
WebAssembly package instead of the native `peisar` package. To import Markdown
files as React components, use the
[`vite-plugin-peisar`](https://github.com/phothinmg/vite-plugin-peisar) plugin.

## Import Markdown with vite-plugin-peisar

The plugin lets a Vite application import Markdown as a React component, or
import its rendered HTML, original source, and frontmatter. Install it with:

```sh
npm install vite-plugin-peisar
```

Add it to your Vite configuration alongside the React plugin:

```ts
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { vitePluginPeisar } from "vite-plugin-peisar";

export default defineConfig({
  plugins: [react(), vitePluginPeisar()],
});
```

The plugin creates `src/vite-peisar-env.d.ts` with the required module
declarations. To add the declarations yourself, create that file with:

```ts
/// <reference types="vite/client" />
/// <reference types="vite-plugin-peisar/client" />
```

Import a Markdown file as a React component:

```tsx
import Guide, { frontmatter, html, raw } from "./guide.md";

export function Page() {
  return (
    <article>
      <h1>{frontmatter.title}</h1>
      <Guide />
    </article>
  );
}
```

The default export renders the Markdown as a React component. Named exports
provide the rendered `html`, original `raw` Markdown, and parsed `frontmatter`.
To import rendered HTML as the default export, use the `?html` query:

```ts
import html, { frontmatter, raw } from "./guide.md?html";
```

See the
[`vite-plugin-peisar` README](https://github.com/phothinmg/vite-plugin-peisar#readme)
for supported extensions and plugin options.

## Use WASM for direct parsing

## Install

```sh
npm i @peisar/peisar-wasm32-wasi
```

## Basic usage

```ts
import { Peisar } from "@peisar/peisar-wasm32-wasi";

const document = new Peisar("# Hello from Vite", { fragment: true });

console.log(document.html);
```

## Vite configuration

The WASM package uses threads, so local development and preview must enable
cross-origin isolation headers. Exclude the package from Vite's dependency
optimizer so its worker setup is preserved.

```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  server: {
    headers: {
      "Cross-Origin-Opener-Policy": "same-origin",
      "Cross-Origin-Embedder-Policy": "require-corp",
    },
  },
  plugins: [
    react(),
    {
      name: "configure-preview-response-headers",
      configurePreviewServer(server) {
        server.middlewares.use((_req, res, next) => {
          res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
          res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
          next();
        });
      },
    },
  ],
  optimizeDeps: {
    exclude: ["@peisar/peisar-wasm32-wasi"],
  },
});
```

After changing the config, clear Vite's optimized dependency cache and restart
the dev server:

```sh
rm -rf node_modules/.vite
npm run dev
```

## Production notes

- Configure the same headers in production.
- Verify `globalThis.crossOriginIsolated === true`.
- With `Cross-Origin-Embedder-Policy: require-corp`, cross-origin assets must
  also allow embedding through CORS or `Cross-Origin-Resource-Policy`.
