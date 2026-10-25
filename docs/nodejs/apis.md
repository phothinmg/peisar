# Node.js API reference

This document reflects the current Node.js-facing surface exposed by the
native `peisar` package.

## Package exports

| Export | Kind | Notes |
| --- | --- | --- |
| `Peisar` | class | Main parse/render API |
| `PeisarCache` | class | Memory-first Markdown and asset cache |
| `frontmatter` | function | Extract YAML front matter and Markdown body |
| `yamlParser` | function | Parse YAML text into a JS object |
| `loadYaml` | function | Load and parse a YAML file from disk |

## `Peisar`

### Constructor

```ts
new Peisar(markdown: string, options?: PeisarOptions)
```

### Getters

| Member | Type | Description |
| --- | --- | --- |
| `ast` | object | Parsed AST as a JS object |
| `html` | string | Rendered HTML |
| `frontmatter` | `Record<string, any> \| null` | Parsed YAML front matter |
| `astJson` | string | AST serialized as JSON |

### Methods

| Method | Description |
| --- | --- |
| `usePlugin(plugin)` | Preferred extension entry point |
| `useVisitor(visitor)` | Deprecated compatibility alias for visitor-only plugins |
| `useParser(parser)` | Deprecated compatibility alias for parser-only plugins |

### Example

```js
const { Peisar } = require("peisar");

const document = new Peisar("See [[Some Page]].", { fragment: true });

document.usePlugin({
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return;
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
    const zero = { line: 0, column: 0, offset: 0 };
    return {
      inline: {
        type: "Link",
        text: [{ type: "Text", value: target, pos: { start: zero, end: zero } }],
        url: `/wiki/${target.replaceAll(" ", "_")}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 2,
    };
  },
});

console.log(document.html);
```

## `PeisarOptions`

All properties are optional.

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `gfm` | `boolean` | `true` | Enable GitHub Flavored Markdown |
| `kramdown` | `boolean` | `true` | Enable Kramdown block attributes |
| `fileName` | `string` | `undefined` | Attach a file name to the parsed document |
| `fragment` | `boolean` | `false` | Render a fragment instead of a full HTML document |
| `charset` | `boolean` | `true` | Include `<meta charset>` in full-document mode |
| `viewport` | `boolean` | `true` | Include `<meta name="viewport">` in full-document mode |
| `title` | `string` | `undefined` | HTML `<title>` in full-document mode |
| `bodyClass` | `string` | `undefined` | Extra classes for `<body>` in full-document mode |
| `style` | `string` | `undefined` | Inline CSS added to `<head>` |

## Plugin object shape

`usePlugin()` accepts a plain object. Every callback is optional.

```ts
type Plugin = {
  visitBlock?: (args: [block: object]) => VisitorControl | void;
  visitInline?: (args: [inline: object]) => InlineVisitorControl | void;
  parseBlock?: (args: [ctx: BlockParserContext]) => BlockParseResult | void;
  parseInline?: (args: [ctx: InlineParserContext]) => InlineParseResult | void;
};
```

### Visitor controls

The visitor callbacks may return:

- `insertBefore`
- `insertAfter`
- `replaceWith`
- `remove`
- `recurse`

### Parser hook contexts

`parseBlock` receives:

```ts
{
  line: string;
  lineIndex: number;
  lines: string[];
}
```

`parseInline` receives:

```ts
{
  rest: string;
  index: number;
}
```

### Parser hook results

`parseBlock` returns:

```ts
{
  block?: object;
  consumed?: number;
}
```

`parseInline` returns:

```ts
{
  inline?: object;
  consumed?: number;
}
```

## `frontmatter()`

```ts
frontmatter(markdown: string): {
  pureMarkdownContent: string;
  yamlData: Record<string, any> | null;
}
```

Behavior:

- front matter must start at the first non-whitespace line
- the opening and closing delimiter is `---`
- invalid YAML throws
- unterminated front matter is treated as normal Markdown content

## `yamlParser()` and `loadYaml()`

```ts
yamlParser(yaml: string): Record<string, any>
loadYaml(filePath: string): Record<string, any>
```

Use these helpers when you need standalone YAML parsing outside the Markdown
pipeline.

## `PeisarCache`

`PeisarCache` recursively loads Markdown from an entry directory and
optionally assets from a second directory. Reads use an in-memory LRU first,
then fall back to a disk archive for evicted entries. The archive is written
as binary shards plus a `map.json` index.

### Construction

```ts
new PeisarCache(entryDir: string, assetsDir?: string, options?: PeisarCacheJsOptions)
PeisarCache.withConfig(config: PeisarCacheConfig): PeisarCache
```

The constructor defaults `assetsDir` to `public` under the current working
directory when that directory exists. `withConfig()` accepts the full
configuration, including memory-tier budgets.

```js
const { PeisarCache } = require("peisar");

const cache = PeisarCache.withConfig({
  entryDir: "contents",
  assetsDir: "public",
  markdown: { fragment: true },
  memory: { byteBudget: 32 * 1024 * 1024, entryBudget: 500 },
  archive: { shardBytes: 8 * 1024 * 1024, flushIntervalMs: 2000 },
  hosting: { provider: "GithubPages", repo: "my-site" },
  outDir: "dist",
  cacheDir: ".peisar-cache",
});

