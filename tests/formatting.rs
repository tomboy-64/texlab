#![feature(async_await)]

use futures::executor::block_on;
use lsp_types::*;
use std::collections::HashMap;
use texlab::formatting::bibtex::BibtexFormattingOptions;
use texlab::scenario::Scenario;

pub async fn run(
    scenario: &'static str,
    file: &'static str,
    options: Option<BibtexFormattingOptions>,
) -> (Scenario, Vec<TextEdit>) {
    let scenario = format!("formatting/{}", scenario);
    let scenario = Scenario::new(&scenario).await;
    scenario.open(file).await;
    scenario.client.options.lock().await.bibtex_formatting = options;

    let params = DocumentFormattingParams {
        text_document: TextDocumentIdentifier::new(scenario.uri(file)),
        options: FormattingOptions {
            tab_size: 4,
            insert_spaces: true,
            properties: HashMap::new(),
        },
    };
    let edits = scenario.server.formatting(params).await.unwrap();
    (scenario, edits)
}

#[test]
fn test_bibtex_entry_default() {
    block_on(async move {
        let (scenario, edits) = run("bibtex/default", "foo.bib", None).await;
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, scenario.read("bar.bib").await);
        assert_eq!(edits[0].range, Range::new_simple(0, 0, 0, 52));
    });
}

#[test]
fn test_bibtex_entry_infinite_line_length() {
    block_on(async move {
        let (scenario, edits) = run(
            "bibtex/infinite_line_length",
            "foo.bib",
            Some(BibtexFormattingOptions { line_length: 0 }),
        )
        .await;
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, scenario.read("bar.bib").await);
        assert_eq!(edits[0].range, Range::new_simple(0, 0, 0, 149));
    });
}

#[test]
fn test_latex() {
    block_on(async move {
        let (_, edits) = run("latex", "foo.tex", None).await;
        assert_eq!(edits, Vec::new());
    })
}
