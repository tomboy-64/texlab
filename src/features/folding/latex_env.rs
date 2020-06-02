use crate::features::{folding::FoldingContext, prelude::*};

pub fn fold_latex_envs(ctx: &mut FoldingContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for env in &table.environments {
            let left_node = &table[env.left.parent];
            let right_node = &table[env.right.parent];
            let item = FoldingRange {
                start_line: left_node.end().line,
                start_character: Some(left_node.end().character),
                end_line: right_node.start().line,
                end_character: Some(right_node.start().character),
                kind: Some(FoldingRangeKind::Region),
            };
            ctx.items.push(item);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn multiline() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![(
                    "main.tex",
                    indoc!(
                        r#"
                        \begin{foo}
                        \end{foo}
                    "#
                    ),
                )])
                .main("main.tex")
                .build()
                .folding(),
        );

        fold_latex_envs(&mut ctx);

        let expected_items = vec![FoldingRange {
            start_line: 0,
            start_character: Some(11),
            end_line: 1,
            end_character: Some(0),
            kind: Some(FoldingRangeKind::Region),
        }];

        assert_eq!(ctx.items, expected_items);
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

        fold_latex_envs(&mut ctx);

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

        fold_latex_envs(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
