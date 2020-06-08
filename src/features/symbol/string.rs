use crate::features::{
    prelude::*,
    symbol::types::{LatexSymbol, LatexSymbolKind, SymbolContext},
};

pub fn find_string_symbols(ctx: &mut SymbolContext) {
    if let DocumentContent::Bibtex(tree) = &ctx.inner.current().content {
        for string_node in tree.children(tree.root) {
            if let Some(string) = &tree.as_string(string_node) {
                if let Some(name) = &string.name {
                    ctx.items.push(LatexSymbol {
                        name: name.text().into(),
                        label: None,
                        kind: LatexSymbolKind::String,
                        deprecated: false,
                        full_range: string.range(),
                        selection_range: name.range(),
                        children: Vec::new(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[tokio::test]
    async fn valid() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@string{key = "value"}"#)])
                .main("main.bib")
                .build()
                .symbol(),
        );

        find_string_symbols(&mut ctx);

        let expected_items = vec![LatexSymbol {
            name: "key".into(),
            label: None,
            kind: LatexSymbolKind::String,
            deprecated: false,
            full_range: Range::new_simple(0, 0, 0, 22),
            selection_range: Range::new_simple(0, 8, 0, 11),
            children: Vec::new(),
        }];

        assert_eq!(ctx.items, expected_items);
    }

    #[tokio::test]
    async fn invalid() {
        let mut ctx = SymbolContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", r#"@string{}"#)])
                .main("main.bib")
                .build()
                .symbol(),
        );

        find_string_symbols(&mut ctx);

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

        find_string_symbols(&mut ctx);

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

        find_string_symbols(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
