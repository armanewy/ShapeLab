//! Background job contracts for the native Foundry surface.

use std::path::PathBuf;
use std::{collections::BTreeMap, fs};

use shape_compile::{
    export::write_model_package,
    validation::{ValidationLimits, validate_model, validation_config_from_recipe_with_limits},
};
use shape_foundry::{
    CandidateLegibilityClass, CandidateVariationMetadata, CandidateVisibleDeltaReport,
    ControlValue, FoundryAssetDocument, FoundryBuildStamp, FoundryCatalogResolver, FoundryCommand,
    FoundryCompilationOutput, FoundryEdit, FoundryPackCompilationOutput, FoundryPackDocument,
    FoundryStyleChangeContext, SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON, SharedProviderPolicy,
    VariationIntent, VariationScope, apply_foundry_command,
    apply_foundry_command_with_style_context, compile_foundry_document, compile_foundry_pack,
    resolve_foundry_catalog,
};
use shape_mesh::TriangleMesh;
use shape_render::foundry::{
    FoundryPreviewBatchRequest, FoundryPreviewCache, FoundryPreviewKind, FoundryPreviewRequest,
    FoundryPreviewResolution, FoundryPreviewVariationMetadata,
    compare_foundry_rendered_visible_delta,
};
use shape_render::{
    Aabb, OrbitCamera, RenderSettings, RenderedImage, clay_readability_render_settings,
    fit_camera_to_bounds, fit_camera_to_bounds_from_angles_around_origin, render_mesh,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRejectionReason,
    FoundryCandidateRequest, generate_foundry_candidate_draft_plans,
};

use super::view_model::{FoundryCandidateCard, FoundryPackView};

const CURRENT_PREVIEW_ID: &str = "current";
const CANDIDATE_PARENT_PREVIEW_ID: &str = "candidate-parent";
const CANDIDATE_PREVIEW_PIXELS: u32 = 512;
const PENDING_CANDIDATE_PREVIEW_MESSAGE: &str = "Preview rendering for this direction.";

/// Deterministic background work requested by the Foundry app state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryJobRequest {
    /// Compile the current semantic source into an exact recipe/artifact.
    CompileCurrent {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot.
        document: Box<FoundryAssetDocument>,
    },
    /// Render a whole-model preview from an already compiled output.
    RenderPreview {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Compiled output to render.
        output: Box<FoundryCompilationOutput>,
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
        /// Optional explicit orbit camera for user-controlled viewport renders.
        camera: Option<OrbitCamera>,
    },
    /// Render a whole-model preview for a transient control value without persisting it.
    PreviewControlValue {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot.
        document: Box<FoundryAssetDocument>,
        /// Control to sample.
        control_id: String,
        /// Transient value to render.
        value: ControlValue,
        /// Requested image width.
        width: u32,
        /// Requested image height.
        height: u32,
    },
    /// Generate candidate directions off the UI thread.
    GenerateCandidates {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot.
        document: Box<FoundryAssetDocument>,
        /// Deterministic search request, including the search mode.
        request: FoundryCandidateRequest,
    },
    /// Render previews and run the card-size legibility gate for generated directions.
    RenderCandidatePreviews {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Source document snapshot used to generate the candidates.
        document: Box<FoundryAssetDocument>,
        /// Deterministic search request, including the search mode.
        request: FoundryCandidateRequest,
        /// Candidate output to preview and filter.
        output: Box<FoundryCandidateOutput>,
    },
    /// Apply a replayable edit and rebuild the resulting document.
    ApplyEdit {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Parent document snapshot.
        document: Box<FoundryAssetDocument>,
        /// Replayable Foundry edit.
        edit: Box<FoundryEdit>,
    },
    /// Compile a family pack.
    CompilePack {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Pack snapshot.
        pack: Box<FoundryPackDocument>,
    },
    /// Compile and export every member in a family pack.
    ExportPack {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Pack snapshot.
        pack: Box<FoundryPackDocument>,
        /// Destination directory. Each member receives a deterministic child folder.
        out_dir: PathBuf,
    },
    /// Export an already compiled model package.
    Export {
        /// Monotonic app-local job ID.
        job_id: u64,
        /// Compiled output to export.
        output: Box<FoundryCompilationOutput>,
        /// Export profile key.
        profile: String,
        /// Destination directory.
        out_dir: PathBuf,
    },
}

/// Active-job buckets used by reducers to reject stale results by kind.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum FoundryJobSlot {
    /// Current document compilation.
    CompileCurrent,
    /// Current document preview render.
    RenderPreview,
    /// Candidate generation.
    GenerateCandidates,
    /// Applying a replayable edit and rebuilding the result.
    ApplyEdit,
    /// Family-pack compilation.
    CompilePack,
    /// Export work.
    Export,
}

impl FoundryJobRequest {
    /// Return the app-local job ID.
    #[must_use]
    pub(crate) fn job_id(&self) -> u64 {
        match self {
            Self::CompileCurrent { job_id, .. }
            | Self::RenderPreview { job_id, .. }
            | Self::PreviewControlValue { job_id, .. }
            | Self::GenerateCandidates { job_id, .. }
            | Self::RenderCandidatePreviews { job_id, .. }
            | Self::ApplyEdit { job_id, .. }
            | Self::CompilePack { job_id, .. }
            | Self::ExportPack { job_id, .. }
            | Self::Export { job_id, .. } => *job_id,
        }
    }

    /// Return the candidate mode for candidate-generation work.
    #[must_use]
    pub(crate) fn candidate_mode(&self) -> Option<FoundryCandidateMode> {
        match self {
            Self::GenerateCandidates { request, .. }
            | Self::RenderCandidatePreviews { request, .. } => Some(request.mode),
            _ => None,
        }
    }

