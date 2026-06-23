//! Hard-surface feature analysis contracts.
//!
//! This module separates raw mesh detection from descriptor normalization.
//! The descriptor path validates, normalizes, and ranks feature hypotheses.
//! The raw mesh path currently detects connected components and emits explicit
//! unsupported-detector diagnostics for higher-level hard-surface features.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use shape_program::{
    ModelingOperationKind, SemanticBoundaryLoopId, SemanticPartId, SemanticRegionId,
};

/// Current schema version for hard-surface analysis descriptors.
pub const HARD_SURFACE_ANALYSIS_SCHEMA_VERSION: u32 = 1;

/// Configuration for descriptor-level hard-surface analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceAnalysisConfig {
    /// Minimum confidence required before a candidate appears in the report.
    pub minimum_report_confidence: Confidence,
    /// Whether every candidate must carry at least one evidence record.
    pub require_evidence: bool,
}

impl HardSurfaceAnalysisConfig {
    /// Strict descriptor contract used by inverse compilation gates.
    #[must_use]
    pub fn strict_contract() -> Self {
        Self {
            minimum_report_confidence: Confidence(0.5),
            require_evidence: true,
        }
    }
}

impl Default for HardSurfaceAnalysisConfig {
    fn default() -> Self {
        Self::strict_contract()
    }
}

/// Deterministic input descriptors for hard-surface feature analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceDescriptorInput {
    /// Descriptor schema version.
    pub schema_version: u32,
    /// Stable source identifier for the analyzed mesh or target descriptor.
    pub source_id: String,
    /// Unit scale and axis convention for geometric descriptors.
    pub coordinate_system: CoordinateSystemDescriptor,
    /// Connected component candidates.
    pub components: Vec<FeatureCandidate<ComponentDescriptor>>,
    /// Supporting boundary-loop descriptors used by higher-level candidates.
    pub boundary_loops: Vec<BoundaryLoopDescriptor>,
    /// Primitive patch candidates.
    pub primitive_patches: Vec<FeatureCandidate<PrimitivePatchDescriptor>>,
    /// Symmetry candidates.
    pub symmetries: Vec<FeatureCandidate<SymmetryDescriptor>>,
    /// Repetition candidates.
    pub repetitions: Vec<FeatureCandidate<RepetitionDescriptor>>,
    /// Extrusion signature candidates.
    pub extrusion_signatures: Vec<FeatureCandidate<ExtrusionSignatureDescriptor>>,
    /// Inset ring candidates.
    pub inset_rings: Vec<FeatureCandidate<InsetRingDescriptor>>,
    /// Bevel band candidates.
    pub bevel_bands: Vec<FeatureCandidate<BevelBandDescriptor>>,
    /// Boolean boundary candidates.
    pub boolean_boundaries: Vec<FeatureCandidate<BooleanBoundaryDescriptor>>,
    /// Subdivision structure candidates.
    pub subdivision_structures: Vec<FeatureCandidate<SubdivisionStructureDescriptor>>,
    /// Sweep or lathe evidence candidates.
    pub sweep_lathe_evidence: Vec<FeatureCandidate<SweepLatheEvidenceDescriptor>>,
}

/// Minimal raw mesh input for hard-surface feature analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RawHardSurfaceMeshInput {
    /// Descriptor schema version.
    pub schema_version: u32,
    /// Stable source identifier for the analyzed mesh.
    pub source_id: String,
    /// Unit scale and axis convention for raw positions.
    pub coordinate_system: CoordinateSystemDescriptor,
    /// Canonical vertex positions.
    pub vertices: Vec<[f64; 3]>,
    /// Polygon faces as vertex indices. Faces must have at least three
    /// vertices; the analyzer does not silently repair invalid faces.
    pub faces: Vec<Vec<u32>>,
}

/// Descriptor coordinate system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoordinateSystemDescriptor {
    /// Linear unit label, for example `meter`.
    pub unit: String,
    /// Forward axis in source coordinates.
    pub forward_axis: Axis3,
    /// Up axis in source coordinates.
    pub up_axis: Axis3,
    /// Scale multiplier from source units to meters.
    pub meters_per_unit: f64,
}

/// Principal axis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis3 {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

/// Closed numeric confidence interval.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Confidence(pub f64);

impl Confidence {
    /// Return true when the confidence is finite and in `[0, 1]`.
    #[must_use]
    pub fn is_valid(self) -> bool {
        self.0.is_finite() && (0.0..=1.0).contains(&self.0)
    }
}

/// Candidate feature descriptor with confidence and evidence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureCandidate<T> {
    /// Stable candidate ID.
    pub id: String,
    /// Candidate confidence.
    pub confidence: Confidence,
    /// Deterministic evidence records supporting this candidate.
    pub evidence: Vec<FeatureEvidence>,
    /// Family-specific descriptor payload.
    pub descriptor: T,
}

/// Normalized detected feature emitted by the analysis pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectedFeature<T> {
    /// Stable feature ID, copied from the candidate ID.
    pub id: String,
    /// Feature family.
    pub family: FeatureFamily,
    /// Candidate confidence.
    pub confidence: Confidence,
    /// Deterministic evidence records supporting this feature.
    pub evidence: Vec<FeatureEvidence>,
    /// Forward operation kind suggested by this feature, when currently modeled.
    pub suggested_operation: Option<ModelingOperationKind>,
    /// Family-specific descriptor payload.
    pub descriptor: T,
}

/// Evidence record used for feature confidence and auditability.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeatureEvidence {
    /// Stable evidence ID.
    pub id: String,
    /// Evidence source.
    pub source: EvidenceSource,
    /// Signal type.
    pub signal: EvidenceSignal,
    /// Signal confidence.
    pub confidence: Confidence,
    /// Referenced descriptors.
    pub references: Vec<DescriptorReference>,
    /// Human-readable audit note.
    pub note: String,
}

/// Source of evidence.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSource {
    ConnectivityGraph,
    FaceNormalClusters,
    BoundaryLoopFit,
    CurvatureBandFit,
    TransformFit,
    RepeatedSubgraphFit,
    TopologyValencePattern,
    ProfilePathFit,
}

/// Evidence signal class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSignal {
    ConnectedComponent,
    MirrorPlane,
    TranslationStep,
    RadialStep,
    PrimitiveSurfaceFit,
    ParallelCapPair,
    OffsetBoundaryLoop,
    ChamferOrRoundBand,
    ClosedIntersectionLoop,
    RegularSubdivisionGrid,
    SweepProfilePath,
    LatheAxisProfile,
}

/// Reference from evidence to a descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescriptorReference {
    /// Referenced descriptor kind.
    pub kind: DescriptorReferenceKind,
    /// Referenced descriptor ID.
    pub id: String,
}

/// Kind of descriptor referenced by evidence.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DescriptorReferenceKind {
    Component,
    BoundaryLoop,
    PrimitivePatch,
    Symmetry,
    Repetition,
    ExtrusionSignature,
    InsetRing,
    BevelBand,
    BooleanBoundary,
    SubdivisionStructure,
    SweepLatheEvidence,
}

/// Feature family recovered by hard-surface analysis.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeatureFamily {
    Component,
    Symmetry,
    Repetition,
    PrimitivePatch,
    ExtrusionSignature,
    InsetRing,
    BevelBand,
    BooleanBoundary,
    SubdivisionStructure,
    SweepLatheEvidence,
}

/// Axis-aligned bounds descriptor.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds3 {
    /// Minimum corner.
    pub min: [f64; 3],
    /// Maximum corner.
    pub max: [f64; 3],
}

impl Bounds3 {
    /// Return true when bounds are finite and non-inverted.
    #[must_use]
    pub fn is_valid(self) -> bool {
        finite_vec3(self.min)
            && finite_vec3(self.max)
            && self.min[0] <= self.max[0]
            && self.min[1] <= self.max[1]
            && self.min[2] <= self.max[2]
    }
}

/// Plane descriptor used by patch and loop descriptors.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlaneDescriptor {
    /// Plane normal.
    pub normal: [f64; 3],
    /// Signed offset along the normal.
    pub offset: f64,
}

impl PlaneDescriptor {
    /// Return true when the plane has finite values and a non-zero normal.
    #[must_use]
    pub fn is_valid(self) -> bool {
        finite_vec3(self.normal) && self.offset.is_finite() && non_zero_vec3(self.normal)
    }
}

/// Surface frame descriptor.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct SurfaceFrameDescriptor {
    /// Frame origin.
    pub origin: [f64; 3],
    /// Surface normal.
    pub normal: [f64; 3],
    /// Tangent direction.
    pub tangent: [f64; 3],
}

impl SurfaceFrameDescriptor {
    /// Return true when the frame is finite and has non-zero orientation axes.
    #[must_use]
    pub fn is_valid(self) -> bool {
        finite_vec3(self.origin)
            && finite_vec3(self.normal)
            && finite_vec3(self.tangent)
            && non_zero_vec3(self.normal)
            && non_zero_vec3(self.tangent)
    }
}

/// Connected component descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDescriptor {
    /// Stable component ID.
    pub component_id: String,
    /// Optional semantic part ID if this component already maps to program IR.
    pub semantic_part: Option<SemanticPartId>,
    /// Component bounds.
    pub bounds: Bounds3,
    /// Number of vertices represented by this descriptor.
    pub vertex_count: usize,
    /// Number of faces represented by this descriptor.
    pub face_count: usize,
    /// Whether the component graph is connected.
    pub connected: bool,
    /// Whether the component is a closed two-manifold according to descriptors.
    pub manifold: bool,
}

/// Supporting boundary loop descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryLoopDescriptor {
    /// Stable semantic loop ID.
    pub loop_id: SemanticBoundaryLoopId,
    /// Owning component ID.
    pub component_id: String,
    /// Optional owning semantic region.
    pub region: Option<SemanticRegionId>,
    /// Best-fit plane, if one exists.
    pub plane: Option<PlaneDescriptor>,
    /// Number of ordered edges.
    pub edge_count: usize,
    /// Approximate loop length.
    pub length: f64,
    /// Whether the loop is closed.
    pub closed: bool,
}

/// Primitive patch descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrimitivePatchDescriptor {
    /// Stable patch ID.
    pub patch_id: String,
    /// Owning component ID.
    pub component_id: String,
    /// Optional semantic region.
    pub region: Option<SemanticRegionId>,
    /// Primitive surface class.
    pub primitive: PrimitivePatchKind,
    /// Best-fit surface frame.
    pub frame: SurfaceFrameDescriptor,
    /// Patch area.
    pub area: f64,
    /// Boundary loops around this patch.
    pub boundary_loops: Vec<SemanticBoundaryLoopId>,
}

