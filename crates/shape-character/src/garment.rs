//! Garment grammar.
//!
//! This module defines deterministic, compact garment contracts. It is not a
//! cloth simulator: the grammar can describe shells, seams, openings, panels,
//! fold curves, and bounded fold fields, but it intentionally rejects arbitrary
//! unbounded cloth dynamics and dense cloth displacement payloads.

use crate::{
    CharacterBaseId, CharacterGrammarId, CharacterLandmarkId, CharacterLoopId, CharacterRegionId,
    ScalarRange,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

/// Garment grammar schema version understood by this crate.
pub const GARMENT_GRAMMAR_SCHEMA_VERSION: u32 = 1;

/// Versioned base schema version understood by this crate.
pub const GARMENT_BASE_SCHEMA_VERSION: u32 = 1;

/// Default garment grammar namespace.
pub const DEFAULT_GARMENT_GRAMMAR_ID: &str = "shape.character.garment.v1";

/// Explicit boundaries that this grammar does not cross.
pub const UNSUPPORTED_GARMENT_BOUNDARIES: [UnsupportedGarmentBoundary; 5] = [
    UnsupportedGarmentBoundary::ArbitraryClothSimulation,
    UnsupportedGarmentBoundary::UnboundedDynamicDrape,
    UnsupportedGarmentBoundary::SelfCollisionSolver,
    UnsupportedGarmentBoundary::MaterialStretchSolver,
    UnsupportedGarmentBoundary::DensePerVertexClothDisplacement,
];

/// Stable garment operation identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GarmentOperationId(pub String);

impl GarmentOperationId {
    /// Build a stable operation identifier from compact semantic payload parts.
    #[must_use]
    pub fn deterministic(kind: GarmentOperationKind, semantic_parts: &[String]) -> Self {
        let mut parts = vec![
            format!("schema={GARMENT_GRAMMAR_SCHEMA_VERSION}"),
            format!("kind={}", kind.as_str()),
        ];
        parts.extend_from_slice(semantic_parts);
        let digest = stable_digest(&parts);
        Self(format!(
            "garment.operation.{}.{}",
            kind.as_str(),
            &digest[..16]
        ))
    }
}

/// Stable garment curve identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GarmentCurveId(pub String);

impl GarmentCurveId {
    /// Build a stable curve identifier from compact semantic payload parts.
    #[must_use]
    pub fn deterministic(semantic_parts: &[String]) -> Self {
        let mut parts = vec![format!("schema={GARMENT_GRAMMAR_SCHEMA_VERSION}")];
        parts.extend_from_slice(semantic_parts);
        let digest = stable_digest(&parts);
        Self(format!("garment.curve.{}", &digest[..16]))
    }
}

/// Stable garment panel identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GarmentPanelId(pub String);

impl GarmentPanelId {
    /// Build a stable panel identifier from compact semantic payload parts.
    #[must_use]
    pub fn deterministic(semantic_parts: &[String]) -> Self {
        let mut parts = vec![format!("schema={GARMENT_GRAMMAR_SCHEMA_VERSION}")];
        parts.extend_from_slice(semantic_parts);
        let digest = stable_digest(&parts);
        Self(format!("garment.panel.{}", &digest[..16]))
    }
}

/// Stable bounded fold-field identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GarmentFoldFieldId(pub String);

impl GarmentFoldFieldId {
    /// Build a stable fold-field identifier from compact semantic payload parts.
    #[must_use]
    pub fn deterministic(semantic_parts: &[String]) -> Self {
        let mut parts = vec![format!("schema={GARMENT_GRAMMAR_SCHEMA_VERSION}")];
        parts.extend_from_slice(semantic_parts);
        let digest = stable_digest(&parts);
        Self(format!("garment.fold_field.{}", &digest[..16]))
    }
}

/// Stable fingerprint for a versioned garment base.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct GarmentBaseFingerprint(pub String);

/// Compact target for body-region-driven garment operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodyRegionTarget {
    /// Semantic body region to shell, offset, seam against, or fold over.
    pub region: CharacterRegionId,
    /// Side selection for symmetric characters.
    pub side: BodyRegionSide,
    /// Normalized authored coverage within this target region.
    pub coverage: ScalarRange,
}

impl BodyRegionTarget {
    /// Construct a whole-region target.
    #[must_use]
    pub fn whole(region: impl Into<String>) -> Self {
        Self {
            region: CharacterRegionId(region.into()),
            side: BodyRegionSide::Whole,
            coverage: normalized_range(1.0),
        }
    }

    /// Construct a side-specific region target.
    #[must_use]
    pub fn sided(region: impl Into<String>, side: BodyRegionSide) -> Self {
        Self {
            region: CharacterRegionId(region.into()),
            side,
            coverage: normalized_range(1.0),
        }
    }

    /// Validate finite normalized coverage and a non-empty body region id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        validate_nonempty("body region", &self.region.0)?;
        validate_normalized_range("body region coverage", self.coverage)
    }

    fn canonical_parts(&self) -> Vec<String> {
        vec![
            format!("region={}", self.region.0),
            format!("side={}", self.side.as_str()),
            format!("coverage={}", range_key(self.coverage)),
        ]
    }
}

/// Side selector for body-region targets.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum BodyRegionSide {
    /// The complete target region.
    Whole,
    /// The left side of a symmetric region.
    Left,
    /// The right side of a symmetric region.
    Right,
    /// Both sides, keeping paired semantics.
    SymmetricPair,
    /// Centerline-only region.
    Centerline,
}

impl BodyRegionSide {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Whole => "whole",
            Self::Left => "left",
            Self::Right => "right",
            Self::SymmetricPair => "symmetric_pair",
            Self::Centerline => "centerline",
        }
    }
}

/// Compact body-surface anchor for garment curves.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySurfaceAnchor {
    /// Target body region.
    pub target: BodyRegionTarget,
    /// Optional semantic body landmark used to stabilize the anchor.
    pub landmark: Option<CharacterLandmarkId>,
    /// Normalized surface coordinates in the target region.
    pub uv: [f32; 2],
}

impl BodySurfaceAnchor {
    /// Construct an anchor without a landmark.
    #[must_use]
    pub fn new(target: BodyRegionTarget, u: f32, v: f32) -> Self {
        Self {
            target,
            landmark: None,
            uv: [u, v],
        }
    }

    /// Validate finite normalized coordinates and the target.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        if let Some(landmark) = &self.landmark {
            validate_nonempty("curve landmark", &landmark.0)?;
        }
        validate_normalized_scalar("curve anchor u", self.uv[0])?;
        validate_normalized_scalar("curve anchor v", self.uv[1])
    }

    fn canonical_parts(&self) -> Vec<String> {
        let mut parts = self.target.canonical_parts();
        parts.push(format!(
            "landmark={}",
            self.landmark
                .as_ref()
                .map(|landmark| landmark.0.as_str())
                .unwrap_or("")
        ));
        parts.push(format!("u={}", scalar_key(self.uv[0])));
        parts.push(format!("v={}", scalar_key(self.uv[1])));
        parts
    }
}

/// Compact garment curve reference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GarmentCurve {
    /// Deterministic curve id.
    pub id: GarmentCurveId,
    /// Curve source.
    pub source: GarmentCurveSource,
}

impl GarmentCurve {
    /// Build a curve from an existing character loop.
    #[must_use]
    pub fn body_loop(loop_id: impl Into<String>, target: BodyRegionTarget) -> Self {
        let source = GarmentCurveSource::BodyLoop {
            loop_id: CharacterLoopId(loop_id.into()),
            target,
        };
        let id = GarmentCurveId::deterministic(&curve_source_parts(&source));
        Self { id, source }
    }

    /// Build an anchored path curve.
    #[must_use]
    pub fn anchored_path(anchors: Vec<BodySurfaceAnchor>, closed: bool) -> Self {
        let source = GarmentCurveSource::AnchoredPath { anchors, closed };
        let id = GarmentCurveId::deterministic(&curve_source_parts(&source));
        Self { id, source }
    }

    /// Validate curve compactness, deterministic id, and finite anchors.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        validate_nonempty("garment curve id", &self.id.0)?;
        match &self.source {
            GarmentCurveSource::BodyLoop { loop_id, target } => {
                validate_nonempty("body loop", &loop_id.0)?;
                target.validate()?;
            }
            GarmentCurveSource::AnchoredPath { anchors, closed } => {
                let min_anchors = if *closed { 3 } else { 2 };
                if anchors.len() < min_anchors {
                    return Err(GarmentValidationError::InsufficientCurveAnchors {
                        required: min_anchors,
                        found: anchors.len(),
                    });
                }
                for anchor in anchors {
                    anchor.validate()?;
                }
            }
        }
        let expected = GarmentCurveId::deterministic(&curve_source_parts(&self.source));
        if self.id != expected {
            return Err(GarmentValidationError::NonDeterministicId {
                field: "garment curve id",
                expected: expected.0,
                found: self.id.0.clone(),
            });
        }
        Ok(())
    }

    /// Returns true when this curve can bound a panel or opening.
    #[must_use]
    pub fn is_closed_boundary(&self) -> bool {
        match &self.source {
            GarmentCurveSource::BodyLoop { .. } => true,
            GarmentCurveSource::AnchoredPath { anchors, closed } => *closed && anchors.len() >= 3,
        }
    }

    fn canonical_parts(&self) -> Vec<String> {
        let mut parts = vec![format!("curve_id={}", self.id.0)];
        parts.extend(curve_source_parts(&self.source));
        parts
    }
}

