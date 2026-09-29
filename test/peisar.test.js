const assert = require("node:assert/strict");
const test = require("node:test");

const { Peisar } = require("../index.js");

test("parses Markdown into an AST and renders a fragment", () => {
  const document = new Peisar("# Hello **world**", { fragment: true });

  assert.equal(document.html, "<h1>Hello <strong>world</strong></h1>\n");
  assert.deepEqual(document.ast.children[0], {
    type: "Heading",
    level: 1,
    children: [
      {
        type: "Text",
        value: "Hello ",
        pos: {
          start: { line: 0, column: 0, offset: 0 },
          end: { line: 0, column: 6, offset: 6 },
        },
      },
      {
        type: "Emphasis",
        level: 1,
        children: [
          {
            type: "Text",
            value: "world",
            pos: {
              start: { line: 0, column: 0, offset: 0 },
              end: { line: 0, column: 5, offset: 5 },
            },
          },
        ],
        pos: {
          start: { line: 0, column: 6, offset: 6 },
          end: { line: 0, column: 15, offset: 15 },
        },
      },
    ],
    pos: {
      start: { line: 0, column: 0, offset: 0 },
      end: { line: 0, column: 0, offset: 0 },
    },
  });
});

test("extracts YAML front matter without rendering it", () => {
  const document = new Peisar(
    "---\ntitle: Hello\ntags:\n  - one\n---\n\n# Hello",
    {
      fragment: true,
    },
  );

  assert.deepEqual(document.frontmatter, { title: "Hello", tags: ["one"] });
  assert.equal(document.html, "<h1>Hello</h1>\n");
});

test("renders GFM task lists by default", () => {
  const document = new Peisar("- [x] Done\n- [ ] Todo", { fragment: true });

  assert.match(document.html, /data-checked="true"/);
  assert.match(document.html, /checked disabled/);
  assert.match(document.html, /task-list-item-checkbox/);
});

test("applies JavaScript AST visitors before rendering", () => {
  const document = new Peisar("# Hello", { fragment: true });

  document.useVisitor({
    visitBlock([block]) {
      if (block.type === "Heading") {
        return { replaceWith: [{ ...block, level: 2 }] };
      }
      return { recurse: true };
    },
  });

  assert.equal(document.html, "<h2>Hello</h2>\n");
  assert.equal(document.ast.children[0].level, 2);
});

test("custom block parser hooks run before built-in parsers", () => {
  const document = new Peisar(":::note\nsome content\n:::\n\n# After", {
    fragment: true,
  });
  const zero = { line: 0, column: 0, offset: 0 };

  document.useParser({
    parseBlock([{ line, lines }]) {
      if (!line.startsWith(":::")) return; // decline
      const name = line.slice(3).trim();
      const closeIndex = lines.findIndex((l, i) => i > 0 && l.trim() === ":::");
      if (closeIndex === -1) return;
      return {
        block: {
          type: "HtmlBlock",
          html: `<div class="note" data-name="${name}"></div>`,
          pos: { start: zero, end: zero },
        },
        consumed: closeIndex + 1,
      };
    },
  });

  assert.equal(
    document.html,
    '<div class="note" data-name="note"></div>\n<h1>After</h1>\n',
  );
  assert.equal(
    document.ast.children[0].html,
    '<div class="note" data-name="note"></div>',
  );
});

test("custom inline parser hooks parse wikilinks", () => {
  const document = new Peisar("See [[Some Page]] here.", { fragment: true });
  const zero = { line: 0, column: 0, offset: 0 };

  document.useParser({
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
        consumed: close + 4,
      };
    },
  });

  assert.match(document.html, /href="https:\/\/wiki\.example\.com\/Some_Page"/);
  assert.match(document.html, />Some Page<\/a>/);
});

test("parser hooks and visitors compose", () => {
  const document = new Peisar("!\n\na lone bang becomes a break", {
    fragment: true,
  });
  const zero = { line: 0, column: 0, offset: 0 };

  document.useParser({
    parseBlock([{ line }]) {
      if (line.trim() !== "!") return; // decline
      return {
        block: { type: "ThematicBreak", pos: { start: zero, end: zero } },
        consumed: 1,
      };
    },
  });

  document.useVisitor({
    visitBlock([block]) {
      if (block.type === "ThematicBreak") {
        return {
          replaceWith: [
            {
              type: "Paragraph",
              children: [
                {
                  type: "Text",
                  value: "replaced",
                  pos: { start: zero, end: zero },
                },
              ],
              pos: { start: zero, end: zero },
            },
          ],
        };
      }
      return { recurse: true };
    },
  });

  assert.equal(
    document.html,
    "<p>replaced</p>\n<p>a lone bang becomes a break</p>\n",
  );
});
