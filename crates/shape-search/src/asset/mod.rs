//! Semantic candidate generation for explicit asset recipes.
pub mod scoring;

use std::collections::{BTreeMap, BTreeSet};

use rand::{RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use shape_asset::{
    ArraySpacingEdit, AssetEdit, AssetEditProgram, AssetRecipe, AssetValidationReport,
    BoundaryLoopId, CountRangeHint, Frame3, GeneratorDimensionEdit, GeometrySource,
    ModelingOperationSpec, OperationId, ParameterDescriptor, PartDefinitionId, PartInstanceId,
    RegionId, Transform3, apply_edit_program, enumerate_parameters, get_scalar,
    validate_asset_recipe,
};
use thiserror::Error;

/// Exploration distance for explicit asset recipes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AssetCandidateMode {
    /// Local semantic edits that preserve topology by default.
    Refine,
    /// Broader edits that may include structural or topology-changing choices.
    Explore,
}

/// Candidate generation request for an [`AssetRecipe`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCandidateRequest {
    /// Deterministic generation seed.
    pub seed: u64,
    /// Number of proposal programs to attempt.
    pub proposal_count: usize,
    /// Maximum number of accepted candidates to return.
    pub result_count: usize,
    /// Search breadth.
    pub mode: AssetCandidateMode,
}

/// One accepted explicit asset candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCandidate {
    /// Stable ID within this generation request.
    pub id: u64,
    /// Edited recipe after applying the program.
    pub recipe: AssetRecipe,
    /// Semantic edit program that produced the candidate.
    pub program: AssetEditProgram,
    /// Per-edit diagnostics explaining each generated change.
    pub diagnostics: AssetCandidateDiagnostics,
}

/// Candidate generation output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCandidateOutput {
    /// Accepted candidates in deterministic proposal order.
    pub candidates: Vec<AssetCandidate>,
    /// Generation-level diagnostics.
    pub diagnostics: AssetCandidateGenerationDiagnostics,
}

/// Diagnostics for one accepted candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetCandidateDiagnostics {
    /// Changed semantic controls and structural choices in program order.
    pub changes: Vec<AssetChangeDiagnostic>,
}

/// Explanation for one generated semantic edit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetChangeDiagnostic {
    /// Stable edit category.
    pub kind: AssetCandidateEditKind,
    /// Stable subject path for the changed item.
    pub subject: String,
    /// Value summary before the edit.
    pub before: String,
    /// Value summary after the edit.
    pub after: String,
    /// Human-readable explanation.
    pub message: String,
    /// Whether this edit can change generated topology.
    pub topology_changing: bool,
}

/// Stable edit categories emitted by asset candidate generation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AssetCandidateEditKind {
    /// Descriptor-backed scalar parameter.
    Parameter,
    /// Part instance transform.
    Transform,
    /// Base generator dimension or proportion.
    GeneratorDimension,
    /// Bevel radius or segment control.
    Bevel,
    /// Sweep profile or path control.
    Sweep,
    /// Lathe profile control.
    Lathe,
    /// Array count.
    ArrayCount,
    /// Array spacing or angle.
    ArraySpacing,
    /// Optional-part visibility.
    OptionalPart,
    /// Compatible part replacement.
    Replacement,
    /// Segment or other detail-density control.
    DetailDensity,
    /// Ordered local modeling operation edit.
    ModelingOperation,
}

/// Generation-level diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetCandidateGenerationDiagnostics {
    /// Requested proposals.
    pub requested_proposals: usize,
    /// Requested result count.
    pub requested_candidates: usize,
    /// Proposals attempted.
    pub attempted_proposals: usize,
    /// Accepted candidates before result truncation.
    pub accepted_candidates: usize,
    /// Returned candidates.
    pub returned_candidates: usize,
    /// Editable opportunities available after mode and lock filters.
    pub available_edit_count: usize,
    /// Opportunities skipped because of parameter, part, subtree, or topology locks.
    pub locked_targets_skipped: usize,
    /// Proposal rejection counters.
    pub rejections: BTreeMap<AssetCandidateRejectionReason, usize>,
}

/// Rejection reason for an attempted asset candidate.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AssetCandidateRejectionReason {
    /// The selected opportunities produced no operation.
    EmptyProgram,
    /// The generated program repeated an already accepted program.
    DuplicateProgram,
    /// Applying the program failed.
    EditRejected,
    /// The edited recipe failed validation.
    ValidationRejected,
}

