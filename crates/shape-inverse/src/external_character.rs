//! External clean-character canonicalization and diagnostics.
//!
//! This module is intentionally diagnostic-first. It canonicalizes clean
//! external character mesh descriptors, ranks them against the versioned
//! known-base character descriptor contract, and explains why exact editable
//! recovery is or is not eligible. It does not claim strict reconstruction.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shape_character::{
    base::base_topology_library,
    corpus::{
        CharacterRawGeometrySize, KnownBaseCharacterMeshFeatures, KnownBaseCharacterMeshSignature,
        character_mesh_artifact_fingerprint, known_base_character_descriptor_for_features,
        known_base_character_feature_candidates, known_base_character_signature_for_features,
    },
};

use crate::analysis::{Axis3, Confidence};

/// Current schema for external clean-character analysis.
pub const EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION: u32 = 1;

const CANONICAL_UNIT: &str = "meters";
const CANONICAL_COORDINATE_SYSTEM: &str = "y_up_z_forward_right_handed";
const POSITION_TOLERANCE_METERS: f64 = 0.002;

/// Configuration for external clean-character analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterAnalysisConfig {
    /// Minimum confidence for candidates retained in the report.
    pub minimum_candidate_confidence: Confidence,
    /// Require clean-mesh flags before exact known-base eligibility.
    pub require_clean_mesh: bool,
    /// Maximum allowed correspondence residual in meters.
    pub correspondence_tolerance_meters: f64,
}

impl ExternalCharacterAnalysisConfig {
    /// Strict import-triage configuration.
    #[must_use]
    pub fn strict_import_triage() -> Self {
        Self {
            minimum_candidate_confidence: Confidence(0.45),
            require_clean_mesh: true,
            correspondence_tolerance_meters: POSITION_TOLERANCE_METERS,
        }
    }
}

impl Default for ExternalCharacterAnalysisConfig {
    fn default() -> Self {
        Self::strict_import_triage()
    }
}

/// Descriptor-level clean external character mesh input.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCleanCharacterMeshInput {
    /// Input schema version.
    pub schema_version: u32,
    /// Stable source identifier.
    pub source_id: String,
    /// Source coordinate system.
    pub coordinate_system: ExternalCharacterCoordinateSystem,
    /// Raw mesh size.
    pub raw_geometry_size: CharacterRawGeometrySize,
    /// Connected component count.
    pub connected_component_count: usize,
    /// Source-space bounds.
    pub bounds_min: [f32; 3],
    /// Source-space bounds.
    pub bounds_max: [f32; 3],
    /// Optional topology fingerprint from a canonical external mesh pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub topology_fingerprint: Option<String>,
    /// Optional position fingerprint from a canonical external mesh pass.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical_position_fingerprint: Option<String>,
    /// Optional ID-independent semantic descriptor fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_descriptor_fingerprint: Option<String>,
    /// Clean external mesh assumptions.
    pub clean_mesh: ExternalCleanMeshFlags,
    /// Optional semantic correspondence observations from landmarks or labels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub correspondences: Vec<ExternalCharacterCorrespondenceObservation>,
}

/// Source coordinate system for external character meshes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterCoordinateSystem {
    /// Unit label: `meters`, `centimeters`, or `millimeters`.
    pub unit: String,
    /// Source axis that maps to canonical character right.
    pub right_axis: Axis3,
    /// Source axis that maps to canonical character up.
    pub up_axis: Axis3,
    /// Source axis that maps to canonical character forward.
    pub forward_axis: Axis3,
    /// Handedness label for diagnostics.
    pub handedness: CoordinateHandedness,
}

impl ExternalCharacterCoordinateSystem {
    /// Canonical Shape Lab character coordinate system.
    #[must_use]
    pub fn canonical() -> Self {
        Self {
            unit: CANONICAL_UNIT.to_owned(),
            right_axis: Axis3::PositiveX,
            up_axis: Axis3::PositiveY,
            forward_axis: Axis3::PositiveZ,
            handedness: CoordinateHandedness::RightHanded,
        }
    }
}

/// Coordinate-system handedness.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoordinateHandedness {
    /// Right-handed source frame.
    RightHanded,
    /// Left-handed source frame.
    LeftHanded,
}

/// Clean-mesh assumptions required before exact import eligibility.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCleanMeshFlags {
    /// Positions are finite and canonicalized before descriptor extraction.
    pub finite_positions: bool,
    /// Mesh components are manifold enough for deterministic topology analysis.
    pub manifold_components: bool,
    /// Vertex order is stable under canonicalization.
    pub stable_vertex_order: bool,
    /// Face order is stable under canonicalization.
    pub stable_face_order: bool,
    /// No extra disconnected components remain unexplained.
    pub no_unexplained_components: bool,
}

impl ExternalCleanMeshFlags {
    /// All strict clean-mesh assumptions set.
    #[must_use]
    pub fn strict_clean() -> Self {
        Self {
            finite_positions: true,
            manifold_components: true,
            stable_vertex_order: true,
            stable_face_order: true,
            no_unexplained_components: true,
        }
    }

    fn all_clean(self) -> bool {
        self.finite_positions
            && self.manifold_components
            && self.stable_vertex_order
            && self.stable_face_order
            && self.no_unexplained_components
    }
}

/// Optional external semantic correspondence observation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterCorrespondenceObservation {
    /// Stable observation ID.
    pub id: String,
    /// Expected semantic target ID, for example `body.torso`.
    pub target_id: String,
    /// Number of observed support groups for this target.
    pub observed_count: usize,
    /// Maximum correspondence residual in meters after canonicalization.
    pub max_error_meters: f64,
}

/// Full external clean-character analysis report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterAnalysisReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Source identifier.
    pub source_id: String,
    /// Canonicalization result.
    pub canonicalization: ExternalCharacterCanonicalization,
    /// Ranked known-base candidates.
    pub candidates: Vec<ExternalCharacterCandidate>,
    /// Best candidate ID, when one survives the confidence threshold.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_candidate_id: Option<String>,
    /// User-facing import outcome.
    pub outcome: ExternalCharacterImportOutcome,
    /// Aggregate rejection reasons for the source mesh.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejection_reasons: Vec<ExternalCharacterRejectionReason>,
    /// Validation and diagnostic issues.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub issues: Vec<ExternalCharacterIssue>,
}

