use crate::features::{
    completion::{
        types::{Item, ItemData},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_bibtex_commands(ctx: &mut CompletionContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        if let Some(cmd) = tree
            .find(pos)
            .into_iter()
            .last()
            .and_then(|node| tree.as_command(node))
        {
            if cmd.token.range().contains(pos) && cmd.token.start().character != pos.character {
                let mut range = cmd.range();
                range.start.character += 1;
                for cmd in &COMPONENT_DATABASE.kernel().commands {
                    let item = Item::new(
                        range,
                        ItemData::ComponentCommand {
                            name: &cmd.name,
                            image: cmd.image.as_deref(),
                            glyph: cmd.glyph.as_deref(),
                            file_names: &[],
                        },
                    );
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
    fn inside_comment() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", r#"\"#)])
            .main("main.bib")
            .line(0)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_bibtex_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn inside_command() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @article{foo, bar=
                        \}
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_bibtex_commands(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 1, 1, 2));
    }

    #[test]
    fn start_of_command() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @article{foo, bar=
                        \}
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_bibtex_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn inside_latex_command() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\"#)])
            .main("main.tex")
            .line(0)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_bibtex_commands(&mut ctx);

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

        complete_bibtex_commands(&mut ctx);

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

        complete_bibtex_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
