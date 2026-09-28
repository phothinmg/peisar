//! Unit tests for `peisar_ast`.
//!
//! These tests cover:
//! - Link reference definitions (block-level parsing)
//! - HTML comments
//! - Reference-style inline links
//! - Existing parsers (headings, paragraphs, lists, code, emphasis, links)
//! - Helper functions (`normalize_label`, `parse_link_url`, `parse_link_title`,
//!   `compute_line_starts`, `is_link_ref_def`, `is_thematic_break`)

use super::parsers::block::{
    compute_line_starts, is_link_ref_def, is_list_marker, is_thematic_break, normalize_label,
    parse_link_ref_def_line, parse_link_title, parse_link_url,
};
use super::parsers::hooks::{AstParser, BlockParseContext, InlineParseContext, ParseHooks};
use super::parsers::inline::{LinkRefMap, parse_inline, parse_inline_with_refs};
use super::parsers::md_to_ast;
use super::parsers::visitor::{VisitControl, visit_document_mut};
use super::tokens::token::{Block, Inline};
use super::{AstOptions, AstVisitor};

// ---------------------------------------------------------------------------
// Helper: count block variants in a document
// ---------------------------------------------------------------------------

fn count_blocks(doc: &crate::ast::Document) -> usize {
    doc.children.len()
}

// ---------------------------------------------------------------------------
// Link reference definitions — block-level
// ---------------------------------------------------------------------------

#[test]
fn test_link_ref_def_basic() {
    let md = "[example]: https://example.com\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 1);
    let def = &doc.link_references[0];
    assert_eq!(def.label, "example");
    assert_eq!(def.url, "https://example.com");
    assert_eq!(def.title, None);
}

#[test]
fn test_link_ref_def_with_title() {
    let md = "[example]: https://example.com \"Example Site\"\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 1);
    let def = &doc.link_references[0];
    assert_eq!(def.label, "example");
    assert_eq!(def.url, "https://example.com");
    assert_eq!(def.title.as_deref(), Some("Example Site"));
}

#[test]
fn test_link_ref_def_single_quote_title() {
    let md = "[ex]: <https://ex.com> 'Title Here'\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 1);
    let def = &doc.link_references[0];
    assert_eq!(def.url, "https://ex.com");
    assert_eq!(def.title.as_deref(), Some("Title Here"));
}

#[test]
fn test_link_ref_def_angle_bracket_url() {
    let md = "[label]: <https://example.com/path?q=1>\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 1);
    assert_eq!(doc.link_references[0].url, "https://example.com/path?q=1");
}

#[test]
fn test_link_ref_def_label_normalisation() {
    // Uppercase label should be normalised to lowercase
    let md = "[My Label]: https://example.com\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references[0].label, "my label");
}

#[test]
fn test_link_ref_def_multiple() {
    let md = "[a]: https://a.com\n[b]: https://b.com\n[c]: https://c.com\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 3);
    assert_eq!(doc.link_references[0].label, "a");
    assert_eq!(doc.link_references[1].label, "b");
    assert_eq!(doc.link_references[2].label, "c");
}

#[test]
fn test_link_ref_def_appears_as_block_child() {
    let md = "[example]: https://example.com\n\nText after.\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    // The ref def should be a block child AND in link_references
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::LinkReferenceDefinition { label, .. } if label == "example"
    )));
    assert_eq!(doc.link_references.len(), 1);
}

#[test]
fn test_is_link_ref_def() {
    assert!(is_link_ref_def("[label]: https://example.com"));
    assert!(is_link_ref_def("  [label]: https://example.com"));
    assert!(is_link_ref_def("[label]: <https://example.com>"));
    assert!(!is_link_ref_def("[label](https://example.com)"));
    assert!(!is_link_ref_def("Not a ref def"));
    // Note: is_link_ref_def is a lightweight check — it does not validate
    // that the label is non-empty. Full validation happens in parse_link_ref_def_line.
    assert!(is_link_ref_def("[]: https://example.com"));
    assert!(!is_link_ref_def("no brackets here"));
}

