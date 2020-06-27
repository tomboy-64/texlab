use crate::features::{
    completion::{
        types::{Item, ItemData, LatexArgumentPattern},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_colors(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in &LANGUAGE_DATA.color_commands {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    for name in &LANGUAGE_DATA.colors {
                        let item = Item::new(range, ItemData::Color { name });
                        ctx.items.push(item);
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn inside_color() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\color{}"#)])
            .main("main.tex")
            .line(0)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_colors(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 7, 0, 7));
    }

    #[test]
    fn outside_color() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\color{}"#)])
            .main("main.tex")
            .line(0)
            .character(8)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_colors(&mut ctx);

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

        complete_colors(&mut ctx);

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

        complete_colors(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
