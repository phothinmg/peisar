# Node.js API reference

This document describes the Node.js-facing API exposed by the native
`peisar` package. The package provides single-document parsing and YAML
helpers; it does not expose a filesystem cache API.

## Package exports

| Export | Kind | Notes |
| --- | --- | --- |
| `Peisar` | class | Main parse/render API |
| `frontmatter` | function | Extract YAML front matter and Markdown body |
| `yamlParser` | function | Parse YAML text into a JavaScript value |
| `loadYaml` | function | Load and parse a YAML file |

## `Peisar`

### Constructor

```ts
new Peisar(markdown: string, options?: PeisarOptions)
```

### Getters

| Member | Type | Description |
| --- | --- | --- |
| `ast` | object | Parsed AST as a JavaScript object |
| `html` | string | Rendered HTML |
| `frontmatter` | `Record<string, any> \| null` | Parsed YAML front matter |
| `astJson` | string | AST serialized as JSON |

### Plugin method

`use(plugin)` registers parser hooks, AST visitors, or both on the document.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });
document.use({
  visitBlock([block]) {
    if (block.type === "Heading") return { recurse: true };
  },
});
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

## Plugin object

Each property is optional:

```ts
type Plugin = {
  visitBlock?: (args: [block: object]) => VisitorControl | void;
  visitInline?: (args: [inline: object]) => InlineVisitorControl | void;
  parseBlock?: (args: [ctx: BlockParserContext]) => BlockParseResult | void;
  parseInline?: (args: [ctx: InlineParserContext]) => InlineParseResult | void;
};
```

Visitor controls include `insertBefore`, `insertAfter`, `replaceWith`,
`remove`, and `recurse`. Block parser hooks return a `block` and the number of
lines consumed; inline hooks return an `inline` and the number of characters
consumed.

## YAML helpers

```ts
frontmatter(markdown: string): {
  pureMarkdownContent: string;
  yamlData: Record<string, any> | null;
}

yamlParser(yaml: string): Record<string, any>
loadYaml(filePath: string): Record<string, any>
```

Use these helpers to process YAML outside the `Peisar` document interface.
