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

Register a plain object with `useParser`. Both callbacks receive a one-item
tuple.

```js
const { Peisar } = require("peisar");

const document = new Peisar("See [[Some Page]].", { fragment: true });
const zero = { line: 0, column: 0, offset: 0 };

document.useParser({
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return;
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
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
```

- `parseBlock([{ line, lineIndex, lines }])` returns `{ block, consumed }`.
- `parseInline([{ rest, index }])` returns `{ inline, consumed }`.
- A block hook consumes at least one line; `consumed: 0` is treated as one.
- An inline result with zero or missing `consumed` is ignored.
- Returned nodes require a complete `pos` object. Use a zero placeholder:
  Peisar replaces the span with the accurate span calculated from `consumed`.
- A Kramdown attribute line after a hook-produced block is applied
  automatically.

Hook registration reparses the original source immediately. Register all
hooks before depending on `ast`, `html`, `frontmatter`, or `astJson`.

## Rust status

The crate publicly exposes `AstParser`, `BlockParseContext`,
`InlineParseContext`, and `ParseHooks` from `peisar::markdown::ast`. They
define the same hook contract used by the implementation:
`try_parse_block` returns `Option<(Block, usize)>` and `try_parse_inline`
returns `Option<(Inline, usize)>`.

The hook-enabled parser entry point is currently private to the crate, so
external Rust consumers cannot yet apply a `ParseHooks` registry to
`Document::parse`. Do not suggest imports from
`peisar::markdown::ast::parsers`: that module is not public. For a working
extension path today, use Node.js `Peisar#useParser`; use the exposed Rust hook
types only when modifying Peisar itself.
