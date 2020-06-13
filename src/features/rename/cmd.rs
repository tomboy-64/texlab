use crate::features::prelude::*;
use std::collections::HashMap;

pub fn prepare_command_rename(ctx: &FeatureContext<TextDocumentPositionParams>) -> Option<Range> {
    let pos = ctx.params.position;
    find_command(&ctx.current().content, pos).map(SyntaxNode::range)
}

pub fn rename_command(ctx: &FeatureContext<RenameParams>) -> Option<WorkspaceEdit> {
    let pos = ctx.params.text_document_position.position;
    let cmd_name = find_command(&ctx.current().content, pos)?.name.text();
    let mut changes = HashMap::new();
    for doc in ctx.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            let edits = table
                .commands
                .iter()
                .filter_map(|node| table.as_command(*node))
                .filter(|cmd| cmd.name.text() == cmd_name)
                .map(|cmd| TextEdit::new(cmd.name.range(), format!("\\{}", ctx.params.new_name)))
                .collect();
            changes.insert(doc.uri.clone().into(), edits);
        }
    }
    Some(WorkspaceEdit::new(changes))
}

fn find_command(content: &DocumentContent, pos: Position) -> Option<&latex::Command> {
    if let DocumentContent::Latex(table) = &content {
        table.as_command(table.find_command_by_short_name_range(pos)?)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn command() {
        let tester = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                        \include{bar.tex}
                        \baz
                    "#
                    ),
                ),
                ("bar.tex", r#"\baz"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(2)
            .new_name("qux")
            .build();

        let uri1 = tester.uri("foo.tex");
        let uri2 = tester.uri("bar.tex");

        let actual_edit = rename_command(&tester.rename()).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri1.into(),
            vec![TextEdit::new(Range::new_simple(1, 0, 1, 4), "\\qux".into())],
        );
        expected_changes.insert(
            uri2.into(),
            vec![TextEdit::new(Range::new_simple(0, 0, 0, 4), "\\qux".into())],
        );

        assert_eq!(actual_edit, WorkspaceEdit::new(expected_changes));
    }

    #[test]
    fn empty_latex_document() {
        let actual_edit = rename_command(
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
        let actual_edit = rename_command(
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
