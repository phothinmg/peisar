//! Compile-verification of every public Rust path referenced in README.md.

use peisar::frontmatter::frontmatter;
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
    let parsed = frontmatter("---\ntitle: Hello\n---\n\n# Hello".to_string()).unwrap();
    let md: &str = parsed.pure_markdown_content();
    assert_eq!(md, "# Hello");
    let yaml = parsed.yaml_data().unwrap();
    assert_eq!(yaml["title"], "Hello");
    let (md, yaml) = parsed.into_parts();
    assert_eq!(md, "# Hello");
    assert!(yaml.is_some());
}

#[test]
fn cache_paths() {
    use peisar::cache::{CachedContent, MemoryConfig, PeisarCache, PeisarCacheConfig};
    use std::fs;
    use std::path::PathBuf;

    // PeisarCache archives under `.peisar-cache` in the current working
    // directory, so run the check from a throwaway directory.
    let tmp = std::env::temp_dir().join(format!("peisar_readme_cache_{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(tmp.join("contents")).unwrap();
    let index = tmp.join("contents").join("index.md");
    fs::write(&index, "# Hello\n").unwrap();

    let orig = std::env::current_dir().unwrap();
    std::env::set_current_dir(&tmp).unwrap();

    // with_config(PeisarCacheConfig { .. }) from the README
    let mut cache = PeisarCache::with_config(PeisarCacheConfig {
        entry_dir: "contents".into(),
        memory: Some(MemoryConfig {
            byte_budget: Some(16 * 1024 * 1024),
            ..Default::default()
        }),
        ..Default::default()
    })
    .unwrap();

    // all(): absolute path -> raw text for every cached markdown file
    let all: std::collections::HashMap<PathBuf, String> = cache.all();
    assert_eq!(all.len(), 1);
    assert_eq!(all[&index], "# Hello\n");

    // get(): variant-preserving lookup by absolute path (memory tier first)
    match cache.get(&index) {
        Some(CachedContent::Text(md)) => assert_eq!(md, "# Hello\n"),
        other => panic!("expected cached text, got {:?}", other),
    }

    // page_objects(): Liquid-ready derived objects
    let pages = cache.page_objects();
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0].title.as_deref(), Some("Hello"));
    // DEFAULT_OUT_DIR is ".peisar" when outDir is not configured.
    assert_eq!(pages[0].out_file_path, ".peisar/index.html");

    // start_watching() + Drop (the Rust equivalent of JS dispose())
    cache.start_watching().unwrap();
    drop(cache);

    std::env::set_current_dir(orig).unwrap();
    let _ = fs::remove_dir_all(&tmp);
}
