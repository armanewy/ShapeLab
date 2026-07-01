//! Kit Capability Adapter contracts.
//!
//! Capability cards are product-facing wrappers over primitive schemas,
//! deterministic presets, future surface boundaries, composition controls, and
//! export options. They describe what can change without exposing internal
//! authoring terms.

use serde::{Deserialize, Serialize};

use crate::{
    PrimitiveKind, PrimitivePropertySchema, box_primitive_property_schema,
    flat_panel_primitive_property_schema, primitive_surface_capability,
    sphere_primitive_property_schema,
};

/// Product-facing capability card for Family Studio Lite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitCapabilityCard {
    /// Stable capability ID.
    pub capability_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Product-facing description.
    pub description: String,
    /// Source kind this card wraps.
    pub source_kind: KitCapabilitySourceKind,
    /// Current availability.
    pub availability: KitCapabilityAvailability,
    /// Plain-language reason.
    pub reason: KitCapabilityAvailabilityReason,
    /// Stable source mapping.
    pub maps_to: String,
    /// Whether a visible test is required before stronger review.
    pub visible_test_required: bool,
}

/// Source wrapped by a capability card.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum KitCapabilitySourceKind {
    /// Primitive property schema entry.
    PrimitiveProperty,
    /// Deterministic preset group.
    PresetSet,
    /// Future surface/look boundary.
    SurfaceLook,
    /// Safe composition offset or attachment control.
    CompositionOffset,
    /// Export capability.
    ExportOption,
}

/// Capability availability.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum KitCapabilityAvailability {
    /// Already included in the kit.
    Included,
    /// User can include it.
    Available,
    /// Recommended for this kit.
    Recommended,
    /// Planned for later.
    Later,
    /// Blocked by missing requirements.
    Blocked,
}

/// Plain-language explanation for availability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitCapabilityAvailabilityReason {
    /// User-facing reason.
    pub plain_language_reason: String,
    /// Suggested next action.
    pub suggested_next_action: String,
    /// User-facing blocker when applicable.
    pub blocked_reason: Option<String>,
    /// Whether current requirements are satisfied.
    pub requirements_satisfied: bool,
}

/// One capability-card validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitCapabilityValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Capability-card validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KitCapabilityValidationReport {
    /// Validation issues.
    pub issues: Vec<KitCapabilityValidationIssue>,
}

