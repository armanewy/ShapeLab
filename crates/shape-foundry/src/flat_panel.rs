//! Flat panel kernel contracts for the second Object Orchard kernel proof.
//!
//! This module is intentionally contract-only. It defines the identity,
//! placement zones, feature modules, and validation rules for a simple upright
//! panel-like family. It does not add UI, geometry generation, UVs, materials,
//! rigging, animation, or runtime LLM integration.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Current schema version for flat-panel contracts.
pub const FLAT_PANEL_CONTRACT_SCHEMA_VERSION: u32 = 1;

/// Stable internal identity for an upright flat panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelKernel {
    /// Schema version.
    pub schema_version: u32,
    /// Stable internal ID.
    pub kernel_id: String,
    /// Internal display name.
    pub display_name: String,
    /// Product-safe statements that define what must remain true.
    pub identity_invariants: Vec<String>,
    /// Canonical axes and orientation.
    pub axes: FlatPanelAxes,
    /// Orientation and capability policy.
    pub orientation: FlatPanelOrientation,
    /// Deterministic zones that future features may attach to.
    pub placement_zones: Vec<FlatPanelPlacementZone>,
    /// Capabilities this primitive explicitly does not claim.
    pub blocked_capabilities: Vec<String>,
}

/// Canonical axes for panel-like objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelAxes {
    /// Horizontal width axis.
    pub width_axis: String,
    /// Thin depth/thickness axis.
    pub thickness_axis: String,
    /// Vertical height axis.
    pub height_axis: String,
    /// Front direction label.
    pub front_direction: String,
    /// Back direction label.
    pub back_direction: String,
}

/// Orientation policy for the primitive panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelOrientation {
    /// The panel is intended to stand upright.
    pub upright: bool,
    /// The panel has a meaningful front/back distinction.
    pub has_front_back: bool,
    /// The primitive only reserves a hinge side; it does not claim hinges yet.
    pub claims_hinge_geometry: bool,
    /// The primitive does not claim open/close motion.
    pub claims_open_close_motion: bool,
}

/// Placement-zone kinds for a panel-like family.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlatPanelZoneKind {
    /// Broad face region.
    Face,
    /// Thin edge band.
    EdgeBand,
    /// Hinge-candidate edge, not hinge behavior by itself.
    HingeCandidateEdge,
    /// Handle-candidate area, not a handle by itself.
    HandleCandidateZone,
    /// Inset-panel candidate area.
    InsetCandidateZone,
    /// Bottom support/contact edge.
    SupportEdge,
}

/// One deterministic placement zone emitted by the flat-panel kernel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelPlacementZone {
    /// Stable zone ID.
    pub zone_id: String,
    /// Product-safe label.
    pub display_label: String,
    /// Zone kind.
    pub zone_kind: FlatPanelZoneKind,
    /// Normalized `[min_x, min_y, max_x, max_y]` bounds.
    pub normalized_bounds: [f32; 4],
    /// Product-safe description.
    pub product_safe_description: String,
    /// Compatible module tags.
    pub compatible_module_tags: Vec<String>,
}

/// Internal feature module contract for flat-panel features.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelFeatureModule {
    /// Stable internal module ID.
    pub module_id: String,
    /// Internal display name.
    pub display_name: String,
    /// Product-safe summary.
    pub product_safe_summary: String,
    /// Required placement zone kinds.
    pub required_zone_kinds: Vec<FlatPanelZoneKind>,
    /// Roles this module provides.
    pub provides_roles: Vec<String>,
    /// Controls this module owns.
    pub provides_controls: Vec<String>,
    /// Candidate hooks this module contributes.
    pub candidate_hooks: Vec<String>,
    /// Quality gates that must pass before product exposure.
    pub quality_gates: Vec<String>,
}

/// Validation report for flat-panel contracts.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelValidationReport {
    /// Validation issues.
    pub issues: Vec<FlatPanelValidationIssue>,
}

