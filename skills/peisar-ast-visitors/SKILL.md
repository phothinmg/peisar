---
name: peisar-ast-visitors
description: Transform Peisar's parsed Markdown AST in Node.js or Rust. Use when rewriting, inserting, removing, or inspecting nodes after parsing and before rendering.
---

# Peisar AST Visitors

Visitors transform existing AST nodes after parsing. They cannot recognize new
syntax; use the parser-hook API for that.

## Node.js

Register a visitor with `use()` on the `Peisar` instance. Both callbacks
receive a **one-item tuple**, not a node directly.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });
document.use({
  visitBlock([block]) {
    if (block.type === "Heading") {
      return { replaceWith: [{ ...block, level: 2 }] };
    }
    return { recurse: true };
  },
});

console.log(document.html); // <h2>Hello</h2>\n
```

`visitBlock` and `visitInline` may return these optional camel-case fields:

| Field                   | Effect                               |
| ----------------------- | ------------------------------------ |
| `recurse: true`         | Keep the node and visit its children |
| `replaceWith: [nodes]`  | Replace the current node             |
| `remove: true`          | Remove the current node              |
| `insertBefore: [nodes]` | Insert siblings before it            |
| `insertAfter: [nodes]`  | Insert siblings after it             |

An omitted return value keeps the node but does not recurse. `recurse`
defaults to `false` — return `{ recurse: true }` (JS) or use
`VisitControl::keep_and_recurse()` (Rust) to visit children. Callbacks may
also mutate the supplied node. Traversal is pre-order, and replacement nodes
are not re-visited. Visitors are applied each time `ast`, `astJson`, `html`,
or `frontmatter` is read, so write visitors that are safe to run repeatedly.

Node `type` values are PascalCase in the `ast` object: block nodes include
`Heading`, `Paragraph`, `CodeBlock`, `BlockQuote`, `List`, `Table`,
`ThematicBreak`, `HtmlBlock`, `LinkReferenceDefinition`, and `Comment`;
inline nodes include `Text`, `Emphasis`, `Code`, `HtmlInline`,
`Strikethrough`, `HardBreak`, `SoftBreak`, `Image`, `Link`, and
`LinkReference`. (The `astJson` string uses snake_case types instead — see
the peisar-parse-markdown skill.)

## Rust

Enable the `ast` and `plugin` Cargo features. AST types are re-exported
from `peisar::ast`; implement `PluginFactory` (from `peisar::plugin`) and
run it over a parsed document with `visit_document_mut`.

```rust
use peisar::ast::{AstOptions, Block, Document};
use peisar::plugin::{InlineVisitControl, PluginFactory, VisitControl, visit_document_mut};

struct DemoteHeadings;

impl PluginFactory for DemoteHeadings {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        if let Block::Heading { level, .. } = block {
            *level = (*level + 1).min(6);
        }
        VisitControl::keep_and_recurse()
    }
    fn visit_inline(&mut self, _inline: &mut peisar::ast::Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

let document = Document::parse("# Hello\n", &AstOptions::default(), None);
let mut document = document;
visit_document_mut(&mut document, &mut DemoteHeadings);
```

`VisitControl` mirrors the JS fields: `insert_before`, `insert_after`,
`replace_with`, `remove`, and `recurse`. The high-level `Peisar` class
does not accept Rust plugins — it registers plugins on the JS side through
`use()` only. Use `Document::parse` plus `visit_document_mut` for Rust-side
transformations, then render with `render_document_html` (the `html`
feature). See the [Rust API reference](../../docs/rust/apis.md) for the
current dependency interfaces and feature requirements.
