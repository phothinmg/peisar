# Vite plugin for Peisar

Use [Peisar](https://github.com/phothinmg/peisar), a Rust-powered Markdown
parser, to import Markdown files into a Vite application. Markdown imports
include a React component and rendered HTML, and expose the source text and
frontmatter as named exports.

## Install

```bash
npm install vite-plugin-peisar
```

## Configure Vite

Add the plugin to your Vite configuration:

```typescript
import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'
import {vitePluginPeisar} from "vite-plugin-peisar";
// https://vite.dev/config/
export default defineConfig({
  plugins: [react(),vitePluginPeisar()],
})
```

If you prefer to create the file manually, add `src/vite-peisar-env.d.ts` with
the following content:

```typescript
/// <reference types="vite/client" />
/// <reference types="vite-plugin-peisar/client" />
```

Otherwise, the plugin creates `vite-peisar-env.d.ts` in the project's `src`
directory if it does not already exist. This file provides TypeScript
declarations for Vite and Markdown modules.

## Import Markdown

Import a Markdown file as a React component:

```typescript
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

- The default export is a React component that renders the generated HTML.
- `html` is the rendered HTML string.
- `raw` is the original Markdown source.
- `frontmatter` is an object parsed from YAML, TOML, or JSON frontmatter (or an
  empty object if the document has none).

Use the `?html` query to import the rendered HTML string as the default export:

```typescript
import html, { frontmatter, raw } from "./guide.md?html";
```

Vite's asset query imports, such as `?raw` and `?url`, are left to Vite to
handle.

## Options

Pass an options object to `vitePluginPeisar(options)`:

| Option | Type | Default | Description |
| --- | --- | --- | --- |
| `fileExtensions` | Array of supported extension literals | `.md`, `.markdown`, `.mdown`, `.mkdn`, `.mkd`, `.mdwn`, `.mkdown`, `.ron` | File extensions to transform. |
| `plugins` | `peisar.PeisarPlugin[]` | `[]` | Peisar plugins to apply while rendering Markdown. |
| `gfm` | `boolean` | `true` | Enable GitHub Flavored Markdown. |
| `kramdown` | `boolean` | `true` | Enable Kramdown syntax. |
| `wrapperClass` | `string` | — | Set a CSS class on the generated React component's wrapper element. |
| `cache` | `boolean` | `true` | Enable the persistent render cache in Vite's cache directory. |


For example, to disable the persistent cache and set a wrapper class:

```typescript
vitePluginPeisar({
  cache: false,
  wrapperClass: "markdown-content",
});
```


