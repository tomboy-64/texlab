use crate::features::{
    completion::{
        types::{Item, ItemData, LatexArgumentPattern},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_arguments(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for comp in &ctx.inner.view.components {
            for cmd in &comp.commands {
                for (i, param) in cmd.parameters.iter().enumerate() {
                    for scope in &ctx.scopes {
                        if let Some((range, _)) = scope.match_argument(
                            LatexArgumentPattern::builder()
                                .tree(&table.tree)
                                .name(&cmd.name)
                                .index(i)
                                .position(pos)
                                .build(),
                        ) {
                            for arg in &param.0 {
                                let item = Item::new(
                                    range,
                                    ItemData::Argument {
                                        name: &arg.name,
                                        image: arg.image.as_deref(),
                                    },
                                );
                                ctx.items.push(item);
                            }
                            return;
                        }
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
    fn inside_mathbb_empty() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                    \usepackage{amsfonts}
                    \mathbb{}
                "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(8)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_arguments(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 8, 1, 8));
    }

    #[test]
    fn inside_mathbb_non_empty() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \usepackage{amsfonts}
                        \mathbb{foo}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(8)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_arguments(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 8, 1, 11));
    }

    #[test]
    fn outside_mathbb_empty() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \usepackage{amsfonts}
                        \mathbb{}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(9)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_arguments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_arguments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_arguments(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
