---
name: peisar-parser-hooks
description: Extend Peisar with custom block or inline syntax during parsing in Node.js or Rust. Use for directives, wikilinks, mentions, and syntax that visitors cannot recognize after parsing.
---

# Peisar Custom Parser Hooks

Parser hooks run before the built-in parser at every candidate block or inline
position. Hooks run in registration order; the first one that returns a result
wins. Return `undefined` in Node.js or `None` in Rust to decline.

Use hooks for new or overridden syntax. Use visitors only for transformations
of nodes that the parser has already produced.

## Node.js

Register a plain object with `usePlugin()`. Parser callbacks receive a
one-item tuple; visitor callbacks may also be included in the same plugin.
`useParser()` remains a deprecated compatibility alias.

```js
const { Peisar } = require("peisar");

const document = new Peisar("See [[Some Page]].", { fragment: true });
const zero = { line: 0, column: 0, offset: 0 };

document.usePlugin({
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return;
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
    return {
      inline: {
        type: "Link",
        text: [
          { type: "Text", value: target, pos: { start: zero, end: zero } },
        ],
        url: `/wiki/${target.replaceAll(" ", "_")}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 2,
    };
  },
});

console.log(document.html); // <p>See <a href="/wiki/Some_Page">Some Page</a>.</p>\n
```

Count `consumed` from the start of `rest`, including any opening delimiter:
here `close` is the index of `]]` within `rest` _after_ `[[` was consumed by
`slice(2)`, so `close + 2` counts the target plus `]]`. Over-consuming eats
the text after the construct (e.g. a trailing period).

- `parseBlock([{ line, lineIndex, lines }])` returns `{ block, consumed }`.
- `parseInline([{ rest, index }])` returns `{ inline, consumed }`.
- A block hook consumes at least one line; `consumed: 0` is treated as one.
- An inline result with zero or missing `consumed` is ignored; `consumed`
  counts characters from the start of `rest`.
- Returned nodes require a complete `pos` object. Use a zero placeholder:
  Peisar replaces the span with the accurate span calculated from `consumed`.
- A Kramdown attribute line after a hook-produced block is applied
  automatically.

Hook registration reparses the original source immediately. Register all
hooks before depending on `ast`, `html`, `frontmatter`, or `astJson`. Nodes
produced by an inline hook are re-scanned from their end position, so a hook
can chain constructs back to back.

## Rust status

The crate exposes `PluginFactory`, `ParserHooks`, and
`md_to_ast_with_hooks` from `peisar::markdown::ast`. Implement
`try_parse_block` and/or `try_parse_inline`, add the plugin to a
`ParserHooks` registry, and pass that registry to `md_to_ast_with_hooks`.
`AstParser`, `BlockParseContext`, `InlineParseContext`, and `ParseHooks` are
deprecated compatibility names. The newer context type names are not
re-exported yet, so use the compatibility context aliases in Rust trait
signatures. Do not import from `peisar::markdown::ast::parsers`; that module
is private.
