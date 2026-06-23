#![forbid(unsafe_code)]

//! Semantic foundry source, control, command, and pack contracts.
//!
//! This crate intentionally contains contracts and validation only. Runtime
//! catalog resolution, compilation, candidate generation, and persistence are
//! implemented in later waves.

pub mod candidate;
pub mod catalog;
pub mod command;
pub mod compile;
pub mod control;
pub mod document;
pub mod edit;
pub mod llm_adapter;
pub mod pack;
pub mod project;
pub mod session;
pub mod validation;

pub use candidate::*;
pub use catalog::*;
pub use command::*;
pub use compile::*;
pub use control::*;
pub use document::*;
pub use edit::*;
pub use llm_adapter::*;
pub use pack::*;
pub use project::*;
pub use session::*;
pub use validation::*;

/// Current schema version for foundry asset source documents.
pub const FOUNDRY_ASSET_DOCUMENT_SCHEMA_VERSION: u32 = 1;
/// Current schema version for customizer profiles.
pub const CUSTOMIZER_PROFILE_SCHEMA_VERSION: u32 = 1;
/// Current schema version for foundry packs.
pub const FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION: u32 = 1;
/// Current schema version for replayable foundry project contracts.
pub const FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION: u32 = 1;
/// Package version for foundry contracts.
pub const SHAPE_FOUNDRY_CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
