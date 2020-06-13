mod cmd;
mod entry;
mod env;
mod label;

use self::{
    cmd::{prepare_command_rename, rename_command},
    entry::{prepare_entry_rename, rename_entry},
    env::{prepare_environment_rename, rename_environment},
    label::{prepare_label_rename, rename_label},
};
use crate::features::prelude::*;

pub fn prepare_rename(ctx: FeatureContext<TextDocumentPositionParams>) -> Option<Range> {
    prepare_entry_rename(&ctx)
        .or_else(|| prepare_command_rename(&ctx))
        .or_else(|| prepare_environment_rename(&ctx))
        .or_else(|| prepare_label_rename(&ctx))
}

pub fn rename(ctx: FeatureContext<RenameParams>) -> Option<WorkspaceEdit> {
    rename_entry(&ctx)
        .or_else(|| rename_command(&ctx))
        .or_else(|| rename_environment(&ctx))
        .or_else(|| rename_label(&ctx))
}
