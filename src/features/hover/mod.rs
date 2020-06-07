#[cfg(feature = "citation")]
mod citation;
mod component;
mod entry_type;
mod field;
mod label;
mod preview;
mod string_reference;

#[cfg(feature = "citation")]
use self::citation::hover_citations;
use self::{
    component::hover_components, entry_type::hover_entry_types, field::hover_fields,
    label::hover_labels, preview::hover_preview, string_reference::hover_string_references,
};
use crate::features::prelude::*;

pub async fn hover(ctx: FeatureContext<HoverParams>) -> Option<Hover> {
    let mut hover = hover_entry_types(&ctx)
        .or_else(|| hover_fields(&ctx))
        .or_else(|| hover_string_references(&ctx))
        .or_else(|| hover_components(&ctx))
        .or_else(|| hover_labels(&ctx));

    if cfg!(feature = "citation") {
        hover = hover.or_else(|| hover_citations(&ctx));
    }

    if hover.is_none() {
        hover = hover_preview(&ctx).await;
    }

    hover
}
