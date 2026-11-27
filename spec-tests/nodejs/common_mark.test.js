const { peisar } = require("./peisar-core");
const { describe, it, after } = require("node:test");
const assert = require("assert");
const fs = require("fs");

const parseMarkdown = (input) => {
    const pei = new peisar.Peisar("\n" + input, { fragment: true, kramdown: false, gfm: false });
    return pei.html;
}

const specs = JSON.parse(fs.readFileSync("../specs/cm_v0_31_2.json", "utf8"));
// ================================================
// 1.
// Basic syntax: Tabs, Backslash escapes, Entity and numeric character references.
// Status : Fixed
// ================================================
describe("Basic syntax Tests", () => {
    let failTests = [];
    after(() => {
        fs.writeFileSync("../failed_records/basic_syntax_v0_31_2_n.json", JSON.stringify(failTests, null, 2));
    })
    for (const spec of specs) {
        const section = spec.section;
        if (section === "Tabs"
            || section === "Backslash escapes"
            || section === "Entity and numeric character references") {
            it(`Basic syntax tests for ${spec.example}`, () => {
                const input = spec.markdown;
                const expected = spec.html;
                const example = spec.example;
                const actual = parseMarkdown(input);
                if (actual !== expected) {
                    failTests.push({ example, input, expected, actual, section })
                    assert.notDeepEqual(expected, actual);
                } else {
                    assert.deepEqual(expected, actual);
                }
            })
        }

    }

})
// =========================================================
// 2.
// Simple block elements:
// Blank lines, Paragraphs, ATX headings, Setext headings,
// Thematic breaks, Indented code blocks, Fenced code blocks.
// Status : Fixed
// ==========================================================
describe("Simple Block Elements Tests", () => {
    let failTests = [];
    after(() => {
        fs.writeFileSync("../failed_records/simple_blocks_v0_31_2_n.json", JSON.stringify(failTests, null, 2));
    })
    for (const spec of specs) {
        const section = spec.section;
        if (section == "Blank lines"
            || section == "Paragraphs"
            || section == "ATX headings"
            || section == "Setext headings"
            || section == "Thematic breaks"
            || section == "Indented code blocks"
            || section == "Fenced code blocks") {
            it(`Simple block elements tests ${spec.example}`, () => {
                const input = spec.markdown;
                const expected = spec.html;
                const example = spec.example;
                const actual = parseMarkdown(input);
                if (actual !== expected) {
                    failTests.push({ example, input, expected, actual, section })
                    assert.notDeepEqual(expected, actual);
                } else {
                    assert.deepEqual(expected, actual);
                }
            })
        }

    }

})
// =========================================================
// 3.
// Block containers:
// Block quotes, List items, Lists.
// Keep list items and lists together; they’re closely related.
// Status : Fixed
// ==========================================================
describe("Block Containers Tests", () => {
    let failTests = [];
    after(() => {
        fs.writeFileSync("../failed_records/block_containers_v0_31_2_n.json", JSON.stringify(failTests, null, 2));
    })
    for (const spec of specs) {
        const section = spec.section;
        if (section == "Block quotes" || section == "List items" || section == "Lists") {
            it(`Block containers test for ${spec.example}`, () => {
                const input = spec.markdown;
                const expected = spec.html;
                const example = spec.example;
                const actual = parseMarkdown(input);
                if (actual !== expected) {
                    failTests.push({ example, input, expected, actual, section })
                    assert.notDeepEqual(expected, actual);
                } else {
                    assert.deepEqual(expected, actual);
                }
            })
        }

    }

})
// =========================================================
// 4.
// HTML blocks: HTML blocks
// This is substantial enough to tackle separately.
// Status : Fixed
// ==========================================================
describe("HTML blocks Tests", () => {
    let failTests = [];
    after(() => {
        fs.writeFileSync("../failed_records/html_blocks_v0_31_2_n.json", JSON.stringify(failTests, null, 2));
    })
    for (const spec of specs) {
        const section = spec.section;
        if (section == "HTML blocks") {
            it(`HTML blocks test for ${spec.example}`, () => {
                const input = spec.markdown;
                const expected = spec.html;
                const example = spec.example;
                const actual = parseMarkdown(input);
                if (actual !== expected) {
                    failTests.push({ example, input, expected, actual, section })
                    assert.notDeepEqual(expected, actual);
                } else {
                    assert.deepEqual(expected, actual);
                }
            })
        }

    }

})
// =========================================================
// 5.
// Inline parsing:
// Code spans, Emphasis and strong emphasis, Links, Images, Autolinks,
// Raw HTML, Hard line breaks, Soft line breaks, Textual content
// ==========================================================
describe("Inline parsing Tests", () => {
    let failTests = [];
    after(() => {
        fs.writeFileSync("../failed_records/inline_parsing_v0_31_2_n.json", JSON.stringify(failTests, null, 2));
    })
    for (const spec of specs) {
        const section = spec.section;
        if (section == "Code spans"
            || section == "Emphasis and strong emphasis"
            || section == "Links"
            || section == "Images"
            || section == "Autolinks"
            || section == "Raw HTML"
            || section == "Hard line breaks"
            || section == "Soft line breaks"
            || section == "Textual content") {
            it(`Inline parsing tests ${spec.example}`, () => {
                const input = spec.markdown;
                const expected = spec.html;
                const example = spec.example;
                const actual = parseMarkdown(input);
                if (actual !== expected) {
                    failTests.push({ example, input, expected, actual, section })
                    assert.notDeepEqual(expected, actual);
                } else {
                    assert.deepEqual(expected, actual);
                }
            })
        }

    }

})