impl FlatPanelValidationReport {
    /// Returns true when no issues were recorded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(&mut self, subject: impl Into<String>, code: &'static str, message: &'static str) {
        self.issues.push(FlatPanelValidationIssue {
            subject: subject.into(),
            code: code.to_owned(),
            message: message.to_owned(),
        });
    }
}

/// One validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlatPanelValidationIssue {
    /// Subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Returns the v0 Flat Panel Primitive kernel contract.
#[must_use]
pub fn flat_panel_primitive_kernel() -> FlatPanelKernel {
    FlatPanelKernel {
        schema_version: FLAT_PANEL_CONTRACT_SCHEMA_VERSION,
        kernel_id: "flat-panel-primitive".to_owned(),
        display_name: "Flat Panel Primitive".to_owned(),
        identity_invariants: vec![
            "remains one upright flat panel".to_owned(),
            "has readable width, height, and thickness".to_owned(),
            "has a meaningful front and back".to_owned(),
            "does not claim to open or close".to_owned(),
        ],
        axes: FlatPanelAxes {
            width_axis: "x".to_owned(),
            thickness_axis: "y".to_owned(),
            height_axis: "z".to_owned(),
            front_direction: "-y".to_owned(),
            back_direction: "+y".to_owned(),
        },
        orientation: FlatPanelOrientation {
            upright: true,
            has_front_back: true,
            claims_hinge_geometry: false,
            claims_open_close_motion: false,
        },
        placement_zones: flat_panel_primitive_zones(),
        blocked_capabilities: vec![
            "surface-material-looks".to_owned(),
            "open-close-motion".to_owned(),
            "rigging".to_owned(),
            "animation".to_owned(),
        ],
    }
}

/// Returns deterministic zones for the Flat Panel Primitive.
#[must_use]
pub fn flat_panel_primitive_zones() -> Vec<FlatPanelPlacementZone> {
    vec![
        zone(
            "front_face",
            "Front face",
            FlatPanelZoneKind::Face,
            [0.0, 0.0, 1.0, 1.0],
            "The broad front face can host future inset or detail features.",
            &["face", "inset"],
        ),
        zone(
            "back_face",
            "Back face",
            FlatPanelZoneKind::Face,
            [0.0, 0.0, 1.0, 1.0],
            "The broad back face mirrors the front for simple panel variations.",
            &["face", "inset"],
        ),
        zone(
            "left_edge",
            "Left edge",
            FlatPanelZoneKind::HingeCandidateEdge,
            [0.0, 0.0, 0.08, 1.0],
            "The left edge can later support hinge-like detail.",
            &["edge", "hinge-candidate"],
        ),
        zone(
            "right_handle_area",
            "Handle area",
            FlatPanelZoneKind::HandleCandidateZone,
            [0.72, 0.38, 0.92, 0.62],
            "A future handle or knob can attach near this side area.",
            &["handle-candidate"],
        ),
        zone(
            "inset_panel_area",
            "Inset panel area",
            FlatPanelZoneKind::InsetCandidateZone,
            [0.18, 0.18, 0.82, 0.82],
            "A future inset panel can fit inside this central region.",
            &["inset", "panel-field"],
        ),
        zone(
            "bottom_support_edge",
            "Bottom edge",
            FlatPanelZoneKind::SupportEdge,
            [0.0, 0.0, 1.0, 0.08],
            "The bottom edge anchors the panel to the support plane.",
            &["support"],
        ),
    ]
}

/// Primitive body module for the flat panel.
#[must_use]
pub fn panel_body_module() -> FlatPanelFeatureModule {
    FlatPanelFeatureModule {
        module_id: "panel-body".to_owned(),
        display_name: "Panel Body".to_owned(),
        product_safe_summary: "Keeps the object as one upright panel.".to_owned(),
        required_zone_kinds: vec![FlatPanelZoneKind::Face, FlatPanelZoneKind::SupportEdge],
        provides_roles: vec!["panel_body".to_owned()],
        provides_controls: vec!["proportions".to_owned(), "edge_softness".to_owned()],
        candidate_hooks: vec!["flat-panel-ideas".to_owned()],
        quality_gates: vec![
            "panel-reads-upright".to_owned(),
            "front-back-readable".to_owned(),
            "edge-softness-visible".to_owned(),
        ],
    }
}

