use crate::features::{link::LinkContext, prelude::*};

pub fn link_latex_includes(ctx: &mut LinkContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        for include in &table.includes {
            let paths = include.paths(&table);
            for (i, targets) in include.all_targets.iter().enumerate() {
                for target in targets {
                    if let Some(item) = ctx.inner.snapshot().find(target).map(|doc| DocumentLink {
                        range: paths[i].range(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[tokio::test]
    async fn empty_latex_document() {
        let mut ctx = LinkContext::new(
            FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .build()
                .link(),
        );

        link_latex_includes(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[tokio::test]
    async fn empty_bibtex_document() {
        let mut ctx = LinkContext::new(
            FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .build()
                .link(),
        );

        link_latex_includes(&mut ctx);

        assert!(ctx.items.is_empty());
    }

    #[tokio::test]
    async fn has_links() {
        let tester = FeatureTester::builder()
            .files(vec![("foo.tex", r#"\input{bar.tex}"#), ("bar.tex", r#""#)])
            .main("foo.tex")
            .build();
        let target = tester.uri("bar.tex");
        let mut ctx = LinkContext::new(tester.link());

        link_latex_includes(&mut ctx);

        let expected_items = vec![DocumentLink {
            range: Range::new_simple(0, 7, 0, 14),
            target: target.into(),
            tooltip: None,
        }];
        assert_eq!(ctx.items, expected_items);
    }
}
