mod bibtex_decl;
mod latex_env;
mod latex_section;

use self::{
    bibtex_decl::fold_bibtex_decls, latex_env::fold_latex_envs, latex_section::fold_latex_sections,
};
use crate::features::prelude::*;

#[derive(Debug, Clone)]
pub struct FoldingContext {
    pub inner: FeatureContext<FoldingRangeParams>,
    pub items: Vec<FoldingRange>,
}

impl FoldingContext {
    pub fn new(inner: FeatureContext<FoldingRangeParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

pub fn fold(ctx: FeatureContext<FoldingRangeParams>) -> Vec<FoldingRange> {
    let mut ctx = FoldingContext::new(ctx);
    fold_bibtex_decls(&mut ctx);
    fold_latex_envs(&mut ctx);
    fold_latex_sections(&mut ctx);
    ctx.items
}
