use crate::{features::prelude::*, syntax::LatexIncludeKind};

pub fn hover_components(req: &FeatureContext<HoverParams>) -> Option<Hover> {
    let table = req.current().content.as_latex()?;
    let pos = req.params.text_document_position_params.position;
    for include in &table.includes {
        match include.kind {
            LatexIncludeKind::Package | LatexIncludeKind::Class => {
                for path in include.paths(&table) {
                    if path.range().contains(pos) {
                        let docs = COMPONENT_DATABASE.documentation(path.text())?;
                        return Some(Hover {
                            contents: HoverContents::Markup(docs),
                            range: Some(path.range()),
                        });
                    }
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;

    #[tokio::test]
    async fn known_package() {
        let actual_hover = hover_components(
            &FeatureTester::builder()
                .files(vec![("main.tex", r#"\usepackage{amsmath}"#)])
                .main("main.tex")
                .line(0)
                .character(15)
                .build()
                .hover(),
        );

        assert_eq!(
            actual_hover.unwrap().range.unwrap(),
            Range::new_simple(0, 12, 0, 19)
        );
    }

    #[tokio::test]
    async fn unknown_class() {
        let actual_hover = hover_components(
            &FeatureTester::builder()
                .files(vec![("main.tex", r#"\documentclass{abcdefghijklmnop}"#)])
                .main("main.tex")
                .line(0)
                .character(20)
                .build()
                .hover(),
        );

        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_latex_document() {
        let actual_hover = hover_components(
            &FeatureTester::builder()
                .files(vec![("main.tex", "")])
                .main("main.tex")
                .line(0)
                .character(0)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }

    #[test]
    fn empty_bibtex_document() {
        let actual_hover = hover_components(
            &FeatureTester::builder()
                .files(vec![("main.bib", "")])
                .main("main.bib")
                .line(0)
                .character(0)
                .build()
                .hover(),
        );
        assert_eq!(actual_hover, None);
    }
}
