use crate::{
    features::{
        prelude::*,
        symbol::types::{LatexSymbol, LatexSymbolKind, SymbolContext},
    },
    syntax::BibtexEntryTypeCategory,
};

pub fn find_entry_symbols(ctx: &mut SymbolContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        for entry_node in tree.children(tree.root) {
            if let Some(entry) = tree
                .as_entry(entry_node)
                .filter(|entry| !entry.is_comment())
                .filter(|entry| entry.key.is_some())
            {
                let category = LANGUAGE_DATA
                    .find_entry_type(&entry.ty.text()[1..])
                    .map(|ty| ty.category)
                    .unwrap_or(BibtexEntryTypeCategory::Misc);

                let key = entry.key.as_ref().unwrap();
                let item = LatexSymbol {
                    name: key.text().to_owned(),
                    label: None,
                    kind: LatexSymbolKind::Entry(category),
                    deprecated: false,
                    full_range: entry.range(),
                    selection_range: key.range(),
                    children: find_field_symbols(tree, entry_node),
                };
                ctx.items.push(item);
            }
        }
    }
}

fn find_field_symbols(tree: &bibtex::Tree, entry_node: NodeIndex) -> Vec<LatexSymbol> {
    let mut children = Vec::new();
    for field in tree
        .children(entry_node)
        .filter_map(|node| tree.as_field(node))
    {
        let item = LatexSymbol {
            name: field.name.text().to_owned(),
            label: None,
            kind: LatexSymbolKind::Field,
            deprecated: false,
            full_range: field.range(),
            selection_range: field.name.range(),
            children: Vec::new(),
        };
        children.push(item);
    }
    children
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn entry() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@article{key, foo = bar, baz = qux}"#)])
                .main("main.bib")
                .build()
                .symbol(),
        );

        find_entry_symbols(&mut ctx);

        let expected_items = vec![LatexSymbol {
            name: "key".into(),
            label: None,
            kind: LatexSymbolKind::Entry(BibtexEntryTypeCategory::Article),
            deprecated: false,
            full_range: Range::new_simple(0, 0, 0, 35),
            selection_range: Range::new_simple(0, 9, 0, 12),
            children: vec![
                LatexSymbol {
                    name: "foo".into(),
                    label: None,
                    kind: LatexSymbolKind::Field,
                    deprecated: false,
                    full_range: Range::new_simple(0, 14, 0, 24),
                    selection_range: Range::new_simple(0, 14, 0, 17),
                    children: Vec::new(),
                },
                LatexSymbol {
                    name: "baz".into(),
                    label: None,
                    kind: LatexSymbolKind::Field,
                    deprecated: false,
                    full_range: Range::new_simple(0, 25, 0, 34),
                    selection_range: Range::new_simple(0, 25, 0, 28),
                    children: Vec::new(),
                },
            ],
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[test]
    fn comment() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@comment{key, foo = bar, baz = qux}"#)])
                .main("main.bib")
                .build()
                .symbol(),
        );

        find_entry_symbols(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .build()
                .symbol(),
        );

        find_entry_symbols(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .build()
                .symbol(),
        );

        find_entry_symbols(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
