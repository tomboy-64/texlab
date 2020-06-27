use crate::{
    features::{
        completion::{
            types::{Item, ItemData, LatexArgumentPattern},
            CompletionContext,
        },
        outline::{Outline, OutlineContext, OutlineContextItem},
        prelude::*,
    },
    syntax::{LatexLabelKind, LatexLabelReferenceSource, Structure},
};
use std::sync::Arc;

pub fn complete_labels(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in LANGUAGE_DATA
                .label_commands
                .iter()
                .filter(|cmd| cmd.kind.is_reference())
            {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    let source = match cmd.kind {
                        LatexLabelKind::Definition => unreachable!(),
                        LatexLabelKind::Reference(source) => source,
                    };

                    for doc in ctx.inner.related() {
                        let snapshot = Arc::clone(&ctx.inner.view.snapshot);
                        let view = DocumentView::analyze(
                            snapshot,
                            Arc::clone(&doc),
                            &ctx.inner.options,
                            &ctx.inner.current_dir,
                        );
                        let options = &ctx.inner.options;
                        let current_dir = &ctx.inner.current_dir;
                        let outline = Outline::analyze(&view, options, current_dir);

                        if let DocumentContent::Latex(table) = &doc.content {
                            for label in table
                                .labels
                                .iter()
                                .filter(|label| label.kind == LatexLabelKind::Definition)
                                .filter(|label| is_included(&table, label, source))
                            {
                                let outline_ctx = OutlineContext::parse(&view, &outline, *label);

                                let kind = match outline_ctx.as_ref().map(|ctx| &ctx.item) {
                                    Some(OutlineContextItem::Section { .. }) => Structure::Section,
                                    Some(OutlineContextItem::Caption { .. }) => Structure::Float,
                                    Some(OutlineContextItem::Theorem { .. }) => Structure::Theorem,
                                    Some(OutlineContextItem::Equation) => Structure::Equation,
                                    Some(OutlineContextItem::Item) => Structure::Item,
                                    None => Structure::Label,
                                };

                                for name in label.names(&table) {
                                    let header = outline_ctx.as_ref().and_then(|ctx| ctx.detail());
                                    let footer =
                                        outline_ctx.as_ref().and_then(|ctx| match &ctx.item {
                                            OutlineContextItem::Caption { text, .. } => {
                                                Some(text.clone())
                                            }
                                            _ => None,
                                        });

                                    let text = outline_ctx
                                        .as_ref()
                                        .map(|ctx| format!("{} {}", name.text(), ctx.reference()))
                                        .unwrap_or_else(|| name.text().into());

                                    let item = Item::new(
                                        range,
                                        ItemData::Label {
                                            name: name.text(),
                                            kind,
                                            header,
                                            footer,
                                            text,
                                        },
                                    );
                                    ctx.items.push(item);
                                }
                            }
                        }
                    }

                    break;
                }
            }
        }
    }
}

fn is_included(
    table: &latex::SymbolTable,
    label: &latex::Label,
    source: LatexLabelReferenceSource,
) -> bool {
    let label_range = table[label.parent].range();
    match source {
        LatexLabelReferenceSource::Everything => true,
        LatexLabelReferenceSource::Math => table
            .environments
            .iter()
            .filter(|env| env.left.is_math(&table))
            .any(|env| env.range(&table).contains_exclusive(label_range.start)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn inside_of_ref() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \addbibresource{bar.bib}
                            \include{baz}
                            \ref{}
                        "#
                    ),
                ),
                ("bar.bib", ""),
                ("baz.tex", r#"\label{foo}\label{bar}\ref{baz}"#),
            ])
            .main("foo.tex")
            .line(2)
            .character(5)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_labels(&mut ctx);

        let actual_labels: Vec<_> = ctx
            .items
            .into_iter()
            .map(|item| item.data.label().to_owned())
            .collect();
        assert_eq!(actual_labels, vec!["foo", "bar"]);
    }

    #[test]
    fn outside_of_ref() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \include{bar}
                            \ref{}
                        "#
                    ),
                ),
                ("bar.tex", r#"\label{foo}\label{bar}"#),
            ])
            .main("foo.tex")
            .line(1)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_labels(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn eqref() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \begin{align}\label{foo}\end{align}\label{bar}
                        \eqref{}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_labels(&mut ctx);

        let actual_labels: Vec<_> = ctx
            .items
            .into_iter()
            .map(|item| item.data.label().to_owned())
            .collect();
        assert_eq!(actual_labels, vec!["foo"]);
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

        complete_labels(&mut ctx);

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

        complete_labels(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
