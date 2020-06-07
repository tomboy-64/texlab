use crate::features::prelude::*;

pub fn hover_fields(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    let tree = ctx.current().content.as_bibtex()?;
    let pos = ctx.params.text_document_position_params.position;
    let name = tree
        .find(pos)
        .into_iter()
        .filter_map(|node| tree.as_field(node))
        .map(|field| &field.name)
        .find(|name| name.range().contains(pos))?;

    let docs = LANGUAGE_DATA.field_documentation(name.text())?;
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: docs.into(),
        }),
        range: Some(name.range()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn known_field() {
        let actual_hover = hover_fields(
            &FeatureTester::builder()
                .files(vec![("main.bib", "@article{foo, author = bar}")])
                .main("main.bib")
                .line(0)
                .character(15)
                .build()
                .hover(),
        );

        let expected_hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: LANGUAGE_DATA.field_documentation("author").unwrap().into(),
            }),
            range: Some(Range::new_simple(0, 14, 0, 20)),
        };
        assert_eq!(actual_hover.unwrap(), expected_hover);
    }

    #[test]
    fn unknown_field() {
        let actual_hover = hover_fields(
            &FeatureTester::builder()
                .files(vec![("main.bib", "@article{foo, bar = baz}")])
                .main("main.bib")
                .line(0)
                .character(15)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_fields(
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
        let actual_hover = hover_fields(
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