/// Asset candidate generation errors.
#[derive(Debug, Error)]
pub enum AssetCandidateError {
    /// Request fields are inconsistent.
    #[error("invalid asset candidate request: {0}")]
    InvalidRequest(&'static str),
    /// The input recipe failed validation.
    #[error("invalid asset recipe")]
    InvalidRecipe(AssetValidationReport),
    /// No semantic edit targets remain after validation, mode filters, and locks.
    #[error("no editable asset targets match the request")]
    NoEditableTargets,
}

/// Generate semantic candidates for an explicit asset recipe.
///
/// Programs are generated deterministically from a seeded ChaCha stream and
/// applied through `shape-asset`, so raw vertices are never mutated directly.
pub fn generate_asset_candidates(
    recipe: &AssetRecipe,
    request: &AssetCandidateRequest,
) -> Result<AssetCandidateOutput, AssetCandidateError> {
    validate_request(request)?;
    let validation = validate_asset_recipe(recipe);
    if !validation.is_valid() {
        return Err(AssetCandidateError::InvalidRecipe(validation));
    }

    let mut skipped = 0;
    let mut opportunities = collect_opportunities(recipe, request.mode, &mut skipped);
    if opportunities.is_empty() {
        return Err(AssetCandidateError::NoEditableTargets);
    }
    sort_opportunities(&mut opportunities, request.mode);

    let mut diagnostics = AssetCandidateGenerationDiagnostics {
        requested_proposals: request.proposal_count,
        requested_candidates: request.result_count,
        attempted_proposals: 0,
        accepted_candidates: 0,
        returned_candidates: 0,
        available_edit_count: opportunities.len(),
        locked_targets_skipped: skipped,
        rejections: BTreeMap::new(),
    };
    let mut seen_programs = BTreeSet::new();
    let mut accepted = Vec::new();

    for proposal_index in 0..request.proposal_count {
        diagnostics.attempted_proposals += 1;
        let proposal_seed = proposal_seed(request.seed, proposal_index as u64);
        let mut rng = ChaCha8Rng::seed_from_u64(proposal_seed);
        let selected = select_opportunities(&opportunities, request.mode, proposal_index, &mut rng);
        let mut operations = Vec::new();
        let mut id_allocator = ProposalIdAllocator::from_recipe(recipe);
        for opportunity in selected {
            if let Some(operation) =
                opportunity.build_edit(recipe, request.mode, &mut rng, &mut id_allocator)
            {
                operations.push(operation);
            }
        }
        if operations.is_empty() {
            increment_rejection(
                &mut diagnostics,
                AssetCandidateRejectionReason::EmptyProgram,
            );
            continue;
        }

        let program = AssetEditProgram {
            label: format!("asset-{:?}-{proposal_index:04}", request.mode).to_ascii_lowercase(),
            seed: proposal_seed,
            operations,
        };
        let program_key = format!("{:?}", program.operations);
        if !seen_programs.insert(program_key) {
            increment_rejection(
                &mut diagnostics,
                AssetCandidateRejectionReason::DuplicateProgram,
            );
            continue;
        }

        let candidate_recipe = match apply_edit_program(recipe, &program) {
            Ok(candidate) => candidate,
            Err(_) => {
                increment_rejection(
                    &mut diagnostics,
                    AssetCandidateRejectionReason::EditRejected,
                );
                continue;
            }
        };
        if !validate_asset_recipe(&candidate_recipe).is_valid() {
            increment_rejection(
                &mut diagnostics,
                AssetCandidateRejectionReason::ValidationRejected,
            );
            continue;
        }

        let changes = program
            .operations
            .iter()
            .map(|operation| diagnose_change(recipe, &candidate_recipe, operation))
            .collect();
        accepted.push(AssetCandidate {
            id: stable_candidate_id(proposal_seed, proposal_index as u64),
            recipe: candidate_recipe,
            program,
            diagnostics: AssetCandidateDiagnostics { changes },
        });
    }

    diagnostics.accepted_candidates = accepted.len();
    accepted.truncate(request.result_count);
    diagnostics.returned_candidates = accepted.len();

    Ok(AssetCandidateOutput {
        candidates: accepted,
        diagnostics,
    })
}

fn validate_request(request: &AssetCandidateRequest) -> Result<(), AssetCandidateError> {
    if request.proposal_count == 0 {
        return Err(AssetCandidateError::InvalidRequest(
            "proposal_count must be greater than zero",
        ));
    }
    if request.result_count == 0 {
        return Err(AssetCandidateError::InvalidRequest(
            "result_count must be greater than zero",
        ));
    }
    Ok(())
}

fn collect_opportunities(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
) -> Vec<EditOpportunity> {
    let mut opportunities = Vec::new();
    collect_parameter_opportunities(recipe, mode, skipped, &mut opportunities);
    collect_instance_opportunities(recipe, mode, skipped, &mut opportunities);
    collect_definition_opportunities(recipe, mode, skipped, &mut opportunities);
    collect_optional_part_opportunities(recipe, mode, skipped, &mut opportunities);
    collect_replacement_opportunities(recipe, mode, skipped, &mut opportunities);
    opportunities
}

fn collect_parameter_opportunities(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
    opportunities: &mut Vec<EditOpportunity>,
) {
    for parameter in enumerate_parameters(recipe) {
        if recipe.locks.contains(&parameter.id) {
            *skipped += 1;
            continue;
        }
        if let Some(definition) = definition_id_from_path(&parameter.path)
            && parameter.topology_changing
            && recipe.topology_locks.contains(&definition)
        {
            *skipped += 1;
            continue;
        }
        if let Some(instance) = instance_id_from_path(&parameter.path)
            && !instance_is_editable(recipe, instance)
        {
            *skipped += 1;
            continue;
        }
        if mode == AssetCandidateMode::Refine && parameter.topology_changing {
            continue;
        }
        let Ok(value) = get_scalar(recipe, &parameter.path) else {
            continue;
        };
        if let Some(range) = effective_parameter_range(recipe, &parameter, value) {
            opportunities.push(EditOpportunity::ScalarParameter {
                parameter,
                current: value,
                range,
            });
        }
    }
}

fn collect_instance_opportunities(
    recipe: &AssetRecipe,
    _mode: AssetCandidateMode,
    skipped: &mut usize,
    opportunities: &mut Vec<EditOpportunity>,
) {
    for (id, instance) in &recipe.instances {
        if instance_is_editable(recipe, *id) {
            opportunities.push(EditOpportunity::Transform {
                instance: *id,
                current: instance.local_transform.clone(),
            });
        } else {
            *skipped += 1;
        }
    }
}

fn collect_definition_opportunities(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
    opportunities: &mut Vec<EditOpportunity>,
) {
    for (definition_id, definition) in &recipe.definitions {
        match &definition.geometry.source {
            GeometrySource::RoundedBox {
                half_extents,
                radius,
            } => {
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::RoundedBoxHalfExtents {
                        definition: *definition_id,
                        current: *half_extents,
                    },
                ));
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::RoundedBoxRadius {
                        definition: *definition_id,
                        current: *radius,
                    },
                ));
            }
            GeometrySource::Cylinder {
                radius,
                height,
                radial_segments,
            } => {
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::CylinderRadius {
                        definition: *definition_id,
                        current: *radius,
                    },
                ));
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::CylinderHeight {
                        definition: *definition_id,
                        current: *height,
                    },
                ));
                push_topology_opportunity(
                    recipe,
                    mode,
                    skipped,
                    *definition_id,
                    EditOpportunity::DetailDensity(DetailDensityTarget::CylinderRadialDetail {
                        definition: *definition_id,
                        current: *radial_segments,
                    }),
                    opportunities,
                );
            }
            GeometrySource::Frustum {
                bottom_radius,
                top_radius,
                height,
                radial_segments,
            } => {
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::FrustumBottomRadius {
                        definition: *definition_id,
                        current: *bottom_radius,
                    },
                ));
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::FrustumTopRadius {
                        definition: *definition_id,
                        current: *top_radius,
                    },
                ));
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::FrustumHeight {
                        definition: *definition_id,
                        current: *height,
                    },
                ));
                push_topology_opportunity(
                    recipe,
                    mode,
                    skipped,
                    *definition_id,
                    EditOpportunity::DetailDensity(DetailDensityTarget::FrustumRadialDetail {
                        definition: *definition_id,
                        current: *radial_segments,
                    }),
                    opportunities,
                );
            }
            GeometrySource::Plate { size, thickness } => {
                if !definition.geometry.operations.iter().any(operation_is_cut) {
                    opportunities.push(EditOpportunity::Dimension(
                        DefinitionDimensionTarget::PlateSize {
                            definition: *definition_id,
                            current: *size,
                        },
                    ));
                }
                opportunities.push(EditOpportunity::Dimension(
                    DefinitionDimensionTarget::PlateThickness {
                        definition: *definition_id,
                        current: *thickness,
                    },
                ));
            }
            GeometrySource::Sweep { profile, path } => {
                for (index, point) in profile.iter().copied().enumerate() {
                    opportunities.push(EditOpportunity::SweepProfilePoint {
                        definition: *definition_id,
                        index,
                        current: point,
                    });
                }
                for (index, frame) in path.iter().cloned().enumerate() {
                    if path.len() <= 2 || (index > 0 && index + 1 < path.len()) {
                        opportunities.push(EditOpportunity::SweepPathFrame {
                            definition: *definition_id,
                            index,
                            current: frame,
                        });
                    }
                }
            }
            GeometrySource::Lathe { profile, segments } => {
                for (index, point) in profile.iter().copied().enumerate() {
                    opportunities.push(EditOpportunity::LatheProfilePoint {
                        definition: *definition_id,
                        index,
                        current: point,
                    });
                }
                push_topology_opportunity(
                    recipe,
                    mode,
                    skipped,
                    *definition_id,
                    EditOpportunity::DetailDensity(DetailDensityTarget::LatheSegmentCount {
                        definition: *definition_id,
                        current: *segments,
                    }),
                    opportunities,
                );
            }
            GeometrySource::LiteralMesh { .. } | GeometrySource::ReservedBooleanResult { .. } => {}
        }

        for operation in &definition.geometry.operations {
            match operation {
                ModelingOperationSpec::SetBevelProfile {
                    operation,
                    radius,
                    segments,
                } => {
                    opportunities.push(EditOpportunity::BevelRadius {
                        definition: *definition_id,
                        operation: *operation,
                        current: *radius,
                    });
                    push_topology_opportunity(
                        recipe,
                        mode,
                        skipped,
                        *definition_id,
                        EditOpportunity::DetailDensity(DetailDensityTarget::BevelSegmentCount {
                            definition: *definition_id,
                            operation: *operation,
                            current: *segments,
                        }),
                        opportunities,
                    );
                }
                ModelingOperationSpec::LinearArray {
                    operation,
                    count,
                    offset,
                } => {
                    if let Some(range) = recipe.variation.count_ranges.get(operation).copied() {
                        push_topology_opportunity(
                            recipe,
                            mode,
                            skipped,
                            *definition_id,
                            EditOpportunity::ArrayCount {
                                definition: *definition_id,
                                operation: *operation,
                                current: *count,
                                range,
                            },
                            opportunities,
                        );
                    }
                    opportunities.push(EditOpportunity::LinearArraySpacing {
                        definition: *definition_id,
                        operation: *operation,
                        current: *offset,
                    });
                }
                ModelingOperationSpec::RadialArray {
                    operation,
                    count,
                    angle_degrees,
                    ..
                } => {
                    if let Some(range) = recipe.variation.count_ranges.get(operation).copied() {
                        push_topology_opportunity(
                            recipe,
                            mode,
                            skipped,
                            *definition_id,
                            EditOpportunity::ArrayCount {
                                definition: *definition_id,
                                operation: *operation,
                                current: *count,
                                range,
                            },
                            opportunities,
                        );
                    }
                    opportunities.push(EditOpportunity::RadialArrayAngle {
                        definition: *definition_id,
                        operation: *operation,
                        current: *angle_degrees,
                    });
                }
                ModelingOperationSpec::RecessedPanelCut {
                    operation,
                    center,
                    size,
                    ..
                } => {
                    push_topology_opportunity(
                        recipe,
                        mode,
                        skipped,
                        *definition_id,
                        EditOpportunity::DuplicateCut {
                            definition: *definition_id,
                            operation: *operation,
                            source: CutDuplicateSource::RecessedPanel {
                                center: *center,
                                size: *size,
                            },
                        },
                        opportunities,
                    );
                }
                ModelingOperationSpec::RectangularThroughCut {
                    operation,
                    center,
                    size,
                    ..
                } => {
                    push_topology_opportunity(
                        recipe,
                        mode,
                        skipped,
                        *definition_id,
                        EditOpportunity::DuplicateCut {
                            definition: *definition_id,
                            operation: *operation,
                            source: CutDuplicateSource::RectangularThrough {
                                center: *center,
                                size: *size,
                            },
                        },
                        opportunities,
                    );
                }
                ModelingOperationSpec::CircularThroughCut {
                    operation,
                    center,
                    radius,
                    ..
                } => {
                    push_topology_opportunity(
                        recipe,
                        mode,
                        skipped,
                        *definition_id,
                        EditOpportunity::DuplicateCut {
                            definition: *definition_id,
                            operation: *operation,
                            source: CutDuplicateSource::CircularThrough {
                                center: *center,
                                radius: *radius,
                            },
                        },
                        opportunities,
                    );
                }
                ModelingOperationSpec::TransformGeometry { .. }
                | ModelingOperationSpec::AddPanel { .. }
                | ModelingOperationSpec::AddTrim { .. }
                | ModelingOperationSpec::MirrorInstances { .. }
                | ModelingOperationSpec::ReservedBoolean { .. }
                | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
            }
        }
    }
}

