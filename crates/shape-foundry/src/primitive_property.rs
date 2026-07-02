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

/// Current schema version for primitive property schemas.
pub const PRIMITIVE_PROPERTY_SCHEMA_VERSION: u32 = 1;

/// Stable primitive kinds that can own property schemas.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveKind {
    /// Editable box primitive.
    BoxPrimitive,
    /// Editable flat panel primitive.
    FlatPanelPrimitive,
    /// Editable sphere-like primitive.
    SpherePrimitive,
    /// Future cylinder primitive; not active in the current product flow.
    CylinderPrimitive,
}

/// Whole primitive property schema.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePropertySchema {
    /// Schema version.
    pub schema_version: u32,
    /// Primitive kind this schema describes.
    pub primitive_kind: PrimitiveKind,
    /// Product-facing name.
    pub display_name: String,
    /// Product-safe identity summary.
    pub identity_summary: String,
    /// User-editable properties.
    pub properties: Vec<PrimitiveProperty>,
    /// Product-safe validation and shape constraints.
    pub constraints: Vec<PrimitiveSchemaConstraint>,
    /// Preview behavior policy.
    pub preview_policy: PrimitivePreviewPolicy,
    /// Export behavior policy.
    pub export_policy: PrimitiveExportPolicy,
}

/// One product-safe schema constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveSchemaConstraint {
    /// Stable constraint ID.
    pub constraint_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Product-safe description.
    pub user_facing_description: String,
}

/// One editable primitive property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveProperty {
    /// Stable property ID.
    pub property_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Value kind.
    pub value_kind: PrimitivePropertyValueKind,
    /// Allowed domain.
    pub domain: PrimitivePropertyDomain,
    /// Default value, which must be valid in the domain.
    pub default_value: PrimitivePropertyValue,
    /// Whether the property changes geometry.
    pub affects_geometry: bool,
    /// Whether edits preserve or discretely change topology.
    pub topology_behavior: PrimitiveTopologyBehavior,
    /// Product-safe description.
    pub user_facing_description: String,
    /// Whether this belongs behind advanced UI.
    pub advanced: bool,
}

/// Supported property value kinds.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitivePropertyValueKind {
    /// Physical length-like value.
    Length,
    /// Unitless ratio.
    Ratio,
    /// Boolean toggle.
    Boolean,
    /// Symbolic choice.
    Choice,
    /// Angle in degrees.
    Angle,
}

/// Allowed value domain for a primitive property.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase", deny_unknown_fields)]
pub enum PrimitivePropertyDomain {
    /// Inclusive bounded length range.
    Length {
        /// Minimum value.
        minimum: f32,
        /// Maximum value.
        maximum: f32,
        /// UI step size.
        step: f32,
    },
    /// Inclusive bounded ratio range.
    Ratio {
        /// Minimum value.
        minimum: f32,
        /// Maximum value.
        maximum: f32,
        /// UI step size.
        step: f32,
    },
    /// Boolean domain.
    Boolean,
    /// Finite symbolic choices.
    Choice {
        /// Legal choices.
        options: Vec<PrimitiveChoiceOption>,
    },
    /// Inclusive bounded angle range in degrees.
    Angle {
        /// Minimum degrees.
        minimum_degrees: f32,
        /// Maximum degrees.
        maximum_degrees: f32,
        /// UI step size in degrees.
        step_degrees: f32,
    },
}

/// One legal symbolic choice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveChoiceOption {
    /// Stable choice ID.
    pub choice_id: String,
    /// Product-facing label.
    pub display_name: String,
}

/// Canonical property value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "PascalCase")]
pub enum PrimitivePropertyValue {
    /// Physical length-like value.
    Length(f32),
    /// Unitless ratio.
    Ratio(f32),
    /// Boolean value.
    Boolean(bool),
    /// Symbolic choice ID.
    Choice(String),
    /// Angle in degrees.
    Angle(f32),
}

/// Property topology behavior.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum PrimitiveTopologyBehavior {
    /// Continuous edit that preserves topology.
    GeometryPreserving,
    /// Discrete edit that may select a different topology.
    DiscreteTopology,
}

/// Preview policy for direct primitive editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePreviewPolicy {
    /// Whether UI should keep the previous valid preview while a rebuild runs.
    pub preserve_previous_valid_preview: bool,
    /// Whether continuous edits may preview while dragging.
    pub continuous_preview_allowed: bool,
    /// Product-safe preview summary.
    pub user_facing_description: String,
}

