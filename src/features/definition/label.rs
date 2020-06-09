use crate::{
    features::{
        definition::DefinitionContext,
        outline::{Outline, OutlineContext, OutlineContextItem},
        prelude::*,
        symbol::build_section_tree,
    },
    syntax::LatexLabelKind,
};
use std::{path::Path, sync::Arc};

pub fn goto_label_definition(ctx: &mut DefinitionContext) {
    if let Some(reference) = find_reference(&ctx.inner) {
        for doc in ctx.inner.related() {
            let snapshot = Arc::clone(&ctx.inner.view.snapshot);
            let view = DocumentView::analyze(
                snapshot,
                Arc::clone(&doc),
                &ctx.inner.options,
                &ctx.inner.current_dir,
            );

            find_definitions(
                &view,
                &ctx.inner.options,
                &ctx.inner.current_dir,
                &reference,
                &mut ctx.items,
            );
        }
    }
}

fn find_reference(ctx: &FeatureContext<GotoDefinitionParams>) -> Option<&latex::Token> {
    if let DocumentContent::Latex(table) = &ctx.current().content {
        let pos = ctx.params.text_document_position_params.position;
        table
            .labels
            .iter()
            .flat_map(|label| label.names(&table))
            .find(|label| label.range().contains(pos))
    } else {
        None
    }
}

fn find_definitions(
    view: &DocumentView,
    options: &Options,
    current_dir: &Path,
    reference: &latex::Token,
    items: &mut Vec<LocationLink>,
) {
    if let DocumentContent::Latex(table) = &view.current.content {
        let outline = Outline::analyze(view, options, current_dir);
        let section_tree = build_section_tree(view, table, options, current_dir);
        for label in &table.labels {
            if label.kind == LatexLabelKind::Definition {
                let context = OutlineContext::parse(view, &outline, *label);
                for name in label.names(&table) {
                    if name.text() == reference.text() {
                        let target_range = if let Some(OutlineContextItem::Section { .. }) =
                            context.as_ref().map(|ctx| &ctx.item)
                        {
                            section_tree
                                .find(reference.text())
                                .map(|sec| sec.full_range)
                        } else {
                            context.as_ref().map(|ctx| ctx.range)
                        };

                        items.push(LocationLink {
                            origin_selection_range: Some(reference.range()),
                            target_uri: view.current.uri.clone().into(),
                            target_range: target_range
                                .unwrap_or_else(|| table[label.parent].range()),
                            target_selection_range: table[label.parent].range(),
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn unknown_context() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                        \begin{a}\begin{b}\label{foo}\end{b}\end{a}
                        \input{baz.tex}
                    "#
                    ),
                ),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("baz.tex")
            .line(0)
            .character(5)
            .build();
        let target_uri = tester.uri("bar.tex");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_label_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(0, 5, 0, 8)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 18, 0, 29),
            target_selection_range: Range::new_simple(0, 18, 0, 29),
        }];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = DefinitionContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .definition(),
        );

        goto_label_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = DefinitionContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .definition(),
        );

        goto_label_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
