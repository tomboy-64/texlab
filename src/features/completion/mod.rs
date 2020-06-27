mod argument;
mod begin_cmd;
mod bibtex_cmd;
mod citation;
mod color;
mod color_model;
mod component;
mod entry_type;
mod field;
mod glossary;
mod import;
mod include;
mod label;
mod theorem;
mod tikz_lib;
mod types;
mod user;
mod util;

pub use self::types::CompletionItemData;

use self::{
    argument::complete_arguments,
    begin_cmd::complete_begin_command,
    bibtex_cmd::complete_bibtex_commands,
    citation::complete_citations,
    color::complete_colors,
    color_model::complete_color_models,
    component::{complete_component_commands, complete_component_environments},
    entry_type::complete_entry_types,
    field::complete_fields,
    glossary::complete_glossary_entries,
    import::{complete_classes, complete_packages},
    include::complete_includes,
    label::complete_labels,
    theorem::complete_theorem_environments,
    tikz_lib::{complete_pgf_libraries, complete_tikz_libraries},
    types::{Item, ItemData, LatexArgument, LatexCompletionScope},
    user::{complete_user_commands, complete_user_environments},
    util::{adjust_kind, component_detail, current_word, image_documentation},
};
use crate::{
    features::prelude::*,
    syntax::{AstNodeIndex, Structure},
};
use fuzzy_matcher::skim::fuzzy_match;
use std::collections::HashSet;

pub const COMPLETION_LIMIT: usize = 50;

#[derive(Debug, Clone)]
pub struct CompletionContext<'a> {
    inner: &'a FeatureContext<CompletionParams>,
    items: Vec<Item<'a>>,
    scopes: Vec<LatexCompletionScope>,
}

impl<'a> CompletionContext<'a> {
    pub fn new(inner: &'a FeatureContext<CompletionParams>) -> Self {
        let scopes = find_scopes(&inner).unwrap_or_default();
        Self {
            inner,
            items: Vec::new(),
            scopes,
        }
    }
}

fn find_scopes(ctx: &FeatureContext<CompletionParams>) -> Option<Vec<LatexCompletionScope>> {
    let mut scopes = Vec::new();

    let table = ctx.current().content.as_latex()?;
    let pos = ctx.params.text_document_position.position;
    if let Some(node) = table.find_command_by_short_name_range(pos) {
        scopes.push(LatexCompletionScope::Command(node));
        return Some(scopes);
    }

    let mut scopes = Vec::new();
    let cmd_node = find_command(table, pos)?;
    for (index, arg_node) in table.children(cmd_node).enumerate() {
        let arg = table.as_group(arg_node).unwrap();
        if arg.kind != latex::GroupKind::Group {
            continue;
        }

        if arg.right.is_some() && !arg.range().contains_exclusive(pos) {
            continue;
        }

        let word = table.children(arg_node).next().is_none()
            || table
                .extract_word(cmd_node, latex::GroupKind::Group, index)
                .is_some();

        scopes.push(LatexCompletionScope::Argument(LatexArgument {
            cmd_node,
            arg_node,
            index,
            word,
        }));
    }
    Some(scopes)
}

fn find_command(table: &latex::SymbolTable, pos: Position) -> Option<AstNodeIndex> {
    table
        .find(pos)
        .into_iter()
        .rev()
        .find(|node| table.as_command(*node).is_some())
}

pub async fn complete(ctx: FeatureContext<CompletionParams>) -> Vec<CompletionItem> {
    let items = complete_all(&ctx).await;
    let mut items = dedup(items);
    preselect(&ctx, &mut items);
    score(&ctx, &mut items);

    items.sort_by_key(|item| (!item.preselect, -item.score.unwrap_or(std::i64::MIN + 1)));
    items
        .into_iter()
        .take(COMPLETION_LIMIT)
        .filter(|item| item.score.is_some())
        .map(|item| convert(&ctx, item))
        .enumerate()
        .map(|(i, item)| append_sort_text(item, i))
        .collect()
}

async fn complete_all<'a>(ctx: &'a FeatureContext<CompletionParams>) -> Vec<Item<'a>> {
    let mut ctx = CompletionContext::new(ctx);
    complete_bibtex_commands(&mut ctx);
    complete_entry_types(&mut ctx);
    complete_fields(&mut ctx);

    complete_arguments(&mut ctx);
    complete_begin_command(&mut ctx);
    complete_colors(&mut ctx);
    complete_color_models(&mut ctx);
    complete_glossary_entries(&mut ctx);
    complete_citations(&mut ctx);
    complete_classes(&mut ctx).await;
    complete_packages(&mut ctx).await;
    complete_includes(&mut ctx).await;
    complete_labels(&mut ctx);
    complete_pgf_libraries(&mut ctx);
    complete_tikz_libraries(&mut ctx);
    complete_component_environments(&mut ctx);
    complete_theorem_environments(&mut ctx);
    complete_user_environments(&mut ctx);
    complete_component_commands(&mut ctx);
    complete_user_commands(&mut ctx);
    ctx.items
}

