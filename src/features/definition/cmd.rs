use crate::features::{definition::DefinitionContext, prelude::*};

pub fn goto_command_definition(ctx: &mut DefinitionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position_params.position;
        if let Some(cmd) = table
            .find(pos)
            .last()
            .and_then(|node| table.as_command(*node))
        {
            for doc in ctx.inner.related() {
                if let DocumentContent::Latex(table) = &doc.content {
                    for item in table
                        .command_definitions
                        .iter()
                        .filter(|def| def.definition_name(&table) == cmd.name.text())
                        .map(|def| {
                            let def_range = table[def.parent].range();
                            LocationLink {
                                origin_selection_range: Some(cmd.range()),
                                target_uri: doc.uri.clone().into(),
                                target_range: def_range,
                                target_selection_range: def_range,
                            }
                        })
                    {
                        ctx.items.push(item);
                    }

                    for item in table
                        .math_operators
                        .iter()
                        .filter(|op| op.definition_name(&table) == cmd.name.text())
                        .map(|op| {
                            let def_range = table[op.parent].range();
                            LocationLink {
                                origin_selection_range: Some(cmd.range()),

                                target_uri: doc.uri.clone().into(),
                                target_range: def_range,
                                target_selection_range: def_range,
                            }
                        })
                    {
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
    fn command_definition() {
        let tester = FeatureTester::builder()
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
                ("bar.tex", r#"\newcommand{\foo}{bar}"#),
                ("baz.tex", r#"\newcommand{\foo}{baz}"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(3)
            .build();
        let target_uri = tester.uri("bar.tex");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_command_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 0, 1, 4)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 0, 0, 22),
            target_selection_range: Range::new_simple(0, 0, 0, 22),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn math_operator() {
        let tester = FeatureTester::builder()
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
                ("bar.tex", r#"\DeclareMathOperator{\foo}{bar}"#),
                ("baz.tex", r#"\DeclareMathOperator{\foo}{baz}"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(3)
            .build();
        let target_uri = tester.uri("bar.tex");
        let mut ctx = DefinitionContext::new(tester.definition());

        goto_command_definition(&mut ctx);

        let expected_items = vec![LocationLink {
            origin_selection_range: Some(Range::new_simple(1, 0, 1, 4)),
            target_uri: target_uri.into(),
            target_range: Range::new_simple(0, 0, 0, 31),
            target_selection_range: Range::new_simple(0, 0, 0, 31),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = DefinitionContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .definition(),
        );

        goto_command_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = DefinitionContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .definition(),
        );

        goto_command_definition(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