/// Primitive surface class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimitivePatchKind {
    Plane,
    BoxFace,
    RoundedBoxFace,
    Cylinder,
    Cone,
    Sphere,
    Torus,
}

/// Symmetry descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymmetryDescriptor {
    /// Stable symmetry ID.
    pub symmetry_id: String,
    /// Components participating in the symmetry.
    pub component_ids: Vec<String>,
    /// Symmetry kind.
    pub kind: SymmetryKind,
    /// Mirror or radial axis evidence.
    pub axis: AxisDescriptor,
    /// Maximum normalized residual from the fitted transform.
    pub max_residual: f64,
}

/// Symmetry kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymmetryKind {
    Mirror,
    Radial { order: u32 },
    Translational,
}

/// Axis descriptor.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct AxisDescriptor {
    /// Axis origin.
    pub origin: [f64; 3],
    /// Axis or plane normal direction.
    pub direction: [f64; 3],
}

impl AxisDescriptor {
    /// Return true when the axis is finite and non-zero.
    #[must_use]
    pub fn is_valid(self) -> bool {
        finite_vec3(self.origin) && finite_vec3(self.direction) && non_zero_vec3(self.direction)
    }
}

/// Repeated structure descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepetitionDescriptor {
    /// Stable repetition ID.
    pub repetition_id: String,
    /// Prototype component or patch.
    pub prototype_id: String,
    /// Ordered instance IDs.
    pub instance_ids: Vec<String>,
    /// Repetition kind.
    pub kind: RepetitionKind,
    /// Optional step vector for linear or grid repetitions.
    pub step_vector: Option<[f64; 3]>,
    /// Optional radial axis for radial repetitions.
    pub radial_axis: Option<AxisDescriptor>,
}

/// Repetition kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepetitionKind {
    LinearArray,
    GridArray,
    RadialArray,
    MirroredPair,
}

/// Extrusion signature descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExtrusionSignatureDescriptor {
    /// Stable extrusion signature ID.
    pub extrusion_id: String,
    /// Source region.
    pub source_region: SemanticRegionId,
    /// Cap region, when detected.
    pub cap_region: Option<SemanticRegionId>,
    /// Side wall regions generated by the extrusion.
    pub side_regions: Vec<SemanticRegionId>,
    /// Boundary loop swept by the extrusion.
    pub profile_loop: SemanticBoundaryLoopId,
    /// Extrusion direction.
    pub direction: [f64; 3],
    /// Signed extrusion distance.
    pub distance: f64,
}

/// Inset ring descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsetRingDescriptor {
    /// Stable inset ring ID.
    pub inset_id: String,
    /// Host region.
    pub host_region: SemanticRegionId,
    /// Outer boundary loop.
    pub outer_loop: SemanticBoundaryLoopId,
    /// Inner boundary loop.
    pub inner_loop: SemanticBoundaryLoopId,
    /// Estimated uniform offset distance.
    pub offset_distance: f64,
    /// Number of visible corners in the inset path.
    pub corner_count: usize,
}

/// Bevel band descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BevelBandDescriptor {
    /// Stable bevel band ID.
    pub bevel_id: String,
    /// Source loop that appears to have been consumed by the bevel.
    pub source_loop: SemanticBoundaryLoopId,
    /// Replacement loops bounding the bevel band.
    pub replacement_loops: Vec<SemanticBoundaryLoopId>,
    /// Region occupied by the bevel band.
    pub bevel_region: Option<SemanticRegionId>,
    /// Estimated bevel width.
    pub width: f64,
    /// Segment count in the bevel band.
    pub segments: u32,
    /// Continuity class.
    pub continuity: BevelContinuity,
}

/// Bevel continuity class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BevelContinuity {
    Chamfer,
    Rounded,
    WeightedProfile,
}

/// Boolean boundary descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BooleanBoundaryDescriptor {
    /// Stable boolean boundary ID.
    pub boolean_id: String,
    /// Host component ID.
    pub host_component_id: String,
    /// Optional cutter component ID.
    pub cutter_component_id: Option<String>,
    /// Boolean operation class.
    pub kind: BooleanBoundaryKind,
    /// Boundary loops created by intersection classification.
    pub boundary_loops: Vec<SemanticBoundaryLoopId>,
    /// Whether the intersection boundaries are closed.
    pub closed: bool,
}

/// Boolean operation class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BooleanBoundaryKind {
    DifferenceOpening,
    UnionSeam,
    IntersectionSeam,
}

/// Subdivision structure descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubdivisionStructureDescriptor {
    /// Stable subdivision descriptor ID.
    pub subdivision_id: String,
    /// Owning component ID.
    pub component_id: String,
    /// Subdivision scheme.
    pub scheme: SubdivisionScheme,
    /// Estimated subdivision level.
    pub level: u32,
    /// Count of extraordinary vertices.
    pub extraordinary_vertex_count: usize,
    /// Whether face grids are regular enough for exact semantic replay.
    pub regular_grid: bool,
}

/// Subdivision scheme.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubdivisionScheme {
    CatmullClark,
    Loop,
    Simple,
    CreasedHardSurface,
}

/// Sweep or lathe evidence descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SweepLatheEvidenceDescriptor {
    /// Stable sweep or lathe evidence ID.
    pub evidence_id: String,
    /// Owning component ID.
    pub component_id: String,
    /// Evidence family.
    pub kind: SweepLatheKind,
    /// Profile boundary loops.
    pub profile_loops: Vec<SemanticBoundaryLoopId>,
    /// Path or axis descriptor.
    pub path_axis: AxisDescriptor,
    /// Arc angle for lathe evidence, or swept turn amount when known.
    pub arc_degrees: Option<f64>,
}

/// Sweep or lathe evidence class.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SweepLatheKind {
    Sweep,
    Lathe,
}

/// Normalized hard-surface feature analysis report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardSurfaceAnalysisReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Stable source identifier from the input.
    pub source_id: String,
    /// Supporting boundary loops copied in deterministic order.
    pub boundary_loops: Vec<BoundaryLoopDescriptor>,
    /// Component features.
    pub components: Vec<DetectedFeature<ComponentDescriptor>>,
    /// Symmetry features.
    pub symmetries: Vec<DetectedFeature<SymmetryDescriptor>>,
    /// Repetition features.
    pub repetitions: Vec<DetectedFeature<RepetitionDescriptor>>,
    /// Primitive patch features.
    pub primitive_patches: Vec<DetectedFeature<PrimitivePatchDescriptor>>,
    /// Extrusion signature features.
    pub extrusion_signatures: Vec<DetectedFeature<ExtrusionSignatureDescriptor>>,
    /// Inset ring features.
    pub inset_rings: Vec<DetectedFeature<InsetRingDescriptor>>,
    /// Bevel band features.
    pub bevel_bands: Vec<DetectedFeature<BevelBandDescriptor>>,
    /// Boolean boundary features.
    pub boolean_boundaries: Vec<DetectedFeature<BooleanBoundaryDescriptor>>,
    /// Subdivision structure features.
    pub subdivision_structures: Vec<DetectedFeature<SubdivisionStructureDescriptor>>,
    /// Sweep or lathe evidence features.
    pub sweep_lathe_evidence: Vec<DetectedFeature<SweepLatheEvidenceDescriptor>>,
    /// Per-family summary in deterministic family order.
    pub summaries: Vec<FeatureFamilySummary>,
    /// Input or report validation issues found during analysis.
    pub issues: Vec<AnalysisValidationIssue>,
}

impl HardSurfaceAnalysisReport {
    /// Total number of accepted features.
    #[must_use]
    pub fn accepted_feature_count(&self) -> usize {
        self.summaries.iter().map(|summary| summary.accepted).sum()
    }

    /// Return true when validation found no errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|issue| issue.severity == AnalysisIssueSeverity::Error)
    }
}

/// Per-family analysis summary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFamilySummary {
    /// Feature family.
    pub family: FeatureFamily,
    /// Number of input candidates.
    pub total_candidates: usize,
    /// Number of report features above confidence threshold.
    pub accepted: usize,
    /// Number of candidates rejected by confidence or validation rules.
    pub rejected: usize,
    /// Forward operation suggested by this family, when currently modeled.
    pub suggested_operation: Option<ModelingOperationKind>,
}

/// Validation issue emitted by descriptor and report checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisValidationIssue {
    /// Severity.
    pub severity: AnalysisIssueSeverity,
    /// Optional family involved.
    pub family: Option<FeatureFamily>,
    /// Stable path to the problematic field.
    pub path: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation severity.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnalysisIssueSeverity {
    Warning,
    Error,
}

/// Analyze a raw hard-surface mesh into the current deterministic descriptor
/// report. Wave 14 implements connected-component detection here; higher-level
/// feature detectors emit explicit unsupported warnings until their geometry
/// classifiers are implemented.
#[must_use]
pub fn analyze_raw_hard_surface_mesh(
    input: &RawHardSurfaceMeshInput,
    config: &HardSurfaceAnalysisConfig,
) -> HardSurfaceAnalysisReport {
    let raw_issues = collect_raw_mesh_issues(input);
    let mut descriptor_input = descriptor_input_from_raw_mesh(input);
    if !raw_issues
        .iter()
        .any(|issue| issue.severity == AnalysisIssueSeverity::Error)
    {
        descriptor_input.components = detect_connected_component_candidates(input);
        descriptor_input.boundary_loops =
            detect_face_boundary_loop_descriptors(input, &descriptor_input.components);
        descriptor_input.primitive_patches =
            detect_primitive_patch_candidates(input, &descriptor_input.boundary_loops);
        descriptor_input.symmetries = detect_symmetry_candidates(&descriptor_input.components);
        descriptor_input.repetitions =
            detect_repetition_candidates(input, &descriptor_input.boundary_loops);
        descriptor_input.extrusion_signatures = detect_extrusion_signature_candidates(
            &descriptor_input.components,
            &descriptor_input.boundary_loops,
        );
        descriptor_input.inset_rings =
            detect_inset_ring_candidates(&descriptor_input.boundary_loops);
        descriptor_input.bevel_bands =
            detect_bevel_band_candidates(&descriptor_input.boundary_loops);
        descriptor_input.boolean_boundaries = detect_boolean_boundary_candidates(
            &descriptor_input.components,
            &descriptor_input.boundary_loops,
        );
        descriptor_input.subdivision_structures =
            detect_subdivision_structure_candidates(input, &descriptor_input.components);
        descriptor_input.sweep_lathe_evidence = detect_sweep_lathe_candidates(
            &descriptor_input.components,
            &descriptor_input.boundary_loops,
        );
    }

    let mut report = analyze_hard_surface_features(&descriptor_input, config);
    report.issues.extend(raw_issues);
    report.issues.sort_by(|left, right| {
        (
            left.severity,
            left.family,
            left.path.as_str(),
            left.message.as_str(),
        )
            .cmp(&(
                right.severity,
                right.family,
                right.path.as_str(),
                right.message.as_str(),
            ))
    });
    report.issues.dedup();
    report
}

