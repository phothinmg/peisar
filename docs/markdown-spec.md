# Peisar Markdown Language Specification

Version 0.3.0 — describes exactly what `peisar` parses and renders with
**default options** (`gfm: true`, `kramdown: true`). Every example in this
document was verified against the parser itself.

Peisar implements a practical CommonMark-style core plus GitHub Flavored
Markdown (GFM) extensions and Kramdown block attributes. Where peisar differs
from strict CommonMark, the difference is called out explicitly.

```js
const { Peisar } = require("peisar");
const doc = new Peisar("# Hello **world**", { fragment: true });
doc.html;      // <h1>Hello <strong>world</strong></h1>\n
doc.ast;       // JavaScript AST object
doc.frontmatter; // YAML object or null
```

---

## 1. Document structure

A document is a sequence of **block** nodes. Blocks contain **inline** content
(or nested blocks). Blank lines separate blocks; they carry no meaning
otherwise. Both `\n` and `\r\n` line endings are accepted.

### 1.1 YAML front matter

An optional YAML block at the very top of the document:

```markdown
---
title: Guide
tags:
  - docs
---

# The body
```

Rules:

- The opening `---` must be the first non-whitespace line and must be followed
  immediately by a newline (`---\n` or `---\r\n`). A trailing space after
  `---` disables front matter.
- The block closes at the next line that starts with `---`.
- Everything after the closing marker is the Markdown body (leading blank
  lines are stripped).
- YAML is deserialized with full serde_yaml semantics: nested maps, lists,
  scalars, quotes.
- An **unterminated** block is not front matter; the text is parsed as normal
  Markdown.
- **Invalid YAML does not throw** — the front matter is discarded and the whole
  input is parsed as Markdown body.
- `document.frontmatter` returns the deserialized YAML object, or `null`.

---

## 2. Blocks

### 2.1 ATX headings

```markdown
# H1
## H2
###### H6
```

- 1–6 `#` characters, optionally indented, followed by a space or end of line.
- Seven or more `#`, or a missing space (`#Hello`), is a paragraph.
- Inline markup inside headings is parsed (`# a *b*` → `<h1>a <em>b</em></h1>`).
- **Difference from CommonMark:** trailing closing hashes are *not* stripped
  (`## Title ##` keeps the ` ##` as text).
- **Setext headings are not supported.** `Title\n===` is a paragraph;
  `Title\n---` is a paragraph followed by a thematic break.

### 2.2 Paragraphs

Consecutive non-blank lines form one paragraph. A paragraph ends at a blank
line or the start of another block construct (heading, fence, thematic break,
list marker, blockquote, HTML block, table, link reference definition, or a
Kramdown attribute line). Lines within a paragraph are joined by soft breaks.

### 2.3 Thematic breaks

```markdown
---
***
___
- - -
```

Three or more of the *same* character (`-`, `*`, or `_`), optionally spaced and
indented, on a line by itself. Renders `<hr>`.

### 2.4 Fenced code blocks

```markdown
```rust
fn main() {}
```
```

- Fence: 3 or more `` ` `` or `~` characters. The closing fence must use the
  same character and be at least as long as the opener (it may be indented).
- The info string after the opening fence is kept **whole** as the language
  hint: `` ```rust extra `` sets `class="language-rust extra"`.
- Content is captured verbatim (no dedent, no escape processing) until the
  closing fence or end of document. `&`, `<`, `>` are HTML-escaped on render.