impl KitCapabilityValidationReport {
    /// Return true when no issues were found.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(KitCapabilityValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return capability cards for one primitive.
#[must_use]
pub fn kit_capability_cards_for_primitive(
    primitive_kind: PrimitiveKind,
    surface_evidence_exists: bool,
) -> Vec<KitCapabilityCard> {
    let schema = match primitive_kind {
        PrimitiveKind::BoxPrimitive => box_primitive_property_schema(),
        PrimitiveKind::FlatPanelPrimitive => flat_panel_primitive_property_schema(),
        PrimitiveKind::SpherePrimitive => sphere_primitive_property_schema(),
        PrimitiveKind::CylinderPrimitive => {
            return vec![blocked_card(
                "unsupported_primitive",
                "Unsupported primitive",
                "This starting point is not available for reusable kits yet.",
                "unsupported_primitive",
            )];
        }
    };

    let mut cards = schema
        .properties
        .iter()
        .map(|property| {
            property_card(
                &format!(
                    "{}.{}",
                    primitive_prefix(primitive_kind),
                    property.property_id
                ),
                &property.display_name,
                &format!("{} can be adjusted.", property.display_name),
                &property.property_id,
            )
        })
        .collect::<Vec<_>>();
    cards.push(preset_card(primitive_kind));
    cards.push(surface_card(primitive_kind, surface_evidence_exists));
    cards
}

/// Return capability cards for the supported Panel with Knob composition.
#[must_use]
pub fn kit_capability_cards_for_panel_with_knob(
    surface_evidence_exists: bool,
) -> Vec<KitCapabilityCard> {
    let mut cards = vec![
        composition_property_card("panel.width", "Panel Width", "Panel width can be adjusted."),
        composition_property_card(
            "panel.height",
            "Panel Height",
            "Panel height can be adjusted.",
        ),
        composition_property_card(
            "panel.thickness",
            "Panel Thickness",
            "Panel thickness can be adjusted.",
        ),
        composition_property_card(
            "panel.edge_softness",
            "Panel Edge Softness",
            "Panel edge softness can be adjusted.",
        ),
        composition_property_card("knob.width", "Knob Width", "Knob width can be adjusted."),
        composition_property_card("knob.height", "Knob Height", "Knob height can be adjusted."),
        composition_property_card("knob.depth", "Knob Depth", "Knob depth can be adjusted."),
        composition_property_card(
            "knob.front_flatten",
            "Knob Front Flatten",
            "Knob front flattening can be adjusted.",
        ),
        composition_property_card(
            "knob.back_flatten",
            "Knob Back Flatten",
            "Knob back flattening can be adjusted.",
        ),
        KitCapabilityCard {
            capability_id: "panel_with_knob.knob_safe_position".to_owned(),
            display_name: "Knob Position".to_owned(),
            description: "Knob position can move within the safe area.".to_owned(),
            source_kind: KitCapabilitySourceKind::CompositionOffset,
            availability: KitCapabilityAvailability::Available,
            reason: available_reason(),
            maps_to: "panel_with_knob.knob_safe_position".to_owned(),
            visible_test_required: true,
        },
        KitCapabilityCard {
            capability_id: "panel_with_knob.presets".to_owned(),
            display_name: "Saved Shapes".to_owned(),
            description: "Named panel and knob shapes can be included.".to_owned(),
            source_kind: KitCapabilitySourceKind::PresetSet,
            availability: KitCapabilityAvailability::Recommended,
            reason: recommended_reason(),
            maps_to: "built_in_primitive_presets".to_owned(),
            visible_test_required: true,
        },
    ];
    cards.push(surface_card(
        PrimitiveKind::FlatPanelPrimitive,
        surface_evidence_exists,
    ));
    cards
}

/// Validate one capability card.
#[must_use]
pub fn validate_kit_capability_card(card: &KitCapabilityCard) -> KitCapabilityValidationReport {
    let mut report = KitCapabilityValidationReport::default();

    validate_identifier(&card.capability_id, "capability_id", &mut report);
    validate_user_copy(&card.display_name, "display_name", &mut report);
    validate_user_copy(&card.description, "description", &mut report);
    validate_user_copy(
        &card.reason.plain_language_reason,
        "reason.plain_language_reason",
        &mut report,
    );
    validate_user_copy(
        &card.reason.suggested_next_action,
        "reason.suggested_next_action",
        &mut report,
    );
    if let Some(blocked_reason) = &card.reason.blocked_reason {
        validate_user_copy(blocked_reason, "reason.blocked_reason", &mut report);
    }
    validate_mapping(card, &mut report);
    if matches!(
        card.availability,
        KitCapabilityAvailability::Later | KitCapabilityAvailability::Blocked
    ) && card.reason.requirements_satisfied
    {
        report.push(
            "reason.requirements_satisfied",
            "kit_capability_later_or_blocked_requires_unsatisfied_reason",
            "Later or Blocked cards must report unsatisfied requirements.",
        );
    }
    if card.reason.plain_language_reason.trim().is_empty()
        || card.reason.suggested_next_action.trim().is_empty()
    {
        report.push(
            "reason",
            "kit_capability_reason_required",
            "Capability cards need plain-language reason and next action.",
        );
    }

    report
}

/// Validate a set of capability cards.
#[must_use]
pub fn validate_kit_capability_cards(cards: &[KitCapabilityCard]) -> KitCapabilityValidationReport {
    let mut report = KitCapabilityValidationReport::default();
    let mut ids = std::collections::BTreeSet::new();
    for (index, card) in cards.iter().enumerate() {
        if !ids.insert(card.capability_id.clone()) {
            report.push(
                format!("cards.{index}.capability_id"),
                "kit_capability_duplicate_id",
                "Capability card IDs must be unique.",
            );
        }
        for issue in validate_kit_capability_card(card).issues {
            report.push(
                format!("cards.{index}.{}", issue.subject),
                issue.code,
                issue.message,
            );
        }
    }
    report
}

fn property_card(
    capability_id: &str,
    display_name: &str,
    description: &str,
    property_id: &str,
) -> KitCapabilityCard {
    KitCapabilityCard {
        capability_id: capability_id.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        source_kind: KitCapabilitySourceKind::PrimitiveProperty,
        availability: KitCapabilityAvailability::Available,
        reason: available_reason(),
        maps_to: property_id.to_owned(),
        visible_test_required: true,
    }
}

fn composition_property_card(
    maps_to: &str,
    display_name: &str,
    description: &str,
) -> KitCapabilityCard {
    KitCapabilityCard {
        capability_id: format!("panel_with_knob.{maps_to}"),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        source_kind: KitCapabilitySourceKind::PrimitiveProperty,
        availability: KitCapabilityAvailability::Available,
        reason: available_reason(),
        maps_to: maps_to.to_owned(),
        visible_test_required: true,
    }
}

fn preset_card(primitive_kind: PrimitiveKind) -> KitCapabilityCard {
    KitCapabilityCard {
        capability_id: format!("{}.presets", primitive_prefix(primitive_kind)),
        display_name: "Saved Shapes".to_owned(),
        description: "Named preset shapes can be included.".to_owned(),
        source_kind: KitCapabilitySourceKind::PresetSet,
        availability: KitCapabilityAvailability::Recommended,
        reason: recommended_reason(),
        maps_to: "built_in_primitive_presets".to_owned(),
        visible_test_required: true,
    }
}

fn surface_card(primitive_kind: PrimitiveKind, surface_evidence_exists: bool) -> KitCapabilityCard {
    let capability = primitive_surface_capability(primitive_kind);
    let available =
        capability.supported && capability.blocked_reasons.is_empty() && surface_evidence_exists;
    KitCapabilityCard {
        capability_id: format!("{}.material_look", primitive_prefix(primitive_kind)),
        display_name: "Material Look".to_owned(),
        description: "Visual finish controls are planned for later.".to_owned(),
        source_kind: KitCapabilitySourceKind::SurfaceLook,
        availability: if available {
            KitCapabilityAvailability::Available
        } else {
            KitCapabilityAvailability::Later
        },
        reason: if available {
            available_reason()
        } else {
            KitCapabilityAvailabilityReason {
                plain_language_reason: "Visual finish evidence is not ready yet.".to_owned(),
                suggested_next_action: "Keep this for a later review pass.".to_owned(),
                blocked_reason: Some("Geometry evidence must stay stable first.".to_owned()),
                requirements_satisfied: false,
            }
        },
        maps_to: "primitive_surface_v0".to_owned(),
        visible_test_required: true,
    }
}

fn blocked_card(
    capability_id: &str,
    display_name: &str,
    description: &str,
    maps_to: &str,
) -> KitCapabilityCard {
    KitCapabilityCard {
        capability_id: capability_id.to_owned(),
        display_name: display_name.to_owned(),
        description: description.to_owned(),
        source_kind: KitCapabilitySourceKind::PrimitiveProperty,
        availability: KitCapabilityAvailability::Blocked,
        reason: KitCapabilityAvailabilityReason {
            plain_language_reason: "This starting point is not supported yet.".to_owned(),
            suggested_next_action: "Choose a supported primitive.".to_owned(),
            blocked_reason: Some("No bounded property schema is available.".to_owned()),
            requirements_satisfied: false,
        },
        maps_to: maps_to.to_owned(),
        visible_test_required: true,
    }
}

fn available_reason() -> KitCapabilityAvailabilityReason {
    KitCapabilityAvailabilityReason {
        plain_language_reason: "This is available from the current shape.".to_owned(),
        suggested_next_action: "Choose whether this can change.".to_owned(),
        blocked_reason: None,
        requirements_satisfied: true,
    }
}

fn recommended_reason() -> KitCapabilityAvailabilityReason {
    KitCapabilityAvailabilityReason {
        plain_language_reason: "Saved shapes make the kit easier to review.".to_owned(),
        suggested_next_action: "Include named shapes if they match the kit.".to_owned(),
        blocked_reason: None,
        requirements_satisfied: true,
    }
}

fn validate_mapping(card: &KitCapabilityCard, report: &mut KitCapabilityValidationReport) {
    let valid = match card.source_kind {
        KitCapabilitySourceKind::PrimitiveProperty => known_property_mapping(&card.maps_to),
        KitCapabilitySourceKind::PresetSet => card.maps_to == "built_in_primitive_presets",
        KitCapabilitySourceKind::SurfaceLook => card.maps_to == "primitive_surface_v0",
        KitCapabilitySourceKind::CompositionOffset => {
            card.maps_to == "panel_with_knob.knob_safe_position"
        }
        KitCapabilitySourceKind::ExportOption => card.maps_to == "geometry_only_export_v0",
    };
    if !valid {
        report.push(
            "maps_to",
            "kit_capability_unknown_mapping",
            "Capability card maps_to must reference a known property, preset, surface boundary, composition control, or export option.",
        );
    }
}

fn known_property_mapping(maps_to: &str) -> bool {
    matches!(
        maps_to,
        "width"
            | "depth"
            | "height"
            | "thickness"
            | "edge_softness"
            | "front_flatten"
            | "back_flatten"
            | "panel.width"
            | "panel.height"
            | "panel.thickness"
            | "panel.edge_softness"
            | "knob.width"
            | "knob.height"
            | "knob.depth"
            | "knob.front_flatten"
            | "knob.back_flatten"
    )
}

fn primitive_prefix(primitive_kind: PrimitiveKind) -> &'static str {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => "box",
        PrimitiveKind::FlatPanelPrimitive => "flat_panel",
        PrimitiveKind::SpherePrimitive => "sphere",
        PrimitiveKind::CylinderPrimitive => "unsupported",
    }
}