fn collect_optional_part_opportunities(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
    opportunities: &mut Vec<EditOpportunity>,
) {
    if mode == AssetCandidateMode::Refine {
        return;
    }
    for instance_id in &recipe.variation.optional_instances {
        let Some(instance) = recipe.instances.get(instance_id) else {
            continue;
        };
        if instance_is_editable(recipe, *instance_id) {
            opportunities.push(EditOpportunity::OptionalPart {
                instance: *instance_id,
                current: instance.enabled,
            });
        } else {
            *skipped += 1;
        }
    }
}

fn collect_replacement_opportunities(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
    opportunities: &mut Vec<EditOpportunity>,
) {
    if mode == AssetCandidateMode::Refine {
        return;
    }
    for (instance_id, instance) in &recipe.instances {
        if !instance_is_editable(recipe, *instance_id) {
            *skipped += 1;
            continue;
        }
        if recipe.topology_locks.contains(&instance.definition) {
            *skipped += 1;
            continue;
        }
        let Some(definition) = recipe.definitions.get(&instance.definition) else {
            continue;
        };
        let Some(group_name) = definition.variant_group.as_ref() else {
            continue;
        };
        let Some(group) = recipe.variation.replacement_groups.get(group_name) else {
            continue;
        };
        for replacement in &group.definitions {
            if *replacement != instance.definition {
                opportunities.push(EditOpportunity::Replacement {
                    instance: *instance_id,
                    to: *replacement,
                });
            }
        }
    }
}

fn push_topology_opportunity(
    recipe: &AssetRecipe,
    mode: AssetCandidateMode,
    skipped: &mut usize,
    definition: PartDefinitionId,
    opportunity: EditOpportunity,
    opportunities: &mut Vec<EditOpportunity>,
) {
    if mode == AssetCandidateMode::Refine {
        return;
    }
    if recipe.topology_locks.contains(&definition) {
        *skipped += 1;
        return;
    }
    opportunities.push(opportunity);
}

fn operation_is_cut(operation: &ModelingOperationSpec) -> bool {
    matches!(
        operation,
        ModelingOperationSpec::RecessedPanelCut { .. }
            | ModelingOperationSpec::RectangularThroughCut { .. }
            | ModelingOperationSpec::CircularThroughCut { .. }
    )
}

fn select_opportunities(
    opportunities: &[EditOpportunity],
    mode: AssetCandidateMode,
    proposal_index: usize,
    rng: &mut ChaCha8Rng,
) -> Vec<EditOpportunity> {
    let target_count = match mode {
        AssetCandidateMode::Refine => {
            if opportunities.len() > 1 && rng.random_bool(0.30) {
                2
            } else {
                1
            }
        }
        AssetCandidateMode::Explore => rng.random_range(3..=5).min(opportunities.len()),
    };
    let mut pool = opportunities.to_vec();
    let mut selected = Vec::with_capacity(target_count);
    if pool.is_empty() {
        return selected;
    }
    let first_index = proposal_index % pool.len();
    selected.push(pool.remove(first_index));
    while selected.len() < target_count && !pool.is_empty() {
        let index = rng.random_range(0..pool.len());
        let opportunity = pool.remove(index);
        if opportunity.is_structural_cut_duplicate()
            && selected
                .iter()
                .any(EditOpportunity::is_structural_cut_duplicate)
        {
            continue;
        }
        selected.push(opportunity);
    }
    selected
}

#[derive(Debug, Clone)]
enum EditOpportunity {
    ScalarParameter {
        parameter: ParameterDescriptor,
        current: f32,
        range: ParameterRange,
    },
    Transform {
        instance: PartInstanceId,
        current: Transform3,
    },
    Dimension(DefinitionDimensionTarget),
    BevelRadius {
        definition: PartDefinitionId,
        operation: OperationId,
        current: f32,
    },
    SweepProfilePoint {
        definition: PartDefinitionId,
        index: usize,
        current: [f32; 2],
    },
    SweepPathFrame {
        definition: PartDefinitionId,
        index: usize,
        current: Frame3,
    },
    LatheProfilePoint {
        definition: PartDefinitionId,
        index: usize,
        current: [f32; 2],
    },
    ArrayCount {
        definition: PartDefinitionId,
        operation: OperationId,
        current: u32,
        range: CountRangeHint,
    },
    LinearArraySpacing {
        definition: PartDefinitionId,
        operation: OperationId,
        current: [f32; 3],
    },
    RadialArrayAngle {
        definition: PartDefinitionId,
        operation: OperationId,
        current: f32,
    },
    OptionalPart {
        instance: PartInstanceId,
        current: bool,
    },
    Replacement {
        instance: PartInstanceId,
        to: PartDefinitionId,
    },
    DuplicateCut {
        definition: PartDefinitionId,
        operation: OperationId,
        source: CutDuplicateSource,
    },
    DetailDensity(DetailDensityTarget),
}

#[derive(Debug, Clone)]
enum CutDuplicateSource {
    RecessedPanel { center: [f32; 2], size: [f32; 2] },
    RectangularThrough { center: [f32; 2], size: [f32; 2] },
    CircularThrough { center: [f32; 2], radius: f32 },
}

impl CutDuplicateSource {
    fn requires_floor_region(&self) -> bool {
        matches!(self, Self::RecessedPanel { .. })
    }