/// Canonicalization result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterCanonicalization {
    /// True when the source descriptor can be interpreted in canonical space.
    pub accepted: bool,
    /// Canonical unit label.
    pub canonical_units: String,
    /// Canonical coordinate-system label.
    pub canonical_coordinate_system: String,
    /// Source-to-meter scale.
    pub scale_to_meters: f64,
    /// Whether axis remapping was required.
    pub axis_remap_required: bool,
    /// Whether unit scaling was required.
    pub scale_normalization_required: bool,
    /// True when handedness already matches the exact known-base frame.
    pub handedness_exact: bool,
    /// Bounds after scale and axis remap.
    pub canonical_bounds_min: [f32; 3],
    /// Bounds after scale and axis remap.
    pub canonical_bounds_max: [f32; 3],
    /// Deterministic canonicalization confidence.
    pub confidence: Confidence,
}

/// Ranked external known-base character candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterCandidate {
    /// Stable candidate ID.
    pub id: String,
    /// Inferred feature set.
    pub features: KnownBaseCharacterMeshFeatures,
    /// Candidate confidence in `[0, 1]`.
    pub confidence: Confidence,
    /// Versioned character base-library fingerprint.
    pub base_library_fingerprint: String,
    /// Mesh signature evidence.
    pub signature: ExternalCharacterSignatureMatch,
    /// Fingerprint evidence.
    pub fingerprints: ExternalCharacterFingerprintMatch,
    /// Correspondence diagnostics.
    pub correspondences: Vec<ExternalCharacterCorrespondenceDiagnostic>,
    /// True when this external descriptor is eligible for a later strict known-base recovery gate.
    pub exact_known_base_eligible: bool,
    /// Candidate-level rejection reasons.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rejection_reasons: Vec<ExternalCharacterRejectionReason>,
}

/// Signature match diagnostics for a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterSignatureMatch {
    /// Vertex count exactly matches the known-base descriptor.
    pub vertex_count_exact: bool,
    /// Face count exactly matches the known-base descriptor.
    pub face_count_exact: bool,
    /// Connected component count exactly matches.
    pub connected_components_exact: bool,
    /// Canonicalized bounds match the descriptor within tolerance.
    pub bounds_within_tolerance: bool,
    /// Maximum bounds error in meters.
    pub max_bounds_error_meters: f64,
}

impl ExternalCharacterSignatureMatch {
    fn exact(&self) -> bool {
        self.vertex_count_exact
            && self.face_count_exact
            && self.connected_components_exact
            && self.bounds_within_tolerance
    }
}

/// Fingerprint match diagnostics for a candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterFingerprintMatch {
    /// Semantic descriptor fingerprint matches, if supplied.
    pub semantic_descriptor: ExternalFingerprintStatus,
    /// Topology fingerprint matches, if supplied.
    pub topology: ExternalFingerprintStatus,
    /// Canonical-position fingerprint matches, if supplied.
    pub canonical_position: ExternalFingerprintStatus,
}

impl ExternalCharacterFingerprintMatch {
    fn exact(&self) -> bool {
        self.semantic_descriptor == ExternalFingerprintStatus::Matched
            && self.topology == ExternalFingerprintStatus::Matched
            && self.canonical_position == ExternalFingerprintStatus::Matched
    }
}

/// Fingerprint status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalFingerprintStatus {
    /// Fingerprint was present and matched the candidate.
    Matched,
    /// Fingerprint was present and did not match.
    Mismatched,
    /// Fingerprint was not supplied by the external analysis path.
    Missing,
}

/// Correspondence diagnostic for one semantic target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterCorrespondenceDiagnostic {
    /// Semantic target ID.
    pub target_id: String,
    /// Diagnostic status.
    pub status: ExternalCorrespondenceStatus,
    /// Observed item count.
    pub observed_count: usize,
    /// Expected item count.
    pub expected_count: usize,
    /// Maximum correspondence error in meters.
    pub max_error_meters: f64,
    /// Human-readable explanation.
    pub message: String,
}

/// Correspondence status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCorrespondenceStatus {
    /// Expected target was observed with acceptable error.
    Matched,
    /// Expected target was not observed.
    Missing,
    /// More observations were present than expected.
    Surplus,
    /// Target was present but residual exceeded tolerance.
    Drifted,
    /// Observation targeted no known semantic slot for this candidate.
    Unexplained,
}

/// User-facing import triage outcome.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCharacterImportOutcome {
    /// Exact known-base recovery may be attempted by a later strict gate.
    ExactKnownBaseEligible,
    /// Known-base family was identified, but strict recovery is not proven.
    PartialKnownBaseDiagnostic,
    /// Mesh is clean enough to analyze, but no candidate is strong enough.
    DiagnosticOnlyUnsupported,
    /// Input descriptor is invalid or cannot be canonicalized.
    InvalidInput,
}

/// Rejection reason.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterRejectionReason {
    /// Stable path.
    pub path: String,
    /// Reason.
    pub message: String,
}

/// Validation or diagnostic issue.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalCharacterIssue {
    /// Severity.
    pub severity: ExternalCharacterIssueSeverity,
    /// Stable path.
    pub path: String,
    /// Message.
    pub message: String,
}

/// Issue severity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCharacterIssueSeverity {
    /// Non-blocking diagnostic.
    Warning,
    /// Blocking validation problem.
    Error,
}

/// Analyze an external clean-character descriptor.
#[must_use]
pub fn analyze_external_clean_character_mesh(
    input: &ExternalCleanCharacterMeshInput,
    config: &ExternalCharacterAnalysisConfig,
) -> ExternalCharacterAnalysisReport {
    let mut issues = collect_input_issues(input, config);
    let canonicalization = canonicalize_input(input, &mut issues);
    let input_has_errors = issues
        .iter()
        .any(|issue| issue.severity == ExternalCharacterIssueSeverity::Error);
    let candidates = if input_has_errors || !canonicalization.accepted {
        Vec::new()
    } else {
        classify_candidates(input, &canonicalization, config)
    };
    let best_candidate_id = candidates.first().map(|candidate| candidate.id.clone());
    let outcome = classify_outcome(&canonicalization, &candidates, input_has_errors);
    let mut rejection_reasons = report_rejection_reasons(&canonicalization, &candidates, outcome);
    rejection_reasons.sort();
    rejection_reasons.dedup();

    issues.sort();
    issues.dedup();

    ExternalCharacterAnalysisReport {
        schema_version: EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION,
        source_id: input.source_id.clone(),
        canonicalization,
        candidates,
        best_candidate_id,
        outcome,
        rejection_reasons,
        issues,
    }
}

