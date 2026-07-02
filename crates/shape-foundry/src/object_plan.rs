//! Structured ObjectPlan contracts for offline primitive planning.
//!
//! ObjectPlans are bounded primitive and composition descriptions. They are
//! intentionally closed to raw mesh payloads, arbitrary transforms, and public
//! publishing shortcuts.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    ContactPolicy, ExportRealizationPolicy, OrientationPolicy, PlacementPolicy, PositionRule,
    RelationshipContract, RelationshipId, RelationshipType, ScalePolicy,
};

use crate::{
    PrimitiveAttachment, PrimitiveAttachmentOffsetPolicy, PrimitiveAttachmentOrientationPolicy,
    PrimitiveAttachmentScalePolicy, PrimitiveCompositionDocument,
    PrimitiveCompositionValidationReport, PrimitiveKind, PrimitiveNode, PrimitiveNodeVisibility,
    PrimitivePropertySchema, PrimitivePropertyValidationReport, PrimitivePropertyValue,
    box_primitive_property_schema, flat_panel_primitive_property_schema,
    primitive_anchor_definitions, sphere_primitive_property_schema,
    validate_primitive_composition_document, validate_primitive_property_values,
};

include!("object_plan/contracts.rs");
include!("object_plan/materialization_summary.rs");
include!("object_plan/composition_materialization.rs");
include!("object_plan/validation_text_helpers.rs");