- With a language: `<pre><code class="language-rust">…</code></pre>`.
- Without: `<pre><code>…</code></pre>`.
- A fence inside a *longer* fence is content (open with `````` to include ``` ).

### 2.5 Indented code blocks

Four spaces or one tab of indentation:

```markdown
    indented code
    second line
```

Consume consecutive lines indented by 4 spaces / 1 tab. A single blank line
inside is kept if the next line is still indented; otherwise the block ends.
No language hint. Renders like a fenced block without a language.

### 2.6 Block quotes

```markdown
> Quoted text.
> More.

> Lazy continuation
still part of the quote
```

- A line starting with `>` (optionally indented) opens a quote; one optional
  space after `>` is stripped from each line.
- Lines without `>` continue the quote (lazy continuation) until a blank line.
- Quotes nest (`> > …`) and their content is re-parsed as a full sub-document
  (headings, lists, tables all work inside).
- Renders `<blockquote>…</blockquote>`.

### 2.7 Lists

```markdown
- unordered
* also unordered
+ also unordered

1. ordered
2) ordered with paren
```

- Markers: `-`, `*`, `+` followed by a space; or 1–9 digits followed by `.`
  or `)` and a space (a bare `.`/`)` at end of line also counts).
- A tab after the marker is **not** supported (`-\tb` is a paragraph).
- Continuation lines indented to the item's content column join the item;
  non-indented "lazy" lines also join unless they start a new block construct.
- A blank line between items makes a **loose list**: item content is wrapped in
  `<p>`. Without blank lines the list is **tight** and a single-paragraph item
  renders inline (no `<p>`), matching CommonMark convention.
- Ordered lists always render `<ol>` starting at 1; the first number is not
  preserved (no `start` attribute).
- Items may contain any nested blocks (paragraphs, code, nested lists, quotes).

### 2.8 Task lists (GFM)

```markdown
- [ ] unchecked
- [x] checked
- [X] also checked
1. [x] ordered task
```

A `[ ]`, `[x]`, or `[X]` marker directly after a list marker turns the item
into a task. The marker is stripped from the text. Renders:

```html
<li class="task-list-item" data-checked="true">
  <input type="checkbox" class="task-list-item-checkbox" checked disabled> done
