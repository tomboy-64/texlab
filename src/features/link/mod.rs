mod latex_import;
mod latex_include;

use self::{latex_import::link_latex_imports, latex_include::link_latex_includes};
use crate::features::prelude::*;

#[derive(Debug, Clone)]
pub struct LinkContext {
    inner: FeatureContext<DocumentLinkParams>,
    items: Vec<DocumentLink>,
}

impl LinkContext {
    pub fn new(inner: FeatureContext<DocumentLinkParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

pub fn link(ctx: FeatureContext<DocumentLinkParams>) -> Vec<DocumentLink> {
    let mut ctx = LinkContext::new(ctx);
    link_latex_imports(&mut ctx);
    link_latex_includes(&mut ctx);
    ctx.items
}
