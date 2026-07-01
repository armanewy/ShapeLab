#![forbid(unsafe_code)]

//! Semantic foundry source, control, command, and pack contracts.
//!
//! This crate intentionally contains contracts and validation only. Runtime
//! catalog resolution, compilation, candidate generation, and persistence are
//! implemented in later waves.

pub mod archetype;
pub mod author_studio;
pub mod candidate;
pub mod catalog;
pub mod command;
pub mod compile;
pub mod control;
pub mod document;
pub mod edit;
pub mod feature_module;
pub mod flat_panel;
pub mod foundation;
pub mod geometry_export;
pub mod kit;
pub mod llm_adapter;
pub mod object_intent;
pub mod object_plan;
pub mod pack;
pub mod personal_kit;
pub mod preference;
pub mod preview_display;
pub mod primitive_composition;
pub mod primitive_preset;
pub mod primitive_property;
pub mod primitive_surface;
pub mod project;
pub mod prototype_pack;
pub mod session;
pub mod validation;
pub mod variation;

pub use archetype::*;
pub use author_studio::*;
pub use candidate::*;
pub use catalog::*;
pub use command::*;
pub use compile::*;
pub use control::*;
pub use document::*;
pub use edit::*;
pub use feature_module::*;
pub use flat_panel::*;
pub use foundation::*;
pub use geometry_export::*;
pub use kit::*;
pub use llm_adapter::*;
pub use object_intent::*;
pub use object_plan::*;
pub use pack::*;
pub use personal_kit::*;
pub use preference::*;
pub use preview_display::*;
pub use primitive_composition::*;
pub use primitive_preset::*;
pub use primitive_property::*;
pub use primitive_surface::*;
pub use project::*;
pub use prototype_pack::*;
pub use session::*;
pub use validation::*;
pub use variation::*;

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
