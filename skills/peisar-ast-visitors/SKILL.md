---
name: peisar-ast-visitors
description: Transform Peisar's parsed Markdown AST in Node.js or Rust. Use when rewriting, inserting, removing, or inspecting nodes after parsing and before rendering.
---

# Peisar AST Visitors

Visitors transform existing AST nodes after parsing. They cannot recognize new
syntax; use the parser-hook API for that.

## Node.js

Register a visitor through the `Peisar` instance. Both callbacks receive a
**one-item tuple**, not a node directly.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });
document.useVisitor({
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

Implement `AstVisitor` and call `visit_document_mut`:

```rust
use peisar::markdown::ast::{AstOptions, AstVisitor, Document};
use peisar::markdown::ast::tokens::token::{Block, Inline};
use peisar::markdown::ast::visitor::{InlineVisitControl, VisitControl, visit_document_mut};

struct PromoteHeadings;

impl AstVisitor for PromoteHeadings {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        if let Block::Heading { level, .. } = block {
            *level = 2;
        }
        VisitControl::keep_and_recurse()
    }

    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

let mut document = Document::parse("# Hello\n", &AstOptions::default(), None);
visit_document_mut(&mut document, &mut PromoteHeadings);
```

Use `VisitControl::keep_and_recurse()`, `VisitControl::remove()`, or
`VisitControl::replace_with(nodes)` for block nodes. `InlineVisitControl`
offers the equivalent inline helpers. Its public fields support insertion too.

Do not use obsolete root paths such as `peisar::visitor` or `peisar::token`;
the visitor API is under `peisar::markdown::ast::visitor` and nodes are under
`peisar::markdown::ast::tokens::token`.