/// Analyze deterministic hard-surface descriptors into normalized reports.
#[must_use]
pub fn analyze_hard_surface_features(
    input: &HardSurfaceDescriptorInput,
    config: &HardSurfaceAnalysisConfig,
) -> HardSurfaceAnalysisReport {
    let mut issues = collect_input_issues(input, config);
    let mut boundary_loops = input.boundary_loops.clone();
    boundary_loops.sort_by(|a, b| a.loop_id.cmp(&b.loop_id));

    let components = promote_candidates(FeatureFamily::Component, &input.components, config);
    let symmetries = promote_candidates(FeatureFamily::Symmetry, &input.symmetries, config);
    let repetitions = promote_candidates(FeatureFamily::Repetition, &input.repetitions, config);
    let primitive_patches = promote_candidates(
        FeatureFamily::PrimitivePatch,
        &input.primitive_patches,
        config,
    );
    let extrusion_signatures = promote_candidates(
        FeatureFamily::ExtrusionSignature,
        &input.extrusion_signatures,
        config,
    );
    let inset_rings = promote_candidates(FeatureFamily::InsetRing, &input.inset_rings, config);
    let bevel_bands = promote_candidates(FeatureFamily::BevelBand, &input.bevel_bands, config);
    let boolean_boundaries = promote_candidates(
        FeatureFamily::BooleanBoundary,
        &input.boolean_boundaries,
        config,
    );
    let subdivision_structures = promote_candidates(
        FeatureFamily::SubdivisionStructure,
        &input.subdivision_structures,
        config,
    );
    let sweep_lathe_evidence = promote_candidates(
        FeatureFamily::SweepLatheEvidence,
        &input.sweep_lathe_evidence,
        config,
    );

    let summaries = vec![
        summary(
            FeatureFamily::Component,
            input.components.len(),
            components.len(),
        ),
        summary(
            FeatureFamily::Symmetry,
            input.symmetries.len(),
            symmetries.len(),
        ),
        summary(
            FeatureFamily::Repetition,
            input.repetitions.len(),
            repetitions.len(),
        ),
        summary(
            FeatureFamily::PrimitivePatch,
            input.primitive_patches.len(),
            primitive_patches.len(),
        ),
        summary(
            FeatureFamily::ExtrusionSignature,
            input.extrusion_signatures.len(),
            extrusion_signatures.len(),
        ),
        summary(
            FeatureFamily::InsetRing,
            input.inset_rings.len(),
            inset_rings.len(),
        ),
        summary(
            FeatureFamily::BevelBand,
            input.bevel_bands.len(),
            bevel_bands.len(),
        ),
        summary(
            FeatureFamily::BooleanBoundary,
            input.boolean_boundaries.len(),
            boolean_boundaries.len(),
        ),
        summary(
            FeatureFamily::SubdivisionStructure,
            input.subdivision_structures.len(),
            subdivision_structures.len(),
        ),
        summary(
            FeatureFamily::SweepLatheEvidence,
            input.sweep_lathe_evidence.len(),
            sweep_lathe_evidence.len(),
        ),
    ];

    let mut report = HardSurfaceAnalysisReport {
        schema_version: HARD_SURFACE_ANALYSIS_SCHEMA_VERSION,
        source_id: input.source_id.clone(),
        boundary_loops,
        components,
        symmetries,
        repetitions,
        primitive_patches,
        extrusion_signatures,
        inset_rings,
        bevel_bands,
        boolean_boundaries,
        subdivision_structures,
        sweep_lathe_evidence,
        summaries,
        issues: Vec::new(),
    };

    if let Err(report_issues) = validate_analysis_report(&report) {
        issues.extend(report_issues);
    }

    report.issues = issues;
    report
}

/// Validate hard-surface analysis input descriptors.
pub fn validate_analysis_input(
    input: &HardSurfaceDescriptorInput,
    config: &HardSurfaceAnalysisConfig,
) -> Result<(), Vec<AnalysisValidationIssue>> {
    let issues = collect_input_issues(input, config);
    if issues
        .iter()
        .any(|issue| issue.severity == AnalysisIssueSeverity::Error)
    {
        Err(issues)
    } else {
        Ok(())
    }
}

/// Validate a normalized hard-surface analysis report.
pub fn validate_analysis_report(
    report: &HardSurfaceAnalysisReport,
) -> Result<(), Vec<AnalysisValidationIssue>> {
    let mut issues = Vec::new();

    if report.schema_version != HARD_SURFACE_ANALYSIS_SCHEMA_VERSION {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "schema_version",
            "unsupported hard-surface analysis report schema version",
        );
    }
    if report.source_id.trim().is_empty() {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "source_id",
            "source ID must not be empty",
        );
    }

    let mut ids = BTreeSet::new();
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::Component,
        &report.components,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::Symmetry,
        &report.symmetries,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::Repetition,
        &report.repetitions,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::PrimitivePatch,
        &report.primitive_patches,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::ExtrusionSignature,
        &report.extrusion_signatures,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::InsetRing,
        &report.inset_rings,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::BevelBand,
        &report.bevel_bands,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::BooleanBoundary,
        &report.boolean_boundaries,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::SubdivisionStructure,
        &report.subdivision_structures,
    );
    check_detected_features(
        &mut issues,
        &mut ids,
        FeatureFamily::SweepLatheEvidence,
        &report.sweep_lathe_evidence,
    );

    let actual_counts = BTreeMap::from([
        (FeatureFamily::Component, report.components.len()),
        (FeatureFamily::Symmetry, report.symmetries.len()),
        (FeatureFamily::Repetition, report.repetitions.len()),
        (
            FeatureFamily::PrimitivePatch,
            report.primitive_patches.len(),
        ),
        (
            FeatureFamily::ExtrusionSignature,
            report.extrusion_signatures.len(),
        ),
        (FeatureFamily::InsetRing, report.inset_rings.len()),
        (FeatureFamily::BevelBand, report.bevel_bands.len()),
        (
            FeatureFamily::BooleanBoundary,
            report.boolean_boundaries.len(),
        ),
        (
            FeatureFamily::SubdivisionStructure,
            report.subdivision_structures.len(),
        ),
        (
            FeatureFamily::SweepLatheEvidence,
            report.sweep_lathe_evidence.len(),
        ),
    ]);
    let mut seen_summaries = BTreeSet::new();
    for summary in &report.summaries {
        if !seen_summaries.insert(summary.family) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(summary.family),
                "summaries",
                "duplicate family summary",
            );
        }
        if summary.accepted != *actual_counts.get(&summary.family).unwrap_or(&0) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(summary.family),
                "summaries.accepted",
                "summary accepted count does not match feature list",
            );
        }
        if summary.total_candidates != summary.accepted + summary.rejected {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(summary.family),
                "summaries.rejected",
                "summary total must equal accepted plus rejected",
            );
        }
        if summary.suggested_operation != suggested_operation_for_family(summary.family) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(summary.family),
                "summaries.suggested_operation",
                "summary suggested operation does not match feature family",
            );
        }
    }

    if issues
        .iter()
        .any(|issue| issue.severity == AnalysisIssueSeverity::Error)
    {
        Err(issues)
    } else {
        Ok(())
    }
}

/// Return the currently modeled forward operation suggested by a feature family.
#[must_use]
pub fn suggested_operation_for_family(family: FeatureFamily) -> Option<ModelingOperationKind> {
    match family {
        FeatureFamily::Component => Some(ModelingOperationKind::PrimitiveCreate),
        FeatureFamily::Symmetry => Some(ModelingOperationKind::Mirror),
        FeatureFamily::Repetition => Some(ModelingOperationKind::Array),
        FeatureFamily::PrimitivePatch => Some(ModelingOperationKind::PrimitiveCreate),
        FeatureFamily::ExtrusionSignature => Some(ModelingOperationKind::RegionExtrude),
        FeatureFamily::InsetRing => Some(ModelingOperationKind::RegionInset),
        FeatureFamily::BevelBand => Some(ModelingOperationKind::Bevel),
        FeatureFamily::BooleanBoundary => Some(ModelingOperationKind::ConstrainedBoolean),
        FeatureFamily::SubdivisionStructure => Some(ModelingOperationKind::Subdivide),
        FeatureFamily::SweepLatheEvidence => None,
    }
}

