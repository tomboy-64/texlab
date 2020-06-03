use crate::features::{link::LinkContext, prelude::*};

pub fn link_latex_imports(ctx: &mut LinkContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for import in &table.imports {
            let file = import.file(&table);
            for target in &import.targets {
                if let Some(item) = ctx.inner.snapshot().find(target).map(|doc| DocumentLink {
                    range: file.range(),
                    target: doc.uri.clone().into(),
                    tooltip: None,
                }) {
                    ctx.items.push(item);
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[test]
    fn empty_latex_document() {
        let mut ctx = LinkContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .build()
                .link(),
        );

        link_latex_imports(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn empty_bibtex_document() {
        let mut ctx = LinkContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .build()
                .link(),
        );

        link_latex_imports(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[test]
    fn has_links() {
        let tester = FeatureTester::builder()
            .files(vec![
                ("foo.tex", r#"\import{bar/}{baz}"#),
                ("bar/baz.tex", r#""#),
            ])
            .main("foo.tex")
            .build();
        let target = tester.uri("bar/baz.tex");
        let mut ctx = LinkContext::new(tester.link());

        link_latex_imports(&mut ctx);

        let expected_items = vec![DocumentLink {
            range: Range::new_simple(0, 14, 0, 17),
            target: target.into(),
            tooltip: None,
        }];

        assert_eq!(ctx.items, expected_items);
    }
}