#[test]
fn test_parse_link_ref_def_line_returns_position() {
    let input = "line one\n[ex]: https://ex.com\n";
    let line_starts = compute_line_starts(input);
    let def = parse_link_ref_def_line("[ex]: https://ex.com", 1, &line_starts, input).unwrap();
    assert_eq!(def.label, "ex");
    assert_eq!(def.url, "https://ex.com");
    assert_eq!(def.pos.start.line, 1);
    assert_eq!(def.pos.start.offset, 9); // "line one\n" = 9 bytes
}

// ---------------------------------------------------------------------------
// normalize_label
// ---------------------------------------------------------------------------

#[test]
fn test_normalize_label() {
    assert_eq!(normalize_label("Hello"), "hello");
    assert_eq!(normalize_label("  Hello  "), "hello");
    assert_eq!(normalize_label("Multiple   Spaces"), "multiple spaces");
    assert_eq!(normalize_label("Mixed\tCASE"), "mixed case");
    assert_eq!(normalize_label(""), "");
}

// ---------------------------------------------------------------------------
// parse_link_url
// ---------------------------------------------------------------------------

#[test]
fn test_parse_link_url_bare() {
    let (url, rest) = parse_link_url("https://example.com rest");
    assert_eq!(url, "https://example.com");
    assert_eq!(rest, " rest");
}

#[test]
fn test_parse_link_url_angle_brackets() {
    let (url, rest) = parse_link_url("<https://example.com> \"title\"");
    assert_eq!(url, "https://example.com");
    assert_eq!(rest, " \"title\"");
}

#[test]
fn test_parse_link_url_no_rest() {
    let (url, rest) = parse_link_url("https://example.com");
    assert_eq!(url, "https://example.com");
    assert_eq!(rest, "");
}

// ---------------------------------------------------------------------------
// parse_link_title
// ---------------------------------------------------------------------------

#[test]
fn test_parse_link_title_double_quote() {
    let (title, consumed) = parse_link_title("\"Hello World\"");
    assert_eq!(title.as_deref(), Some("Hello World"));
    // "Hello World" = 13 chars: 1 open quote + 11 content + 1 close quote
    assert_eq!(consumed, 13);
}

#[test]
fn test_parse_link_title_single_quote() {
    let (title, _consumed) = parse_link_title("'Hello'");
    assert_eq!(title.as_deref(), Some("Hello"));
}

#[test]
fn test_parse_link_title_paren() {
    let (title, _consumed) = parse_link_title("(Hello)");
    assert_eq!(title.as_deref(), Some("Hello"));
}

#[test]
fn test_parse_link_title_none() {
    let (title, _) = parse_link_title("");
    assert_eq!(title, None);

    let (title, _) = parse_link_title("not a title");
    assert_eq!(title, None);
}

// ---------------------------------------------------------------------------
// compute_line_starts
// ---------------------------------------------------------------------------

#[test]
fn test_compute_line_starts() {
    assert_eq!(compute_line_starts("a\nb\nc"), vec![0, 2, 4]);
    assert_eq!(compute_line_starts("no newlines"), vec![0]);
    assert_eq!(compute_line_starts(""), vec![0]);
    assert_eq!(compute_line_starts("\n"), vec![0, 1]);
    assert_eq!(compute_line_starts("a\n"), vec![0, 2]);
}

// ---------------------------------------------------------------------------
// is_thematic_break
// ---------------------------------------------------------------------------

#[test]
fn test_is_thematic_break() {
    assert!(is_thematic_break("---"));
    assert!(is_thematic_break("***"));
    assert!(is_thematic_break("___"));
    assert!(is_thematic_break("- - -"));
    assert!(is_thematic_break("  ---  "));
    assert!(!is_thematic_break("--"));
    assert!(!is_thematic_break("abc"));
    assert!(!is_thematic_break(""));
}

// ---------------------------------------------------------------------------
// is_list_marker
// ---------------------------------------------------------------------------

