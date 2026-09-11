use std::fs;
// use peisar::ast::Inline;
use peisar::config::{ParseOptions, RenderOptions};
use peisar::html;
use peisar::parser;
use peisar::plugin_factory::{PluginContext, PluginPipeline};
use peisar::plugins::BlockquoteAlert;

fn main() {
    let txt = fs::read_to_string("test.md").unwrap();

    let parse_opts = ParseOptions {
        gfm: true,
        kramdown: true,
    };
    let doc = parser::parse_with(&txt, &parse_opts);
    let mut pipeline = PluginPipeline::new();
    pipeline.add(BlockquoteAlert::default());

    // Pretty-print the AST as JSON
    let doc_json = serde_json::to_string_pretty(&doc).unwrap();

    // Render as a full HTML document (fragment = false)
    let render_opts = RenderOptions {
        fragment: false,
        charset: true,
        viewport: true,
        title: Some("peisar output".to_string()),
        body_class: Some("markdown-body".to_string()),
        style: None,
    };
    let ctx = PluginContext::new(parse_opts);
    let html = html::render_document_with(&doc, &render_opts, &pipeline, &ctx);

    fs::write("aa.json", &doc_json).ok();
    fs::write("aa.html", &html).ok();
}