/// Export policy for direct primitive editing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveExportPolicy {
    /// Whether the current primitive can be exported alone.
    pub export_current_primitive: bool,
    /// Product-safe export summary.
    pub user_facing_description: String,
    /// Product-safe limitations that must remain visible.
    pub limitations: Vec<String>,
}

/// One primitive property validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePropertyValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Primitive property validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitivePropertyValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PrimitivePropertyValidationIssue>,
}

impl PrimitivePropertyValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(&mut self, subject: impl Into<String>, code: &'static str, message: &'static str) {
        self.issues.push(PrimitivePropertyValidationIssue {
            subject: subject.into(),
            code: code.to_owned(),
            message: message.to_owned(),
        });
    }
}

/// Return the v0 Box Primitive property schema.
#[must_use]
pub fn box_primitive_property_schema() -> PrimitivePropertySchema {
    PrimitivePropertySchema {
        schema_version: PRIMITIVE_PROPERTY_SCHEMA_VERSION,
        primitive_kind: PrimitiveKind::BoxPrimitive,
        display_name: "Box Primitive".to_owned(),
        identity_summary: "One editable clay box with bounded dimensions and edge softness."
            .to_owned(),
        properties: vec![
            length_property("width", "Width", 2.0),
            length_property("depth", "Depth", 1.4),
            length_property("height", "Height", 1.0),
            ratio_property("edge_softness", "Edge Softness", 0.08, 0.0, 0.35),
        ],
        constraints: vec![
            constraint(
                "positive_dimensions",
                "Positive dimensions",
                "Width, Depth, and Height must remain positive.",
            ),
            constraint(
                "bounded_softness",
                "Bounded edge softness",
                "Edge Softness must remain within the approved primitive range.",
            ),
        ],
        preview_policy: direct_preview_policy(),
        export_policy: direct_export_policy("Exports the current clay box primitive."),
    }
}

/// Return the v0 Flat Panel Primitive property schema.
#[must_use]
pub fn flat_panel_primitive_property_schema() -> PrimitivePropertySchema {
    PrimitivePropertySchema {
        schema_version: PRIMITIVE_PROPERTY_SCHEMA_VERSION,
        primitive_kind: PrimitiveKind::FlatPanelPrimitive,
        display_name: "Flat Panel Primitive".to_owned(),
        identity_summary: "One upright clay panel with bounded width, height, and thickness."
            .to_owned(),
        properties: vec![
            length_property("width", "Width", 1.8),
            length_property("height", "Height", 2.6),
            length_property("thickness", "Thickness", 0.18),
            ratio_property("edge_softness", "Edge Softness", 0.05, 0.0, 0.3),
        ],
        constraints: vec![
            constraint(
                "positive_dimensions",
                "Positive dimensions",
                "Width, Height, and Thickness must remain positive.",
            ),
            constraint(
                "readable_thickness",
                "Readable thickness",
                "Thickness must stay within the approved flat panel range.",
            ),
        ],
        preview_policy: direct_preview_policy(),
        export_policy: direct_export_policy("Exports the current clay flat panel primitive."),
    }
}

/// Return the v0 Sphere Primitive property schema.
#[must_use]
pub fn sphere_primitive_property_schema() -> PrimitivePropertySchema {
    PrimitivePropertySchema {
        schema_version: PRIMITIVE_PROPERTY_SCHEMA_VERSION,
        primitive_kind: PrimitiveKind::SpherePrimitive,
        display_name: "Sphere Primitive".to_owned(),
        identity_summary: "One closed round clay volume with bounded flattening controls."
            .to_owned(),
        properties: vec![
            length_property("width", "Width", 1.0),
            length_property("height", "Height", 1.0),
            length_property("depth", "Depth", 1.0),
            ratio_property("front_flatten", "Front Flatten", 0.0, 0.0, 0.8),
            ratio_property("back_flatten", "Back Flatten", 0.0, 0.0, 0.8),
        ],
        constraints: vec![
            constraint(
                "positive_dimensions",
                "Positive dimensions",
                "Width, Height, and Depth must remain positive.",
            ),
            constraint(
                "bounded_flattening",
                "Bounded flattening",
                "Front Flatten and Back Flatten must remain within the approved range.",
            ),
        ],
        preview_policy: direct_preview_policy(),
        export_policy: direct_export_policy("Exports the current clay sphere primitive."),
    }
}

