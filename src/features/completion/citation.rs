use crate::{
    features::{
        completion::{
            types::{Item, ItemData, LatexArgumentPattern},
            CompletionContext,
        },
        prelude::*,
    },
    syntax::{BibtexEntryTypeCategory, Structure},
};
use once_cell::sync::Lazy;
use petgraph::graph::NodeIndex;
use regex::Regex;

pub fn complete_citations(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in &LANGUAGE_DATA.citation_commands {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    for doc in ctx.inner.related() {
                        if let DocumentContent::Bibtex(tree) = &doc.content {
                            for entry_node in tree.children(tree.root) {
                                if let Some(item) = make_item(range, doc, tree, entry_node) {
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

fn make_item<'a>(
    item_range: Range,
    doc: &'a Document,
    tree: &'a bibtex::Tree,
    entry_node: NodeIndex,
) -> Option<Item<'a>> {
    let entry = tree.as_entry(entry_node)?;
    if entry.is_comment() {
        return None;
    }

    let key = entry.key.as_ref()?.text();
    let options = BibtexFormattingOptions::default();
    let params = bibtex::FormattingParams {
        insert_spaces: true,
        tab_size: 4,
        options: &options,
    };
    let entry_code = bibtex::format(tree, entry_node, params);
    let text = format!(
        "{} {}",
        &key,
        WHITESPACE_REGEX
            .replace_all(
                &entry_code
                    .replace('{', "")
                    .replace('}', "")
                    .replace(',', " ")
                    .replace('=', " "),
                " ",
            )
            .trim()
    );

    let ty = LANGUAGE_DATA
        .find_entry_type(&entry.ty.text()[1..])
        .map(|ty| Structure::Entry(ty.category))
        .unwrap_or_else(|| Structure::Entry(BibtexEntryTypeCategory::Misc));

    let item = Item::new(
        item_range,
        ItemData::Citation {
            uri: &doc.uri,
            key,
            text,
            ty,
        },
    );
    Some(item)
}

static WHITESPACE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("\\s+").unwrap());

#[cfg(test)]
mod test {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn incomplete() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "main.tex",
                    indoc!(
                        r#"
                            \addbibresource{main.bib}
                            \cite{
                            \begin{foo}
                            \end{bar}
                        "#
                    ),
                ),
                ("main.bib", "@article{foo,}"),
            ])
            .main("main.tex")
            .line(1)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_citations(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "foo");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 6, 1, 6));
    }

    #[test]
    fn empty_key() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \addbibresource{bar.bib}
                            \cite{}
                        "#
                    ),
                ),
                ("bar.bib", "@article{foo,}"),
                ("baz.bib", "@article{bar,}"),
            ])
            .main("foo.tex")
            .line(1)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_citations(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "foo");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 6, 1, 6));
    }

    #[test]
    fn single_key() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \addbibresource{bar.bib}
                            \cite{foo}
                        "#
                    ),
                ),
                ("bar.bib", "@article{foo,}"),
                ("baz.bib", "@article{bar,}"),
            ])
            .main("foo.tex")
            .line(1)
            .character(6)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_citations(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "foo");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 6, 1, 9));
    }

    #[test]
    fn second_key() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \addbibresource{bar.bib}
                            \cite{foo,}
                        "#
                    ),
                ),
                ("bar.bib", "@article{foo,}"),
                ("baz.bib", "@article{bar,}"),
            ])
            .main("foo.tex")
            .line(1)
            .character(10)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_citations(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "foo");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 10, 1, 10));
    }

    #[test]
    fn outside_cite() {
        let inner = FeatureTester::builder()
            .files(vec![
                (
                    "foo.tex",
                    indoc!(
                        r#"
                            \addbibresource{bar.bib}
                            \cite{}
                        "#
                    ),
                ),
                ("bar.bib", "@article{foo,}"),
                ("baz.bib", "@article{bar,}"),
            ])
            .main("foo.tex")
            .line(1)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_citations(&mut ctx);

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

        complete_citations(&mut ctx);

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

        complete_citations(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
