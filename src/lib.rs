mod ast;
mod config;
pub mod frontmatter;
mod html;
pub use config::PeisarOptions;
use napi::Env;
use napi_derive::napi;

use crate::html::{RenderOptions, render_document_html};
pub use ast::{
    Document, JsVisitor, PeisarAst,
    tokens::{Attributes, span, token},
    visitor,
};
use config::get_options;
use serde_json::Value;

#[napi]
pub struct Peisar {
    render_opts: RenderOptions,
    peisar_ast: PeisarAst,
}

#[napi]
impl Peisar {
    #[napi(constructor)]
    pub fn new(raw_md: String, options: Option<PeisarOptions>) -> Self {
        let opts = get_options(options);
        let ast_opts = opts.ast_opts;
        let render_opts = opts.render_opts;
        let peisar_ast = PeisarAst::new(raw_md, Some(ast_opts));
        Self {
            render_opts,
            peisar_ast,
        }
    }
    #[napi(getter)]
    pub fn ast(&mut self) -> Document {
        self.peisar_ast.get_ast()
    }
    #[napi]
    pub fn use_visitor(&mut self, env: Env, visitor: JsVisitor) {
        self.peisar_ast.add_visitor(env, visitor);
    }
    #[napi(getter)]
    pub fn html(&mut self) -> String {
        let doc = self.ast();
        let html = render_document_html(&doc, Some(self.render_opts.clone()));
        html
    }
    #[napi(getter)]
    pub fn frontmatter(&mut self) -> Option<Value> {
        self.peisar_ast.get_frontmatter()
    }
    #[napi(getter)]
    pub fn ast_json(&mut self) -> String {
        self.peisar_ast.ast_json()
    }
}