/// Return default property values keyed by property ID.
#[must_use]
pub fn primitive_default_property_values(
    schema: &PrimitivePropertySchema,
) -> BTreeMap<String, PrimitivePropertyValue> {
    schema
        .properties
        .iter()
        .map(|property| (property.property_id.clone(), property.default_value.clone()))
        .collect()
}

/// Build semantic property descriptors for one direct primitive schema.
#[must_use]
pub fn primitive_property_descriptors_for_kind(
    primitive_kind: PrimitiveKind,
) -> Vec<PropertyDescriptor> {
    if primitive_kind == PrimitiveKind::CylinderPrimitive {
        return Vec::new();
    }
    let schema = primitive_property_schema_for_kind(primitive_kind);
    schema
        .properties
        .iter()
        .map(|property| primitive_property_descriptor(&schema, property))
        .collect()
}

/// Validate a primitive property schema contract.
#[must_use]
pub fn validate_primitive_property_schema(
    schema: &PrimitivePropertySchema,
) -> PrimitivePropertyValidationReport {
    let mut report = PrimitivePropertyValidationReport::default();
    if schema.schema_version != PRIMITIVE_PROPERTY_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_primitive_property_schema",
            "Primitive property schema version is not supported.",
        );
    }
    validate_product_label(
        &mut report,
        "display_name",
        &schema.display_name,
        "invalid_schema_display_name",
    );
    validate_product_label(
        &mut report,
        "identity_summary",
        &schema.identity_summary,
        "invalid_schema_identity_summary",
    );
    if schema.properties.is_empty() {
        report.push(
            "properties",
            "missing_primitive_properties",
            "Primitive schemas must expose at least one property.",
        );
    }

    let mut property_ids = BTreeSet::new();
    for (index, property) in schema.properties.iter().enumerate() {
        validate_primitive_property(&mut report, index, property);
        if !property_ids.insert(property.property_id.as_str()) {
            report.push(
                format!("properties.{index}.property_id"),
                "duplicate_primitive_property",
                "Primitive property IDs must be unique.",
            );
        }
    }

    for required in required_property_ids(schema.primitive_kind) {
        if !property_ids.contains(required) {
            report.push(
                format!("properties.{required}"),
                "missing_required_primitive_property",
                "Primitive schema is missing a required property.",
            );
        }
    }

    for (index, constraint) in schema.constraints.iter().enumerate() {
        validate_identifier(
            &mut report,
            format!("constraints.{index}.constraint_id"),
            &constraint.constraint_id,
            "invalid_primitive_constraint_id",
        );
        validate_product_label(
            &mut report,
            format!("constraints.{index}.display_name"),
            &constraint.display_name,
            "invalid_primitive_constraint_label",
        );
        validate_product_label(
            &mut report,
            format!("constraints.{index}.user_facing_description"),
            &constraint.user_facing_description,
            "invalid_primitive_constraint_description",
        );
    }

    validate_product_label(
        &mut report,
        "preview_policy.user_facing_description",
        &schema.preview_policy.user_facing_description,
        "invalid_preview_policy_description",
    );
    validate_product_label(
        &mut report,
        "export_policy.user_facing_description",
        &schema.export_policy.user_facing_description,
        "invalid_export_policy_description",
    );
    for (index, limitation) in schema.export_policy.limitations.iter().enumerate() {
        validate_product_label(
            &mut report,
            format!("export_policy.limitations.{index}"),
            limitation,
            "invalid_export_policy_limitation",
        );
    }

    report
}

/// Validate current primitive property values before they can become state.
#[must_use]
pub fn validate_primitive_property_values(
    schema: &PrimitivePropertySchema,
    values: &BTreeMap<String, PrimitivePropertyValue>,
) -> PrimitivePropertyValidationReport {
    let mut report = validate_primitive_property_schema(schema);
    let properties = schema
        .properties
        .iter()
        .map(|property| (property.property_id.as_str(), property))
        .collect::<BTreeMap<_, _>>();

    for property_id in properties.keys() {
        if !values.contains_key(*property_id) {
            report.push(
                format!("values.{property_id}"),
                "missing_current_property_value",
                "Current primitive state is missing a required property value.",
            );
        }
    }

    for (property_id, value) in values {
        let Some(property) = properties.get(property_id.as_str()) else {
            report.push(
                format!("values.{property_id}"),
                "unknown_current_property_value",
                "Current primitive state references an unknown property.",
            );
            continue;
        };
        if !property_value_matches_kind(value, property.value_kind)
            || !domain_contains_value(&property.domain, value)
        {
            report.push(
                format!("values.{property_id}"),
                "invalid_current_property_value",
                "Current primitive state contains a value outside the property domain.",
            );
        }
    }

    report
}

