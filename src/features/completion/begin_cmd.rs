use crate::features::{
    completion::{
        types::{Item, ItemData, LatexCompletionScope},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_begin_command(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for scope in &ctx.scopes {
            if let LatexCompletionScope::Command(cmd_node) = scope {
                let cmd = table.as_command(*cmd_node).unwrap();
                let range = cmd.short_name_range();
                ctx.items.push(Item::new(range, ItemData::BeginCommand));
                break;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn after_backslash() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\"#)])
            .main("main.tex")
            .line(0)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_begin_command(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
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

        complete_begin_command(&mut ctx);

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

        complete_begin_command(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
