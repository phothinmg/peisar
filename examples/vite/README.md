# Peisar with Vite App Example

Vite browser applications must use Peisar's WebAssembly package rather than
the native `peisar` package:

```sh
npm i @peisar/peisar-wasm32-wasi
```

Import it directly in client-side code:

```ts
import { Peisar } from "@peisar/peisar-wasm32-wasi";

const document = new Peisar("# Hello from Vite", { fragment: true });
console.log(document.html);
```

The WebAssembly package uses threads and requires cross-origin isolation. Add
the required headers and exclude the package from Vite's dependency optimizer
so its internal worker is handled correctly:

```ts
// vite.config.ts
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

After changing this configuration, clear Vite's optimized-dependency cache and
restart the development server:

```sh
rm -rf node_modules/.vite
npm run dev
```

For production builds, configure the same headers in the server or hosting
platform that serves the app. Vite's `server` and `preview` headers apply only
to local development and `vite preview`. Verify the deployed app reports
`globalThis.crossOriginIsolated === true`. With
`Cross-Origin-Embedder-Policy: require-corp`, cross-origin assets must also
allow embedding through CORS or `Cross-Origin-Resource-Policy`.

## Deploy

### Vercel

` vercel.json`

```json
{
  "headers": [
    {
      "source": "/(.*)",
      "headers": [
        {
          "key": "Cross-Origin-Opener-Policy",
          "value": "same-origin"
        },
        {
          "key": "Cross-Origin-Embedder-Policy",
          "value": "require-corp"
        }
      ]
    }
  ]
}
```
