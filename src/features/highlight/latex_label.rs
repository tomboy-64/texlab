use crate::{
    features::{highlight::HighlightContext, prelude::*},
    syntax::LatexLabelKind,
};

pub fn highlight_latex_labels(ctx: &mut HighlightContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position_params.position;
        if let Some(name) = table
            .labels
            .iter()
            .flat_map(|label| label.names(&table))
            .find(|label| label.range().contains(pos))
            .map(latex::Token::text)
        {
            for label_group in &table.labels {
                for label in label_group.names(&table) {
                    if label.text() == name {
                        let kind = match label_group.kind {
                            LatexLabelKind::Definition => DocumentHighlightKind::Write,
                            LatexLabelKind::Reference(_) => DocumentHighlightKind::Read,
                        };

                        let item = DocumentHighlight {
                            range: label.range(),
                            kind: Some(kind),
                        };
                        ctx.items.push(item);
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
    fn has_label() {
        let mut ctx = HighlightContext::new(
            FeatureTester::builder()
                .files(vec![(
                    "main.tex",
                    indoc!(
                        r#"
                            \label{foo}
                            \ref{foo}
                        "#
                    ),
                )])
                .main("main.tex")
                .line(0)
                .character(7)
                .build()
                .highlight(),
        );

        highlight_latex_labels(&mut ctx);

        let expected_items = vec![
            DocumentHighlight {
                range: Range::new_simple(0, 7, 0, 10),
                kind: Some(DocumentHighlightKind::Write),
            },
            DocumentHighlight {
                range: Range::new_simple(1, 5, 1, 8),
                kind: Some(DocumentHighlightKind::Read),
            },
        ];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = HighlightContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .highlight(),
        );

        highlight_latex_labels(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = HighlightContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .highlight(),
        );

        highlight_latex_labels(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
