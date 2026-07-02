//! Direct primitive property schema contracts.
//!
//! These contracts define the bounded, product-facing properties users may edit
//! for primitive Make workflows. They intentionally do not expose mesh topology,
//! raw transforms, provider paths, or arbitrary modeling operations.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    KernelKind, OrchardControlFamily, PropertyAffect, PropertyAuthoringEffect, PropertyDescriptor,
    PropertyDescriptorDomain, PropertyDescriptorValue, PropertyReviewImportance,
};

include!("primitive_property/contracts_schemas.rs");
include!("primitive_property/validation.rs");
include!("primitive_property/descriptors.rs");