#[test]
fn test_is_list_marker() {
    assert!(is_list_marker("- item"));
    assert!(is_list_marker("* item"));
    assert!(is_list_marker("+ item"));
    assert!(is_list_marker("1. item"));
    assert!(is_list_marker("1) item"));
    assert!(is_list_marker("  - item"));
    assert!(!is_list_marker("not a list"));
    assert!(!is_list_marker("-- not a list"));
}

// ---------------------------------------------------------------------------
// HTML comments
// ---------------------------------------------------------------------------

#[test]
fn test_html_comment_block() {
    let md = "<!-- this is a comment -->\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::Comment { value, .. } if value == "this is a comment"
    )));
}

#[test]
fn test_html_comment_multiline() {
    let md = "<!-- line one\nline two\nline three -->\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::Comment { value, .. } if value.contains("line one") && value.contains("line three")
    )));
}

#[test]
fn test_html_comment_empty() {
    let md = "<!---->\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::Comment { value, .. } if value.is_empty()
    )));
}

// ---------------------------------------------------------------------------
// Reference-style links — inline
// ---------------------------------------------------------------------------

fn make_ref_map() -> LinkRefMap {
    let mut map = LinkRefMap::new();
    map.insert(
        "example".to_string(),
        (
            "https://example.com".to_string(),
            Some("Example".to_string()),
        ),
    );
    map
}

#[test]
fn test_full_reference_link() {
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[click here][example]", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::LinkReference { label, url, .. }
            if label == "example" && url == "https://example.com"
    )));
}

#[test]
fn test_collapsed_reference_link() {
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[example][]", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::LinkReference { label, url, .. }
            if label == "example" && url == "https://example.com"
    )));
}

#[test]
fn test_shortcut_reference_link() {
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[example]", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::LinkReference { label, url, .. }
            if label == "example" && url == "https://example.com"
    )));
}

#[test]
fn test_reference_link_with_title() {
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[example][]", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::LinkReference { title, .. } if title.as_deref() == Some("Example")
    )));
}

#[test]
fn test_reference_link_no_refs_returns_text() {
    // Without a ref map, [example] should NOT become a LinkReference
    let tokens = parse_inline("[example]", None);
    assert!(
        !tokens
            .iter()
            .any(|t| matches!(t, Inline::LinkReference { .. }))
    );
}

#[test]
fn test_reference_link_unresolved_label_returns_text() {
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[nonexistent]", None, Some(&refs));
    assert!(
        !tokens
            .iter()
            .any(|t| matches!(t, Inline::LinkReference { .. }))
    );
}

#[test]
fn test_reference_link_case_insensitive_label() {
    let refs = make_ref_map();
    // [EXAMPLE] should resolve because labels are normalised to lowercase
    let tokens = parse_inline_with_refs("[EXAMPLE]", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::LinkReference { label, .. } if label == "example"
    )));
}

#[test]
fn test_inline_link_takes_precedence_over_reference() {
    // [text](url) should produce Inline::Link, not Inline::LinkReference
    let refs = make_ref_map();
    let tokens = parse_inline_with_refs("[example](https://other.com)", None, Some(&refs));
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Link { url, .. } if url == "https://other.com"
    )));
    assert!(
        !tokens
            .iter()
            .any(|t| matches!(t, Inline::LinkReference { .. }))
    );
}

// ---------------------------------------------------------------------------
// Full document integration tests
// ---------------------------------------------------------------------------

#[test]
fn test_document_with_ref_def_and_reference_link() {
    let md = "[label]: https://example.com \"Title\"\n\nA [link][label] here.\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.link_references.len(), 1);
    assert_eq!(count_blocks(&doc), 2); // ref def block + paragraph

    // The paragraph should contain a LinkReference inline
    let para = doc.children.iter().find_map(|b| match b {
        Block::Paragraph { children, .. } => Some(children),
        _ => None,
    });
    assert!(para.is_some());
    let para = para.unwrap();
    assert!(para.iter().any(|t| matches!(
        t,
        Inline::LinkReference { label, url, .. }
            if label == "label" && url == "https://example.com"
    )));
}

