const { Peisar } = require("./peisar");
const fs = require("fs");
const { myFactoryPlugin, myPlugin } = require("./plugin.js");

const text = fs.readFileSync("example.md", "utf8");

const doc = new Peisar(text, { fragment: true });

const usePlugin = function () {
  doc.use(myPlugin);
  return doc.html;
};
const useFactoryPlugin = function () {
  doc.use(myFactoryPlugin(0, 0, 0));
  return doc.html;
};

console.log(doc.frontmatter); // { title: 'example' }
console.log(usePlugin()); // <h2>Hello</h2>\n<p>See <a href="/wiki/Some_Page">Some Page</a>.</p>
console.log(useFactoryPlugin()); // <h2>Hello</h2>\n<p>See <a href="/wiki/Some_Page">Some Page</a>.</p>
console.log(doc.ast);
/*
{
  nodeType: 'root',
  pos: {
    start: { line: 0, column: 0, offset: 0 },
    end: { line: 2, column: 0, offset: 9 }
  },
  children: [
    { type: 'Heading', level: 2, children: [Array], pos: [Object] },
    { type: 'Paragraph', children: [Array], pos: [Object] }
  ],
  linkReferences: []
}
 */
console.log(doc.astJson); // {"type":"root","file_name":null,"pos":{"start":{"line":0,"column":0,"offset":0},"end":{"line":2,"column":0,"offset":9}},"children":[{"type":"heading","level":2,"children":[{"type":"text","value":"Hello","pos":{"start":{"line":0,"column":0,"offset":0},"end":{"line":0,"column":5,"offset":5}}}],"pos":{"start":{"line":0,"column":0,"offset":0},"end":{"line":1,"column":0,"offset":8}}},{"type":"paragraph","children":[{"type":"text","value":"See ","pos":{"start":{"line":0,"column":0,"offset":0},"end":{"line":0,"column":4,"offset":4}}},{"type":"link","text":[{"type":"text","value":"Some Page","pos":{"start":{"line":0,"column":0,"offset":0},"end":{"line":0,"column":0,"offset":0}}}],"url":"/wiki/Some_Page","title":null,"autolink":false,"pos":{"start":{"line":0,"column":4,"offset":4},"end":{"line":0,"column":17,"offset":17}}},{"type":"text","value":".","pos":{"start":{"line":0,"column":17,"offset":17},"end":{"line":0,"column":18,"offset":18}}}],"pos":{"start":{"line":2,"column":0,"offset":9},"end":{"line":2,"column":0,"offset":9}}}]}
