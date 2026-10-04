# Peisar with Next.js App Example

Next.js provides a built-in configuration option called `serverExternalPackages`.This opts specific dependencies out of Server Component bundling and allows the server to use a native Node.js `require` to load them directly from `node_modules`.

Add `peisar` to `serverExternalPackages` in your configuration file:

For `next.config.ts` (TypeScript)

```ts
import type { NextConfig } from 'next'

const nextConfig: NextConfig = {
  serverExternalPackages: ["peisar"],
}

export default nextConfig

```

For `next.config.js` or `next.config.mjs` (JavaScript)

```js
/** @type {import('next').NextConfig} */
const nextConfig = {
  serverExternalPackages: ["peisar"],
}

module.exports = nextConfig // or export default nextConfig if using .mjs

```