    /// Return the stale-rejection slot for this request.
    #[must_use]
    pub(crate) fn slot(&self) -> FoundryJobSlot {
        match self {
            Self::CompileCurrent { .. } => FoundryJobSlot::CompileCurrent,
            Self::RenderPreview { .. } => FoundryJobSlot::RenderPreview,
            Self::PreviewControlValue { .. } => FoundryJobSlot::RenderPreview,
            Self::GenerateCandidates { .. } | Self::RenderCandidatePreviews { .. } => {
                FoundryJobSlot::GenerateCandidates
            }
            Self::ApplyEdit { .. } => FoundryJobSlot::ApplyEdit,
            Self::CompilePack { .. } => FoundryJobSlot::CompilePack,
            Self::ExportPack { .. } => FoundryJobSlot::Export,
            Self::Export { .. } => FoundryJobSlot::Export,
        }
    }
}

/// Result event emitted by a Foundry background worker.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FoundryJobEvent {
    /// Compilation completed.
    CompileFinished {
        /// Matching app-local job ID.
        job_id: u64,
        /// Exact compilation output.
        output: Box<FoundryCompilationOutput>,
    },
    /// A preview image was rendered.
    PreviewRendered {
        /// Matching app-local job ID.
        job_id: u64,
        /// Stable preview slot ID.
        preview_id: String,
        /// RGBA8 image bytes.
        rgba8: Vec<u8>,
        /// Image width.
        width: u32,
        /// Image height.
        height: u32,
        /// Camera used for the preview.
        camera: OrbitCamera,
        /// Build represented by this preview.
        build: Option<FoundryBuildStamp>,
    },
    /// Candidate search completed.
    CandidatesGenerated {
        /// Matching app-local job ID.
        job_id: u64,
        /// Originating search request, including the search mode.
        request: FoundryCandidateRequest,
        /// Search output and diagnostics.
        output: Box<FoundryCandidateOutput>,
        /// UI-ready candidate cards.
        cards: Vec<FoundryCandidateCard>,
    },
    /// Candidate card previews completed and visually weak directions were filtered.
    CandidatePreviewsRendered {
        /// Matching app-local job ID.
        job_id: u64,
        /// Originating search request, including the search mode.
        request: FoundryCandidateRequest,
        /// UI-ready candidate cards after the legibility gate.
        cards: Vec<FoundryCandidateCard>,
        /// Number of generated cards rejected by the preview legibility gate.
        rejected_count: usize,
    },
    /// A Foundry edit was applied.
    EditApplied {
        /// Matching app-local job ID.
        job_id: u64,
        /// Replayable edit.
        edit: Box<FoundryEdit>,
        /// Rebuilt output after the edit.
        output: Box<FoundryCompilationOutput>,
    },
    /// Pack compilation completed.
    PackCompiled {
        /// Matching app-local job ID.
        job_id: u64,
        /// UI-ready pack view.
        pack: Box<FoundryPackView>,
    },
    /// Pack export completed.
    PackExportFinished {
        /// Matching app-local job ID.
        job_id: u64,
        /// Export profile key.
        profile: String,
        /// Destination directory.
        out_dir: PathBuf,
        /// Number of member packages written.
        member_count: usize,
    },
    /// Export completed.
    ExportFinished {
        /// Matching app-local job ID.
        job_id: u64,
        /// Export profile key.
        profile: String,
        /// Destination directory.
        out_dir: PathBuf,
    },
    /// A job failed.
    Failed {
        /// Matching app-local job ID.
        job_id: u64,
        /// Human-readable diagnostic.
        message: String,
    },
}

impl FoundryJobEvent {
    /// Return the app-local job ID.
    #[must_use]
    pub(crate) fn job_id(&self) -> u64 {
        match self {
            Self::CompileFinished { job_id, .. }
            | Self::PreviewRendered { job_id, .. }
            | Self::CandidatesGenerated { job_id, .. }
            | Self::CandidatePreviewsRendered { job_id, .. }
            | Self::EditApplied { job_id, .. }
            | Self::PackCompiled { job_id, .. }
            | Self::PackExportFinished { job_id, .. }
            | Self::ExportFinished { job_id, .. }
            | Self::Failed { job_id, .. } => *job_id,
        }
    }
}

/// Run one Foundry job. Callers should invoke this from a worker thread.
pub(crate) fn run_foundry_job(
    request: FoundryJobRequest,
    resolver: &impl FoundryCatalogResolver,
    preview_cache: &mut FoundryPreviewCache,
) -> FoundryJobEvent {
    let job_id = request.job_id();
    let result = match request {
        FoundryJobRequest::CompileCurrent { document, .. } => {
            compile_foundry_document(&document, resolver)
                .map(|output| FoundryJobEvent::CompileFinished {
                    job_id,
                    output: Box::new(output),
                })
                .map_err(|error| format!("{error:?}"))
        }
        FoundryJobRequest::RenderPreview {
            output,
            width,
            height,
            camera,
            ..
        } => render_preview(job_id, &output, width, height, camera, preview_cache),
        FoundryJobRequest::PreviewControlValue {
            document,
            control_id,
            value,
            width,
            height,
            ..
        } => render_transient_control_preview(
            job_id,
            &document,
            &control_id,
            value,
            (width, height),
            resolver,
            preview_cache,
        ),
        FoundryJobRequest::GenerateCandidates {
            document, request, ..
        } => generate_foundry_candidate_draft_plans(&document, resolver, &request)
            .map_err(|error| error.to_string())
            .map(|output| {
                let cards =
                    candidate_cards_from_output_pending_previews(&output, Some(request.mode), None);
                FoundryJobEvent::CandidatesGenerated {
                    job_id,
                    request,
                    output: Box::new(output),
                    cards,
                }
            }),
        FoundryJobRequest::RenderCandidatePreviews {
            document,
            request,
            output,
            ..
        } => {
            let original_count = output.candidates.len();
            candidate_cards_from_output_with_previews(
                &document,
                &output,
                Some(request.mode),
                None,
                resolver,
                preview_cache,
            )
            .map(|cards| {
                let rejected_count = original_count.saturating_sub(cards.len());
                FoundryJobEvent::CandidatePreviewsRendered {
                    job_id,
                    request,
                    cards,
                    rejected_count,
                }
            })
        }
        FoundryJobRequest::ApplyEdit { document, edit, .. } => {
            apply_edit_and_compile(&document, &edit, resolver).map(|output| {
                FoundryJobEvent::EditApplied {
                    job_id,
                    edit,
                    output: Box::new(output),
                }
            })
        }
        FoundryJobRequest::CompilePack { pack, .. } => compile_foundry_pack(&pack, resolver)
            .map(|output| FoundryJobEvent::PackCompiled {
                job_id,
                pack: Box::new(pack_view_from_output(output)),
            })
            .map_err(|error| format!("{error:?}")),
        FoundryJobRequest::ExportPack { pack, out_dir, .. } => {
            export_foundry_pack(job_id, &pack, out_dir, resolver)
        }
        FoundryJobRequest::Export {
            output,
            profile,
            out_dir,
            ..
        } => write_model_package(&output.recipe, &output.artifact, &out_dir)
            .map(|_| FoundryJobEvent::ExportFinished {
                job_id,
                profile,
                out_dir,
            })
            .map_err(|error| error.to_string()),
    };

    result.unwrap_or_else(|error| FoundryJobEvent::Failed {
        job_id,
        message: error.to_string(),
    })
}

