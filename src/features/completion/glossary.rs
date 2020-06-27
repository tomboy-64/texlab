use crate::{
    features::{
        completion::{
            types::{Item, ItemData, LatexArgumentPattern},
            CompletionContext,
        },
        prelude::*,
    },
    syntax::LatexGlossaryEntryKind::{Acronym, General},
};

pub fn complete_glossary_entries(ctx: &mut CompletionContext) {
    if let DocumentContent::Latex(table) = &ctx.inner.current().content {
        let pos = ctx.inner.params.text_document_position.position;
        for scope in &ctx.scopes {
            for cmd in &LANGUAGE_DATA.glossary_entry_reference_commands {
                if let Some((range, _)) = scope.match_argument(
                    LatexArgumentPattern::builder()
                        .tree(&table.tree)
                        .name(&cmd.name[1..])
                        .index(cmd.index)
                        .position(pos)
                        .build(),
                ) {
                    for doc in ctx.inner.related() {
                        if let DocumentContent::Latex(table) = &doc.content {
                            for entry in &table.glossary_entries {
                                match (cmd.kind, entry.kind) {
                                    (Acronym, Acronym)
                                    | (General, General)
                                    | (General, Acronym) => {
                                        let name = entry.label(&table).text();
                                        let data = ItemData::GlossaryEntry { name };
                                        let item = Item::new(range, data);
                                        ctx.items.push(item);
                                    }
                                    (Acronym, General) => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::testing::FeatureTester;
    use indoc::indoc;

    #[test]
    fn acronym() {
        let inner = FeatureTester::builder()
            .files(vec![(
                "main.tex",
                indoc!(
                    r#"
                        \newacronym{lvm}{LVM}{Logical Volume Manager}
                        \acrfull{foo}
                    "#
                ),
            )])
            .main("main.tex")
            .line(1)
            .character(9)
            .build()
            .completion();
        let mut ctx = CompletionContext::new(&inner);

        complete_glossary_entries(&mut ctx);

        assert_eq!(ctx.items.len(), 1);
        assert_eq!(ctx.items[0].data.label(), "lvm");
        assert_eq!(ctx.items[0].range, Range::new_simple(1, 9, 1, 12));
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

        complete_glossary_entries(&mut ctx);

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

        complete_glossary_entries(&mut ctx);

        assert!(ctx.items.is_empty());
    }
}
