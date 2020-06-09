use crate::features::{definition::DefinitionContext, prelude::*};

pub fn goto_string_definition(ctx: &mut DefinitionContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position_params.position;
        if let Some(reference) = find_reference(tree, pos) {
            let uri = &ctx.inner.current().uri;
            for node in tree.children(tree.root) {
                if let Some(string) = tree.as_string(node) {
                    if let Some(name) = &string.name {
                        if name.text() == reference.text() {
                            ctx.items.push(LocationLink {
                                origin_selection_range: Some(reference.range()),
                                target_uri: uri.clone().into(),
                                target_range: string.range(),
                                target_selection_range: name.range(),
                            });
                        }
                    }
                }
            }
        }
    }
}

fn find_reference(tree: &bibtex::Tree, pos: Position) -> Option<&bibtex::Token> {
    let mut nodes = tree.find(pos);
    nodes.reverse();
    match (
        &tree.graph[nodes[0]],
        nodes.get(1).map(|node| &tree.graph[*node]),
    ) {
        (bibtex::Node::Word(word), Some(bibtex::Node::Field(_)))
        | (bibtex::Node::Word(word), Some(bibtex::Node::Concat(_))) => Some(&word.token),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn simple() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @string{foo = {bar}}
                        @article{bar, author = foo}
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(24)
            .build();
        let target_uri = tester.uri("main.bib");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_string_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 23, 1, 26)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 0, 0, 20),
            target_selection_range: Range::new_simple(0, 8, 0, 11),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn concat() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                    @string{foo = {bar}}
                    @article{bar, author = foo # "bar"}
                "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(24)
            .build();
        let target_uri = tester.uri("main.bib");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_string_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 23, 1, 26)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 0, 0, 20),
            target_selection_range: Range::new_simple(0, 8, 0, 11),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn field() {
        let mut ctx = DefinitionContext::new(
            FeatureTester::builder()
                .files(vec![(
                    "main.bib",
                    indoc!(
                        r#"
                            @string{foo = {bar}}
                            @article{bar, author = foo}
                        "#
                    ),
                )])
                .main("main.bib")
                .line(1)
                .character(18)
                .build()
                .definition(),
        );

        goto_string_definition(&mut ctx);

        assert!(ctx.items.is_empty());
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

        goto_string_definition(&mut ctx);

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

        goto_string_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
