use peisar::plugin_factory::{PluginContext, PluginPipeline};
use peisar::plugins::BlockquoteAlert;
use peisar::{
    config::{ParseOptions, RenderOptions},
    parser,
};

fn main() {
    let md = std::fs::read_to_string("/tmp/test_alerts.md").unwrap();
    let opts = ParseOptions::default();
    let doc = parser::parse_with(&md, &opts);
    let mut pipeline = PluginPipeline::new();

    let render_opts = RenderOptions::default();
    let ctx = PluginContext::new(opts);
    pipeline.add(BlockquoteAlert::default());
    let html = peisar::html::render_document_with(&doc, &render_opts, &pipeline, &ctx);
    println!("{}", html);
}