fn validate_primitive_property(
    report: &mut PrimitivePropertyValidationReport,
    index: usize,
    property: &PrimitiveProperty,
) {
    let subject = format!("properties.{index}");
    validate_identifier(
        report,
        format!("{subject}.property_id"),
        &property.property_id,
        "invalid_primitive_property_id",
    );
    validate_product_label(
        report,
        format!("{subject}.display_name"),
        &property.display_name,
        "invalid_primitive_property_label",
    );
    validate_product_label(
        report,
        format!("{subject}.user_facing_description"),
        &property.user_facing_description,
        "invalid_primitive_property_description",
    );
    validate_property_domain(report, &subject, property);

    if !domain_matches_kind(&property.domain, property.value_kind) {
        report.push(
            format!("{subject}.domain"),
            "property_domain_kind_mismatch",
            "Property domain must match its declared value kind.",
        );
    }
    if !property_value_matches_kind(&property.default_value, property.value_kind) {
        report.push(
            format!("{subject}.default_value"),
            "property_default_kind_mismatch",
            "Property default value must match its declared value kind.",
        );
    } else if !domain_contains_value(&property.domain, &property.default_value) {
        report.push(
            format!("{subject}.default_value"),
            "property_default_outside_domain",
            "Property default value must be inside the domain.",
        );
    }
    if property.topology_behavior == PrimitiveTopologyBehavior::DiscreteTopology
        && property.value_kind != PrimitivePropertyValueKind::Choice
    {
        report.push(
            format!("{subject}.topology_behavior"),
            "topology_change_must_be_choice",
            "Topology-changing primitive properties must be discrete choices.",
        );
    }
}

fn validate_property_domain(
    report: &mut PrimitivePropertyValidationReport,
    subject: &str,
    property: &PrimitiveProperty,
) {
    match &property.domain {
        PrimitivePropertyDomain::Length {
            minimum,
            maximum,
            step,
        }
        | PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step,
        } => validate_numeric_domain(
            report,
            format!("{subject}.domain"),
            *minimum,
            *maximum,
            *step,
        ),
        PrimitivePropertyDomain::Angle {
            minimum_degrees,
            maximum_degrees,
            step_degrees,
        } => validate_numeric_domain(
            report,
            format!("{subject}.domain"),
            *minimum_degrees,
            *maximum_degrees,
            *step_degrees,
        ),
        PrimitivePropertyDomain::Boolean => {}
        PrimitivePropertyDomain::Choice { options } => {
            if options.is_empty() {
                report.push(
                    format!("{subject}.domain.options"),
                    "empty_choice_domain",
                    "Choice domains must include at least one option.",
                );
            }
            let mut option_ids = BTreeSet::new();
            for (index, option) in options.iter().enumerate() {
                validate_identifier(
                    report,
                    format!("{subject}.domain.options.{index}.choice_id"),
                    &option.choice_id,
                    "invalid_choice_id",
                );
                validate_product_label(
                    report,
                    format!("{subject}.domain.options.{index}.display_name"),
                    &option.display_name,
                    "invalid_choice_label",
                );
                if !option_ids.insert(option.choice_id.as_str()) {
                    report.push(
                        format!("{subject}.domain.options.{index}.choice_id"),
                        "duplicate_choice_id",
                        "Choice IDs must be unique within one property.",
                    );
                }
            }
        }
    }
}

fn validate_numeric_domain(
    report: &mut PrimitivePropertyValidationReport,
    subject: String,
    minimum: f32,
    maximum: f32,
    step: f32,
) {
    if !minimum.is_finite() || !maximum.is_finite() || !step.is_finite() {
        report.push(
            subject,
            "non_finite_property_domain",
            "Numeric property domains must use finite bounds and step sizes.",
        );
    } else if minimum > maximum || step <= 0.0 {
        report.push(
            subject,
            "invalid_property_domain_range",
            "Numeric property domains must have ordered bounds and a positive step size.",
        );
    }
}

