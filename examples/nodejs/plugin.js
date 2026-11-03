const zero = { line: 0, column: 0, offset: 0 };
/**
 * Example to create plugin for Peisar
 *
 * @type {import("./peisar").Plugin}
 */
exports.myPlugin = {
  // Visitor transform existing AST nodes after parsing.
  // They cannot recognize new syntax
  visitBlock([block]) {
    if (block.type === "Heading") {
      return { replaceWith: [{ ...block, level: 2 }] };
    }
    return { recurse: true };
  },
  // Parser hook for new syntax
  parseInline([{ rest }]) {
    if (!rest.startsWith("[[")) return;
    const close = rest.indexOf("]]");
    if (close === -1) return;
    const target = rest.slice(2, close);
    return {
      inline: {
        type: "Link",
        text: [
          { type: "Text", value: target, pos: { start: zero, end: zero } },
        ],
        url: `/wiki/${target.replaceAll(" ", "_")}`,
        autolink: false,
        pos: { start: zero, end: zero },
      },
      consumed: close + 2,
    };
  },
};

/**
 *
 * Example to create plugin as factory function
 *
 * @param {number} line
 * @param {number} column
 * @param {number} offset
 *
 * @returns {import("./peisar").Plugin}
 */
exports.myFactoryPlugin = function (line, column, offset) {
  const z = { line, column, offset };
  return {
    // Visitor transform existing AST nodes after parsing.
    // They cannot recognize new syntax
    visitBlock([block]) {
      if (block.type === "Heading") {
        return { replaceWith: [{ ...block, level: 2 }] };
      }
      return { recurse: true };
    },
    // Parser hook for new syntax
    parseInline([{ rest }]) {
      if (!rest.startsWith("[[")) return;
      const close = rest.indexOf("]]");
      if (close === -1) return;
      const target = rest.slice(2, close);
      return {
        inline: {
          type: "Link",
          text: [
            { type: "Text", value: target, pos: { start: zero, end: zero } },
          ],
          url: `/wiki/${target.replaceAll(" ", "_")}`,
          autolink: false,
          pos: { start: z, end: z },
        },
        consumed: close + 2,
      };
    },
  };
};
