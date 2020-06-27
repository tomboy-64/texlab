use crate::features::{
    completion::{
        types::{Item, ItemData, LatexCompletionScope},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_component_commands(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for scope in &ctx.scopes {
            if let LatexCompletionScope::Command(cmd_node) = scope {
                let cmd = table.tree.as_command(*cmd_node).unwrap();
                let range = cmd.short_name_range();
                for comp in &ctx.inner.view.components {
                    for cmd in &comp.commands {
                        ctx.items.push(Item::new(
                            range,
                            ItemData::ComponentCommand {
                                name: &cmd.name,
                                image: cmd.image.as_deref(),
                                glyph: cmd.glyph.as_deref(),
                                file_names: &comp.file_names,
                            },
                        ));
                    }
                }
                break;
            }
        }
    }
}

pub fn complete_component_environments(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            if let Some((range, _)) = scope.match_environment(&table.tree, pos) {
                for comp in &ctx.inner.view.components {
                    for env in &comp.environments {
                        ctx.items.push(Item::new(
                            range,
                            ItemData::ComponentEnvironment {
                                name: env,
                                file_names: &comp.file_names,
                            },
                        ));
                    }
                }
                break;
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
    fn command_start() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\use"#)])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn command_end() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\use"#)])
            .main("main.tex")
            .line(0)
            .character(4)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 1, 0, 4));
    }

    #[test]
    fn command_word() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"use"#)])
            .main("main.tex")
            .line(0)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn command_package() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \usepackage{lipsum}
                        \lips
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(ctx.items.iter().any(|item| item.data.label() == "lipsum"));
    }

    #[test]
    fn command_package_comma_separated() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \usepackage{geometry, lipsum}
                        \lips
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(ctx.items.iter().any(|item| item.data.label() == "lipsum"));
    }

    #[test]
    fn command_class() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \documentclass{book}
                        \chap
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_commands(&mut ctx);

        assert!(ctx.items.iter().any(|item| item.data.label() == "chapter"));
    }

    #[test]
    fn environment_inside_of_empty_begin() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\begin{}"#)])
            .main("main.tex")
            .line(0)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 7, 0, 7));
    }

    #[test]
    fn environment_inside_of_non_empty_end() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\end{foo}"#)])
            .main("main.tex")
            .line(0)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 5, 0, 8));
    }

    #[test]
    fn environment_outside_of_empty_begin() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\begin{}"#)])
            .main("main.tex")
            .line(0)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn environment_outside_of_empty_end() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\end{}"#)])
            .main("main.tex")
            .line(0)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn environment_inside_of_other_command() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\foo{bar}"#)])
            .main("main.tex")
            .line(0)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn environment_inside_second_argument() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\begin{foo}{bar}"#)])
            .main("main.tex")
            .line(0)
            .character(14)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn environment_unterminated() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\begin{foo"#)])
            .main("main.tex")
            .line(0)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_component_environments(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 7, 0, 10));
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

        complete_component_commands(&mut ctx);

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

        complete_component_commands(&mut ctx);

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

        complete_component_environments(&mut ctx);

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

        complete_component_environments(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
