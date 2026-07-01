//! Primitive Surface V0 boundary contracts.
//!
//! These contracts describe future primitive-aware surface policy candidates.
//! They do not enable a user-facing surface UI, emit texture paths, or approve
//! material looks.

use serde::{Deserialize, Serialize};

use crate::PrimitiveKind;

/// Future primitive-aware surface capability boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveSurfaceCapability {
    /// Primitive kind this policy applies to.
    pub primitive_kind: PrimitiveKind,
    /// Whether user-facing primitive surface controls are enabled.
    pub supported: bool,
    /// Future UV policy candidate.
    pub uv_policy: UvPolicy,
    /// Future material slot policy candidate.
    pub material_slot_policy: MaterialSlotPolicy,
    /// Surface properties allowed by this boundary. V0 keeps this empty.
    pub allowed_surface_properties: Vec<String>,
    /// Plain-language blockers.
    pub blocked_reasons: Vec<String>,
    /// Surface work must remain review-gated.
    pub review_required: bool,
}

/// Future UV policy candidates.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UvPolicy {
    /// No UV policy is available.
    None,
    /// Future box projection candidate.
    BoxProjection,
    /// Future planar projection candidate.
    PlanarProjection,
    /// Future spherical projection candidate.
    SphericalProjection,
    /// Future cylindrical projection candidate.
    CylindricalProjection,
    /// Future composition policy that delegates to each primitive node.
    PerNodePrimitivePolicy,
}

/// Future material slot policy candidates.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum MaterialSlotPolicy {
    /// V0 primitive output remains neutral clay only.
    NeutralClayOnly,
    /// Future one-slot policy.
    SingleMaterialSlot,
    /// Future per-primitive policy for compositions.
    PerPrimitiveSlot,
    /// Reserved for a later face-group policy.
    PerFaceGroupSlot,
}

/// One primitive surface policy validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveSurfaceValidationIssue {
    /// Stable subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Validation report for primitive surface policies.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimitiveSurfaceValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<PrimitiveSurfaceValidationIssue>,
}

