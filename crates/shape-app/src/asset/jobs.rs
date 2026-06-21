//! Background job contracts for explicit asset authoring.

#![allow(dead_code)]

use std::collections::BTreeSet;
use std::path::PathBuf;

use shape_asset::{
    AssetEdit, AssetEditProgram, AssetRecipe, ParameterId, RevisionId, apply_edit_program,
    get_scalar,
};
use shape_compile::{
    AssetArtifact, CompileValidationReport, ConstructionTimelineReport,
    build_construction_timeline_report, compile_asset,
};
use shape_core::Aabb;
use shape_mesh::TriangleMesh;
use shape_render::{OrbitCamera, RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh};

const DEFAULT_GENERATED_CANDIDATES: usize = 4;

/// Monotonic app job identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AssetJobId(pub u64);

/// Monotonic candidate generation identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AssetGenerationId(pub u64);

/// Stable candidate identifier within the app state.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AssetCandidateId(pub u64);

/// Active-job buckets used by reducers to reject stale results by kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum AssetJobSlot {
    CompileCurrentAsset,
    RenderCurrentPreview,
    GenerateCandidates,
    CompileCandidatePreviews,
    ExportPackage,
}

/// Candidate generation mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AssetGenerationMode {
    Refine,
    Explore,
}

/// Requested output policy for a background job.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum AssetOutputPolicy {
    PreviewOnly,
    PackageExport,
    PreviewAndPackage,
}

/// Optional validation budgets carried with deterministic job requests.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct AssetValidationLimits {
    pub max_issues: usize,
}

/// Candidate recipe snapshot generated off the UI thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCandidate {
    pub id: AssetCandidateId,
    pub slot: usize,
    pub label: String,
    pub recipe: AssetRecipe,
    pub changed_parameters: BTreeSet<ParameterId>,
}

/// Compact validation summary suitable for candidate grids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssetValidationSummary {
    pub valid: bool,
    pub issue_count: usize,
}

/// Lightweight preview and topology summary for a candidate.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCandidatePreview {
    pub candidate_id: AssetCandidateId,
    pub recipe_revision: RevisionId,
    pub thumbnail_rgba: Vec<u8>,
    pub thumbnail_width: u32,
    pub thumbnail_height: u32,
    pub camera: OrbitCamera,
    pub part_count: u64,
    pub triangle_count: u64,
    pub region_count: u64,
    pub validation_summary: AssetValidationSummary,
    pub changed_parameters: BTreeSet<ParameterId>,
    pub construction_timeline_summary: Vec<String>,
}

/// Current asset render preview.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetPreview {
    pub recipe_revision: RevisionId,
    pub mesh: TriangleMesh,
    pub image: RenderedImage,
    pub camera: OrbitCamera,
}

/// Full compile output plus timeline for the current recipe.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetCompileOutput {
    pub recipe_revision: RevisionId,
    pub artifact: AssetArtifact,
    pub timeline: ConstructionTimelineReport,
}

/// Deterministic background work request.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetJobRequest {
    pub job_id: AssetJobId,
    pub recipe_revision: RevisionId,
    pub kind: AssetJobKind,
    pub recipe: AssetRecipe,
    pub validation_limits: Option<AssetValidationLimits>,
    pub output_policy: AssetOutputPolicy,
}

impl AssetJobRequest {
    pub(crate) fn slot(&self) -> AssetJobSlot {
        self.kind.slot()
    }

    pub(crate) fn generation_id(&self) -> Option<AssetGenerationId> {
        self.kind.generation_id()
    }
}

/// Job kind-specific payloads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetJobKind {
    CompileCurrentAsset,
    RenderCurrentPreview {
        camera: Option<OrbitCamera>,
        render_settings: RenderSettings,
    },
    GenerateCandidates {
        generation_id: AssetGenerationId,
        mode: AssetGenerationMode,
        max_candidates: usize,
    },
    CompileCandidatePreviews {
        generation_id: AssetGenerationId,
        candidates: Vec<AssetCandidate>,
        camera: OrbitCamera,
        render_settings: RenderSettings,
    },
    ExportPackage {
        path: PathBuf,
    },
}

impl AssetJobKind {
    pub(crate) fn slot(&self) -> AssetJobSlot {
        match self {
            Self::CompileCurrentAsset => AssetJobSlot::CompileCurrentAsset,
            Self::RenderCurrentPreview { .. } => AssetJobSlot::RenderCurrentPreview,
            Self::GenerateCandidates { .. } => AssetJobSlot::GenerateCandidates,
            Self::CompileCandidatePreviews { .. } => AssetJobSlot::CompileCandidatePreviews,
            Self::ExportPackage { .. } => AssetJobSlot::ExportPackage,
        }
    }