fn classify_candidates(
    input: &ExternalCleanCharacterMeshInput,
    canonicalization: &ExternalCharacterCanonicalization,
    config: &ExternalCharacterAnalysisConfig,
) -> Vec<ExternalCharacterCandidate> {
    let mut candidates = known_base_character_feature_candidates()
        .into_iter()
        .filter_map(|features| {
            let signature = known_base_character_signature_for_features(features);
            let descriptor = known_base_character_descriptor_for_features(features);
            let signature_match = signature_match(input, canonicalization, signature);
            let fingerprints = fingerprint_match(input, &descriptor);
            let correspondences =
                correspondence_diagnostics(input, features, config.correspondence_tolerance_meters);
            let correspondence_score = correspondence_score(&correspondences);
            let confidence = candidate_confidence(
                &signature_match,
                &fingerprints,
                correspondence_score,
                input.clean_mesh.all_clean(),
                canonicalization.accepted,
            );
            (confidence >= config.minimum_candidate_confidence.0).then(|| {
                let mut rejection_reasons = candidate_rejection_reasons(
                    input,
                    canonicalization,
                    &signature_match,
                    &fingerprints,
                    &correspondences,
                    config,
                );
                rejection_reasons.sort();
                rejection_reasons.dedup();
                let exact_known_base_eligible = rejection_reasons.is_empty()
                    && signature_match.exact()
                    && fingerprints.exact()
                    && canonicalization.handedness_exact
                    && (!config.require_clean_mesh || input.clean_mesh.all_clean());
                ExternalCharacterCandidate {
                    id: candidate_id(features),
                    features,
                    confidence: Confidence(confidence),
                    base_library_fingerprint: base_topology_library().fingerprint().0,
                    signature: signature_match,
                    fingerprints,
                    correspondences,
                    exact_known_base_eligible,
                    rejection_reasons,
                }
            })
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .confidence
            .partial_cmp(&left.confidence)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.id.cmp(&right.id))
    });
    candidates
}

fn signature_match(
    input: &ExternalCleanCharacterMeshInput,
    canonicalization: &ExternalCharacterCanonicalization,
    signature: KnownBaseCharacterMeshSignature,
) -> ExternalCharacterSignatureMatch {
    let expected_min = bounds_from_bits(signature.bounds_min_bits);
    let expected_max = bounds_from_bits(signature.bounds_max_bits);
    let bounds_error = max_bounds_error(
        canonicalization.canonical_bounds_min,
        canonicalization.canonical_bounds_max,
        expected_min,
        expected_max,
    );
    ExternalCharacterSignatureMatch {
        vertex_count_exact: input.raw_geometry_size.vertex_count == signature.vertex_count,
        face_count_exact: input.raw_geometry_size.face_count == signature.face_count,
        connected_components_exact: input.connected_component_count
            == signature.connected_component_count,
        bounds_within_tolerance: bounds_error <= POSITION_TOLERANCE_METERS,
        max_bounds_error_meters: bounds_error,
    }
}

fn fingerprint_match(
    input: &ExternalCleanCharacterMeshInput,
    descriptor: &shape_character::corpus::KnownBaseCharacterMeshDescriptor,
) -> ExternalCharacterFingerprintMatch {
    ExternalCharacterFingerprintMatch {
        semantic_descriptor: fingerprint_status(
            input.semantic_descriptor_fingerprint.as_deref(),
            &descriptor.semantic_descriptor_fingerprint,
        ),
        topology: fingerprint_status(
            input.topology_fingerprint.as_deref(),
            &descriptor.topology_fingerprint,
        ),
        canonical_position: fingerprint_status(
            input.canonical_position_fingerprint.as_deref(),
            &descriptor.canonical_position_fingerprint,
        ),
    }
}

fn fingerprint_status(found: Option<&str>, expected: &str) -> ExternalFingerprintStatus {
    match found {
        Some(found) if found == expected => ExternalFingerprintStatus::Matched,
        Some(_) => ExternalFingerprintStatus::Mismatched,
        None => ExternalFingerprintStatus::Missing,
    }
}

fn candidate_confidence(
    signature: &ExternalCharacterSignatureMatch,
    fingerprints: &ExternalCharacterFingerprintMatch,
    correspondence_score: f64,
    clean_mesh: bool,
    canonicalized: bool,
) -> f64 {
    let signature_score = [
        signature.vertex_count_exact,
        signature.face_count_exact,
        signature.connected_components_exact,
        signature.bounds_within_tolerance,
    ]
    .into_iter()
    .filter(|matched| *matched)
    .count() as f64
        / 4.0;
    let fingerprint_score = [
        fingerprints.semantic_descriptor,
        fingerprints.topology,
        fingerprints.canonical_position,
    ]
    .into_iter()
    .map(|status| match status {
        ExternalFingerprintStatus::Matched => 1.0,
        ExternalFingerprintStatus::Missing => 0.35,
        ExternalFingerprintStatus::Mismatched => 0.0,
    })
    .sum::<f64>()
        / 3.0;
    (0.12 * f64::from(canonicalized)
        + 0.10 * f64::from(clean_mesh)
        + 0.43 * signature_score
        + 0.25 * fingerprint_score
        + 0.10 * correspondence_score)
        .clamp(0.0, 1.0)
}

fn correspondence_diagnostics(
    input: &ExternalCleanCharacterMeshInput,
    features: KnownBaseCharacterMeshFeatures,
    tolerance: f64,
) -> Vec<ExternalCharacterCorrespondenceDiagnostic> {
    let expected = expected_correspondence_targets(features);
    let expected_set = expected.iter().copied().collect::<BTreeSet<_>>();
    let mut diagnostics = Vec::new();
    for target_id in expected {
        let observations = input
            .correspondences
            .iter()
            .filter(|observation| observation.target_id == target_id)
            .collect::<Vec<_>>();
        if observations.is_empty() {
            diagnostics.push(correspondence(
                target_id,
                ExternalCorrespondenceStatus::Missing,
                0,
                1,
                0.0,
                "expected semantic target was not observed",
            ));
            continue;
        }

        let observed_count = observations.iter().fold(0usize, |sum, observation| {
            sum.saturating_add(observation.observed_count)
        });
        let max_error_meters = observations
            .iter()
            .map(|observation| observation.max_error_meters)
            .fold(0.0, f64::max);
        let tolerance_allows =
            tolerance.is_finite() && tolerance >= 0.0 && max_error_meters <= tolerance;
        let (status, message) = if observations.len() > 1 {
            (
                ExternalCorrespondenceStatus::Surplus,
                "semantic target has duplicate external correspondence observations",
            )
        } else if observed_count == 0 {
            (
                ExternalCorrespondenceStatus::Missing,
                "expected semantic target was labelled but has no observed support",
            )
        } else if observed_count > 1 {
            (
                ExternalCorrespondenceStatus::Surplus,
                "semantic target has more observed support groups than expected",
            )
        } else if !tolerance_allows {
            (
                ExternalCorrespondenceStatus::Drifted,
                "semantic target residual exceeds the clean-character tolerance",
            )
        } else {
            (
                ExternalCorrespondenceStatus::Matched,
                "semantic target correspondence is within tolerance",
            )
        };
        diagnostics.push(correspondence(
            target_id,
            status,
            observed_count,
            1,
            max_error_meters,
            message,
        ));
    }
    for observation in &input.correspondences {
        if !expected_set.contains(observation.target_id.as_str()) {
            diagnostics.push(correspondence(
                &observation.target_id,
                ExternalCorrespondenceStatus::Unexplained,
                observation.observed_count,
                0,
                observation.max_error_meters,
                "external correspondence target is outside this known-base candidate",
            ));
        }
    }
    diagnostics.sort_by(|left, right| {
        (
            left.target_id.as_str(),
            left.status,
            left.observed_count,
            left.expected_count,
        )
            .cmp(&(
                right.target_id.as_str(),
                right.status,
                right.observed_count,
                right.expected_count,
            ))
    });
    diagnostics
}

