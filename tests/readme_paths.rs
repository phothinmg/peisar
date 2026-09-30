//! Compile-verification of every public Rust path referenced in README.md.

use peisar::frontmatter::parse_markdown_frontmatter;
use peisar::markdown::ast::tokens::Attributes;
use peisar::markdown::ast::tokens::span::Span;
use peisar::markdown::ast::tokens::token::{Block, Inline};
use peisar::markdown::ast::visitor::{InlineVisitControl, VisitControl, visit_document_mut};
use peisar::markdown::ast::{
    AstOptions, AstParser, AstVisitor, BlockParseContext, Document, InlineParseContext, LinkRefMap,
    ParseHooks, PeisarAst, parse_inline, parse_inline_with_refs,
};
use peisar::markdown::config::PeisarOptions;
use peisar::markdown::html::{RenderOptions, render_document_html};
use peisar::markdown::peisar::Peisar;

#[test]
fn markdown_module_paths() {
    // Document::parse
    let doc = Document::parse("# Hello **world**\n", &AstOptions::default(), None);
    assert_eq!(doc.node_type, "root");

    // Strict CommonMark options
    let opts = AstOptions {
        gfm: false,
        kramdown: false,
        file_name: Some("doc.md".into()),
    };
    let doc = Document::parse("# Title\n", &opts, opts.file_name.clone());
    assert_eq!(doc.file_name.as_deref(), Some("doc.md"));

    // Peisar mirrors the JS class
    let mut document = Peisar::new("# Hello **world**".into(), None);
    assert!(document.html().len() > 0);

    let mut fragment = Peisar::new(
        "# Hello **world**".into(),
        Some(PeisarOptions {
            fragment: Some(true),
            ..Default::default()
        }),
    );
    assert_eq!(fragment.html(), "<h1>Hello <strong>world</strong></h1>\n");

    // PeisarAst
    let mut ast = PeisarAst::new("---\ntitle: Hello\n---\n\n# Hello".into(), None);
    assert_eq!(
        ast.get_frontmatter().unwrap()["title"].as_str(),
        Some("Hello")
    );
    assert!(matches!(
        &ast.get_ast().children[0],
        Block::Heading { level: 1, .. }
    ));
    let json = ast.ast_json();
    assert!(json.contains("\"type\":\"heading\""));

    // Inline parsing
    let inlines = parse_inline("*hi* and `code`", None);
    assert!(matches!(&inlines[0], Inline::Emphasis { .. }));

    let mut refs = LinkRefMap::new();
    refs.insert("example".into(), ("https://example.com".into(), None));
    let inlines = parse_inline_with_refs("see [example][]", None, Some(&refs));
    assert!(matches!(
        &inlines[1],
        Inline::LinkReference { label, url, .. }
            if label == "example" && url == "https://example.com"
    ));

    // render_document_html
    let doc = Document::parse("# Hi\n", &AstOptions::default(), None);
    let html = render_document_html(
        &doc,
        Some(RenderOptions {
            fragment: true,
            ..Default::default()
        }),
    );
    assert_eq!(html, "<h1>Hi</h1>\n");

    // Attributes helper
    let attrs = Attributes {
        id: Some("intro".into()),
        classes: Some(vec!["a".into(), "b".into()]),
        attributes: None,
    };
    assert_eq!(attrs.to_html_attr_string(), "id=\"intro\" class=\"a b\"");
}

#[test]
fn visitor_paths() {
    struct AddTitleClass;

    impl AstVisitor for AddTitleClass {
        fn visit_block(&mut self, block: &mut Block) -> VisitControl {
            if let Block::Heading { attrs: slot, .. } = block {
                let mut attrs = slot.take().unwrap_or_default();
                let mut classes = attrs.classes.take().unwrap_or_default();
                classes.push("title".into());
                attrs.classes = Some(classes);
                *slot = Some(attrs);
                VisitControl::keep_and_recurse()
            } else {
                VisitControl::default()
            }
        }

        fn visit_inline(&mut self, _inline: &mut Inline) -> InlineVisitControl {
            InlineVisitControl::default()
        }
    }

    let mut doc = Document::parse("# Hello\n", &AstOptions::default(), None);
    visit_document_mut(&mut doc, &mut AddTitleClass);
    let html = render_document_html(
        &doc,
        Some(RenderOptions {
            fragment: true,
            ..Default::default()
        }),
    );
    assert_eq!(html, "<h1 class=\"title\">Hello</h1>\n");
}

#[test]
fn hooks_paths() {
    struct Directive;

    impl AstParser for Directive {
        fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
            let name = ctx.line.trim().strip_prefix(":::")?;
            if name.is_empty() {
                return None;
            }
            let close = ctx.lines.iter().skip(1).position(|l| l.trim() == ":::")?;
            Some((
                Block::HtmlBlock {
                    html: format!("<div class=\"directive\" data-name=\"{name}\">"),
                    pos: Span::default(),
                    attrs: None,
                },
                close + 2,
            ))
        }

        fn try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)> {
            if !ctx.rest.starts_with("@@") {
                return None;
            }
            let end = ctx.rest[2..].find(' ')?;
            Some((
                Inline::HtmlInline {
                    html: format!("<span>{}</span>", &ctx.rest[2..2 + end]),
                    pos: Span::default(),
                },
                end + 3,
            ))
        }
    }

    let _hooks = ParseHooks::empty().with(&Directive);
    assert!(!_hooks.is_empty());
}

#[test]
fn frontmatter_paths() {
    let parsed = parse_markdown_frontmatter("---\ntitle: Hello\n---\n\n# Hello").unwrap();
    let md: &str = parsed.pure_markdown_content();
    assert_eq!(md, "# Hello");
    let yaml = parsed.yaml_data().unwrap();
    assert_eq!(yaml["title"], "Hello");
    let (md, yaml) = parsed.into_parts();
    assert_eq!(md, "# Hello");
    assert!(yaml.is_some());
}
