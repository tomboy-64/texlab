use crate::{
    components::COMPONENT_DATABASE,
    config::ConfigManager,
    diagnostics::DiagnosticsManager,
    features::{
        build::BuildEngine,
        completion::{complete, CompletionItemData, COMPLETION_LIMIT},
        definition::goto_definition,
        folding::fold,
        highlight::highlight,
        hover::hover,
        link::link,
        reference::find_all_references,
        rename::{prepare_rename, rename},
        symbol::{find_document_symbols, find_workspace_symbols},
        FeatureContext,
    },
    forward_search,
    protocol::{
        AsUri, BibtexFormatter, BuildParams, BuildResult, ForwardSearchResult, Options, Uri,
    },
    syntax::{bibtex, latexindent, CharStream, SyntaxNode},
    tex::{Distribution, DistributionKind, KpsewhichError},
    workspace::{DocumentContent, Workspace},
};
use futures::{
    lock::Mutex,
    task::{Spawn, SpawnExt},
};
use language_server::{
    async_trait::async_trait,
    jsonrpc,
    types::{
        ClientCapabilities, ClientInfo, CompletionItem, CompletionList, CompletionOptions,
        CompletionParams, CompletionResponse, DidChangeConfigurationParams,
        DidChangeTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
        DocumentFormattingParams, DocumentHighlight, DocumentHighlightParams, DocumentLink,
        DocumentLinkOptions, DocumentLinkParams, DocumentSymbolParams, DocumentSymbolResponse,
        Documentation, ExecuteCommandOptions, ExecuteCommandParams, FoldingRange,
        FoldingRangeParams, FoldingRangeProviderCapability, GotoDefinitionParams,
        GotoDefinitionResponse, Hover, HoverParams, InitializeParams, InitializeResult,
        InitializedParams, Location, MessageType, Position, PrepareRenameResponse,
        PublishDiagnosticsParams, Range, ReferenceParams, RenameOptions, RenameParams,
        RenameProviderCapability, SaveOptions, ServerCapabilities, ServerInfo, ShowMessageParams,
        SymbolInformation, TextDocumentIdentifier, TextDocumentPositionParams,
        TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions, TextEdit,
        WorkDoneProgressCancelParams, WorkDoneProgressOptions, WorkspaceEdit,
        WorkspaceSymbolParams,
    },
    LanguageClient, LanguageServer, Middleware, Result,
};
use log::{debug, error, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use typed_builder::TypedBuilder;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum LintReason {
    Change,
    Save,
}

struct State<E> {
    executor: E,
    distro: Arc<dyn Distribution>,
    current_dir: Arc<PathBuf>,
    client_info: OnceCell<Option<ClientInfo>>,
    client_capabilities: OnceCell<Arc<ClientCapabilities>>,
    config_manager: OnceCell<ConfigManager>,
    workspace: Workspace,
    build_engine: BuildEngine,
    diagnostics_manager: DiagnosticsManager,
    last_pos_by_uri: Mutex<HashMap<Uri, Position>>,
}

impl<E> State<E> {
    pub fn new(params: LatexLanguageServerParams<E>) -> Self {
        let LatexLanguageServerParams {
            executor,
            distro,
            current_dir,
        } = params;
        let workspace = Workspace::new(distro.clone(), Arc::clone(&current_dir));
        Self {
            executor,
            distro,
            current_dir,
            client_info: OnceCell::new(),
            client_capabilities: OnceCell::new(),
            config_manager: OnceCell::new(),
            workspace,
            build_engine: BuildEngine::new(),
            diagnostics_manager: DiagnosticsManager::default(),
            last_pos_by_uri: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(TypedBuilder)]
pub struct LatexLanguageServerParams<E> {
    executor: E,
    distro: Arc<dyn Distribution>,
    current_dir: Arc<PathBuf>,
}

#[derive(Clone)]
pub struct LatexLanguageServer<E> {
    state: Arc<State<E>>,
}

impl<E: Send + Sync + Spawn + Clone + 'static> LatexLanguageServer<E> {
    pub fn new(params: LatexLanguageServerParams<E>) -> Self {
        Self {
            state: Arc::new(State::new(params)),
        }
    }

    async fn pull_configuration(&self, client: &dyn LanguageClient) -> Options {
        let config_manager = self.state.config_manager.get().unwrap();
        let has_changed = config_manager.pull(client).await;
        let options = config_manager.get().await;
        if has_changed {
            self.state.workspace.reparse(&options).await;
        }
        options
    }

    async fn load_distribution(&self, client: &dyn LanguageClient) {
        info!("Detected TeX distribution: {}", self.state.distro.kind());
        if self.state.distro.kind() == DistributionKind::Unknown {
            let params = ShowMessageParams {
                message: "Your TeX distribution could not be detected. \
                          Please make sure that your distribution is in your PATH."
                    .into(),
                typ: MessageType::Error,
            };
            client.show_message(params).await;
        }

        if let Err(why) = self.state.distro.load().await {
            let message = match why {
                KpsewhichError::NotInstalled | KpsewhichError::InvalidOutput => {
                    "An error occurred while executing `kpsewhich`.\
                     Please make sure that your distribution is in your PATH \
                     environment variable and provides the `kpsewhich` tool."
                }
                KpsewhichError::CorruptDatabase | KpsewhichError::NoDatabase => {
                    "The file database of your TeX distribution seems \
                     to be corrupt. Please rebuild it and try again."
                }
                KpsewhichError::Decode(_) => {
                    "An error occurred while decoding the output of `kpsewhich`."
                }
                KpsewhichError::IO(why) => {
                    error!("An I/O error occurred while executing 'kpsewhich': {}", why);
                    "An I/O error occurred while executing 'kpsewhich'"
                }
            };
            let params = ShowMessageParams {
                message: message.into(),
                typ: MessageType::Error,
            };
            client.show_message(params).await;
        };
    }

    async fn publish_diagnostics(&self, client: &dyn LanguageClient) {
        let snapshot = self.state.workspace.get().await;
        for doc in &snapshot.0 {
            let diagnostics = self.state.diagnostics_manager.get(doc).await;
            let params = PublishDiagnosticsParams {
                uri: doc.uri.clone().into(),
                diagnostics,
                version: None,
            };
            client.publish_diagnostics(params).await;
        }
    }

    async fn run_linter(&self, uri: &Uri, reason: LintReason) {
        let options = self
            .state
            .config_manager
            .get()
            .unwrap()
            .get()
            .await
            .latex
            .and_then(|opts| opts.lint)
            .unwrap_or_default();

        let should_lint = match reason {
            LintReason::Change => options.on_change(),
            LintReason::Save => options.on_save() || options.on_change(),
        };

        if should_lint {
            let snapshot = self.state.workspace.get().await;
            if let Some(doc) = snapshot.find(uri) {
                if let DocumentContent::Latex(_) = &doc.content {
                    self.state
                        .diagnostics_manager
                        .latex
                        .update(uri, &doc.text)
                        .await;
                }
            }
        }
    }

    async fn build(
        &self,
        params: BuildParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<BuildResult> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client.as_ref())
            .await?;

        let pos = {
            self.state
                .last_pos_by_uri
                .lock()
                .await
                .get(&ctx.current().uri)
                .map(|pos| *pos)
                .unwrap_or_default()
        };

        let res = self
            .state
            .build_engine
            .execute(&ctx, Arc::clone(&client))
            .await;

        if ctx
            .options
            .latex
            .and_then(|opts| opts.build)
            .unwrap_or_default()
            .forward_search_after()
            && !self.state.build_engine.is_busy().await
        {
            let params = TextDocumentPositionParams::new(ctx.params.text_document, pos);
            self.forward_search(params, client.as_ref()).await?;
        }

        Ok(res)
    }

    async fn forward_search(
        &self,
        params: TextDocumentPositionParams,
        client: &dyn LanguageClient,
    ) -> Result<ForwardSearchResult> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client)
            .await?;

        forward_search::search(
            &ctx.view.snapshot,
            &ctx.current().uri,
            ctx.params.position.line,
            &ctx.options,
            &self.state.current_dir,
        )
        .await
        .ok_or_else(|| jsonrpc::Error::internal_error("Unable to execute forward search".into()))
    }

    async fn make_feature_context<P>(
        &self,
        uri: Uri,
        params: P,
        client: &dyn LanguageClient,
    ) -> Result<FeatureContext<P>> {
        let options = self.pull_configuration(client).await;
        let snapshot = self.state.workspace.get().await;
        let client_capabilities = Arc::clone(&self.state.client_capabilities.get().unwrap());
        match snapshot.find(&uri) {
            Some(current) => Ok(FeatureContext {
                params,
                view: crate::features::DocumentView::analyze(
                    snapshot,
                    current,
                    &options,
                    &self.state.current_dir,
                ),
                distro: Arc::clone(&self.state.distro),
                client_capabilities,
                options,
                current_dir: Arc::clone(&self.state.current_dir),
            }),
            None => {
                let msg = format!("Unknown document: {}", uri);
                Err(jsonrpc::Error::internal_error(msg))
            }
        }
    }

    async fn run_latexindent(old_text: &str, extension: &str, edits: &mut Vec<TextEdit>) {
        match latexindent::format(old_text, extension).await {
            Ok(new_text) => {
                let mut stream = CharStream::new(&old_text);
                while stream.next().is_some() {}
                let range = Range::new(Position::new(0, 0), stream.current_position);
                edits.push(TextEdit::new(range, new_text));
            }
            Err(why) => {
                debug!("Failed to run latexindent.pl: {}", why);
            }
        }
    }

    async fn update_build_diagnostics(&self, client: Arc<dyn LanguageClient>) {
        let snapshot = self.state.workspace.get().await;
        let options = self.state.config_manager.get().unwrap().get().await;

        for doc in snapshot.0.iter().filter(|doc| doc.uri.scheme() == "file") {
            if let DocumentContent::Latex(table) = &doc.content {
                if table.is_standalone {
                    match self
                        .state
                        .diagnostics_manager
                        .build
                        .update(&snapshot, &doc.uri, &options, &self.state.current_dir)
                        .await
                    {
                        Ok(true) => {
                            let server = self.clone();
                            let client = Arc::clone(&client);
                            self.state
                                .executor
                                .spawn(async move { server.publish_diagnostics(client.as_ref()).await })
                                .unwrap();
                        }
                        Ok(false) => (),
                        Err(why) => {
                            warn!("Unable to read log file ({}): {}", why, doc.uri.as_str())
                        }
                    }
                }
            }
        }
    }
}

