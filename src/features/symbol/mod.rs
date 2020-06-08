mod entry;
mod project_order;
mod section;
mod string;
mod types;

use self::{
    entry::find_entry_symbols,
    project_order::ProjectOrdering,
    section::find_section_symbols,
    string::find_string_symbols,
    types::{SymbolContext, WorkspaceSymbol},
};
use crate::{features::prelude::*, tex::Distribution};
use std::{
    cmp::Reverse,
    path::{Path, PathBuf},
    sync::Arc,
};

pub fn find_document_symbols(ctx: FeatureContext<DocumentSymbolParams>) -> DocumentSymbolResponse {
    let ctx = find_all_symbols(ctx);
    if ctx
        .inner
        .client_capabilities
        .has_hierarchical_document_symbol_support()
    {
        DocumentSymbolResponse::Nested(
            ctx.items
                .into_iter()
                .map(|item| item.into_document_symbol())
                .collect(),
        )
    } else {
        let mut buffer = Vec::new();
        for item in ctx.items {
            item.flatten(&mut buffer);
        }
        let uri = &ctx.inner.current().uri;
        let mut buffer = buffer
            .into_iter()
            .map(|item| item.into_symbol_info(uri.clone()))
            .collect();
        sort_symbols(
            ctx.inner.snapshot(),
            &ctx.inner.options,
            &ctx.inner.current_dir,
            &mut buffer,
        );
        DocumentSymbolResponse::Flat(buffer)
    }
}

pub fn find_workspace_symbols<'a>(
    distro: Arc<dyn Distribution>,
    client_capabilities: Arc<ClientCapabilities>,
    snapshot: Arc<Snapshot>,
    options: &'a Options,
    current_dir: Arc<PathBuf>,
    params: &'a WorkspaceSymbolParams,
) -> Vec<SymbolInformation> {
    let mut symbols = Vec::new();

    for doc in &snapshot.0 {
        let uri: Uri = doc.uri.clone();
        let ctx = FeatureContext {
            params: DocumentSymbolParams {
                text_document: TextDocumentIdentifier::new(uri.clone().into()),
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            },
            view: DocumentView::analyze(
                Arc::clone(&snapshot),
                Arc::clone(&doc),
                &options,
                &current_dir,
            ),
            distro: distro.clone(),
            client_capabilities: Arc::clone(&client_capabilities),
            options: options.clone(),
            current_dir: Arc::clone(&current_dir),
        };

        let mut buffer = Vec::new();
        for symbol in find_all_symbols(ctx).items {
            symbol.flatten(&mut buffer);
        }

        for symbol in buffer {
            symbols.push(WorkspaceSymbol {
                search_text: symbol.search_text(),
                info: symbol.into_symbol_info(uri.clone()),
            });
        }
    }

    let query_words: Vec<_> = params
        .query
        .split_whitespace()
        .map(str::to_lowercase)
        .collect();
    let mut filtered = Vec::new();
    for symbol in symbols {
        let mut included = true;
        for word in &query_words {
            if !symbol.search_text.contains(word) {
                included = false;
                break;
            }
        }

        if included {
            filtered.push(symbol.info);
        }
    }
    sort_symbols(&snapshot, options, &current_dir, &mut filtered);
    filtered
}

fn sort_symbols(
    snapshot: &Snapshot,
    options: &Options,
    current_dir: &Path,
    symbols: &mut Vec<SymbolInformation>,
) {
    let ordering = ProjectOrdering::analyze(snapshot, options, current_dir);
    symbols.sort_by(|left, right| {
        let left_key = (
            ordering.get(&Uri::from(left.location.uri.clone())),
            left.location.range.start,
            Reverse(left.location.range.end),
        );
        let right_key = (
            ordering.get(&Uri::from(right.location.uri.clone())),
            right.location.range.start,
            Reverse(right.location.range.end),
        );
        left_key.cmp(&right_key)
    });
}

fn find_all_symbols(ctx: FeatureContext<DocumentSymbolParams>) -> SymbolContext {
    let mut ctx = SymbolContext::new(ctx);
    find_entry_symbols(&mut ctx);
    find_string_symbols(&mut ctx);
    find_section_symbols(&mut ctx);
    ctx
}
