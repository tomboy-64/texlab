use crate::{citeproc::render_citation, features::prelude::*, syntax::Span};
use log::warn;

pub fn hover_citations(ctx: &FeatureContext<HoverParams>) -> Option<Hover> {
    let (tree, src_key, entry) = find_entry(ctx)?;
    if entry.is_comment() {
        None
    } else {
        let key = entry.key.as_ref()?;
        match render_citation(&tree, key.text()) {
            Some(markdown) => Some(Hover {
                contents: HoverContents::Markup(markdown),
                range: Some(src_key.range()),
            }),
            None => {
                warn!("Failed to render entry: {}", key.text());
                None
            }
        }
    }
}

fn find_entry(ctx: &FeatureContext<HoverParams>) -> Option<(&bibtex::Tree, &Span, &bibtex::Entry)> {
    let key = find_key(ctx)?;
    for tree in ctx
        .related()
        .iter()
        .filter_map(|doc| doc.content.as_bibtex())
    {
        for entry in tree
            .children(tree.root)
            .filter_map(|node| tree.as_entry(node))
        {
            if let Some(current_key) = &entry.key {
                if current_key.text() == key.text {
                    return Some((tree, key, entry));
                }
            }
        }
    }
    None
}

fn find_key(ctx: &FeatureContext<HoverParams>) -> Option<&Span> {
    let pos = ctx.params.text_document_position_params.position;
    match &ctx.current().content {
        DocumentContent::Latex(table) => table
            .citations
            .iter()
            .flat_map(|citation| citation.keys(&table))
            .find(|key| key.range().contains(pos))
            .map(|token| &token.span),
        DocumentContent::Bibtex(tree) => tree
            .children(tree.root)
            .filter_map(|node| tree.as_entry(node))
            .filter_map(|entry| entry.key.as_ref())
            .find(|key| key.range().contains(pos))
            .map(|token| &token.span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn inside_reference() {
        let actual_hover = hover_citations(
            &FeatureTester::builder()
                .files(vec![
                    (
                        "main.bib",
                        "@article{foo, author = {Foo Bar}, title = {Baz Qux}, year = 1337}",
                    ),
                    (
                        "main.tex",
                        indoc!(
                            r#"
                            \addbibresource{main.bib}
                            \cite{foo}
                        "#
                        ),
                    ),
                ])
                .main("main.tex")
                .line(1)
                .character(7)
                .build()
                .hover(),
        );

        let expected_hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "Bar, F. (1337). *Baz Qux*.".into(),
            }),
            range: Some(Range::new_simple(1, 6, 1, 9)),
        };
        assert_eq!(actual_hover.unwrap(), expected_hover);
    }

    #[test]
    fn inside_definition() {
        let actual_hover = hover_citations(
            &FeatureTester::builder()
                .files(vec![
                    (
                        "main.bib",
                        "@article{foo, author = {Foo Bar}, title = {Baz Qux}, year = 1337}",
                    ),
                    (
                        "main.tex",
                        indoc!(
                            r#"
                            \addbibresource{main.bib}
                            \cite{foo}
                        "#
                        ),
                    ),
                ])
                .main("main.bib")
                .line(0)
                .character(11)
                .build()
                .hover(),
        );

        let expected_hover = Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: "Bar, F. (1337). *Baz Qux*.".into(),
            }),
            range: Some(Range::new_simple(0, 9, 0, 12)),
        };
        assert_eq!(actual_hover.unwrap(), expected_hover);
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_citations(
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
        let actual_hover = hover_citations(
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