#[test]
fn test_heading_and_paragraph() {
    let md = "# Title\n\nA paragraph.\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert_eq!(doc.children.len(), 2);
    assert!(matches!(&doc.children[0], Block::Heading { level: 1, .. }));
    assert!(matches!(&doc.children[1], Block::Paragraph { .. }));
}

#[test]
fn test_code_block_fenced() {
    let md = "```rust\nfn main() {}\n```\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::CodeBlock { lang, code, .. }
            if lang.as_deref() == Some("rust") && code == "fn main() {}"
    )));
}

#[test]
fn test_thematic_break_in_document() {
    let md = "Before\n\n---\n\nAfter\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(
        doc.children
            .iter()
            .any(|b| matches!(b, Block::ThematicBreak { .. }))
    );
}

#[test]
fn test_unordered_list() {
    let md = "- one\n- two\n- three\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::List { ordered: false, items, .. } if items.len() == 3
    )));
}

#[test]
fn test_ordered_list() {
    let md = "1. one\n2. two\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(doc.children.iter().any(|b| matches!(
        b,
        Block::List { ordered: true, items, .. } if items.len() == 2
    )));
}

#[test]
fn test_block_quote() {
    let md = "> quoted text\n";
    let doc = md_to_ast(md, &AstOptions::default(), None);
    assert!(
        doc.children
            .iter()
            .any(|b| matches!(b, Block::BlockQuote { .. }))
    );
}

#[test]
fn test_inline_emphasis_bold() {
    let tokens = parse_inline("**bold**", None);
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Emphasis { level, .. } if matches!(level, super::tokens::token::EmphasisLevel::Bold)
    )));
}

#[test]
fn test_inline_emphasis_italic() {
    let tokens = parse_inline("*italic*", None);
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Emphasis { level, .. } if matches!(level, super::tokens::token::EmphasisLevel::Italic)
    )));
}

#[test]
fn test_inline_code() {
    let tokens = parse_inline("`code`", None);
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Code { code, .. } if code == "code"
    )));
}

#[test]
fn test_inline_link() {
    let tokens = parse_inline("[text](https://example.com)", None);
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Link { url, .. } if url == "https://example.com"
    )));
}

#[test]
fn test_inline_image() {
    let tokens = parse_inline("![alt](https://example.com/img.png)", None);
    assert!(tokens.iter().any(|t| matches!(
        t,
        Inline::Image { url, alt, .. }
            if url == "https://example.com/img.png" && alt == "alt"
    )));
}

#[test]
fn test_empty_document() {
    let doc = md_to_ast("", &AstOptions::default(), None);
    assert_eq!(doc.children.len(), 0);
}

// ---------------------------------------------------------------------------
// PeisarAst high-level API
// ---------------------------------------------------------------------------

#[test]
fn test_peisar_ast_basic() {
    use super::PeisarAst;
    let md = "# Hello\n\nWorld.\n";
    let mut ast = PeisarAst::new(md.to_string(), None);
    assert_eq!(ast.get_ast().children.len(), 2);
}

#[test]
fn test_peisar_ast_take_ast() {
    use super::PeisarAst;
    let mut ast = PeisarAst::new("# T\n\nbody\n".to_string(), None);
    let doc = ast.take_ast();
    assert_eq!(doc.children.len(), 2);
    // After taking, the ast should have a default Document
    assert_eq!(ast.get_ast().children.len(), 0);
}

#[test]
fn test_peisar_ast_frontmatter() {
    use super::PeisarAst;
    let md = "---\ntitle: Test\n---\n\n# Hello\n";
    let mut ast = PeisarAst::new(md.to_string(), None);
    assert_eq!(
        ast.get_frontmatter()
            .and_then(|value| value["title"].as_str().map(str::to_owned)),
        Some("Test".to_string())
    );
    assert!(
        ast.get_ast()
            .children
            .iter()
            .any(|b| matches!(b, Block::Heading { level: 1, .. }))
    );
}