console.log(cache.pages());
cache.dispose();
```

### Configuration

`entryDir` is required when calling `withConfig()`; the Rust
`PeisarCacheConfig::default()` value is `"contents"`. Other configuration
fields are optional.

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `entryDir` | `string` | `"contents"` | Markdown source directory; relative paths use the current working directory |
| `assetsDir` | `string \| undefined` | Existing `public` directory, otherwise none | Optional asset source directory |
| `markdown` | `PeisarOptions` | Peisar defaults | Markdown parsing and rendering options used for page HTML |
| `memory` | `MemoryConfig` | 64 MiB, unlimited entries | LRU limits; `0` means unlimited |
| `archive` | `ArchiveConfig` | 8 MiB shards, 2000 ms | Pending-write flush thresholds; zero disables the corresponding threshold |
| `hosting` | `HostingConfig` | Custom provider, root path | Base URL configuration for page and asset slugs |
| `outDir` | `string` | `".peisar"` | Output root used in page and asset `outFilePath` values |
| `cacheDir` | `string` | `".peisar-cache"` | Archive directory under the current working directory |

The regular constructor options are `PeisarCacheJsOptions`: `markdown`,
`archive`, `hosting`, `outDir`, and `cacheDir`. Use `withConfig()` to set
`memory` budgets.

`MemoryConfig` fields:

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `byteBudget` | `number` | `67108864` | Maximum raw content bytes held in memory; zero is unlimited |
| `entryBudget` | `number` | `0` | Maximum in-memory entries; zero is unlimited |

`ArchiveConfig` fields:

| Property | Type | Default | Description |
| --- | --- | --- | --- |
| `shardBytes` | `number` | `8388608` | Flush pending content after reaching this size; zero disables size-triggered flushes |
| `flushIntervalMs` | `number` | `2000` | Flush pending content after this interval; zero disables timer-triggered flushes |

`HostingConfig` fields are `provider`, `repo`, `siteUrl`, and `pathPrefix`.
Providers are `"GithubPages"`, `"Vercel"`, `"Netlify"`,
`"CloudflarePages"`, and `"Custom"`. GitHub Pages project sites use `repo` to
form a `/<repo>/` base path; `pathPrefix` adds an explicit path prefix.

### Discovery and cache tiers

- Markdown extensions: `md`, `markdown`, `mdown`, `mkdn`, `mkd`, `mdwn`,
  `mkdown`, and `ron`.
- Asset extensions: common image, script/style, font, and audio/video types
  (`png`, `jpg`, `svg`, `js`, `css`, `woff2`, `mp4`, and others).
- Markdown and textual assets (`js`, `css`, `map`, `svg`) are cached as
  UTF-8 text. Other recognized assets are cached as bytes.
- The in-memory LRU is the hot tier. Evicted entries remain readable through
  the archive tier, so its budgets limit the hot set rather than total cache
  availability.
- Archive writes run in the background and flush on the size threshold,
  interval, `flush()`, watcher shutdown, or disposal.

### Methods

| Method | Description |
| --- | --- |
| `getText(absPath)` | Read a cached text entry, or `null`; checks memory and then the archive |
| `getBinary(absPath)` | Read a cached binary asset as a `Buffer`, or `null` |
| `listFiles()` | Absolute paths for all cached entries |
| `markdownFiles()` / `assetFiles()` | Absolute paths filtered to Markdown or assets |
| `pages()` / `assets()` | Derived page or asset objects, sorted by slug |
| `getPage(absPath)` / `getAsset(absPath)` | One derived object for an absolute source path, or `null` |
| `archived()` | Absolute paths already flushed to the archive |
| `mapJson()` | Current archive index JSON; pending entries are not included until flushed |
| `flush()` | Flush pending archive writes and wait for completion |
| `baseUrl()` / `siteUrl()` / `resolveUrl(path)` | Read or apply configured hosting URL values |
| `usePlugin(plugin)` | Register a Markdown plugin applied when page HTML is rendered |
| `startWatchingJs()` | Watch the entry and asset directories recursively |
| `onChange(callback)` / `offChange(id)` | Subscribe to and remove file-change callbacks |
| `dispose()` | Stop watching, flush writes, and clear callbacks; safe to call repeatedly |

`pages()` returns objects with `layout`, `fileName`, `slug`, `html`,
`outFilePath`, `inputFilePath`, `title`, `summary`, `tags`, `publishDate`,
and `extra`. `layout` defaults to `"default"`; title falls back to the first
Markdown heading. `assets()` returns `fileName`, `slug`, `raw`, `outFilePath`,
`inputFilePath`, `ext`, and `fileType`. Binary asset `raw` values are base64
strings; `getBinary()` returns the raw bytes.

Call `onChange()` before starting the watcher to receive
`{ path, kind, isMarkdown }` events. `kind` is `"create"`, `"modify"`,
`"remove"`, or `"other"`; callbacks receive raw watcher events while pretty
stderr logs coalesce duplicate file events. In Node.js, `startWatchingJs()`
keeps the event loop alive until `dispose()` stops the watcher.
