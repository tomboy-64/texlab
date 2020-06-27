use crate::features::{
    completion::{
        types::{Item, ItemData, LatexCompletionScope},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_user_commands(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for scope in &ctx.scopes {
            if let LatexCompletionScope::Command(current_cmd_node) = scope {
                let current_cmd = table.as_command(*current_cmd_node).unwrap();
                for table in ctx
                    .inner
                    .related()
                    .into_iter()
                    .flat_map(|doc| doc.content.as_latex())
                {
                    for item in table
                        .commands
                        .iter()
                        .filter(|cmd_node| *cmd_node != current_cmd_node)
                        .map(|cmd_node| {
                            let name = &table.as_command(*cmd_node).unwrap().name.text()[1..];
                            Item::new(
                                current_cmd.short_name_range(),
                                ItemData::UserCommand { name },
                            )
                        })
                    {
                        ctx.items.push(item);
                    }
                }
                break;
            }
        }
    }
}

pub fn complete_user_environments(ctx: &mut CompletionContext) {
    fn make_item(
        table: &latex::SymbolTable,
        delim: latex::EnvironmentDelimiter,
        name_range: Range,
    ) -> Option<Item> {
        delim
            .name(&table)
            .map(|name| Item::new(name_range, ItemData::UserEnvironment { name: &name.text() }))
    }

    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        if let Some((range, cmd_node)) = ctx
            .scopes
            .iter()
            .flat_map(|scope| scope.match_environment(&table.tree, pos))
            .next()
        {
            for doc in ctx.inner.related() {
                if let DocumentContent::Latex(table) = &doc.content {
                    for env in &table.environments {
                        if (env.left.parent == cmd_node || env.right.parent == cmd_node)
                            && doc.uri == ctx.inner.current().uri
                        {
                            continue;
                        }

                        if let Some(item) = make_item(&table, env.left, range) {
                            ctx.items.push(item);
                        }

                        if let Some(item) = make_item(&table, env.right, range) {
                            ctx.items.push(item);
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
    use itertools::Itertools;

    #[test]
    fn command() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \include{bar}
                            \foo
                        "#
                    ),
                ),
                ("bar.tex", r#"\bar"#),
                ("baz.tex", r#"\baz"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_commands(&mut ctx);

        let actual_labels: Vec<_> = ctx
            .items
            .into_iter()
            .map(|item| item.data.label().to_owned())
            .collect();
        assert_eq!(actual_labels, vec!["include", "bar"]);
    }

    #[test]
    fn environment() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \include{bar}
                            \begin{foo}
                        "#
                    ),
                ),
                ("bar.tex", r#"\begin{bar}\end{bar}"#),
                ("baz.tex", r#"\begin{baz}\end{baz}"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(9)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_environments(&mut ctx);

        let actual_labels: Vec<_> = ctx
            .items
            .into_iter()
            .map(|item| item.data.label().to_owned())
            .unique()
            .collect();
        assert_eq!(actual_labels, vec!["bar"]);
    }

    #[test]
    fn empty_latex_document_command() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document_command() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document_environment() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document_environment() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_user_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