/// Compact curve source; no dense cloth path payloads are stored here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GarmentCurveSource {
    /// Reuse a versioned character topology loop on a body region.
    BodyLoop {
        /// Loop id from the character base topology.
        loop_id: CharacterLoopId,
        /// Body target for the loop.
        target: BodyRegionTarget,
    },
    /// Sparse authored surface anchors.
    AnchoredPath {
        /// Ordered sparse anchors.
        anchors: Vec<BodySurfaceAnchor>,
        /// Whether this path forms a closed loop.
        closed: bool,
    },
}

/// Versioned garment base with a deterministic fingerprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GarmentBase {
    /// Base schema version.
    pub schema_version: u32,
    /// Grammar namespace.
    pub grammar: CharacterGrammarId,
    /// Stable deterministic base id.
    pub id: CharacterBaseId,
    /// Base source operation.
    pub source: ShellFromBodyRegionOperation,
    /// Deterministic fingerprint over the versioned source contract.
    pub fingerprint: GarmentBaseFingerprint,
}

impl GarmentBase {
    /// Build a versioned shell base from a body-region target.
    #[must_use]
    pub fn shell_from_body_region(
        target: BodyRegionTarget,
        offset: ScalarRange,
        thickness: ScalarRange,
    ) -> Self {
        let source = ShellFromBodyRegionOperation::new(target, offset, thickness);
        let grammar = CharacterGrammarId(DEFAULT_GARMENT_GRAMMAR_ID.to_owned());
        let id = CharacterBaseId(format!(
            "garment.base.shell.{}",
            &stable_digest(&garment_base_id_parts(
                GARMENT_BASE_SCHEMA_VERSION,
                &grammar,
                &source
            ))[..16]
        ));
        let mut base = Self {
            schema_version: GARMENT_BASE_SCHEMA_VERSION,
            grammar,
            id,
            source,
            fingerprint: GarmentBaseFingerprint(String::new()),
        };
        base.fingerprint = base.compute_fingerprint();
        base
    }

    /// Compute the deterministic fingerprint for this base contract.
    #[must_use]
    pub fn compute_fingerprint(&self) -> GarmentBaseFingerprint {
        let mut parts = vec![
            format!("base_schema={}", self.schema_version),
            format!("grammar={}", self.grammar.0),
            format!("base_id={}", self.id.0),
        ];
        parts.extend(self.source.canonical_parts());
        GarmentBaseFingerprint(format!(
            "garment.base.v{}.{}",
            self.schema_version,
            stable_digest(&parts)
        ))
    }

    /// Validate schema, source operation, deterministic id, and fingerprint.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        if self.schema_version != GARMENT_BASE_SCHEMA_VERSION {
            return Err(GarmentValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: GARMENT_BASE_SCHEMA_VERSION,
            });
        }
        validate_nonempty("garment grammar id", &self.grammar.0)?;
        validate_nonempty("garment base id", &self.id.0)?;
        self.source.validate()?;
        let expected_id = CharacterBaseId(format!(
            "garment.base.shell.{}",
            &stable_digest(&garment_base_id_parts(
                self.schema_version,
                &self.grammar,
                &self.source
            ))[..16]
        ));
        if self.id != expected_id {
            return Err(GarmentValidationError::NonDeterministicId {
                field: "garment base id",
                expected: expected_id.0,
                found: self.id.0.clone(),
            });
        }
        let expected_fingerprint = self.compute_fingerprint();
        if self.fingerprint != expected_fingerprint {
            return Err(GarmentValidationError::FingerprintMismatch {
                expected: expected_fingerprint.0,
                found: self.fingerprint.0.clone(),
            });
        }
        Ok(())
    }
}

/// Operation kinds supported by the garment grammar.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum GarmentOperationKind {
    /// Create a shell from a versioned body region.
    ShellFromBodyRegion,
    /// Offset a garment surface from its source body target.
    Offset,
    /// Set garment thickness.
    Thickness,
    /// Add a seam along a compact curve.
    Seam,
    /// Add an opening along a compact boundary curve.
    Opening,
    /// Add a panel bounded by a compact curve.
    Panel,
    /// Add a fold curve.
    FoldCurve,
    /// Add a fold field with explicit finite bounds.
    BoundedFoldField,
}

impl GarmentOperationKind {
    /// All supported garment operation kinds.
    pub const ALL: [Self; 8] = [
        Self::ShellFromBodyRegion,
        Self::Offset,
        Self::Thickness,
        Self::Seam,
        Self::Opening,
        Self::Panel,
        Self::FoldCurve,
        Self::BoundedFoldField,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ShellFromBodyRegion => "shell_from_body_region",
            Self::Offset => "offset",
            Self::Thickness => "thickness",
            Self::Seam => "seam",
            Self::Opening => "opening",
            Self::Panel => "panel",
            Self::FoldCurve => "fold_curve",
            Self::BoundedFoldField => "bounded_fold_field",
        }
    }
}

/// Compact garment operation contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GarmentOperation {
    /// Create a garment shell from a body region.
    ShellFromBodyRegion(ShellFromBodyRegionOperation),
    /// Offset a garment surface.
    Offset(OffsetOperation),
    /// Set garment thickness.
    Thickness(ThicknessOperation),
    /// Add a seam.
    Seam(SeamOperation),
    /// Add an opening.
    Opening(OpeningOperation),
    /// Add a panel.
    Panel(PanelOperation),
    /// Add a fold curve.
    FoldCurve(FoldCurveOperation),
    /// Add a bounded fold field.
    BoundedFoldField(BoundedFoldFieldOperation),
}

impl GarmentOperation {
    /// Construct a shell-from-body-region operation.
    #[must_use]
    pub fn shell_from_body_region(
        target: BodyRegionTarget,
        offset: ScalarRange,
        thickness: ScalarRange,
    ) -> Self {
        Self::ShellFromBodyRegion(ShellFromBodyRegionOperation::new(target, offset, thickness))
    }

    /// Construct an offset operation.
    #[must_use]
    pub fn offset(target: BodyRegionTarget, distance: ScalarRange) -> Self {
        Self::Offset(OffsetOperation::new(target, distance))
    }

    /// Construct a thickness operation.
    #[must_use]
    pub fn thickness(target: BodyRegionTarget, thickness: ScalarRange) -> Self {
        Self::Thickness(ThicknessOperation::new(target, thickness))
    }

    /// Construct a seam operation.
    #[must_use]
    pub fn seam(target: BodyRegionTarget, curve: GarmentCurve, allowance: ScalarRange) -> Self {
        Self::Seam(SeamOperation::new(
            target,
            curve,
            SeamKind::Structural,
            allowance,
        ))
    }

    /// Construct an opening operation.
    #[must_use]
    pub fn opening(
        target: BodyRegionTarget,
        boundary: GarmentCurve,
        clearance: ScalarRange,
    ) -> Self {
        Self::Opening(OpeningOperation::new(
            target,
            boundary,
            OpeningFinish::Hemmed,
            clearance,
        ))
    }

    /// Construct a panel operation.
    #[must_use]
    pub fn panel(target: BodyRegionTarget, boundary: GarmentCurve) -> Self {
        Self::Panel(PanelOperation::new(
            target,
            boundary,
            PanelRole::Main,
            signed_degrees_range(0.0),
        ))
    }

    /// Construct a fold-curve operation.
    #[must_use]
    pub fn fold_curve(
        target: BodyRegionTarget,
        curve: GarmentCurve,
        amplitude: ScalarRange,
        radius: ScalarRange,
    ) -> Self {
        Self::FoldCurve(FoldCurveOperation::new(target, curve, amplitude, radius))
    }

    /// Construct a bounded fold-field operation.
    #[must_use]
    pub fn bounded_fold_field(target: BodyRegionTarget, field: BoundedFoldField) -> Self {
        Self::BoundedFoldField(BoundedFoldFieldOperation::new(target, field))
    }

    /// Operation id.
    #[must_use]
    pub fn operation_id(&self) -> &GarmentOperationId {
        match self {
            Self::ShellFromBodyRegion(operation) => &operation.operation,
            Self::Offset(operation) => &operation.operation,
            Self::Thickness(operation) => &operation.operation,
            Self::Seam(operation) => &operation.operation,
            Self::Opening(operation) => &operation.operation,
            Self::Panel(operation) => &operation.operation,
            Self::FoldCurve(operation) => &operation.operation,
            Self::BoundedFoldField(operation) => &operation.operation,
        }
    }

    /// Operation kind.
    #[must_use]
    pub const fn kind(&self) -> GarmentOperationKind {
        match self {
            Self::ShellFromBodyRegion(_) => GarmentOperationKind::ShellFromBodyRegion,
            Self::Offset(_) => GarmentOperationKind::Offset,
            Self::Thickness(_) => GarmentOperationKind::Thickness,
            Self::Seam(_) => GarmentOperationKind::Seam,
            Self::Opening(_) => GarmentOperationKind::Opening,
            Self::Panel(_) => GarmentOperationKind::Panel,
            Self::FoldCurve(_) => GarmentOperationKind::FoldCurve,
            Self::BoundedFoldField(_) => GarmentOperationKind::BoundedFoldField,
        }
    }