fn dedup<'a>(items: Vec<Item<'a>>) -> Vec<Item<'a>> {
    let mut labels = HashSet::new();
    let mut insert = vec![false; items.len()];
    for (i, item) in items.iter().enumerate() {
        insert[i] = labels.insert(item.data.label());
    }
    items
        .into_iter()
        .enumerate()
        .filter(|(i, _)| insert[*i])
        .map(|(_, item)| item)
        .collect()
}

fn preselect(ctx: &FeatureContext<CompletionParams>, items: &mut [Item]) {
    let pos = ctx.params.text_document_position.position;
    if let DocumentContent::Latex(table) = &ctx.current().content {
        for env in &table.environments {
            if let Some(name) = env.left.name(&table) {
                let right_args = table
                    .extract_group(env.right.parent, latex::GroupKind::Group, 0)
                    .unwrap();
                let right_args_range = table[right_args].range();
                let cond1 = right_args_range.contains_exclusive(pos);
                let cond2 = table
                    .as_group(right_args)
                    .and_then(|group| group.right.as_ref())
                    .is_none()
                    && right_args_range.contains(pos);

                if cond1 || cond2 {
                    for symbol in items.iter_mut() {
                        if symbol.data.label() == name.text() {
                            symbol.preselect = true;
                            break;
                        }
                    }
                }
            }
        }
    }
}

fn score(ctx: &FeatureContext<CompletionParams>, items: &mut Vec<Item>) {
    let current_word = current_word(ctx);
    let pattern = current_word.as_deref().unwrap_or_default();
    for item in items {
        item.score = match &item.data {
            ItemData::ComponentCommand { name, .. } => fuzzy_match(name, pattern),
            ItemData::ComponentEnvironment { name, .. } => fuzzy_match(name, pattern),
            ItemData::UserCommand { name } => fuzzy_match(name, pattern),
            ItemData::UserEnvironment { name } => fuzzy_match(name, pattern),
            ItemData::Label { text, .. } => fuzzy_match(&text, pattern),
            ItemData::Class { name } => fuzzy_match(&name, pattern),
            ItemData::Package { name } => fuzzy_match(&name, pattern),
            ItemData::PgfLibrary { name } => fuzzy_match(name, pattern),
            ItemData::TikzLibrary { name } => fuzzy_match(name, pattern),
            ItemData::File { name } => fuzzy_match(name, pattern),
            ItemData::Directory { name } => fuzzy_match(name, pattern),
            ItemData::Citation { text, .. } => fuzzy_match(&text, pattern),
            ItemData::Argument { name, .. } => fuzzy_match(&name, pattern),
            ItemData::BeginCommand => fuzzy_match("begin", pattern),
            ItemData::Color { name } => fuzzy_match(name, pattern),
            ItemData::ColorModel { name } => fuzzy_match(name, pattern),
            ItemData::GlossaryEntry { name } => fuzzy_match(name, pattern),
            ItemData::EntryType { ty } => fuzzy_match(&ty.name, pattern),
            ItemData::Field { field } => fuzzy_match(&field.name, pattern),
        };
    }
}