// TODO: Add $/cancelRequest to language-server crate
#[async_trait]
impl<E: Send + Sync + Spawn + Clone + 'static> LanguageServer for LatexLanguageServer<E> {
    async fn initialize(
        &self,
        params: InitializeParams,
        _client: Arc<dyn LanguageClient>,
    ) -> Result<InitializeResult> {
        let client_capabilties = Arc::new(params.capabilities);
        self.state
            .config_manager
            .set(ConfigManager::new(Arc::clone(&client_capabilties)))
            .expect("initialize was called two times");
        self.state
            .client_capabilities
            .set(client_capabilties)
            .unwrap();
        self.state.client_info.set(params.client_info).unwrap();

        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::Full),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(SaveOptions {
                        include_text: Some(false),
                    }),
                },
            )),
            hover_provider: Some(true),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(true),
                trigger_characters: Some(vec![
                    "\\".into(),
                    "{".into(),
                    "}".into(),
                    "@".into(),
                    "/".into(),
                    " ".into(),
                ]),
                ..CompletionOptions::default()
            }),
            definition_provider: Some(true),
            references_provider: Some(true),
            document_highlight_provider: Some(true),
            document_symbol_provider: Some(true),
            workspace_symbol_provider: Some(true),
            document_formatting_provider: Some(true),
            rename_provider: Some(RenameProviderCapability::Options(RenameOptions {
                prepare_provider: Some(true),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            document_link_provider: Some(DocumentLinkOptions {
                resolve_provider: Some(false),
                work_done_progress_options: WorkDoneProgressOptions::default(),
            }),
            folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
            execute_command_provider: Some(ExecuteCommandOptions {
                commands: vec!["build".into(), "forwardSearch".into()],
                work_done_progress_options: WorkDoneProgressOptions {
                    work_done_progress: None,
                },
            }),
            ..ServerCapabilities::default()
        };

        Lazy::force(&COMPONENT_DATABASE);
        Ok(InitializeResult {
            capabilities,
            server_info: Some(ServerInfo {
                name: "TexLab".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams, client: Arc<dyn LanguageClient>) {
        let server = self.clone();
        self.state
            .executor
            .spawn(async move {
                server.pull_configuration(client.as_ref()).await;
                server
                    .state
                    .config_manager
                    .get()
                    .unwrap()
                    .register(client.as_ref())
                    .await;
                server.load_distribution(client.as_ref()).await;
                server.publish_diagnostics(client.as_ref()).await;
            })
            .unwrap();
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams, client: Arc<dyn LanguageClient>) {
        let uri = params.text_document.uri.clone();
        let options = self.state.config_manager.get().unwrap().get().await;
        self.state
            .workspace
            .add(params.text_document, &options)
            .await;
        let server = self.clone();
        self.state
            .executor
            .spawn(async move {
                let _ = server
                    .state
                    .workspace
                    .detect_root(&uri.clone().into(), &options)
                    .await;

                server.run_linter(&uri.into(), LintReason::Save).await;

                server.publish_diagnostics(client.as_ref()).await;
            })
            .unwrap();
    }

    async fn did_change(
        &self,
        params: DidChangeTextDocumentParams,
        client: Arc<dyn LanguageClient>,
    ) {
        let options = self.state.config_manager.get().unwrap().get().await;
        for change in params.content_changes {
            let uri = params.text_document.uri.clone();
            self.state
                .workspace
                .update(uri.into(), change.text, &options)
                .await;
        }

        let uri = params.text_document.uri;
        let server = self.clone();
        self.state
            .executor
            .spawn(async move {
                server.run_linter(&uri.into(), LintReason::Change).await;
                server.publish_diagnostics(client.as_ref()).await;
            })
            .unwrap();
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams, client: Arc<dyn LanguageClient>) {
        let server = self.clone();
        self.state
            .executor
            .spawn(async move {
                let options = server
                    .state
                    .config_manager
                    .get()
                    .unwrap()
                    .get()
                    .await
                    .latex
                    .and_then(|opts| opts.build)
                    .unwrap_or_default();

                let uri = params.text_document.uri;

                if options.on_save() {
                    let text_document = TextDocumentIdentifier::new(uri.clone());
                    server
                        .build(BuildParams { text_document }, Arc::clone(&client))
                        .await
                        .unwrap();
                }

                server.run_linter(&uri.into(), LintReason::Save).await;
                server.publish_diagnostics(client.as_ref()).await;
            })
            .unwrap();
    }

    async fn did_change_configuration(
        &self,
        params: DidChangeConfigurationParams,
        _client: Arc<dyn LanguageClient>,
    ) {
        let config_manager = self.state.config_manager.get().unwrap();
        config_manager.push(params.settings).await;
        let options = config_manager.get().await;
        self.state.workspace.reparse(&options).await;
    }

    async fn work_done_progress_cancel(
        &self,
        params: WorkDoneProgressCancelParams,
        _client: Arc<dyn LanguageClient>,
    ) {
        self.state.build_engine.cancel(params.token).await;
    }

    async fn completion(
        &self,
        params: CompletionParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<CompletionResponse> {
        let ctx = self
            .make_feature_context(
                params.text_document_position.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;

        {
            self.state.last_pos_by_uri.lock().await.insert(
                ctx.current().uri.clone(),
                ctx.params.text_document_position.position,
            );
        }

        let items = complete(ctx).await;

        let is_incomplete = if self
            .state
            .client_info
            .get()
            .and_then(|info| info.as_ref())
            .map(|info| info.name.as_str())
            .unwrap_or_default()
            == "vscode"
        {
            true
        } else {
            items.len() >= COMPLETION_LIMIT
        };

        Ok(CompletionResponse::List(CompletionList {
            is_incomplete,
            items,
        }))
    }

    async fn completion_resolve(
        &self,
        mut item: CompletionItem,
        _client: Arc<dyn LanguageClient>,
    ) -> Result<CompletionItem> {
        let data: CompletionItemData = serde_json::from_value(item.data.clone().unwrap()).unwrap();
        match data {
            CompletionItemData::Package | CompletionItemData::Class => {
                item.documentation = COMPONENT_DATABASE
                    .documentation(&item.label)
                    .map(Documentation::MarkupContent);
            }
            #[cfg(feature = "citation")]
            CompletionItemData::Citation { uri, key } => {
                let snapshot = self.state.workspace.get().await;
                if let Some(doc) = snapshot.find(&uri) {
                    if let DocumentContent::Bibtex(tree) = &doc.content {
                        let markup = crate::citeproc::render_citation(&tree, &key);
                        item.documentation = markup.map(Documentation::MarkupContent);
                    }
                }
            }
            _ => {}
        };
        Ok(item)
    }

    async fn hover(
        &self,
        params: HoverParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Option<Hover>> {
        let ctx = self
            .make_feature_context(
                params.text_document_position_params.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;

        self.state.last_pos_by_uri.lock().await.insert(
            ctx.current().uri.clone(),
            ctx.params.text_document_position_params.position,
        );

        Ok(hover(ctx).await)
    }

    async fn definition(
        &self,
        params: GotoDefinitionParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<GotoDefinitionResponse> {
        let ctx = self
            .make_feature_context(
                params.text_document_position_params.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;

        Ok(goto_definition(ctx))
    }

    async fn references(
        &self,
        params: ReferenceParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<Location>> {
        let ctx = self
            .make_feature_context(
                params.text_document_position.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;
        Ok(find_all_references(ctx))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<DocumentHighlight>> {
        let ctx = self
            .make_feature_context(
                params.text_document_position_params.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;
        Ok(highlight(ctx))
    }

    async fn workspace_symbol(
        &self,
        params: WorkspaceSymbolParams,
        _client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<SymbolInformation>> {
        let distro = self.state.distro.clone();
        let client_capabilities = Arc::clone(self.state.client_capabilities.get().unwrap());
        let snapshot = self.state.workspace.get().await;
        let options = self.state.config_manager.get().unwrap().get().await;
        let symbols = find_workspace_symbols(
            distro,
            client_capabilities,
            snapshot,
            &options,
            Arc::clone(&self.state.current_dir),
            &params,
        );
        Ok(symbols)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<DocumentSymbolResponse> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client.as_ref())
            .await?;

        Ok(find_document_symbols(ctx))
    }

    async fn document_link(
        &self,
        params: DocumentLinkParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<DocumentLink>> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client.as_ref())
            .await?;

        Ok(link(ctx))
    }

    async fn formatting(
        &self,
        params: DocumentFormattingParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<TextEdit>> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client.as_ref())
            .await?;
        let mut edits = Vec::new();
        match &ctx.current().content {
            DocumentContent::Latex(_) => {
                Self::run_latexindent(&ctx.current().text, "tex", &mut edits).await;
            }
            DocumentContent::Bibtex(tree) => {
                let options = ctx
                    .options
                    .bibtex
                    .clone()
                    .and_then(|opts| opts.formatting)
                    .unwrap_or_default();

                match options.formatter.unwrap_or_default() {
                    BibtexFormatter::Texlab => {
                        let params = bibtex::FormattingParams {
                            tab_size: ctx.params.options.tab_size as usize,
                            insert_spaces: ctx.params.options.insert_spaces,
                            options: &options,
                        };

                        for node in tree.children(tree.root) {
                            let should_format = match &tree.graph[node] {
                                bibtex::Node::Preamble(_) | bibtex::Node::String(_) => true,
                                bibtex::Node::Entry(entry) => !entry.is_comment(),
                                _ => false,
                            };
                            if should_format {
                                let text = bibtex::format(&tree, node, params);
                                edits.push(TextEdit::new(tree.graph[node].range(), text));
                            }
                        }
                    }
                    BibtexFormatter::Latexindent => {
                        Self::run_latexindent(&ctx.current().text, "bib", &mut edits).await;
                    }
                }
            }
        }
        Ok(edits)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Option<PrepareRenameResponse>> {
        let ctx = self
            .make_feature_context(params.as_uri(), params, client.as_ref())
            .await?;
        Ok(prepare_rename(ctx).map(PrepareRenameResponse::Range))
    }

    async fn rename(
        &self,
        params: RenameParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Option<WorkspaceEdit>> {
        let ctx = self
            .make_feature_context(
                params.text_document_position.as_uri(),
                params,
                client.as_ref(),
            )
            .await?;
        Ok(rename(ctx))
    }

    async fn folding_range(
        &self,
        params: FoldingRangeParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Vec<FoldingRange>> {
        let ctx = self
            .make_feature_context(params.text_document.as_uri(), params, client.as_ref())
            .await?;
        Ok(fold(ctx))
    }

    async fn execute_command(
        &self,
        mut params: ExecuteCommandParams,
        client: Arc<dyn LanguageClient>,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "build" => {
                if params.arguments.len() != 1 {
                    return Err(jsonrpc::Error::internal_error(
                        "Invalid number of arguments".into(),
                    ));
                }
                let params = serde_json::from_value(params.arguments.pop().unwrap())
                    .map_err(|why| jsonrpc::Error::internal_error(format!("{}", why)))?;

                let result = self.build(params, client).await?;
                Ok(serde_json::to_value(result).ok())
            }
            "forwardSearch" => {
                if params.arguments.len() != 1 {
                    return Err(jsonrpc::Error::internal_error(
                        "Invalid number of arguments".into(),
                    ));
                }

                let params = serde_json::from_value(params.arguments.pop().unwrap())
                    .map_err(|why| jsonrpc::Error::internal_error(format!("{}", why)))?;
                let result = self.forward_search(params, client.as_ref()).await?;
                Ok(serde_json::to_value(result).ok())
            }
            _ => Err(jsonrpc::Error::internal_error(format!(
                "Unknown command: {}",
                params.command.as_str()
            ))),
        }
    }
}

#[async_trait]
impl<E: Spawn + Clone + Send + Sync + 'static> Middleware for LatexLanguageServer<E> {
    async fn on_incoming_message(
        &self,
        _message: &mut jsonrpc::Message,
        _client: Arc<dyn LanguageClient>,
    ) {
        if let Some(config_manager) = self.state.config_manager.get() {
            let options = config_manager.get().await;
            self.state.workspace.detect_children(&options).await;
            self.state.workspace.reparse_all_if_newer(&options).await;
        }
    }
    async fn on_outgoing_response(
        &self,
        _request: &jsonrpc::Request,
        _response: &mut jsonrpc::Response,
        client: Arc<dyn LanguageClient>,
    ) {
        self.update_build_diagnostics(client).await;
    }

    async fn on_outgoing_request(
        &self,
        _request: &mut jsonrpc::Request,
        _client: Arc<dyn LanguageClient>,
    ) {
    }

    async fn on_outgoing_notification(
        &self,
        _notification: &mut jsonrpc::Notification,
        _client: Arc<dyn LanguageClient>,
    ) {
    }
}
