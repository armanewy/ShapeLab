//! Native Foundry application boundary contracts.
//!
//! This module intentionally freezes DTOs and command/job boundaries only.
//! Runtime state reduction and panel rendering are implemented by later waves.

#![allow(dead_code)]

pub(crate) mod commands;
pub(crate) mod jobs;
pub(crate) mod panels;
pub(crate) mod state;
pub(crate) mod view_model;

#[allow(unused_imports)]
pub(crate) use commands::FoundryAppCommand;
#[allow(unused_imports)]
pub(crate) use jobs::{FoundryJobEvent, FoundryJobRequest};
#[allow(unused_imports)]
pub(crate) use panels::history::{
    FoundryHistoryActionDispatch, FoundryHistoryActionIntent, FoundryHistoryActionKind,
    FoundryHistoryView,
};
#[allow(unused_imports)]
pub(crate) use state::FoundryAppState;
#[allow(unused_imports)]
pub(crate) use view_model::{
    FoundryCandidateCard, FoundryControlView, FoundryOptionCard, FoundryPackView,
};

/// Build the native Foundry semantic history snapshot for app-shell consumers.
#[must_use]
pub(crate) fn build_foundry_history_view(state: &FoundryAppState) -> FoundryHistoryView {
    panels::history::build_history_view(state)
}