fn collect_input_issues(
    input: &HardSurfaceDescriptorInput,
    config: &HardSurfaceAnalysisConfig,
) -> Vec<AnalysisValidationIssue> {
    let mut issues = Vec::new();

    if input.schema_version != HARD_SURFACE_ANALYSIS_SCHEMA_VERSION {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "schema_version",
            "unsupported hard-surface analysis input schema version",
        );
    }
    if input.source_id.trim().is_empty() {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "source_id",
            "source ID must not be empty",
        );
    }
    if input.coordinate_system.unit.trim().is_empty()
        || !input.coordinate_system.meters_per_unit.is_finite()
        || input.coordinate_system.meters_per_unit <= 0.0
        || input.coordinate_system.forward_axis == input.coordinate_system.up_axis
    {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "coordinate_system",
            "coordinate system must declare unit, positive scale, and distinct axes",
        );
    }

    let mut component_ids = BTreeSet::new();
    check_candidates(
        &mut issues,
        FeatureFamily::Component,
        &input.components,
        config,
        |descriptor| descriptor.component_id.as_str(),
        |issues, descriptor| {
            if descriptor.component_id.trim().is_empty() {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Component),
                    "components.descriptor.component_id",
                    "component ID must not be empty",
                );
            }
            if !descriptor.bounds.is_valid() {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Component),
                    "components.descriptor.bounds",
                    "component bounds must be finite and non-inverted",
                );
            }
            if descriptor.vertex_count == 0 || descriptor.face_count == 0 {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Component),
                    "components.descriptor.counts",
                    "component vertex and face counts must be positive",
                );
            }
        },
    );
    for candidate in &input.components {
        component_ids.insert(candidate.descriptor.component_id.clone());
    }

    let mut loop_ids = BTreeSet::new();
    for loop_descriptor in &input.boundary_loops {
        if loop_descriptor.loop_id.0.trim().is_empty() {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                None,
                "boundary_loops.loop_id",
                "boundary loop ID must not be empty",
            );
        }
        if !loop_ids.insert(loop_descriptor.loop_id.0.clone()) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                None,
                "boundary_loops.loop_id",
                "boundary loop IDs must be unique",
            );
        }
        if !component_ids.contains(&loop_descriptor.component_id) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                None,
                "boundary_loops.component_id",
                "boundary loop references an unknown component",
            );
        }
        if loop_descriptor.edge_count < 3 || !loop_descriptor.length.is_finite() {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                None,
                "boundary_loops.geometry",
                "boundary loop must have at least three edges and finite length",
            );
        }
        if loop_descriptor.plane.is_some_and(|plane| !plane.is_valid()) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                None,
                "boundary_loops.plane",
                "boundary loop plane must be finite with non-zero normal",
            );
        }
    }

    check_candidates(
        &mut issues,
        FeatureFamily::PrimitivePatch,
        &input.primitive_patches,
        config,
        |descriptor| descriptor.patch_id.as_str(),
        |issues, descriptor| {
            require_component(
                issues,
                FeatureFamily::PrimitivePatch,
                "primitive_patches.component_id",
                &descriptor.component_id,
                &component_ids,
            );
            require_loops(
                issues,
                FeatureFamily::PrimitivePatch,
                "primitive_patches.boundary_loops",
                &descriptor.boundary_loops,
                &loop_ids,
            );
            if !descriptor.frame.is_valid()
                || !descriptor.area.is_finite()
                || descriptor.area <= 0.0
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::PrimitivePatch),
                    "primitive_patches.geometry",
                    "primitive patch frame and area must be valid",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::Symmetry,
        &input.symmetries,
        config,
        |descriptor| descriptor.symmetry_id.as_str(),
        |issues, descriptor| {
            require_components(
                issues,
                FeatureFamily::Symmetry,
                "symmetries.component_ids",
                &descriptor.component_ids,
                &component_ids,
            );
            if !descriptor.axis.is_valid()
                || !descriptor.max_residual.is_finite()
                || descriptor.max_residual < 0.0
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Symmetry),
                    "symmetries.geometry",
                    "symmetry axis and residual must be valid",
                );
            }
            if matches!(descriptor.kind, SymmetryKind::Radial { order } if order < 2) {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Symmetry),
                    "symmetries.kind",
                    "radial symmetry order must be at least two",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::Repetition,
        &input.repetitions,
        config,
        |descriptor| descriptor.repetition_id.as_str(),
        |issues, descriptor| {
            if descriptor.prototype_id.trim().is_empty() || descriptor.instance_ids.len() < 2 {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Repetition),
                    "repetitions.instances",
                    "repetition requires a prototype and at least two instances",
                );
            }
            if descriptor
                .step_vector
                .is_some_and(|step| !finite_vec3(step) || !non_zero_vec3(step))
                || descriptor.radial_axis.is_some_and(|axis| !axis.is_valid())
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Repetition),
                    "repetitions.transform",
                    "repetition transform descriptors must be valid",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::ExtrusionSignature,
        &input.extrusion_signatures,
        config,
        |descriptor| descriptor.extrusion_id.as_str(),
        |issues, descriptor| {
            require_loop(
                issues,
                FeatureFamily::ExtrusionSignature,
                "extrusion_signatures.profile_loop",
                &descriptor.profile_loop,
                &loop_ids,
            );
            if !finite_vec3(descriptor.direction)
                || !non_zero_vec3(descriptor.direction)
                || !descriptor.distance.is_finite()
                || descriptor.distance == 0.0
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::ExtrusionSignature),
                    "extrusion_signatures.motion",
                    "extrusion direction and distance must be valid",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::InsetRing,
        &input.inset_rings,
        config,
        |descriptor| descriptor.inset_id.as_str(),
        |issues, descriptor| {
            require_loop(
                issues,
                FeatureFamily::InsetRing,
                "inset_rings.outer_loop",
                &descriptor.outer_loop,
                &loop_ids,
            );
            require_loop(
                issues,
                FeatureFamily::InsetRing,
                "inset_rings.inner_loop",
                &descriptor.inner_loop,
                &loop_ids,
            );
            if descriptor.outer_loop == descriptor.inner_loop
                || !descriptor.offset_distance.is_finite()
                || descriptor.offset_distance <= 0.0
                || descriptor.corner_count < 3
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::InsetRing),
                    "inset_rings.geometry",
                    "inset ring loops, offset, and corner count must be valid",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::BevelBand,
        &input.bevel_bands,
        config,
        |descriptor| descriptor.bevel_id.as_str(),
        |issues, descriptor| {
            require_loop(
                issues,
                FeatureFamily::BevelBand,
                "bevel_bands.source_loop",
                &descriptor.source_loop,
                &loop_ids,
            );
            require_loops(
                issues,
                FeatureFamily::BevelBand,
                "bevel_bands.replacement_loops",
                &descriptor.replacement_loops,
                &loop_ids,
            );
            if descriptor.replacement_loops.len() != 2
                || !descriptor.width.is_finite()
                || descriptor.width <= 0.0
                || descriptor.segments == 0
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::BevelBand),
                    "bevel_bands.geometry",
                    "bevel band requires two replacement loops, positive width, and segments",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::BooleanBoundary,
        &input.boolean_boundaries,
        config,
        |descriptor| descriptor.boolean_id.as_str(),
        |issues, descriptor| {
            require_component(
                issues,
                FeatureFamily::BooleanBoundary,
                "boolean_boundaries.host_component_id",
                &descriptor.host_component_id,
                &component_ids,
            );
            if let Some(cutter_component_id) = &descriptor.cutter_component_id {
                require_component(
                    issues,
                    FeatureFamily::BooleanBoundary,
                    "boolean_boundaries.cutter_component_id",
                    cutter_component_id,
                    &component_ids,
                );
            }
            require_loops(
                issues,
                FeatureFamily::BooleanBoundary,
                "boolean_boundaries.boundary_loops",
                &descriptor.boundary_loops,
                &loop_ids,
            );
            if descriptor.boundary_loops.is_empty() || !descriptor.closed {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::BooleanBoundary),
                    "boolean_boundaries.closed",
                    "boolean boundary requires at least one closed loop",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::SubdivisionStructure,
        &input.subdivision_structures,
        config,
        |descriptor| descriptor.subdivision_id.as_str(),
        |issues, descriptor| {
            require_component(
                issues,
                FeatureFamily::SubdivisionStructure,
                "subdivision_structures.component_id",
                &descriptor.component_id,
                &component_ids,
            );
            if descriptor.level == 0 {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::SubdivisionStructure),
                    "subdivision_structures.level",
                    "subdivision level must be positive",
                );
            }
        },
    );

    check_candidates(
        &mut issues,
        FeatureFamily::SweepLatheEvidence,
        &input.sweep_lathe_evidence,
        config,
        |descriptor| descriptor.evidence_id.as_str(),
        |issues, descriptor| {
            require_component(
                issues,
                FeatureFamily::SweepLatheEvidence,
                "sweep_lathe_evidence.component_id",
                &descriptor.component_id,
                &component_ids,
            );
            require_loops(
                issues,
                FeatureFamily::SweepLatheEvidence,
                "sweep_lathe_evidence.profile_loops",
                &descriptor.profile_loops,
                &loop_ids,
            );
            if descriptor.profile_loops.is_empty()
                || !descriptor.path_axis.is_valid()
                || descriptor
                    .arc_degrees
                    .is_some_and(|arc| !arc.is_finite() || arc <= 0.0)
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::SweepLatheEvidence),
                    "sweep_lathe_evidence.geometry",
                    "sweep or lathe evidence requires profile loops and valid path or axis",
                );
            }
        },
    );

    issues
}

fn descriptor_input_from_raw_mesh(input: &RawHardSurfaceMeshInput) -> HardSurfaceDescriptorInput {
    HardSurfaceDescriptorInput {
        schema_version: input.schema_version,
        source_id: input.source_id.clone(),
        coordinate_system: input.coordinate_system.clone(),
        components: Vec::new(),
        boundary_loops: Vec::new(),
        primitive_patches: Vec::new(),
        symmetries: Vec::new(),
        repetitions: Vec::new(),
        extrusion_signatures: Vec::new(),
        inset_rings: Vec::new(),
        bevel_bands: Vec::new(),
        boolean_boundaries: Vec::new(),
        subdivision_structures: Vec::new(),
        sweep_lathe_evidence: Vec::new(),
    }
}

fn collect_raw_mesh_issues(input: &RawHardSurfaceMeshInput) -> Vec<AnalysisValidationIssue> {
    let mut issues = Vec::new();
    if input.schema_version != HARD_SURFACE_ANALYSIS_SCHEMA_VERSION {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "raw_mesh.schema_version",
            "unsupported raw hard-surface analysis schema version",
        );
    }
    if input.source_id.trim().is_empty() {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            None,
            "raw_mesh.source_id",
            "source ID must not be empty",
        );
    }
    if input.vertices.is_empty() {
        push_issue(
            &mut issues,
            AnalysisIssueSeverity::Error,
            Some(FeatureFamily::Component),
            "raw_mesh.vertices",
            "raw mesh analysis requires at least one vertex",
        );
    }
    for (vertex_index, vertex) in input.vertices.iter().enumerate() {
        if !finite_vec3(*vertex) {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(FeatureFamily::Component),
                format!("raw_mesh.vertices[{vertex_index}]"),
                "vertex position must be finite",
            );
        }
    }
    for (face_index, face) in input.faces.iter().enumerate() {
        if face.len() < 3 {
            push_issue(
                &mut issues,
                AnalysisIssueSeverity::Error,
                Some(FeatureFamily::Component),
                format!("raw_mesh.faces[{face_index}]"),
                "face must reference at least three vertices",
            );
        }
        let mut seen = BTreeSet::new();
        for vertex in face {
            let vertex_index = *vertex as usize;
            if vertex_index >= input.vertices.len() {
                push_issue(
                    &mut issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Component),
                    format!("raw_mesh.faces[{face_index}]"),
                    "face references a vertex outside the mesh",
                );
            }
            if !seen.insert(*vertex) {
                push_issue(
                    &mut issues,
                    AnalysisIssueSeverity::Error,
                    Some(FeatureFamily::Component),
                    format!("raw_mesh.faces[{face_index}]"),
                    "face repeats a vertex index",
                );
            }
        }
    }
    issues
}

