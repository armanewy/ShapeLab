//! Background job contracts for the native Foundry surface.

use std::path::PathBuf;
use std::{collections::BTreeMap, fs};

use shape_compile::export::write_model_package;
use shape_core::Aabb;
use shape_foundry::{
    ControlValue, FoundryAssetDocument, FoundryBuildStamp, FoundryCatalogResolver, FoundryCommand,
    FoundryCompilationOutput, FoundryEdit, FoundryPackCompilationOutput, FoundryPackDocument,
    FoundryStyleChangeContext, SharedProviderPolicy, apply_foundry_command,
    apply_foundry_command_with_style_context, compile_foundry_document, compile_foundry_pack,
    resolve_foundry_catalog,
};
use shape_mesh::TriangleMesh;
use shape_render::foundry::{
    FoundryPreviewBatchRequest, FoundryPreviewCache, FoundryPreviewKind, FoundryPreviewRequest,
    FoundryPreviewResolution,
};
use shape_render::{OrbitCamera, RenderSettings, fit_camera_to_bounds, render_mesh};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRequest,
    generate_foundry_candidate_plans,
};

use super::view_model::{FoundryCandidateCard, FoundryPackView};

const CURRENT_PREVIEW_ID: &str = "current";
const CANDIDATE_PREVIEW_PIXELS: u32 = 128;

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
            Self::GenerateCandidates { request, .. } => Some(request.mode),
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
            Self::GenerateCandidates { .. } => FoundryJobSlot::GenerateCandidates,
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
            ..
        } => render_preview(job_id, &output, width, height, preview_cache),
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
        } => generate_foundry_candidate_plans(&document, resolver, &request)
            .map_err(|error| error.to_string())
            .and_then(|output| {
                let cards = candidate_cards_from_output_with_previews(
                    &document,
                    &output,
                    Some(request.mode),
                    None,
                    resolver,
                    preview_cache,
                )?;
                Ok(FoundryJobEvent::CandidatesGenerated {
                    job_id,
                    request,
                    output: Box::new(output),
                    cards,
                })
            }),
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
    preview_cache: &mut FoundryPreviewCache,
) -> Result<FoundryJobEvent, String> {
    if width == height
        && let Some(resolution) = FoundryPreviewResolution::from_pixels(width)
    {
        let request = foundry_preview_request(output);
        let mut batch =
            FoundryPreviewBatchRequest::new("current-document", vec![request], resolution);
        batch.max_parallel_jobs = 1;
        let rendered = preview_cache
            .render_batch(batch)
            .map_err(|error| error.to_string())?;
        let preview = rendered
            .previews
            .into_iter()
            .next()
            .ok_or_else(|| "preview batch did not return an image".to_owned())?;
        return Ok(FoundryJobEvent::PreviewRendered {
            job_id,
            preview_id: preview.preview_id,
            rgba8: preview.image.rgba8,
            width: preview.image.width,
            height: preview.image.height,
            camera: preview.camera,
            build: Some(output.build_stamp.clone()),
        });
    }

    let mesh = preview_mesh_from_output(output);
    let camera = fit_camera_to_bounds(mesh.bounds);
    let settings = RenderSettings {
        width,
        height,
        ..RenderSettings::default()
    };
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
    render_preview(job_id, &output, width, height, preview_cache)
}

fn foundry_preview_request(output: &FoundryCompilationOutput) -> FoundryPreviewRequest {
    preview_request_for_output(
        CURRENT_PREVIEW_ID,
        FoundryPreviewKind::ChangedRoleOverlay {
            role: "current".to_owned(),
        },
        output,
    )
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
            let validation_label = if candidate.conformance.accepted {
                "Accepted".to_owned()
            } else {
                "Rejected".to_owned()
            };
            let validation_detail = (!candidate.conformance.accepted).then(|| {
                format!(
                    "{} required failure(s), {} advisory issue(s)",
                    candidate.conformance.required_failure_count,
                    candidate.conformance.advisory_issue_count
                )
            });
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
                rejections: BTreeMap::new(),
                validation_label,
                validation_detail,
                selectable: candidate.conformance.accepted,
                selected: selected.is_some_and(|selected| selected == &candidate.id),
            }
        })
        .collect()
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
    let parent_request = preview_request_for_output(
        "candidate-parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        &parent_output,
    );
    let mut requests = vec![parent_request.clone()];
    let mut candidate_requests = BTreeMap::new();
    for (index, candidate) in output.candidates.iter().enumerate() {
        match compile_foundry_document(&candidate.document, resolver) {
            Ok(candidate_output) => {
                let preview_id = format!("candidate-{}", candidate.id.0);
                let request = preview_request_for_output(
                    preview_id.clone(),
                    FoundryPreviewKind::CandidateCard {
                        candidate_id: candidate.id.0.clone(),
                    },
                    &candidate_output,
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

    let mut batch = FoundryPreviewBatchRequest::new(
        "candidate-directions",
        requests,
        FoundryPreviewResolution::from_pixels(CANDIDATE_PREVIEW_PIXELS)
            .expect("candidate preview size is supported"),
    );
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

        return Ok(cards);
    }

    for (preview_id, (index, request)) in candidate_requests {
        let mut batch = FoundryPreviewBatchRequest::new(
            format!("candidate-direction-{preview_id}"),
            vec![parent_request.clone(), request],
            FoundryPreviewResolution::from_pixels(CANDIDATE_PREVIEW_PIXELS)
                .expect("candidate preview size is supported"),
        );
        batch.max_parallel_jobs = 2;
        match preview_cache.render_batch(batch) {
            Ok(rendered) => {
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

    Ok(cards)
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
