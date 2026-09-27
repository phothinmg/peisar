mod config;
pub use config::PeisarOptions;
use napi_derive::napi;
use peisar_ast::{AstOptions, BlockCallback, Document, InlineCallback, JsVisitor, PeisarAstJs};
use peisar_html::{RenderOptions, render_document_html};

#[napi]
pub struct Peisar {
    ast: Document,
    visitors: Vec<JsVisitor>,
    frontmatter: Option<String>,
    render_opts: RenderOptions,
    peisar_ast: PeisarAstJs,
}

impl Peisar {
    pub fn new(raw_md: String, options: Option<PeisarOptions>) {
        let opts = get_options(options);
        let ast_opts = opts.ast_opts;
        let render_opts = opts.render_opts;
        let mut peisar_ast = PeisarAstJs::new(raw_md, Some(ast_opts));
        let frontmatter = peisar_ast.get_frontmatter();
        let ast = peisar_ast.ast();
    }
}

struct GetOptions {
    pub ast_opts: AstOptions,
    pub render_opts: RenderOptions,
}

fn get_options(options: Option<PeisarOptions>) -> GetOptions {
    let opts = options.unwrap_or(PeisarOptions::default());
    let ast_opts = AstOptions {
        gfm: opts.gfm.unwrap(),
        kramdown: opts.kramdown.unwrap(),
        file_name: opts.file_name,
    };
    let render_opts = RenderOptions {
        fragment: opts.fragment.unwrap(),
        charset: opts.charset.unwrap(),
        viewport: opts.viewport.unwrap(),
        title: opts.title,
        body_class: opts.body_class,
        style: opts.style,
    };
    GetOptions {
        ast_opts,
        render_opts,
    }
}
