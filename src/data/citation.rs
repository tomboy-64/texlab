use lsp_types::*;
use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::tempdir;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum CitationError {
    WriteFailed,
    InvalidEntry,
    NodeNotInstalled,
    ScriptFaulty,
    InvalidOutput,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Citation<'a> {
    entry_code: &'a str,
}

impl<'a> Citation<'a> {
    pub fn new(entry_code: &'a str) -> Self {
        Self { entry_code }
    }

    pub fn render(&self) -> Result<MarkupContent, CitationError> {
        let now = std::time::Instant::now();

        let directory = tempdir().map_err(|_| CitationError::WriteFailed)?;
        let entry_path = directory.path().join("entry.bib");
        fs::write(entry_path, self.entry_code).map_err(|_| CitationError::WriteFailed)?;

        let mut process = Command::new("node")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(directory.path())
            .spawn()
            .map_err(|_| CitationError::NodeNotInstalled)?;

        process
            .stdin
            .as_mut()
            .unwrap()
            .write_all(SCRIPT.as_bytes())
            .map_err(|_| CitationError::ScriptFaulty)?;

        let output = process
            .wait_with_output()
            .map_err(|_| CitationError::ScriptFaulty)?;

        if output.status.success() {
            let html =
                String::from_utf8(output.stdout).map_err(|_| CitationError::InvalidOutput)?;
            let markdown = html2md::parse_html(&html);

            log::info!("CITATION = {} ms", now.elapsed().as_millis());

            Ok(MarkupContent {
                kind: MarkupKind::Markdown,
                value: Cow::from(markdown),
            })
        } else {
            Err(CitationError::InvalidEntry)
        }
    }
}

const SCRIPT: &str = include_str!("../../citeproc/dist/citeproc.js");
