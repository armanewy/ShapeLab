
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