/// Hinge-edge feature module. This is visible clay edge detail, not door motion.
#[must_use]
pub fn hinge_edge_module() -> FlatPanelFeatureModule {
    FlatPanelFeatureModule {
        module_id: "hinge-edge".to_owned(),
        display_name: "Hinge Edge".to_owned(),
        product_safe_summary: "Adds a visible hinge-side edge without open or close behavior."
            .to_owned(),
        required_zone_kinds: vec![FlatPanelZoneKind::HingeCandidateEdge],
        provides_roles: vec!["hinge_edge".to_owned()],
        provides_controls: vec!["hinge_edge_style".to_owned()],
        candidate_hooks: vec!["hinged-panel-ideas".to_owned()],
        quality_gates: vec![
            "hinge-edge-visible".to_owned(),
            "hinge-edge-not-motion".to_owned(),
            "hinge-edge-attached".to_owned(),
            "hinge-edge-endpoint-visible".to_owned(),
        ],
    }
}

/// Future handle-area module. This is a contract only, not product geometry yet.
#[must_use]
pub fn panel_handle_module() -> FlatPanelFeatureModule {
    FlatPanelFeatureModule {
        module_id: "panel-handle".to_owned(),
        display_name: "Panel Handle".to_owned(),
        product_safe_summary: "Adds a simple visible handle or knob area later.".to_owned(),
        required_zone_kinds: vec![FlatPanelZoneKind::HandleCandidateZone],
        provides_roles: vec!["panel_handle".to_owned()],
        provides_controls: vec!["handle_style".to_owned()],
        candidate_hooks: vec!["handle-panel-ideas".to_owned()],
        quality_gates: vec!["handle-visible".to_owned(), "handle-attached".to_owned()],
    }
}

/// Validate one flat-panel kernel contract.
#[must_use]
pub fn validate_flat_panel_kernel(kernel: &FlatPanelKernel) -> FlatPanelValidationReport {
    let mut report = FlatPanelValidationReport::default();

    if kernel.schema_version != FLAT_PANEL_CONTRACT_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_schema_version",
            "Flat-panel schema version is not supported.",
        );
    }
    validate_identifier(&mut report, "kernel_id", &kernel.kernel_id);
    validate_label(&mut report, "display_name", &kernel.display_name);
    if kernel.identity_invariants.is_empty() {
        report.push(
            "identity_invariants",
            "missing_identity_invariants",
            "Flat-panel kernels must declare identity invariants.",
        );
    }
    if !kernel.orientation.upright || !kernel.orientation.has_front_back {
        report.push(
            "orientation",
            "invalid_orientation",
            "Flat Panel Primitive must be upright with a readable front and back.",
        );
    }
    if kernel.orientation.claims_open_close_motion {
        report.push(
            "orientation.claims_open_close_motion",
            "motion_overclaim",
            "Flat Panel Primitive must not claim open/close motion.",
        );
    }
    validate_zones(&mut report, &kernel.placement_zones);
    report
}

