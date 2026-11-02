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

Rust AST types are re-exported from `peisar::ast` with the `ast` Cargo
feature, and plugin interfaces are available from `peisar::plugin` with the
`plugin` feature. See the Rust API reference for the current dependency
interfaces and feature requirements.
