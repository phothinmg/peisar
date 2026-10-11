# Node.js usage: Vite

Vite browser applications should use Peisar's WebAssembly package instead of
the native `peisar` package.

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

This matches the example app in [examples/vite](../../examples/vite).

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
