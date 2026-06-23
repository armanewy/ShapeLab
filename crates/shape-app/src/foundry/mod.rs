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
pub(crate) use commands::{FoundryAppCommand, FoundryAppEffect};
#[allow(unused_imports)]
pub(crate) use jobs::{FoundryJobEvent, FoundryJobRequest, FoundryJobSlot, run_foundry_job};
#[allow(unused_imports)]
pub(crate) use state::{FoundryAppState, FoundryAppStateError, FoundryPreviewImage};
#[allow(unused_imports)]
pub(crate) use view_model::{
    FoundryCandidateCard, FoundryControlView, FoundryOptionCard, FoundryPackView,
};