</li>
```

(`data-checked="true"` and `checked` appear only for checked items.)

### 2.9 HTML blocks

A line starting with `<` followed by a letter, `/`, or `!` begins a raw HTML
block. All lines until the next blank line are captured **verbatim** and
emitted unescaped. `<!DOCTYPE html>` is passed through the same way.

### 2.10 HTML comments

A line starting with `<!--` opens a comment block, captured until the line
containing `-->` (or the next blank line if never closed). The AST stores the
comment text without the `<!--`/`-->` markers; rendering re-emits
`<!-- value -->`.

### 2.11 Tables (GFM)

```markdown
| Name | Value |
| :--- | ----: |
| a    | 1     |
```

- A table starts where a line containing `|` is followed by a delimiter row
  whose cells contain only `-` and `:`, with at least one `-` per cell.
- Leading/trailing pipes are optional; cells are split on `|` and trimmed.
- Alignment per column: `:--` left, `--:` right, `:--:` center, `--` default.
- Header/delimiter/row cell counts may differ; extra cells are kept, missing
  ones absent.
- Body rows continue until a blank line or a line without `|`.
- Inline markup is parsed inside cells.
- **Difference from GFM:** escaped pipes (`\|`) are *not* supported — `\|` ends
  the cell and keeps the backslash.
- Renders `<table><thead>…<tbody>…`; alignment emits
  `style="text-align: center"` / `"right"` (left/default emit nothing).

### 2.12 Link reference definitions

```markdown
[google]: https://google.com "Google"
[wiki]: <https://w.org> (Wiki)
```

- Form: `[label]: url` with optional title in `"…"`, `'…'`, or `(…)`.
- The URL may be wrapped in `<…>` (kept verbatim, not unwrapped) or bare
  (up to first whitespace).
- Definitions are collected in a document-wide pre-pass, so they may appear
  **after** their uses and inside any container.
- They render nothing; they appear in the AST both as block nodes and in the
  root's `linkReferences` array.
- Labels are normalized: trimmed, internal whitespace collapsed, lowercased.

### 2.13 Kramdown block attributes

```markdown
# Heading
{: #title .big data-role="note"}

Paragraph with attributes.
{: .lead}

```code```
{: .cb}
```

- An attribute line is `{…}` on its own line, placed **after** the block it
  annotates. Blank lines between the block and the `{…}` line are allowed
  (and consumed).
- Syntax inside the braces (a `:` after `{` is optional, i.e. `{:` and `{`
  both work):
  - `#id` → HTML `id`
  - `.class` → `class` (repeatable, joined with spaces)
  - `key="value"` / `key='value'` → arbitrary attributes
  - `bare` → boolean attribute rendered with an empty value (`bare=""`)
  - `{`/`}` nesting and `}` inside quoted values are handled.
- Supported on: headings, paragraphs, code blocks, block quotes, lists, HTML
  blocks, tables, and parser-hook blocks. **Not** thematic breaks (an attr
  line after `---` becomes a paragraph carrying the attributes).
- Inside a block quote, an attribute line **ends the quote** and applies to
  it.
- Rendering order: `id`, then `class`, then key/value pairs; `& " < >` in
  values are HTML-escaped.

---

## 3. Inlines

### 3.1 Emphasis

```markdown
*italic*   **bold**
_italic_   __bold__
```

- `*` and `_` are interchangeable. A run of 2+ is tried as **bold** first;
  if no closer is found, a single-marker **italic** is tried.
- Closing: any later run of the same character with length ≥ the opener.
- **Intraword emphasis works:** `a*b*c` → `a<em>b</em>c`, and `_snake_case_`
  becomes `<em>snake</em>case_` (unlike CommonMark).
- Nesting: `**a *b* c**` → bold containing italic.
- `***both***` parses as bold with a stray `*` on each side.
- An unclosed run may still match shorter: `**unclosed` yields
  `<em></em>unclosed`.
- Renders `<em>` / `<strong>`.

### 3.2 Inline code

```markdown
`code`        ``code with ` inside``
```

- An opening backtick run matches the next run of the *same length*; the
  content between is kept verbatim (spaces preserved, nothing escaped in the
  AST; `& < > "` `'` are escaped on render).
- No language, no backslash processing inside.

### 3.3 Strikethrough (GFM)

```markdown
~~deleted~~
```

Renders `<del>deleted</del>`. Content is parsed as inline markup.

### 3.4 Links

```markdown
[text](https://example.com)
[text](https://example.com "Title")
[text](url 'single-quoted title')
[nested [brackets]](url)
```

- Square brackets nest by depth; parentheses nest by depth.
- An optional title follows the URL separated by whitespace, in `"` or `'`.
  Parens after the URL are treated as part of the URL, not a title.
- The link text is parsed as inline markup (`[**b**](u)` keeps the bold).
- `<…>`-wrapped destinations are kept **verbatim** (angle brackets are not
  stripped and end up in the `href`).
- Renders `<a href="…" title="…">…</a>` with `& " < >` escaped in `href`.

### 3.5 Images

```markdown
![alt text](image.png "Title")
```

Alt text is a raw string (not parsed as markup). Renders
`<img src="…" alt="…" title="…">`.

### 3.6 Reference links

```markdown
[text][label]     full
[label][]         collapsed
[label]           shortcut

[label]: https://example.com "Title"
```

- All three forms resolve against the document-wide reference map
  (definitions may appear anywhere, including after the use).
- Labels match case-insensitively with collapsed whitespace.
- An unresolved reference stays literal text: `[missing][nope]`.
- The AST node is `LinkReference` with the resolved `url`/`title`; rendering is
  identical to an inline link.

### 3.7 Autolinks (GFM)

```markdown
https://example.com
http://example.com/path
www.example.com
```

- Bare `http://`, `https://`, and `www.` URLs become links — **lowercase
  schemes only** (`WWW.` does not link).
- `www.` links get `https://` prepended in the `href`; the visible text shows
  the full `https://www.…` URL.
- Matching requires only that the position starts with `h`/`w` — there is no
  word-boundary check (`xhttps://a.com` links).
- The URL ends at whitespace, `<`, `>`, or trailing punctuation
  (`. , ; : ! ? )`) at the end of the run.
- Angle-bracket autolinks (`<https://…>`) are **not** supported and stay
  literal.

### 3.8 Line breaks

| Source | AST | HTML |
|---|---|---|
| `a  \nb` (2+ trailing spaces) | `HardBreak` | `a<br>\nb` |
| `a\\\nb` (backslash) | `HardBreak` | `a<br>\nb` |
| `a\nb` (plain newline) | `SoftBreak` | `a\nb` |

Trailing spaces / a trailing backslash with no following newline are kept as
literal text.

### 3.9 Inline HTML

`<tag …>`, `</tag>`, and `<!-- … -->` are captured as raw `HtmlInline` nodes
and emitted verbatim. Inside link text, inline HTML is preserved.

### 3.10 Text and escaping — important differences

- **No backslash escapes.** `\*` does not prevent emphasis; the backslash is
  kept as literal text and the `*` still parses. `\`` and `\n`-style escapes
  do not exist. The only backslash behavior is the hard break (3.8).
- **No character-entity decoding.** `&amp;` stays the six characters `&amp;`
  in the AST (rendered as `&amp;amp;`).
- Plain text is HTML-escaped on render: `&` `&amp;`, `<` `&lt;`, `>`
  `&gt;`, `"` `&quot;`, `'` `&#39;`.

---

## 4. Options

All options are optional; defaults shown:

| Option | Default | Effect |
|---|---|---|
| `gfm` | `true` | Tables, strikethrough, task lists, autolinks |
| `kramdown` | `true` | `{…}` attribute lines |
| `fragment` | `false` | `true`: body only; `false`: full HTML document |
| `fileName` | `null` | Attached to the AST root |
| `charset` | `true` | `<meta charset>` in full-document mode |
| `viewport` | `true` | `<meta name="viewport">` in full-document mode |
| `title` | `null` | `<title>` |
| `bodyClass` | `null` | `<body class="…">` |
| `style` | `null` | Inline CSS in a `<style>` tag |

With `gfm: false`: table syntax becomes paragraph text, `~~x~~` stays literal,
`[ ]` markers stay text, bare URLs are not linked. With `kramdown: false`,
`{…}` lines are ordinary paragraph text.

Full-document output shape:

```html
<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>…</title>
<style>…</style>
</head>
<body class="…">
…blocks…
</body>
</html>
```

---

## 5. AST shape

- Root: `{ nodeType: "root", fileName?, pos, children: Block[], linkReferences[] }`.
- Every node carries `pos: { start, end }` — a half-open span with zero-based
  `line`, `column` (characters), and `offset` (bytes).
- **Spans of nodes inside containers** (block quotes, list items) are relative
  to that container's sub-document, not the original file.
- In the JavaScript `ast` property, `type` values are PascalCase (`Heading`,
 `Paragraph`, `Link`); `astJson` mirrors the Rust serialization with
  snake_case types (`heading`, `paragraph`, `link`).

Node kinds — blocks: `Heading` (level 1–6), `Paragraph`, `CodeBlock`
(lang/code), `BlockQuote`, `List` (ordered/items), `ThematicBreak`, `HtmlBlock`,
`Table` (header/rows/alignments), `LinkReferenceDefinition` (label/url/title),
`Comment`; inlines: `Text`, `Emphasis` (level `"Italic"` / `"Bold"`), `Code`,
`HtmlInline`, `Strikethrough`, `HardBreak`, `SoftBreak`, `Image`
(alt/url/title), `Link` (text/url/title/autolink), `LinkReference`
(text/label/url/title).

Task state: `"Unchecked"` / `"Checked"` (type `PeisarTaskState`). Table
alignment: `"Default"` / `"Left"` / `"Center"` / `"Right"` (type
`PeisarTableCellAlignment`, arrays as `PeisarTableCellAlignments`). Emphasis
level type: `PeisarEmphasisLevel`. All four type unions are exported from the
package root.

---

## 6. Extensibility (not default syntax)

Custom syntax is added per-document, not by the language itself:

- `useParser({ parseBlock, parseInline })` — hooks that run **before** the
  built-in matchers; block hooks receive `{ line, lineIndex, lines }` and
  return `{ block, consumed }`; inline hooks receive `{ rest, index }` and
  return `{ inline, consumed }`. Registering a hook re-parses the source.
- `useVisitor({ visitBlock, visitInline })` — post-parse transforms with
  control objects (`insertBefore`, `insertAfter`, `replaceWith`, `remove`,
  `recurse`).

These do not change the default language described above.