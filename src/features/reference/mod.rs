mod bibtex_entry;
mod bibtex_string;
mod latex_label;

use self::{
    bibtex_entry::find_bibtex_entry_references, bibtex_string::find_bibtex_string_references,
    latex_label::find_latex_label_references,
};
use crate::features::prelude::*;

#[derive(Debug, Clone)]
pub struct ReferenceContext {
    pub inner: FeatureContext<ReferenceParams>,
    pub items: Vec<Location>,
}

impl ReferenceContext {
    pub fn new(inner: FeatureContext<ReferenceParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

pub fn find_all_references(ctx: FeatureContext<ReferenceParams>) -> Vec<Location> {
    let mut ctx = ReferenceContext::new(ctx);
    find_bibtex_entry_references(&mut ctx);
    find_bibtex_string_references(&mut ctx);
    find_latex_label_references(&mut ctx);
    ctx.items
}
