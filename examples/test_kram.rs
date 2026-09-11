use peisar::{parser, config::{ParseOptions, RenderOptions}};

fn main() {
    let opts = ParseOptions::default();
    let render_opts = RenderOptions::default();

    // Test 1: heading with inline attrs
    let doc = parser::parse_with("# Hello {#my-id .big .red}", &opts);
    println!("=== Heading attrs (AST) ===");
    println!("{}", serde_json::to_string_pretty(&doc).unwrap());

    let html = peisar::html::render_document_with(&doc, &render_opts, &peisar::plugin_factory::PluginPipeline::new(), &peisar::plugin_factory::PluginContext::new(opts.clone()));
    println!("=== Heading attrs (HTML) ===");
    println!("{}", html);

    // Test 2: paragraph with trailing attrs
    let doc2 = parser::parse_with("Hello world\n{#para-id .cls}", &opts);
    println!("=== Paragraph trailing attrs (HTML) ===");
    let html2 = peisar::html::render_document_with(&doc2, &render_opts, &peisar::plugin_factory::PluginPipeline::new(), &peisar::plugin_factory::PluginContext::new(opts.clone()));
    println!("{}", html2);

    // Test 3: thematic break with trailing attrs
    let doc3 = parser::parse_with("---\n{#hr-id}", &opts);
    println!("=== Thematic break trailing attrs (HTML) ===");
    let html3 = peisar::html::render_document_with(&doc3, &render_opts, &peisar::plugin_factory::PluginPipeline::new(), &peisar::plugin_factory::PluginContext::new(opts.clone()));
    println!("{}", html3);
}