struct ReplacingVisitor {
    calls: usize,
}

impl AstVisitor for ReplacingVisitor {
    fn visit_block(&mut self, block: &mut Block) -> VisitControl {
        self.calls += 1;
        VisitControl::replace_with(vec![block.clone()])
    }
}

#[test]
fn replacing_a_node_does_not_revisit_its_replacement() {
    let mut doc = md_to_ast("# Heading\n", &AstOptions::default(), None);
    let mut visitor = ReplacingVisitor { calls: 0 };

    visit_document_mut(&mut doc, &mut visitor);

    assert_eq!(visitor.calls, 1);
    assert!(matches!(doc.children.as_slice(), [Block::Heading { .. }]));
}

// ---------------------------------------------------------------------------
// Custom parser hooks
// ---------------------------------------------------------------------------

/// Hook that converts `:::name ... :::` container blocks into HtmlBlock
/// nodes (consuming all lines between the markers).
struct DirectiveHook;

impl AstParser for DirectiveHook {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        let name = ctx.line.trim().strip_prefix(":::")?;
        if name.is_empty() {
            return None;
        }
        // Find the closing `:::` line.
        let close = ctx.lines.iter().skip(1).position(|l| l.trim() == ":::")?;
        let html = format!("<div class=\"directive\" data-name=\"{}\">", name);
        Some((
            Block::HtmlBlock {
                html,
                pos: Default::default(),
                attrs: None,
            },
            close + 2,
        ))
    }
}

#[test]
fn block_hook_parses_custom_directive_syntax() {
    let hook = DirectiveHook;
    let hooks = ParseHooks::empty().with(&hook);
    let doc = super::parsers::md_to_ast_with_hooks(
        ":::note\ncontent stays\n:::\n\n# After\n",
        &AstOptions::default(),
        None,
        &hooks,
    );

    assert_eq!(doc.children.len(), 2);
    match &doc.children[0] {
        Block::HtmlBlock { html, pos, .. } => {
            assert_eq!(html, "<div class=\"directive\" data-name=\"note\">");
            // Span covers the directive lines [0, 3).
            assert_eq!(pos.start.line, 0);
            assert_eq!(pos.end.line, 3);
        }
        other => panic!("expected HtmlBlock, got {:?}", other),
    }
    assert!(matches!(doc.children[1], Block::Heading { level: 1, .. }));
}

/// Hook that overrides a built-in construct: turns `!` lines into
/// ThematicBreak nodes instead of paragraphs.
struct ExclamationHook;

impl AstParser for ExclamationHook {
    fn try_parse_block(&self, ctx: &BlockParseContext) -> Option<(Block, usize)> {
        if ctx.line.trim() != "!" {
            return None;
        }
        Some((
            Block::ThematicBreak {
                pos: Default::default(),
            },
            1,
        ))
    }
}

#[test]
fn block_hook_runs_before_builtin_matchers() {
    let hook = ExclamationHook;
    let hooks = ParseHooks::empty().with(&hook);
    // A lone `!` line would normally parse as a paragraph; the hook must
    // claim it first.
    let doc = super::parsers::md_to_ast_with_hooks("!\n", &AstOptions::default(), None, &hooks);

    assert!(matches!(doc.children[0], Block::ThematicBreak { .. }));
}

/// Inline hook that parses `[[wikilink]]` into a link node.
struct WikiLinkHook;

impl AstParser for WikiLinkHook {
    fn try_parse_inline(&self, ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        let rest = ctx.rest.strip_prefix("[[")?;
        let close = rest.find("]]")?;
        let target = &rest[..close];
        let url = format!("https://wiki.example.com/{}", target.replace(' ', "_"));
        Some((
            Inline::Link {
                text: vec![Inline::Text {
                    value: target.to_string(),
                    pos: Default::default(),
                }],
                url,
                title: None,
                autolink: false,
                pos: Default::default(),
            },
            close + 4,
        ))
    }
}