fn correspondence_score(diagnostics: &[ExternalCharacterCorrespondenceDiagnostic]) -> f64 {
    if diagnostics.is_empty() {
        return 0.35;
    }
    let matched = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.status == ExternalCorrespondenceStatus::Matched)
        .count() as f64;
    matched / diagnostics.len() as f64
}

fn candidate_rejection_reasons(
    input: &ExternalCleanCharacterMeshInput,
    canonicalization: &ExternalCharacterCanonicalization,
    signature: &ExternalCharacterSignatureMatch,
    fingerprints: &ExternalCharacterFingerprintMatch,
    correspondences: &[ExternalCharacterCorrespondenceDiagnostic],
    config: &ExternalCharacterAnalysisConfig,
) -> Vec<ExternalCharacterRejectionReason> {
    let mut reasons = Vec::new();
    if config.require_clean_mesh && !input.clean_mesh.all_clean() {
        reasons.push(rejection(
            "clean_mesh",
            "clean mesh assumptions are not all satisfied",
        ));
    }
    if !canonicalization.handedness_exact {
        reasons.push(rejection(
            "canonicalization.handedness",
            "left-handed source is diagnostic-only for exact recovery",
        ));
    }
    if !signature.exact() {
        reasons.push(rejection(
            "candidate.signature",
            "mesh counts, component count, or canonical bounds do not exactly match",
        ));
    }
    if !fingerprints.exact() {
        reasons.push(rejection(
            "candidate.fingerprints",
            "semantic, topology, and position fingerprints are not all present and matched",
        ));
    }
    if correspondences.iter().any(|diagnostic| {
        matches!(
            diagnostic.status,
            ExternalCorrespondenceStatus::Surplus
                | ExternalCorrespondenceStatus::Drifted
                | ExternalCorrespondenceStatus::Unexplained
        )
    }) {
        reasons.push(rejection(
            "candidate.correspondence",
            "one or more semantic correspondences are drifted, surplus, or unexplained",
        ));
    }
    reasons
}

fn report_rejection_reasons(
    canonicalization: &ExternalCharacterCanonicalization,
    candidates: &[ExternalCharacterCandidate],
    outcome: ExternalCharacterImportOutcome,
) -> Vec<ExternalCharacterRejectionReason> {
    let mut reasons = Vec::new();
    if !canonicalization.accepted {
        reasons.push(rejection(
            "canonicalization",
            "source coordinate system or units could not be canonicalized",
        ));
    }
    match outcome {
        ExternalCharacterImportOutcome::ExactKnownBaseEligible => {}
        ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic => {
            if let Some(best) = candidates.first() {
                reasons.extend(best.rejection_reasons.clone());
            }
        }
        ExternalCharacterImportOutcome::DiagnosticOnlyUnsupported => reasons.push(rejection(
            "classification",
            "no known-base character candidate reached the reporting confidence threshold",
        )),
        ExternalCharacterImportOutcome::InvalidInput => reasons.push(rejection(
            "input",
            "external character descriptor failed validation",
        )),
    }
    reasons
}

fn classify_outcome(
    canonicalization: &ExternalCharacterCanonicalization,
    candidates: &[ExternalCharacterCandidate],
    input_has_errors: bool,
) -> ExternalCharacterImportOutcome {
    if input_has_errors || !canonicalization.accepted {
        return ExternalCharacterImportOutcome::InvalidInput;
    }
    if candidates
        .iter()
        .any(|candidate| candidate.exact_known_base_eligible)
    {
        ExternalCharacterImportOutcome::ExactKnownBaseEligible
    } else if candidates.is_empty() {
        ExternalCharacterImportOutcome::DiagnosticOnlyUnsupported
    } else {
        ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
    }
}

fn canonicalize_input(
    input: &ExternalCleanCharacterMeshInput,
    issues: &mut Vec<ExternalCharacterIssue>,
) -> ExternalCharacterCanonicalization {
    let scale_to_meters = match input.coordinate_system.unit.as_str() {
        "meters" => Some(1.0),
        "centimeters" => Some(0.01),
        "millimeters" => Some(0.001),
        _ => None,
    };
    let axes_valid = axes_are_orthogonal(
        input.coordinate_system.right_axis,
        input.coordinate_system.up_axis,
        input.coordinate_system.forward_axis,
    );
    let axis_parity_right_handed = axes_form_right_handed_basis(
        input.coordinate_system.right_axis,
        input.coordinate_system.up_axis,
        input.coordinate_system.forward_axis,
    );
    let handedness_exact = input.coordinate_system.handedness == CoordinateHandedness::RightHanded
        && axis_parity_right_handed;
    if !axes_valid {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "coordinate_system",
            "right, up, and forward axes must refer to three distinct source axes",
        ));
    }
    if scale_to_meters.is_none() {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "coordinate_system.unit",
            "unit must be meters, centimeters, or millimeters",
        ));
    }
    if !handedness_exact {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Warning,
            "coordinate_system.handedness",
            "source handedness label and signed axis parity must be right-handed for exact recovery",
        ));
    }
    let scale = scale_to_meters.unwrap_or(1.0);
    let (canonical_bounds_min, canonical_bounds_max) = transform_bounds(
        input.bounds_min,
        input.bounds_max,
        input.coordinate_system.right_axis,
        input.coordinate_system.up_axis,
        input.coordinate_system.forward_axis,
        scale,
    );
    let axis_remap_required = input.coordinate_system.right_axis != Axis3::PositiveX
        || input.coordinate_system.up_axis != Axis3::PositiveY
        || input.coordinate_system.forward_axis != Axis3::PositiveZ;
    let scale_normalization_required = (scale - 1.0).abs() > f64::EPSILON;
    let accepted = axes_valid && scale_to_meters.is_some();
    ExternalCharacterCanonicalization {
        accepted,
        canonical_units: CANONICAL_UNIT.to_owned(),
        canonical_coordinate_system: CANONICAL_COORDINATE_SYSTEM.to_owned(),
        scale_to_meters: scale,
        axis_remap_required,
        scale_normalization_required,
        handedness_exact,
        canonical_bounds_min,
        canonical_bounds_max,
        confidence: Confidence(if accepted { 1.0 } else { 0.0 }),
    }
}

