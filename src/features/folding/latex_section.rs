use crate::features::{folding::FoldingContext, prelude::*};

pub fn fold_latex_sections(ctx: &mut FoldingContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let sections = &table.sections;
        for i in 0..sections.len() {
            let current = &sections[i];
            if let Some(next) = sections
                .iter()
                .skip(i + 1)
                .find(|sec| current.level >= sec.level)
            {
                let next_node = &table[next.parent];
                if next_node.start().line > 0 {
                    let current_node = &table[current.parent];
                    let item = FoldingRange {
                        start_line: current_node.end().line,
                        start_character: Some(current_node.end().character),
                        end_line: next_node.start().line - 1,
                        end_character: Some(0),
                        kind: Some(FoldingRangeKind::Region),
                    };
                    ctx.items.push(item);
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
    fn nested() {
        let mut ctx = FoldingContext::new(
            FeatureTester::builder()
                .files(vec![(
                    "main.tex",
                    indoc!(
                        r#"
                            \section{Foo}
                            foo
                            \subsection{Bar}
                            bar
                            \section{Baz}
                            baz
                            \section{Qux}
                        "#
                    ),
                )])
                .main("main.tex")
                .build()
                .folding(),
        );

        fold_latex_sections(&mut ctx);

        let expected_items = vec![
            FoldingRange {
                start_line: 0,
                start_character: Some(13),
                end_line: 3,
                end_character: Some(0),
                kind: Some(FoldingRangeKind::Region),
            },
            FoldingRange {
                start_line: 2,
                start_character: Some(16),
                end_line: 3,
                end_character: Some(0),
                kind: Some(FoldingRangeKind::Region),
            },
            FoldingRange {
                start_line: 4,
                start_character: Some(13),
                end_line: 5,
                end_character: Some(0),
                kind: Some(FoldingRangeKind::Region),
            },
        ];

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

        fold_latex_sections(&mut ctx);

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

        fold_latex_sections(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
