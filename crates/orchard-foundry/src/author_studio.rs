//! Foundry Author Studio contracts for internal kit authors.
//!
//! The studio is a technical authoring lane over Foundry kit packages. It does
//! not change the novice Visual Foundry surface and it does not bypass kit
//! validation, quality gates, or review manifests.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    ControlProfileControlKind, ControlProfileTopologyBehavior, DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS,
    FoundryKitPackage,
};

include!("author_studio/contracts.rs");
include!("author_studio/descriptor_validation.rs");
include!("author_studio/quality_exports.rs");
include!("author_studio/tests.rs");