/// Validate a flat-panel feature module against a kernel.
#[must_use]
pub fn validate_flat_panel_feature_module(
    kernel: &FlatPanelKernel,
    module: &FlatPanelFeatureModule,
) -> FlatPanelValidationReport {
    let mut report = validate_flat_panel_kernel(kernel);
    validate_identifier(&mut report, "module_id", &module.module_id);
    validate_label(&mut report, "display_name", &module.display_name);
    validate_label(
        &mut report,
        "product_safe_summary",
        &module.product_safe_summary,
    );
    if contains_internal_terms(&module.product_safe_summary) {
        report.push(
            "product_safe_summary",
            "internal_term_in_user_summary",
            "Product-safe summary must not expose internal authoring terms.",
        );
    }
    if module.required_zone_kinds.is_empty() {
        report.push(
            "required_zone_kinds",
            "missing_required_zones",
            "Feature modules must declare required zones.",
        );
    }
    let available_zones: BTreeSet<_> = kernel
        .placement_zones
        .iter()
        .map(|zone| zone.zone_kind.clone())
        .collect();
    for (index, required) in module.required_zone_kinds.iter().enumerate() {
        if !available_zones.contains(required) {
            report.push(
                format!("required_zone_kinds.{index}"),
                "missing_required_zone",
                "Required zone is not available on this kernel.",
            );
        }
    }
    if module.provides_controls.is_empty() {
        report.push(
            "provides_controls",
            "missing_controls",
            "Feature modules must own at least one visible or testable control.",
        );
    }
    if module.candidate_hooks.is_empty() {
        report.push(
            "candidate_hooks",
            "missing_candidate_hooks",
            "Feature modules must declare candidate hooks.",
        );
    }
    if module.quality_gates.is_empty() {
        report.push(
            "quality_gates",
            "missing_quality_gates",
            "Feature modules must declare quality gates.",
        );
    }
    report
}

fn zone(
    zone_id: &str,
    display_label: &str,
    zone_kind: FlatPanelZoneKind,
    normalized_bounds: [f32; 4],
    product_safe_description: &str,
    compatible_module_tags: &[&str],
) -> FlatPanelPlacementZone {
    FlatPanelPlacementZone {
        zone_id: zone_id.to_owned(),
        display_label: display_label.to_owned(),
        zone_kind,
        normalized_bounds,
        product_safe_description: product_safe_description.to_owned(),
        compatible_module_tags: compatible_module_tags
            .iter()
            .map(|tag| (*tag).to_owned())
            .collect(),
    }
}

fn validate_zones(report: &mut FlatPanelValidationReport, zones: &[FlatPanelPlacementZone]) {
    if zones.is_empty() {
        report.push(
            "placement_zones",
            "missing_placement_zones",
            "Flat-panel kernels must declare placement zones.",
        );
        return;
    }
    let mut seen = BTreeSet::new();
    for (index, zone) in zones.iter().enumerate() {
        let subject = format!("placement_zones.{index}");
        validate_identifier(report, format!("{subject}.zone_id"), &zone.zone_id);
        validate_label(
            report,
            format!("{subject}.display_label"),
            &zone.display_label,
        );
        validate_label(
            report,
            format!("{subject}.product_safe_description"),
            &zone.product_safe_description,
        );
        if !seen.insert(zone.zone_id.as_str()) {
            report.push(
                format!("{subject}.zone_id"),
                "duplicate_zone_id",
                "Placement zone IDs must be unique.",
            );
        }
        let [min_x, min_y, max_x, max_y] = zone.normalized_bounds;
        let finite = [min_x, min_y, max_x, max_y]
            .iter()
            .all(|value| value.is_finite());
        if !finite
            || min_x < 0.0
            || min_y < 0.0
            || max_x > 1.0
            || max_y > 1.0
            || min_x >= max_x
            || min_y >= max_y
        {
            report.push(
                format!("{subject}.normalized_bounds"),
                "invalid_zone_bounds",
                "Placement zone bounds must be finite normalized rectangles.",
            );
        }
        if contains_internal_terms(&zone.product_safe_description) {
            report.push(
                format!("{subject}.product_safe_description"),
                "internal_term_in_user_copy",
                "Placement zone descriptions must not expose internal terms.",
            );
        }
    }
}

fn validate_identifier(
    report: &mut FlatPanelValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        report.push(
            subject,
            "invalid_identifier",
            "Identifiers must be lowercase ASCII identifiers.",
        );
    }
}

fn validate_label(report: &mut FlatPanelValidationReport, subject: impl Into<String>, value: &str) {
    if value.trim().is_empty() {
        report.push(subject, "missing_label", "Labels must be non-empty.");
    }
}

