use crate::{features::prelude::*, syntax::Span};
use std::collections::HashMap;

pub fn prepare_label_rename(ctx: &FeatureContext<TextDocumentPositionParams>) -> Option<Range> {
    let pos = ctx.params.position;
    find_label(&ctx.current().content, pos).map(Span::range)
}

pub fn rename_label(ctx: &FeatureContext<RenameParams>) -> Option<WorkspaceEdit> {
    let pos = ctx.params.text_document_position.position;
    let name = find_label(&ctx.current().content, pos)?;
    let mut changes = HashMap::new();
    for doc in ctx.related() {
        if let DocumentContent::Latex(table) = &doc.content {
            let edits = table
                .labels
                .iter()
                .flat_map(|label| label.names(&table))
                .filter(|label| label.text() == name.text)
                .map(|label| TextEdit::new(label.range(), ctx.params.new_name.clone()))
                .collect();
            changes.insert(doc.uri.clone().into(), edits);
        }
    }
    Some(WorkspaceEdit::new(changes))
}

fn find_label(content: &DocumentContent, pos: Position) -> Option<&Span> {
    let table = content.as_latex()?;
    table
        .labels
        .iter()
        .flat_map(|label| label.names(&table))
        .find(|label| label.range().contains(pos))
        .map(|label| &label.span)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn label() {
        let tester = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \label{foo}
                            \include{bar}
                        "#
                    ),
                ),
                ("bar.tex", r#"\ref{foo}"#),
                ("baz.tex", r#"\ref{foo}"#),
            ])
            .main("foo.tex")
            .line(0)
            .character(7)
            .new_name("bar")
            .build();
        let uri1 = tester.uri("foo.tex");
        let uri2 = tester.uri("bar.tex");

        let actual_edit = rename_label(&tester.rename()).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri1.into(),
            vec![TextEdit::new(Range::new_simple(0, 7, 0, 10), "bar".into())],
        );
        expected_changes.insert(
            uri2.into(),
            vec![TextEdit::new(Range::new_simple(0, 5, 0, 8), "bar".into())],
        );
        assert_eq!(actual_edit, WorkspaceEdit::new(expected_changes));
    }

    #[test]
    fn command_args() {
        let actual_edit = rename_label(
            &FeatureTester::builder()
                .files(vec![("main.tex", r#"\foo{bar}"#)])
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
        let actual_edit = rename_label(
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
        let actual_edit = rename_label(
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
