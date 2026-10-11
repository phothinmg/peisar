# Node.js usage

Peisar's main Node.js package is the native addon published as `peisar`.
Use it in Node.js servers, scripts, build tools, and frameworks that can load
native `.node` binaries.

This page is the Node.js overview. Use it as the entry point for framework,
runtime, and API-specific guides.

## At a glance

- Package: `peisar`
- Module surface: `require("peisar")` or `import ... from "peisar"`
- Runtime requirement: see the `engines.node` field in the root
  [package.json](../../package.json)
- Browser and WASI runtimes: see [WASM](./wasm.md)
- Full API details: see [Node.js API reference](./apis.md)

For browser-only runtimes or environments that cannot load native addons, use
the WebAssembly package documented in [WASM](./wasm.md).

## Install

```sh
npm i peisar
```

## Basic usage

```js
const { Peisar } = require("peisar");

const document = new Peisar(
  `---
title: Hello
---

# Hello **world**`,
  { fragment: true },
);

console.log(document.html);
console.log(document.frontmatter);
console.log(document.astJson);
```

## Current API structure

The Node.js package currently centers around these exports:

| Export | Purpose |
| --- | --- |
| `Peisar` | Parse one Markdown document and read `ast`, `html`, `frontmatter`, or `astJson` |
| `frontmatter()` | Split Markdown into body content and YAML metadata |
| `yamlParser()` | Parse inline YAML into a JS object |
| `loadYaml()` | Load and parse a YAML file |
| `PeisarCache` | Cache Markdown and assets for static-site-style workflows |

Detailed signatures and option shapes are in the
[Node.js API reference](./apis.md).

## Preferred extension API

Use `usePlugin()` for all new Node.js integrations. A plugin may provide any
combination of:

- `visitBlock`
- `visitInline`
- `parseBlock`
- `parseInline`

`useVisitor()` and `useParser()` are still supported as backward-compatible
aliases for older integrations, but they are now the deprecated path.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });

document.usePlugin({
  visitBlock([block]) {
    if (block.type === "Heading") {
      return { recurse: true };
    }
  },
});
```

## Related guides

- [Next.js](./nextjs.md)
- [Vite](./vite.md)
- [WASM environments](./wasm.md)
- [Node.js API reference](./apis.md)
- [Documentation hub](../README.md)

## Examples

- [Next.js example](../../examples/next)
- [Vite example](../../examples/vite)