    /// Body-region target for this operation.
    #[must_use]
    pub fn target(&self) -> &BodyRegionTarget {
        match self {
            Self::ShellFromBodyRegion(operation) => &operation.target,
            Self::Offset(operation) => &operation.target,
            Self::Thickness(operation) => &operation.target,
            Self::Seam(operation) => &operation.target,
            Self::Opening(operation) => &operation.target,
            Self::Panel(operation) => &operation.target,
            Self::FoldCurve(operation) => &operation.target,
            Self::BoundedFoldField(operation) => &operation.target,
        }
    }

    /// Compact operation contract for UI, migration, and validation surfaces.
    #[must_use]
    pub fn contract(&self) -> GarmentOperationContract {
        GarmentOperationContract {
            operation: self.operation_id().clone(),
            kind: self.kind(),
            target: self.target().clone(),
            scalar_ranges: self.scalar_contracts(),
            compact_payload: self.compact_payload(),
            unsupported_boundaries: unsupported_boundaries().to_vec(),
        }
    }

    /// Validate this operation.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        match self {
            Self::ShellFromBodyRegion(operation) => operation.validate(),
            Self::Offset(operation) => operation.validate(),
            Self::Thickness(operation) => operation.validate(),
            Self::Seam(operation) => operation.validate(),
            Self::Opening(operation) => operation.validate(),
            Self::Panel(operation) => operation.validate(),
            Self::FoldCurve(operation) => operation.validate(),
            Self::BoundedFoldField(operation) => operation.validate(),
        }
    }

    fn scalar_contracts(&self) -> Vec<NamedScalarRange> {
        match self {
            Self::ShellFromBodyRegion(operation) => vec![
                NamedScalarRange::new("offset", operation.offset),
                NamedScalarRange::new("thickness", operation.thickness),
            ],
            Self::Offset(operation) => vec![NamedScalarRange::new("distance", operation.distance)],
            Self::Thickness(operation) => {
                vec![NamedScalarRange::new("thickness", operation.thickness)]
            }
            Self::Seam(operation) => {
                vec![NamedScalarRange::new("allowance", operation.allowance)]
            }
            Self::Opening(operation) => {
                vec![NamedScalarRange::new("clearance", operation.clearance)]
            }
            Self::Panel(operation) => {
                vec![NamedScalarRange::new(
                    "grain_degrees",
                    operation.grain_degrees,
                )]
            }
            Self::FoldCurve(operation) => vec![
                NamedScalarRange::new("amplitude", operation.amplitude),
                NamedScalarRange::new("radius", operation.radius),
            ],
            Self::BoundedFoldField(operation) => operation.field.scalar_contracts(),
        }
    }

    fn compact_payload(&self) -> CompactGarmentPayload {
        match self {
            Self::ShellFromBodyRegion(_) => CompactGarmentPayload::ShellFromBodyRegion,
            Self::Offset(_) => CompactGarmentPayload::Offset,
            Self::Thickness(_) => CompactGarmentPayload::Thickness,
            Self::Seam(operation) => CompactGarmentPayload::Seam {
                curve: operation.curve.clone(),
                kind: operation.kind,
            },
            Self::Opening(operation) => CompactGarmentPayload::Opening {
                boundary: operation.boundary.clone(),
                finish: operation.finish,
            },
            Self::Panel(operation) => CompactGarmentPayload::Panel {
                panel: operation.panel.clone(),
                boundary: operation.boundary.clone(),
                role: operation.role,
            },
            Self::FoldCurve(operation) => CompactGarmentPayload::FoldCurve {
                curve: operation.curve.clone(),
            },
            Self::BoundedFoldField(operation) => CompactGarmentPayload::BoundedFoldField {
                field: operation.field.id.clone(),
                bounds: operation.field.bounds.kind(),
                direction: operation.field.direction,
                bounds_payload: operation.field.bounds.canonical_parts(),
            },
        }
    }
}

/// Compact operation contract emitted from a full operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GarmentOperationContract {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Operation kind.
    pub kind: GarmentOperationKind,
    /// Body-region target.
    pub target: BodyRegionTarget,
    /// Finite authored scalar ranges used by this operation.
    pub scalar_ranges: Vec<NamedScalarRange>,
    /// Small payload descriptor without dense cloth data.
    pub compact_payload: CompactGarmentPayload,
    /// Explicitly unsupported boundaries.
    pub unsupported_boundaries: Vec<UnsupportedGarmentBoundary>,
}

/// Named scalar range used in compact operation contracts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NamedScalarRange {
    /// Scalar name.
    pub name: String,
    /// Finite range and default.
    pub range: ScalarRange,
}

impl NamedScalarRange {
    #[must_use]
    pub fn new(name: impl Into<String>, range: ScalarRange) -> Self {
        Self {
            name: name.into(),
            range,
        }
    }
}

/// Dense-data-free operation payload descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CompactGarmentPayload {
    /// Shell sourced from a body region.
    ShellFromBodyRegion,
    /// Scalar offset payload.
    Offset,
    /// Scalar thickness payload.
    Thickness,
    /// Seam along a compact curve.
    Seam {
        /// Compact curve payload.
        curve: GarmentCurve,
        /// Seam kind.
        kind: SeamKind,
    },
    /// Opening along a compact boundary.
    Opening {
        /// Compact boundary curve payload.
        boundary: GarmentCurve,
        /// Opening finish.
        finish: OpeningFinish,
    },
    /// Panel bounded by a compact curve.
    Panel {
        /// Panel id.
        panel: GarmentPanelId,
        /// Compact boundary curve payload.
        boundary: GarmentCurve,
        /// Panel role.
        role: PanelRole,
    },
    /// Fold curve payload.
    FoldCurve {
        /// Compact curve payload.
        curve: GarmentCurve,
    },
    /// Bounded fold field payload.
    BoundedFoldField {
        /// Fold field id.
        field: GarmentFoldFieldId,
        /// Bounds kind.
        bounds: FoldFieldBoundsKind,
        /// Fold direction.
        direction: FoldDirection,
        /// Compact finite bounds payload.
        bounds_payload: Vec<String>,
    },
}

/// Shell operation sourced from a body region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellFromBodyRegionOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region to shell from.
    pub target: BodyRegionTarget,
    /// Signed shell offset range, in authored character units.
    pub offset: ScalarRange,
    /// Positive finite shell thickness range.
    pub thickness: ScalarRange,
}

impl ShellFromBodyRegionOperation {
    /// Construct a deterministic shell operation.
    #[must_use]
    pub fn new(target: BodyRegionTarget, offset: ScalarRange, thickness: ScalarRange) -> Self {
        let mut parts = target.canonical_parts();
        parts.push(format!("offset={}", range_key(offset)));
        parts.push(format!("thickness={}", range_key(thickness)));
        let operation =
            GarmentOperationId::deterministic(GarmentOperationKind::ShellFromBodyRegion, &parts);
        Self {
            operation,
            target,
            offset,
            thickness,
        }
    }

    /// Validate shell target, ranges, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        validate_scalar_range("shell offset", self.offset)?;
        validate_positive_range("shell thickness", self.thickness)?;
        validate_operation_id(
            "shell operation id",
            &self.operation,
            GarmentOperationKind::ShellFromBodyRegion,
            &self.canonical_parts_without_id(),
        )
    }

    fn canonical_parts(&self) -> Vec<String> {
        let mut parts = vec![format!("operation={}", self.operation.0)];
        parts.extend(self.canonical_parts_without_id());
        parts
    }

    fn canonical_parts_without_id(&self) -> Vec<String> {
        let mut parts = self.target.canonical_parts();
        parts.push(format!("offset={}", range_key(self.offset)));
        parts.push(format!("thickness={}", range_key(self.thickness)));
        parts
    }
}

/// Offset operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OffsetOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Signed finite offset distance.
    pub distance: ScalarRange,
}

impl OffsetOperation {
    /// Construct a deterministic offset operation.
    #[must_use]
    pub fn new(target: BodyRegionTarget, distance: ScalarRange) -> Self {
        let mut parts = target.canonical_parts();
        parts.push(format!("distance={}", range_key(distance)));
        let operation = GarmentOperationId::deterministic(GarmentOperationKind::Offset, &parts);
        Self {
            operation,
            target,
            distance,
        }
    }

    /// Validate target, finite range, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        validate_scalar_range("offset distance", self.distance)?;
        validate_operation_id(
            "offset operation id",
            &self.operation,
            GarmentOperationKind::Offset,
            &self.canonical_parts_without_id(),
        )
    }

    fn canonical_parts_without_id(&self) -> Vec<String> {
        let mut parts = self.target.canonical_parts();
        parts.push(format!("distance={}", range_key(self.distance)));
        parts
    }
}

/// Thickness operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThicknessOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Positive finite thickness range.
    pub thickness: ScalarRange,
}

