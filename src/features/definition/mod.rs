mod cmd;
mod entry;
mod label;
mod string;

use self::{
    cmd::goto_command_definition, entry::goto_entry_definition, label::goto_label_definition,
    string::goto_string_definition,
};
use crate::features::prelude::*;

#[derive(Debug, Clone)]
pub struct DefinitionContext {
    inner: FeatureContext<GotoDefinitionParams>,
    items: Vec<LocationLink>,
}

impl DefinitionContext {
    pub fn new(inner: FeatureContext<GotoDefinitionParams>) -> Self {
        Self {
            inner,
            items: Vec::new(),
        }
    }
}

pub fn goto_definition(ctx: FeatureContext<GotoDefinitionParams>) -> GotoDefinitionResponse {
    let mut ctx = DefinitionContext::new(ctx);
    goto_string_definition(&mut ctx);
    goto_entry_definition(&mut ctx);
    goto_command_definition(&mut ctx);
    goto_label_definition(&mut ctx);
    if ctx.inner.client_capabilities.has_definition_link_support() {
        GotoDefinitionResponse::Link(ctx.items)
    } else {
        GotoDefinitionResponse::Array(
            ctx.items
                .into_iter()
                .map(|link| Location::new(link.target_uri, link.target_selection_range))
                .collect(),
        )
    }
}
