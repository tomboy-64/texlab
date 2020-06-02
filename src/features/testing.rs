use crate::{
    features::{DocumentView, FeatureContext},
    protocol::{LatexBuildOptions, LatexOptions, Options, Uri},
    tex::{Language, Resolver, UnknownDistribution},
    workspace::{Document, DocumentParams, Snapshot},
};
use lsp_types::{
    ClientCapabilities, CompletionParams, DocumentLinkParams, DocumentSymbolParams,
    FoldingRangeParams, PartialResultParams, Position, ReferenceContext, ReferenceParams,
    RenameParams, TextDocumentIdentifier, TextDocumentPositionParams, WorkDoneProgressParams,
};
use std::{path::PathBuf, sync::Arc};
use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder)]
pub struct FeatureTester<'a> {
    main: &'a str,
    files: Vec<(&'a str, &'a str)>,
    #[builder(default)]
    line: u64,
    #[builder(default)]
    character: u64,
    #[builder(default)]
    new_name: &'a str,
    #[builder(default)]
    include_declaration: bool,
    #[builder(default)]
    client_capabilities: ClientCapabilities,
    #[builder(default=std::env::temp_dir())]
    current_dir: PathBuf,
    #[builder(default, setter(strip_option))]
    root_dir: Option<PathBuf>,
    #[builder(default, setter(strip_option))]
    output_dir: Option<PathBuf>,
}

impl<'a> FeatureTester<'a> {
    pub fn uri(&self, name: &str) -> Uri {
        let path = self.current_dir.join(name);
        Uri::from_file_path(path).unwrap()
    }

    fn identifier(&self) -> TextDocumentIdentifier {
        let uri = self.uri(&self.main);
        TextDocumentIdentifier::new(uri.into())
    }

    fn view(&self) -> DocumentView {
        let mut snapshot = Snapshot::new();
        let resolver = Resolver::default();
        let options = self.options();
        for (name, text) in &self.files {
            let uri = self.uri(name);
            let path = uri.to_file_path().unwrap();
            let language = path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(Language::by_extension)
                .unwrap();
            let doc = Document::open(DocumentParams {
                uri,
                text: text.trim().into(),
                language,
                resolver: &resolver,
                options: &options,
                current_dir: &self.current_dir,
            });
            snapshot.push(doc);
        }
        let current = snapshot.find(&self.uri(&self.main)).unwrap();
        DocumentView::analyze(Arc::new(snapshot), current, &options, &self.current_dir)
    }

    fn options(&self) -> Options {
        Options {
            latex: Some(LatexOptions {
                build: Some(LatexBuildOptions {
                    output_directory: self.output_dir.clone(),
                    ..LatexBuildOptions::default()
                }),
                root_directory: self.root_dir.clone(),
                ..LatexOptions::default()
            }),
            ..Options::default()
        }
    }

    fn context<P>(&self, params: P) -> FeatureContext<P> {
        FeatureContext {
            params,
            view: self.view(),
            client_capabilities: Arc::new(self.client_capabilities.clone()),
            distro: Arc::new(UnknownDistribution::default()),
            options: self.options(),
            current_dir: Arc::new(self.current_dir.clone()),
        }
    }

    pub fn completion(self) -> FeatureContext<CompletionParams> {
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                Position::new(self.line, self.character),
            ),
            context: None,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.context(params)
    }

    pub fn position(self) -> FeatureContext<TextDocumentPositionParams> {
        let text_document = self.identifier();
        let position = Position::new(self.line, self.character);
        let params = TextDocumentPositionParams::new(text_document, position);
        self.context(params)
    }

    pub fn folding(self) -> FeatureContext<FoldingRangeParams> {
        let text_document = self.identifier();
        let params = FoldingRangeParams {
            text_document,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.context(params)
    }

    pub fn link(self) -> FeatureContext<DocumentLinkParams> {
        let text_document = self.identifier();
        let params = DocumentLinkParams {
            text_document,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.context(params)
    }

    pub fn reference(self) -> FeatureContext<ReferenceParams> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                Position::new(self.line, self.character),
            ),
            context: ReferenceContext {
                include_declaration: self.include_declaration,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.context(params)
    }

    pub fn rename(self) -> FeatureContext<RenameParams> {
        let params = RenameParams {
            text_document_position: TextDocumentPositionParams::new(
                self.identifier(),
                Position::new(self.line, self.character),
            ),
            new_name: self.new_name.into(),
            work_done_progress_params: WorkDoneProgressParams::default(),
        };
        self.context(params)
    }

    pub fn symbol(self) -> FeatureContext<DocumentSymbolParams> {
        let text_document = self.identifier();
        let params = DocumentSymbolParams {
            text_document,
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        self.context(params)
    }
}
