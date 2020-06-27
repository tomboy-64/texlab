use crate::features::{
    completion::{
        types::{Item, ItemData},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_theorem_environments(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            if let Some((range, _)) = scope.match_environment(&table.tree, pos) {
                for table in ctx
                    .inner
                    .related()
                    .into_iter()
                    .filter_map(|doc| doc.content.as_latex())
                {
                    for theorem in &table.theorem_definitions {
                        let name = theorem.name(&table).text();
                        let data = ItemData::UserEnvironment { name };
                        let item = Item::new(range, data);
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
    fn inside_begin() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \newtheorem{theorem}{Theorem}
                        \begin{th}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(8)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_theorem_environments(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "theorem");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 7, 1, 9));
    }

    #[test]
    fn outside_begin() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \newtheorem{theorem}{Theorem}
                        \begin{th}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(10)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_theorem_environments(&mut ctx);

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

        complete_theorem_environments(&mut ctx);

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

        complete_theorem_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