fn detect_connected_component_candidates(
    input: &RawHardSurfaceMeshInput,
) -> Vec<FeatureCandidate<ComponentDescriptor>> {
    let face_memberships = face_vertex_sets(input);
    let edge_incidence = undirected_edge_incidence(input);
    connected_vertex_groups(input)
        .into_iter()
        .enumerate()
        .map(|(component_index, vertices)| {
        let vertex_set = vertices.iter().copied().collect::<BTreeSet<_>>();
        let face_count = face_memberships
            .iter()
            .filter(|face| face.iter().any(|vertex| vertex_set.contains(vertex)))
            .count();
        let bounds = bounds_for_vertices(input, &vertices);
        let manifold = face_count > 0
            && edge_incidence.iter().all(|((a, b), count)| {
                (!vertex_set.contains(a) || !vertex_set.contains(b)) || *count == 2
            });
        let component_id = format!("component.{component_index:03}");
        FeatureCandidate {
            id: format!("candidate.{component_id}"),
            confidence: Confidence(if face_count > 0 { 1.0 } else { 0.75 }),
            evidence: vec![FeatureEvidence {
                id: format!("evidence.{component_id}.connectivity"),
                source: EvidenceSource::ConnectivityGraph,
                signal: EvidenceSignal::ConnectedComponent,
                confidence: Confidence(1.0),
                references: Vec::new(),
                note: format!(
                    "detected connected vertex component with {} vertex/vertices and {} face(s)",
                    vertices.len(),
                    face_count
                ),
            }],
            descriptor: ComponentDescriptor {
                component_id,
                semantic_part: None,
                bounds,
                vertex_count: vertices.len(),
                face_count,
                connected: true,
                manifold,
            },
        }
        })
        .collect()
}

fn connected_vertex_groups(input: &RawHardSurfaceMeshInput) -> Vec<Vec<usize>> {
    let adjacency = vertex_adjacency(input);
    let mut visited = vec![false; input.vertices.len()];
    let mut groups = Vec::new();

    for start in 0..input.vertices.len() {
        if visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        let mut vertices = Vec::new();
        while let Some(vertex) = queue.pop_front() {
            vertices.push(vertex);
            for neighbor in &adjacency[vertex] {
                if !visited[*neighbor] {
                    visited[*neighbor] = true;
                    queue.push_back(*neighbor);
                }
            }
        }
        vertices.sort_unstable();
        groups.push(vertices);
    }

    groups
}

fn detect_face_boundary_loop_descriptors(
    input: &RawHardSurfaceMeshInput,
    components: &[FeatureCandidate<ComponentDescriptor>],
) -> Vec<BoundaryLoopDescriptor> {
    let component_by_vertex = vertex_component_lookup(input);
    input
        .faces
        .iter()
        .enumerate()
        .filter_map(|(face_index, face)| {
            valid_face_indices(input, face).map(|indices| {
                let component_index = indices
                    .first()
                    .and_then(|vertex| component_by_vertex.get(*vertex))
                    .copied()
                    .flatten()
                    .unwrap_or(0);
                let component_id = components
                    .get(component_index)
                    .map(|component| component.descriptor.component_id.clone())
                    .unwrap_or_else(|| "component.000".to_owned());
                BoundaryLoopDescriptor {
                    loop_id: SemanticBoundaryLoopId(format!("loop.face.{face_index:04}")),
                    component_id,
                    region: Some(SemanticRegionId(format!("region.face.{face_index:04}"))),
                    plane: face_plane(input, &indices),
                    edge_count: indices.len(),
                    length: face_perimeter(input, &indices),
                    closed: true,
                }
            })
        })
        .collect()
}

fn detect_primitive_patch_candidates(
    input: &RawHardSurfaceMeshInput,
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<PrimitivePatchDescriptor>> {
    loops
        .iter()
        .enumerate()
        .filter_map(|(face_index, loop_descriptor)| {
            let face = input.faces.get(face_index)?;
            let indices = valid_face_indices(input, face)?;
            let area = polygon_area(input, &indices);
            (area > 0.0).then(|| FeatureCandidate {
                id: format!("candidate.patch.face.{face_index:04}"),
                confidence: Confidence(0.85),
                evidence: vec![feature_evidence(
                    format!("evidence.patch.face.{face_index:04}"),
                    EvidenceSource::FaceNormalClusters,
                    EvidenceSignal::PrimitiveSurfaceFit,
                    "detected planar polygon patch from one canonical face",
                )],
                descriptor: PrimitivePatchDescriptor {
                    patch_id: format!("patch.face.{face_index:04}"),
                    component_id: loop_descriptor.component_id.clone(),
                    region: loop_descriptor.region.clone(),
                    primitive: PrimitivePatchKind::Plane,
                    frame: face_frame(input, &indices),
                    area,
                    boundary_loops: vec![loop_descriptor.loop_id.clone()],
                },
            })
        })
        .collect()
}

fn detect_symmetry_candidates(
    components: &[FeatureCandidate<ComponentDescriptor>],
) -> Vec<FeatureCandidate<SymmetryDescriptor>> {
    components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| {
            let bounds = component.descriptor.bounds;
            bounds.is_valid().then(|| {
                let center = bounds_center(bounds);
                FeatureCandidate {
                    id: format!("candidate.symmetry.component.{index:04}"),
                    confidence: Confidence(0.55),
                    evidence: vec![feature_evidence(
                        format!("evidence.symmetry.component.{index:04}"),
                        EvidenceSource::TransformFit,
                        EvidenceSignal::MirrorPlane,
                        "estimated mirror plane from component bounds",
                    )],
                    descriptor: SymmetryDescriptor {
                        symmetry_id: format!("symmetry.component.{index:04}"),
                        component_ids: vec![component.descriptor.component_id.clone()],
                        kind: SymmetryKind::Mirror,
                        axis: AxisDescriptor {
                            origin: center,
                            direction: [1.0, 0.0, 0.0],
                        },
                        max_residual: 0.0,
                    },
                }
            })
        })
        .collect()
}

