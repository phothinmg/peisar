# Node.js usage: Next.js

Use the native `peisar` package in Next.js when Markdown parsing happens on
the server.

If you need client-side rendering in the browser, follow the [Vite](./vite.md)
or [WASM](./wasm.md) guides instead.

## Install

```sh
npm i peisar
```

## Configure Next.js

Next.js should treat `peisar` as a server-side external package so Node.js can
load the native addon from `node_modules`.

### `next.config.ts`

```ts
import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  serverExternalPackages: ["peisar"],
};

export default nextConfig;
```

### `next.config.js` or `next.config.mjs`

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  serverExternalPackages: ["peisar"],
};

module.exports = nextConfig;
// or: export default nextConfig
```

## Example usage

```tsx
import { Peisar } from "peisar";
import fs from "node:fs";

const content = fs.readFileSync("README.md", "utf8");
const document = new Peisar(content, { fragment: true });

export default function Home() {
  return <div dangerouslySetInnerHTML={{ __html: document.html }} />;
}
```

This matches the example app in [examples/next](../../examples/next).

## When not to use the native package

Do not use the native Node.js package in environments that cannot load `.node`
bindings, such as:

- browser bundles
- Edge runtimes
- WASI-only environments

For those cases, use the [WASM guide](./wasm.md).