impl ThicknessOperation {
    /// Construct a deterministic thickness operation.
    #[must_use]
    pub fn new(target: BodyRegionTarget, thickness: ScalarRange) -> Self {
        let mut parts = target.canonical_parts();
        parts.push(format!("thickness={}", range_key(thickness)));
        let operation = GarmentOperationId::deterministic(GarmentOperationKind::Thickness, &parts);
        Self {
            operation,
            target,
            thickness,
        }
    }

    /// Validate target, positive thickness range, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        validate_positive_range("garment thickness", self.thickness)?;
        validate_operation_id(
            "thickness operation id",
            &self.operation,
            GarmentOperationKind::Thickness,
            &self.canonical_parts_without_id(),
        )
    }

    fn canonical_parts_without_id(&self) -> Vec<String> {
        let mut parts = self.target.canonical_parts();
        parts.push(format!("thickness={}", range_key(self.thickness)));
        parts
    }
}

/// Seam operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeamOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Seam curve.
    pub curve: GarmentCurve,
    /// Seam kind.
    pub kind: SeamKind,
    /// Non-negative finite seam allowance.
    pub allowance: ScalarRange,
}

impl SeamOperation {
    /// Construct a deterministic seam operation.
    #[must_use]
    pub fn new(
        target: BodyRegionTarget,
        curve: GarmentCurve,
        kind: SeamKind,
        allowance: ScalarRange,
    ) -> Self {
        let parts = seam_parts(&target, &curve, kind, allowance);
        let operation = GarmentOperationId::deterministic(GarmentOperationKind::Seam, &parts);
        Self {
            operation,
            target,
            curve,
            kind,
            allowance,
        }
    }

    /// Validate target, curve, allowance, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        self.curve.validate()?;
        validate_nonnegative_range("seam allowance", self.allowance)?;
        validate_operation_id(
            "seam operation id",
            &self.operation,
            GarmentOperationKind::Seam,
            &seam_parts(&self.target, &self.curve, self.kind, self.allowance),
        )
    }
}

/// Seam semantics.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SeamKind {
    /// Structural stitched seam.
    Structural,
    /// Decorative seam without topology ownership.
    Decorative,
    /// Symmetry join seam.
    SymmetryJoin,
}

impl SeamKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Structural => "structural",
            Self::Decorative => "decorative",
            Self::SymmetryJoin => "symmetry_join",
        }
    }
}

/// Opening operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpeningOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Opening boundary curve.
    pub boundary: GarmentCurve,
    /// Boundary finish.
    pub finish: OpeningFinish,
    /// Non-negative finite clearance from the body.
    pub clearance: ScalarRange,
}

impl OpeningOperation {
    /// Construct a deterministic opening operation.
    #[must_use]
    pub fn new(
        target: BodyRegionTarget,
        boundary: GarmentCurve,
        finish: OpeningFinish,
        clearance: ScalarRange,
    ) -> Self {
        let parts = opening_parts(&target, &boundary, finish, clearance);
        let operation = GarmentOperationId::deterministic(GarmentOperationKind::Opening, &parts);
        Self {
            operation,
            target,
            boundary,
            finish,
            clearance,
        }
    }

    /// Validate target, boundary, clearance, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        self.boundary.validate()?;
        if !self.boundary.is_closed_boundary() {
            return Err(GarmentValidationError::OpenOpeningBoundary {
                operation: self.operation.0.clone(),
            });
        }
        validate_nonnegative_range("opening clearance", self.clearance)?;
        validate_operation_id(
            "opening operation id",
            &self.operation,
            GarmentOperationKind::Opening,
            &opening_parts(&self.target, &self.boundary, self.finish, self.clearance),
        )
    }
}

/// Opening edge finish.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OpeningFinish {
    /// Raw contract edge.
    Raw,
    /// Hemmed edge.
    Hemmed,
    /// Bound edge.
    Bound,
    /// Ribbed edge.
    Ribbed,
}

impl OpeningFinish {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Hemmed => "hemmed",
            Self::Bound => "bound",
            Self::Ribbed => "ribbed",
        }
    }
}

/// Panel operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PanelOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Deterministic panel id.
    pub panel: GarmentPanelId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Panel boundary curve.
    pub boundary: GarmentCurve,
    /// Panel role.
    pub role: PanelRole,
    /// Finite authored grain rotation in degrees.
    pub grain_degrees: ScalarRange,
}

impl PanelOperation {
    /// Construct a deterministic panel operation.
    #[must_use]
    pub fn new(
        target: BodyRegionTarget,
        boundary: GarmentCurve,
        role: PanelRole,
        grain_degrees: ScalarRange,
    ) -> Self {
        let panel_parts = panel_parts(&target, &boundary, role);
        let panel = GarmentPanelId::deterministic(&panel_parts);
        let mut operation_parts = panel_parts;
        operation_parts.push(format!("grain_degrees={}", range_key(grain_degrees)));
        let operation =
            GarmentOperationId::deterministic(GarmentOperationKind::Panel, &operation_parts);
        Self {
            operation,
            panel,
            target,
            boundary,
            role,
            grain_degrees,
        }
    }

    /// Validate target, boundary, panel id, grain range, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        self.boundary.validate()?;
        if !self.boundary.is_closed_boundary() {
            return Err(GarmentValidationError::OpenPanelBoundary {
                panel: self.panel.0.clone(),
            });
        }
        validate_scalar_range("panel grain degrees", self.grain_degrees)?;
        let expected_panel =
            GarmentPanelId::deterministic(&panel_parts(&self.target, &self.boundary, self.role));
        if self.panel != expected_panel {
            return Err(GarmentValidationError::NonDeterministicId {
                field: "garment panel id",
                expected: expected_panel.0,
                found: self.panel.0.clone(),
            });
        }
        let mut operation_parts = panel_parts(&self.target, &self.boundary, self.role);
        operation_parts.push(format!("grain_degrees={}", range_key(self.grain_degrees)));
        validate_operation_id(
            "panel operation id",
            &self.operation,
            GarmentOperationKind::Panel,
            &operation_parts,
        )
    }
}

/// Panel role.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PanelRole {
    /// Main visible garment panel.
    Main,
    /// Reinforcement or structural insert.
    Reinforcement,
    /// Lining panel.
    Lining,
    /// Decorative applique.
    Applique,
}

impl PanelRole {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Reinforcement => "reinforcement",
            Self::Lining => "lining",
            Self::Applique => "applique",
        }
    }
}

/// Fold-curve operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoldCurveOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Fold curve.
    pub curve: GarmentCurve,
    /// Signed finite fold amplitude.
    pub amplitude: ScalarRange,
    /// Positive finite fold radius.
    pub radius: ScalarRange,
}

impl FoldCurveOperation {
    /// Construct a deterministic fold-curve operation.
    #[must_use]
    pub fn new(
        target: BodyRegionTarget,
        curve: GarmentCurve,
        amplitude: ScalarRange,
        radius: ScalarRange,
    ) -> Self {
        let parts = fold_curve_parts(&target, &curve, amplitude, radius);
        let operation = GarmentOperationId::deterministic(GarmentOperationKind::FoldCurve, &parts);
        Self {
            operation,
            target,
            curve,
            amplitude,
            radius,
        }
    }

    /// Validate target, curve, ranges, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        self.curve.validate()?;
        validate_scalar_range("fold curve amplitude", self.amplitude)?;
        validate_positive_range("fold curve radius", self.radius)?;
        validate_operation_id(
            "fold curve operation id",
            &self.operation,
            GarmentOperationKind::FoldCurve,
            &fold_curve_parts(&self.target, &self.curve, self.amplitude, self.radius),
        )
    }
}

/// Bounded fold-field operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundedFoldFieldOperation {
    /// Deterministic operation id.
    pub operation: GarmentOperationId,
    /// Body region target.
    pub target: BodyRegionTarget,
    /// Bounded fold field.
    pub field: BoundedFoldField,
}

impl BoundedFoldFieldOperation {
    /// Construct a deterministic bounded fold-field operation.
    #[must_use]
    pub fn new(target: BodyRegionTarget, field: BoundedFoldField) -> Self {
        let parts = bounded_fold_field_operation_parts(&target, &field);
        let operation =
            GarmentOperationId::deterministic(GarmentOperationKind::BoundedFoldField, &parts);
        Self {
            operation,
            target,
            field,
        }
    }

    /// Validate target, field bounds, ranges, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.target.validate()?;
        self.field.validate()?;
        validate_operation_id(
            "bounded fold field operation id",
            &self.operation,
            GarmentOperationKind::BoundedFoldField,
            &bounded_fold_field_operation_parts(&self.target, &self.field),
        )
    }
}

/// Compact bounded fold field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundedFoldField {
    /// Deterministic field id.
    pub id: GarmentFoldFieldId,
    /// Explicit finite bounds. `Unbounded` is rejected by validation.
    pub bounds: FoldFieldBounds,
    /// Fold direction.
    pub direction: FoldDirection,
    /// Signed finite fold amplitude.
    pub amplitude: ScalarRange,
    /// Positive finite wavelength.
    pub wavelength: ScalarRange,
    /// Normalized falloff.
    pub falloff: ScalarRange,
}