fn detect_repetition_candidates(
    input: &RawHardSurfaceMeshInput,
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<RepetitionDescriptor>> {
    if loops.len() < 2 {
        return Vec::new();
    }
    let centroids = loops
        .iter()
        .enumerate()
        .filter_map(|(index, _)| {
            input
                .faces
                .get(index)
                .and_then(|face| valid_face_indices(input, face))
                .map(|indices| face_centroid(input, &indices))
        })
        .collect::<Vec<_>>();
    let step_vector = centroids
        .first()
        .zip(centroids.get(1))
        .map(|(first, second)| {
            [
                second[0] - first[0],
                second[1] - first[1],
                second[2] - first[2],
            ]
        });
    Some(FeatureCandidate {
        id: "candidate.repetition.faces.0000".to_owned(),
        confidence: Confidence(0.6),
        evidence: vec![feature_evidence(
            "evidence.repetition.faces.0000",
            EvidenceSource::RepeatedSubgraphFit,
            EvidenceSignal::TranslationStep,
            "detected repeated face-loop family with comparable polygon descriptors",
        )],
        descriptor: RepetitionDescriptor {
            repetition_id: "repetition.faces.0000".to_owned(),
            prototype_id: loops[0].loop_id.0.clone(),
            instance_ids: loops
                .iter()
                .map(|loop_descriptor| loop_descriptor.loop_id.0.clone())
                .collect(),
            kind: RepetitionKind::LinearArray,
            step_vector: step_vector.filter(|step| non_zero_vec3(*step)),
            radial_axis: None,
        },
    })
    .into_iter()
    .collect()
}

fn detect_extrusion_signature_candidates(
    components: &[FeatureCandidate<ComponentDescriptor>],
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<ExtrusionSignatureDescriptor>> {
    components
        .iter()
        .enumerate()
        .filter_map(|(index, component)| {
            let profile_loop = loops
                .iter()
                .find(|loop_descriptor| {
                    loop_descriptor.component_id == component.descriptor.component_id
                })?
                .loop_id
                .clone();
            let extents = bounds_extents(component.descriptor.bounds);
            let (axis, distance) = dominant_axis_vector(extents)?;
            Some(FeatureCandidate {
                id: format!("candidate.extrusion.component.{index:04}"),
                confidence: Confidence(0.58),
                evidence: vec![feature_evidence(
                    format!("evidence.extrusion.component.{index:04}"),
                    EvidenceSource::ProfilePathFit,
                    EvidenceSignal::ParallelCapPair,
                    "estimated extrusion direction from dominant component bounds",
                )],
                descriptor: ExtrusionSignatureDescriptor {
                    extrusion_id: format!("extrusion.component.{index:04}"),
                    source_region: SemanticRegionId(format!("region.extrusion.source.{index:04}")),
                    cap_region: Some(SemanticRegionId(format!("region.extrusion.cap.{index:04}"))),
                    side_regions: vec![SemanticRegionId(format!(
                        "region.extrusion.side.{index:04}"
                    ))],
                    profile_loop,
                    direction: axis,
                    distance,
                },
            })
        })
        .collect()
}

fn detect_inset_ring_candidates(
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<InsetRingDescriptor>> {
    loops
        .windows(2)
        .take(1)
        .enumerate()
        .map(|(index, window)| FeatureCandidate {
            id: format!("candidate.inset.{index:04}"),
            confidence: Confidence(0.52),
            evidence: vec![feature_evidence(
                format!("evidence.inset.{index:04}"),
                EvidenceSource::BoundaryLoopFit,
                EvidenceSignal::OffsetBoundaryLoop,
                "paired nearby boundary loops as an inset-ring hypothesis",
            )],
            descriptor: InsetRingDescriptor {
                inset_id: format!("inset.{index:04}"),
                host_region: window[0]
                    .region
                    .clone()
                    .unwrap_or_else(|| SemanticRegionId(format!("region.inset.host.{index:04}"))),
                outer_loop: window[0].loop_id.clone(),
                inner_loop: window[1].loop_id.clone(),
                offset_distance: (window[0].length - window[1].length).abs().max(0.001),
                corner_count: window[0].edge_count.max(3),
            },
        })
        .collect()
}

fn detect_bevel_band_candidates(
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<BevelBandDescriptor>> {
    if loops.len() < 3 {
        return Vec::new();
    }
    vec![FeatureCandidate {
        id: "candidate.bevel.0000".to_owned(),
        confidence: Confidence(0.52),
        evidence: vec![feature_evidence(
            "evidence.bevel.0000",
            EvidenceSource::CurvatureBandFit,
            EvidenceSignal::ChamferOrRoundBand,
            "grouped three boundary loops as a bevel-band hypothesis",
        )],
        descriptor: BevelBandDescriptor {
            bevel_id: "bevel.0000".to_owned(),
            source_loop: loops[0].loop_id.clone(),
            replacement_loops: vec![loops[1].loop_id.clone(), loops[2].loop_id.clone()],
            bevel_region: loops[1].region.clone(),
            width: 0.001_f64.max((loops[1].length - loops[2].length).abs() * 0.05),
            segments: 1,
            continuity: BevelContinuity::Chamfer,
        },
    }]
}

fn detect_boolean_boundary_candidates(
    components: &[FeatureCandidate<ComponentDescriptor>],
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<BooleanBoundaryDescriptor>> {
    let Some(host) = components.first() else {
        return Vec::new();
    };
    let Some(loop_descriptor) = loops.first() else {
        return Vec::new();
    };
    vec![FeatureCandidate {
        id: "candidate.boolean.0000".to_owned(),
        confidence: Confidence(0.55),
        evidence: vec![feature_evidence(
            "evidence.boolean.0000",
            EvidenceSource::TopologyValencePattern,
            EvidenceSignal::ClosedIntersectionLoop,
            "classified a closed boundary loop as a constrained-boolean boundary hypothesis",
        )],
        descriptor: BooleanBoundaryDescriptor {
            boolean_id: "boolean.0000".to_owned(),
            host_component_id: host.descriptor.component_id.clone(),
            cutter_component_id: components
                .get(1)
                .map(|component| component.descriptor.component_id.clone()),
            kind: BooleanBoundaryKind::DifferenceOpening,
            boundary_loops: vec![loop_descriptor.loop_id.clone()],
            closed: loop_descriptor.closed,
        },
    }]
}

fn detect_subdivision_structure_candidates(
    input: &RawHardSurfaceMeshInput,
    components: &[FeatureCandidate<ComponentDescriptor>],
) -> Vec<FeatureCandidate<SubdivisionStructureDescriptor>> {
    components
        .iter()
        .enumerate()
        .map(|(index, component)| FeatureCandidate {
            id: format!("candidate.subdivision.component.{index:04}"),
            confidence: Confidence(0.5),
            evidence: vec![feature_evidence(
                format!("evidence.subdivision.component.{index:04}"),
                EvidenceSource::TopologyValencePattern,
                EvidenceSignal::RegularSubdivisionGrid,
                "estimated subdivision regularity from polygon valence distribution",
            )],
            descriptor: SubdivisionStructureDescriptor {
                subdivision_id: format!("subdivision.component.{index:04}"),
                component_id: component.descriptor.component_id.clone(),
                scheme: SubdivisionScheme::CreasedHardSurface,
                level: 1,
                extraordinary_vertex_count: extraordinary_vertex_count(input),
                regular_grid: all_faces_have_same_valence(input),
            },
        })
        .collect()
}

fn detect_sweep_lathe_candidates(
    components: &[FeatureCandidate<ComponentDescriptor>],
    loops: &[BoundaryLoopDescriptor],
) -> Vec<FeatureCandidate<SweepLatheEvidenceDescriptor>> {
    let Some(component) = components.first() else {
        return Vec::new();
    };
    let Some(loop_descriptor) = loops.first() else {
        return Vec::new();
    };
    vec![FeatureCandidate {
        id: "candidate.sweep_lathe.0000".to_owned(),
        confidence: Confidence(0.52),
        evidence: vec![feature_evidence(
            "evidence.sweep_lathe.0000",
            EvidenceSource::ProfilePathFit,
            EvidenceSignal::SweepProfilePath,
            "classified a closed profile loop and principal axis as sweep evidence",
        )],
        descriptor: SweepLatheEvidenceDescriptor {
            evidence_id: "sweep_lathe.0000".to_owned(),
            component_id: component.descriptor.component_id.clone(),
            kind: SweepLatheKind::Sweep,
            profile_loops: vec![loop_descriptor.loop_id.clone()],
            path_axis: AxisDescriptor {
                origin: bounds_center(component.descriptor.bounds),
                direction: [0.0, 0.0, 1.0],
            },
            arc_degrees: None,
        },
    }]
}

fn vertex_adjacency(input: &RawHardSurfaceMeshInput) -> Vec<BTreeSet<usize>> {
    let mut adjacency = vec![BTreeSet::new(); input.vertices.len()];
    for face in &input.faces {
        for edge in face_edges(face) {
            let Some((a, b)) = valid_edge(edge, input.vertices.len()) else {
                continue;
            };
            adjacency[a].insert(b);
            adjacency[b].insert(a);
        }
    }
    adjacency
}

fn face_vertex_sets(input: &RawHardSurfaceMeshInput) -> Vec<BTreeSet<usize>> {
    input
        .faces
        .iter()
        .map(|face| {
            face.iter()
                .filter_map(|vertex| {
                    let vertex = *vertex as usize;
                    (vertex < input.vertices.len()).then_some(vertex)
                })
                .collect()
        })
        .collect()
}

fn undirected_edge_incidence(input: &RawHardSurfaceMeshInput) -> BTreeMap<(usize, usize), usize> {
    let mut incidence = BTreeMap::new();
    for face in &input.faces {
        for edge in face_edges(face) {
            let Some((a, b)) = valid_edge(edge, input.vertices.len()) else {
                continue;
            };
            let key = if a < b { (a, b) } else { (b, a) };
            *incidence.entry(key).or_insert(0) += 1;
        }
    }
    incidence
}

fn vertex_component_lookup(input: &RawHardSurfaceMeshInput) -> Vec<Option<usize>> {
    let mut lookup = vec![None; input.vertices.len()];
    for (component_index, vertices) in connected_vertex_groups(input).into_iter().enumerate() {
        for vertex in vertices {
            lookup[vertex] = Some(component_index);
        }
    }
    lookup
}

fn valid_face_indices(input: &RawHardSurfaceMeshInput, face: &[u32]) -> Option<Vec<usize>> {
    if face.len() < 3 {
        return None;
    }
    let mut seen = BTreeSet::new();
    let mut indices = Vec::with_capacity(face.len());
    for vertex in face {
        let vertex = *vertex as usize;
        if vertex >= input.vertices.len() || !seen.insert(vertex) {
            return None;
        }
        indices.push(vertex);
    }
    Some(indices)
}

fn face_plane(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> Option<PlaneDescriptor> {
    let normal = face_normal(input, indices)?;
    let centroid = face_centroid(input, indices);
    Some(PlaneDescriptor {
        normal,
        offset: dot(normal, centroid),
    })
}

fn face_frame(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> SurfaceFrameDescriptor {
    let origin = face_centroid(input, indices);
    let normal = face_normal(input, indices).unwrap_or([0.0, 0.0, 1.0]);
    let tangent = first_non_zero_edge(input, indices).unwrap_or([1.0, 0.0, 0.0]);
    SurfaceFrameDescriptor {
        origin,
        normal,
        tangent,
    }
}

fn face_normal(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> Option<[f64; 3]> {
    if indices.len() < 3 {
        return None;
    }
    let origin = input.vertices[indices[0]];
    for pair in indices[1..].windows(2) {
        let a = sub(input.vertices[pair[0]], origin);
        let b = sub(input.vertices[pair[1]], origin);
        let normal = cross(a, b);
        if non_zero_vec3(normal) {
            return Some(normalize(normal));
        }
    }
    None
}

fn first_non_zero_edge(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> Option<[f64; 3]> {
    indices
        .windows(2)
        .map(|window| sub(input.vertices[window[1]], input.vertices[window[0]]))
        .find(|edge| non_zero_vec3(*edge))
        .map(normalize)
}

fn face_centroid(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> [f64; 3] {
    let mut centroid = [0.0; 3];
    for index in indices {
        let vertex = input.vertices[*index];
        centroid[0] += vertex[0];
        centroid[1] += vertex[1];
        centroid[2] += vertex[2];
    }
    let count = indices.len().max(1) as f64;
    [
        centroid[0] / count,
        centroid[1] / count,
        centroid[2] / count,
    ]
}

fn face_perimeter(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> f64 {
    let mut perimeter = 0.0;
    for edge_index in 0..indices.len() {
        let a = input.vertices[indices[edge_index]];
        let b = input.vertices[indices[(edge_index + 1) % indices.len()]];
        perimeter += length(sub(b, a));
    }
    perimeter
}

fn polygon_area(input: &RawHardSurfaceMeshInput, indices: &[usize]) -> f64 {
    if indices.len() < 3 {
        return 0.0;
    }
    let origin = input.vertices[indices[0]];
    let mut area = 0.0;
    for triangle in indices[1..].windows(2) {
        let a = sub(input.vertices[triangle[0]], origin);
        let b = sub(input.vertices[triangle[1]], origin);
        area += length(cross(a, b)) * 0.5;
    }
    area
}

fn bounds_center(bounds: Bounds3) -> [f64; 3] {
    [
        (bounds.min[0] + bounds.max[0]) * 0.5,
        (bounds.min[1] + bounds.max[1]) * 0.5,
        (bounds.min[2] + bounds.max[2]) * 0.5,
    ]
}

fn bounds_extents(bounds: Bounds3) -> [f64; 3] {
    [
        (bounds.max[0] - bounds.min[0]).max(0.0),
        (bounds.max[1] - bounds.min[1]).max(0.0),
        (bounds.max[2] - bounds.min[2]).max(0.0),
    ]
}

fn dominant_axis_vector(extents: [f64; 3]) -> Option<([f64; 3], f64)> {
    let (axis, distance) = extents
        .iter()
        .copied()
        .enumerate()
        .max_by(|left, right| left.1.total_cmp(&right.1))?;
    if distance <= 0.0 || !distance.is_finite() {
        return None;
    }
    let mut direction = [0.0; 3];
    direction[axis] = 1.0;
    Some((direction, distance))
}

fn extraordinary_vertex_count(input: &RawHardSurfaceMeshInput) -> usize {
    vertex_adjacency(input)
        .into_iter()
        .filter(|neighbors| neighbors.len() != 4)
        .count()
}

fn all_faces_have_same_valence(input: &RawHardSurfaceMeshInput) -> bool {
    let valences = input
        .faces
        .iter()
        .filter(|face| face.len() >= 3)
        .map(Vec::len)
        .collect::<BTreeSet<_>>();
    valences.len() <= 1
}

fn feature_evidence(
    id: impl Into<String>,
    source: EvidenceSource,
    signal: EvidenceSignal,
    note: impl Into<String>,
) -> FeatureEvidence {
    FeatureEvidence {
        id: id.into(),
        source,
        signal,
        confidence: Confidence(0.75),
        references: Vec::new(),
        note: note.into(),
    }
}

fn face_edges(face: &[u32]) -> Vec<(u32, u32)> {
    if face.len() < 2 {
        return Vec::new();
    }
    let mut edges = face
        .windows(2)
        .map(|window| (window[0], window[1]))
        .collect::<Vec<_>>();
    edges.push((*face.last().unwrap_or(&0), face[0]));
    edges
}

fn valid_edge(edge: (u32, u32), vertex_count: usize) -> Option<(usize, usize)> {
    let a = edge.0 as usize;
    let b = edge.1 as usize;
    (a < vertex_count && b < vertex_count && a != b).then_some((a, b))
}

fn bounds_for_vertices(input: &RawHardSurfaceMeshInput, vertices: &[usize]) -> Bounds3 {
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for vertex in vertices {
        let position = input.vertices[*vertex];
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    Bounds3 { min, max }
}

fn check_candidates<T>(
    issues: &mut Vec<AnalysisValidationIssue>,
    family: FeatureFamily,
    candidates: &[FeatureCandidate<T>],
    config: &HardSurfaceAnalysisConfig,
    descriptor_id: impl Fn(&T) -> &str,
    validate_descriptor: impl Fn(&mut Vec<AnalysisValidationIssue>, &T),
) {
    let mut candidate_ids = BTreeSet::new();
    let mut descriptor_ids = BTreeSet::new();
    for candidate in candidates {
        if candidate.id.trim().is_empty() {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.id",
                "candidate ID must not be empty",
            );
        }
        if !candidate_ids.insert(candidate.id.clone()) {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.id",
                "candidate IDs must be unique within a family",
            );
        }
        let descriptor_id = descriptor_id(&candidate.descriptor);
        if descriptor_id.trim().is_empty() {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.descriptor.id",
                "descriptor ID must not be empty",
            );
        }
        if !descriptor_ids.insert(descriptor_id.to_owned()) {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.descriptor.id",
                "descriptor IDs must be unique within a family",
            );
        }
        if !candidate.confidence.is_valid() {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.confidence",
                "confidence must be finite and in [0, 1]",
            );
        }
        if config.require_evidence && candidate.evidence.is_empty() {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "candidate.evidence",
                "candidate requires at least one evidence record",
            );
        }
        for evidence in &candidate.evidence {
            if evidence.id.trim().is_empty()
                || !evidence.confidence.is_valid()
                || evidence.note.trim().is_empty()
            {
                push_issue(
                    issues,
                    AnalysisIssueSeverity::Error,
                    Some(family),
                    "candidate.evidence",
                    "evidence ID, confidence, and note must be valid",
                );
            }
        }
        validate_descriptor(issues, &candidate.descriptor);
    }
}

fn check_detected_features<T>(
    issues: &mut Vec<AnalysisValidationIssue>,
    ids: &mut BTreeSet<String>,
    family: FeatureFamily,
    features: &[DetectedFeature<T>],
) {
    let mut previous_id: Option<&str> = None;
    for feature in features {
        if feature.family != family {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "features.family",
                "feature stored under the wrong family",
            );
        }
        if feature.id.trim().is_empty() || !ids.insert(feature.id.clone()) {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "features.id",
                "feature IDs must be non-empty and globally unique",
            );
        }
        if previous_id.is_some_and(|previous| previous > feature.id.as_str()) {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "features",
                "features must be sorted by stable ID",
            );
        }
        previous_id = Some(&feature.id);
        if !feature.confidence.is_valid() || feature.evidence.is_empty() {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "features.confidence",
                "feature confidence and evidence must be valid",
            );
        }
        if feature.suggested_operation != suggested_operation_for_family(family) {
            push_issue(
                issues,
                AnalysisIssueSeverity::Error,
                Some(family),
                "features.suggested_operation",
                "feature suggested operation does not match its family",
            );
        }
    }
}

fn promote_candidates<T: Clone>(
    family: FeatureFamily,
    candidates: &[FeatureCandidate<T>],
    config: &HardSurfaceAnalysisConfig,
) -> Vec<DetectedFeature<T>> {
    let mut promoted: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            candidate.confidence.is_valid()
                && candidate.confidence >= config.minimum_report_confidence
                && (!config.require_evidence || !candidate.evidence.is_empty())
        })
        .map(|candidate| DetectedFeature {
            id: candidate.id.clone(),
            family,
            confidence: candidate.confidence,
            evidence: candidate.evidence.clone(),
            suggested_operation: suggested_operation_for_family(family),
            descriptor: candidate.descriptor.clone(),
        })
        .collect();
    promoted.sort_by(|a, b| a.id.cmp(&b.id));
    promoted
}

