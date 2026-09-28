const assert = require('node:assert/strict')
const test = require('node:test')

const { Peisar } = require('../index.js')

test('parses Markdown into an AST and renders a fragment', () => {
  const document = new Peisar('# Hello **world**', { fragment: true })

  assert.equal(document.html, '<h1>Hello <strong>world</strong></h1>\n')
  assert.deepEqual(document.ast.children[0], {
    type: 'Heading',
    level: 1,
    children: [
      {
        type: 'Text',
        value: 'Hello ',
        pos: {
          start: { line: 0, column: 0, offset: 0 },
          end: { line: 0, column: 6, offset: 6 },
        },
      },
      {
        type: 'Emphasis',
        level: 1,
        children: [
          {
            type: 'Text',
            value: 'world',
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
  })
})

test('extracts YAML front matter without rendering it', () => {
  const document = new Peisar('---\ntitle: Hello\ntags:\n  - one\n---\n\n# Hello', {
    fragment: true,
  })

  assert.deepEqual(document.frontmatter, { title: 'Hello', tags: ['one'] })
  assert.equal(document.html, '<h1>Hello</h1>\n')
})

test('renders GFM task lists by default', () => {
  const document = new Peisar('- [x] Done\n- [ ] Todo', { fragment: true })

  assert.match(document.html, /data-checked="true"/)
  assert.match(document.html, /checked disabled/)
  assert.match(document.html, /task-list-item-checkbox/)
})

test('applies JavaScript AST visitors before rendering', () => {
  const document = new Peisar('# Hello', { fragment: true })

  document.useVisitor({
    visitBlock([block]) {
      if (block.type === 'Heading') {
        return { replaceWith: [{ ...block, level: 2 }] }
      }
      return { recurse: true }
    },
  })

  assert.equal(document.html, '<h2>Hello</h2>\n')
  assert.equal(document.ast.children[0].level, 2)
})