    pub(crate) fn generation_id(&self) -> Option<AssetGenerationId> {
        match self {
            Self::GenerateCandidates { generation_id, .. }
            | Self::CompileCandidatePreviews { generation_id, .. } => Some(*generation_id),
            Self::CompileCurrentAsset
            | Self::RenderCurrentPreview { .. }
            | Self::ExportPackage { .. } => None,
        }
    }
}

/// Reducible job events. Reducers compare both job and generation IDs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssetJobEvent {
    Queued {
        job_id: AssetJobId,
        slot: AssetJobSlot,
    },
    Started {
        job_id: AssetJobId,
        slot: AssetJobSlot,
    },
    Progress {
        job_id: AssetJobId,
        slot: AssetJobSlot,
        completed: u32,
        total: u32,
    },
    CompileReady {
        job_id: AssetJobId,
        recipe_revision: RevisionId,
        output: AssetCompileOutput,
    },
    PreviewReady {
        job_id: AssetJobId,
        recipe_revision: RevisionId,
        preview: AssetPreview,
    },
    CandidatesReady {
        job_id: AssetJobId,
        generation_id: AssetGenerationId,
        recipe_revision: RevisionId,
        candidates: Vec<AssetCandidate>,
    },
    CandidatePreviewsReady {
        job_id: AssetJobId,
        generation_id: AssetGenerationId,
        recipe_revision: RevisionId,
        previews: Vec<AssetCandidatePreview>,
    },
    ExportPackageReady {
        job_id: AssetJobId,
        recipe_revision: RevisionId,
        path: PathBuf,
        artifact: AssetArtifact,
    },
    Failed {
        job_id: AssetJobId,
        message: String,
    },
    Cancelled {
        job_id: AssetJobId,
    },
}

/// Run one asset job on a worker thread.
pub(crate) fn run_asset_job(request: AssetJobRequest) -> Vec<AssetJobEvent> {
    let slot = request.slot();
    let mut events = vec![
        AssetJobEvent::Started {
            job_id: request.job_id,
            slot,
        },
        AssetJobEvent::Progress {
            job_id: request.job_id,
            slot,
            completed: 0,
            total: 1,
        },
    ];

    let final_event = match &request.kind {
        AssetJobKind::CompileCurrentAsset => compile_current(&request),
        AssetJobKind::RenderCurrentPreview {
            camera,
            render_settings,
        } => render_current(&request, camera.clone(), render_settings),
        AssetJobKind::GenerateCandidates {
            generation_id,
            mode,
            max_candidates,
        } => Ok(AssetJobEvent::CandidatesReady {
            job_id: request.job_id,
            generation_id: *generation_id,
            recipe_revision: request.recipe_revision,
            candidates: generate_candidates(
                &request.recipe,
                *generation_id,
                *mode,
                *max_candidates,
            ),
        }),
        AssetJobKind::CompileCandidatePreviews {
            generation_id,
            candidates,
            camera,
            render_settings,
        } => compile_candidate_previews(
            &request,
            *generation_id,
            candidates,
            camera,
            render_settings,
        ),
        AssetJobKind::ExportPackage { path } => compile_asset(&request.recipe)
            .map(|artifact| AssetJobEvent::ExportPackageReady {
                job_id: request.job_id,
                recipe_revision: request.recipe_revision,
                path: path.clone(),
                artifact,
            })
            .map_err(|error| error.to_string()),
    };

    events.push(match final_event {
        Ok(event) => event,
        Err(message) => AssetJobEvent::Failed {
            job_id: request.job_id,
            message,
        },
    });
    events.push(AssetJobEvent::Progress {
        job_id: request.job_id,
        slot,
        completed: 1,
        total: 1,
    });
    events
}

fn compile_current(request: &AssetJobRequest) -> Result<AssetJobEvent, String> {
    let artifact = compile_asset(&request.recipe).map_err(|error| error.to_string())?;
    let timeline = build_construction_timeline_report(&request.recipe, &artifact);
    Ok(AssetJobEvent::CompileReady {
        job_id: request.job_id,
        recipe_revision: request.recipe_revision,
        output: AssetCompileOutput {
            recipe_revision: request.recipe_revision,
            artifact,
            timeline,
        },
    })
}

fn render_current(
    request: &AssetJobRequest,
    camera: Option<OrbitCamera>,
    render_settings: &RenderSettings,
) -> Result<AssetJobEvent, String> {
    let artifact = compile_asset(&request.recipe).map_err(|error| error.to_string())?;
    let mesh = preview_mesh_from_artifact(&artifact);
    let camera = camera.unwrap_or_else(|| fit_camera_to_bounds(mesh.bounds));
    let image = render_mesh(&mesh, &camera, render_settings).map_err(|error| error.to_string())?;
    Ok(AssetJobEvent::PreviewReady {
        job_id: request.job_id,
        recipe_revision: request.recipe_revision,
        preview: AssetPreview {
            recipe_revision: request.recipe_revision,
            mesh,
            image,
            camera,
        },
    })
}

