use crate::features::{
    completion::{
        types::{Item, ItemData, LatexArgumentPattern},
        CompletionContext,
    },
    prelude::*,
};

const MODEL_NAMES: &[&str] = &["gray", "rgb", "RGB", "HTML", "cmyk"];

pub fn complete_color_models(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in &LANGUAGE_DATA.color_model_commands {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    for name in MODEL_NAMES {
                        let item = Item::new(range, ItemData::ColorModel { name });
                        ctx.items.push(item);
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn inside_define_color() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\definecolor{name}{}"#)])
            .main("main.tex")
            .line(0)
            .character(19)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_color_models(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 19, 0, 19));
    }

    #[test]
    fn inside_define_color_set() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\definecolorset{}"#)])
            .main("main.tex")
            .line(0)
            .character(16)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_color_models(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 16, 0, 16));
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

        complete_color_models(&mut ctx);

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

        complete_color_models(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
