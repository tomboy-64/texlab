use crate::{features::prelude::*, syntax::Span};
use std::collections::HashMap;

pub fn prepare_entry_rename(ctx: &FeatureContext<TextDocumentPositionParams>) -> Option<Range> {
    find_key(&ctx.current().content, ctx.params.position).map(Span::range)
}

pub fn rename_entry(ctx: &FeatureContext<RenameParams>) -> Option<WorkspaceEdit> {
    let key_name = find_key(
        &ctx.current().content,
        ctx.params.text_document_position.position,
    )?;
    let mut changes = HashMap::new();
    for doc in ctx.related() {
        let edits = match &doc.content {
            DocumentContent::Latex(table) => table
                .citations
                .iter()
                .flat_map(|citation| citation.keys(&table))
                .filter(|citation| citation.text() == key_name.text)
                .map(|citation| TextEdit::new(citation.range(), ctx.params.new_name.clone()))
                .collect(),
            DocumentContent::Bibtex(tree) => tree
                .children(tree.root)
                .filter_map(|node| tree.as_entry(node))
                .filter_map(|entry| entry.key.as_ref())
                .filter(|entry_key| entry_key.text() == key_name.text)
                .map(|entry_key| TextEdit::new(entry_key.range(), ctx.params.new_name.clone()))
                .collect(),
        };
        changes.insert(doc.uri.clone().into(), edits);
    }
    Some(WorkspaceEdit::new(changes))
}

fn find_key(content: &DocumentContent, pos: Position) -> Option<&Span> {
    match content {
        DocumentContent::Latex(table) => table
            .citations
            .iter()
            .flat_map(|citation| citation.keys(&table))
            .find(|key| key.range().contains(pos))
            .map(|key| &key.span),
        DocumentContent::Bibtex(tree) => tree
            .children(tree.root)
            .filter_map(|node| tree.as_entry(node))
            .filter_map(|entry| entry.key.as_ref())
            .find(|key| key.range().contains(pos))
            .map(|key| &key.span),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn entry() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("main.bib", r#"@article{foo, bar = baz}"#),
                (
                    "main.tex",
                    indoc!(
                        r#"
                        \addbibresource{main.bib}
                        \cite{foo}
                    "#
                    ),
                ),
            ])
            .main("main.bib")
            .line(0)
            .character(9)
            .new_name("qux")
            .build();
        let uri1 = tester.uri("main.bib");
        let uri2 = tester.uri("main.tex");

        let actual_edit = rename_entry(&tester.rename()).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri1.into(),
            vec![TextEdit::new(Range::new_simple(0, 9, 0, 12), "qux".into())],
        );
        expected_changes.insert(
            uri2.into(),
            vec![TextEdit::new(Range::new_simple(1, 6, 1, 9), "qux".into())],
        );
        let expected_edit = WorkspaceEdit::new(expected_changes);
        assert_eq!(actual_edit, expected_edit);
    }

    #[test]
    fn citation() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("main.bib", r#"@article{foo, bar = baz}"#),
                (
                    "main.tex",
                    indoc!(
                        r#"
                        \addbibresource{main.bib}
                        \cite{foo}
                    "#
                    ),
                ),
            ])
            .main("main.tex")
            .line(1)
            .character(6)
            .new_name("qux")
            .build();
        let uri1 = tester.uri("main.bib");
        let uri2 = tester.uri("main.tex");

        let actual_edit = rename_entry(&tester.rename()).unwrap();

        let mut expected_changes = HashMap::new();
        expected_changes.insert(
            uri1.into(),
            vec![TextEdit::new(Range::new_simple(0, 9, 0, 12), "qux".into())],
        );
        expected_changes.insert(
            uri2.into(),
            vec![TextEdit::new(Range::new_simple(1, 6, 1, 9), "qux".into())],
        );
        let expected_edit = WorkspaceEdit::new(expected_changes);

        assert_eq!(actual_edit, expected_edit);
    }

    #[test]
    fn field_name() {
        let actual_edit = rename_entry(
            &FeatureTester::builder()
                .files(vec![("main.bib", r#"@article{foo, bar = baz}"#)])
                .main("main.bib")
                .line(0)
                .character(14)
                .new_name("qux")
                .build()
                .rename(),
        );

        assert_eq!(actual_edit, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_edit = rename_entry(
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
        let actual_edit = rename_entry(
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
