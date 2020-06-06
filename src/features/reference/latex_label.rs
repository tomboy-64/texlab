use crate::{
    features::{prelude::*, reference::ReferenceContext},
    syntax::LatexLabelKind,
};

pub fn find_latex_label_references(ctx: &mut ReferenceContext) {
    if let Some(def) = find_name(ctx).map(ToOwned::to_owned) {
        for doc in ctx.inner.related() {
            if let DocumentContent::Latex(table) = &doc.content {
                for label in &table.labels {
                    if is_included(ctx, *label) {
                        for name in label.names(&table) {
                            if name.text() == def {
                                let item = Location::new(doc.uri.clone().into(), name.range());
                                ctx.items.push(item);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn find_name(ctx: &ReferenceContext) -> Option<&str> {
    let pos = ctx.inner.params.text_document_position.position;
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        table
            .labels
            .iter()
            .flat_map(|label| label.names(&table))
            .find(|label| (*label).range().contains(pos))
            .map(latex::Token::text)
    } else {
        None
    }
}

fn is_included(ctx: &ReferenceContext, label: latex::Label) -> bool {
    match label.kind {
        LatexLabelKind::Reference(_) => true,
        LatexLabelKind::Definition => ctx.inner.params.context.include_declaration,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn definition() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \input{foo.tex}
                            \ref{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(8)
            .build();
        let uri = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_latex_label_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(1, 5, 1, 8))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn definition_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \input{foo.tex}
                            \ref{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(8)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("foo.tex");
        let uri2 = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_latex_label_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri1.into(), Range::new_simple(0, 7, 0, 10)),
            Location::new(uri2.into(), Range::new_simple(1, 5, 1, 8)),
        ];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn reference() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                        \input{foo.tex}
                        \ref{foo}
                    "#
                    ),
                ),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("bar.tex")
            .line(1)
            .character(7)
            .build();
        let uri = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_latex_label_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(1, 5, 1, 8))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn reference_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\label{foo}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \input{foo.tex}
                            \ref{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("bar.tex")
            .line(1)
            .character(7)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("foo.tex");
        let uri2 = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_latex_label_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri2.into(), Range::new_simple(1, 5, 1, 8)),
            Location::new(uri1.into(), Range::new_simple(0, 7, 0, 10)),
        ];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = ReferenceContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .reference(),
        );

        find_latex_label_references(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = ReferenceContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .reference(),
        );

        find_latex_label_references(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
