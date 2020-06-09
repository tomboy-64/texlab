use crate::features::{definition::DefinitionContext, prelude::*};

pub fn goto_entry_definition(ctx: &mut DefinitionContext) {
    if let Some(reference) = find_reference(&ctx.inner) {
        for doc in ctx.inner.related() {
            if let DocumentContent::Bibtex(tree) = &doc.content {
                for entry in tree
                    .children(tree.root)
                    .filter_map(|node| tree.as_entry(node))
                {
                    if let Some(key) = &entry.key {
                        if key.text() == reference.text() {
                            ctx.items.push(LocationLink {
                                origin_selection_range: Some(reference.range()),
                                target_uri: doc.uri.clone().into(),
                                target_range: entry.range(),
                                target_selection_range: key.range(),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn find_reference(ctx: &FeatureContext<GotoDefinitionParams>) -> Option<&latex::Token> {
    let pos = ctx.params.text_document_position_params.position;
    ctx.current().content.as_latex().and_then(|table| {
        table
            .citations
            .iter()
            .flat_map(|citation| citation.keys(&table))
            .find(|key| key.range().contains(pos))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn has_definition() {
        let tester = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                        \addbibresource{baz.bib}
                        \cite{foo}
                    "#
                    ),
                ),
                ("bar.bib", r#"@article{foo, bar = {baz}}"#),
                ("baz.bib", r#"@article{foo, bar = {baz}}"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(6)
            .build();
        let target_uri = tester.uri("baz.bib");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_entry_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 6, 1, 9)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 0, 0, 26),
            target_selection_range: Range::new_simple(0, 9, 0, 12),
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

        goto_entry_definition(&mut ctx);

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

        goto_entry_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
