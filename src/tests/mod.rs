#[cfg(test)]
mod tests {
    use crate::ast::*;
    use crate::config::{ParseOptions, RenderOptions};
    use crate::html;
    use crate::parser::inline::parse_inline;
    use crate::parser::{parse, parse_with};
    use crate::plugin_factory::{Plugin, PluginContext, PluginPipeline};
    use crate::visitor::{NodeCounter, visit_document};

    fn first_block(doc: &Document) -> &Block {
        &doc.children[0]
    }

    fn html_of(md: &str, fragment: bool) -> String {
        let doc = parse(md);
        html::render_document(&doc, Some(fragment))
    }

    // -----------------------------------------------------------------------
    // CommonMark
    // -----------------------------------------------------------------------

    #[test]
    fn parses_heading() {
        let doc = parse("# Hello *World*");
        match first_block(&doc) {
            Block::Heading {
                level, children, ..
            } => {
                assert_eq!(*level, 1);
                assert_eq!(children.len(), 2);
                assert!(matches!(&children[0], Inline::Text { value, .. } if value == "Hello "));
                match &children[1] {
                    Inline::Emphasis {
                        level: EmphasisLevel::Italic,
                        children,
                        ..
                    } => {
                        assert_eq!(children.len(), 1);
                        assert!(
                            matches!(&children[0], Inline::Text { value, .. } if value == "World")
                        );
                    }
                    other => panic!("expected emphasis, got {other:?}"),
                }
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn parses_code_block_with_lang() {
        let doc = parse("```rust\nfn hi() {}\n```");
        match first_block(&doc) {
            Block::CodeBlock { lang, code, .. } => {
                assert_eq!(lang.as_deref(), Some("rust"));
                assert_eq!(code, "fn hi() {}");
            }
            other => panic!("expected code block, got {other:?}"),
        }
    }

    #[test]
    fn parses_thematic_break() {
        for input in ["---", "***", "___", "  - - -  "] {
            let doc = parse(input);
            assert!(
                matches!(first_block(&doc), Block::ThematicBreak { .. }),
                "input: {input}"
            );
        }
    }

    #[test]
    fn parses_unordered_list() {
        let doc = parse("- a\n- b\n- c");
        match first_block(&doc) {
            Block::List { ordered, items, .. } => {
                assert!(!*ordered);
                assert_eq!(items.len(), 3);
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn parses_ordered_list() {
        let doc = parse("1. first\n2. second");
        match first_block(&doc) {
            Block::List { ordered, items, .. } => {
                assert!(*ordered);
                assert_eq!(items.len(), 2);
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn parses_block_quote() {
        let doc = parse("> hello\n> world");
        match first_block(&doc) {
            Block::BlockQuote { children, .. } => {
                assert_eq!(children.len(), 1);
            }
            other => panic!("expected block quote, got {other:?}"),
        }
    }

    #[test]
    fn parses_link() {
        let tokens = parse_inline("[text](https://x.com)");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Inline::Link {
                text,
                url,
                title,
                autolink,
                ..
            } => {
                assert_eq!(url, "https://x.com");
                assert!(!autolink);
                assert!(title.is_none());
                assert_eq!(text.len(), 1);
                assert!(matches!(&text[0], Inline::Text { value, .. } if value == "text"));
            }
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn parses_link_with_title() {
        let tokens = parse_inline(r#"[t](https://x.com "Title")"#);
        match &tokens[0] {
            Inline::Link { title, .. } => assert_eq!(title.as_deref(), Some("Title")),
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn parses_image() {
        let tokens = parse_inline("![alt](img.png)");
        match &tokens[0] {
            Inline::Image { alt, url, .. } => {
                assert_eq!(alt, "alt");
                assert_eq!(url, "img.png");
            }
            other => panic!("expected image, got {other:?}"),
        }
    }

    #[test]
    fn parses_bold_and_italic() {
        let tokens = parse_inline("**bold** and *italic*");
        assert!(tokens.iter().any(|t| matches!(
            t,
            Inline::Emphasis {
                level: EmphasisLevel::Bold,
                ..
            }
        )));
        assert!(tokens.iter().any(|t| matches!(
            t,
            Inline::Emphasis {
                level: EmphasisLevel::Italic,
                ..
            }
        )));
    }

    #[test]
    fn parses_inline_code() {
        let tokens = parse_inline("`x = 1`");
        assert_eq!(tokens.len(), 1);
        assert!(matches!(&tokens[0], Inline::Code { code, .. } if code == "x = 1"));
    }

    // -----------------------------------------------------------------------
    // GFM
    // -----------------------------------------------------------------------

    #[test]
    fn gfm_strikethrough() {
        let tokens = parse_inline("~~deleted~~");
        assert!(
            tokens
                .iter()
                .any(|t| matches!(t, Inline::Strikethrough { .. }))
        );
    }

    #[test]
    fn gfm_strikethrough_renders_del() {
        let h = html_of("~~text~~", true);
        assert!(h.contains("<del>text</del>"));
    }

    #[test]
    fn gfm_table() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |\n";
        let doc = parse(md);
        match first_block(&doc) {
            Block::Table { table, .. } => {
                assert_eq!(table.header.cells.len(), 2);
                assert_eq!(table.rows.len(), 1);
                assert_eq!(table.rows[0].cells.len(), 2);
            }
            other => panic!("expected table, got {other:?}"),
        }
    }

    #[test]
    fn gfm_table_renders_html() {
        let h = html_of("| A | B |\n|---|---|\n| 1 | 2 |\n", true);
        assert!(h.contains("<table>"));
        assert!(h.contains("<thead>"));
        assert!(h.contains("<tbody>"));
        assert!(h.contains("<th>A</th>"));
        assert!(h.contains("<td>1</td>"));
    }

    #[test]
    fn gfm_table_alignment() {
        let md = "| A | B | C |\n|:---|:---:|---:|\n| 1 | 2 | 3 |\n";
        let doc = parse(md);
        match first_block(&doc) {
            Block::Table { table, .. } => {
                assert_eq!(table.alignments.len(), 3);
                assert!(matches!(table.alignments[0], TableCellAlignment::Left));
                assert!(matches!(table.alignments[1], TableCellAlignment::Center));
                assert!(matches!(table.alignments[2], TableCellAlignment::Right));
            }
            other => panic!("expected table, got {other:?}"),
        }
    }

    #[test]
    fn gfm_task_list() {
        let md = "- [x] done\n- [ ] todo\n";
        let doc = parse(md);
        match first_block(&doc) {
            Block::List { items, .. } => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0].task, Some(TaskState::Checked)));
                assert!(matches!(items[1].task, Some(TaskState::Unchecked)));
            }
            other => panic!("expected list, got {other:?}"),
        }
    }

    #[test]
    fn gfm_task_list_renders_checkbox() {
        let h = html_of("- [x] done\n- [ ] todo\n", true);
        assert!(h.contains(r#"type="checkbox" checked"#));
        assert!(h.contains(r#"type="checkbox" disabled"#));
    }

    #[test]
    fn gfm_autolink() {
        let tokens = parse_inline("Visit https://example.com today");
        let found = tokens.iter().any(|t| {
            matches!(
                t,
                Inline::Link { url, autolink: true, .. } if url == "https://example.com"
            )
        });
        assert!(found, "expected autolink, got {tokens:?}");
    }

    // -----------------------------------------------------------------------
    // Kramdown attributes
    // -----------------------------------------------------------------------

    #[test]
    fn kramdown_heading_id() {
        let md = "# Title\n{:#my-id}";
        let doc = parse(md);
        match first_block(&doc) {
            Block::Heading {
                attrs, children, ..
            } => {
                // children should not contain the {:...} text
                assert!(attrs.as_ref().unwrap().id.as_deref() == Some("my-id"));
                // heading text should be "Title"
                assert!(matches!(&children[0], Inline::Text { value, .. } if value == "Title"));
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn kramdown_heading_classes() {
        let md = "# Title\n{:.big .red}";
        let doc = parse(md);
        match first_block(&doc) {
            Block::Heading { attrs, .. } => {
                let a = attrs.as_ref().unwrap();
                assert_eq!(a.classes, vec!["big", "red"]);
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn kramdown_heading_kv() {
        let md = "# Title\n{:data-toggle=\"modal\"}";
        let doc = parse(md);
        match first_block(&doc) {
            Block::Heading { attrs, .. } => {
                let a = attrs.as_ref().unwrap();
                assert_eq!(a.attributes, vec![("data-toggle".into(), "modal".into())]);
            }
            other => panic!("expected heading, got {other:?}"),
        }
    }

    #[test]
    fn kramdown_renders_html_attrs() {
        let h = html_of("# Title\n{:#hdr .big .red data-x=\"y\"}", true);
        assert!(h.contains(r#"id="hdr""#), "got: {h}");
        assert!(h.contains(r#"class="big red""#), "got: {h}");
        assert!(h.contains(r#"data-x="y""#), "got: {h}");
    }

    #[test]
    fn kramdown_trailing_block_attrs() {
        // After a code block, a {...} line applies to it
        let md = "```\ncode\n```\n{#code-id}\n";
        let doc = parse(md);
        match first_block(&doc) {
            Block::CodeBlock { attrs, .. } => {
                assert_eq!(attrs.as_ref().unwrap().id.as_deref(), Some("code-id"));
            }
            other => panic!("expected code block, got {other:?}"),
        }
    }

    #[test]
    fn kramdown_disabled() {
        let opts = ParseOptions {
            kramdown: false,
            gfm: true,
        };
        let doc = parse_with("# Title\n{:#my-id}", &opts);
        match first_block(&doc) {
            Block::Heading { attrs, .. } => {
                // kramdown disabled → the {:...} line is just a paragraph
                assert!(attrs.is_none());
            }
            other => panic!("expected heading, got {other:?}"),
        }
        // The {:...} line should appear as a separate paragraph
        assert_eq!(doc.children.len(), 2);
        match &doc.children[1] {
            Block::Paragraph { children, .. } => {
                assert!(
                    matches!(&children[0], Inline::Text { value, .. } if value.contains("{:#my-id}"))
                );
            }
            other => panic!("expected paragraph, got {other:?}"),
        }
    }

    // -----------------------------------------------------------------------
    // Config / fragment
    // -----------------------------------------------------------------------

    #[test]
    fn renders_full_document() {
        let doc = parse("# Hi");
        let h = html::render_document(&doc, Some(false));
        assert!(h.contains("<!DOCTYPE html>"));
        assert!(h.contains("<html>"));
        assert!(h.contains("<body>"));
        assert!(h.contains("</body>"));
        assert!(h.contains("</html>"));
    }

    #[test]
    fn renders_fragment() {
        let doc = parse("# Hi");
        let h = html::render_document(&doc, Some(true));
        assert!(!h.contains("<!DOCTYPE html>"));
        assert!(!h.contains("<html>"));
        assert!(h.contains("<h1>Hi</h1>"));
    }

    #[test]
    fn render_options_title_and_style() {
        let doc = parse("# Hi");
        let opts = RenderOptions {
            fragment: false,
            charset: true,
            viewport: true,
            title: Some("Test".to_string()),
            body_class: Some("doc".to_string()),
            style: Some("body { color: red; }".to_string()),
        };
        let h = html::render_document_with(
            &doc,
            &opts,
            &PluginPipeline::new(),
            &PluginContext::new(ParseOptions::default()),
        );
        assert!(h.contains("<title>Test</title>"));
        assert!(h.contains("body { color: red; }"));
        assert!(h.contains(r#"class="doc""#));
    }

    // -----------------------------------------------------------------------
    // Visitor
    // -----------------------------------------------------------------------

    #[test]
    fn visitor_counts_nodes() {
        let doc = parse("# Title\n\nparagraph with **bold**.\n\n- a\n- b");
        let mut counter = NodeCounter::default();
        visit_document(&mut counter, &doc);
        assert_eq!(counter.headings, 1);
        assert_eq!(counter.paragraphs, 3);
        assert_eq!(counter.emphasis, 1);
        assert_eq!(counter.lists, 1);
        assert_eq!(counter.list_items, 2);
    }

    #[test]
    fn visitor_counts_gfm() {
        let doc = parse("| A | B |\n|---|---|\n| 1 | 2 |\n\n~~strike~~");
        let mut counter = NodeCounter::default();
        visit_document(&mut counter, &doc);
        assert_eq!(counter.tables, 1);
        assert_eq!(counter.strikethroughs, 1);
    }

    #[test]
    fn escapes_html_in_text() {
        let h = html_of("< not a tag & stuff", true);
        assert!(h.contains("&lt; not a tag &amp; stuff"));
    }

    #[test]
    fn renders_code_block_lang_class() {
        let h = html_of("```js\nconst x = 1;\n```", true);
        assert!(h.contains("class=\"language-js\""));
    }

    #[test]
    fn custom_visitor_collects_links() {
        let md = "# Heading\n\n[one](https://a.com) and [two](https://b.com)\n\n## Sub\n\n![img](pic.png)";
        let doc = parse(md);

        let mut collector = LinkCollector::default();
        visit_document(&mut collector, &doc);

        assert_eq!(collector.heading_count, 2);
        assert_eq!(collector.urls, vec!["https://a.com", "https://b.com"]);
        assert!(!collector.urls.contains(&"pic.png".to_string()));
    }

    #[derive(Default)]
    struct LinkCollector {
        urls: Vec<String>,
        heading_count: usize,
    }

    impl crate::visitor::AstNodeVisitor for LinkCollector {
        fn visit_heading(
            &mut self,
            _level: u8,
            children: &[Inline],
            _attrs: Option<&KramdownAttributes>,
        ) {
            self.heading_count += 1;
            for inline in children {
                self.visit_inline(inline);
            }
        }

        fn visit_link(&mut self, text: &[Inline], url: &str, _title: Option<&str>) {
            self.urls.push(url.to_string());
            for inline in text {
                self.visit_inline(inline);
            }
        }
    }

    // -----------------------------------------------------------------------
    // Plugins
    // -----------------------------------------------------------------------

    #[test]
    fn plugin_heading_class() {
        struct AddClass;
        impl Plugin for AddClass {
            fn name(&self) -> &str {
                "add-class"
            }
            fn transform_ast(&self, doc: &mut Document, _ctx: &PluginContext) {
                for block in &mut doc.children {
                    if let Block::Heading { attrs, .. } = block {
                        let mut a = attrs.take().unwrap_or_default();
                        a.classes.push("anchor".into());
                        *attrs = Some(a);
                    }
                }
            }
        }

        let doc = parse("# Hi\n\nText");
        let ctx = PluginContext::new(ParseOptions::default());
        let mut pipeline = PluginPipeline::new();
        pipeline.add(AddClass);

        let h = html::render_document_with(&doc, &RenderOptions::default(), &pipeline, &ctx);
        assert!(h.contains(r#"class="anchor""#), "got: {h}");
    }

    #[test]
    fn plugin_wrap_div() {
        use crate::plugins::WrapDiv;

        let doc = parse("# Hi");
        let ctx = PluginContext::new(ParseOptions::default());
        let mut pipeline = PluginPipeline::new();
        pipeline.add(WrapDiv {
            class: "wrapper".to_string(),
        });

        let h = html::render_document_with(&doc, &RenderOptions::default(), &pipeline, &ctx);
        assert!(h.contains(r#"<div class="wrapper">"#));
        assert!(h.contains("</div>"));
    }

    #[test]
    fn plugin_pipeline_names() {
        use crate::plugins::WrapDiv;

        let mut pipeline = PluginPipeline::new();
        pipeline.add(WrapDiv {
            class: "x".to_string(),
        });
        let names: Vec<&str> = pipeline.plugin_names().collect();
        assert_eq!(names, vec!["wrap-div"]);
    }
}