fn collect_input_issues(
    input: &ExternalCleanCharacterMeshInput,
    config: &ExternalCharacterAnalysisConfig,
) -> Vec<ExternalCharacterIssue> {
    let mut issues = Vec::new();
    if !config.minimum_candidate_confidence.is_valid() {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "config.minimum_candidate_confidence",
            "minimum candidate confidence must be finite and in [0, 1]",
        ));
    }
    if !config.correspondence_tolerance_meters.is_finite()
        || config.correspondence_tolerance_meters < 0.0
    {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "config.correspondence_tolerance_meters",
            "correspondence tolerance must be finite and non-negative",
        ));
    }
    if input.schema_version != EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "schema_version",
            "unsupported external character analysis schema version",
        ));
    }
    if input.source_id.trim().is_empty() {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "source_id",
            "source ID must not be empty",
        ));
    }
    if input.raw_geometry_size.vertex_count == 0 || input.raw_geometry_size.face_count == 0 {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "raw_geometry_size",
            "external character mesh must contain vertices and faces",
        ));
    }
    let expected_position_bytes = input
        .raw_geometry_size
        .vertex_count
        .checked_mul(3)
        .and_then(|value| value.checked_mul(4));
    if expected_position_bytes != Some(input.raw_geometry_size.position_bytes) {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "raw_geometry_size.position_bytes",
            "position byte count must match f32 xyz storage",
        ));
    }
    let expected_topology_bytes = input
        .raw_geometry_size
        .face_count
        .checked_mul(3)
        .and_then(|value| value.checked_mul(4));
    if expected_topology_bytes != Some(input.raw_geometry_size.topology_bytes) {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "raw_geometry_size.topology_bytes",
            "topology byte count must match u32 triangle index storage",
        ));
    }
    if input.connected_component_count == 0 {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "connected_component_count",
            "external character mesh must have at least one component",
        ));
    }
    if !bounds_are_valid(input.bounds_min, input.bounds_max) {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Error,
            "bounds",
            "bounds must be finite and ordered",
        ));
    }
    if config.require_clean_mesh && !input.clean_mesh.all_clean() {
        issues.push(issue(
            ExternalCharacterIssueSeverity::Warning,
            "clean_mesh",
            "mesh can be classified, but exact recovery eligibility requires all clean flags",
        ));
    }
    let mut correspondence_ids = BTreeSet::new();
    for correspondence in &input.correspondences {
        let correspondence_id = correspondence.id.trim();
        if correspondence_id.is_empty() || correspondence.target_id.trim().is_empty() {
            issues.push(issue(
                ExternalCharacterIssueSeverity::Error,
                "correspondences",
                "correspondence IDs and target IDs must not be empty",
            ));
        } else if !correspondence_ids.insert(correspondence_id.to_owned()) {
            issues.push(issue(
                ExternalCharacterIssueSeverity::Error,
                "correspondences.id",
                "correspondence IDs must be unique",
            ));
        }
        if !correspondence.max_error_meters.is_finite() || correspondence.max_error_meters < 0.0 {
            issues.push(issue(
                ExternalCharacterIssueSeverity::Error,
                "correspondences.max_error_meters",
                "correspondence error must be finite and non-negative",
            ));
        }
    }
    issues
}

fn expected_correspondence_targets(features: KnownBaseCharacterMeshFeatures) -> Vec<&'static str> {
    let mut targets = vec![
        "character.body",
        "head.cranium",
        "body.torso",
        "head.scalp",
        "body.loop.neck",
    ];
    if features.posed {
        targets.push("body.pose.joint_chain");
    }
    if features.asymmetric {
        targets.push("body.shoulder.left");
    }
    if features.clothed {
        targets.push("garment.torso_shell");
        targets.push("garment.neck_opening");
    }
    if features.hair {
        targets.push("hair.mass.primary");
        targets.push("hair.card.primary");
    }
    targets
}

fn candidate_id(features: KnownBaseCharacterMeshFeatures) -> String {
    format!(
        "external.character.candidate.sym{}_asym{}_pose{}_cloth{}_hair{}_topo{}",
        u8::from(features.symmetric),
        u8::from(features.asymmetric),
        u8::from(features.posed),
        u8::from(features.clothed),
        u8::from(features.hair),
        u8::from(features.topology_edited)
    )
}

fn correspondence(
    target_id: &str,
    status: ExternalCorrespondenceStatus,
    observed_count: usize,
    expected_count: usize,
    max_error_meters: f64,
    message: &str,
) -> ExternalCharacterCorrespondenceDiagnostic {
    ExternalCharacterCorrespondenceDiagnostic {
        target_id: target_id.to_owned(),
        status,
        observed_count,
        expected_count,
        max_error_meters,
        message: message.to_owned(),
    }
}

