use crate::{
    features::{
        completion::{
            types::{Item, ItemData, LatexArgumentPattern},
            CompletionContext,
        },
        prelude::*,
    },
    syntax::LatexIncludeKind,
};
use std::{borrow::Cow, collections::HashSet};

pub async fn complete_classes<'a>(ctx: &mut CompletionContext<'a>) {
    let kind = LatexIncludeKind::Class;
    complete_imports(ctx, kind, |name| ItemData::Class { name }).await;
}

pub async fn complete_packages<'a>(ctx: &mut CompletionContext<'a>) {
    let kind = LatexIncludeKind::Package;
    complete_imports(ctx, kind, |name| ItemData::Package { name }).await;
}

async fn complete_imports<'a, F>(
    ctx: &mut CompletionContext<'a>,
    kind: LatexIncludeKind,
    mut factory: F,
) where
    F: FnMut(Cow<'a, str>) -> ItemData<'a>,
{
    let extension = if kind == LatexIncludeKind::Package {
        "sty"
    } else {
        "cls"
    };

    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in LANGUAGE_DATA
                .include_commands
                .iter()
                .filter(|cmd| cmd.kind == kind)
            {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    let resolver = ctx.inner.distro.resolver().await;
                    let mut file_names = HashSet::new();
                    for file_name in COMPONENT_DATABASE
                        .components
                        .iter()
                        .flat_map(|comp| comp.file_names.iter())
                        .filter(|file_name| file_name.ends_with(extension))
                    {
                        file_names.insert(file_name);
                        let stem = &file_name[0..file_name.len() - 4];
                        let data = factory(stem.into());
                        let item = Item::new(range, data);
                        ctx.items.push(item);
                    }

                    for file_name in resolver.files_by_name.keys().filter(|file_name| {
                        file_name.ends_with(extension) && !file_names.contains(file_name)
                    }) {
                        let stem = &file_name[0..file_name.len() - 4];
                        let data = factory(stem.to_owned().into());
                        let item = Item::new(range, data);
                        ctx.items.push(item);
                    }

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

    #[tokio::test]
    async fn class() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\documentclass{}"#)])
            .main("main.tex")
            .line(0)
            .character(15)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_classes(&mut ctx).await;

        assert!(ctx.items.iter().any(|item| item.data.label() == "beamer"));
        assert!(ctx.items.iter().all(|item| item.data.label() != "amsmath"));
    }

    #[tokio::test]
    async fn package() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", r#"\usepackage{}"#)])
            .main("main.tex")
            .line(0)
            .character(12)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_packages(&mut ctx).await;

        assert!(ctx.items.iter().all(|item| item.data.label() != "beamer"));
        assert!(ctx.items.iter().any(|item| item.data.label() == "amsmath"));
    }

    #[tokio::test]
    async fn empty_latex_document_class() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_classes(&mut ctx).await;

        assert!(ctx.items.is_empty());
    }

    #[tokio::test]
    async fn empty_bibtex_document_class() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_classes(&mut ctx).await;

        assert!(ctx.items.is_empty());
    }

    #[tokio::test]
    async fn empty_latex_document_package() {
        let inner = FeatureTester::builder()
            .files(vec![("main.tex", "")])
            .main("main.tex")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_packages(&mut ctx).await;

        assert!(ctx.items.is_empty());
    }

    #[tokio::test]
    async fn empty_bibtex_document_package() {
        let inner = FeatureTester::builder()
            .files(vec![("main.bib", "")])
            .main("main.bib")
            .line(0)
            .character(0)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_packages(&mut ctx).await;

        assert!(ctx.items.is_empty());
    }
}
