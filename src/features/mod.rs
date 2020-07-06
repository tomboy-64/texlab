pub mod build;
pub mod completion;
pub mod definition;
pub mod folding;
pub mod highlight;
pub mod hover;
pub mod link;
pub mod outline;
pub mod prelude;
pub mod reference;
pub mod rename;
pub mod symbol;
pub mod testing;

use crate::{
    components::{Component, COMPONENT_DATABASE},
    protocol::Options,
    tex::Distribution,
    workspace::{Document, DocumentContent, Snapshot},
};
use itertools::Itertools;
use language_server::types::ClientCapabilities;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct DocumentView {
    pub snapshot: Arc<Snapshot>,
    pub current: Arc<Document>,
    pub related: Vec<Arc<Document>>,
    pub components: Vec<&'static Component>,
}

impl DocumentView {
    pub fn analyze(
        snapshot: Arc<Snapshot>,
        current: Arc<Document>,
        options: &Options,
        current_dir: &Path,
    ) -> Self {
        let related = snapshot.relations(&current.uri, options, current_dir);

        let mut start_components = vec![COMPONENT_DATABASE.kernel()];
        for doc in &related {
            if let DocumentContent::Latex(table) = &doc.content {
                table
                    .components
                    .iter()
                    .flat_map(|file| COMPONENT_DATABASE.find(file))
                    .for_each(|component| start_components.push(component))
            }
        }

        let mut all_components = Vec::new();
        for component in start_components {
            all_components.push(component);
            component
                .references
                .iter()
                .flat_map(|file| COMPONENT_DATABASE.find(&file))
                .for_each(|component| all_components.push(component))
        }

        let components = all_components
            .into_iter()
            .unique_by(|component| &component.file_names)
            .collect();

        Self {
            snapshot,
            current,
            related,
            components,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FeatureContext<P> {
    pub params: P,
    pub view: DocumentView,
    pub distro: Arc<dyn Distribution>,
    pub client_capabilities: Arc<ClientCapabilities>,
    pub options: Options,
    pub current_dir: Arc<PathBuf>,
}

impl<P> FeatureContext<P> {
    pub fn snapshot(&self) -> &Snapshot {
        &self.view.snapshot
    }

    pub fn current(&self) -> &Document {
        &self.view.current
    }

    pub fn related(&self) -> &[Arc<Document>] {
        &self.view.related
    }
}