fn validate_identifier(
    report: &mut PrimitivePropertyValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-'))
        || contains_internal_term(value)
    {
        report.push(
            subject,
            code,
            "Stable primitive IDs must be lowercase product-safe identifiers.",
        );
    }
}

fn validate_product_label(
    report: &mut PrimitivePropertyValidationReport,
    subject: impl Into<String>,
    value: &str,
    code: &'static str,
) {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || contains_internal_term(trimmed)
        || contains_blender_like_term(trimmed)
        || trimmed.contains("::")
        || trimmed.contains('/')
        || trimmed.contains('\\')
    {
        report.push(
            subject,
            code,
            "Primitive property labels and descriptions must be product-safe.",
        );
    }
}

fn domain_matches_kind(
    domain: &PrimitivePropertyDomain,
    value_kind: PrimitivePropertyValueKind,
) -> bool {
    matches!(
        (domain, value_kind),
        (
            PrimitivePropertyDomain::Length { .. },
            PrimitivePropertyValueKind::Length
        ) | (
            PrimitivePropertyDomain::Ratio { .. },
            PrimitivePropertyValueKind::Ratio
        ) | (
            PrimitivePropertyDomain::Boolean,
            PrimitivePropertyValueKind::Boolean
        ) | (
            PrimitivePropertyDomain::Choice { .. },
            PrimitivePropertyValueKind::Choice
        ) | (
            PrimitivePropertyDomain::Angle { .. },
            PrimitivePropertyValueKind::Angle
        )
    )
}

fn property_value_matches_kind(
    value: &PrimitivePropertyValue,
    value_kind: PrimitivePropertyValueKind,
) -> bool {
    matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Length(value), PrimitivePropertyValueKind::Length)
            if value.is_finite()
    ) || matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Ratio(value), PrimitivePropertyValueKind::Ratio)
            if value.is_finite()
    ) || matches!(
        (value, value_kind),
        (
            PrimitivePropertyValue::Boolean(_),
            PrimitivePropertyValueKind::Boolean
        )
    ) || matches!(
        (value, value_kind),
        (
            PrimitivePropertyValue::Choice(_),
            PrimitivePropertyValueKind::Choice
        )
    ) || matches!(
        (value, value_kind),
        (PrimitivePropertyValue::Angle(value), PrimitivePropertyValueKind::Angle)
            if value.is_finite()
    )
}

fn domain_contains_value(domain: &PrimitivePropertyDomain, value: &PrimitivePropertyValue) -> bool {
    match (domain, value) {
        (
            PrimitivePropertyDomain::Length {
                minimum, maximum, ..
            },
            PrimitivePropertyValue::Length(value),
        )
        | (
            PrimitivePropertyDomain::Ratio {
                minimum, maximum, ..
            },
            PrimitivePropertyValue::Ratio(value),
        ) => value.is_finite() && value >= minimum && value <= maximum,
        (
            PrimitivePropertyDomain::Angle {
                minimum_degrees,
                maximum_degrees,
                ..
            },
            PrimitivePropertyValue::Angle(value),
        ) => value.is_finite() && value >= minimum_degrees && value <= maximum_degrees,
        (PrimitivePropertyDomain::Boolean, PrimitivePropertyValue::Boolean(_)) => true,
        (PrimitivePropertyDomain::Choice { options }, PrimitivePropertyValue::Choice(choice)) => {
            options.iter().any(|option| option.choice_id == *choice)
        }
        _ => false,
    }
}

fn required_property_ids(primitive_kind: PrimitiveKind) -> &'static [&'static str] {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => &["width", "depth", "height", "edge_softness"],
        PrimitiveKind::FlatPanelPrimitive => &["width", "height", "thickness", "edge_softness"],
        PrimitiveKind::SpherePrimitive => {
            &["width", "height", "depth", "front_flatten", "back_flatten"]
        }
        PrimitiveKind::CylinderPrimitive => &["radius", "height", "sides", "edge_softness"],
    }
}

