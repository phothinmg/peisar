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

Register a plain object with `Peisar.use()`. Parser callbacks receive a
one-item tuple; visitor callbacks may also be included in the same plugin.

```js
const { Peisar } = require("peisar");

const document = new Peisar("See [[Some Page]].", { fragment: true });
const zero = { line: 0, column: 0, offset: 0 };

document.use({
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

## Rust

No Cargo features are required for Rust parser hooks. Implement
`PluginFactory::try_parse_block` / `try_parse_inline` (from
`peisar::plugin`), register the hook in a `ParserHooks` registry, and parse
with `md_to_ast_with_hooks` (re-exported from `peisar::ast`).

```rust
use peisar::ast::{AstOptions, Document, md_to_ast_with_hooks};
use peisar::plugin::{BlockParserContext, ParserHooks, PluginFactory};

struct BangBreak;

impl PluginFactory for BangBreak {
    fn try_parse_block(&self, ctx: &BlockParserContext) -> Option<(peisar::ast::Block, usize)> {
        if ctx.line.trim() != "!" {
            return None; // decline
        }
        let block = peisar::ast::Block::ThematicBreak {
            pos: Default::default(),
        };
        Some((block, 1))
    }
}

let hooks = ParserHooks::empty().with(&BangBreak);
let document: Document =
    md_to_ast_with_hooks("!\n", &AstOptions::default(), None, &hooks);
```

Hooks run in registration order; the first hook that returns `Some`
claims the position. Block hooks consume at least one line; inline hooks
return the node and the number of characters consumed from `rest`.
The high-level `Peisar` class registers plugins on the JS side through
`use()` only — use `md_to_ast_with_hooks` for Rust-side hooks. See the
[Rust API reference](../../docs/rust/apis.md) for the current public API.
