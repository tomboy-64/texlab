use crate::{
    features::{
        outline::{Outline, OutlineContext},
        prelude::*,
    },
    syntax::LatexLabelKind,
};
use std::sync::Arc;

pub fn hover_labels(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    let table = ctx.current().content.as_latex()?;
    let pos = ctx.params.text_document_position_params.position;
    let reference = find_reference(table, pos)?;
    let (doc, def) = find_definition(&ctx.view, reference)?;

    let snapshot = Arc::clone(&ctx.view.snapshot);
    let view = DocumentView::analyze(snapshot, doc, &ctx.options, &ctx.current_dir);
    let outline = Outline::analyze(&view, &ctx.options, &ctx.current_dir);
    let outline_ctx = OutlineContext::parse(&view, &outline, def)?;
    let markup = outline_ctx.documentation();
    Some(Hover {
        contents: HoverContents::Markup(markup),
        range: Some(reference.range()),
    })
}

fn find_reference(table: &latex::SymbolTable, pos: Position) -> Option<&latex::Token> {
    for label in &table.labels {
        let names = label.names(&table);
        if names.len() == 1 && table[label.parent].range().contains(pos) {
            return Some(&label.names(&table)[0]);
        }

        for name in &names {
            if name.range().contains(pos) {
                return Some(name);
            }
        }
    }
    None
}

fn find_definition(
    view: &DocumentView,
    reference: &latex::Token,
) -> Option<(Arc<Document>, latex::Label)> {
    for doc in &view.related {
        if let DocumentContent::Latex(table) = &doc.content {
            for label in &table.labels {
                if label.kind == LatexLabelKind::Definition {
                    for name in label.names(&table) {
                        if name.text() == reference.text() {
                            return Some((Arc::clone(&doc), *label));
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn section() {
        let actual_hover = hover_labels(
            &FeatureTester::builder()
                .files(vec![("main.tex", r#"\section{Foo}\label{sec:foo}"#)])
                .main("main.tex")
                .line(0)
                .character(23)
                .build()
                .hover(),
        );

        assert_eq!(
            actual_hover.unwrap().range.unwrap(),
            Range::new_simple(0, 20, 0, 27)
        );
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_labels(
            &FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_bibtex_document() {
        let actual_hover = hover_labels(
            &FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }
}