fn primitive_property_schema_for_kind(primitive_kind: PrimitiveKind) -> PrimitivePropertySchema {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => box_primitive_property_schema(),
        PrimitiveKind::FlatPanelPrimitive => flat_panel_primitive_property_schema(),
        PrimitiveKind::SpherePrimitive => sphere_primitive_property_schema(),
        PrimitiveKind::CylinderPrimitive => unreachable!("Cylinder is not product-active"),
    }
}

fn primitive_property_descriptor(
    schema: &PrimitivePropertySchema,
    property: &PrimitiveProperty,
) -> PropertyDescriptor {
    let control_family = control_family_for_property(&property.property_id, property.value_kind);
    let affects = affects_for_control_family(control_family);
    PropertyDescriptor {
        id: format!(
            "{}.{}",
            descriptor_prefix(schema.primitive_kind),
            property.property_id
        ),
        path: format!(
            "primitive.{}.{}",
            descriptor_prefix(schema.primitive_kind),
            property.property_id
        ),
        label: property.display_name.clone(),
        beginner_description: property.user_facing_description.clone(),
        group: group_for_control_family(control_family).to_owned(),
        domain: descriptor_domain(&property.domain),
        default_value: descriptor_value(&property.default_value),
        topology_changing: property.topology_behavior
            == PrimitiveTopologyBehavior::DiscreteTopology,
        affects,
        review_importance: if property.advanced {
            PropertyReviewImportance::Advanced
        } else {
            PropertyReviewImportance::Primary
        },
        control_family,
        authoring_effect: PropertyAuthoringEffect::SetProperty,
    }
}

fn descriptor_prefix(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "box",
        PrimitiveKind::FlatPanelPrimitive => "flat_panel",
        PrimitiveKind::SpherePrimitive => "sphere",
        PrimitiveKind::CylinderPrimitive => "cylinder",
    }
}

/// Convert a primitive kind into the shared kernel kind when product-active.
#[must_use]
pub fn kernel_kind_for_primitive_kind(primitive_kind: PrimitiveKind) -> Option<KernelKind> {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => Some(KernelKind::BoxPrimitive),
        PrimitiveKind::FlatPanelPrimitive => Some(KernelKind::FlatPanelPrimitive),
        PrimitiveKind::SpherePrimitive => Some(KernelKind::SpherePrimitive),
        PrimitiveKind::CylinderPrimitive => None,
    }
}

fn control_family_for_property(
    property_id: &str,
    value_kind: PrimitivePropertyValueKind,
) -> OrchardControlFamily {
    match property_id {
        "width" | "depth" | "height" | "thickness" | "radius" => OrchardControlFamily::Stretch,
        "edge_softness" | "front_flatten" | "back_flatten" => OrchardControlFamily::Profile,
        _ => match value_kind {
            PrimitivePropertyValueKind::Choice | PrimitivePropertyValueKind::Boolean => {
                OrchardControlFamily::Option
            }
            PrimitivePropertyValueKind::Angle => OrchardControlFamily::Attachment,
            PrimitivePropertyValueKind::Length | PrimitivePropertyValueKind::Ratio => {
                OrchardControlFamily::Stretch
            }
        },
    }
}

fn affects_for_control_family(control_family: OrchardControlFamily) -> Vec<PropertyAffect> {
    match control_family {
        OrchardControlFamily::Stretch => vec![PropertyAffect::Dimensions],
        OrchardControlFamily::Profile => vec![PropertyAffect::Profile],
        OrchardControlFamily::Attachment => vec![PropertyAffect::AttachmentPlacement],
        OrchardControlFamily::Band
        | OrchardControlFamily::Pattern
        | OrchardControlFamily::Option => {
            vec![PropertyAffect::Composition]
        }
    }
}

fn group_for_control_family(control_family: OrchardControlFamily) -> &'static str {
    match control_family {
        OrchardControlFamily::Stretch => "Dimensions",
        OrchardControlFamily::Profile => "Profile",
        OrchardControlFamily::Attachment => "Placement",
        OrchardControlFamily::Band => "Band",
        OrchardControlFamily::Pattern => "Pattern",
        OrchardControlFamily::Option => "Options",
    }
}

