---
name: peisar-ast-visitors
description: Transform Peisar's parsed Markdown AST in Node.js or Rust. Use when rewriting, inserting, removing, or inspecting nodes after parsing and before rendering.
---

# Peisar AST Visitors

Visitors transform existing AST nodes after parsing. They cannot recognize new
syntax; use the parser-hook API for that.

## Node.js

Register a visitor with `usePlugin()` on the `Peisar` instance. Both
callbacks receive a **one-item tuple**, not a node directly. `useVisitor()`
is a deprecated compatibility alias.

```js
const { Peisar } = require("peisar");

const document = new Peisar("# Hello", { fragment: true });
document.usePlugin({
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

`AstVisitor` is deprecated in favor of `PluginFactory`, but the current
public Rust AST module does not re-export the visitor control types needed to
implement Rust visitor callbacks. Use the Node.js `usePlugin()` interface
for supported AST rewriting; Rust consumers can use the parser API described
in the `peisar-parser-hooks` skill.

Rust node types remain under
`peisar::markdown::ast::tokens::token::{Block, Inline}`. Do not use
obsolete root paths such as `peisar::visitor` or `peisar::token`.
