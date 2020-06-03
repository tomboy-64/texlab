mod latex_label;

use self::latex_label::highlight_latex_labels;
use crate::features::prelude::*;

#[derive(Debug, Clone)]
pub struct HighlightContext {
    inner: FeatureContext<DocumentHighlightParams>,
    items: Vec<DocumentHighlight>,
}

impl HighlightContext {
    pub fn new(inner: FeatureContext<DocumentHighlightParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

pub fn highlight(ctx: FeatureContext<DocumentHighlightParams>) -> Vec<DocumentHighlight> {
    let mut ctx = HighlightContext::new(ctx);
    highlight_latex_labels(&mut ctx);
    ctx.items
}
