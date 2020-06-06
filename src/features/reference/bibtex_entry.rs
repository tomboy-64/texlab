use crate::features::{prelude::*, reference::ReferenceContext};

pub fn find_bibtex_entry_references(ctx: &mut ReferenceContext) {
    if let Some(key) = find_key(ctx) {
        for doc in ctx.inner.related() {
            match &doc.content {
                DocumentContent::Latex(table) => {
                    for item in table
                        .citations
                        .iter()
                        .flat_map(|citation| citation.keys(&table))
                        .filter(|citation| citation.text() == key)
                        .map(|citation| Location::new(doc.uri.clone().into(), citation.range()))
                    {
                        ctx.items.push(item);
                    }
                }
                DocumentContent::Bibtex(tree) => {
                    if ctx.inner.params.context.include_declaration {
                        let uri: Url = doc.uri.clone().into();
                        for item in tree
                            .children(tree.root)
                            .filter_map(|node| tree.as_entry(node))
                            .filter_map(|entry| entry.key.as_ref())
                            .filter(|key_tok| key_tok.text() == key)
                            .map(|key_tok| Location::new(uri.clone(), key_tok.range()))
                        {
                            ctx.items.push(item);
                        }
                    }
                }
            }
        }
    }
}

fn find_key(ctx: &ReferenceContext) -> Option<String> {
    let pos = ctx.inner.params.text_document_position.position;
    match &ctx.inner.current().content {
        DocumentContent::Latex(table) => table
            .citations
            .iter()
            .flat_map(|citation| citation.keys(&table))
            .find(|key| key.range().contains(pos))
            .map(|key| key.text().into()),
        DocumentContent::Bibtex(tree) => tree
            .children(tree.root)
            .filter_map(|node| tree.as_entry(node))
            .filter_map(|entry| entry.key.as_ref())
            .find(|key| key.range().contains(pos))
            .map(|key| key.text().into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn entry() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.bib", r#"@article{foo, bar = {baz}}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \addbibresource{foo.bib}
                            \cite{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\cite{foo}"#),
            ])
            .main("foo.bib")
            .line(0)
            .character(9)
            .build();
        let uri = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_entry_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(1, 6, 1, 9))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn entry_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.bib", r#"@article{foo, bar = {baz}}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \addbibresource{foo.bib}
                            \cite{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\cite{foo}"#),
            ])
            .main("foo.bib")
            .line(0)
            .character(9)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("foo.bib");
        let uri2 = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_entry_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri1.into(), Range::new_simple(0, 9, 0, 12)),
            Location::new(uri2.into(), Range::new_simple(1, 6, 1, 9)),
        ];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn citation() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.bib", r#"@article{foo, bar = {baz}}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \addbibresource{foo.bib}
                            \cite{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\cite{foo}"#),
            ])
            .main("bar.tex")
            .line(1)
            .character(8)
            .build();
        let uri = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_entry_references(&mut ctx);

        let expected_items = vec![Location::new(uri.into(), Range::new_simple(1, 6, 1, 9))];
        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn citation_include_declaration() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.bib", r#"@article{foo, bar = {baz}}"#),
                (
                    "bar.tex",
                    indoc!(
                        r#"
                            \addbibresource{foo.bib}
                            \cite{foo}
                        "#
                    ),
                ),
                ("baz.tex", r#"\cite{foo}"#),
            ])
            .main("bar.tex")
            .line(1)
            .character(8)
            .include_declaration(true)
            .build();
        let uri1 = tester.uri("foo.bib");
        let uri2 = tester.uri("bar.tex");
        let mut ctx = ReferenceContext::new(tester.reference());

        find_bibtex_entry_references(&mut ctx);

        let expected_items = vec![
            Location::new(uri2.into(), Range::new_simple(1, 6, 1, 9)),
            Location::new(uri1.into(), Range::new_simple(0, 9, 0, 12)),
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

        find_bibtex_entry_references(&mut ctx);

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

        find_bibtex_entry_references(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