    fn center_offset(&self, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> [f32; 2] {
        let mut offset = [0.0, 0.0];
        let center = self.center();
        let axis = if center[1].abs() > 0.35 {
            1
        } else if center[0].abs() > 0.35 {
            0
        } else {
            rng.random_range(0..2)
        };
        let outward_sign = if center[axis] > 0.10 {
            1.0
        } else if center[axis] < -0.10 {
            -1.0
        } else if rng.random_bool(0.5) {
            1.0
        } else {
            -1.0
        };
        let sign = if matches!(self, Self::RecessedPanel { .. }) {
            -outward_sign
        } else {
            outward_sign
        };
        let base_spacing = match self {
            Self::RecessedPanel { size, .. } | Self::RectangularThrough { size, .. } => {
                (size[axis].abs() * 1.55).max(0.42)
            }
            Self::CircularThrough { radius, .. } => (radius.abs() * 4.20).max(0.42),
        };
        let exploration_scale = match mode {
            AssetCandidateMode::Refine => 1.0,
            AssetCandidateMode::Explore => rng.random_range(0.85..1.70),
        };
        offset[axis] = sign * base_spacing * exploration_scale;
        offset
    }

    fn center(&self) -> [f32; 2] {
        match self {
            Self::RecessedPanel { center, .. }
            | Self::RectangularThrough { center, .. }
            | Self::CircularThrough { center, .. } => *center,
        }
    }
}

#[derive(Debug, Clone)]
struct ProposalIdAllocator {
    next_operation: u64,
    next_region: u64,
    next_boundary_loop: u64,
}

impl ProposalIdAllocator {
    fn from_recipe(recipe: &AssetRecipe) -> Self {
        Self {
            next_operation: recipe.next_ids.operation,
            next_region: recipe.next_ids.region,
            next_boundary_loop: recipe.next_ids.boundary_loop,
        }
    }

    fn operation(&mut self) -> OperationId {
        let id = OperationId(self.next_operation);
        self.next_operation = self.next_operation.saturating_add(1);
        id
    }

    fn region(&mut self) -> RegionId {
        let id = RegionId(self.next_region);
        self.next_region = self.next_region.saturating_add(1);
        id
    }

    fn boundary_loop(&mut self) -> BoundaryLoopId {
        let id = BoundaryLoopId(self.next_boundary_loop);
        self.next_boundary_loop = self.next_boundary_loop.saturating_add(1);
        id
    }
}

impl EditOpportunity {
    fn is_structural_cut_duplicate(&self) -> bool {
        matches!(self, Self::DuplicateCut { .. })
    }

    fn build_edit(
        &self,
        _recipe: &AssetRecipe,
        mode: AssetCandidateMode,
        rng: &mut ChaCha8Rng,
        ids: &mut ProposalIdAllocator,
    ) -> Option<AssetEdit> {
        match self {
            Self::ScalarParameter {
                parameter,
                current,
                range,
            } => Some(AssetEdit::SetScalar {
                parameter: parameter.id,
                value: mutate_scalar(*current, *range, mode, rng)?,
            }),
            Self::Transform { instance, current } => Some(AssetEdit::SetTransform {
                instance: *instance,
                transform: mutate_transform(current, mode, rng),
            }),
            Self::Dimension(target) => target.build_edit(mode, rng),
            Self::BevelRadius {
                definition,
                operation,
                current,
            } => Some(AssetEdit::SetBevelSettings {
                definition: *definition,
                operation: *operation,
                radius: Some(mutate_non_negative(*current, mode, rng)),
                segments: None,
            }),
            Self::SweepProfilePoint {
                definition,
                index,
                current,
            } => {
                let mut point = *current;
                let axis = rng.random_range(0..2);
                point[axis] = if axis == 0 {
                    mutate_non_negative(point[axis], mode, rng)
                } else {
                    mutate_signed(point[axis], mode, rng)
                };
                Some(AssetEdit::SetSweepProfilePoint {
                    definition: *definition,
                    index: *index,
                    point,
                })
            }
            Self::SweepPathFrame {
                definition,
                index,
                current,
            } => {
                let mut frame = current.clone();
                let axis = rng.random_range(0..3);
                frame.origin[axis] = mutate_signed(frame.origin[axis], mode, rng);
                Some(AssetEdit::SetSweepPathFrame {
                    definition: *definition,
                    index: *index,
                    frame,
                })
            }
            Self::LatheProfilePoint {
                definition,
                index,
                current,
            } => {
                let mut point = *current;
                let axis = rng.random_range(0..2);
                point[axis] = if axis == 0 {
                    mutate_non_negative(point[axis], mode, rng)
                } else {
                    mutate_signed(point[axis], mode, rng)
                };
                Some(AssetEdit::SetLatheProfilePoint {
                    definition: *definition,
                    index: *index,
                    point,
                })
            }
            Self::ArrayCount {
                definition,
                operation,
                current,
                range,
            } => Some(AssetEdit::SetArrayCount {
                definition: *definition,
                operation: *operation,
                count: mutate_count(*current, range.minimum, range.maximum, rng)?,
            }),
            Self::LinearArraySpacing {
                definition,
                operation,
                current,
            } => {
                let mut offset = *current;
                let axis = rng.random_range(0..3);
                offset[axis] = mutate_signed(offset[axis], mode, rng);
                Some(AssetEdit::SetArraySpacing {
                    definition: *definition,
                    operation: *operation,
                    spacing: ArraySpacingEdit::LinearOffset(offset),
                })
            }
            Self::RadialArrayAngle {
                definition,
                operation,
                current,
            } => Some(AssetEdit::SetArraySpacing {
                definition: *definition,
                operation: *operation,
                spacing: ArraySpacingEdit::RadialAngleDegrees(mutate_angle(*current, mode, rng)),
            }),
            Self::OptionalPart { instance, current } => Some(AssetEdit::SetOptionalPartEnabled {
                instance: *instance,
                enabled: !*current,
            }),
            Self::Replacement { instance, to, .. } => Some(AssetEdit::ReplaceInstanceDefinition {
                instance: *instance,
                definition: *to,
            }),
            Self::DuplicateCut {
                definition,
                operation,
                source,
            } => Some(AssetEdit::DuplicateCutOperation {
                definition: *definition,
                source: *operation,
                operation: ids.operation(),
                entry_loop: ids.boundary_loop(),
                secondary_loop: ids.boundary_loop(),
                rim_region: ids.region(),
                wall_region: ids.region(),
                floor_region: source.requires_floor_region().then(|| ids.region()),
                center_offset: source.center_offset(mode, rng),
            }),
            Self::DetailDensity(target) => target.build_edit(rng),
        }
    }

    fn kind(&self) -> AssetCandidateEditKind {
        match self {
            Self::ScalarParameter { .. } => AssetCandidateEditKind::Parameter,
            Self::Transform { .. } => AssetCandidateEditKind::Transform,
            Self::Dimension(_) => AssetCandidateEditKind::GeneratorDimension,
            Self::BevelRadius { .. } => AssetCandidateEditKind::Bevel,
            Self::SweepProfilePoint { .. } | Self::SweepPathFrame { .. } => {
                AssetCandidateEditKind::Sweep
            }
            Self::LatheProfilePoint { .. } => AssetCandidateEditKind::Lathe,
            Self::ArrayCount { .. } => AssetCandidateEditKind::ArrayCount,
            Self::LinearArraySpacing { .. } | Self::RadialArrayAngle { .. } => {
                AssetCandidateEditKind::ArraySpacing
            }
            Self::OptionalPart { .. } => AssetCandidateEditKind::OptionalPart,
            Self::Replacement { .. } => AssetCandidateEditKind::Replacement,
            Self::DuplicateCut { .. } => AssetCandidateEditKind::ModelingOperation,
            Self::DetailDensity(_) => AssetCandidateEditKind::DetailDensity,
        }
    }