fn contains_internal_terms(value: &str) -> bool {
    const TERMS: &[&str] = &[
        "kernel",
        "module",
        "provider",
        "slot",
        "placement zone",
        "conformance",
        "fingerprint",
        "topology",
        "uv",
        "rigging",
        "animation",
    ];
    let lower = value.to_ascii_lowercase();
    TERMS.iter().any(|term| lower.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_panel_kernel_validates() {
        let report = validate_flat_panel_kernel(&flat_panel_primitive_kernel());
        assert!(report.is_valid(), "{report:?}");
    }

    #[test]
    fn flat_panel_zones_are_deterministic_and_finite() {
        let first = flat_panel_primitive_zones();
        let second = flat_panel_primitive_zones();
        assert_eq!(first, second);
        for zone in first {
            assert!(zone.normalized_bounds.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn primitive_does_not_claim_motion_or_materials() {
        let kernel = flat_panel_primitive_kernel();
        assert!(!kernel.orientation.claims_hinge_geometry);
        assert!(!kernel.orientation.claims_open_close_motion);
        assert!(
            kernel
                .blocked_capabilities
                .contains(&"surface-material-looks".to_owned())
        );
        assert!(
            kernel
                .blocked_capabilities
                .contains(&"animation".to_owned())
        );
    }

    #[test]
    fn panel_body_module_validates_against_primitive() {
        let report = validate_flat_panel_feature_module(
            &flat_panel_primitive_kernel(),
            &panel_body_module(),
        );
        assert!(report.is_valid(), "{report:?}");
    }

    #[test]
    fn hinge_edge_module_validates_against_primitive_without_claiming_motion() {
        let report = validate_flat_panel_feature_module(
            &flat_panel_primitive_kernel(),
            &hinge_edge_module(),
        );
        assert!(report.is_valid(), "{report:?}");
        let module = hinge_edge_module();
        assert_eq!(
            module.required_zone_kinds,
            vec![FlatPanelZoneKind::HingeCandidateEdge]
        );
        assert_eq!(module.provides_roles, vec!["hinge_edge"]);
        assert_eq!(module.provides_controls, vec!["hinge_edge_style"]);
        assert_eq!(module.candidate_hooks, vec!["hinged-panel-ideas"]);
        assert!(
            module
                .quality_gates
                .contains(&"hinge-edge-not-motion".to_owned())
        );
        assert!(
            module
                .quality_gates
                .contains(&"hinge-edge-attached".to_owned())
        );
        assert!(
            module
                .quality_gates
                .contains(&"hinge-edge-endpoint-visible".to_owned())
        );
    }

    #[test]
    fn missing_required_zone_rejects_module() {
        let mut kernel = flat_panel_primitive_kernel();
        kernel
            .placement_zones
            .retain(|zone| zone.zone_kind != FlatPanelZoneKind::HandleCandidateZone);
        let report = validate_flat_panel_feature_module(&kernel, &panel_handle_module());
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "missing_required_zone")
        );
    }

    #[test]
    fn invalid_zone_bounds_fail() {
        let mut kernel = flat_panel_primitive_kernel();
        kernel.placement_zones[0].normalized_bounds = [0.9, 0.0, 0.1, 1.0];
        let report = validate_flat_panel_kernel(&kernel);
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "invalid_zone_bounds")
        );
    }

    #[test]
    fn user_copy_rejects_internal_terms() {
        let mut module = panel_body_module();
        module.product_safe_summary = "This module exposes provider slots.".to_owned();
        let report = validate_flat_panel_feature_module(&flat_panel_primitive_kernel(), &module);
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "internal_term_in_user_summary")
        );
    }

    #[test]
    fn serde_roundtrip_is_deterministic() {
        let kernel = flat_panel_primitive_kernel();
        let json = serde_json::to_string_pretty(&kernel).expect("serialize flat panel kernel");
        let roundtrip: FlatPanelKernel =
            serde_json::from_str(&json).expect("deserialize flat panel kernel");
        assert_eq!(kernel, roundtrip);
    }
}
