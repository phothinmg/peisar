use peisar::{parser, config::{ParseOptions, RenderOptions}};
use peisar::plugin_factory::{PluginPipeline, PluginContext};

fn main() {
    let opts = ParseOptions::default();
    let render_opts = RenderOptions::default();
    let pipe = PluginPipeline::new();
    let ctx = PluginContext::new(opts.clone());

    let inputs = vec![
        "# Hello\n{:#my-id .big .red data-toggle=\"modal\"}",
        "Hello\n{:#para-id .cls}",
        "---\n{:#hr-id}",
        "> quote\n{:#q-id}",
        "- item\n{:#list-id}",
        "```rust\ncode\n```\n{:#code-id}",
        "| A | B |\n|---|---|\n| 1 | 2 |\n{:#tbl-id}",
    ];
    for input in &inputs {
        let doc = parser::parse_with(input, &opts);
        let html = peisar::html::render_document_with(&doc, &render_opts, &pipe, &ctx);
        println!("INPUT: {:?}", input);
        println!("HTML:  {}", html.replace('\n', "⏎\n        "));
        println!();
    }
}
