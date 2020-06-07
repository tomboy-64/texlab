use crate::features::prelude::*;

pub fn hover_entry_types(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    let tree = ctx.current().content.as_bibtex()?;
    for entry in tree
        .children(tree.root)
        .filter_map(|node| tree.as_entry(node))
    {
        if entry
            .ty
            .range()
            .contains(ctx.params.text_document_position_params.position)
        {
            let ty = &entry.ty.text()[1..];
            let docs = LANGUAGE_DATA.entry_type_documentation(ty)?;
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: docs.into(),
                }),
                range: Some(entry.ty.range()),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn known_entry_type() {
        let actual_hover = hover_entry_types(
            &FeatureTester::builder()
                .files(vec![("main.bib", "@article{foo,}")])
                .main("main.bib")
                .line(0)
                .character(3)
                .build()
                .hover(),
        );

        let expected_hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: LANGUAGE_DATA
                    .entry_type_documentation("article")
                    .unwrap()
                    .into(),
            }),
            range: Some(Range::new_simple(0, 0, 0, 8)),
        };
        assert_eq!(actual_hover.unwrap(), expected_hover);
    }

    #[test]
    fn unknown_entry_type() {
        let actual_hover = hover_entry_types(
            &FeatureTester::builder()
                .files(vec![("main.tex", "@foo{bar,}")])
                .main("main.tex")
                .line(0)
                .character(3)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn entry_key() {
        let actual_hover = hover_entry_types(
            &FeatureTester::builder()
                .files(vec![("main.bib", "@article{foo,}")])
                .main("main.bib")
                .line(0)
                .character(11)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_entry_types(
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
        let actual_hover = hover_entry_types(
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
