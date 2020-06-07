use crate::features::prelude::*;

pub fn hover_string_references(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    let tree = ctx.current().content.as_bibtex()?;
    let pos = ctx.params.text_document_position_params.position;
    let reference = find_reference(tree, pos)?;
    for string_node in tree.children(tree.root) {
        let hover = find_definition(tree, string_node, reference);
        if hover.is_some() {
            return hover;
        }
    }
    None
}

fn find_reference(tree: &bibtex::Tree, pos: Position) -> Option<&bibtex::Token> {
    let mut results = tree.find(pos);
    results.reverse();
    match (
        &tree.graph[results[0]],
        results.get(1).map(|node| &tree.graph[*node]),
    ) {
        (bibtex::Node::Word(reference), Some(bibtex::Node::Concat(_))) => Some(&reference.token),
        (bibtex::Node::Word(reference), Some(bibtex::Node::Field(_))) => Some(&reference.token),
        _ => None,
    }
}

fn find_definition(
    tree: &bibtex::Tree,
    string_node: NodeIndex,
    reference: &bibtex::Token,
) -> Option<Hover> {
    let string = tree.as_string(string_node)?;
    if string.name.as_ref()?.text() != reference.text() {
        return None;
    }

    let options = BibtexFormattingOptions {
        line_length: None,
        formatter: None,
    };
    let text = bibtex::format(
        tree,
        tree.children(string_node).next()?,
        bibtex::FormattingParams {
            tab_size: 4,
            insert_spaces: true,
            options: &options,
        },
    );
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: text,
        }),
        range: Some(reference.range()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn inside_reference() {
        let actual_hover = hover_string_references(
            &FeatureTester::builder()
                .files(vec![(
                    "main.bib",
                    indoc!(
                        r#"
                            @string{foo = "Foo"}
                            @string{bar = "Bar"}
                            @article{baz, author = bar}
                        "#
                    ),
                )])
                .main("main.bib")
                .line(2)
                .character(24)
                .build()
                .hover(),
        );

        let expected_hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: "\"Bar\"".into(),
            }),
            range: Some(Range::new_simple(2, 23, 2, 26)),
        };
        assert_eq!(actual_hover.unwrap(), expected_hover);
    }

    #[test]
    fn unknown_field() {
        let actual_hover = hover_string_references(
            &FeatureTester::builder()
                .files(vec![(
                    "main.bib",
                    indoc!(
                        r#"
                            @string{foo = "Foo"}
                            @string{bar = "Bar"}
                            @article{baz, author = bar}
                        "#
                    ),
                )])
                .main("main.bib")
                .line(2)
                .character(20)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_string_references(
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
        let actual_hover = hover_string_references(
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
