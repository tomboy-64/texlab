use crate::features::{
    completion::{
        types::{Item, ItemData},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_fields(ctx: &mut CompletionContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        match tree
            .find(pos)
            .into_iter()
            .last()
            .map(|node| &tree.graph[node])
        {
            Some(bibtex::Node::Field(field)) => {
                if field.name.range().contains(pos) {
                    make_items(&mut ctx.items, field.name.range());
                    return;
                }
            }
            Some(bibtex::Node::Entry(entry)) => {
                if !entry.is_comment() && !entry.ty.range().contains(pos) {
                    let range = Range::new(pos, pos);
                    if let Some(key) = &entry.key {
                        if !key.range().contains(pos) {
                            make_items(&mut ctx.items, range);
                            return;
                        }
                    } else {
                        make_items(&mut ctx.items, range);
                        return;
                    }
                }
            }
            _ => (),
        }
    }
}

fn make_items(items: &mut Vec<Item>, range: Range) {
    for field in &LANGUAGE_DATA.fields {
        let item = Item::new(range, ItemData::Field { field });
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn inside_first_field() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @article{foo,
                        bar}
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_fields(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 0, 1, 3));
    }

    #[test]
    fn inside_second_field() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@article{foo, bar = {baz}, qux}")])
            .main("main.bib")
            .line(0)
            .character(27)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_fields(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 27, 0, 30));
    }

    #[test]
    fn inside_entry() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @article{foo,
                        }
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_fields(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 0, 1, 0));
    }

    #[test]
    fn inside_content() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.bib",
                indoc!(
                    r#"
                        @article{foo,
                        bar = {baz}}
                    "#
                ),
            )])
            .main("main.bib")
            .line(1)
            .character(7)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_fields(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn inside_entry_type() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@article{foo,}")])
            .main("main.bib")
            .line(0)
            .character(3)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_fields(&mut ctx);

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

        complete_fields(&mut ctx);

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

        complete_fields(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