impl BoundedFoldField {
    /// Construct a fold field bounded by a body region.
    #[must_use]
    pub fn body_region(
        target: BodyRegionTarget,
        direction: FoldDirection,
        amplitude: ScalarRange,
        wavelength: ScalarRange,
        falloff: ScalarRange,
    ) -> Self {
        Self::new(
            FoldFieldBounds::BodyRegion(target),
            direction,
            amplitude,
            wavelength,
            falloff,
        )
    }

    /// Construct a bounded fold field.
    #[must_use]
    pub fn new(
        bounds: FoldFieldBounds,
        direction: FoldDirection,
        amplitude: ScalarRange,
        wavelength: ScalarRange,
        falloff: ScalarRange,
    ) -> Self {
        let mut parts =
            fold_field_parts_without_id(&bounds, direction, amplitude, wavelength, falloff);
        let id = GarmentFoldFieldId::deterministic(&parts);
        parts.push(format!("field_id={}", id.0));
        Self {
            id,
            bounds,
            direction,
            amplitude,
            wavelength,
            falloff,
        }
    }

    /// Returns true when the fold field has explicit bounds.
    #[must_use]
    pub const fn is_bounded(&self) -> bool {
        !matches!(self.bounds, FoldFieldBounds::Unbounded)
    }

    /// Validate explicit bounds, finite ranges, and deterministic id.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        self.bounds.validate()?;
        validate_scalar_range("fold field amplitude", self.amplitude)?;
        validate_positive_range("fold field wavelength", self.wavelength)?;
        validate_normalized_range("fold field falloff", self.falloff)?;
        let expected = GarmentFoldFieldId::deterministic(&fold_field_parts_without_id(
            &self.bounds,
            self.direction,
            self.amplitude,
            self.wavelength,
            self.falloff,
        ));
        if self.id != expected {
            return Err(GarmentValidationError::NonDeterministicId {
                field: "garment fold field id",
                expected: expected.0,
                found: self.id.0.clone(),
            });
        }
        Ok(())
    }

    fn scalar_contracts(&self) -> Vec<NamedScalarRange> {
        let mut contracts = vec![
            NamedScalarRange::new("amplitude", self.amplitude),
            NamedScalarRange::new("wavelength", self.wavelength),
            NamedScalarRange::new("falloff", self.falloff),
        ];
        if let FoldFieldBounds::CurveBand { half_width, .. } = &self.bounds {
            contracts.push(NamedScalarRange::new("curve_band_half_width", *half_width));
        }
        contracts
    }

    fn canonical_parts(&self) -> Vec<String> {
        let mut parts = vec![format!("field_id={}", self.id.0)];
        parts.extend(fold_field_parts_without_id(
            &self.bounds,
            self.direction,
            self.amplitude,
            self.wavelength,
            self.falloff,
        ));
        parts
    }
}

/// Fold-field bounds. `Unbounded` exists only so validators can reject it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FoldFieldBounds {
    /// Bounded to a body region.
    BodyRegion(BodyRegionTarget),
    /// Bounded to a panel.
    Panel(GarmentPanelId),
    /// Bounded to a finite band around a curve.
    CurveBand {
        /// Center curve.
        center: GarmentCurve,
        /// Positive half width of the band.
        half_width: ScalarRange,
    },
    /// Explicitly unsupported unbounded fold field.
    Unbounded,
}

impl FoldFieldBounds {
    /// Bounds kind without payload.
    #[must_use]
    pub const fn kind(&self) -> FoldFieldBoundsKind {
        match self {
            Self::BodyRegion(_) => FoldFieldBoundsKind::BodyRegion,
            Self::Panel(_) => FoldFieldBoundsKind::Panel,
            Self::CurveBand { .. } => FoldFieldBoundsKind::CurveBand,
            Self::Unbounded => FoldFieldBoundsKind::Unbounded,
        }
    }

    /// Validate finite explicit bounds.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        match self {
            Self::BodyRegion(target) => target.validate(),
            Self::Panel(panel) => validate_nonempty("fold field panel", &panel.0),
            Self::CurveBand { center, half_width } => {
                center.validate()?;
                validate_positive_range("fold field curve-band half width", *half_width)
            }
            Self::Unbounded => Err(GarmentValidationError::UnboundedFoldField),
        }
    }

    fn canonical_parts(&self) -> Vec<String> {
        match self {
            Self::BodyRegion(target) => {
                let mut parts = vec!["bounds=body_region".to_owned()];
                parts.extend(target.canonical_parts());
                parts
            }
            Self::Panel(panel) => vec!["bounds=panel".to_owned(), format!("panel_id={}", panel.0)],
            Self::CurveBand { center, half_width } => {
                let mut parts = vec!["bounds=curve_band".to_owned()];
                parts.extend(center.canonical_parts());
                parts.push(format!("half_width={}", range_key(*half_width)));
                parts
            }
            Self::Unbounded => vec!["bounds=unbounded".to_owned()],
        }
    }
}

/// Fold-field bounds kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FoldFieldBoundsKind {
    /// Body-region bounds.
    BodyRegion,
    /// Panel bounds.
    Panel,
    /// Curve-band bounds.
    CurveBand,
    /// Unsupported unbounded field.
    Unbounded,
}

/// Fold direction for compact bounded fields.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FoldDirection {
    /// Along the target region's U axis.
    AlongU,
    /// Along the target region's V axis.
    AlongV,
    /// Normal to a curve.
    CurveNormal,
    /// Gravity-aligned authored direction.
    GravityAligned,
}

impl FoldDirection {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AlongU => "along_u",
            Self::AlongV => "along_v",
            Self::CurveNormal => "curve_normal",
            Self::GravityAligned => "gravity_aligned",
        }
    }
}

/// Unsupported boundaries for this grammar.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UnsupportedGarmentBoundary {
    /// Arbitrary simulation steps are out of scope for this grammar.
    ArbitraryClothSimulation,
    /// Unbounded dynamic drape fields are out of scope.
    UnboundedDynamicDrape,
    /// Self-collision solving is out of scope.
    SelfCollisionSolver,
    /// Material stretch/strain solving is out of scope.
    MaterialStretchSolver,
    /// Dense per-vertex cloth displacement payloads are out of scope.
    DensePerVertexClothDisplacement,
}

/// Returns the explicit unsupported garment boundaries.
#[must_use]
pub const fn unsupported_boundaries() -> &'static [UnsupportedGarmentBoundary] {
    &UNSUPPORTED_GARMENT_BOUNDARIES
}

/// Versioned garment grammar document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GarmentGrammarDocument {
    /// Document schema version.
    pub schema_version: u32,
    /// Grammar namespace.
    pub grammar: CharacterGrammarId,
    /// Versioned bases.
    pub bases: Vec<GarmentBase>,
    /// Compact operations.
    pub operations: Vec<GarmentOperation>,
}