    fn subject(&self) -> String {
        match self {
            Self::ScalarParameter { parameter, .. } => format!("parameter.{}", parameter.id.0),
            Self::Transform { instance, .. } => format!("instance.{}.transform", instance.0),
            Self::Dimension(target) => target.subject(),
            Self::BevelRadius {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.bevel.radius",
                definition.0, operation.0
            ),
            Self::SweepProfilePoint {
                definition, index, ..
            } => format!("definition.{}.geometry.sweep.profile.{index}", definition.0),
            Self::SweepPathFrame {
                definition, index, ..
            } => format!("definition.{}.geometry.sweep.path.{index}", definition.0),
            Self::LatheProfilePoint {
                definition, index, ..
            } => format!("definition.{}.geometry.lathe.profile.{index}", definition.0),
            Self::ArrayCount {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.array.count",
                definition.0, operation.0
            ),
            Self::LinearArraySpacing {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.linear_array.offset",
                definition.0, operation.0
            ),
            Self::RadialArrayAngle {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.radial_array.angle_degrees",
                definition.0, operation.0
            ),
            Self::OptionalPart { instance, .. } => format!("instance.{}.optional", instance.0),
            Self::Replacement { instance, .. } => format!("instance.{}.definition", instance.0),
            Self::DuplicateCut {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.duplicate",
                definition.0, operation.0
            ),
            Self::DetailDensity(target) => target.subject(),
        }
    }

    fn explore_priority(&self) -> u8 {
        match self.kind() {
            AssetCandidateEditKind::OptionalPart => 0,
            AssetCandidateEditKind::Replacement => 1,
            AssetCandidateEditKind::ModelingOperation => 2,
            AssetCandidateEditKind::ArrayCount => 3,
            AssetCandidateEditKind::DetailDensity => 4,
            AssetCandidateEditKind::ArraySpacing => 5,
            AssetCandidateEditKind::Transform => 6,
            AssetCandidateEditKind::GeneratorDimension => 7,
            AssetCandidateEditKind::Bevel => 8,
            AssetCandidateEditKind::Sweep => 9,
            AssetCandidateEditKind::Lathe => 10,
            AssetCandidateEditKind::Parameter => 11,
        }
    }

    fn refine_priority(&self) -> u8 {
        match self.kind() {
            AssetCandidateEditKind::Parameter => 0,
            AssetCandidateEditKind::Transform => 1,
            AssetCandidateEditKind::GeneratorDimension => 2,
            AssetCandidateEditKind::Bevel => 3,
            AssetCandidateEditKind::Sweep => 4,
            AssetCandidateEditKind::Lathe => 5,
            AssetCandidateEditKind::ArraySpacing => 6,
            AssetCandidateEditKind::ArrayCount
            | AssetCandidateEditKind::OptionalPart
            | AssetCandidateEditKind::Replacement
            | AssetCandidateEditKind::DetailDensity
            | AssetCandidateEditKind::ModelingOperation => 7,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct ParameterRange {
    minimum: f32,
    maximum: f32,
    step: f32,
    mutation_sigma: f32,
}

#[derive(Debug, Clone)]
enum DefinitionDimensionTarget {
    RoundedBoxHalfExtents {
        definition: PartDefinitionId,
        current: [f32; 3],
    },
    RoundedBoxRadius {
        definition: PartDefinitionId,
        current: f32,
    },
    CylinderRadius {
        definition: PartDefinitionId,
        current: f32,
    },
    CylinderHeight {
        definition: PartDefinitionId,
        current: f32,
    },
    FrustumBottomRadius {
        definition: PartDefinitionId,
        current: f32,
    },
    FrustumTopRadius {
        definition: PartDefinitionId,
        current: f32,
    },
    FrustumHeight {
        definition: PartDefinitionId,
        current: f32,
    },
    PlateSize {
        definition: PartDefinitionId,
        current: [f32; 2],
    },
    PlateThickness {
        definition: PartDefinitionId,
        current: f32,
    },
}

impl DefinitionDimensionTarget {
    fn build_edit(&self, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> Option<AssetEdit> {
        match self {
            Self::RoundedBoxHalfExtents {
                definition,
                current,
            } => {
                let mut value = *current;
                let axis = rng.random_range(0..3);
                value[axis] = mutate_positive(value[axis], mode, rng);
                Some(AssetEdit::SetGeneratorDimension {
                    definition: *definition,
                    dimension: GeneratorDimensionEdit::RoundedBoxHalfExtents(value),
                })
            }
            Self::RoundedBoxRadius {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::RoundedBoxRadius(mutate_non_negative(
                    *current, mode, rng,
                )),
            }),
            Self::CylinderRadius {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::CylinderRadius(mutate_positive(
                    *current, mode, rng,
                )),
            }),
            Self::CylinderHeight {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::CylinderHeight(mutate_positive(
                    *current, mode, rng,
                )),
            }),
            Self::FrustumBottomRadius {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::FrustumBottomRadius(mutate_non_negative(
                    *current, mode, rng,
                )),
            }),
            Self::FrustumTopRadius {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::FrustumTopRadius(mutate_non_negative(
                    *current, mode, rng,
                )),
            }),
            Self::FrustumHeight {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::FrustumHeight(mutate_positive(
                    *current, mode, rng,
                )),
            }),
            Self::PlateSize {
                definition,
                current,
            } => {
                let mut value = *current;
                let axis = rng.random_range(0..2);
                value[axis] = mutate_positive(value[axis], mode, rng);
                Some(AssetEdit::SetGeneratorDimension {
                    definition: *definition,
                    dimension: GeneratorDimensionEdit::PlateSize(value),
                })
            }
            Self::PlateThickness {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::PlateThickness(mutate_positive(
                    *current, mode, rng,
                )),
            }),
        }
    }

    fn subject(&self) -> String {
        match self {
            Self::RoundedBoxHalfExtents { definition, .. } => {
                format!(
                    "definition.{}.geometry.rounded_box.half_extents",
                    definition.0
                )
            }
            Self::RoundedBoxRadius { definition, .. } => {
                format!("definition.{}.geometry.rounded_box.radius", definition.0)
            }
            Self::CylinderRadius { definition, .. } => {
                format!("definition.{}.geometry.cylinder.radius", definition.0)
            }
            Self::CylinderHeight { definition, .. } => {
                format!("definition.{}.geometry.cylinder.height", definition.0)
            }
            Self::FrustumBottomRadius { definition, .. } => {
                format!("definition.{}.geometry.frustum.bottom_radius", definition.0)
            }
            Self::FrustumTopRadius { definition, .. } => {
                format!("definition.{}.geometry.frustum.top_radius", definition.0)
            }
            Self::FrustumHeight { definition, .. } => {
                format!("definition.{}.geometry.frustum.height", definition.0)
            }
            Self::PlateSize { definition, .. } => {
                format!("definition.{}.geometry.plate.size", definition.0)
            }
            Self::PlateThickness { definition, .. } => {
                format!("definition.{}.geometry.plate.thickness", definition.0)
            }
        }
    }
}

#[derive(Debug, Clone)]
enum DetailDensityTarget {
    CylinderRadialDetail {
        definition: PartDefinitionId,
        current: u32,
    },
    FrustumRadialDetail {
        definition: PartDefinitionId,
        current: u32,
    },
    LatheSegmentCount {
        definition: PartDefinitionId,
        current: u32,
    },
    BevelSegmentCount {
        definition: PartDefinitionId,
        operation: OperationId,
        current: u32,
    },
}

