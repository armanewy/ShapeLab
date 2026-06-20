//! Schema-3 deformation decompiler contracts.
//!
//! This module is additive to the schema-2 implementation. Schema 3 introduces
//! ordered explanatory operator programs, bend contracts, and diagnostics
//! contracts without changing the existing package writer, verifier, CLI, or
//! Blender adapter behavior.

pub mod bend;
pub mod bend_fit;
pub mod blender;
pub mod decompile;
pub mod diagnostics;
pub mod inference;
pub mod package;
pub mod program;