fn descriptor_domain(domain: &PrimitivePropertyDomain) -> PropertyDescriptorDomain {
    match domain {
        PrimitivePropertyDomain::Length {
            minimum,
            maximum,
            step,
        } => PropertyDescriptorDomain::Length {
            minimum: *minimum,
            maximum: *maximum,
            step: *step,
        },
        PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step,
        } => PropertyDescriptorDomain::Ratio {
            minimum: *minimum,
            maximum: *maximum,
            step: *step,
        },
        PrimitivePropertyDomain::Boolean => PropertyDescriptorDomain::Boolean,
        PrimitivePropertyDomain::Choice { options } => PropertyDescriptorDomain::Choice {
            options: options
                .iter()
                .map(|option| option.choice_id.clone())
                .collect(),
        },
        PrimitivePropertyDomain::Angle {
            minimum_degrees,
            maximum_degrees,
            step_degrees,
        } => PropertyDescriptorDomain::Angle {
            minimum_degrees: *minimum_degrees,
            maximum_degrees: *maximum_degrees,
            step_degrees: *step_degrees,
        },
    }
}

fn descriptor_value(value: &PrimitivePropertyValue) -> PropertyDescriptorValue {
    match value {
        PrimitivePropertyValue::Length(value) => PropertyDescriptorValue::Length(*value),
        PrimitivePropertyValue::Ratio(value) => PropertyDescriptorValue::Ratio(*value),
        PrimitivePropertyValue::Boolean(value) => PropertyDescriptorValue::Boolean(*value),
        PrimitivePropertyValue::Choice(value) => PropertyDescriptorValue::Choice(value.clone()),
        PrimitivePropertyValue::Angle(value) => PropertyDescriptorValue::Angle(*value),
    }
}

fn length_property(property_id: &str, display_name: &str, default_value: f32) -> PrimitiveProperty {
    PrimitiveProperty {
        property_id: property_id.to_owned(),
        display_name: display_name.to_owned(),
        value_kind: PrimitivePropertyValueKind::Length,
        domain: PrimitivePropertyDomain::Length {
            minimum: 0.05,
            maximum: 6.0,
            step: 0.01,
        },
        default_value: PrimitivePropertyValue::Length(default_value),
        affects_geometry: true,
        topology_behavior: PrimitiveTopologyBehavior::GeometryPreserving,
        user_facing_description: format!("Adjusts the primitive {display_name}."),
        advanced: false,
    }
}

fn ratio_property(
    property_id: &str,
    display_name: &str,
    default_value: f32,
    minimum: f32,
    maximum: f32,
) -> PrimitiveProperty {
    PrimitiveProperty {
        property_id: property_id.to_owned(),
        display_name: display_name.to_owned(),
        value_kind: PrimitivePropertyValueKind::Ratio,
        domain: PrimitivePropertyDomain::Ratio {
            minimum,
            maximum,
            step: 0.01,
        },
        default_value: PrimitivePropertyValue::Ratio(default_value),
        affects_geometry: true,
        topology_behavior: PrimitiveTopologyBehavior::GeometryPreserving,
        user_facing_description: format!("Adjusts the primitive {display_name}."),
        advanced: false,
    }
}

fn constraint(
    constraint_id: &str,
    display_name: &str,
    user_facing_description: &str,
) -> PrimitiveSchemaConstraint {
    PrimitiveSchemaConstraint {
        constraint_id: constraint_id.to_owned(),
        display_name: display_name.to_owned(),
        user_facing_description: user_facing_description.to_owned(),
    }
}

fn direct_preview_policy() -> PrimitivePreviewPolicy {
    PrimitivePreviewPolicy {
        preserve_previous_valid_preview: true,
        continuous_preview_allowed: true,
        user_facing_description: "Keep the last valid primitive preview while updates compile."
            .to_owned(),
    }
}

fn direct_export_policy(user_facing_description: &str) -> PrimitiveExportPolicy {
    PrimitiveExportPolicy {
        export_current_primitive: true,
        user_facing_description: user_facing_description.to_owned(),
        limitations: vec![
            "This is not a textured, rigged, animated, or game-ready package.".to_owned(),
        ],
    }
}

fn contains_internal_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "provider",
        "scalar path",
        "slot",
        "operation id",
        "internal",
        "recipe",
        "raw transform",
    ]
    .iter()
    .any(|term| lower.contains(term))
}

fn contains_blender_like_term(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "vertex",
        "vertices",
        "face",
        "faces",
        "loop",
        "loops",
        "cage",
        "boolean",
        "sculpt",
        "mesh transform",
    ]
    .iter()
    .any(|term| lower.contains(term))
}
