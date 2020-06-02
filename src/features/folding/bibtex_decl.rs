use crate::features::{folding::FoldingContext, prelude::*};

pub fn fold_bibtex_decls(ctx: &mut FoldingContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        for decl in tree.children(tree.root) {
            if let Some(item) = make_item(tree, decl) {
                ctx.items.push(item);
            }
        }
    }
}

fn make_item(tree: &bibtex::Tree, decl: NodeIndex) -> Option<FoldingRange> {
    let (ty, right) = match &tree.graph[decl] {
        bibtex::Node::Preamble(preamble) => (Some(&preamble.ty), preamble.right.as_ref()),
        bibtex::Node::String(string) => (Some(&string.ty), string.right.as_ref()),
        bibtex::Node::Entry(entry) => (Some(&entry.ty), entry.right.as_ref()),
        bibtex::Node::Root(_)
        | bibtex::Node::Comment(_)
        | bibtex::Node::Field(_)
        | bibtex::Node::Word(_)
        | bibtex::Node::Command(_)
        | bibtex::Node::QuotedContent(_)
        | bibtex::Node::BracedContent(_)
        | bibtex::Node::Concat(_) => (None, None),
    };

    Some(FoldingRange {
        start_line: ty?.start().line,
        start_character: Some(ty?.start().character),
        end_line: right?.end().line,
        end_character: Some(right?.end().character),
        kind: Some(FoldingRangeKind::Region),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn preamble() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@preamble{"foo"}"#)])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        let expected_items = vec![FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 0,
            end_character: Some(16),
            kind: Some(FoldingRangeKind::Region),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn string() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@string{foo = "bar"}"#)])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        let expected_items = vec![FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 0,
            end_character: Some(20),
            kind: Some(FoldingRangeKind::Region),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn entry() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![(
                    "main.bib",
                    indoc!(
                        r#"
                            @article{foo, 
                                bar = baz
                            }
                        "#
                    ),
                )])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        let expected_items = vec![FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 2,
            end_character: Some(1),
            kind: Some(FoldingRangeKind::Region),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn comment() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "foo")])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn entry_invalid() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "@article{foo,")])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .build()
                .folding(),
        );

        fold_bibtex_decls(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
