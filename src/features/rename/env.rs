use crate::features::prelude::*;
use std::collections::HashMap;

pub fn prepare_environment_rename(
    ctx: &FeatureContext<TextDocumentPositionParams>,
) -> Option<Range> {
    let pos = ctx.params.position;
    let (left_name, right_name) = find_environment(&ctx.current().content, pos)?;
    let range = if left_name.range().contains(pos) {
        left_name.range()
    } else {
        right_name.range()
    };
    Some(range)
}

pub fn rename_environment(ctx: &FeatureContext<RenameParams>) -> Option<WorkspaceEdit> {
    let (left_name, right_name) = find_environment(
        &ctx.current().content,
        ctx.params.text_document_position.position,
    )?;
    let edits = vec![
        TextEdit::new(left_name.range(), ctx.params.new_name.clone()),
        TextEdit::new(right_name.range(), ctx.params.new_name.clone()),
    ];
    let mut changes = HashMap::new();
    changes.insert(ctx.current().uri.clone().into(), edits);
    Some(WorkspaceEdit::new(changes))
}

fn find_environment(
    content: &DocumentContent,
    pos: Position,
) -> Option<(&latex::Token, &latex::Token)> {
    if let DocumentContent::Latex(table) = content {
        for env in &table.environments {
            if let Some(left_name) = env.left.name(&table) {
                if let Some(right_name) = env.right.name(&table) {
                    if left_name.range().contains(pos) || right_name.range().contains(pos) {
                        return Some((left_name, right_name));
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn environment() {
        let tester = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                    \begin{foo}
                    \end{bar}
                "#
                ),
            )])
            .main("main.tex")
            .line(0)
            .character(8)
            .new_name("baz")
            .build();
        let uri = tester.uri("main.tex");

        let actual_edit = rename_environment(&tester.rename()).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri.into(),
            vec![
                TextEdit::new(Range::new_simple(0, 7, 0, 10), "baz".into()),
                TextEdit::new(Range::new_simple(1, 5, 1, 8), "baz".into()),
            ],
        );
        assert_eq!(actual_edit, WorkspaceEdit::new(expected_changes));
    }

    #[test]
    fn command() {
        let actual_edit = rename_environment(
            &FeatureTester::builder()
                .files(vec![(
                    "main.tex",
                    indoc!(
                        r#"
                            \begin{foo}
                            \end{bar}
                        "#
                    ),
                )])
                .main("main.tex")
                .line(0)
                .character(5)
                .new_name("baz")
                .build()
                .rename(),
        );

        assert_eq!(actual_edit, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_edit = rename_environment(
            &FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .new_name("")
                .build()
                .rename(),
        );

        assert_eq!(actual_edit, None);
    }

    #[test]
    fn empty_bibtex_document() {
        let actual_edit = rename_environment(
            &FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .new_name("")
                .build()
                .rename(),
        );

        assert_eq!(actual_edit, None);
    }
}
