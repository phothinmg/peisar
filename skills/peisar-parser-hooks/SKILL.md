---
name: peisar-parser-hooks
description: Extend the Peisar Markdown parser with custom syntax (directives, wikilinks, mentions) using parser hooks — useParser in JavaScript or the AstParser trait in Rust. Use when adding new Markdown syntax that must parse during parsing, not after.
---

# Peisar Custom Parser Hooks

Parser hooks let extensions parse custom syntax **during** parsing, before the built-in matchers — unlike visitors, which transform the finished AST afterwards. This means hooks can both introduce new syntax _and_ override built-in constructs (e.g. make `:::note` a container, or `[[Page]]` a link).

## When to Use

- Adding directive/container syntax (`:::note ... :::`)
- Adding wikilinks, mentions, or shortcodes (`[[Page]]`, `@user`, `:emoji:`)
- Overriding a built-in construct with custom semantics
- Any syntax that must be recognized _everywhere_ Markdown is parsed — headings, emphasis children, block quotes, list items, and table cells included

If you only need to _transform existing nodes_ after parsing, use visitors (`useVisitor` / `AstVisitor`) instead.

## How Hooks Work

- **Block hooks** run at the start of every block position, before the built-in block matchers.
- **Inline hooks** run at every character position of inline content, before the built-in inline matchers.
- Hooks run in **registration order**; the first hook to claim a position wins.
- Returning `undefined`/`None` declines, letting later hooks and the built-in parser try.

## JavaScript (`useParser`)

`useParser` takes an object with optional `parseBlock` and `parseInline` callbacks. **The hook must be registered before reading `ast`/`html`** — the document is parsed at construction, so registering a hook re-parses the original Markdown.

```js
const { Peisar } = require("peisar");

const document = new Peisar(":::note\nmy note\n:::\n\nSee [[Some Page]].", {
  fragment: true,
});

// Positions must be complete objects — the hook's own `pos` value is a
// placeholder; accurate spans are computed automatically from `consumed`.
const zero = { line: 0, column: 0, offset: 0 };

document.useParser({
  // Block hook: receives [{ line, lineIndex, lines }]
  parseBlock([{ line, lines }]) {
    if (!line.startsWith(":::")) return; // decline — let other parsers try
    const name = line.slice(3).trim();
    const closeIndex = lines.findIndex((l, i) => i > 0 && l.trim() === ":::");
    if (closeIndex === -1) return;
    return {
      block: {
        type: "HtmlBlock",
        html: `<div class="note" data-name="${name}"></div>`,
        pos: { start: zero, end: zero },
      },
      consumed: closeIndex + 1, // lines consumed (at least 1)
    };
  },

  // Inline hook: receives [{ rest, index }]
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return; // decline
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
    return {
      inline: {
        type: "Link",
        text: [
          { type: "Text", value: target, pos: { start: zero, end: zero } },
        ],
        url: `https://wiki.example.com/${target.replace(/ /g, "_")}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 4, // characters consumed
    };
  },
});

console.log(document.html);
// <div class="note" data-name="note"></div>
// <p>See <a href="https://wiki.example.com/Some_Page">Some Page</a>.</p>
```

### Context shapes

- `parseBlock` receives `{ line, lineIndex, lines }` — the current line, its 0-based index in the (sub-)document, and the remaining lines (`lines[0] === line`).
- `parseInline` receives `{ rest, index }` — the remaining inline text from the current position, and the 0-based character index.

### Return shapes and rules

| Hook          | Returns                               | Must satisfy                                       |
| ------------- | ------------------------------------- | -------------------------------------------------- |
| `parseBlock`  | `{ block, consumed }` or `undefined`  | `consumed` ≥ 1 line (`0` is treated as `1`)        |
| `parseInline` | `{ inline, consumed }` or `undefined` | `consumed` ≥ 1 char (`0` makes the result ignored) |

- The returned node needs a complete `pos` object (`{ start, end }` each `{ line, column, offset }`) — pass the `zero` placeholder; the engine overwrites it from `consumed`.
- A Kramdown `{:...}` attribute line following a hook-parsed block is applied automatically.
- Returning `undefined` (or omitting the node) declines the position.

## Rust (`AstParser` trait)

```rust
use peisar::token::Block;
use peisar::{AstParser, BlockParseContext, ParseHooks};

struct Directive;

impl AstParser for Directive {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        let name = ctx.line.trim().strip_prefix(":::")?;
        if name.is_empty() {
            return None; // decline — let other parsers try
        }
        let close = ctx.lines.iter().skip(1).position(|l| l.trim() == ":::")?;
        Some((
            Block::HtmlBlock {
                html: format!("<div class=\"directive\" data-name=\"{name}\">"),
                pos: Default::default(), // real span computed automatically
                attrs: None,
            },
            close + 2, // lines consumed (≥ 1)
        ))
    }
}
```

Run hooks through `ParseHooks` and `md_to_ast_with_hooks`:

```rust
use peisar::parsers::md_to_ast_with_hooks;

let hook = Directive;
let mut hooks = ParseHooks::empty();
hooks.push(&hook);
let doc = md_to_ast_with_hooks(":::note\n", &AstOptions::default(), None, &hooks);
```

The trait also offers `try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)>` with the same semantics as the JS `parseInline` hook. Both methods are optional — implement only the phase you need.

## Hook vs Visitor Decision Guide

| Need                                                  | Use                                                         |
| ----------------------------------------------------- | ----------------------------------------------------------- |
| Recognize new syntax that doesn't exist in Markdown   | Parser hook                                                 |
| Change how existing nodes render or are structured    | Visitor                                                     |
| Override a built-in construct (e.g. redefine `![]()`) | Parser hook                                                 |
| Add a class to every heading                          | Visitor                                                     |
| Syntax must nest inside emphasis, quotes, table cells | Parser hook (hooks run everywhere inline content is parsed) |
| Count or extract info without changing the tree       | Visitor                                                     |

## Common Mistakes

- **Reading `html` before `useParser`** — always register hooks first; they trigger a re-parse, but stale reads are confusing and wrong.
- **Block hook returning `consumed: 0`** — it is treated as `1`, likely eating only part of the construct.
- **Inline hook returning `consumed: 0`** — the result is ignored entirely.
- **Passing a missing/partial `pos`** — napi deserialization requires the complete `{ start: {...}, end: {...} }` shape; use the `zero` placeholder.
- **Forgetting to decline** — hooks run at _every_ position; a hook that never declines will consume the whole document.
- **Expecting hook-returned nodes to get Kramdown attrs manually** — attribute lines after the block are applied automatically by the engine.