fn convert(ctx: &FeatureContext<CompletionParams>, item: Item) -> CompletionItem {
    let mut new_item = match item.data {
        ItemData::ComponentCommand {
            name,
            image,
            glyph,
            file_names,
        } => {
            let detail = glyph.map_or_else(
                || component_detail(file_names),
                |glyph| format!("{}, {}", glyph, component_detail(file_names)),
            );
            let documentation = image.and_then(|img| image_documentation(&ctx, &name, img));
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                kind: Some(adjust_kind(ctx, Structure::Command.completion_kind())),
                data: Some(CompletionItemData::Command.into()),
                documentation,
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::new_simple(name.into(), detail)
            }
        }
        ItemData::ComponentEnvironment { name, file_names } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                kind: Some(adjust_kind(ctx, Structure::Environment.completion_kind())),
                data: Some(CompletionItemData::Environment.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::new_simple(name.into(), component_detail(file_names))
            }
        }
        ItemData::UserCommand { name } => {
            let detail = "user-defined".into();
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                kind: Some(adjust_kind(ctx, Structure::Command.completion_kind())),
                data: Some(CompletionItemData::Command.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::new_simple(name.into(), detail)
            }
        }
        ItemData::UserEnvironment { name } => {
            let detail = "user-defined".into();
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                kind: Some(adjust_kind(ctx, Structure::Environment.completion_kind())),
                data: Some(CompletionItemData::Environment.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::new_simple(name.into(), detail)
            }
        }
        ItemData::Label {
            name,
            kind,
            header,
            footer,
            text,
        } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, kind.completion_kind())),
                data: Some(CompletionItemData::Label.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                filter_text: Some(text.clone()),
                sort_text: Some(text),
                detail: header,
                documentation: footer.map(Documentation::String),
                ..CompletionItem::default()
            }
        }
        ItemData::Class { name } => {
            let text_edit = TextEdit::new(item.range, name.as_ref().into());
            CompletionItem {
                label: name.into_owned(),
                kind: Some(adjust_kind(ctx, Structure::Class.completion_kind())),
                data: Some(CompletionItemData::Class.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::Package { name } => {
            let text_edit = TextEdit::new(item.range, name.as_ref().into());
            CompletionItem {
                label: name.into_owned(),
                kind: Some(adjust_kind(ctx, Structure::Package.completion_kind())),
                data: Some(CompletionItemData::Package.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::PgfLibrary { name } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::PgfLibrary.completion_kind())),
                data: Some(CompletionItemData::PgfLibrary.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::TikzLibrary { name } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::TikzLibrary.completion_kind())),
                data: Some(CompletionItemData::TikzLibrary.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::File { name } => {
            let text_edit = TextEdit::new(item.range, name.clone());
            CompletionItem {
                label: name,
                kind: Some(adjust_kind(ctx, Structure::File.completion_kind())),
                data: Some(CompletionItemData::File.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::Directory { name } => {
            let text_edit = TextEdit::new(item.range, name.clone());
            CompletionItem {
                label: name,
                kind: Some(adjust_kind(ctx, Structure::Folder.completion_kind())),
                data: Some(CompletionItemData::Folder.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::Citation { uri, key, text, ty } => {
            let text_edit = TextEdit::new(item.range, key.into());
            CompletionItem {
                label: key.into(),
                kind: Some(adjust_kind(ctx, ty.completion_kind())),
                filter_text: Some(text.clone()),
                sort_text: Some(text),
                data: Some(
                    CompletionItemData::Citation {
                        uri: uri.clone(),
                        key: key.into(),
                    }
                    .into(),
                ),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::Argument { name, image } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::Argument.completion_kind())),
                data: Some(CompletionItemData::Argument.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                documentation: image.and_then(|image| image_documentation(&ctx, &name, image)),
                ..CompletionItem::default()
            }
        }
        ItemData::BeginCommand => CompletionItem {
            kind: Some(adjust_kind(ctx, Structure::Snippet.completion_kind())),
            data: Some(CompletionItemData::CommandSnippet.into()),
            insert_text: Some("begin{$1}\n\t\n\\end{$1}".into()),
            insert_text_format: Some(InsertTextFormat::Snippet),
            ..CompletionItem::new_simple("begin".into(), component_detail(&[]))
        },
        ItemData::Color { name } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::Color.completion_kind())),
                data: Some(CompletionItemData::Color.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::ColorModel { name } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::ColorModel.completion_kind())),
                data: Some(CompletionItemData::ColorModel.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::GlossaryEntry { name } => {
            let text_edit = TextEdit::new(item.range, name.into());
            CompletionItem {
                label: name.into(),
                kind: Some(adjust_kind(ctx, Structure::GlossaryEntry.completion_kind())),
                data: Some(CompletionItemData::GlossaryEntry.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                ..CompletionItem::default()
            }
        }
        ItemData::EntryType { ty } => {
            let text_edit = TextEdit::new(item.range, (&ty.name).into());
            let kind = Structure::Entry(ty.category).completion_kind();
            CompletionItem {
                label: (&ty.name).into(),
                kind: Some(adjust_kind(ctx, kind)),
                data: Some(CompletionItemData::EntryType.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                documentation: ty.documentation.as_ref().map(|doc| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc.into(),
                    })
                }),
                ..CompletionItem::default()
            }
        }
        ItemData::Field { field } => {
            let text_edit = TextEdit::new(item.range, (&field.name).into());
            CompletionItem {
                label: (&field.name).into(),
                kind: Some(adjust_kind(ctx, Structure::Field.completion_kind())),
                data: Some(CompletionItemData::FieldName.into()),
                text_edit: Some(CompletionTextEdit::Edit(text_edit)),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: (&field.documentation).into(),
                })),
                ..CompletionItem::default()
            }
        }
    };
    new_item.preselect = Some(item.preselect);
    new_item
}

fn append_sort_text(mut item: CompletionItem, index: usize) -> CompletionItem {
    let sort_prefix = format!("{:0>2}", index);
    match &item.sort_text {
        Some(sort_text) => {
            item.sort_text = Some(format!("{} {}", sort_prefix, sort_text));
        }
        None => {
            item.sort_text = Some(sort_prefix);
        }
    };
    item
}