impl PrimitiveSurfaceValidationReport {
    /// Return true when no issues were discovered.
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
        self.issues.push(PrimitiveSurfaceValidationIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Return the disabled V0 primitive surface policies.
#[must_use]
pub fn primitive_surface_capabilities_v0() -> Vec<PrimitiveSurfaceCapability> {
    vec![
        primitive_surface_capability(PrimitiveKind::BoxPrimitive),
        primitive_surface_capability(PrimitiveKind::FlatPanelPrimitive),
        primitive_surface_capability(PrimitiveKind::SpherePrimitive),
        panel_with_knob_surface_capability(),
    ]
}

/// Return a disabled V0 primitive surface policy for one primitive kind.
#[must_use]
pub fn primitive_surface_capability(primitive_kind: PrimitiveKind) -> PrimitiveSurfaceCapability {
    let uv_policy = match primitive_kind {
        PrimitiveKind::BoxPrimitive => UvPolicy::BoxProjection,
        PrimitiveKind::FlatPanelPrimitive => UvPolicy::PlanarProjection,
        PrimitiveKind::SpherePrimitive => UvPolicy::SphericalProjection,
        PrimitiveKind::CylinderPrimitive => UvPolicy::CylindricalProjection,
    };
    PrimitiveSurfaceCapability {
        primitive_kind,
        supported: false,
        uv_policy,
        material_slot_policy: MaterialSlotPolicy::NeutralClayOnly,
        allowed_surface_properties: Vec::new(),
        blocked_reasons: primitive_surface_blockers(),
        review_required: true,
    }
}

/// Return the disabled V0 policy for the supported Panel with Knob composition.
#[must_use]
pub fn panel_with_knob_surface_capability() -> PrimitiveSurfaceCapability {
    PrimitiveSurfaceCapability {
        primitive_kind: PrimitiveKind::FlatPanelPrimitive,
        supported: false,
        uv_policy: UvPolicy::PerNodePrimitivePolicy,
        material_slot_policy: MaterialSlotPolicy::NeutralClayOnly,
        allowed_surface_properties: Vec::new(),
        blocked_reasons: primitive_surface_blockers(),
        review_required: true,
    }
}

/// Return true only when primitive surface UI may be exposed.
#[must_use]
pub fn primitive_surface_ui_enabled(capability: &PrimitiveSurfaceCapability) -> bool {
    capability.supported
        && capability.blocked_reasons.is_empty()
        && capability.review_required
        && !capability.allowed_surface_properties.is_empty()
}

/// Validate a Primitive Surface V0 boundary policy.
#[must_use]
pub fn validate_primitive_surface_capability(
    capability: &PrimitiveSurfaceCapability,
) -> PrimitiveSurfaceValidationReport {
    let mut report = PrimitiveSurfaceValidationReport::default();

    if capability.supported {
        report.push(
            "supported",
            "primitive_surface_ui_disabled_v0",
            "Primitive Surface V0 defines policy boundaries only and cannot enable UI.",
        );
    }
    if capability.blocked_reasons.is_empty() {
        report.push(
            "blocked_reasons",
            "primitive_surface_requires_blockers_v0",
            "Primitive Surface V0 policies must report why surface controls remain unavailable.",
        );
    }
    if !capability.review_required {
        report.push(
            "review_required",
            "primitive_surface_review_required",
            "Primitive Surface V0 policies must remain review-gated.",
        );
    }
    if !capability.allowed_surface_properties.is_empty() {
        report.push(
            "allowed_surface_properties",
            "primitive_surface_properties_disabled_v0",
            "Primitive Surface V0 must not expose editable surface properties.",
        );
    }
    if capability.material_slot_policy != MaterialSlotPolicy::NeutralClayOnly {
        report.push(
            "material_slot_policy",
            "primitive_surface_material_slots_disabled_v0",
            "Primitive Surface V0 must remain neutral clay only.",
        );
    }
    validate_uv_policy(capability, &mut report);
    validate_surface_text(&capability.blocked_reasons, "blocked_reasons", &mut report);
    validate_surface_text(
        &capability.allowed_surface_properties,
        "allowed_surface_properties",
        &mut report,
    );

    report
}

fn primitive_surface_blockers() -> Vec<String> {
    vec![
        "Geometry export path must stay stable.".to_owned(),
        "Future slot policy needs validation evidence.".to_owned(),
        "UV policy needs visual evidence.".to_owned(),
        "Texture evidence does not exist yet.".to_owned(),
        "Export reports must remain truthful.".to_owned(),
    ]
}

fn validate_uv_policy(
    capability: &PrimitiveSurfaceCapability,
    report: &mut PrimitiveSurfaceValidationReport,
) {
    let valid = matches!(
        (capability.primitive_kind, capability.uv_policy),
        (PrimitiveKind::BoxPrimitive, UvPolicy::BoxProjection)
            | (
                PrimitiveKind::FlatPanelPrimitive,
                UvPolicy::PlanarProjection
            )
            | (
                PrimitiveKind::FlatPanelPrimitive,
                UvPolicy::PerNodePrimitivePolicy
            )
            | (
                PrimitiveKind::SpherePrimitive,
                UvPolicy::SphericalProjection
            )
            | (
                PrimitiveKind::CylinderPrimitive,
                UvPolicy::CylindricalProjection
            )
            | (_, UvPolicy::None)
    );
    if !valid {
        report.push(
            "uv_policy",
            "primitive_surface_uv_policy_incompatible",
            "Primitive Surface V0 UV policy is incompatible with the primitive kind.",
        );
    }
}

fn validate_surface_text(
    values: &[String],
    subject: &str,
    report: &mut PrimitiveSurfaceValidationReport,
) {
    for (index, value) in values.iter().enumerate() {
        let lower = value.to_ascii_lowercase();
        for forbidden in [
            "material editor",
            "game-ready",
            "texture path",
            ".png",
            ".jpg",
            ".jpeg",
            ".webp",
            ".ktx",
        ] {
            if lower.contains(forbidden) {
                report.push(
                    format!("{subject}.{index}"),
                    "primitive_surface_forbidden_claim",
                    "Primitive Surface V0 text must not claim unsupported surface output.",
                );
            }
        }
    }
}
