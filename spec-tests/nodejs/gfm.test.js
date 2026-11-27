const { peisar } = require("./peisar-core");
const { describe, it } = require("node:test");
const assert = require("node:assert/strict");

function render(markdown, gfm) {
    return new peisar.Peisar(markdown, {
        fragment: true,
        gfm,
        kramdown: false,
    }).html;
}

describe("GFM examples", () => {
    it("renders strikethrough, autolinks, task lists, and tables", () => {
        const html = render(
            "~~gone~~ and https://example.com\n\n- [x] done\n\n| A | B |\n| --- | ---: |\n| one | two |\n",
            true,
        );

        assert.ok(html.includes("<del>gone</del>"));
        assert.ok(
            html.includes('<a href="https://example.com">https://example.com</a>'),
        );
        assert.ok(html.includes('class="task-list-item"'));
        assert.ok(html.includes("<table>"));
        assert.ok(html.includes('<td style="text-align: right">two</td>'));
    });

    it("does not render GFM extensions when disabled", () => {
        const html = render("~~gone~~\n\n| A | B |\n| --- | --- |\n", false);

        assert.ok(html.includes("~~gone~~"));
        assert.ok(!html.includes("<del>"));
        assert.ok(!html.includes("<table>"));
    });
});