fn compile_candidate_previews(
    request: &AssetJobRequest,
    generation_id: AssetGenerationId,
    candidates: &[AssetCandidate],
    camera: &OrbitCamera,
    render_settings: &RenderSettings,
) -> Result<AssetJobEvent, String> {
    let mut previews = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let artifact = compile_asset(&candidate.recipe).map_err(|error| error.to_string())?;
        let mesh = preview_mesh_from_artifact(&artifact);
        let image =
            render_mesh(&mesh, camera, render_settings).map_err(|error| error.to_string())?;
        previews.push(candidate_preview_from_artifact(
            request.recipe_revision,
            candidate,
            &artifact,
            image,
            camera.clone(),
        ));
    }
    Ok(AssetJobEvent::CandidatePreviewsReady {
        job_id: request.job_id,
        generation_id,
        recipe_revision: request.recipe_revision,
        previews,
    })
}

fn generate_candidates(
    recipe: &AssetRecipe,
    generation_id: AssetGenerationId,
    mode: AssetGenerationMode,
    max_candidates: usize,
) -> Vec<AssetCandidate> {
    let max_candidates = max_candidates.max(1);
    let mut candidates = Vec::with_capacity(max_candidates.min(DEFAULT_GENERATED_CANDIDATES));
    for parameter in recipe.parameters.values() {
        if candidates.len() >= max_candidates {
            break;
        }
        if recipe.locks.contains(&parameter.id) {
            continue;
        }
        let Ok(before) = get_scalar(recipe, &parameter.path) else {
            continue;
        };
        let magnitude = match mode {
            AssetGenerationMode::Refine => parameter.step.max(parameter.mutation_sigma * 0.35),
            AssetGenerationMode::Explore => parameter.step.max(parameter.mutation_sigma),
        };
        if !magnitude.is_finite() || magnitude <= 0.0 {
            continue;
        }
        let sign = if candidates.len().is_multiple_of(2) {
            1.0
        } else {
            -1.0
        };
        let mut after = (before + magnitude * sign).clamp(parameter.minimum, parameter.maximum);
        if after == before {
            after = (before - magnitude * sign).clamp(parameter.minimum, parameter.maximum);
        }
        if after == before {
            continue;
        }
        let program = AssetEditProgram {
            label: format!("Candidate {}", candidates.len() + 1),
            seed: generation_id.0 + candidates.len() as u64,
            operations: vec![AssetEdit::SetScalar {
                parameter: parameter.id,
                value: after,
            }],
        };
        let Ok(candidate_recipe) = apply_edit_program(recipe, &program) else {
            continue;
        };
        let mut changed_parameters = BTreeSet::new();
        changed_parameters.insert(parameter.id);
        candidates.push(AssetCandidate {
            id: AssetCandidateId(generation_id.0.saturating_mul(1_000) + candidates.len() as u64),
            slot: candidates.len(),
            label: program.label,
            recipe: candidate_recipe,
            changed_parameters,
        });
    }
    candidates
}

pub(crate) fn candidate_preview_from_artifact(
    recipe_revision: RevisionId,
    candidate: &AssetCandidate,
    artifact: &AssetArtifact,
    image: RenderedImage,
    camera: OrbitCamera,
) -> AssetCandidatePreview {
    let timeline = build_construction_timeline_report(&candidate.recipe, artifact);
    let validation_summary = validation_summary(&artifact.validation_report);
    AssetCandidatePreview {
        candidate_id: candidate.id,
        recipe_revision,
        thumbnail_rgba: image.rgba8,
        thumbnail_width: image.width,
        thumbnail_height: image.height,
        camera,
        part_count: artifact.statistics.part_count,
        triangle_count: artifact.statistics.triangle_count,
        region_count: artifact
            .provenance_report
            .part_region_operation_mappings
            .iter()
            .filter(|mapping| mapping.region.is_some())
            .count() as u64,
        validation_summary,
        changed_parameters: candidate.changed_parameters.clone(),
        construction_timeline_summary: timeline
            .stages
            .into_iter()
            .map(|stage| stage.summary)
            .collect(),
    }
}

pub(crate) fn preview_mesh_from_artifact(artifact: &AssetArtifact) -> TriangleMesh {
    let mesh = &artifact.combined_preview.mesh;
    TriangleMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: mesh.bounds.min.into(),
            max: mesh.bounds.max.into(),
        },
    }
}

fn validation_summary(report: &CompileValidationReport) -> AssetValidationSummary {
    AssetValidationSummary {
        valid: report.is_valid(),
        issue_count: report.issues.len(),
    }
}
