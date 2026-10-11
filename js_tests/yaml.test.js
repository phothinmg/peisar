const assert = require("node:assert/strict");
const test = require("node:test");

const { loadYaml, yamlParser } = require("./peisarloc");

test("Load YAML file", () => {
  const data = loadYaml("helpers/config.yml");
  assert.deepEqual(data, { name: "config", foo: { bar: 123, biz: "foo" } });
});

test("yamlParser parses nested objects, arrays, and scalar types", () => {
  const data = yamlParser(
    [
      "title: Hello world",
      "enabled: false",
      "count: 0",
      "empty:",
      "items:",
      "  - id: 1",
      "    tags:",
      "      - one",
      "      - two",
      "  - id: 2",
      "    tags: []",
      "",
    ].join("\n"),
  );

  assert.deepEqual(data, {
    title: "Hello world",
    enabled: false,
    count: 0,
    empty: null,
    items: [
      { id: 1, tags: ["one", "two"] },
      { id: 2, tags: [] },
    ],
  });
});

test("yamlParser supports top-level YAML sequences", () => {
  const data = yamlParser("- alpha\n- beta\n- gamma\n");

  assert.deepEqual(data, ["alpha", "beta", "gamma"]);
});

test("yamlParser throws a helpful error for invalid YAML", () => {
  assert.throws(
    () => yamlParser("bad: [1, 2\n"),
    /Failed to parse YAML: .*expected ',' or '\]'/,
  );
});

test("yamlParser rejects multi-document YAML input", () => {
  assert.throws(
    () => yamlParser("first: true\n---\nsecond: false\n"),
    /more than one document is not supported/,
  );
});