fn validate_identifier(
    value: &str,
    subject: impl Into<String>,
    report: &mut KitCapabilityValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.push(
            subject,
            "kit_capability_identifier_required",
            "Capability card identifiers are required.",
        );
    }
}

fn validate_user_copy(
    value: &str,
    subject: impl Into<String>,
    report: &mut KitCapabilityValidationReport,
) {
    let subject = subject.into();
    if value.trim().is_empty() {
        report.push(
            subject.clone(),
            "kit_capability_copy_required",
            "Capability card copy is required.",
        );
    }
    let lower = value.to_ascii_lowercase();
    for forbidden in [
        "kernel",
        "module",
        "provider",
        "slot",
        "topology",
        "fingerprint",
        "conformance",
        "artifact",
        "raw transform",
        "mesh payload",
        "generated variation",
        "candidate",
        "runtime llm",
        "public catalog",
        "publish",
        "uv",
        "rigging",
        "animation",
        "game-ready",
    ] {
        if lower.contains(forbidden) {
            report.push(
                subject.clone(),
                "kit_capability_user_copy_forbidden_term",
                "Capability card user-facing copy must stay product-safe.",
            );
        }
    }
}

#[allow(dead_code)]
fn _schema_property_ids(schema: &PrimitivePropertySchema) -> Vec<String> {
    schema
        .properties
        .iter()
        .map(|property| property.property_id.clone())
        .collect()
}
