# Node.js API reference

This document covers the Node.js API in the `peisar` namespace exposed by the
`peisar` package. Its main entry point parses and renders a single Markdown
document.

## `peisar` API

| Export | Kind | Purpose |
| --- | --- | --- |
| `Peisar` | class | Parse Markdown and access its AST, HTML, and frontmatter |
| `PeisarAst` | class | Parse Markdown and access its AST and frontmatter |
| `PeisarOptions`, `AstOptions`, `RenderOptions` | interfaces | Configure parsing and rendering |
| `PeisarDocument`, `Block`, `Inline`, and supporting AST types | types | Describe AST nodes, spans, attributes, and table/list data |
| `PeisarPlugin` and plugin context/result/control types | interfaces and types | Implement parser hooks and AST visitors |
| `Frontmatter`, `FrontmatterType`, `FrontmatterResult` | interface and type | Describe extracted frontmatter |

## `Peisar`

### Constructor

```ts
new Peisar(markdown: string, options?: PeisarOptions)
```

### Getters

| Member | Type | Description |
| --- | --- | --- |
| `ast` | `PeisarDocument` | Parsed AST as a JavaScript object |
| `html` | string | Rendered HTML |
| `frontmatter` | `Frontmatter` | Detected format and raw frontmatter content; not deserialized |

### Methods

`use(plugin)` registers parser hooks, AST visitors, or both on the document.
Registering a plugin invalidates any cached HTML.

`setHtmlCacheEnabled(enabled)` enables or disables an instance-local cache of
the most recently rendered HTML. Caching is disabled by default and is cleared
when disabled or when a plugin is registered.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });
document.use({
  visitBlock([block]) {
    if (block.type === "Heading") return { recurse: true };
  },
});
```

Visitor callbacks receive a one-element tuple containing the node, so
`visitBlock([block])` and `visitInline([inline])` are the callback forms.
Visitors run when the AST or HTML is accessed. Parser hooks run before the
built-in matchers; registering them reparses the original source.

## `PeisarAst`

`PeisarAst` provides lower-level parsing without rendering:

```js
const { PeisarAst } = require("peisar");

const parsed = new PeisarAst("# Hello");
const ast = parsed.getAst();
const frontmatter = parsed.getFrontmatter();
```

It also provides `addPlugin(plugin)` and `clearPlugins()`. Its optional
`AstOptions` supports `gfm`, `kramdown`, and `fileName`.

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
consumed. A parser hook can return `undefined` to decline the current input.

The generated TypeScript declarations place these classes and types under
`peisar`; JavaScript examples can import the classes from the package root as
shown above.

Frontmatter is recognized at the start of the Markdown source:

| Format | Opening | Closing |
| --- | --- | --- |
| YAML | `---` | `---` |
| TOML | `+++` | `+++` |
| JSON | `{` or optional `;;;` followed by `{` | Matching `}`; with `;;;`, a closing `;;;` is required |

The `raw` property contains the text between YAML/TOML delimiters or the
complete JSON object without optional `;;;` delimiters. `frontmatterType` is
one of `"Yaml"`, `"Toml"`, `"Json"`, or `"Empty"`. No YAML, TOML, or JSON
parsing or validation is done. When no complete frontmatter is found, the
type is `"Empty"` and `raw` is an empty string.