#[test]
fn inline_hook_parses_wikilinks() {
    let hook = WikiLinkHook;
    let hooks = ParseHooks::empty().with(&hook);
    let doc = super::parsers::md_to_ast_with_hooks(
        "See [[Some Page]] here.\n",
        &AstOptions::default(),
        None,
        &hooks,
    );

    match &doc.children[0] {
        Block::Paragraph { children, .. } => {
            assert_eq!(children.len(), 3);
            match &children[1] {
                Inline::Link { text, url, pos, .. } => {
                    assert!(matches!(
                        text.as_slice(),
                        [Inline::Text { value, .. }] if value == "Some Page"
                    ));
                    assert_eq!(url, "https://wiki.example.com/Some_Page");
                    // Span computed by the engine from `consumed`
                    // (`[[Some Page]]` = 13 chars starting at column 4).
                    assert_eq!(pos.start.line, 0);
                    assert_eq!(pos.start.column, 4);
                    assert_eq!(pos.end.column, 17);
                }
                other => panic!("expected Link, got {:?}", other),
            }
        }
        other => panic!("expected Paragraph, got {:?}", other),
    }
}

#[test]
fn inline_hook_applies_inside_emphasis_and_block_quotes() {
    let hook = WikiLinkHook;
    let hooks = ParseHooks::empty().with(&hook);
    let doc = super::parsers::md_to_ast_with_hooks(
        "> quote with **[[Bold Link]]**\n",
        &AstOptions::default(),
        None,
        &hooks,
    );

    match &doc.children[0] {
        Block::BlockQuote { children, .. } => match &children[0] {
            Block::Paragraph { children, .. } => {
                // Children: Text("quote with "), Emphasis(**[[Bold Link]]**)
                match &children[1] {
                    Inline::Emphasis { children, .. } => {
                        assert!(matches!(
                            children.as_slice(),
                            [Inline::Link { url, .. }] if url == "https://wiki.example.com/Bold_Link"
                        ));
                    }
                    other => panic!("expected Emphasis, got {:?}", other),
                }
            }
            other => panic!("expected Paragraph, got {:?}", other),
        },
        other => panic!("expected BlockQuote, got {:?}", other),
    }
}

/// First hook always declines; the second one must still get a chance.
struct DecliningHook;
impl AstParser for DecliningHook {
    fn try_parse_block(&self, _ctx: &BlockParseContext) -> Option<(Block, usize)> {
        None
    }
    fn try_parse_inline(&self, _ctx: &InlineParseContext) -> Option<(Inline, usize)> {
        None
    }
}

#[test]
fn multiple_hooks_tried_in_registration_order() {
    let declining = DecliningHook;
    let wikilink = WikiLinkHook;
    let hooks = ParseHooks::empty().with(&declining).with(&wikilink);
    let doc =
        super::parsers::md_to_ast_with_hooks("plain text\n", &AstOptions::default(), None, &hooks);
    // Declining hooks must not disturb built-in parsing.
    assert!(matches!(doc.children[0], Block::Paragraph { .. }));

    let hooks = ParseHooks::empty().with(&wikilink);
    let doc =
        super::parsers::md_to_ast_with_hooks("[[Link]]\n", &AstOptions::default(), None, &hooks);
    assert!(matches!(
        &doc.children[0],
        Block::Paragraph { children, .. } if matches!(children.as_slice(),
            [Inline::Link { .. }])
    ));
}

#[test]
fn hooks_do_not_run_when_none_registered() {
    // `[[wikilink]]` is plain text without a hook.
    let doc = md_to_ast("See [[Some Page]] here.\n", &AstOptions::default(), None);
    match &doc.children[0] {
        Block::Paragraph { children, .. } => {
            assert!(children.iter().all(|c| matches!(c, Inline::Text { .. })));
        }
        other => panic!("expected Paragraph, got {:?}", other),
    }
}