impl GarmentGrammarDocument {
    /// Construct an empty garment grammar document.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            schema_version: GARMENT_GRAMMAR_SCHEMA_VERSION,
            grammar: CharacterGrammarId(DEFAULT_GARMENT_GRAMMAR_ID.to_owned()),
            bases: Vec::new(),
            operations: Vec::new(),
        }
    }

    /// Validate schema, bases, operations, and unique deterministic ids.
    pub fn validate(&self) -> GarmentValidationResult<()> {
        if self.schema_version != GARMENT_GRAMMAR_SCHEMA_VERSION {
            return Err(GarmentValidationError::UnsupportedSchemaVersion {
                found: self.schema_version,
                supported: GARMENT_GRAMMAR_SCHEMA_VERSION,
            });
        }
        validate_nonempty("garment grammar id", &self.grammar.0)?;
        let mut base_ids = BTreeSet::new();
        for base in &self.bases {
            base.validate()?;
            if base.grammar != self.grammar {
                return Err(GarmentValidationError::GrammarMismatch {
                    field: "garment base grammar",
                    expected: self.grammar.0.clone(),
                    found: base.grammar.0.clone(),
                });
            }
            if !base_ids.insert(base.id.0.clone()) {
                return Err(GarmentValidationError::DuplicateId {
                    field: "garment base id",
                    id: base.id.0.clone(),
                });
            }
        }
        let mut operation_ids = BTreeSet::new();
        let mut panel_ids = BTreeSet::new();
        let mut fold_field_ids = BTreeSet::new();
        for operation in &self.operations {
            operation.validate()?;
            let id = operation.operation_id().0.clone();
            if !operation_ids.insert(id.clone()) {
                return Err(GarmentValidationError::DuplicateId {
                    field: "garment operation id",
                    id,
                });
            }
            if let GarmentOperation::Panel(panel) = operation
                && !panel_ids.insert(panel.panel.0.clone())
            {
                return Err(GarmentValidationError::DuplicateId {
                    field: "garment panel id",
                    id: panel.panel.0.clone(),
                });
            }
            if let GarmentOperation::BoundedFoldField(fold_field) = operation
                && !fold_field_ids.insert(fold_field.field.id.0.clone())
            {
                return Err(GarmentValidationError::DuplicateId {
                    field: "garment fold field id",
                    id: fold_field.field.id.0.clone(),
                });
            }
        }
        for operation in &self.operations {
            if let GarmentOperation::BoundedFoldField(fold_field) = operation
                && let FoldFieldBounds::Panel(panel) = &fold_field.field.bounds
                && !panel_ids.contains(&panel.0)
            {
                return Err(GarmentValidationError::MissingReference {
                    field: "garment fold field panel",
                    id: panel.0.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Garment validation result.
pub type GarmentValidationResult<T> = Result<T, GarmentValidationError>;

/// Garment grammar validation errors.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum GarmentValidationError {
    /// Schema version is not supported.
    #[error("unsupported garment schema version {found}; supported version is {supported}")]
    UnsupportedSchemaVersion {
        /// Version found in the document.
        found: u32,
        /// Version supported by this crate.
        supported: u32,
    },
    /// Required id is empty.
    #[error("{field} must not be empty")]
    EmptyId {
        /// Field name.
        field: &'static str,
    },
    /// Deterministic id does not match payload.
    #[error("{field} is not deterministic: expected {expected}, found {found}")]
    NonDeterministicId {
        /// Field name.
        field: &'static str,
        /// Expected deterministic id.
        expected: String,
        /// Found id.
        found: String,
    },
    /// Nested record belongs to a different grammar namespace.
    #[error("{field} grammar mismatch: expected {expected}, found {found}")]
    GrammarMismatch {
        /// Field name.
        field: &'static str,
        /// Document grammar ID.
        expected: String,
        /// Nested record grammar ID.
        found: String,
    },
    /// Duplicate deterministic id.
    #[error("duplicate {field}: {id}")]
    DuplicateId {
        /// Field name.
        field: &'static str,
        /// Duplicate id.
        id: String,
    },
    /// Reference to an ID that is not present in the document.
    #[error("{field} references missing id {id}")]
    MissingReference {
        /// Field name.
        field: &'static str,
        /// Missing id.
        id: String,
    },
    /// Panel boundary is not closed.
    #[error("panel {panel} boundary is not closed")]
    OpenPanelBoundary {
        /// Panel id.
        panel: String,
    },
    /// Opening boundary is not closed.
    #[error("opening {operation} boundary is not closed")]
    OpenOpeningBoundary {
        /// Opening operation id.
        operation: String,
    },
    /// Base fingerprint does not match the payload.
    #[error("garment base fingerprint mismatch: expected {expected}, found {found}")]
    FingerprintMismatch {
        /// Expected fingerprint.
        expected: String,
        /// Found fingerprint.
        found: String,
    },
    /// Scalar range is invalid or non-finite.
    #[error("{field} must have finite ordered min/default/max values")]
    InvalidScalarRange {
        /// Field name.
        field: &'static str,
    },
    /// Scalar range must be non-negative.
    #[error("{field} must be non-negative")]
    NegativeScalarRange {
        /// Field name.
        field: &'static str,
    },
    /// Scalar range must be positive.
    #[error("{field} must be strictly positive")]
    NonPositiveScalarRange {
        /// Field name.
        field: &'static str,
    },
    /// Scalar must be normalized.
    #[error("{field} must be finite and normalized to 0..=1")]
    InvalidNormalizedScalar {
        /// Field name.
        field: &'static str,
    },
    /// Scalar range must be normalized.
    #[error("{field} range must be finite and contained in 0..=1")]
    InvalidNormalizedRange {
        /// Field name.
        field: &'static str,
    },
    /// Curve is underspecified.
    #[error("garment curve has {found} anchors; {required} required")]
    InsufficientCurveAnchors {
        /// Minimum required anchors.
        required: usize,
        /// Anchors found.
        found: usize,
    },
    /// Unbounded fold fields are unsupported.
    #[error("fold fields must be explicitly bounded; arbitrary cloth simulation is unsupported")]
    UnboundedFoldField,
}

/// Default shell offset range.
#[must_use]
pub const fn shell_offset_range() -> ScalarRange {
    ScalarRange {
        min: -0.05,
        max: 0.08,
        default: 0.0,
    }
}

/// Default shell thickness range.
#[must_use]
pub const fn shell_thickness_range() -> ScalarRange {
    ScalarRange {
        min: 0.001,
        max: 0.08,
        default: 0.006,
    }
}

/// Default seam allowance range.
#[must_use]
pub const fn seam_allowance_range() -> ScalarRange {
    ScalarRange {
        min: 0.0,
        max: 0.04,
        default: 0.01,
    }
}

/// Default opening clearance range.
#[must_use]
pub const fn opening_clearance_range() -> ScalarRange {
    ScalarRange {
        min: 0.0,
        max: 0.08,
        default: 0.015,
    }
}

/// Default fold amplitude range.
#[must_use]
pub const fn fold_amplitude_range() -> ScalarRange {
    ScalarRange {
        min: -0.06,
        max: 0.06,
        default: 0.01,
    }
}

/// Default fold radius range.
#[must_use]
pub const fn fold_radius_range() -> ScalarRange {
    ScalarRange {
        min: 0.001,
        max: 0.08,
        default: 0.012,
    }
}

/// Default fold wavelength range.
#[must_use]
pub const fn fold_wavelength_range() -> ScalarRange {
    ScalarRange {
        min: 0.01,
        max: 0.5,
        default: 0.08,
    }
}

/// Default fold falloff range.
#[must_use]
pub const fn fold_falloff_range() -> ScalarRange {
    ScalarRange {
        min: 0.0,
        max: 1.0,
        default: 0.6,
    }
}

/// Validate a finite scalar range with contained default.
pub fn validate_scalar_range(
    field: &'static str,
    range: ScalarRange,
) -> GarmentValidationResult<()> {
    if range.is_valid() {
        Ok(())
    } else {
        Err(GarmentValidationError::InvalidScalarRange { field })
    }
}

/// Validate a finite non-negative scalar range.
pub fn validate_nonnegative_range(
    field: &'static str,
    range: ScalarRange,
) -> GarmentValidationResult<()> {
    validate_scalar_range(field, range)?;
    if range.min >= 0.0 && range.default >= 0.0 {
        Ok(())
    } else {
        Err(GarmentValidationError::NegativeScalarRange { field })
    }
}

/// Validate a finite strictly positive scalar range.
pub fn validate_positive_range(
    field: &'static str,
    range: ScalarRange,
) -> GarmentValidationResult<()> {
    validate_scalar_range(field, range)?;
    if range.min > 0.0 && range.default > 0.0 {
        Ok(())
    } else {
        Err(GarmentValidationError::NonPositiveScalarRange { field })
    }
}

/// Validate a finite normalized scalar range.
pub fn validate_normalized_range(
    field: &'static str,
    range: ScalarRange,
) -> GarmentValidationResult<()> {
    validate_scalar_range(field, range)?;
    if range.min >= 0.0 && range.max <= 1.0 {
        Ok(())
    } else {
        Err(GarmentValidationError::InvalidNormalizedRange { field })
    }
}

fn validate_normalized_scalar(field: &'static str, value: f32) -> GarmentValidationResult<()> {
    if value.is_finite() && (0.0..=1.0).contains(&value) {
        Ok(())
    } else {
        Err(GarmentValidationError::InvalidNormalizedScalar { field })
    }
}

fn validate_nonempty(field: &'static str, value: &str) -> GarmentValidationResult<()> {
    if value.trim().is_empty() {
        Err(GarmentValidationError::EmptyId { field })
    } else {
        Ok(())
    }
}

fn validate_operation_id(
    field: &'static str,
    operation: &GarmentOperationId,
    kind: GarmentOperationKind,
    semantic_parts: &[String],
) -> GarmentValidationResult<()> {
    validate_nonempty(field, &operation.0)?;
    let expected = GarmentOperationId::deterministic(kind, semantic_parts);
    if operation == &expected {
        Ok(())
    } else {
        Err(GarmentValidationError::NonDeterministicId {
            field,
            expected: expected.0,
            found: operation.0.clone(),
        })
    }
}

fn normalized_range(default: f32) -> ScalarRange {
    ScalarRange {
        min: 0.0,
        max: 1.0,
        default,
    }
}

fn signed_degrees_range(default: f32) -> ScalarRange {
    ScalarRange {
        min: -180.0,
        max: 180.0,
        default,
    }
}

fn garment_base_id_parts(
    schema_version: u32,
    grammar: &CharacterGrammarId,
    source: &ShellFromBodyRegionOperation,
) -> Vec<String> {
    let mut parts = vec![
        format!("base_schema={schema_version}"),
        format!("grammar={}", grammar.0),
    ];
    parts.extend(source.canonical_parts_without_id());
    parts
}

fn curve_source_parts(source: &GarmentCurveSource) -> Vec<String> {
    match source {
        GarmentCurveSource::BodyLoop { loop_id, target } => {
            let mut parts = vec![
                "curve_source=body_loop".to_owned(),
                format!("loop_id={}", loop_id.0),
            ];
            parts.extend(target.canonical_parts());
            parts
        }
        GarmentCurveSource::AnchoredPath { anchors, closed } => {
            let mut parts = vec![
                "curve_source=anchored_path".to_owned(),
                format!("closed={closed}"),
                format!("anchor_count={}", anchors.len()),
            ];
            for (index, anchor) in anchors.iter().enumerate() {
                parts.push(format!("anchor_index={index}"));
                parts.extend(anchor.canonical_parts());
            }
            parts
        }
    }
}

fn seam_parts(
    target: &BodyRegionTarget,
    curve: &GarmentCurve,
    kind: SeamKind,
    allowance: ScalarRange,
) -> Vec<String> {
    let mut parts = target.canonical_parts();
    parts.extend(curve.canonical_parts());
    parts.push(format!("seam_kind={}", kind.as_str()));
    parts.push(format!("allowance={}", range_key(allowance)));
    parts
}

fn opening_parts(
    target: &BodyRegionTarget,
    boundary: &GarmentCurve,
    finish: OpeningFinish,
    clearance: ScalarRange,
) -> Vec<String> {
    let mut parts = target.canonical_parts();
    parts.extend(boundary.canonical_parts());
    parts.push(format!("opening_finish={}", finish.as_str()));
    parts.push(format!("clearance={}", range_key(clearance)));
    parts
}

fn panel_parts(target: &BodyRegionTarget, boundary: &GarmentCurve, role: PanelRole) -> Vec<String> {
    let mut parts = target.canonical_parts();
    parts.extend(boundary.canonical_parts());
    parts.push(format!("panel_role={}", role.as_str()));
    parts
}

fn fold_curve_parts(
    target: &BodyRegionTarget,
    curve: &GarmentCurve,
    amplitude: ScalarRange,
    radius: ScalarRange,
) -> Vec<String> {
    let mut parts = target.canonical_parts();
    parts.extend(curve.canonical_parts());
    parts.push(format!("amplitude={}", range_key(amplitude)));
    parts.push(format!("radius={}", range_key(radius)));
    parts
}

fn bounded_fold_field_operation_parts(
    target: &BodyRegionTarget,
    field: &BoundedFoldField,
) -> Vec<String> {
    let mut parts = target.canonical_parts();
    parts.extend(field.canonical_parts());
    parts
}

fn fold_field_parts_without_id(
    bounds: &FoldFieldBounds,
    direction: FoldDirection,
    amplitude: ScalarRange,
    wavelength: ScalarRange,
    falloff: ScalarRange,
) -> Vec<String> {
    let mut parts = bounds.canonical_parts();
    parts.push(format!("direction={}", direction.as_str()));
    parts.push(format!("amplitude={}", range_key(amplitude)));
    parts.push(format!("wavelength={}", range_key(wavelength)));
    parts.push(format!("falloff={}", range_key(falloff)));
    parts
}

fn stable_digest(parts: &[String]) -> String {
    let mut hasher = blake3::Hasher::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b";");
    }
    hasher.finalize().to_hex().to_string()
}

fn range_key(range: ScalarRange) -> String {
    format!(
        "{}..{}@{}",
        scalar_key(range.min),
        scalar_key(range.max),
        scalar_key(range.default)
    )
}

fn scalar_key(value: f32) -> String {
    let canonical = if value == 0.0 { 0.0 } else { value };
    format!("{:08x}", canonical.to_bits())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn torso_target() -> BodyRegionTarget {
        BodyRegionTarget::whole("torso")
    }

    fn sleeve_target() -> BodyRegionTarget {
        BodyRegionTarget::sided("arm", BodyRegionSide::SymmetricPair)
    }

    fn neckline_curve() -> GarmentCurve {
        GarmentCurve::body_loop("neckline", torso_target())
    }

    fn closed_panel_curve() -> GarmentCurve {
        GarmentCurve::anchored_path(
            vec![
                BodySurfaceAnchor::new(torso_target(), 0.2, 0.2),
                BodySurfaceAnchor::new(torso_target(), 0.8, 0.2),
                BodySurfaceAnchor::new(torso_target(), 0.8, 0.8),
                BodySurfaceAnchor::new(torso_target(), 0.2, 0.8),
            ],
            true,
        )
    }

    fn open_panel_curve() -> GarmentCurve {
        GarmentCurve::anchored_path(
            vec![
                BodySurfaceAnchor::new(torso_target(), 0.2, 0.2),
                BodySurfaceAnchor::new(torso_target(), 0.8, 0.2),
                BodySurfaceAnchor::new(torso_target(), 0.8, 0.8),
            ],
            false,
        )
    }

    fn base_with_grammar(mut base: GarmentBase, grammar: &str) -> GarmentBase {
        base.grammar = CharacterGrammarId(grammar.to_owned());
        base.id = CharacterBaseId(format!(
            "garment.base.shell.{}",
            &stable_digest(&garment_base_id_parts(
                base.schema_version,
                &base.grammar,
                &base.source
            ))[..16]
        ));
        base.fingerprint = base.compute_fingerprint();
        base
    }

    #[test]
    fn required_operations_exist() {
        assert_eq!(GarmentOperationKind::ALL.len(), 8);
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::ShellFromBodyRegion));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::Offset));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::Thickness));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::Seam));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::Opening));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::Panel));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::FoldCurve));
        assert!(GarmentOperationKind::ALL.contains(&GarmentOperationKind::BoundedFoldField));

        let operations = vec![
            GarmentOperation::shell_from_body_region(
                torso_target(),
                shell_offset_range(),
                shell_thickness_range(),
            ),
            GarmentOperation::offset(torso_target(), shell_offset_range()),
            GarmentOperation::thickness(torso_target(), shell_thickness_range()),
            GarmentOperation::seam(torso_target(), neckline_curve(), seam_allowance_range()),
            GarmentOperation::opening(torso_target(), neckline_curve(), opening_clearance_range()),
            GarmentOperation::panel(torso_target(), closed_panel_curve()),
            GarmentOperation::fold_curve(
                torso_target(),
                neckline_curve(),
                fold_amplitude_range(),
                fold_radius_range(),
            ),
            GarmentOperation::bounded_fold_field(
                sleeve_target(),
                BoundedFoldField::body_region(
                    sleeve_target(),
                    FoldDirection::AlongV,
                    fold_amplitude_range(),
                    fold_wavelength_range(),
                    fold_falloff_range(),
                ),
            ),
        ];
        for operation in operations {
            operation.validate().expect("operation should validate");
            let contract = operation.contract();
            assert_eq!(contract.kind, operation.kind());
            assert!(!contract.scalar_ranges.is_empty());
            assert!(
                contract
                    .unsupported_boundaries
                    .contains(&UnsupportedGarmentBoundary::ArbitraryClothSimulation)
            );
        }
    }

    #[test]
    fn bases_are_versioned_and_fingerprinted() {
        let base = GarmentBase::shell_from_body_region(
            torso_target(),
            shell_offset_range(),
            shell_thickness_range(),
        );
        assert_eq!(base.schema_version, GARMENT_BASE_SCHEMA_VERSION);
        assert_eq!(base.grammar.0, DEFAULT_GARMENT_GRAMMAR_ID);
        assert!(base.id.0.starts_with("garment.base.shell."));
        assert_eq!(base.fingerprint, base.compute_fingerprint());
        base.validate().expect("base should validate");

        let same = GarmentBase::shell_from_body_region(
            torso_target(),
            shell_offset_range(),
            shell_thickness_range(),
        );
        assert_eq!(base.id, same.id);
        assert_eq!(base.fingerprint, same.fingerprint);

        let different = GarmentBase::shell_from_body_region(
            sleeve_target(),
            shell_offset_range(),
            shell_thickness_range(),
        );
        assert_ne!(base.id, different.id);
        assert_ne!(base.fingerprint, different.fingerprint);

        let alternate_grammar = base_with_grammar(base.clone(), "shape.character.garment.alt");
        assert_ne!(base.id, alternate_grammar.id);
        alternate_grammar
            .validate()
            .expect("alternate grammar-scoped base id should validate");

        let mut stale_grammar = base.clone();
        stale_grammar.grammar = CharacterGrammarId("shape.character.garment.alt".to_owned());
        stale_grammar.fingerprint = stale_grammar.compute_fingerprint();
        assert!(matches!(
            stale_grammar.validate(),
            Err(GarmentValidationError::NonDeterministicId {
                field: "garment base id",
                ..
            })
        ));
    }

    #[test]
    fn document_validation_enforces_base_grammar_namespace() {
        let base = GarmentBase::shell_from_body_region(
            torso_target(),
            shell_offset_range(),
            shell_thickness_range(),
        );

        let mut mismatched = GarmentGrammarDocument::empty();
        mismatched.bases = vec![base_with_grammar(
            base.clone(),
            "shape.character.garment.alt",
        )];
        assert!(matches!(
            mismatched.validate(),
            Err(GarmentValidationError::GrammarMismatch {
                field: "garment base grammar",
                expected,
                found,
            }) if expected == DEFAULT_GARMENT_GRAMMAR_ID && found == "shape.character.garment.alt"
        ));

        let mut alternate = GarmentGrammarDocument::empty();
        alternate.grammar = CharacterGrammarId("shape.character.garment.alt".to_owned());
        alternate.bases = vec![base_with_grammar(base, "shape.character.garment.alt")];
        alternate
            .validate()
            .expect("matching alternate grammar base should validate");
    }

    #[test]
    fn default_ranges_are_valid() {
        let ranges = [
            shell_offset_range(),
            shell_thickness_range(),
            seam_allowance_range(),
            opening_clearance_range(),
            fold_amplitude_range(),
            fold_radius_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        ];
        for range in ranges {
            assert!(range.is_valid());
        }
        validate_positive_range("shell thickness", shell_thickness_range()).unwrap();
        validate_positive_range("fold radius", fold_radius_range()).unwrap();
        validate_positive_range("fold wavelength", fold_wavelength_range()).unwrap();
        validate_normalized_range("fold falloff", fold_falloff_range()).unwrap();
    }

    #[test]
    fn fold_fields_are_bounded() {
        let field = BoundedFoldField::body_region(
            torso_target(),
            FoldDirection::AlongU,
            fold_amplitude_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        );
        assert!(field.is_bounded());
        field.validate().expect("bounded field should validate");

        let invalid = BoundedFoldField::new(
            FoldFieldBounds::Unbounded,
            FoldDirection::GravityAligned,
            fold_amplitude_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        );
        assert!(!invalid.is_bounded());
        assert!(matches!(
            invalid.validate(),
            Err(GarmentValidationError::UnboundedFoldField)
        ));
    }

    #[test]
    fn invalid_non_finite_operations_are_rejected() {
        let operation = GarmentOperation::offset(
            torso_target(),
            ScalarRange {
                min: -0.01,
                max: f32::INFINITY,
                default: 0.0,
            },
        );
        assert!(matches!(
            operation.validate(),
            Err(GarmentValidationError::InvalidScalarRange {
                field: "offset distance"
            })
        ));
    }

    #[test]
    fn invalid_zero_thickness_operations_are_rejected() {
        let operation = GarmentOperation::thickness(
            torso_target(),
            ScalarRange {
                min: 0.0,
                max: 0.05,
                default: 0.0,
            },
        );
        assert!(matches!(
            operation.validate(),
            Err(GarmentValidationError::NonPositiveScalarRange {
                field: "garment thickness"
            })
        ));

        let shell = GarmentOperation::shell_from_body_region(
            torso_target(),
            shell_offset_range(),
            ScalarRange {
                min: 0.0,
                max: 0.05,
                default: 0.0,
            },
        );
        assert!(matches!(
            shell.validate(),
            Err(GarmentValidationError::NonPositiveScalarRange {
                field: "shell thickness"
            })
        ));
    }

    #[test]
    fn open_panel_boundaries_are_rejected() {
        let operation = GarmentOperation::panel(torso_target(), open_panel_curve());

        assert!(matches!(
            operation.validate(),
            Err(GarmentValidationError::OpenPanelBoundary { .. })
        ));
    }

    #[test]
    fn open_opening_boundaries_are_rejected() {
        let operation = GarmentOperation::opening(
            torso_target(),
            open_panel_curve(),
            opening_clearance_range(),
        );

        assert!(matches!(
            operation.validate(),
            Err(GarmentValidationError::OpenOpeningBoundary { .. })
        ));
    }

    #[test]
    fn compact_payloads_preserve_curve_sources() {
        let curve = neckline_curve();
        let seam = GarmentOperation::seam(torso_target(), curve.clone(), seam_allowance_range());
        let CompactGarmentPayload::Seam {
            curve: seam_curve, ..
        } = seam.compact_payload()
        else {
            panic!("expected seam payload");
        };
        assert_eq!(seam_curve, curve);

        let opening_boundary = closed_panel_curve();
        let opening = GarmentOperation::opening(
            torso_target(),
            opening_boundary.clone(),
            opening_clearance_range(),
        );
        let CompactGarmentPayload::Opening { boundary, .. } = opening.compact_payload() else {
            panic!("expected opening payload");
        };
        assert_eq!(boundary, opening_boundary);

        let panel_boundary = closed_panel_curve();
        let panel = GarmentOperation::panel(torso_target(), panel_boundary.clone());
        let CompactGarmentPayload::Panel { boundary, .. } = panel.compact_payload() else {
            panic!("expected panel payload");
        };
        assert_eq!(boundary, panel_boundary);

        let fold_curve = GarmentOperation::fold_curve(
            torso_target(),
            curve.clone(),
            fold_amplitude_range(),
            fold_radius_range(),
        );
        let CompactGarmentPayload::FoldCurve {
            curve: payload_curve,
        } = fold_curve.compact_payload()
        else {
            panic!("expected fold curve payload");
        };
        assert_eq!(payload_curve, curve);
    }

    #[test]
    fn document_validation_rejects_duplicate_ids_and_missing_panel_references() {
        let panel = GarmentOperation::panel(torso_target(), closed_panel_curve());
        let mut duplicate_panels = GarmentGrammarDocument::empty();
        duplicate_panels.operations = vec![
            panel.clone(),
            GarmentOperation::Panel(PanelOperation::new(
                torso_target(),
                closed_panel_curve(),
                PanelRole::Main,
                signed_degrees_range(5.0),
            )),
        ];
        assert!(matches!(
            duplicate_panels.validate(),
            Err(GarmentValidationError::DuplicateId {
                field: "garment panel id",
                ..
            })
        ));

        let missing_panel = GarmentPanelId("garment.panel.missing".to_owned());
        let field = BoundedFoldField::new(
            FoldFieldBounds::Panel(missing_panel.clone()),
            FoldDirection::CurveNormal,
            fold_amplitude_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        );
        let fold = GarmentOperation::bounded_fold_field(torso_target(), field);
        let mut missing_reference = GarmentGrammarDocument::empty();
        missing_reference.operations = vec![fold];
        assert!(matches!(
            missing_reference.validate(),
            Err(GarmentValidationError::MissingReference { field, id })
                if field == "garment fold field panel" && id == missing_panel.0
        ));

        let existing_panel = match &panel {
            GarmentOperation::Panel(panel) => panel.panel.clone(),
            _ => unreachable!("panel constructor returns a panel operation"),
        };
        let first_field = BoundedFoldField::new(
            FoldFieldBounds::Panel(existing_panel),
            FoldDirection::AlongU,
            fold_amplitude_range(),
            fold_wavelength_range(),
            fold_falloff_range(),
        );
        let first_fold = GarmentOperation::bounded_fold_field(torso_target(), first_field.clone());
        let second_fold = GarmentOperation::bounded_fold_field(sleeve_target(), first_field);
        let mut duplicate_fields = GarmentGrammarDocument::empty();
        duplicate_fields.operations = vec![panel, first_fold, second_fold];
        assert!(matches!(
            duplicate_fields.validate(),
            Err(GarmentValidationError::DuplicateId {
                field: "garment fold field id",
                ..
            })
        ));
    }

    #[test]
    fn bounded_fold_field_payload_preserves_direction_and_bounds() {
        let panel = GarmentOperation::panel(torso_target(), closed_panel_curve());
        let panel_id = match &panel {
            GarmentOperation::Panel(panel) => panel.panel.clone(),
            _ => unreachable!("panel constructor returns a panel operation"),
        };
        let operation = GarmentOperation::bounded_fold_field(
            torso_target(),
            BoundedFoldField::new(
                FoldFieldBounds::Panel(panel_id.clone()),
                FoldDirection::CurveNormal,
                fold_amplitude_range(),
                fold_wavelength_range(),
                fold_falloff_range(),
            ),
        );

        let CompactGarmentPayload::BoundedFoldField {
            bounds,
            direction,
            bounds_payload,
            ..
        } = operation.compact_payload()
        else {
            panic!("expected bounded fold field payload");
        };
        assert_eq!(bounds, FoldFieldBoundsKind::Panel);
        assert_eq!(direction, FoldDirection::CurveNormal);
        assert_eq!(
            bounds_payload,
            vec![
                "bounds=panel".to_owned(),
                format!("panel_id={}", panel_id.0)
            ]
        );
    }

    #[test]
    fn curve_band_fold_field_exposes_half_width_contract() {
        let half_width = ScalarRange {
            min: 0.01,
            max: 0.10,
            default: 0.05,
        };
        let operation = GarmentOperation::bounded_fold_field(
            torso_target(),
            BoundedFoldField::new(
                FoldFieldBounds::CurveBand {
                    center: neckline_curve(),
                    half_width,
                },
                FoldDirection::CurveNormal,
                fold_amplitude_range(),
                fold_wavelength_range(),
                fold_falloff_range(),
            ),
        );
        operation
            .validate()
            .expect("curve-band field should validate");
        let contract = operation.contract();
        assert!(
            contract.scalar_ranges.iter().any(|range| {
                range.name == "curve_band_half_width" && range.range == half_width
            })
        );
        let CompactGarmentPayload::BoundedFoldField {
            bounds,
            bounds_payload,
            ..
        } = contract.compact_payload
        else {
            panic!("expected bounded fold field payload");
        };
        assert_eq!(bounds, FoldFieldBoundsKind::CurveBand);
        assert!(
            bounds_payload
                .iter()
                .any(|part| part == &format!("half_width={}", range_key(half_width)))
        );
    }

    #[test]
    fn scalar_keys_preserve_adjacent_floats() {
        assert_ne!(
            scalar_key(1.0),
            scalar_key(f32::from_bits(1.0f32.to_bits() + 1))
        );
        assert_eq!(scalar_key(0.0), scalar_key(-0.0));
    }
}
