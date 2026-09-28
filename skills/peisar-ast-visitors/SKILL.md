---
name: peisar-ast-visitors
description: Transform the Peisar Markdown AST after parsing using the visitor API in JavaScript (useVisitor) or Rust (AstVisitor trait + visit_document_mut). Use when rewriting, adding classes to, or removing Markdown nodes before rendering.
---

# Peisar AST Visitors

Visitors transform the parsed AST _after_ parsing, before rendering or reading `ast`/`astJson`. Each visitor callback receives a node and returns a control object that can keep, mutate, replace, remove, or insert around it.

## When to Use

- Adding classes/IDs to headings or blocks (e.g. for styling or anchor links)
- Rewriting or removing nodes (e.g. stripping images, changing heading levels)
- Counting or inspecting nodes during traversal
- Any post-parse AST mutation in JS or Rust

Not for introducing _new syntax_ — use parser hooks (`useParser` / `AstParser`) for that; visitors only see nodes the parsers produced.

## JavaScript (`useVisitor`)

The visitor is a plain object with optional `visitBlock` and/or `visitInline` callbacks. Each callback receives a **one-item node tuple** and returns a control object (or `undefined` to leave everything untouched).

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });

document.useVisitor({
  visitBlock([block]) {
    if (block.type === "Heading") {
      return {
        replaceWith: [
          { ...block, attrs: { ...block.attrs, classes: ["title"] } },
        ],
      };
    }
    return { recurse: true }; // keep node, visit its children
  },
});

console.log(document.html); // <h1 class="title">Hello</h1>
```

### Control object fields (all optional)

| Field                   | Effect                                                     |
| ----------------------- | ---------------------------------------------------------- |
| `recurse: true`         | Keep the node (with in-place edits) and visit its children |
| `replaceWith: [nodes]`  | Replace the current node with these nodes                  |
| `remove: true`          | Remove the node entirely                                   |
| `insertBefore: [nodes]` | Insert nodes before the current node                       |
| `insertAfter: [nodes]`  | Insert nodes after the current node                        |

Omitting all fields (or returning `undefined`) keeps the node and does **not** recurse.

### Rules

- Callbacks may mutate the node in place (it's the live node) and additionally return a control object.
- Replacement nodes are **not re-visited** — don't rely on a second pass over replacements (prevents infinite loops).
- `visitInline` mirrors `visitBlock` with the same fields, for inline nodes (`Text`, `Link`, `Image`, …).
- Visitors run at every read of `ast`, `html`, `astJson` (and `frontmatter` re-runs them too); registration order is preserved.
- Register visitors **before** first property access.

### Rewriting inline content

```js
document.useVisitor({
  visitInline([inline]) {
    if (inline.type === "Link" && inline.url.startsWith("http://")) {
      return {
        replaceWith: [
          { ...inline, url: inline.url.replace("http://", "https://") },
        ],
      };
    }
    return { recurse: true };
  },
});
```

## Rust (`AstVisitor` + `visit_document_mut`)

Implement the `AstVisitor` trait and run it with `visit_document_mut`:

```rust
use peisar::token::Block;
use peisar::visitor::{VisitControl, visit_document_mut};
use peisar::{AstOptions, AstVisitor, Document};

struct AddTitleClass;

impl AstVisitor for AddTitleClass {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        if let Block::Heading { attrs: slot, .. } = block {
            let mut attrs = slot.take().unwrap_or_default();
            let mut classes = attrs.classes.take().unwrap_or_default();
            classes.push("title".into());
            attrs.classes = Some(classes);
            *slot = Some(attrs);
            VisitControl::keep_and_recurse() // mutate in place, then recurse
        } else {
            VisitControl::default() // leave everything else untouched
        }
    }

    fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
        InlineVisitControl::default()
    }
}

let mut doc = Document::parse("# Hello\n", &AstOptions::default(), None);
visit_document_mut(&mut doc, &mut AddTitleClass);
```

### Control types

`VisitControl` (blocks) and `InlineVisitControl` (inlines) share the same fields and convenience constructors:

- `keep_and_recurse()` — keep (mutated) node, visit children
- `remove()` — drop the node
- `replace_with(nodes)` — swap in new nodes
- `insert_before` / `insert_after` fields for insertion
- `Default` — keep, don't recurse

`visit_block` runs pre-order; replacement nodes are not re-visited.

## Node Types Reference

Block types: `Heading`, `Paragraph`, `CodeBlock`, `BlockQuote`, `List`, `Table`, `ThematicBreak`, `HtmlBlock`, `LinkReferenceDefinition`, `Comment`.

Inline types: `Text`, `Emphasis`, `Code`, `HtmlInline`, `Strikethrough`, `HardBreak`, `SoftBreak`, `Image`, `Link`, `LinkReference`.

Note: in JS the `type` strings are the **Rust variant names** (PascalCase, e.g. `'HtmlBlock'`); in serialized `ast_json()` from Rust they are snake_case (`'html_block'`). Every node has `pos: { start, end }` with zero-based `{ line, column, offset }`.

## Common Mistakes

- **Forgetting the tuple destructuring** — JS callbacks receive `([block])`, not `(block)`.
- **Expecting `replaceWith` nodes to be visited** — they are inserted as-is, no re-visit.
- **Expecting `recurse` to be the default** — it is not; return `{ recurse: true }` (or `keep_and_recurse()`) explicitly when you want children visited.
- **Mutating without returning a control** — in-place mutations apply even when returning `undefined`, but children won't be visited unless you return `recurse`.
