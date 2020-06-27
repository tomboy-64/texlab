use crate::features::{
    completion::{
        types::{Item, ItemData},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_entry_types(ctx: &mut CompletionContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for decl in tree.children(tree.root) {
            match &tree.graph[decl] {
                bibtex::Node::Preamble(preamble) => {
                    if contains(&preamble.ty, pos) {
                        make_items(&mut ctx.items, preamble.ty.range());
                        return;
                    }
                }
                bibtex::Node::String(string) => {
                    if contains(&string.ty, pos) {
                        make_items(&mut ctx.items, string.ty.range());
                        return;
                    }
                }
                bibtex::Node::Entry(entry) => {
                    if contains(&entry.ty, pos) {
                        make_items(&mut ctx.items, entry.ty.range());
                        return;
                    }
                }
                _ => {}
            }
        }
    }
}

fn contains(ty: &bibtex::Token, pos: Position) -> bool {
    ty.range().contains(pos) && ty.start().character != pos.character
}

fn make_items(items: &mut Vec<Item>, mut range: Range) {
    range.start.character += 1;
    for ty in &LANGUAGE_DATA.entry_types {
        let item = Item::new(range, ItemData::EntryType { ty });
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn before_at_sign() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_entry_types(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn after_at_sign() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@")])
            .main("main.bib")
            .line(0)
            .character(1)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_entry_types(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 1, 0, 1));
    }

    #[test]
    fn inside_entry_type() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@foo")])
            .main("main.bib")
            .line(0)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_entry_types(&mut ctx);

        assert!(!ctx.items.is_empty());
        assert_eq!(ctx.items[0].range, Range::new_simple(0, 1, 0, 4));
    }

    #[test]
    fn inside_entry_key() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "@article{foo,}")])
            .main("main.bib")
            .line(0)
            .character(11)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_entry_types(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn inside_comments() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "foo")])
            .main("main.bib")
            .line(0)
            .character(2)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_entry_types(&mut ctx);

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

        complete_entry_types(&mut ctx);

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

        complete_entry_types(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
