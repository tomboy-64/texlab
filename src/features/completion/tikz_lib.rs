use crate::features::{
    completion::{
        types::{Item, ItemData, LatexArgumentPattern},
        CompletionContext,
    },
    prelude::*,
};

pub fn complete_pgf_libraries(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        if let Some((range, _)) = ctx
            .scopes
            .iter()
            .filter_map(|scope| {
                scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name("usepgflibrary")
                        .index(0)
                        .position(pos)
                        .build(),
                )
            })
            .next()
        {
            for name in &LANGUAGE_DATA.pgf_libraries {
                let item = Item::new(range, ItemData::PgfLibrary { name });
                ctx.items.push(item);
            }
        }
    }
}

pub fn complete_tikz_libraries(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        if let Some((range, _)) = ctx
            .scopes
            .iter()
            .filter_map(|scope| {
                scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name("usetikzlibrary")
                        .index(0)
                        .position(pos)
                        .build(),
                )
            })
            .next()
        {
            for name in &LANGUAGE_DATA.tikz_libraries {
                let item = Item::new(range, ItemData::TikzLibrary { name });
                ctx.items.push(item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn pgf_library() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\usepgflibrary{}"#)])
            .main("main.tex")
            .line(0)
            .character(15)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_pgf_libraries(&mut ctx);

        assert!(!ctx.items.is_empty());
    }

    #[test]
    fn tikz_library() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\usetikzlibrary{}"#)])
            .main("main.tex")
            .line(0)
            .character(16)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_tikz_libraries(&mut ctx);

        assert!(!ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document_pgf() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_pgf_libraries(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document_pgf() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_pgf_libraries(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_latex_document_tikz() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_tikz_libraries(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document_tikz() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_tikz_libraries(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
