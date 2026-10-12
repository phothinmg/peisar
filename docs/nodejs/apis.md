# Node.js API reference

This document reflects the current Node.js-facing surface exposed by the
native `peisar` package.

## Package exports

| Export | Kind | Notes |
| --- | --- | --- |
| `Peisar` | class | Main parse/render API |
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
