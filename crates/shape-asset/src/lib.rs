#![forbid(unsafe_code)]

//! Serializable, part-aware asset recipe contracts for the explicit modeling lane.
//!
//! These contracts are intentionally separate from the legacy implicit
//! `ShapeDocument` editor. IDs in this crate are semantic: part, operation,
//! region, and socket IDs must remain stable when unrelated scalar parameters change.
//! Generated vertex and face IDs
//! are owned by downstream polygon crates and are deterministic only for a given
//! topology signature. Generated boundary-loop IDs are semantic and stay stable
//! across parameter changes that preserve the feature topology.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize, de};
use thiserror::Error;

pub mod edits;
pub mod parameters;
pub mod patterns;
pub mod property_descriptor;
pub mod relationships;

pub use edits::*;
pub use parameters::*;
pub use patterns::*;
pub use property_descriptor::*;
pub use relationships::*;

/// Current schema version for asset recipes.
pub const ASSET_RECIPE_SCHEMA_VERSION: u32 = 8;
/// Package version for asset-recipe contracts.
pub const SHAPE_ASSET_CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");
const BOUNDARY_BEVEL_PROFILE_MIN: f32 = 0.05;
const BOUNDARY_BEVEL_PROFILE_MAX: f32 = 8.0;
const CUT_SCALAR_SAFETY_MARGIN: f32 = 0.001;
const SCALAR_RANGE_TOLERANCE: f32 = 1.0e-6;

include!("asset_core/ids.rs");
include!("asset_core/spatial.rs");
include!("asset_core/recipe.rs");
include!("asset_core/semantic_shells.rs");
include!("asset_core/geometry_types.rs");
include!("asset_core/scalar_ranges.rs");
include!("asset_core/geometry_wire.rs");
include!("asset_core/references.rs");
include!("asset_core/edit_contracts.rs");
include!("asset_core/validation_api.rs");
include!("asset_core/validation_structure.rs");
include!("asset_core/validation_semantic.rs");
include!("asset_core/validation_identifiers.rs");
include!("asset_core/validation_operations.rs");
include!("asset_core/scalar_access.rs");
include!("asset_core/edit_application.rs");
include!("asset_core/edit_cut_duplication.rs");
include!("asset_core/utility.rs");
include!("asset_core/tests.rs");