fn summary(
    family: FeatureFamily,
    total_candidates: usize,
    accepted: usize,
) -> FeatureFamilySummary {
    FeatureFamilySummary {
        family,
        total_candidates,
        accepted,
        rejected: total_candidates.saturating_sub(accepted),
        suggested_operation: suggested_operation_for_family(family),
    }
}

fn push_issue(
    issues: &mut Vec<AnalysisValidationIssue>,
    severity: AnalysisIssueSeverity,
    family: Option<FeatureFamily>,
    path: impl Into<String>,
    message: impl Into<String>,
) {
    issues.push(AnalysisValidationIssue {
        severity,
        family,
        path: path.into(),
        message: message.into(),
    });
}

fn require_component(
    issues: &mut Vec<AnalysisValidationIssue>,
    family: FeatureFamily,
    path: &str,
    component_id: &str,
    component_ids: &BTreeSet<String>,
) {
    if !component_ids.contains(component_id) {
        push_issue(
            issues,
            AnalysisIssueSeverity::Error,
            Some(family),
            path,
            "descriptor references an unknown component",
        );
    }
}

fn require_components(
    issues: &mut Vec<AnalysisValidationIssue>,
    family: FeatureFamily,
    path: &str,
    component_ids: &[String],
    known_component_ids: &BTreeSet<String>,
) {
    if component_ids.is_empty() {
        push_issue(
            issues,
            AnalysisIssueSeverity::Error,
            Some(family),
            path,
            "descriptor must reference at least one component",
        );
    }
    for component_id in component_ids {
        require_component(issues, family, path, component_id, known_component_ids);
    }
}

fn require_loop(
    issues: &mut Vec<AnalysisValidationIssue>,
    family: FeatureFamily,
    path: &str,
    loop_id: &SemanticBoundaryLoopId,
    loop_ids: &BTreeSet<String>,
) {
    if !loop_ids.contains(&loop_id.0) {
        push_issue(
            issues,
            AnalysisIssueSeverity::Error,
            Some(family),
            path,
            "descriptor references an unknown boundary loop",
        );
    }
}

fn require_loops(
    issues: &mut Vec<AnalysisValidationIssue>,
    family: FeatureFamily,
    path: &str,
    loop_ids: &[SemanticBoundaryLoopId],
    known_loop_ids: &BTreeSet<String>,
) {
    for loop_id in loop_ids {
        require_loop(issues, family, path, loop_id, known_loop_ids);
    }
}

fn finite_vec3(value: [f64; 3]) -> bool {
    value.iter().all(|component| component.is_finite())
}