impl DetailDensityTarget {
    fn build_edit(&self, rng: &mut ChaCha8Rng) -> Option<AssetEdit> {
        match self {
            Self::CylinderRadialDetail {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::CylinderRadialSegments(mutate_detail_count(
                    *current, 3, rng,
                )?),
            }),
            Self::FrustumRadialDetail {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::FrustumRadialSegments(mutate_detail_count(
                    *current, 3, rng,
                )?),
            }),
            Self::LatheSegmentCount {
                definition,
                current,
            } => Some(AssetEdit::SetGeneratorDimension {
                definition: *definition,
                dimension: GeneratorDimensionEdit::LatheSegments(mutate_detail_count(
                    *current, 3, rng,
                )?),
            }),
            Self::BevelSegmentCount {
                definition,
                operation,
                current,
            } => Some(AssetEdit::SetBevelSettings {
                definition: *definition,
                operation: *operation,
                radius: None,
                segments: Some(mutate_detail_count(*current, 1, rng)?),
            }),
        }
    }

    fn subject(&self) -> String {
        match self {
            Self::CylinderRadialDetail { definition, .. } => {
                format!(
                    "definition.{}.geometry.cylinder.radial_segments",
                    definition.0
                )
            }
            Self::FrustumRadialDetail { definition, .. } => {
                format!(
                    "definition.{}.geometry.frustum.radial_segments",
                    definition.0
                )
            }
            Self::LatheSegmentCount { definition, .. } => {
                format!("definition.{}.geometry.lathe.segments", definition.0)
            }
            Self::BevelSegmentCount {
                definition,
                operation,
                ..
            } => format!(
                "definition.{}.operation.{}.bevel.segments",
                definition.0, operation.0
            ),
        }
    }
}

fn sort_opportunities(opportunities: &mut [EditOpportunity], mode: AssetCandidateMode) {
    opportunities.sort_by_key(|opportunity| {
        let priority = match mode {
            AssetCandidateMode::Refine => opportunity.refine_priority(),
            AssetCandidateMode::Explore => opportunity.explore_priority(),
        };
        (priority, opportunity.subject())
    });
}

fn effective_parameter_range(
    recipe: &AssetRecipe,
    parameter: &ParameterDescriptor,
    current: f32,
) -> Option<ParameterRange> {
    let override_range = recipe
        .variation
        .parameter_range_overrides
        .get(&parameter.id);
    let minimum = override_range
        .map(|range| range.minimum.max(parameter.minimum))
        .unwrap_or(parameter.minimum);
    let maximum = override_range
        .map(|range| range.maximum.min(parameter.maximum))
        .unwrap_or(parameter.maximum);
    if !(minimum.is_finite() && maximum.is_finite() && minimum < maximum) {
        return None;
    }
    if current < minimum || current > maximum {
        return None;
    }
    let step = override_range
        .and_then(|range| range.step)
        .unwrap_or(parameter.step)
        .max(f32::EPSILON);
    let mutation_sigma = override_range
        .and_then(|range| range.mutation_sigma)
        .unwrap_or(parameter.mutation_sigma)
        .max(step);
    Some(ParameterRange {
        minimum,
        maximum,
        step,
        mutation_sigma,
    })
}

fn mutate_scalar(
    current: f32,
    range: ParameterRange,
    mode: AssetCandidateMode,
    rng: &mut ChaCha8Rng,
) -> Option<f32> {
    if range.minimum >= range.maximum {
        return None;
    }
    let scale = match mode {
        AssetCandidateMode::Refine => 0.85,
        AssetCandidateMode::Explore => 2.50,
    };
    let direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    let magnitude = rng.random_range(0.6..=1.4) * range.mutation_sigma * scale;
    let mut value = current + direction * magnitude.max(range.step);
    if mode == AssetCandidateMode::Explore && rng.random_bool(0.25) {
        value = rng.random_range(range.minimum..=range.maximum);
    }
    value = snap_to_step(
        value.clamp(range.minimum, range.maximum),
        range.minimum,
        range.step,
    );
    if (value - current).abs() <= f32::EPSILON {
        value = (current + direction * range.step).clamp(range.minimum, range.maximum);
    }
    if (value - current).abs() <= f32::EPSILON {
        None
    } else {
        Some(value)
    }
}

fn mutate_transform(
    current: &Transform3,
    mode: AssetCandidateMode,
    rng: &mut ChaCha8Rng,
) -> Transform3 {
    let mut transform = current.clone();
    let scale = match mode {
        AssetCandidateMode::Refine => 1.0,
        AssetCandidateMode::Explore => 3.0,
    };
    let first = rng.random_range(0..3);
    mutate_transform_channel(&mut transform, first, scale, rng);
    if mode == AssetCandidateMode::Explore {
        let second = rng.random_range(0..3);
        mutate_transform_channel(&mut transform, second, scale, rng);
    }
    transform
}

fn mutate_transform_channel(
    transform: &mut Transform3,
    channel: usize,
    scale: f32,
    rng: &mut ChaCha8Rng,
) {
    let axis = rng.random_range(0..3);
    let sign = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    match channel {
        0 => transform.translation[axis] += sign * rng.random_range(0.04..=0.16) * scale,
        1 => transform.rotation_degrees[axis] += sign * rng.random_range(4.0..=14.0) * scale,
        _ => {
            let factor = 1.0 + sign * rng.random_range(0.035..=0.10) * scale;
            transform.scale[axis] = (transform.scale[axis] * factor).clamp(0.20, 5.0);
        }
    }
}