fn apply_edit_and_compile(
    document: &FoundryAssetDocument,
    edit: &FoundryEdit,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryCompilationOutput, String> {
    let mut document = document.clone();
    for command in &edit.commands {
        apply_edit_command(&mut document, command, resolver)?;
    }
    compile_foundry_document(&document, resolver).map_err(|error| format!("{error:?}"))
}

fn apply_edit_command(
    document: &mut FoundryAssetDocument,
    command: &FoundryCommand,
    resolver: &impl FoundryCatalogResolver,
) -> Result<(), String> {
    if let FoundryCommand::SetStyle {
        style_content_ref,
        style_implementation_ref,
    } = command
    {
        let mut context_document = document.clone();
        context_document.style_content_ref = style_content_ref.clone();
        context_document.style_implementation_ref = style_implementation_ref.clone();
        context_document.catalog_lock = None;
        context_document.build_stamp = None;
        let catalog = resolve_foundry_catalog(&context_document, resolver)
            .map_err(|error| format!("{error:?}"))?;
        let style_context = FoundryStyleChangeContext {
            profile: Some(&catalog.customizer_profile),
            family_implementation: Some(&catalog.family_implementation),
            style_implementation: Some(&catalog.style_implementation),
        };
        apply_foundry_command_with_style_context(document, command, style_context)
            .map_err(|error| format!("{error:?}"))?;
        return Ok(());
    }

    apply_foundry_command(document, command).map_err(|error| format!("{error:?}"))?;
    Ok(())
}

fn render_preview(
    job_id: u64,
    output: &FoundryCompilationOutput,
    width: u32,
    height: u32,
    camera: Option<OrbitCamera>,
    _preview_cache: &mut FoundryPreviewCache,
) -> Result<FoundryJobEvent, String> {
    let mesh = preview_mesh_from_output(output);
    let readable_camera = readable_preview_camera_for_output(
        output,
        mesh.bounds,
        width as f32 / height.max(1) as f32,
    );
    let camera = camera
        .map(|camera| camera.clamped())
        .or(readable_camera)
        .unwrap_or_else(|| fit_camera_to_bounds(mesh.bounds));
    let settings = clay_readability_render_settings(width, height);
    let image = render_mesh(&mesh, &camera, &settings).map_err(|error| error.to_string())?;
    Ok(FoundryJobEvent::PreviewRendered {
        job_id,
        preview_id: CURRENT_PREVIEW_ID.to_owned(),
        rgba8: image.rgba8,
        width: image.width,
        height: image.height,
        camera,
        build: Some(output.build_stamp.clone()),
    })
}

fn readable_preview_camera_for_output(
    output: &FoundryCompilationOutput,
    bounds: Aabb,
    aspect_ratio: f32,
) -> Option<OrbitCamera> {
    let family_id = output.catalog.customizer_profile.family_id.as_str();
    let box_like = family_id.contains("box_primitive")
        || family_id.contains("lidded_box")
        || family_id.contains("trimmed_box");
    let panel_like = family_id.contains("flat_panel")
        || family_id.contains("hinged_panel")
        || family_id.contains("handled_panel")
        || family_id.contains("panel_with_knob");
    (box_like || panel_like)
        .then(|| fit_camera_to_bounds_from_angles_around_origin(bounds, 35.0, 20.0, aspect_ratio))
}

fn export_foundry_pack(
    job_id: u64,
    pack: &FoundryPackDocument,
    out_dir: PathBuf,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryJobEvent, String> {
    let output = compile_foundry_pack(pack, resolver).map_err(|error| format!("{error:?}"))?;
    fs::create_dir_all(&out_dir).map_err(|error| error.to_string())?;
    for (member_id, member_output) in &output.member_outputs {
        let member_dir = out_dir.join(safe_export_segment(member_id));
        write_model_package(&member_output.recipe, &member_output.artifact, &member_dir)
            .map_err(|error| error.to_string())?;
    }
    Ok(FoundryJobEvent::PackExportFinished {
        job_id,
        profile: output.pack.export_profile.profile,
        out_dir,
        member_count: output.member_outputs.len(),
    })
}

fn safe_export_segment(segment: &str) -> String {
    let sanitized = segment
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "member".to_owned()
    } else {
        sanitized
    }
}

fn render_transient_control_preview(
    job_id: u64,
    document: &FoundryAssetDocument,
    control_id: &str,
    value: ControlValue,
    size: (u32, u32),
    resolver: &impl FoundryCatalogResolver,
    preview_cache: &mut FoundryPreviewCache,
) -> Result<FoundryJobEvent, String> {
    let mut preview_document = document.clone();
    apply_foundry_command(
        &mut preview_document,
        &FoundryCommand::SetControl {
            control_id: control_id.to_owned(),
            value,
        },
    )
    .map_err(|error| format!("{error:?}"))?;
    let output = compile_foundry_document(&preview_document, resolver)
        .map_err(|error| format!("{error:?}"))?;
    let (width, height) = size;
    render_preview(job_id, &output, width, height, None, preview_cache)
}

fn preview_request_for_output(
    preview_id: impl Into<String>,
    kind: FoundryPreviewKind,
    output: &FoundryCompilationOutput,
) -> FoundryPreviewRequest {
    let mut request = FoundryPreviewRequest::new(
        preview_id,
        kind,
        output.build_stamp.geometry_input_fingerprint.0.to_hex(),
        preview_mesh_from_output(output),
    );
    request.sampled_control_state = output
        .document
        .control_state
        .iter()
        .filter_map(|(control_id, value)| {
            preview_control_value(value).map(|value| (control_id.clone(), value))
        })
        .collect();
    request.provider_choices = output
        .provider_override_reports
        .iter()
        .map(|report| (report.role.clone(), report.provider_id.clone()))
        .collect();
    let intent = output.document.variation_state.intent.clone().normalized();
    request.variation_metadata = preview_variation_metadata_from_intent(&intent, None);
    request
}

fn preview_control_value(
    value: &ControlValue,
) -> Option<shape_render::foundry::FoundryPreviewControlValue> {
    match value {
        ControlValue::Scalar(value) => {
            value
                .is_finite()
                .then_some(shape_render::foundry::FoundryPreviewControlValue::Scalar(
                    *value,
                ))
        }
        ControlValue::Integer(value) => Some(
            shape_render::foundry::FoundryPreviewControlValue::Integer(*value),
        ),
        ControlValue::Toggle(value) => Some(
            shape_render::foundry::FoundryPreviewControlValue::Toggle(*value),
        ),
        ControlValue::Choice(value) => Some(
            shape_render::foundry::FoundryPreviewControlValue::Choice(value.clone()),
        ),
        ControlValue::Provider(value) => Some(
            shape_render::foundry::FoundryPreviewControlValue::Provider(value.clone()),
        ),
    }
}

fn preview_mesh_from_output(output: &FoundryCompilationOutput) -> TriangleMesh {
    let mesh = &output.artifact.combined_preview.mesh;
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

/// Build UI-ready candidate cards from generated candidate output.
pub(crate) fn candidate_cards_from_output(
    output: &FoundryCandidateOutput,
    mode: Option<FoundryCandidateMode>,
    selected: Option<&shape_foundry::FoundryCandidateId>,
) -> Vec<FoundryCandidateCard> {
    output
        .candidates
        .iter()
        .enumerate()
        .map(|(slot, candidate)| {
            let changed_roles = changed_roles(&candidate.edit.commands);
            let variation_metadata = &candidate.variation_metadata;
            let visible_delta = &variation_metadata.visible_delta;
            let validation_label = if candidate.conformance.accepted {
                visible_delta.label().to_owned()
            } else {
                "Rejected".to_owned()
            };
            let validation_detail = (!candidate.conformance.accepted)
                .then(|| {
                    format!(
                        "{} required failure(s), {} advisory issue(s)",
                        candidate.conformance.required_failure_count,
                        candidate.conformance.advisory_issue_count
                    )
                })
                .or_else(|| visible_delta_detail(visible_delta));
            let mut rejections = BTreeMap::new();
            let selectable =
                candidate.conformance.accepted && visible_delta.legibility_class.selectable();
            if !visible_delta.legibility_class.selectable() {
                rejections.insert(
                    rejection_reason_for_legibility(visible_delta.legibility_class),
                    1,
                );
            }
            FoundryCandidateCard {
                id: candidate.id.clone(),
                slot,
                mode,
                parent: false,
                title: candidate.label.clone(),
                subtitle: candidate_subtitle(mode, candidate.changed_controls.len()),
                preview_id: Some(format!("candidate-{}", candidate.id.0)),
                rgba8: Vec::new(),
                width: 0,
                height: 0,
                camera: None,
                preview_failure: None,
                changed_controls: candidate.changed_controls.clone(),
                changed_roles,
                explanations: candidate.diagnostics.changes.clone(),
                rejections,
                validation_label,
                validation_detail,
                selectable,
                selected: selected.is_some_and(|selected| selected == &candidate.id),
                variation_intent_label: variation_metadata.intent.human_label.clone(),
                variation_scope_label: variation_metadata.intent.scope.display_label().to_owned(),
                variation_channel_labels: variation_metadata
                    .intent
                    .channels
                    .iter()
                    .map(|channel| channel.display_label().to_owned())
                    .collect(),
                visible_delta_label: visible_delta.label().to_owned(),
                what_changed_summary: what_changed_summary(
                    variation_metadata,
                    &candidate.diagnostics.changes,
                ),
                legibility_class: visible_delta.legibility_class,
                focus_part_label: focus_part_label(&variation_metadata.intent.scope),
                surface_unavailable_reason: surface_unavailable_reason(variation_metadata),
            }
        })
        .collect()
}

/// Build candidate cards that can be shown before preview rendering completes.
pub(crate) fn candidate_cards_from_output_pending_previews(
    output: &FoundryCandidateOutput,
    mode: Option<FoundryCandidateMode>,
    selected: Option<&shape_foundry::FoundryCandidateId>,
) -> Vec<FoundryCandidateCard> {
    let mut cards = candidate_cards_from_output(output, mode, selected);
    for card in &mut cards {
        card.preview_failure = Some(PENDING_CANDIDATE_PREVIEW_MESSAGE.to_owned());
        card.validation_label = "Preview pending".to_owned();
        card.validation_detail =
            Some("This direction can be chosen after its card preview renders.".to_owned());
        card.selectable = false;
    }
    cards
}

pub(crate) fn candidate_cards_from_output_with_previews(
    parent_document: &FoundryAssetDocument,
    output: &FoundryCandidateOutput,
    mode: Option<FoundryCandidateMode>,
    selected: Option<&shape_foundry::FoundryCandidateId>,
    resolver: &impl FoundryCatalogResolver,
    preview_cache: &mut FoundryPreviewCache,
) -> Result<Vec<FoundryCandidateCard>, String> {
    let mut cards = candidate_cards_from_output(output, mode, selected);
    if cards.is_empty() {
        return Ok(cards);
    }

    let Some(preview_resolution) = FoundryPreviewResolution::from_pixels(CANDIDATE_PREVIEW_PIXELS)
    else {
        for card in &mut cards {
            card.preview_failure = Some(format!(
                "Candidate preview size {CANDIDATE_PREVIEW_PIXELS}px is not supported."
            ));
            card.selectable = false;
        }
        return Ok(cards);
    };

    let parent_output = match compile_foundry_document(parent_document, resolver) {
        Ok(output) => output,
        Err(error) => {
            let message = format!("Parent preview compile failed: {error:?}");
            for card in &mut cards {
                card.preview_failure = Some(message.clone());
            }
            return Ok(cards);
        }
    };
    let parent_preview_mesh = preview_mesh_from_output(&parent_output);
    let parent_preview_camera =
        readable_preview_camera_for_output(&parent_output, parent_preview_mesh.bounds, 1.0);
    let mut parent_request = preview_request_for_output(
        CANDIDATE_PARENT_PREVIEW_ID,
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        &parent_output,
    );
    let parent_intent = output
        .candidates
        .first()
        .map(|candidate| candidate.variation_metadata.intent.clone())
        .unwrap_or_else(|| parent_output.document.variation_state.intent.clone());
    parent_request.variation_metadata =
        preview_variation_metadata_from_intent(&parent_intent, None);
    let mut requests = vec![parent_request.clone()];
    let mut candidate_requests = BTreeMap::new();
    for (index, candidate) in output.candidates.iter().enumerate() {
        match compile_foundry_document(&candidate.document, resolver) {
            Ok(candidate_output) => {
                if !compiled_model_validation_is_valid(&candidate_output) {
                    cards[index].preview_failure =
                        Some("Candidate model validation failed.".to_owned());
                    cards[index].selectable = false;
                    continue;
                }
                let preview_id = format!("candidate-{}", candidate.id.0);
                let mut request = preview_request_for_output(
                    preview_id.clone(),
                    FoundryPreviewKind::CandidateCard {
                        candidate_id: candidate.id.0.clone(),
                    },
                    &candidate_output,
                );
                request.variation_metadata = preview_variation_metadata_from_candidate(
                    &candidate.variation_metadata,
                    Some(candidate.variation_metadata.visible_delta.legibility_class),
                );
                candidate_requests.insert(preview_id, (index, request.clone()));
                requests.push(request);
            }
            Err(error) => {
                mark_candidate_preview_compile_failure(&mut cards[index], &error);
            }
        }
    }

    if candidate_requests.is_empty() {
        return Ok(cards);
    }

    let mut batch =
        FoundryPreviewBatchRequest::new("candidate-directions", requests, preview_resolution);
    batch.camera = parent_preview_camera.clone();
    batch.max_parallel_jobs = 4;
    if let Ok(rendered) = preview_cache.render_batch(batch) {
        let rendered_by_id = rendered
            .previews
            .into_iter()
            .map(|preview| (preview.preview_id.clone(), preview))
            .collect::<BTreeMap<_, _>>();

        for (preview_id, (index, _)) in &candidate_requests {
            if let Some(preview) = rendered_by_id.get(preview_id) {
                apply_candidate_preview(&mut cards[*index], preview);
            } else {
                cards[*index].preview_failure = Some(format!(
                    "Candidate preview `{preview_id}` was not rendered."
                ));
            }
        }

        return Ok(filter_legible_candidate_cards(
            cards,
            rendered_by_id
                .get(CANDIDATE_PARENT_PREVIEW_ID)
                .map(|preview| preview.image.rgba8.as_slice()),
            rendered_by_id
                .get(CANDIDATE_PARENT_PREVIEW_ID)
                .map(|preview| (preview.image.width, preview.image.height)),
            mode,
        ));
    }

    let mut parent_preview_rgba8 = None;
    let mut parent_preview_size = None;
    for (preview_id, (index, request)) in candidate_requests {
        let mut batch = FoundryPreviewBatchRequest::new(
            format!("candidate-direction-{preview_id}"),
            vec![parent_request.clone(), request],
            preview_resolution,
        );
        batch.camera = parent_preview_camera.clone();
        batch.max_parallel_jobs = 2;
        match preview_cache.render_batch(batch) {
            Ok(rendered) => {
                if parent_preview_rgba8.is_none()
                    && let Some(parent_preview) = rendered
                        .previews
                        .iter()
                        .find(|preview| preview.preview_id == CANDIDATE_PARENT_PREVIEW_ID)
                {
                    parent_preview_rgba8 = Some(parent_preview.image.rgba8.clone());
                    parent_preview_size =
                        Some((parent_preview.image.width, parent_preview.image.height));
                }
                let preview = rendered
                    .previews
                    .iter()
                    .find(|preview| preview.preview_id == preview_id);
                if let Some(preview) = preview {
                    apply_candidate_preview(&mut cards[index], preview);
                } else {
                    cards[index].preview_failure = Some(format!(
                        "Candidate preview `{preview_id}` was not rendered."
                    ));
                }
            }
            Err(error) => {
                cards[index].preview_failure = Some(format!("Preview render failed: {error}"));
            }
        }
    }

    Ok(filter_legible_candidate_cards(
        cards,
        parent_preview_rgba8.as_deref(),
        parent_preview_size,
        mode,
    ))
}

fn compiled_model_validation_is_valid(output: &FoundryCompilationOutput) -> bool {
    if !output.artifact.validation_report.is_valid() {
        return false;
    }
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    validate_model(&output.artifact, &config).is_valid()
}

#[derive(Debug, Copy, Clone)]
struct PreviewLegibilityDelta {
    mean_pixel_delta: f32,
    changed_pixel_ratio: f32,
    silhouette_delta: f32,
    score: f32,
}

impl PreviewLegibilityDelta {
    fn score(self) -> f32 {
        self.score
    }
}

#[derive(Debug, Copy, Clone)]
struct PreviewLegibilityThreshold {
    score: f32,
    changed_pixel_ratio: f32,
    silhouette_delta: f32,
}

fn filter_legible_candidate_cards(
    cards: Vec<FoundryCandidateCard>,
    parent_rgba8: Option<&[u8]>,
    parent_size: Option<(u32, u32)>,
    mode: Option<FoundryCandidateMode>,
) -> Vec<FoundryCandidateCard> {
    let Some(parent_rgba8) = parent_rgba8 else {
        return cards;
    };
    let Some((parent_width, parent_height)) = parent_size else {
        return cards;
    };
    let threshold = legibility_threshold(mode);
    let duplicate_threshold = duplicate_legibility_threshold(mode);
    let mut kept = Vec::with_capacity(cards.len());
    for mut card in cards {
        if card.preview_failure.is_some() || card.width == 0 || card.height == 0 {
            kept.push(card);
            continue;
        }
        let Some(parent_delta) = preview_legibility_delta(
            parent_rgba8,
            parent_width,
            parent_height,
            &card.rgba8,
            card.width,
            card.height,
        ) else {
            card.preview_failure =
                Some("Preview could not be compared against the current model.".to_owned());
            card.selectable = false;
            kept.push(card);
            continue;
        };
        let preview_class = preview_delta_class(parent_delta, mode);
        if !passes_legibility_threshold(parent_delta, threshold) {
            continue;
        }
        let duplicate = kept.iter().any(|kept_card: &FoundryCandidateCard| {
            kept_card.preview_failure.is_none()
                && preview_legibility_delta(
                    &kept_card.rgba8,
                    kept_card.width,
                    kept_card.height,
                    &card.rgba8,
                    card.width,
                    card.height,
                )
                .is_some_and(|delta| !passes_legibility_threshold(delta, duplicate_threshold))
        });
        if duplicate {
            continue;
        }
        let conformance_accepted = card.validation_label != "Rejected";
        card.legibility_class = preview_class;
        card.visible_delta_label = preview_class.display_label().to_owned();
        card.validation_label = preview_class.display_label().to_owned();
        card.validation_detail = Some(format!(
            "Card-size visible change score {:.1}%.",
            parent_delta.score() * 100.0
        ));
        for reason in [
            FoundryCandidateRejectionReason::DescriptorRejected,
            FoundryCandidateRejectionReason::TooSubtle,
            FoundryCandidateRejectionReason::DuplicateLooking,
        ] {
            card.rejections.remove(&reason);
        }
        card.selectable = conformance_accepted && preview_class.selectable();
        kept.push(card);
    }
    kept
}

fn preview_legibility_delta(
    left_rgba8: &[u8],
    left_width: u32,
    left_height: u32,
    right_rgba8: &[u8],
    right_width: u32,
    right_height: u32,
) -> Option<PreviewLegibilityDelta> {
    let parent = RenderedImage {
        width: left_width,
        height: left_height,
        rgba8: left_rgba8.to_vec(),
    };
    let candidate = RenderedImage {
        width: right_width,
        height: right_height,
        rgba8: right_rgba8.to_vec(),
    };
    let delta = compare_foundry_rendered_visible_delta(
        &parent,
        &candidate,
        RenderSettings::default().background,
    );
    delta.available().then_some(PreviewLegibilityDelta {
        mean_pixel_delta: delta.mean_pixel_delta,
        changed_pixel_ratio: delta.changed_pixel_ratio,
        silhouette_delta: delta.silhouette_delta,
        score: delta.score,
    })
}

fn passes_legibility_threshold(
    delta: PreviewLegibilityDelta,
    threshold: PreviewLegibilityThreshold,
) -> bool {
    delta.score() >= threshold.score
        || delta.changed_pixel_ratio >= threshold.changed_pixel_ratio
        || delta.silhouette_delta >= threshold.silhouette_delta
}

fn legibility_threshold(mode: Option<FoundryCandidateMode>) -> PreviewLegibilityThreshold {
    match mode.unwrap_or(FoundryCandidateMode::Explore) {
        FoundryCandidateMode::Refine => PreviewLegibilityThreshold {
            score: 0.020,
            changed_pixel_ratio: 0.040,
            silhouette_delta: 0.014,
        },
        FoundryCandidateMode::Explore => PreviewLegibilityThreshold {
            score: 0.040,
            changed_pixel_ratio: 0.085,
            silhouette_delta: 0.030,
        },
        FoundryCandidateMode::Silhouette => PreviewLegibilityThreshold {
            score: 0.055,
            changed_pixel_ratio: 0.105,
            silhouette_delta: 0.045,
        },
        FoundryCandidateMode::Structure => PreviewLegibilityThreshold {
            score: 0.045,
            changed_pixel_ratio: 0.090,
            silhouette_delta: 0.032,
        },
        FoundryCandidateMode::Detail => PreviewLegibilityThreshold {
            score: 0.012,
            changed_pixel_ratio: 0.022,
            silhouette_delta: 0.008,
        },
    }
}

fn duplicate_legibility_threshold(
    mode: Option<FoundryCandidateMode>,
) -> PreviewLegibilityThreshold {
    let threshold = legibility_threshold(mode);
    PreviewLegibilityThreshold {
        score: threshold.score * 0.55,
        changed_pixel_ratio: threshold.changed_pixel_ratio * 0.55,
        silhouette_delta: threshold.silhouette_delta * 0.55,
    }
}

fn apply_candidate_preview(
    card: &mut FoundryCandidateCard,
    preview: &shape_render::foundry::FoundryRenderedPreview,
) {
    card.preview_id = Some(preview.preview_id.clone());
    card.rgba8 = preview.image.rgba8.clone();
    card.width = preview.image.width;
    card.height = preview.image.height;
    card.camera = Some(preview.camera.clone());
    card.preview_failure = None;
    if let Some(class) = preview.variation_metadata.legibility_class {
        card.legibility_class = class;
        card.visible_delta_label = class.display_label().to_owned();
    }
}

fn mark_candidate_preview_compile_failure(
    card: &mut FoundryCandidateCard,
    _error: &impl std::fmt::Debug,
) {
    card.preview_failure = Some("Preview unavailable for this direction.".to_owned());
    card.validation_label = "Preview unavailable".to_owned();
    card.validation_detail =
        Some("This direction cannot be chosen because its preview did not compile.".to_owned());
    card.selectable = false;
    card.legibility_class = CandidateLegibilityClass::Unsupported;
    card.visible_delta_label = CandidateLegibilityClass::Unsupported
        .display_label()
        .to_owned();
}

fn preview_variation_metadata_from_candidate(
    metadata: &CandidateVariationMetadata,
    legibility_class: Option<CandidateLegibilityClass>,
) -> FoundryPreviewVariationMetadata {
    preview_variation_metadata_from_intent(&metadata.intent, legibility_class)
}

fn preview_variation_metadata_from_intent(
    intent: &VariationIntent,
    legibility_class: Option<CandidateLegibilityClass>,
) -> FoundryPreviewVariationMetadata {
    let intent = intent.clone().normalized();
    let selected_part_group = intent.scope.semantic_part_group_id().map(str::to_owned);
    let material_slot_id = match &intent.scope {
        VariationScope::MaterialSlot { slot_id, .. } => Some(slot_id.clone()),
        _ => None,
    };
    FoundryPreviewVariationMetadata {
        scope: intent.scope,
        channels: intent.channels,
        selected_part_group,
        material_slot_id,
        legibility_class,
    }
}

fn what_changed_summary(
    metadata: &CandidateVariationMetadata,
    explanations: &[shape_search::foundry::FoundryCandidateControlChange],
) -> String {
    let mut parts = Vec::new();
    for group in &metadata.changed_part_groups {
        if group.visible {
            parts.push(format!("{}: {}", group.display_name, group.change_label));
        }
    }
    for slot in &metadata.changed_material_slots {
        if slot.surface_payload_ready {
            parts.push(format!("{}: {}", slot.display_name, slot.change_label));
        }
    }
    for change in explanations {
        if parts.len() >= 3 {
            break;
        }
        if let Some(label) = safe_product_fragment(&change.control_label) {
            parts.push(format!("{label} adjusted"));
        }
    }
    if parts.is_empty() {
        let scope = metadata.intent.scope.display_label();
        let delta = metadata.visible_delta.label();
        return format!("{scope}: {delta}");
    }
    parts.truncate(3);
    parts.join(" · ")
}

fn focus_part_label(scope: &VariationScope) -> Option<String> {
    match scope {
        VariationScope::SemanticPartGroup { display_name, .. } => Some(display_name.clone()),
        _ => None,
    }
}

fn surface_unavailable_reason(metadata: &CandidateVariationMetadata) -> Option<String> {
    let surface_unavailable = metadata
        .changed_material_slots
        .iter()
        .any(|slot| !slot.surface_payload_ready)
        || metadata
            .visible_delta
            .blocking_reasons
            .iter()
            .any(|reason| {
                reason == SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON
                    || reason.to_ascii_lowercase().contains("surface")
            });
    surface_unavailable.then(|| SURFACE_VISUAL_VARIATION_UNAVAILABLE_REASON.to_owned())
}

fn visible_delta_detail(report: &CandidateVisibleDeltaReport) -> Option<String> {
    if !report.blocking_reasons.is_empty() {
        return Some(report.blocking_reasons.join(" "));
    }
    report.manual_review_required.then(|| {
        "This direction needs manual review before stronger visual-difference claims.".to_owned()
    })
}

fn rejection_reason_for_legibility(
    class: CandidateLegibilityClass,
) -> FoundryCandidateRejectionReason {
    match class {
        CandidateLegibilityClass::TooSubtle => FoundryCandidateRejectionReason::TooSubtle,
        CandidateLegibilityClass::DuplicateLooking => {
            FoundryCandidateRejectionReason::DuplicateLooking
        }
        CandidateLegibilityClass::Unsupported => {
            FoundryCandidateRejectionReason::UnsupportedChannel
        }
        CandidateLegibilityClass::Strong
        | CandidateLegibilityClass::Clear
        | CandidateLegibilityClass::SubtleButExplainable
        | CandidateLegibilityClass::DetailOnly => {
            FoundryCandidateRejectionReason::DescriptorRejected
        }
    }
}

fn preview_delta_class(
    delta: PreviewLegibilityDelta,
    mode: Option<FoundryCandidateMode>,
) -> CandidateLegibilityClass {
    if matches!(mode, Some(FoundryCandidateMode::Detail)) {
        return CandidateLegibilityClass::DetailOnly;
    }
    let score = delta.score();
    if score >= 0.10 || delta.changed_pixel_ratio >= 0.16 || delta.silhouette_delta >= 0.075 {
        CandidateLegibilityClass::Strong
    } else if score >= 0.04 || delta.changed_pixel_ratio >= 0.085 || delta.silhouette_delta >= 0.030
    {
        CandidateLegibilityClass::Clear
    } else {
        CandidateLegibilityClass::SubtleButExplainable
    }
}

fn safe_product_fragment(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || crate::foundry::ui::copy::first_forbidden_product_term(trimmed).is_some()
    {
        return None;
    }
    let lowercase = trimmed.to_ascii_lowercase();
    let raw_markers = [
        "provider",
        "scalar",
        "semantic",
        "operation",
        "compiler",
        "decompiler",
        "fragment",
        "remap",
        "conformance",
        "socket",
        "port",
        "recipe",
    ];
    (!raw_markers.iter().any(|marker| lowercase.contains(marker))).then(|| trimmed.to_owned())
}

fn candidate_subtitle(mode: Option<FoundryCandidateMode>, change_count: usize) -> String {
    let mode = mode
        .map(|mode| format!("{mode:?}"))
        .unwrap_or_else(|| "Candidate".to_owned());
    match change_count {
        0 => mode,
        1 => format!("{mode} · 1 change"),
        count => format!("{mode} · {count} changes"),
    }
}

fn changed_roles(commands: &[FoundryCommand]) -> Vec<String> {
    let mut roles = Vec::new();
    for command in commands {
        match command {
            FoundryCommand::SelectProvider { role, .. }
            | FoundryCommand::SetRolePresence { role, .. }
                if !roles.contains(role) =>
            {
                roles.push(role.clone());
            }
            _ => {}
        }
    }
    roles
}

/// Build a deterministic pack view from a pack compilation output.
pub(crate) fn pack_view_from_output(output: FoundryPackCompilationOutput) -> FoundryPackView {
    let shared_provider_choices = match &output.pack.shared_provider_policy {
        SharedProviderPolicy::Independent => BTreeMap::new(),
        SharedProviderPolicy::SharedExact(providers) => providers
            .iter()
            .map(|(role, provider_ref)| (role.clone(), provider_ref.stable_id.clone()))
            .collect(),
    };
    let mut member_override_counts = BTreeMap::new();
    for difference in &output.report.differences {
        *member_override_counts
            .entry(difference.member_id.clone())
            .or_insert(0) += 1;
    }
    let coherence_warnings = output
        .report
        .conformance_status
        .issues
        .iter()
        .map(|issue| issue.message.clone())
        .collect::<Vec<_>>();
    FoundryPackView {
        pack_id: Some(output.pack.pack_id.clone()),
        members: output
            .pack
            .members
            .iter()
            .map(|(member_id, document)| (member_id.clone(), document.document_id.clone()))
            .collect(),
        selected_member: output.pack.members.keys().next().cloned(),
        shared_locks: output.pack.shared_locks.clone(),
        shared_provider_choices,
        member_override_counts,
        coherence_warnings,
        coherent: output.report.conformance_status.accepted,
        can_export: output.report.conformance_status.accepted && !output.pack.members.is_empty(),
        pack: Some(output.pack),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_preview_preserves_clay_readability_edge_aids() {
        let fixture = shape_foundry_catalog::sphere_primitive::fixture_catalog();
        let output =
            compile_foundry_document(&fixture.document, &fixture).expect("fixture should compile");
        let event = render_preview(
            1,
            &output,
            128,
            128,
            None,
            &mut FoundryPreviewCache::default(),
        )
        .expect("current preview should render");

        let FoundryJobEvent::PreviewRendered {
            rgba8,
            width,
            height,
            ..
        } = event
        else {
            panic!("expected preview event");
        };
        let background = clay_readability_render_settings(width, height).background;
        let colors = unique_foreground_colors(&rgba8, background);

        assert!(
            colors.len() > 3,
            "current previews should preserve lit clay and edge-aid colors"
        );
        assert!(
            colors
                .keys()
                .any(|[red, green, blue]| red != green || green != blue),
            "current previews must not be flattened to comparison gray bands"
        );
    }

    #[test]
    fn sphere_preview_remains_visible_from_orbit_angles() {
        let fixture = shape_foundry_catalog::sphere_primitive::fixture_catalog();
        let output =
            compile_foundry_document(&fixture.document, &fixture).expect("fixture should compile");
        for (yaw, pitch) in [(90.0, 0.0), (180.0, 18.0), (270.0, -18.0)] {
            let event = render_preview(
                1,
                &output,
                128,
                128,
                Some(OrbitCamera {
                    yaw_degrees: yaw,
                    pitch_degrees: pitch,
                    ..OrbitCamera::default()
                }),
                &mut FoundryPreviewCache::default(),
            )
            .expect("current preview should render");

            let FoundryJobEvent::PreviewRendered {
                rgba8,
                width,
                height,
                ..
            } = event
            else {
                panic!("expected preview event");
            };
            let background = clay_readability_render_settings(width, height).background;
            let foreground_pixels = rgba8
                .chunks_exact(4)
                .filter(|pixel| **pixel != background)
                .count();

            assert!(
                foreground_pixels > 1_000,
                "sphere orbit preview should keep a visible volume at yaw {yaw} pitch {pitch}, got {foreground_pixels} foreground pixels"
            );
        }
    }

    #[test]
    fn preview_legibility_filter_promotes_rendered_selectable_candidate() {
        let parent = solid_rgba(16, 16, [16, 20, 24, 255]);
        let candidate = solid_rgba(16, 16, [220, 230, 238, 255]);
        let mut card = test_candidate_card("candidate-a", candidate);
        card.selectable = false;
        card.legibility_class = CandidateLegibilityClass::TooSubtle;
        card.validation_label = CandidateLegibilityClass::TooSubtle
            .display_label()
            .to_owned();
        card.rejections
            .insert(FoundryCandidateRejectionReason::TooSubtle, 1);

        let cards = filter_legible_candidate_cards(
            vec![card],
            Some(&parent),
            Some((16, 16)),
            Some(FoundryCandidateMode::Explore),
        );

        assert_eq!(cards.len(), 1);
        assert!(cards[0].selectable);
        assert!(cards[0].preview_failure.is_none());
        assert!(cards[0].legibility_class.selectable());
        assert!(
            !cards[0]
                .rejections
                .contains_key(&FoundryCandidateRejectionReason::TooSubtle)
        );
    }

    fn solid_rgba(width: usize, height: usize, color: [u8; 4]) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(width * height * 4);
        for _ in 0..(width * height) {
            rgba.extend_from_slice(&color);
        }
        rgba
    }

    fn test_candidate_card(id: &str, rgba8: Vec<u8>) -> FoundryCandidateCard {
        FoundryCandidateCard {
            id: shape_foundry::FoundryCandidateId(id.to_owned()),
            slot: 0,
            mode: Some(FoundryCandidateMode::Explore),
            parent: false,
            title: "Rendered candidate".to_owned(),
            subtitle: "Explore".to_owned(),
            preview_id: Some(format!("candidate-{id}")),
            rgba8,
            width: 16,
            height: 16,
            camera: Some(OrbitCamera::default()),
            preview_failure: None,
            changed_controls: Vec::new(),
            changed_roles: Vec::new(),
            explanations: Vec::new(),
            rejections: BTreeMap::new(),
            validation_label: "Ready".to_owned(),
            validation_detail: None,
            selectable: true,
            selected: false,
            variation_intent_label: "Complete look".to_owned(),
            variation_scope_label: "Whole asset".to_owned(),
            variation_channel_labels: Vec::new(),
            visible_delta_label: "Clear change".to_owned(),
            what_changed_summary: "The rendered candidate is visibly different.".to_owned(),
            legibility_class: CandidateLegibilityClass::Clear,
            focus_part_label: None,
            surface_unavailable_reason: None,
        }
    }

    fn unique_foreground_colors(rgba8: &[u8], background: [u8; 4]) -> BTreeMap<[u8; 3], usize> {
        let mut colors = BTreeMap::new();
        for pixel in rgba8.chunks_exact(4) {
            if [pixel[0], pixel[1], pixel[2], pixel[3]] == background {
                continue;
            }
            *colors.entry([pixel[0], pixel[1], pixel[2]]).or_insert(0) += 1;
        }
        colors
    }
}