fn rejection(path: &str, message: &str) -> ExternalCharacterRejectionReason {
    ExternalCharacterRejectionReason {
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

fn issue(
    severity: ExternalCharacterIssueSeverity,
    path: &str,
    message: &str,
) -> ExternalCharacterIssue {
    ExternalCharacterIssue {
        severity,
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

fn bounds_from_bits(bits: [u32; 3]) -> [f32; 3] {
    [
        f32::from_bits(bits[0]),
        f32::from_bits(bits[1]),
        f32::from_bits(bits[2]),
    ]
}

fn max_bounds_error(
    min: [f32; 3],
    max: [f32; 3],
    expected_min: [f32; 3],
    expected_max: [f32; 3],
) -> f64 {
    min.into_iter()
        .chain(max)
        .zip(expected_min.into_iter().chain(expected_max))
        .map(|(actual, expected)| f64::from((actual - expected).abs()))
        .fold(0.0, f64::max)
}

fn transform_bounds(
    min: [f32; 3],
    max: [f32; 3],
    right_axis: Axis3,
    up_axis: Axis3,
    forward_axis: Axis3,
    scale: f64,
) -> ([f32; 3], [f32; 3]) {
    let mut out_min = [f64::INFINITY; 3];
    let mut out_max = [f64::NEG_INFINITY; 3];
    for x in [min[0], max[0]] {
        for y in [min[1], max[1]] {
            for z in [min[2], max[2]] {
                let point = [f64::from(x), f64::from(y), f64::from(z)];
                let mapped = [
                    axis_value(point, right_axis) * scale,
                    axis_value(point, up_axis) * scale,
                    axis_value(point, forward_axis) * scale,
                ];
                for axis in 0..3 {
                    out_min[axis] = out_min[axis].min(mapped[axis]);
                    out_max[axis] = out_max[axis].max(mapped[axis]);
                }
            }
        }
    }
    (
        [out_min[0] as f32, out_min[1] as f32, out_min[2] as f32],
        [out_max[0] as f32, out_max[1] as f32, out_max[2] as f32],
    )
}

fn axis_value(point: [f64; 3], axis: Axis3) -> f64 {
    match axis {
        Axis3::PositiveX => point[0],
        Axis3::NegativeX => -point[0],
        Axis3::PositiveY => point[1],
        Axis3::NegativeY => -point[1],
        Axis3::PositiveZ => point[2],
        Axis3::NegativeZ => -point[2],
    }
}

fn axes_are_orthogonal(right: Axis3, up: Axis3, forward: Axis3) -> bool {
    let axes = [axis_family(right), axis_family(up), axis_family(forward)];
    axes[0] != axes[1] && axes[0] != axes[2] && axes[1] != axes[2]
}

fn axes_form_right_handed_basis(right: Axis3, up: Axis3, forward: Axis3) -> bool {
    let right = axis_vector(right);
    let up = axis_vector(up);
    let forward = axis_vector(forward);
    determinant_3x3(right, up, forward) == 1
}

fn axis_vector(axis: Axis3) -> [i32; 3] {
    match axis {
        Axis3::PositiveX => [1, 0, 0],
        Axis3::NegativeX => [-1, 0, 0],
        Axis3::PositiveY => [0, 1, 0],
        Axis3::NegativeY => [0, -1, 0],
        Axis3::PositiveZ => [0, 0, 1],
        Axis3::NegativeZ => [0, 0, -1],
    }
}

fn determinant_3x3(right: [i32; 3], up: [i32; 3], forward: [i32; 3]) -> i32 {
    right[0] * (up[1] * forward[2] - up[2] * forward[1])
        - right[1] * (up[0] * forward[2] - up[2] * forward[0])
        + right[2] * (up[0] * forward[1] - up[1] * forward[0])
}

fn axis_family(axis: Axis3) -> u8 {
    match axis {
        Axis3::PositiveX | Axis3::NegativeX => 0,
        Axis3::PositiveY | Axis3::NegativeY => 1,
        Axis3::PositiveZ | Axis3::NegativeZ => 2,
    }
}

fn bounds_are_valid(min: [f32; 3], max: [f32; 3]) -> bool {
    min.iter().all(|value| value.is_finite())
        && max.iter().all(|value| value.is_finite())
        && min[0] < max[0]
        && min[1] < max[1]
        && min[2] < max[2]
}

/// Convert a Wave 17/18 public corpus artifact into an external descriptor
/// fixture. This intentionally exposes only public mesh descriptor fields.
#[must_use]
pub fn external_input_from_character_mesh_artifact(
    source_id: impl Into<String>,
    mesh: &shape_character::corpus::CharacterMeshArtifact,
) -> ExternalCleanCharacterMeshInput {
    ExternalCleanCharacterMeshInput {
        schema_version: EXTERNAL_CHARACTER_ANALYSIS_SCHEMA_VERSION,
        source_id: source_id.into(),
        coordinate_system: ExternalCharacterCoordinateSystem::canonical(),
        raw_geometry_size: mesh.raw_geometry_size,
        connected_component_count: mesh.connected_component_count,
        bounds_min: mesh.bounds_min,
        bounds_max: mesh.bounds_max,
        topology_fingerprint: Some(mesh.topology_fingerprint.clone()),
        canonical_position_fingerprint: Some(mesh.canonical_position_fingerprint.clone()),
        semantic_descriptor_fingerprint: Some(mesh.semantic_descriptor_fingerprint.clone()),
        clean_mesh: ExternalCleanMeshFlags::strict_clean(),
        correspondences: Vec::new(),
    }
}

/// Refresh the public artifact fingerprint after descriptor-only test edits.
#[must_use]
pub fn external_character_artifact_fingerprint(
    artifact: &shape_character::corpus::CharacterMeshArtifact,
) -> String {
    character_mesh_artifact_fingerprint(artifact)
}

#[cfg(test)]
mod tests {
    use shape_character::corpus::{
        CharacterMeshArtifact, generated_character_corpus,
        known_base_character_descriptor_for_features, known_base_character_signature_for_features,
    };

    use super::*;

    #[test]
    fn exact_public_character_mesh_is_exact_known_base_eligible() {
        let corpus = generated_character_corpus(201);
        let input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.exact",
            &corpus.cases[4].mesh,
        ));

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::ExactKnownBaseEligible
        );
        let best = report
            .candidates
            .first()
            .expect("candidate should be ranked");
        assert!(best.exact_known_base_eligible);
        assert!(report.canonicalization.handedness_exact);
        assert!(best.signature.bounds_within_tolerance);
        assert_eq!(
            best.fingerprints.topology,
            ExternalFingerprintStatus::Matched
        );
        assert_eq!(
            best.fingerprints.canonical_position,
            ExternalFingerprintStatus::Matched
        );
    }

    #[test]
    fn canonicalization_normalizes_centimeter_z_up_descriptors() {
        let corpus = generated_character_corpus(202);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.cm_z_up",
            &corpus.cases[0].mesh,
        );
        input.coordinate_system = ExternalCharacterCoordinateSystem {
            unit: "centimeters".to_owned(),
            right_axis: Axis3::PositiveX,
            up_axis: Axis3::PositiveZ,
            forward_axis: Axis3::NegativeY,
            handedness: CoordinateHandedness::RightHanded,
        };
        let original_min = input.bounds_min;
        let original_max = input.bounds_max;
        input.bounds_min = [
            original_min[0] * 100.0,
            -original_max[2] * 100.0,
            original_min[1] * 100.0,
        ];
        input.bounds_max = [
            original_max[0] * 100.0,
            -original_min[2] * 100.0,
            original_max[1] * 100.0,
        ];

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert!(report.canonicalization.accepted);
        assert!(report.canonicalization.axis_remap_required);
        assert!(report.canonicalization.scale_normalization_required);
        assert_eq!(report.canonicalization.scale_to_meters, 0.01);
        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::ExactKnownBaseEligible
        );
    }

    #[test]
    fn canonical_axis_unit_scaling_does_not_report_axis_remap() {
        let corpus = generated_character_corpus(212);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.cm_canonical_axes",
            &corpus.cases[0].mesh,
        );
        input.coordinate_system.unit = "centimeters".to_owned();
        input.bounds_min = [
            input.bounds_min[0] * 100.0,
            input.bounds_min[1] * 100.0,
            input.bounds_min[2] * 100.0,
        ];
        input.bounds_max = [
            input.bounds_max[0] * 100.0,
            input.bounds_max[1] * 100.0,
            input.bounds_max[2] * 100.0,
        ];

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert!(report.canonicalization.accepted);
        assert!(!report.canonicalization.axis_remap_required);
        assert!(report.canonicalization.scale_normalization_required);
        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::ExactKnownBaseEligible
        );
    }

    #[test]
    fn invalid_config_blocks_candidates_and_exact_eligibility() {
        let corpus = generated_character_corpus(209);
        let input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.invalid_config",
            &corpus.cases[4].mesh,
        ));
        let config = ExternalCharacterAnalysisConfig {
            minimum_candidate_confidence: Confidence(f64::NAN),
            require_clean_mesh: true,
            correspondence_tolerance_meters: f64::NAN,
        };

        let report = analyze_external_clean_character_mesh(&input, &config);

        assert_eq!(report.outcome, ExternalCharacterImportOutcome::InvalidInput);
        assert!(report.candidates.is_empty());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "config.minimum_candidate_confidence")
        );
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "config.correspondence_tolerance_meters")
        );
    }

    #[test]
    fn left_handed_input_is_diagnostic_not_exact() {
        let corpus = generated_character_corpus(210);
        let mut input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.left_handed",
            &corpus.cases[4].mesh,
        ));
        input.coordinate_system.handedness = CoordinateHandedness::LeftHanded;

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert!(report.canonicalization.accepted);
        assert!(!report.canonicalization.handedness_exact);
        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        let best = report.candidates.first().expect("diagnostic candidate");
        assert!(!best.exact_known_base_eligible);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "coordinate_system.handedness")
        );
        assert!(
            best.rejection_reasons
                .iter()
                .any(|reason| reason.path == "canonicalization.handedness")
        );
    }

    #[test]
    fn axis_parity_mismatch_is_diagnostic_not_exact() {
        let corpus = generated_character_corpus(213);
        let mut input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.axis_parity",
            &corpus.cases[4].mesh,
        ));
        input.coordinate_system = ExternalCharacterCoordinateSystem {
            unit: "meters".to_owned(),
            right_axis: Axis3::PositiveX,
            up_axis: Axis3::PositiveZ,
            forward_axis: Axis3::PositiveY,
            handedness: CoordinateHandedness::RightHanded,
        };
        input.bounds_min = [
            input.bounds_min[0],
            input.bounds_min[2],
            input.bounds_min[1],
        ];
        input.bounds_max = [
            input.bounds_max[0],
            input.bounds_max[2],
            input.bounds_max[1],
        ];

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert!(report.canonicalization.accepted);
        assert!(!report.canonicalization.handedness_exact);
        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        let best = report.candidates.first().expect("diagnostic candidate");
        assert!(!best.exact_known_base_eligible);
        assert!(
            best.rejection_reasons
                .iter()
                .any(|reason| reason.path == "canonicalization.handedness")
        );
    }

    #[test]
    fn missing_fingerprints_keep_classification_partial_not_exact() {
        let corpus = generated_character_corpus(203);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.partial",
            &corpus.cases[2].mesh,
        );
        input.topology_fingerprint = None;
        input.canonical_position_fingerprint = None;
        input.semantic_descriptor_fingerprint = None;

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        let best = report.candidates.first().expect("partial candidate");
        assert!(!best.exact_known_base_eligible);
        assert_eq!(
            best.fingerprints.topology,
            ExternalFingerprintStatus::Missing
        );
        assert!(best.rejection_reasons.iter().any(|reason| {
            reason.path == "candidate.fingerprints"
                && reason.message.contains("not all present and matched")
        }));
    }

    #[test]
    fn stale_topology_or_position_fingerprints_are_diagnostic_failures() {
        let corpus = generated_character_corpus(204);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.stale",
            &corpus.cases[1].mesh,
        );
        input.topology_fingerprint = Some("stale".to_owned());

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        assert_eq!(
            report.candidates[0].fingerprints.topology,
            ExternalFingerprintStatus::Mismatched
        );
        assert!(!report.candidates[0].exact_known_base_eligible);

        let mut input = external_input_from_character_mesh_artifact(
            "external.case.stale_pos",
            &corpus.cases[1].mesh,
        );
        input.canonical_position_fingerprint = Some("stale".to_owned());
        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );
        assert_eq!(
            report.candidates[0].fingerprints.canonical_position,
            ExternalFingerprintStatus::Mismatched
        );
    }

    #[test]
    fn correspondence_diagnostics_report_missing_drifted_and_unexplained_targets() {
        let corpus = generated_character_corpus(205);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.correspondence",
            &corpus.cases[4].mesh,
        );
        input.correspondences = vec![
            ExternalCharacterCorrespondenceObservation {
                id: "obs.body".to_owned(),
                target_id: "character.body".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            },
            ExternalCharacterCorrespondenceObservation {
                id: "obs.hair".to_owned(),
                target_id: "hair.mass.primary".to_owned(),
                observed_count: 1,
                max_error_meters: 0.25,
            },
            ExternalCharacterCorrespondenceObservation {
                id: "obs.extra".to_owned(),
                target_id: "prop.sword".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            },
        ];

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );
        let candidate = report
            .candidates
            .iter()
            .find(|candidate| candidate.features.hair)
            .expect("hair candidate");

        assert!(candidate.correspondences.iter().any(|diagnostic| {
            diagnostic.target_id == "hair.mass.primary"
                && diagnostic.status == ExternalCorrespondenceStatus::Drifted
        }));
        assert!(candidate.correspondences.iter().any(|diagnostic| {
            diagnostic.target_id == "prop.sword"
                && diagnostic.status == ExternalCorrespondenceStatus::Unexplained
        }));
        assert!(
            candidate
                .correspondences
                .iter()
                .any(|diagnostic| { diagnostic.status == ExternalCorrespondenceStatus::Missing })
        );
    }

    #[test]
    fn single_surplus_support_observation_blocks_exact_eligibility() {
        let corpus = generated_character_corpus(214);
        let mut input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.surplus_support",
            &corpus.cases[4].mesh,
        ));
        let body = input
            .correspondences
            .iter_mut()
            .find(|observation| observation.target_id == "character.body")
            .expect("body observation");
        body.observed_count = 2;

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        let best = report.candidates.first().expect("diagnostic candidate");
        assert!(!best.exact_known_base_eligible);
        assert!(best.correspondences.iter().any(|diagnostic| {
            diagnostic.target_id == "character.body"
                && diagnostic.status == ExternalCorrespondenceStatus::Surplus
        }));
    }

    #[test]
    fn duplicate_correspondence_ids_are_invalid_input() {
        let corpus = generated_character_corpus(215);
        let mut input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.duplicate_correspondence_id",
            &corpus.cases[4].mesh,
        ));
        input.correspondences[1].id = input.correspondences[0].id.clone();

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(report.outcome, ExternalCharacterImportOutcome::InvalidInput);
        assert!(report.candidates.is_empty());
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.path == "correspondences.id")
        );
    }

    #[test]
    fn external_input_json_rejects_hidden_program_fields() {
        let corpus = generated_character_corpus(216);
        let input = external_input_from_character_mesh_artifact(
            "external.case.hidden_field",
            &corpus.cases[0].mesh,
        );
        let mut value = serde_json::to_value(&input).expect("serialize external input");
        value
            .as_object_mut()
            .expect("external input object")
            .insert(
                "source_program".to_owned(),
                serde_json::json!({"program_id": "private.answer.key"}),
            );

        let error = serde_json::from_value::<ExternalCleanCharacterMeshInput>(value)
            .expect_err("hidden program field must be rejected");

        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn surplus_and_unexplained_correspondences_block_exact_eligibility() {
        let corpus = generated_character_corpus(211);
        let mut input = with_expected_correspondences(external_input_from_character_mesh_artifact(
            "external.case.surplus_correspondence",
            &corpus.cases[4].mesh,
        ));
        input
            .correspondences
            .push(ExternalCharacterCorrespondenceObservation {
                id: "obs.body.duplicate".to_owned(),
                target_id: "character.body".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            });
        input
            .correspondences
            .push(ExternalCharacterCorrespondenceObservation {
                id: "obs.extra".to_owned(),
                target_id: "prop.sword".to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            });

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::PartialKnownBaseDiagnostic
        );
        let best = report.candidates.first().expect("diagnostic candidate");
        assert!(!best.exact_known_base_eligible);
        assert!(best.correspondences.iter().any(|diagnostic| {
            diagnostic.target_id == "character.body"
                && diagnostic.status == ExternalCorrespondenceStatus::Surplus
        }));
        assert!(best.correspondences.iter().any(|diagnostic| {
            diagnostic.target_id == "prop.sword"
                && diagnostic.status == ExternalCorrespondenceStatus::Unexplained
        }));
        assert!(
            best.rejection_reasons
                .iter()
                .any(|reason| reason.path == "candidate.correspondence")
        );
    }

    #[test]
    fn invalid_dirty_external_mesh_stays_invalid_input() {
        let corpus = generated_character_corpus(206);
        let mut input = external_input_from_character_mesh_artifact(
            "external.case.invalid",
            &corpus.cases[0].mesh,
        );
        input.bounds_min[1] = f32::NAN;
        input.clean_mesh.stable_vertex_order = false;

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(report.outcome, ExternalCharacterImportOutcome::InvalidInput);
        assert!(report.candidates.is_empty());
        assert!(report.issues.iter().any(|issue| issue.path == "bounds"));
        assert!(report.issues.iter().any(|issue| issue.path == "clean_mesh"));
    }

    #[test]
    fn ranking_is_deterministic_and_prefers_matching_feature_signature() {
        let corpus = generated_character_corpus(207);
        let input = external_input_from_character_mesh_artifact(
            "external.case.rank",
            &corpus.cases[3].mesh,
        );

        let first = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );
        let second = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(first, second);
        let best = first.candidates.first().expect("best candidate");
        assert!(best.features.hair);
        assert!(best.features.topology_edited);
    }

    #[test]
    fn handcrafted_descriptor_for_unadvertised_topology_combo_is_unsupported() {
        let features = KnownBaseCharacterMeshFeatures {
            symmetric: true,
            asymmetric: false,
            posed: false,
            clothed: false,
            hair: false,
            topology_edited: true,
        };
        let mesh = mesh_for_features("external.case.invalid_topology", features);
        let input =
            external_input_from_character_mesh_artifact("external.case.invalid_topology", &mesh);

        let report = analyze_external_clean_character_mesh(
            &input,
            &ExternalCharacterAnalysisConfig::default(),
        );

        assert_eq!(
            report.outcome,
            ExternalCharacterImportOutcome::DiagnosticOnlyUnsupported
        );
        assert!(report.candidates.is_empty());
    }

    fn with_expected_correspondences(
        mut input: ExternalCleanCharacterMeshInput,
    ) -> ExternalCleanCharacterMeshInput {
        input.correspondences = vec![
            "character.body",
            "head.cranium",
            "body.torso",
            "head.scalp",
            "body.loop.neck",
            "body.pose.joint_chain",
            "body.shoulder.left",
            "garment.torso_shell",
            "garment.neck_opening",
            "hair.mass.primary",
            "hair.card.primary",
        ]
        .into_iter()
        .enumerate()
        .map(
            |(index, target_id)| ExternalCharacterCorrespondenceObservation {
                id: format!("obs.{index}"),
                target_id: target_id.to_owned(),
                observed_count: 1,
                max_error_meters: 0.0,
            },
        )
        .collect();
        input
    }

    fn mesh_for_features(
        id: &str,
        features: KnownBaseCharacterMeshFeatures,
    ) -> CharacterMeshArtifact {
        let signature = known_base_character_signature_for_features(features);
        let descriptor = known_base_character_descriptor_for_features(features);
        let mut mesh = CharacterMeshArtifact {
            id: id.to_owned(),
            canonical_units: "meters".to_owned(),
            coordinate_system: "y_up_z_forward_right_handed".to_owned(),
            semantic_descriptor_fingerprint: descriptor.semantic_descriptor_fingerprint,
            raw_geometry_size: CharacterRawGeometrySize {
                vertex_count: signature.vertex_count,
                face_count: signature.face_count,
                position_bytes: signature.vertex_count * 3 * 4,
                topology_bytes: signature.face_count * 3 * 4,
            },
            connected_component_count: signature.connected_component_count,
            bounds_min: bounds_from_bits(signature.bounds_min_bits),
            bounds_max: bounds_from_bits(signature.bounds_max_bits),
            topology_fingerprint: descriptor.topology_fingerprint,
            canonical_position_fingerprint: descriptor.canonical_position_fingerprint,
            artifact_fingerprint: String::new(),
        };
        mesh.artifact_fingerprint = external_character_artifact_fingerprint(&mesh);
        mesh
    }
}