fn mutate_positive(current: f32, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> f32 {
    mutate_non_negative(current, mode, rng).max(0.001)
}

fn mutate_non_negative(current: f32, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> f32 {
    let value = mutate_signed(current, mode, rng);
    value.max(0.0)
}

fn mutate_signed(current: f32, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> f32 {
    let scale = match mode {
        AssetCandidateMode::Refine => 0.12,
        AssetCandidateMode::Explore => 0.38,
    };
    let base = current.abs().max(0.08);
    let direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    current + direction * base * rng.random_range(0.25..=1.0) * scale
}

fn mutate_angle(current: f32, mode: AssetCandidateMode, rng: &mut ChaCha8Rng) -> f32 {
    let limit = match mode {
        AssetCandidateMode::Refine => 8.0,
        AssetCandidateMode::Explore => 35.0,
    };
    let direction = if rng.random_bool(0.5) { 1.0 } else { -1.0 };
    current + direction * rng.random_range(3.0..=limit)
}

fn mutate_count(current: u32, minimum: u32, maximum: u32, rng: &mut ChaCha8Rng) -> Option<u32> {
    if minimum >= maximum {
        return None;
    }
    let mut value = rng.random_range(minimum..=maximum);
    if value == current {
        value = if value < maximum {
            value + 1
        } else {
            value - 1
        };
    }
    Some(value)
}

fn mutate_detail_count(current: u32, minimum: u32, rng: &mut ChaCha8Rng) -> Option<u32> {
    let lower = minimum.max(current.saturating_sub(8));
    let upper = current.saturating_add(8).min(128).max(lower);
    mutate_count(current, lower, upper, rng)
}

fn snap_to_step(value: f32, minimum: f32, step: f32) -> f32 {
    if !step.is_finite() || step <= 0.0 {
        return value;
    }
    minimum + ((value - minimum) / step).round() * step
}

fn diagnose_change(
    before: &AssetRecipe,
    after: &AssetRecipe,
    operation: &AssetEdit,
) -> AssetChangeDiagnostic {
    let kind = diagnostic_kind(before, operation);
    let subject = edit_subject(operation);
    let topology_changing = edit_is_topology_changing(before, operation);
    let (before_value, after_value, message) = match operation {
        AssetEdit::SetScalar { parameter, .. } => {
            let descriptor = before.parameters.get(parameter);
            let path = descriptor
                .map(|descriptor| descriptor.path.as_str())
                .unwrap_or("<unknown>");
            let label = descriptor
                .map(|descriptor| descriptor.label.as_str())
                .unwrap_or("parameter");
            (
                scalar_summary(before, path),
                scalar_summary(after, path),
                format!("changed descriptor-backed parameter '{label}'"),
            )
        }
        AssetEdit::SetTransform { instance, .. } => (
            before
                .instances
                .get(instance)
                .map(|instance| transform_summary(&instance.local_transform))
                .unwrap_or_else(|| "<missing>".to_owned()),
            after
                .instances
                .get(instance)
                .map(|instance| transform_summary(&instance.local_transform))
                .unwrap_or_else(|| "<missing>".to_owned()),
            "changed part instance transform".to_owned(),
        ),
        AssetEdit::SetOptionalPartEnabled { instance, .. }
        | AssetEdit::SetEnabled { instance, .. } => (
            before
                .instances
                .get(instance)
                .map(|instance| instance.enabled.to_string())
                .unwrap_or_else(|| "<missing>".to_owned()),
            after
                .instances
                .get(instance)
                .map(|instance| instance.enabled.to_string())
                .unwrap_or_else(|| "<missing>".to_owned()),
            "changed optional part presence".to_owned(),
        ),
        AssetEdit::ReplaceInstanceDefinition { instance, .. } => (
            before
                .instances
                .get(instance)
                .map(|instance| format!("definition.{}", instance.definition.0))
                .unwrap_or_else(|| "<missing>".to_owned()),
            after
                .instances
                .get(instance)
                .map(|instance| format!("definition.{}", instance.definition.0))
                .unwrap_or_else(|| "<missing>".to_owned()),
            "replaced instance with a compatible part definition".to_owned(),
        ),
        AssetEdit::SetGeneratorDimension { dimension, .. } => (
            "generator dimension".to_owned(),
            generator_dimension_summary(dimension),
            "changed generator dimension or detail density".to_owned(),
        ),
        AssetEdit::SetBevelSettings {
            radius, segments, ..
        } => (
            "bevel settings".to_owned(),
            format!("radius={radius:?}; segments={segments:?}"),
            "changed bevel settings".to_owned(),
        ),
        AssetEdit::SetSweepProfilePoint { point, .. } => (
            "sweep profile point".to_owned(),
            format_array2(*point),
            "changed sweep thickness/profile".to_owned(),
        ),
        AssetEdit::SetSweepPathFrame { frame, .. } => (
            "sweep path frame".to_owned(),
            transform_point_summary(&frame.origin),
            "changed sweep path curvature".to_owned(),
        ),
        AssetEdit::SetLatheProfilePoint { point, .. } => (
            "lathe profile point".to_owned(),
            format_array2(*point),
            "changed lathe profile parameter".to_owned(),
        ),
        AssetEdit::SetArrayCount { count, .. } => (
            "array count".to_owned(),
            count.to_string(),
            "changed authored array count".to_owned(),
        ),
        AssetEdit::SetArraySpacing { spacing, .. } => (
            "array spacing".to_owned(),
            array_spacing_summary(spacing),
            "changed array spacing".to_owned(),
        ),
        AssetEdit::InsertModelingOperation {
            operation, index, ..
        } => (
            "operation list".to_owned(),
            format!("insert {} at {index}", operation_label(operation)),
            "inserted a local modeling operation".to_owned(),
        ),
        AssetEdit::RemoveModelingOperation { operation, .. } => (
            format!("operation.{}", operation.0),
            "removed".to_owned(),
            "removed a local modeling operation".to_owned(),
        ),
        AssetEdit::DuplicateCutOperation {
            source,
            operation,
            center_offset,
            ..
        } => (
            format!("operation.{}", source.0),
            format!(
                "duplicate operation.{} offset={}",
                operation.0,
                format_array2(*center_offset)
            ),
            "duplicated a semantic cut operation".to_owned(),
        ),
        AssetEdit::MoveModelingOperation {
            operation,
            new_index,
            ..
        } => (
            format!("operation.{}", operation.0),
            format!("index {new_index}"),
            "moved a local modeling operation".to_owned(),
        ),
        _ => (
            "semantic edit".to_owned(),
            format!("{operation:?}"),
            "changed asset recipe structure".to_owned(),
        ),
    };
    AssetChangeDiagnostic {
        kind,
        subject,
        before: before_value,
        after: after_value,
        message,
        topology_changing,
    }
}

fn diagnostic_kind(recipe: &AssetRecipe, operation: &AssetEdit) -> AssetCandidateEditKind {
    match operation {
        AssetEdit::SetScalar { .. } => AssetCandidateEditKind::Parameter,
        AssetEdit::SetTransform { .. } => AssetCandidateEditKind::Transform,
        AssetEdit::SetGeneratorDimension { dimension, .. } => {
            if dimension.topology_changing() {
                AssetCandidateEditKind::DetailDensity
            } else {
                AssetCandidateEditKind::GeneratorDimension
            }
        }
        AssetEdit::SetBevelSettings { segments, .. } => {
            if segments.is_some() {
                AssetCandidateEditKind::DetailDensity
            } else {
                AssetCandidateEditKind::Bevel
            }
        }
        AssetEdit::SetSweepProfilePoint { .. } | AssetEdit::SetSweepPathFrame { .. } => {
            AssetCandidateEditKind::Sweep
        }
        AssetEdit::SetLatheProfilePoint { .. } => AssetCandidateEditKind::Lathe,
        AssetEdit::SetArrayCount { .. } => AssetCandidateEditKind::ArrayCount,
        AssetEdit::SetArraySpacing { .. } => AssetCandidateEditKind::ArraySpacing,
        AssetEdit::SetOptionalPartEnabled { .. } | AssetEdit::SetEnabled { .. } => {
            AssetCandidateEditKind::OptionalPart
        }
        AssetEdit::ReplaceInstanceDefinition { .. } => AssetCandidateEditKind::Replacement,
        AssetEdit::InsertModelingOperation { .. }
        | AssetEdit::RemoveModelingOperation { .. }
        | AssetEdit::DuplicateCutOperation { .. }
        | AssetEdit::MoveModelingOperation { .. }
        | AssetEdit::SetOperationScalar { .. } => AssetCandidateEditKind::ModelingOperation,
        AssetEdit::ReplaceDefinition { definition } => {
            if recipe
                .definitions
                .get(&definition.id)
                .is_some_and(|existing| existing.geometry.source != definition.geometry.source)
            {
                AssetCandidateEditKind::Replacement
            } else {
                AssetCandidateEditKind::GeneratorDimension
            }
        }
        _ => AssetCandidateEditKind::GeneratorDimension,
    }
}

fn edit_is_topology_changing(recipe: &AssetRecipe, operation: &AssetEdit) -> bool {
    match operation {
        AssetEdit::SetScalar { parameter, .. } => recipe
            .parameters
            .get(parameter)
            .is_some_and(|parameter| parameter.topology_changing),
        AssetEdit::SetOperationScalar { .. } => true,
        AssetEdit::SetGeneratorDimension { dimension, .. } => dimension.topology_changing(),
        AssetEdit::SetBevelSettings { segments, .. } => segments.is_some(),
        AssetEdit::SetOptionalPartEnabled { .. }
        | AssetEdit::ReplaceGeometrySource { .. }
        | AssetEdit::AddInstance { .. }
        | AssetEdit::RemoveInstance { .. }
        | AssetEdit::ReplaceInstanceDefinition { .. }
        | AssetEdit::InsertModelingOperation { .. }
        | AssetEdit::RemoveModelingOperation { .. }
        | AssetEdit::DuplicateCutOperation { .. }
        | AssetEdit::MoveModelingOperation { .. }
        | AssetEdit::SetArrayCount { .. }
        | AssetEdit::DuplicateInstance { .. }
        | AssetEdit::MirrorInstance { .. }
        | AssetEdit::Attach { .. }
        | AssetEdit::Detach { .. } => true,
        AssetEdit::SetTransform { .. }
        | AssetEdit::SetEnabled { .. }
        | AssetEdit::SetSweepProfilePoint { .. }
        | AssetEdit::SetSweepPathFrame { .. }
        | AssetEdit::SetLatheProfilePoint { .. }
        | AssetEdit::SetArraySpacing { .. }
        | AssetEdit::SetLock { .. }
        | AssetEdit::SetInstanceLock { .. }
        | AssetEdit::SetSubtreeLock { .. }
        | AssetEdit::SetTopologyLock { .. }
        | AssetEdit::ReorderChildInstances { .. } => false,
        AssetEdit::ReplaceDefinition { definition } => recipe
            .definitions
            .get(&definition.id)
            .is_some_and(|existing| existing.geometry != definition.geometry),
    }
}

fn edit_subject(operation: &AssetEdit) -> String {
    match operation {
        AssetEdit::SetScalar { parameter, .. } => format!("parameter.{}", parameter.0),
        AssetEdit::SetTransform { instance, .. }
        | AssetEdit::SetEnabled { instance, .. }
        | AssetEdit::SetOptionalPartEnabled { instance, .. }
        | AssetEdit::ReplaceInstanceDefinition { instance, .. } => {
            format!("instance.{}", instance.0)
        }
        AssetEdit::SetGeneratorDimension { definition, .. }
        | AssetEdit::SetOperationScalar { definition, .. }
        | AssetEdit::ReplaceGeometrySource { definition, .. }
        | AssetEdit::SetBevelSettings { definition, .. }
        | AssetEdit::SetSweepProfilePoint { definition, .. }
        | AssetEdit::SetSweepPathFrame { definition, .. }
        | AssetEdit::SetLatheProfilePoint { definition, .. }
        | AssetEdit::InsertModelingOperation { definition, .. }
        | AssetEdit::RemoveModelingOperation { definition, .. }
        | AssetEdit::DuplicateCutOperation { definition, .. }
        | AssetEdit::MoveModelingOperation { definition, .. }
        | AssetEdit::SetArrayCount { definition, .. }
        | AssetEdit::SetArraySpacing { definition, .. } => format!("definition.{}", definition.0),
        AssetEdit::ReplaceDefinition { definition } => format!("definition.{}", definition.id.0),
        AssetEdit::AddInstance { instance } => format!("instance.{}", instance.id.0),
        AssetEdit::RemoveInstance { instance }
        | AssetEdit::DuplicateInstance { instance, .. }
        | AssetEdit::MirrorInstance { instance, .. }
        | AssetEdit::Attach { instance, .. }
        | AssetEdit::Detach { instance }
        | AssetEdit::SetInstanceLock { instance, .. }
        | AssetEdit::SetSubtreeLock { instance, .. } => format!("instance.{}", instance.0),
        AssetEdit::SetLock { parameter, .. } => format!("parameter.{}", parameter.0),
        AssetEdit::SetTopologyLock { definition, .. } => format!("definition.{}", definition.0),
        AssetEdit::ReorderChildInstances { parent, .. } => parent
            .map(|parent| format!("instance.{}", parent.0))
            .unwrap_or_else(|| "root_instances".to_owned()),
    }
}

fn operation_label(operation: &ModelingOperationSpec) -> &'static str {
    match operation {
        ModelingOperationSpec::TransformGeometry { .. } => "transform",
        ModelingOperationSpec::SetBevelProfile { .. } => "bevel",
        ModelingOperationSpec::AddPanel { .. } => "panel",
        ModelingOperationSpec::AddTrim { .. } => "trim",
        ModelingOperationSpec::RecessedPanelCut { .. } => "recessed panel cut",
        ModelingOperationSpec::RectangularThroughCut { .. } => "rectangular through cut",
        ModelingOperationSpec::CircularThroughCut { .. } => "circular through cut",
        ModelingOperationSpec::MirrorInstances { .. } => "mirror",
        ModelingOperationSpec::LinearArray { .. } => "linear array",
        ModelingOperationSpec::RadialArray { .. } => "radial array",
        ModelingOperationSpec::ReservedBoolean { .. } => "reserved boolean",
        ModelingOperationSpec::ReservedDeformationProgram { .. } => "reserved deformation",
    }
}

fn scalar_summary(recipe: &AssetRecipe, path: &str) -> String {
    get_scalar(recipe, path)
        .map(|value| format!("{value:.4}"))
        .unwrap_or_else(|_| "<unavailable>".to_owned())
}

fn transform_summary(transform: &Transform3) -> String {
    format!(
        "t={}; r={}; s={}",
        transform_point_summary(&transform.translation),
        transform_point_summary(&transform.rotation_degrees),
        transform_point_summary(&transform.scale)
    )
}

fn transform_point_summary(value: &[f32; 3]) -> String {
    format!("[{:.4}, {:.4}, {:.4}]", value[0], value[1], value[2])
}

fn format_array2(value: [f32; 2]) -> String {
    format!("[{:.4}, {:.4}]", value[0], value[1])
}

fn generator_dimension_summary(dimension: &GeneratorDimensionEdit) -> String {
    match dimension {
        GeneratorDimensionEdit::RoundedBoxHalfExtents(value) => {
            format!(
                "rounded_box.half_extents={}",
                transform_point_summary(value)
            )
        }
        GeneratorDimensionEdit::RoundedBoxRadius(value) => format!("rounded_box.radius={value:.4}"),
        GeneratorDimensionEdit::CylinderRadius(value) => format!("cylinder.radius={value:.4}"),
        GeneratorDimensionEdit::CylinderHeight(value) => format!("cylinder.height={value:.4}"),
        GeneratorDimensionEdit::CylinderRadialSegments(value) => {
            format!("cylinder.radial_segments={value}")
        }
        GeneratorDimensionEdit::FrustumBottomRadius(value) => {
            format!("frustum.bottom_radius={value:.4}")
        }
        GeneratorDimensionEdit::FrustumTopRadius(value) => {
            format!("frustum.top_radius={value:.4}")
        }
        GeneratorDimensionEdit::FrustumHeight(value) => format!("frustum.height={value:.4}"),
        GeneratorDimensionEdit::FrustumRadialSegments(value) => {
            format!("frustum.radial_segments={value}")
        }
        GeneratorDimensionEdit::PlateSize(value) => format!("plate.size={}", format_array2(*value)),
        GeneratorDimensionEdit::PlateThickness(value) => format!("plate.thickness={value:.4}"),
        GeneratorDimensionEdit::LatheSegments(value) => format!("lathe.segments={value}"),
    }
}

fn array_spacing_summary(spacing: &ArraySpacingEdit) -> String {
    match spacing {
        ArraySpacingEdit::LinearOffset(value) => {
            format!("linear.offset={}", transform_point_summary(value))
        }
        ArraySpacingEdit::RadialAxis(value) => {
            format!("radial.axis={}", transform_point_summary(value))
        }
        ArraySpacingEdit::RadialAngleDegrees(value) => format!("radial.angle_degrees={value:.4}"),
    }
}

fn instance_is_editable(recipe: &AssetRecipe, instance: PartInstanceId) -> bool {
    if recipe.instance_locks.contains(&instance) {
        return false;
    }
    !recipe
        .subtree_locks
        .iter()
        .any(|root| *root == instance || instance_is_descendant_of(recipe, instance, *root))
}

fn instance_is_descendant_of(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
    root: PartInstanceId,
) -> bool {
    let mut cursor = recipe.instances.get(&instance).and_then(|item| item.parent);
    while let Some(parent) = cursor {
        if parent == root {
            return true;
        }
        cursor = recipe.instances.get(&parent).and_then(|item| item.parent);
    }
    false
}

fn definition_id_from_path(path: &str) -> Option<PartDefinitionId> {
    let mut parts = path.split('.');
    match (parts.next(), parts.next()) {
        (Some("definition"), Some(raw)) => raw.parse::<u64>().ok().map(PartDefinitionId),
        _ => None,
    }
}

fn instance_id_from_path(path: &str) -> Option<PartInstanceId> {
    let mut parts = path.split('.');
    match (parts.next(), parts.next()) {
        (Some("instance"), Some(raw)) => raw.parse::<u64>().ok().map(PartInstanceId),
        _ => None,
    }
}

fn proposal_seed(seed: u64, proposal_index: u64) -> u64 {
    let mut value = seed ^ proposal_index.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

fn stable_candidate_id(seed: u64, proposal_index: u64) -> u64 {
    proposal_seed(seed ^ 0xa5a5_5a5a_1234_5678, proposal_index)
}

fn increment_rejection(
    diagnostics: &mut AssetCandidateGenerationDiagnostics,
    reason: AssetCandidateRejectionReason,
) {
    *diagnostics.rejections.entry(reason).or_insert(0) += 1;
}