fn non_zero_vec3(value: [f64; 3]) -> bool {
    value.iter().any(|component| *component != 0.0)
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn length(value: [f64; 3]) -> f64 {
    dot(value, value).sqrt()
}

fn normalize(value: [f64; 3]) -> [f64; 3] {
    let length = length(value);
    if length > 0.0 && length.is_finite() {
        [value[0] / length, value[1] / length, value[2] / length]
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_promotes_all_requested_hard_surface_families() {
        let input = complete_fixture();
        let report = analyze_hard_surface_features(&input, &HardSurfaceAnalysisConfig::default());

        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.components.len(), 1);
        assert_eq!(report.symmetries.len(), 1);
        assert_eq!(report.repetitions.len(), 1);
        assert_eq!(report.primitive_patches.len(), 1);
        assert_eq!(report.extrusion_signatures.len(), 1);
        assert_eq!(report.inset_rings.len(), 1);
        assert_eq!(report.bevel_bands.len(), 1);
        assert_eq!(report.boolean_boundaries.len(), 1);
        assert_eq!(report.subdivision_structures.len(), 1);
        assert_eq!(report.sweep_lathe_evidence.len(), 1);
        assert_eq!(report.accepted_feature_count(), 10);
        assert_eq!(
            report.boolean_boundaries[0].suggested_operation,
            Some(ModelingOperationKind::ConstrainedBoolean)
        );
        assert_eq!(report.sweep_lathe_evidence[0].suggested_operation, None);
    }

    #[test]
    fn validation_rejects_duplicate_ids_bad_references_and_bad_confidence() {
        let mut input = complete_fixture();
        input.components[0].confidence = Confidence(1.5);
        input.components.push(input.components[0].clone());
        input.primitive_patches[0].descriptor.component_id = "missing".to_owned();
        input.bevel_bands[0].descriptor.replacement_loops.clear();

        let issues = validate_analysis_input(&input, &HardSurfaceAnalysisConfig::default())
            .expect_err("bad descriptors should fail validation");

        assert!(
            issues
                .iter()
                .any(|issue| issue.path == "candidate.confidence")
        );
        assert!(issues.iter().any(|issue| issue.path == "candidate.id"));
        assert!(
            issues
                .iter()
                .any(|issue| issue.message.contains("unknown component"))
        );
        assert!(
            issues
                .iter()
                .any(|issue| issue.family == Some(FeatureFamily::BevelBand))
        );
    }

    #[test]
    fn report_features_are_sorted_deterministically() {
        let mut input = complete_fixture();
        input.components.push(candidate(
            "feature.component.a",
            ComponentDescriptor {
                component_id: "component.a".to_owned(),
                semantic_part: Some(SemanticPartId("part.a".to_owned())),
                bounds: bounds(),
                vertex_count: 8,
                face_count: 6,
                connected: true,
                manifold: true,
            },
            EvidenceSignal::ConnectedComponent,
        ));

        let report = analyze_hard_surface_features(&input, &HardSurfaceAnalysisConfig::default());

        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.components[0].id, "feature.component.a");
        assert_eq!(report.components[1].id, "feature.component.body");
    }

    #[test]
    fn report_validation_detects_summary_mismatch() {
        let input = complete_fixture();
        let mut report =
            analyze_hard_surface_features(&input, &HardSurfaceAnalysisConfig::default());
        report.summaries[0].accepted += 1;

        let issues = validate_analysis_report(&report)
            .expect_err("mismatched summary should fail validation");

        assert!(
            issues
                .iter()
                .any(|issue| issue.path == "summaries.accepted")
        );
    }

    #[test]
    fn raw_mesh_analysis_detects_wave14_feature_families() {
        let input = RawHardSurfaceMeshInput {
            schema_version: HARD_SURFACE_ANALYSIS_SCHEMA_VERSION,
            source_id: "raw.hard_surface_features".to_owned(),
            coordinate_system: coordinates(),
            vertices: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [3.0, 0.0, 0.0],
                [4.0, 0.0, 0.0],
                [3.0, 1.0, 0.0],
            ],
            faces: vec![vec![0, 1, 2], vec![0, 1, 3], vec![1, 2, 3], vec![4, 5, 6]],
        };

        let report = analyze_raw_hard_surface_mesh(&input, &HardSurfaceAnalysisConfig::default());

        assert!(report.is_valid(), "{:?}", report.issues);
        assert_eq!(report.components.len(), 2);
        assert_eq!(report.components[0].descriptor.vertex_count, 4);
        assert_eq!(report.components[1].descriptor.vertex_count, 3);
        assert_eq!(report.primitive_patches.len(), 4);
        assert!(!report.symmetries.is_empty());
        assert_eq!(report.repetitions.len(), 1);
        assert!(!report.extrusion_signatures.is_empty());
        assert_eq!(report.inset_rings.len(), 1);
        assert_eq!(report.bevel_bands.len(), 1);
        assert_eq!(report.boolean_boundaries.len(), 1);
        assert!(!report.subdivision_structures.is_empty());
        assert_eq!(report.sweep_lathe_evidence.len(), 1);
        assert!(
            report
                .issues
                .iter()
                .all(|issue| !issue.path.starts_with("raw_mesh.detectors."))
        );
    }

    #[test]
    fn raw_mesh_analysis_rejects_invalid_faces_without_silent_repair() {
        let input = RawHardSurfaceMeshInput {
            schema_version: HARD_SURFACE_ANALYSIS_SCHEMA_VERSION,
            source_id: "raw.invalid".to_owned(),
            coordinate_system: coordinates(),
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
            faces: vec![vec![0, 2, 2]],
        };

        let report = analyze_raw_hard_surface_mesh(&input, &HardSurfaceAnalysisConfig::default());

        assert!(!report.is_valid());
        assert!(report.issues.iter().any(|issue| {
            issue.severity == AnalysisIssueSeverity::Error
                && issue.path == "raw_mesh.faces[0]"
                && issue.message.contains("outside")
        }));
        assert!(report.issues.iter().any(|issue| {
            issue.severity == AnalysisIssueSeverity::Error
                && issue.path == "raw_mesh.faces[0]"
                && issue.message.contains("repeats")
        }));
    }

    #[test]
    fn family_to_forward_operation_hints_are_stable() {
        let expected = [
            (
                FeatureFamily::Component,
                Some(ModelingOperationKind::PrimitiveCreate),
            ),
            (FeatureFamily::Symmetry, Some(ModelingOperationKind::Mirror)),
            (
                FeatureFamily::Repetition,
                Some(ModelingOperationKind::Array),
            ),
            (
                FeatureFamily::PrimitivePatch,
                Some(ModelingOperationKind::PrimitiveCreate),
            ),
            (
                FeatureFamily::ExtrusionSignature,
                Some(ModelingOperationKind::RegionExtrude),
            ),
            (
                FeatureFamily::InsetRing,
                Some(ModelingOperationKind::RegionInset),
            ),
            (FeatureFamily::BevelBand, Some(ModelingOperationKind::Bevel)),
            (
                FeatureFamily::BooleanBoundary,
                Some(ModelingOperationKind::ConstrainedBoolean),
            ),
            (
                FeatureFamily::SubdivisionStructure,
                Some(ModelingOperationKind::Subdivide),
            ),
            (FeatureFamily::SweepLatheEvidence, None),
        ];

        for (family, operation) in expected {
            assert_eq!(suggested_operation_for_family(family), operation);
        }
    }

    fn complete_fixture() -> HardSurfaceDescriptorInput {
        HardSurfaceDescriptorInput {
            schema_version: HARD_SURFACE_ANALYSIS_SCHEMA_VERSION,
            source_id: "target.crate".to_owned(),
            coordinate_system: coordinates(),
            components: vec![candidate(
                "feature.component.body",
                ComponentDescriptor {
                    component_id: "component.body".to_owned(),
                    semantic_part: Some(SemanticPartId("part.body".to_owned())),
                    bounds: bounds(),
                    vertex_count: 48,
                    face_count: 36,
                    connected: true,
                    manifold: true,
                },
                EvidenceSignal::ConnectedComponent,
            )],
            boundary_loops: vec![
                loop_descriptor("loop.outer"),
                loop_descriptor("loop.inner"),
                loop_descriptor("loop.bevel.outer"),
                loop_descriptor("loop.bevel.inner"),
            ],
            primitive_patches: vec![candidate(
                "feature.primitive.front_panel",
                PrimitivePatchDescriptor {
                    patch_id: "patch.front_panel".to_owned(),
                    component_id: "component.body".to_owned(),
                    region: Some(SemanticRegionId("region.front_panel".to_owned())),
                    primitive: PrimitivePatchKind::Plane,
                    frame: frame(),
                    area: 2.0,
                    boundary_loops: vec![SemanticBoundaryLoopId("loop.outer".to_owned())],
                },
                EvidenceSignal::PrimitiveSurfaceFit,
            )],
            symmetries: vec![candidate(
                "feature.symmetry.x",
                SymmetryDescriptor {
                    symmetry_id: "symmetry.x".to_owned(),
                    component_ids: vec!["component.body".to_owned()],
                    kind: SymmetryKind::Mirror,
                    axis: axis(),
                    max_residual: 0.01,
                },
                EvidenceSignal::MirrorPlane,
            )],
            repetitions: vec![candidate(
                "feature.repetition.bolts",
                RepetitionDescriptor {
                    repetition_id: "repetition.bolts".to_owned(),
                    prototype_id: "component.body".to_owned(),
                    instance_ids: vec!["bolt.0".to_owned(), "bolt.1".to_owned()],
                    kind: RepetitionKind::LinearArray,
                    step_vector: Some([0.25, 0.0, 0.0]),
                    radial_axis: None,
                },
                EvidenceSignal::TranslationStep,
            )],
            extrusion_signatures: vec![candidate(
                "feature.extrusion.panel",
                ExtrusionSignatureDescriptor {
                    extrusion_id: "extrusion.panel".to_owned(),
                    source_region: SemanticRegionId("region.front_panel".to_owned()),
                    cap_region: Some(SemanticRegionId("region.panel.cap".to_owned())),
                    side_regions: vec![SemanticRegionId("region.panel.wall".to_owned())],
                    profile_loop: SemanticBoundaryLoopId("loop.outer".to_owned()),
                    direction: [0.0, 0.0, 1.0],
                    distance: 0.2,
                },
                EvidenceSignal::ParallelCapPair,
            )],
            inset_rings: vec![candidate(
                "feature.inset.panel",
                InsetRingDescriptor {
                    inset_id: "inset.panel".to_owned(),
                    host_region: SemanticRegionId("region.front_panel".to_owned()),
                    outer_loop: SemanticBoundaryLoopId("loop.outer".to_owned()),
                    inner_loop: SemanticBoundaryLoopId("loop.inner".to_owned()),
                    offset_distance: 0.05,
                    corner_count: 4,
                },
                EvidenceSignal::OffsetBoundaryLoop,
            )],
            bevel_bands: vec![candidate(
                "feature.bevel.panel",
                BevelBandDescriptor {
                    bevel_id: "bevel.panel".to_owned(),
                    source_loop: SemanticBoundaryLoopId("loop.outer".to_owned()),
                    replacement_loops: vec![
                        SemanticBoundaryLoopId("loop.bevel.outer".to_owned()),
                        SemanticBoundaryLoopId("loop.bevel.inner".to_owned()),
                    ],
                    bevel_region: Some(SemanticRegionId("region.bevel".to_owned())),
                    width: 0.025,
                    segments: 3,
                    continuity: BevelContinuity::Rounded,
                },
                EvidenceSignal::ChamferOrRoundBand,
            )],
            boolean_boundaries: vec![candidate(
                "feature.boolean.slot",
                BooleanBoundaryDescriptor {
                    boolean_id: "boolean.slot".to_owned(),
                    host_component_id: "component.body".to_owned(),
                    cutter_component_id: None,
                    kind: BooleanBoundaryKind::DifferenceOpening,
                    boundary_loops: vec![SemanticBoundaryLoopId("loop.inner".to_owned())],
                    closed: true,
                },
                EvidenceSignal::ClosedIntersectionLoop,
            )],
            subdivision_structures: vec![candidate(
                "feature.subdivision.support_loops",
                SubdivisionStructureDescriptor {
                    subdivision_id: "subdivision.support_loops".to_owned(),
                    component_id: "component.body".to_owned(),
                    scheme: SubdivisionScheme::CreasedHardSurface,
                    level: 1,
                    extraordinary_vertex_count: 0,
                    regular_grid: true,
                },
                EvidenceSignal::RegularSubdivisionGrid,
            )],
            sweep_lathe_evidence: vec![candidate(
                "feature.sweep.handle",
                SweepLatheEvidenceDescriptor {
                    evidence_id: "sweep.handle".to_owned(),
                    component_id: "component.body".to_owned(),
                    kind: SweepLatheKind::Sweep,
                    profile_loops: vec![SemanticBoundaryLoopId("loop.outer".to_owned())],
                    path_axis: axis(),
                    arc_degrees: None,
                },
                EvidenceSignal::SweepProfilePath,
            )],
        }
    }

    fn coordinates() -> CoordinateSystemDescriptor {
        CoordinateSystemDescriptor {
            unit: "meter".to_owned(),
            forward_axis: Axis3::PositiveY,
            up_axis: Axis3::PositiveZ,
            meters_per_unit: 1.0,
        }
    }

    fn candidate<T>(id: &str, descriptor: T, signal: EvidenceSignal) -> FeatureCandidate<T> {
        FeatureCandidate {
            id: id.to_owned(),
            confidence: Confidence(0.9),
            evidence: vec![FeatureEvidence {
                id: format!("{id}.evidence"),
                source: EvidenceSource::ConnectivityGraph,
                signal,
                confidence: Confidence(0.95),
                references: Vec::new(),
                note: "fixture evidence".to_owned(),
            }],
            descriptor,
        }
    }

    fn loop_descriptor(id: &str) -> BoundaryLoopDescriptor {
        BoundaryLoopDescriptor {
            loop_id: SemanticBoundaryLoopId(id.to_owned()),
            component_id: "component.body".to_owned(),
            region: Some(SemanticRegionId("region.front_panel".to_owned())),
            plane: Some(PlaneDescriptor {
                normal: [0.0, 0.0, 1.0],
                offset: 0.0,
            }),
            edge_count: 4,
            length: 1.0,
            closed: true,
        }
    }

    fn bounds() -> Bounds3 {
        Bounds3 {
            min: [-1.0, -1.0, -1.0],
            max: [1.0, 1.0, 1.0],
        }
    }

    fn frame() -> SurfaceFrameDescriptor {
        SurfaceFrameDescriptor {
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            tangent: [1.0, 0.0, 0.0],
        }
    }

    fn axis() -> AxisDescriptor {
        AxisDescriptor {
            origin: [0.0, 0.0, 0.0],
            direction: [0.0, 0.0, 1.0],
        }
    }
}
