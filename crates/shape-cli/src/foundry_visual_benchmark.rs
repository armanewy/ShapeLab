use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, bail};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use shape_compile::CompileValidationReport;
use shape_compile::export::{verify_model_package, write_grouped_obj_export, write_model_package};
use shape_compile::validation::{
    ModelValidationReport, ValidationLimits, validate_model,
    validation_config_from_recipe_with_limits,
};
use shape_foundry::{
    ControlDeltaExplanation, ControlEvaluationContext, ControlKind, ControlTopologyBehavior,
    ControlValue, CustomizerControl, CustomizerProfile, FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION,
    FoundryAssetDocument, FoundryCandidateId, FoundryCompilationOutput, FoundryDocumentId,
    FoundryPackDocument, FoundryPackExportProfile, FoundryUsabilityEvent, FoundryUsabilityLog,
    FoundryUsabilityMetrics, FoundryUsabilityRecord, PackCoherencePolicy, SharedProviderPolicy,
    compile_foundry_document, compile_foundry_pack, default_control_value,
    effective_control_domain, evaluate_control_state, explain_control_delta,
    whole_model_preview_sample_requests_with_count,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, expanded_profiles, roman_bridge, scifi_crate, stylized_lamp,
};
use shape_mesh::TriangleMesh;
use shape_render::foundry::{
    FoundryChangedRoleOverlay, FoundryPreviewBatchRequest, FoundryPreviewCache,
    FoundryPreviewCacheStats, FoundryPreviewControlValue, FoundryPreviewKind,
    FoundryPreviewRequest, FoundryPreviewResolution, render_foundry_previews,
};
use shape_render::{OrbitCamera, RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh};
use shape_search::foundry::{
    FoundryCandidateChangeKind, FoundryCandidateControlChange, FoundryCandidateDiagnostics,
    FoundryCandidateGenerationDiagnostics, FoundryCandidateMode, FoundryCandidateOutput,
    FoundryCandidatePlan, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

use crate::{render_mesh_from_triangles, save_contact_sheet, save_png, write_json};

const DEFAULT_BENCHMARK_SEED: u64 = 42;
const REQUIRED_CANDIDATE_COUNT: usize = 6;
const DEFAULT_FOUNDRY_PROPOSALS: usize = 72;
const MIN_FOUNDRY_PROPOSALS: usize = 24;
const MAX_FOUNDRY_PROPOSALS: usize = 72;
const FULL_PREVIEW_SIZE: u32 = 512;
const CONTROL_SAMPLE_COUNT: usize = 5;
const CANDIDATE_RENDER_PARALLELISM: usize = 4;
const DEFAULT_WINDOWS_BLENDER_EXE: &str =
    r"C:\Program Files\Blender Foundation\Blender 4.5\blender.exe";

#[derive(Debug, clap::Args)]
pub struct FoundryVisualBenchmarkArgs {
    /// Built-in foundry benchmark profile.
    #[arg(long, value_enum)]
    profile: FoundryVisualBenchmarkProfile,
    /// Deterministic foundry candidate seed.
    #[arg(long, default_value_t = DEFAULT_BENCHMARK_SEED)]
    seed: u64,
    /// Number of raw proposal programs to evaluate per required candidate mode.
    #[arg(long, default_value_t = DEFAULT_FOUNDRY_PROPOSALS)]
    proposal_count: usize,
    /// Output profile directory.
    #[arg(long)]
    out_dir: PathBuf,
    /// Optional Blender executable used for representative create/reopen verification.
    #[arg(long)]
    blender_exe: Option<PathBuf>,
    /// Skip Blender runtime verification even when Blender is installed.
    #[arg(long)]
    skip_blender: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum FoundryVisualBenchmarkProfile {
    #[value(name = "roman-bridge")]
    RomanBridge,
    #[value(name = "roman-bridge-hq", alias = "roman-bridge-hq-v1")]
    RomanBridgeHq,
    #[value(name = "sci-fi-crate", alias = "scifi-crate")]
    ScifiCrate,
    #[value(name = "stylized-lamp")]
    StylizedLamp,
    #[value(name = "market-stall")]
    MarketStall,
    #[value(name = "sci-fi-door")]
    SciFiDoor,
    #[value(name = "storage-barrel")]
    StorageBarrel,
    #[value(name = "signpost")]
    Signpost,
    #[value(name = "workshop-chair")]
    WorkshopChair,
    #[value(name = "handcart")]
    Handcart,
    #[value(name = "stylized-tree")]
    StylizedTree,
}

impl FoundryVisualBenchmarkProfile {
    fn slug(self) -> &'static str {
        match self {
            Self::RomanBridge => "roman-bridge",
            Self::RomanBridgeHq => "roman-bridge-hq",
            Self::ScifiCrate => "sci-fi-crate",
            Self::StylizedLamp => "stylized-lamp",
            Self::MarketStall => "market-stall",
            Self::SciFiDoor => "sci-fi-door",
            Self::StorageBarrel => "storage-barrel",
            Self::Signpost => "signpost",
            Self::WorkshopChair => "workshop-chair",
            Self::Handcart => "handcart",
            Self::StylizedTree => "stylized-tree",
        }
    }

    fn fixture(self) -> FoundryFixtureCatalog {
        match self {
            Self::RomanBridge => roman_bridge::fixture_catalog(),
            Self::RomanBridgeHq => roman_bridge::hq_fixture_catalog(),
            Self::ScifiCrate => scifi_crate::fixture_catalog(),
            Self::StylizedLamp => stylized_lamp::fixture_catalog(),
            Self::MarketStall => expanded_profiles::market_stall_fixture_catalog(),
            Self::SciFiDoor => expanded_profiles::scifi_door_fixture_catalog(),
            Self::StorageBarrel => expanded_profiles::barrel_fixture_catalog(),
            Self::Signpost => expanded_profiles::signpost_fixture_catalog(),
            Self::WorkshopChair => expanded_profiles::chair_fixture_catalog(),
            Self::Handcart => expanded_profiles::handcart_fixture_catalog(),
            Self::StylizedTree => expanded_profiles::stylized_tree_fixture_catalog(),
        }
    }
}

#[derive(Debug)]
struct CompiledCandidate {
    plan: BenchmarkCandidatePlan,
    output: FoundryCompilationOutput,
    model_validation: ModelValidationReport,
}

#[derive(Debug, Clone)]
struct BenchmarkCandidatePlan {
    id: FoundryCandidateId,
    label: String,
    document: FoundryAssetDocument,
    changed_controls: Vec<String>,
    diagnostics: FoundryCandidateDiagnostics,
    recipe_fingerprint: Option<String>,
}

impl From<FoundryCandidatePlan> for BenchmarkCandidatePlan {
    fn from(value: FoundryCandidatePlan) -> Self {
        Self {
            id: value.id,
            label: value.label,
            document: value.document,
            changed_controls: value.changed_controls,
            diagnostics: value.diagnostics,
            recipe_fingerprint: Some(value.recipe_fingerprint),
        }
    }
}

#[derive(Debug)]
struct CandidateModeArtifacts {
    summary: CandidateModeSummary,
    first_candidate_document: Option<FoundryAssetDocument>,
}

#[derive(Debug, Copy, Clone)]
struct CandidateModeRenderRequest<'a> {
    profile_dir: &'a Path,
    mode_dir_name: &'a str,
    mode: FoundryCandidateMode,
    seed: u64,
    proposal_count: usize,
    required_count: Option<usize>,
}

#[derive(Debug, Copy, Clone)]
struct BenchmarkRenderInputs<'a> {
    fixture: &'a FoundryFixtureCatalog,
    parent_output: &'a FoundryCompilationOutput,
}

#[derive(Debug, Serialize)]
struct CandidateModeFile {
    mode: String,
    seed: u64,
    proposal_count: usize,
    requested_count: usize,
    parent_fingerprint: String,
    shared_camera: CameraSummary,
    diagnostics: Option<FoundryCandidateGenerationDiagnostics>,
    candidates: Vec<CandidateCardSummary>,
    unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateModeSummary {
    mode: String,
    seed: u64,
    proposal_count: usize,
    requested_count: usize,
    returned_count: usize,
    diagnostics: Option<FoundryCandidateGenerationDiagnostics>,
    unavailable_reason: Option<String>,
}

#[derive(Debug, Serialize)]
struct CandidateCardSummary {
    slot: usize,
    id: FoundryCandidateId,
    label: String,
    image: String,
    changed_controls: Vec<String>,
    changed_roles: Vec<RoleChangeSummary>,
    explanations: Vec<CandidateExplanation>,
    recipe_fingerprint: String,
    artifact_fingerprint: String,
    conformance_accepted: bool,
    model_validation_valid: bool,
    visual_delta_from_parent: u64,
    mesh: MeshSummary,
}

#[derive(Debug, Clone, Serialize)]
struct CandidateExplanation {
    control_id: String,
    control_label: String,
    kind: String,
    before: String,
    after: String,
    message: String,
    details: Vec<ControlDeltaExplanation>,
    topology_changing: bool,
}

#[derive(Debug, Clone, Serialize)]
struct RoleChangeSummary {
    role: String,
    previous_provider: Option<String>,
    current_provider: Option<String>,
    changed_controls: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ControlStripFile {
    control_id: String,
    label: String,
    control_kind: String,
    topology_behavior: String,
    sample_policy: String,
    sample_count: usize,
    shared_camera: CameraSummary,
    visual_delta_from_parent: u64,
    samples: Vec<ControlSampleSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct ControlStripSummary {
    control_id: String,
    label: String,
    control_kind: String,
    topology_behavior: String,
    sample_policy: String,
    sample_count: usize,
    visual_delta_from_parent: u64,
    geometry_fingerprints: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ControlSampleSummary {
    index: usize,
    value: ControlValue,
    label: String,
    image: String,
    geometry_fingerprint: String,
    model_validation_valid: bool,
    visual_delta_from_parent: u64,
}

#[derive(Debug, Serialize)]
struct OptionGalleryFile {
    control_id: String,
    label: String,
    gallery_kind: String,
    role: Option<String>,
    option_count: usize,
    shared_camera: CameraSummary,
    options: Vec<OptionGalleryItemSummary>,
}

#[derive(Debug, Clone, Serialize)]
struct OptionGallerySummary {
    control_id: String,
    label: String,
    gallery_kind: String,
    role: Option<String>,
    option_count: usize,
    rendered_option_count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct OptionGalleryItemSummary {
    index: usize,
    value: ControlValue,
    label: String,
    image: String,
    geometry_fingerprint: String,
    model_validation_valid: bool,
}

#[derive(Debug, Serialize)]
struct ParentSummary {
    preview: String,
    wireframe_preview: String,
    package_dir: String,
    package_verification: String,
    blender_verification: BlenderVerificationSummary,
    mesh: MeshSummary,
}

#[derive(Debug, Clone, Serialize)]
struct BlenderVerificationSummary {
    representative: String,
    script_exported: bool,
    package_checksums_match: bool,
    package_topology_matches_manifest: bool,
    package_numeric_payloads_finite: bool,
    runtime_status: String,
    runtime_report: Option<BlenderRuntimeReport>,
    blender_runtime_required_for_gate: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct BlenderRuntimeReport {
    objects: u64,
    schema_version: u64,
    blend: String,
    verify_reopen: bool,
}

#[derive(Debug, Clone, Serialize)]
struct PackExportSummary {
    directory: String,
    pack_document: String,
    pack_report: String,
    member_count: usize,
    report_fingerprint: String,
    package_verifications: Vec<String>,
}

#[derive(Debug, Serialize)]
struct BenchmarkMetrics {
    profile: String,
    fixture_slug: String,
    seed: u64,
    proposal_count: usize,
    required_candidate_count: usize,
    parent: ParentSummary,
    candidate_modes: Vec<CandidateModeSummary>,
    primary_controls: Vec<ControlStripSummary>,
    option_galleries: Vec<OptionGallerySummary>,
    all_primary_controls_measurable: bool,
    invalid_primary_controls: Vec<String>,
    invalid_state_became_current: bool,
    provider_options_total: usize,
    provider_options_rendered: usize,
    preview_cache: PreviewCacheSummary,
    usability: FoundryUsabilityMetrics,
    coherent_pack: PackExportSummary,
    advanced_recipe_required: bool,
    deterministic_replay: DeterministicReplaySummary,
}

#[derive(Debug, Clone, Serialize)]
struct PreviewCacheSummary {
    len: usize,
    capacity: usize,
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[derive(Debug, Clone, Serialize)]
struct DeterministicReplaySummary {
    wall_clock_timings_written: bool,
    absolute_paths_written: bool,
    fixed_seeded_generation: bool,
    repeated_output_expected_identical: bool,
}

#[derive(Debug, Serialize)]
struct BenchmarkConformanceFile<'a> {
    summary: &'a shape_foundry::FoundryConformanceSummary,
    final_conformance: &'a shape_family_compile::conformance::FamilyConformanceReport,
}

#[derive(Debug, Serialize)]
struct BenchmarkValidationFile<'a> {
    compile_validation: &'a CompileValidationReport,
    model_validation: &'a ModelValidationReport,
    model_valid: bool,
}

#[derive(Debug, Copy, Clone, Serialize)]
struct MeshSummary {
    vertices: usize,
    triangles: usize,
    parts: u64,
}

#[derive(Debug, Copy, Clone, Serialize)]
struct CameraSummary {
    target: [f32; 3],
    yaw_degrees: f32,
    pitch_degrees: f32,
    distance: f32,
    vertical_fov_degrees: f32,
}

pub fn run_foundry_visual_benchmark(args: FoundryVisualBenchmarkArgs) -> anyhow::Result<()> {
    let proposal_count = args
        .proposal_count
        .clamp(MIN_FOUNDRY_PROPOSALS, MAX_FOUNDRY_PROPOSALS);
    let fixture = args.profile.fixture();
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    let parent_output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("parent foundry compilation failed: {error:#?}"))?;
    let parent_model_validation = validate_compiled_foundry_output(&parent_output, "parent")?;

    write_json(args.out_dir.join("source-document.json"), &fixture.document)?;
    write_json(
        args.out_dir.join("catalog-lock.json"),
        &parent_output.catalog.catalog_lock,
    )?;
    write_json(
        args.out_dir.join("customizer-profile.json"),
        &parent_output.catalog.customizer_profile,
    )?;
    write_json(
        args.out_dir.join("conformance.json"),
        &BenchmarkConformanceFile {
            summary: &parent_output.conformance_summary,
            final_conformance: &parent_output.final_conformance,
        },
    )?;
    write_json(
        args.out_dir.join("validation.json"),
        &BenchmarkValidationFile {
            compile_validation: &parent_output.artifact.validation_report,
            model_validation: &parent_model_validation,
            model_valid: parent_model_validation.is_valid(),
        },
    )?;

    let parent_summary = write_parent_outputs(
        &args.out_dir,
        &parent_output,
        args.blender_exe.as_deref(),
        args.skip_blender,
    )?;
    let mut preview_cache = FoundryPreviewCache::new(512);

    let render_inputs = BenchmarkRenderInputs {
        fixture: &fixture,
        parent_output: &parent_output,
    };
    let refine = render_candidate_mode(
        CandidateModeRenderRequest {
            profile_dir: &args.out_dir,
            mode_dir_name: "refine",
            mode: FoundryCandidateMode::Refine,
            seed: args.seed ^ 0x729f_0001,
            proposal_count,
            required_count: Some(REQUIRED_CANDIDATE_COUNT),
        },
        render_inputs,
        &mut preview_cache,
    )?;
    let explore = render_candidate_mode(
        CandidateModeRenderRequest {
            profile_dir: &args.out_dir,
            mode_dir_name: "explore",
            mode: FoundryCandidateMode::Explore,
            seed: args.seed ^ 0x729f_0002,
            proposal_count,
            required_count: Some(REQUIRED_CANDIDATE_COUNT),
        },
        render_inputs,
        &mut preview_cache,
    )?;

    let auxiliary_proposals = proposal_count.min(MIN_FOUNDRY_PROPOSALS);
    let silhouette = render_candidate_mode(
        CandidateModeRenderRequest {
            profile_dir: &args.out_dir,
            mode_dir_name: "silhouette",
            mode: FoundryCandidateMode::Silhouette,
            seed: args.seed ^ 0x729f_0003,
            proposal_count: auxiliary_proposals,
            required_count: None,
        },
        render_inputs,
        &mut preview_cache,
    )?;
    let structure = render_candidate_mode(
        CandidateModeRenderRequest {
            profile_dir: &args.out_dir,
            mode_dir_name: "structure",
            mode: FoundryCandidateMode::Structure,
            seed: args.seed ^ 0x729f_0004,
            proposal_count: auxiliary_proposals,
            required_count: None,
        },
        render_inputs,
        &mut preview_cache,
    )?;
    let detail = render_candidate_mode(
        CandidateModeRenderRequest {
            profile_dir: &args.out_dir,
            mode_dir_name: "detail",
            mode: FoundryCandidateMode::Detail,
            seed: args.seed ^ 0x729f_0005,
            proposal_count: auxiliary_proposals,
            required_count: None,
        },
        render_inputs,
        &mut preview_cache,
    )?;

    let control_summaries =
        render_control_strips(&args.out_dir, &fixture, &parent_output, &mut preview_cache)?;
    let option_galleries =
        render_option_galleries(&args.out_dir, &fixture, &parent_output, &mut preview_cache)?;

    let pack_summary = export_coherent_pack(
        &args.out_dir,
        args.profile.slug(),
        &fixture,
        &parent_output.document,
        refine.first_candidate_document.as_ref(),
        explore.first_candidate_document.as_ref(),
    )?;

    let invalid_primary_controls = invalid_primary_controls(&parent_output);
    if !invalid_primary_controls.is_empty() {
        bail!("primary controls with no valid current state: {invalid_primary_controls:?}");
    }
    let all_primary_controls_measurable = control_summaries
        .iter()
        .all(|control| control.visual_delta_from_parent > 0);
    if !all_primary_controls_measurable {
        let offenders = control_summaries
            .iter()
            .filter(|control| control.visual_delta_from_parent == 0)
            .map(|control| control.control_id.as_str())
            .collect::<Vec<_>>();
        bail!("primary controls did not produce measurable visual change: {offenders:?}");
    }

    let provider_options_total = option_galleries
        .iter()
        .filter(|gallery| gallery.gallery_kind == "provider")
        .map(|gallery| gallery.option_count)
        .sum::<usize>();
    let provider_options_rendered = option_galleries
        .iter()
        .filter(|gallery| gallery.gallery_kind == "provider")
        .map(|gallery| gallery.rendered_option_count)
        .sum::<usize>();
    if provider_options_total != provider_options_rendered {
        bail!(
            "provider gallery did not render all options: rendered {provider_options_rendered} of {provider_options_total}"
        );
    }

    let mut usability = FoundryUsabilityLog::new();
    usability.record(FoundryUsabilityRecord::new(
        0,
        FoundryUsabilityEvent::ProfileOpened,
    ));
    usability.record(FoundryUsabilityRecord::new(
        100,
        FoundryUsabilityEvent::BuildCompleted,
    ));
    usability.record(FoundryUsabilityRecord::new(
        300,
        FoundryUsabilityEvent::CandidateRequest {
            requested_count: (REQUIRED_CANDIDATE_COUNT * 2) as u32,
        },
    ));
    usability.record(FoundryUsabilityRecord::new(
        700,
        FoundryUsabilityEvent::CandidateSurvival {
            survived_count: (refine.summary.returned_count + explore.summary.returned_count) as u32,
        },
    ));
    usability.record(FoundryUsabilityRecord::new(
        900,
        FoundryUsabilityEvent::CandidateAccepted { accepted_count: 1 },
    ));
    usability.record(FoundryUsabilityRecord::new(
        1_200,
        FoundryUsabilityEvent::Export,
    ));

    let cache = preview_cache.stats();
    let metrics = BenchmarkMetrics {
        profile: args.profile.slug().to_owned(),
        fixture_slug: fixture.slug,
        seed: args.seed,
        proposal_count,
        required_candidate_count: REQUIRED_CANDIDATE_COUNT,
        parent: parent_summary,
        candidate_modes: vec![
            refine.summary,
            explore.summary,
            silhouette.summary,
            structure.summary,
            detail.summary,
        ],
        primary_controls: control_summaries,
        option_galleries,
        all_primary_controls_measurable,
        invalid_primary_controls,
        invalid_state_became_current: false,
        provider_options_total,
        provider_options_rendered,
        preview_cache: PreviewCacheSummary::from(cache),
        usability: usability.metrics(),
        coherent_pack: pack_summary,
        advanced_recipe_required: false,
        deterministic_replay: DeterministicReplaySummary {
            wall_clock_timings_written: false,
            absolute_paths_written: false,
            fixed_seeded_generation: true,
            repeated_output_expected_identical: true,
        },
    };
    write_json(args.out_dir.join("metrics.json"), &metrics)?;

    println!(
        "Rendered foundry visual benchmark {} to {}",
        args.profile.slug(),
        args.out_dir.display()
    );
    Ok(())
}

fn render_candidate_mode(
    request: CandidateModeRenderRequest<'_>,
    inputs: BenchmarkRenderInputs<'_>,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<CandidateModeArtifacts> {
    let mode_dir = request.profile_dir.join(request.mode_dir_name);
    fs::create_dir_all(&mode_dir).with_context(|| format!("creating {}", mode_dir.display()))?;
    let candidate_request = FoundryCandidateRequest {
        seed: request.seed,
        proposal_count: request.proposal_count,
        result_count: request.required_count.unwrap_or(REQUIRED_CANDIDATE_COUNT),
        mode: request.mode,
        strategy_id: None,
        preference_profile: None,
        variation_intent: shape_foundry::VariationIntent::default(),
    };
    let output = match generate_foundry_candidate_plans(
        &inputs.fixture.document,
        inputs.fixture,
        &candidate_request,
    ) {
        Ok(output) => output,
        Err(error) if request.required_count.is_none() => {
            return write_empty_candidate_mode(
                &mode_dir,
                request,
                inputs.parent_output,
                cache,
                format!("{error}"),
            );
        }
        Err(error) => {
            return Err(anyhow::anyhow!(
                "foundry {:?} candidate search failed: {error}",
                request.mode
            ));
        }
    };

    render_candidate_output(&mode_dir, request, output, inputs, cache)
}

fn write_empty_candidate_mode(
    mode_dir: &Path,
    request: CandidateModeRenderRequest<'_>,
    parent_output: &FoundryCompilationOutput,
    cache: &mut FoundryPreviewCache,
    unavailable_reason: String,
) -> anyhow::Result<CandidateModeArtifacts> {
    let batch = render_preview_batch(
        cache,
        request.mode_dir_name,
        vec![preview_request_for_output(
            "parent",
            FoundryPreviewKind::CandidateCard {
                candidate_id: "parent".to_owned(),
            },
            parent_output,
            &parent_output.catalog.customizer_profile,
            Vec::new(),
        )],
    )?;
    save_contact_sheet(
        &batch.previews[0].image,
        &[],
        mode_dir.join("contact-sheet.png"),
    )?;
    let file = CandidateModeFile {
        mode: request.mode_dir_name.to_owned(),
        seed: request.seed,
        proposal_count: request.proposal_count,
        requested_count: REQUIRED_CANDIDATE_COUNT,
        parent_fingerprint: geometry_fingerprint(parent_output),
        shared_camera: CameraSummary::from(&batch.camera),
        diagnostics: None,
        candidates: Vec::new(),
        unavailable_reason: Some(unavailable_reason.clone()),
    };
    write_json(mode_dir.join("candidates.json"), &file)?;
    Ok(CandidateModeArtifacts {
        summary: CandidateModeSummary {
            mode: request.mode_dir_name.to_owned(),
            seed: request.seed,
            proposal_count: request.proposal_count,
            requested_count: REQUIRED_CANDIDATE_COUNT,
            returned_count: 0,
            diagnostics: None,
            unavailable_reason: Some(unavailable_reason),
        },
        first_candidate_document: None,
    })
}

fn render_candidate_output(
    mode_dir: &Path,
    request: CandidateModeRenderRequest<'_>,
    candidate_output: FoundryCandidateOutput,
    inputs: BenchmarkRenderInputs<'_>,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<CandidateModeArtifacts> {
    let mode_dir_name = request.mode_dir_name;
    let parent_output = inputs.parent_output;
    let mut plans = candidate_output
        .candidates
        .into_iter()
        .map(BenchmarkCandidatePlan::from)
        .collect::<Vec<_>>();
    if let Some(required_count) = request.required_count
        && plans.len() < required_count
    {
        supplement_candidate_plans(
            mode_dir_name,
            request.mode,
            required_count,
            &mut plans,
            inputs.fixture,
            parent_output,
        )?;
    }
    if let Some(required_count) = request.required_count
        && plans.len() != required_count
    {
        bail!(
            "{mode_dir_name} generated {} candidate(s), expected {required_count}; diagnostics: {:#?}",
            plans.len(),
            candidate_output.diagnostics
        );
    }

    let mut compiled = compile_valid_candidate_plans(plans, inputs.fixture);
    if let Some(required_count) = request.required_count
        && compiled.len() < required_count
    {
        let mut valid_plans = compiled
            .iter()
            .map(|candidate| candidate.plan.clone())
            .collect::<Vec<_>>();
        let start = valid_plans.len();
        supplement_candidate_plans(
            mode_dir_name,
            request.mode,
            required_count,
            &mut valid_plans,
            inputs.fixture,
            parent_output,
        )?;
        let supplemental = valid_plans.into_iter().skip(start).collect::<Vec<_>>();
        compiled.extend(compile_valid_candidate_plans(supplemental, inputs.fixture));
    }
    if let Some(required_count) = request.required_count {
        if compiled.len() < required_count {
            bail!(
                "{mode_dir_name} produced {} valid candidate(s), expected {required_count}; diagnostics: {:#?}",
                compiled.len(),
                candidate_output.diagnostics
            );
        }
        compiled.truncate(required_count);
    }

    let mut preview_requests = Vec::with_capacity(compiled.len() + 1);
    preview_requests.push(preview_request_for_output(
        "parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        parent_output,
        &parent_output.catalog.customizer_profile,
        Vec::new(),
    ));
    for (slot, candidate) in compiled.iter().enumerate() {
        let overlays = changed_role_overlays(
            &parent_output.document,
            &candidate.plan.document,
            &parent_output.catalog.customizer_profile,
            &candidate.plan.changed_controls,
        );
        preview_requests.push(preview_request_for_output(
            format!("{}-candidate-{slot:02}", mode_dir_name),
            FoundryPreviewKind::CandidateCard {
                candidate_id: candidate.plan.id.0.clone(),
            },
            &candidate.output,
            &parent_output.catalog.customizer_profile,
            overlays,
        ));
    }
    let batch = render_preview_batch(cache, mode_dir_name, preview_requests)?;
    let parent_image = &batch.previews[0].image;
    let candidate_refs = batch.previews[1..]
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_contact_sheet(
        parent_image,
        &candidate_refs,
        mode_dir.join("contact-sheet.png"),
    )?;
    save_png(parent_image, mode_dir.join("parent.png"))?;

    let mut summaries = Vec::with_capacity(compiled.len());
    for (slot, candidate) in compiled.iter().enumerate() {
        let preview = &batch.previews[slot + 1];
        let image = format!("candidate-{slot:02}.png");
        save_png(&preview.image, mode_dir.join(&image))?;
        summaries.push(CandidateCardSummary {
            slot,
            id: candidate.plan.id.clone(),
            label: candidate.plan.label.clone(),
            image,
            changed_controls: candidate.plan.changed_controls.clone(),
            changed_roles: preview
                .changed_role_overlays
                .iter()
                .map(RoleChangeSummary::from)
                .collect(),
            explanations: candidate_explanations(&candidate.plan.diagnostics),
            recipe_fingerprint: candidate.output.build_stamp.recipe_fingerprint.0.to_hex(),
            artifact_fingerprint: artifact_fingerprint(&candidate.output),
            conformance_accepted: candidate.output.conformance_summary.accepted,
            model_validation_valid: candidate.model_validation.is_valid(),
            visual_delta_from_parent: image_delta(parent_image, &preview.image),
            mesh: artifact_mesh_summary(&candidate.output),
        });
    }

    let first_candidate_document = compiled
        .first()
        .map(|candidate| candidate.plan.document.clone());
    let diagnostics = Some(candidate_output.diagnostics.clone());
    let file = CandidateModeFile {
        mode: mode_dir_name.to_owned(),
        seed: request.seed,
        proposal_count: request.proposal_count,
        requested_count: REQUIRED_CANDIDATE_COUNT,
        parent_fingerprint: geometry_fingerprint(parent_output),
        shared_camera: CameraSummary::from(&batch.camera),
        diagnostics: diagnostics.clone(),
        candidates: summaries,
        unavailable_reason: None,
    };
    write_json(mode_dir.join("candidates.json"), &file)?;

    Ok(CandidateModeArtifacts {
        summary: CandidateModeSummary {
            mode: mode_dir_name.to_owned(),
            seed: request.seed,
            proposal_count: request.proposal_count,
            requested_count: REQUIRED_CANDIDATE_COUNT,
            returned_count: compiled.len(),
            diagnostics,
            unavailable_reason: None,
        },
        first_candidate_document,
    })
}

fn compile_valid_candidate_plans(
    plans: Vec<BenchmarkCandidatePlan>,
    fixture: &FoundryFixtureCatalog,
) -> Vec<CompiledCandidate> {
    plans
        .into_iter()
        .filter_map(|plan| {
            let output = compile_foundry_document(&plan.document, fixture).ok()?;
            let model_validation = validate_compiled_foundry_output(&output, &plan.id.0).ok()?;
            Some(CompiledCandidate {
                plan,
                output,
                model_validation,
            })
        })
        .collect()
}

fn supplement_candidate_plans(
    mode_dir_name: &str,
    mode: FoundryCandidateMode,
    required_count: usize,
    plans: &mut Vec<BenchmarkCandidatePlan>,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
) -> anyhow::Result<()> {
    let profile = &parent_output.catalog.customizer_profile;
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    let mut seen_states = plans
        .iter()
        .filter_map(|plan| serde_json::to_string(&plan.document.control_state).ok())
        .collect::<BTreeSet<_>>();
    let mut seen_recipes = plans
        .iter()
        .filter_map(|plan| plan.recipe_fingerprint.clone())
        .collect::<BTreeSet<_>>();
    seen_recipes.insert(parent_output.build_stamp.recipe_fingerprint.0.to_hex());

    let mut controls = profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| left.id.cmp(&right.id));

    let mut supplement_index = 0_usize;
    for control in controls {
        if mode == FoundryCandidateMode::Refine
            && control.topology_behavior != ControlTopologyBehavior::TopologyPreserving
        {
            continue;
        }
        let current = current_control_value(&parent_output.document, control, context)?;
        let values = control_sample_values(control, context)?;
        for value in values {
            if value == current {
                continue;
            }
            let mut document = parent_output.document.clone();
            document.catalog_lock = None;
            document.build_stamp = None;
            document
                .control_state
                .insert(control.id.clone(), value.clone());
            let state_key = serde_json::to_string(&document.control_state)
                .context("serializing supplement control state")?;
            if !seen_states.insert(state_key) {
                continue;
            }
            let output = match compile_foundry_document(&document, fixture) {
                Ok(output) => output,
                Err(_) => continue,
            };
            if validate_compiled_foundry_output(&output, &control.id).is_err() {
                continue;
            }
            let recipe_fingerprint = output.build_stamp.recipe_fingerprint.0.to_hex();
            if !seen_recipes.insert(recipe_fingerprint.clone()) {
                continue;
            }
            if geometry_fingerprint(&output) == geometry_fingerprint(parent_output) {
                continue;
            }
            let diagnostics = supplement_candidate_diagnostics(
                profile,
                context,
                control,
                current.clone(),
                value.clone(),
            )?;
            plans.push(BenchmarkCandidatePlan {
                id: FoundryCandidateId(format!(
                    "foundry-supplement-{mode_dir_name}-{supplement_index:02}"
                )),
                label: format!(
                    "{} supplement: {} {}",
                    mode_dir_name,
                    control.label,
                    control_value_label(control, &value)
                ),
                document,
                changed_controls: vec![control.id.clone()],
                diagnostics,
                recipe_fingerprint: Some(recipe_fingerprint),
            });
            supplement_index += 1;
            if plans.len() == required_count {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn supplement_candidate_diagnostics(
    profile: &CustomizerProfile,
    context: ControlEvaluationContext<'_>,
    control: &CustomizerControl,
    before: ControlValue,
    after: ControlValue,
) -> anyhow::Result<FoundryCandidateDiagnostics> {
    let delta = explain_control_delta(
        profile,
        context,
        &control.id,
        Some(before.clone()),
        after.clone(),
    )
    .map_err(|error| {
        anyhow::anyhow!(
            "explaining supplement candidate for control {} failed: {error:?}",
            control.id
        )
    })?;
    let before_label = control_value_label(control, &before);
    let after_label = control_value_label(control, &after);
    Ok(FoundryCandidateDiagnostics {
        changes: vec![FoundryCandidateControlChange {
            kind: candidate_change_kind(control),
            control_id: control.id.clone(),
            control_label: control.label.clone(),
            before: before_label.clone(),
            after: after_label.clone(),
            message: format!(
                "{} changed from `{before_label}` to `{after_label}`.",
                control.label
            ),
            details: delta.explanations,
            topology_changing: control.topology_behavior
                == ControlTopologyBehavior::TopologyChanging,
        }],
    })
}

fn current_control_value(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> anyhow::Result<ControlValue> {
    document
        .control_state
        .get(&control.id)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| {
            default_control_value(control, context).map_err(|error| {
                anyhow::anyhow!("default value for control {} failed: {error:?}", control.id)
            })
        })
}

fn candidate_change_kind(control: &CustomizerControl) -> FoundryCandidateChangeKind {
    let text = format!("{} {}", control.id, control.label).to_ascii_lowercase();
    if text.contains("detail")
        || text.contains("edge")
        || text.contains("bevel")
        || text.contains("trim")
        || text.contains("corner")
    {
        return FoundryCandidateChangeKind::Detail;
    }
    match control.kind {
        ControlKind::ContinuousAxis { .. } => FoundryCandidateChangeKind::Numeric,
        ControlKind::IntegerStepper { .. } => FoundryCandidateChangeKind::Repetition,
        ControlKind::Toggle { .. } => FoundryCandidateChangeKind::Choice,
        ControlKind::ChoiceGallery { .. } => FoundryCandidateChangeKind::Choice,
        ControlKind::ProviderGallery { .. } => FoundryCandidateChangeKind::Provider,
    }
}

fn render_control_strips(
    profile_dir: &Path,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<Vec<ControlStripSummary>> {
    let strips_dir = profile_dir.join("control-strips");
    fs::create_dir_all(&strips_dir)
        .with_context(|| format!("creating {}", strips_dir.display()))?;
    let profile = &parent_output.catalog.customizer_profile;
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    let mut summaries = Vec::new();

    for control in profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        let samples = control_sample_values(control, context)?;
        if matches!(control.kind, ControlKind::ContinuousAxis { .. })
            && samples.len() != CONTROL_SAMPLE_COUNT
        {
            bail!(
                "continuous control {} produced {} samples, expected {CONTROL_SAMPLE_COUNT}",
                control.id,
                samples.len()
            );
        }
        if control.topology_behavior == ControlTopologyBehavior::TopologyChanging
            && samples.is_empty()
        {
            bail!(
                "topology-changing control {} has no discrete samples",
                control.id
            );
        }

        let control_dir = strips_dir.join(&control.id);
        fs::create_dir_all(&control_dir)
            .with_context(|| format!("creating {}", control_dir.display()))?;
        let rendered = render_control_values(
            ControlValueRenderRequest {
                output_dir: &control_dir,
                comparison_id: &format!("control-strip-{}", control.id),
                control,
                values: &samples,
                provider_gallery_kind: false,
            },
            fixture,
            parent_output,
            cache,
        )?;
        write_json(control_dir.join("samples.json"), &rendered.file)?;
        summaries.push(rendered.summary);
    }

    write_json(strips_dir.join("summary.json"), &summaries)?;
    Ok(summaries)
}

fn render_option_galleries(
    profile_dir: &Path,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<Vec<OptionGallerySummary>> {
    let galleries_dir = profile_dir.join("option-galleries");
    fs::create_dir_all(&galleries_dir)
        .with_context(|| format!("creating {}", galleries_dir.display()))?;
    let mut summaries = Vec::new();

    for control in parent_output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        let options = gallery_option_values(control);
        if options.is_empty() {
            continue;
        }
        let control_dir = galleries_dir.join(&control.id);
        fs::create_dir_all(&control_dir)
            .with_context(|| format!("creating {}", control_dir.display()))?;
        let gallery = render_gallery_values(
            &control_dir,
            &format!("option-gallery-{}", control.id),
            control,
            &options,
            fixture,
            parent_output,
            cache,
        )?;
        write_json(control_dir.join("options.json"), &gallery.file)?;
        summaries.push(gallery.summary);
    }

    write_json(galleries_dir.join("summary.json"), &summaries)?;
    Ok(summaries)
}

struct RenderedControlValues {
    file: ControlStripFile,
    summary: ControlStripSummary,
}

struct ControlValueRenderRequest<'a> {
    output_dir: &'a Path,
    comparison_id: &'a str,
    control: &'a CustomizerControl,
    values: &'a [ControlValue],
    provider_gallery_kind: bool,
}

fn render_control_values(
    request: ControlValueRenderRequest<'_>,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<RenderedControlValues> {
    let control = request.control;
    let continuous = matches!(control.kind, ControlKind::ContinuousAxis { .. });
    let mut seen_recipes = BTreeSet::new();
    let mut outputs = compile_valid_control_values(
        fixture,
        parent_output,
        control,
        request.values,
        &mut seen_recipes,
        if continuous {
            Some(CONTROL_SAMPLE_COUNT)
        } else {
            None
        },
    );
    if continuous && outputs.len() < CONTROL_SAMPLE_COUNT {
        let dense_values = authored_continuous_samples_with_count(control, 33)?;
        let remaining = CONTROL_SAMPLE_COUNT - outputs.len();
        outputs.extend(compile_valid_control_values(
            fixture,
            parent_output,
            control,
            &dense_values,
            &mut seen_recipes,
            Some(remaining),
        ));
    }
    if continuous && outputs.len() != CONTROL_SAMPLE_COUNT {
        bail!(
            "continuous control {} produced {} valid samples, expected {CONTROL_SAMPLE_COUNT}",
            control.id,
            outputs.len()
        );
    }
    if outputs.is_empty() {
        bail!("control {} produced no valid samples", control.id);
    }

    let mut requests = Vec::with_capacity(outputs.len() + 1);
    requests.push(preview_request_for_output(
        "parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        parent_output,
        &parent_output.catalog.customizer_profile,
        Vec::new(),
    ));
    for (index, (value, output, _)) in outputs.iter().enumerate() {
        let kind = if request.provider_gallery_kind {
            provider_preview_kind(control, value, index)
        } else if matches!(control.kind, ControlKind::ContinuousAxis { .. })
            && control.topology_behavior == ControlTopologyBehavior::TopologyPreserving
        {
            FoundryPreviewKind::SliderFilmstrip {
                control_id: control.id.clone(),
                sample_index: index as u32,
            }
        } else {
            FoundryPreviewKind::DiscreteStrip {
                control_id: control.id.clone(),
                value_index: index as u32,
            }
        };
        requests.push(preview_request_for_output(
            format!("{}-sample-{index:02}", control.id),
            kind,
            output,
            &parent_output.catalog.customizer_profile,
            changed_role_overlays_for_control_value(&parent_output.document, control, value),
        ));
    }

    let batch = render_preview_batch(cache, request.comparison_id, requests)?;
    let parent_image = &batch.previews[0].image;
    let sample_refs = batch.previews[1..]
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_contact_sheet(
        parent_image,
        &sample_refs,
        request.output_dir.join("contact-sheet.png"),
    )?;
    save_png(parent_image, request.output_dir.join("parent.png"))?;

    let mut samples = Vec::with_capacity(outputs.len());
    let mut fingerprints = Vec::with_capacity(outputs.len());
    let mut max_delta = 0_u64;
    for (index, (value, output, model_validation)) in outputs.iter().enumerate() {
        let preview = &batch.previews[index + 1];
        let image = format!("sample-{index:02}.png");
        save_png(&preview.image, request.output_dir.join(&image))?;
        let delta = image_delta(parent_image, &preview.image);
        max_delta = max_delta.max(delta);
        let fingerprint = geometry_fingerprint(output);
        fingerprints.push(fingerprint.clone());
        samples.push(ControlSampleSummary {
            index,
            value: value.clone(),
            label: control_value_label(control, value),
            image,
            geometry_fingerprint: fingerprint,
            model_validation_valid: model_validation.is_valid(),
            visual_delta_from_parent: delta,
        });
    }

    let file = ControlStripFile {
        control_id: control.id.clone(),
        label: control.label.clone(),
        control_kind: control_kind_name(control),
        topology_behavior: topology_behavior_name(control.topology_behavior),
        sample_policy: if matches!(control.kind, ControlKind::ContinuousAxis { .. }) {
            "five_continuous_samples".to_owned()
        } else {
            "all_discrete_values".to_owned()
        },
        sample_count: samples.len(),
        shared_camera: CameraSummary::from(&batch.camera),
        visual_delta_from_parent: max_delta,
        samples,
    };
    let summary = ControlStripSummary {
        control_id: control.id.clone(),
        label: control.label.clone(),
        control_kind: file.control_kind.clone(),
        topology_behavior: file.topology_behavior.clone(),
        sample_policy: file.sample_policy.clone(),
        sample_count: file.sample_count,
        visual_delta_from_parent: max_delta,
        geometry_fingerprints: fingerprints,
    };
    Ok(RenderedControlValues { file, summary })
}

struct RenderedGalleryValues {
    file: OptionGalleryFile,
    summary: OptionGallerySummary,
}

fn render_gallery_values(
    output_dir: &Path,
    comparison_id: &str,
    control: &CustomizerControl,
    values: &[(ControlValue, String)],
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    cache: &mut FoundryPreviewCache,
) -> anyhow::Result<RenderedGalleryValues> {
    let raw_values = values
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    let rendered = render_control_values(
        ControlValueRenderRequest {
            output_dir,
            comparison_id,
            control,
            values: &raw_values,
            provider_gallery_kind: true,
        },
        fixture,
        parent_output,
        cache,
    )?;
    let shared_camera = rendered.file.shared_camera;
    let options = rendered
        .file
        .samples
        .into_iter()
        .map(|sample| OptionGalleryItemSummary {
            index: sample.index,
            value: sample.value,
            label: sample.label,
            image: sample.image,
            geometry_fingerprint: sample.geometry_fingerprint,
            model_validation_valid: sample.model_validation_valid,
        })
        .collect::<Vec<_>>();
    let role = provider_control_role(control);
    let gallery_kind = gallery_kind(control);
    let file = OptionGalleryFile {
        control_id: control.id.clone(),
        label: control.label.clone(),
        gallery_kind: gallery_kind.clone(),
        role: role.clone(),
        option_count: values.len(),
        shared_camera,
        options,
    };
    let summary = OptionGallerySummary {
        control_id: control.id.clone(),
        label: control.label.clone(),
        gallery_kind,
        role,
        option_count: values.len(),
        rendered_option_count: file.options.len(),
    };
    Ok(RenderedGalleryValues { file, summary })
}

fn compile_valid_control_values(
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    control: &CustomizerControl,
    values: &[ControlValue],
    seen_recipes: &mut BTreeSet<String>,
    limit: Option<usize>,
) -> Vec<(
    ControlValue,
    FoundryCompilationOutput,
    ModelValidationReport,
)> {
    let mut outputs = Vec::new();
    for value in values {
        if limit.is_some_and(|limit| outputs.len() >= limit) {
            break;
        }
        let Ok(output) =
            compile_with_control_value(fixture, &parent_output.document, control, value)
        else {
            continue;
        };
        let recipe_fingerprint = output.build_stamp.recipe_fingerprint.0.to_hex();
        if !seen_recipes.insert(recipe_fingerprint) {
            continue;
        }
        let Ok(model_validation) = validate_compiled_foundry_output(&output, &control.id) else {
            continue;
        };
        outputs.push((value.clone(), output, model_validation));
    }
    outputs
}

fn write_parent_outputs(
    profile_dir: &Path,
    output: &FoundryCompilationOutput,
    blender_exe: Option<&Path>,
    skip_blender: bool,
) -> anyhow::Result<ParentSummary> {
    let parent_dir = profile_dir.join("parent");
    fs::create_dir_all(&parent_dir)
        .with_context(|| format!("creating {}", parent_dir.display()))?;
    write_json(parent_dir.join("source-document.json"), &output.document)?;
    write_json(parent_dir.join("recipe.json"), &output.recipe)?;
    write_json(parent_dir.join("build-stamp.json"), &output.build_stamp)?;

    let preview = render_foundry_artifact(output, false)?;
    let wireframe = render_foundry_artifact(output, true)?;
    save_png(&preview, parent_dir.join("preview.png"))?;
    save_png(&wireframe, parent_dir.join("preview-wireframe.png"))?;

    let package_dir = parent_dir.join("model-package");
    let paths = write_model_package(&output.recipe, &output.artifact, &package_dir)
        .with_context(|| format!("writing parent package {}", package_dir.display()))?;
    let verification = verify_model_package(&package_dir)
        .with_context(|| format!("verifying parent package {}", package_dir.display()))?;
    write_json(parent_dir.join("package-verification.json"), &verification)?;
    let (runtime_status, runtime_report) =
        run_representative_blender_verification(&package_dir, blender_exe, skip_blender)?;
    let blender_verification = BlenderVerificationSummary {
        representative: "parent".to_owned(),
        script_exported: paths.blender_reconstruct.exists(),
        package_checksums_match: verification.checksums_match,
        package_topology_matches_manifest: verification.topology_matches_manifest,
        package_numeric_payloads_finite: verification.finite_numeric_payloads,
        runtime_status,
        runtime_report,
        blender_runtime_required_for_gate: false,
    };
    write_json(
        parent_dir.join("blender-verification.json"),
        &blender_verification,
    )?;
    let grouped_obj = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
        .context("writing parent grouped OBJ")?;
    fs::write(parent_dir.join("asset.obj"), grouped_obj.obj)
        .with_context(|| format!("writing parent asset OBJ to {}", parent_dir.display()))?;
    write_json(
        parent_dir.join("grouped-obj-report.json"),
        &grouped_obj.report,
    )?;

    Ok(ParentSummary {
        preview: "parent/preview.png".to_owned(),
        wireframe_preview: "parent/preview-wireframe.png".to_owned(),
        package_dir: "parent/model-package".to_owned(),
        package_verification: "parent/package-verification.json".to_owned(),
        blender_verification,
        mesh: artifact_mesh_summary(output),
    })
}

fn run_representative_blender_verification(
    package_dir: &Path,
    explicit_blender_exe: Option<&Path>,
    skip_blender: bool,
) -> anyhow::Result<(String, Option<BlenderRuntimeReport>)> {
    if skip_blender {
        return Ok(("skipped".to_owned(), None));
    }
    let Some(blender_exe) = resolve_blender_exe(explicit_blender_exe)? else {
        return Ok(("not_available".to_owned(), None));
    };

    let parent_dir = package_dir
        .parent()
        .context("parent model-package directory should have a parent")?;
    fs::create_dir_all(parent_dir.join("blender-check"))
        .with_context(|| format!("creating {}", parent_dir.join("blender-check").display()))?;
    let output = Command::new(&blender_exe)
        .current_dir(package_dir)
        .arg("--background")
        .arg("--python")
        .arg("blender_reconstruct.py")
        .arg("--")
        .arg("--package-dir")
        .arg(".")
        .arg("--out-dir")
        .arg(Path::new("..").join("blender-check"))
        .arg("--verify-reopen")
        .output()
        .with_context(|| {
            format!(
                "running Blender verification with {}",
                blender_exe.display()
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!(
            "representative Blender verification failed with status {:?}: {}",
            output.status.code(),
            stderr.trim()
        );
    }
    let report = parse_blender_runtime_report(&stdout).with_context(|| {
        format!(
            "Blender verification did not emit a JSON report; stdout was: {}",
            stdout.trim()
        )
    })?;
    if !report.verify_reopen {
        bail!("representative Blender verification did not reopen the saved blend");
    }
    Ok(("passed".to_owned(), Some(report)))
}

fn resolve_blender_exe(explicit_blender_exe: Option<&Path>) -> anyhow::Result<Option<PathBuf>> {
    if let Some(path) = explicit_blender_exe {
        if !path.exists() {
            bail!(
                "explicit Blender executable does not exist: {}",
                path.display()
            );
        }
        return Ok(Some(path.to_path_buf()));
    }
    if let Some(path) = std::env::var_os("SHAPE_LAB_BLENDER_EXE").map(PathBuf::from) {
        if !path.exists() {
            bail!(
                "SHAPE_LAB_BLENDER_EXE points to a missing executable: {}",
                path.display()
            );
        }
        return Ok(Some(path));
    }
    let default_windows_path = PathBuf::from(DEFAULT_WINDOWS_BLENDER_EXE);
    if default_windows_path.exists() {
        return Ok(Some(default_windows_path));
    }
    Ok(None)
}

fn parse_blender_runtime_report(stdout: &str) -> Option<BlenderRuntimeReport> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line.trim()).ok())
        .next_back()
}

fn export_coherent_pack(
    profile_dir: &Path,
    profile_slug: &str,
    fixture: &FoundryFixtureCatalog,
    parent_document: &FoundryAssetDocument,
    refine_document: Option<&FoundryAssetDocument>,
    explore_document: Option<&FoundryAssetDocument>,
) -> anyhow::Result<PackExportSummary> {
    let refine_document =
        refine_document.context("refine candidate is required for coherent pack export")?;
    let explore_document =
        explore_document.context("explore candidate is required for coherent pack export")?;
    let pack_dir = profile_dir.join("coherent-pack");
    fs::create_dir_all(&pack_dir).with_context(|| format!("creating {}", pack_dir.display()))?;

    let mut pack = FoundryPackDocument {
        schema_version: FOUNDRY_PACK_DOCUMENT_SCHEMA_VERSION,
        pack_id: format!("{profile_slug}-visual-benchmark-pack"),
        shared_family_ref: parent_document.family_content_ref.clone(),
        shared_style_ref: parent_document.style_content_ref.clone(),
        shared_locks: Vec::new(),
        shared_controls: BTreeMap::new(),
        shared_provider_policy: SharedProviderPolicy::Independent,
        members: BTreeMap::new(),
        coherence_policy: PackCoherencePolicy::ExactFamilyAndStyle,
        export_profile: FoundryPackExportProfile {
            profile: "canonical-model-package".to_owned(),
            require_all_members: true,
        },
        catalog_lock: None,
    };
    pack.members.insert(
        "parent".to_owned(),
        pack_member_document(parent_document, "parent"),
    );
    pack.members.insert(
        "refine-00".to_owned(),
        pack_member_document(refine_document, "refine-00"),
    );
    pack.members.insert(
        "explore-00".to_owned(),
        pack_member_document(explore_document, "explore-00"),
    );

    let output = compile_foundry_pack(&pack, fixture)
        .map_err(|error| anyhow::anyhow!("coherent pack compilation failed: {error:#?}"))?;
    write_json(pack_dir.join("pack-document.json"), &output.pack)?;
    write_json(pack_dir.join("pack-report.json"), &output.report)?;

    let mut package_verifications = Vec::new();
    for (member_id, member_output) in &output.member_outputs {
        let member_dir = pack_dir.join("members").join(member_id);
        fs::create_dir_all(&member_dir)
            .with_context(|| format!("creating {}", member_dir.display()))?;
        let package_dir = member_dir.join("model-package");
        write_model_package(&member_output.recipe, &member_output.artifact, &package_dir)
            .with_context(|| format!("writing pack member package {}", package_dir.display()))?;
        let verification = verify_model_package(&package_dir)
            .with_context(|| format!("verifying pack member package {}", package_dir.display()))?;
        write_json(member_dir.join("package-verification.json"), &verification)?;
        package_verifications.push(format!(
            "coherent-pack/members/{member_id}/package-verification.json"
        ));
    }

    Ok(PackExportSummary {
        directory: "coherent-pack".to_owned(),
        pack_document: "coherent-pack/pack-document.json".to_owned(),
        pack_report: "coherent-pack/pack-report.json".to_owned(),
        member_count: output.member_outputs.len(),
        report_fingerprint: output.report.report_fingerprint.to_hex(),
        package_verifications,
    })
}

fn pack_member_document(document: &FoundryAssetDocument, suffix: &str) -> FoundryAssetDocument {
    let mut document = document.clone();
    document.document_id = FoundryDocumentId(format!("{}-{suffix}", document.document_id.0));
    document.catalog_lock = None;
    document.build_stamp = None;
    document
}

fn render_foundry_artifact(
    output: &FoundryCompilationOutput,
    wireframe: bool,
) -> anyhow::Result<RenderedImage> {
    let mesh = render_mesh_from_triangles(&output.artifact.combined_preview);
    let camera = fit_camera_to_bounds(mesh.bounds);
    let settings = RenderSettings {
        width: FULL_PREVIEW_SIZE,
        height: FULL_PREVIEW_SIZE,
        wireframe,
        ..RenderSettings::default()
    };
    render_mesh(&mesh, &camera, &settings).context("rendering foundry parent preview")
}

fn render_preview_batch(
    cache: &mut FoundryPreviewCache,
    comparison_id: &str,
    items: Vec<FoundryPreviewRequest>,
) -> anyhow::Result<shape_render::foundry::FoundryPreviewBatchOutput> {
    let mut request = FoundryPreviewBatchRequest::new(
        comparison_id.to_owned(),
        items,
        FoundryPreviewResolution::Px128,
    );
    request.max_parallel_jobs = CANDIDATE_RENDER_PARALLELISM;
    render_foundry_previews(cache, request).map_err(|error| {
        anyhow::anyhow!("rendering foundry preview batch {comparison_id}: {error}")
    })
}

fn preview_request_for_output(
    preview_id: impl Into<String>,
    kind: FoundryPreviewKind,
    output: &FoundryCompilationOutput,
    profile: &CustomizerProfile,
    changed_role_overlays: Vec<FoundryChangedRoleOverlay>,
) -> FoundryPreviewRequest {
    let mut request = FoundryPreviewRequest::new(
        preview_id,
        kind,
        geometry_fingerprint(output),
        mesh_for_output(output),
    );
    request.sampled_control_state = preview_control_state(&output.document.control_state);
    request.provider_choices = effective_provider_choices(output, profile);
    request.changed_role_overlays = changed_role_overlays;
    request
}

fn mesh_for_output(output: &FoundryCompilationOutput) -> TriangleMesh {
    render_mesh_from_triangles(&output.artifact.combined_preview)
}

fn compile_with_control_value(
    fixture: &FoundryFixtureCatalog,
    parent_document: &FoundryAssetDocument,
    control: &CustomizerControl,
    value: &ControlValue,
) -> anyhow::Result<FoundryCompilationOutput> {
    let mut document = parent_document.clone();
    document.catalog_lock = None;
    document.build_stamp = None;
    document
        .control_state
        .insert(control.id.clone(), value.clone());
    compile_foundry_document(&document, fixture).map_err(|error| {
        anyhow::anyhow!(
            "control {} value {} failed foundry compilation: {error:#?}",
            control.id,
            control_value_label(control, value)
        )
    })
}

fn validate_compiled_foundry_output(
    output: &FoundryCompilationOutput,
    label: &str,
) -> anyhow::Result<ModelValidationReport> {
    if !output.artifact.validation_report.is_valid() {
        bail!(
            "{label} compile validation failed with {} issue(s)",
            output.artifact.validation_report.issues.len()
        );
    }
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    let report = validate_model(&output.artifact, &config);
    if !report.is_valid() {
        bail!(
            "{label} model validation failed with {} issue(s): {:#?}",
            report.issues.len(),
            report.issues
        );
    }
    Ok(report)
}

fn control_sample_values(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> anyhow::Result<Vec<ControlValue>> {
    if matches!(control.kind, ControlKind::ContinuousAxis { .. }) {
        if let Ok(requests) =
            whole_model_preview_sample_requests_with_count(control, context, CONTROL_SAMPLE_COUNT)
            && !requests.is_empty()
        {
            return Ok(requests.into_iter().map(|request| request.value).collect());
        }
        return authored_continuous_samples(control);
    }
    let domain =
        effective_control_domain(control, context).unwrap_or_else(|_| control.domain.clone());
    let mut values = domain
        .discrete_values
        .iter()
        .filter(|value| domain.contains_available_value(value))
        .cloned()
        .collect::<Vec<_>>();
    values.sort_by(control_value_order);
    values.dedup();
    Ok(values)
}

fn authored_continuous_samples(control: &CustomizerControl) -> anyhow::Result<Vec<ControlValue>> {
    authored_continuous_samples_with_count(control, CONTROL_SAMPLE_COUNT)
}

fn authored_continuous_samples_with_count(
    control: &CustomizerControl,
    sample_count: usize,
) -> anyhow::Result<Vec<ControlValue>> {
    let Some(first) = control.domain.continuous_intervals.first() else {
        bail!("continuous control {} has no authored interval", control.id);
    };
    let last = control.domain.continuous_intervals.last().unwrap_or(first);
    let minimum = first.minimum;
    let maximum = last.maximum;
    if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
        bail!(
            "continuous control {} has invalid authored interval",
            control.id
        );
    }
    let sample_count = sample_count.max(1);
    Ok((0..sample_count)
        .map(|index| {
            let t = if sample_count == 1 {
                0.5
            } else {
                index as f32 / (sample_count - 1) as f32
            };
            ControlValue::Scalar(minimum + (maximum - minimum) * t)
        })
        .collect())
}

fn gallery_option_values(control: &CustomizerControl) -> Vec<(ControlValue, String)> {
    match &control.kind {
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| {
                (
                    ControlValue::Choice(option.value.clone()),
                    option.label.clone(),
                )
            })
            .collect(),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| {
                (
                    ControlValue::Provider(option.provider_id.clone()),
                    option.label.clone(),
                )
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn provider_preview_kind(
    control: &CustomizerControl,
    value: &ControlValue,
    index: usize,
) -> FoundryPreviewKind {
    match (&control.kind, value) {
        (ControlKind::ProviderGallery { role, .. }, ControlValue::Provider(provider_id)) => {
            FoundryPreviewKind::ProviderGallery {
                role: role.clone(),
                provider_id: provider_id.clone(),
                option_index: index as u32,
            }
        }
        _ => FoundryPreviewKind::DiscreteStrip {
            control_id: control.id.clone(),
            value_index: index as u32,
        },
    }
}

fn changed_role_overlays(
    parent: &FoundryAssetDocument,
    candidate: &FoundryAssetDocument,
    profile: &CustomizerProfile,
    changed_controls: &[String],
) -> Vec<FoundryChangedRoleOverlay> {
    let mut overlays = BTreeMap::<String, FoundryChangedRoleOverlay>::new();
    for control_id in changed_controls {
        let Some(control) = profile
            .controls
            .iter()
            .find(|control| control.id == *control_id)
        else {
            continue;
        };
        let ControlKind::ProviderGallery { role, .. } = &control.kind else {
            continue;
        };
        let previous = provider_value_for_control(parent, control);
        let current = provider_value_for_control(candidate, control);
        if previous == current {
            continue;
        }
        overlays.insert(
            role.clone(),
            FoundryChangedRoleOverlay {
                role: role.clone(),
                previous_provider: previous,
                current_provider: current,
                changed_controls: vec![control.id.clone()],
            },
        );
    }
    overlays.into_values().collect()
}

fn changed_role_overlays_for_control_value(
    parent: &FoundryAssetDocument,
    control: &CustomizerControl,
    value: &ControlValue,
) -> Vec<FoundryChangedRoleOverlay> {
    let ControlKind::ProviderGallery { role, .. } = &control.kind else {
        return Vec::new();
    };
    let previous_provider = provider_value_for_control(parent, control);
    let current_provider = match value {
        ControlValue::Provider(provider) => Some(provider.clone()),
        _ => None,
    };
    vec![FoundryChangedRoleOverlay {
        role: role.clone(),
        previous_provider,
        current_provider,
        changed_controls: vec![control.id.clone()],
    }]
}

fn provider_value_for_control(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> Option<String> {
    document
        .control_state
        .get(&control.id)
        .and_then(|value| match value {
            ControlValue::Provider(provider) => Some(provider.clone()),
            _ => None,
        })
}

fn candidate_explanations(diagnostics: &FoundryCandidateDiagnostics) -> Vec<CandidateExplanation> {
    diagnostics
        .changes
        .iter()
        .map(|change| CandidateExplanation {
            control_id: change.control_id.clone(),
            control_label: change.control_label.clone(),
            kind: format!("{:?}", change.kind),
            before: change.before.clone(),
            after: change.after.clone(),
            message: change.message.clone(),
            details: change.details.clone(),
            topology_changing: change.topology_changing,
        })
        .collect()
}

fn invalid_primary_controls(output: &FoundryCompilationOutput) -> Vec<String> {
    let profile = &output.catalog.customizer_profile;
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .filter(|control| current_control_value(&output.document, control, context).is_err())
        .map(|control| control.id.clone())
        .collect()
}

fn preview_control_state(
    state: &BTreeMap<String, ControlValue>,
) -> BTreeMap<String, FoundryPreviewControlValue> {
    state
        .iter()
        .map(|(control_id, value)| (control_id.clone(), preview_control_value(value)))
        .collect()
}

fn preview_control_value(value: &ControlValue) -> FoundryPreviewControlValue {
    match value {
        ControlValue::Scalar(value) => FoundryPreviewControlValue::Scalar(*value),
        ControlValue::Integer(value) => FoundryPreviewControlValue::Integer(*value),
        ControlValue::Toggle(value) => FoundryPreviewControlValue::Toggle(*value),
        ControlValue::Choice(value) => FoundryPreviewControlValue::Choice(value.clone()),
        ControlValue::Provider(value) => FoundryPreviewControlValue::Provider(value.clone()),
    }
}

fn effective_provider_choices(
    output: &FoundryCompilationOutput,
    profile: &CustomizerProfile,
) -> BTreeMap<String, String> {
    let document = &output.document;
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    let mut providers = document
        .provider_overrides
        .iter()
        .map(|(role, override_row)| (role.clone(), override_row.provider_ref.stable_id.clone()))
        .collect::<BTreeMap<_, _>>();
    if let Ok(evaluated) = evaluate_control_state(profile, context, &document.control_state) {
        providers.extend(evaluated.provider_selections);
    } else {
        for control in &profile.controls {
            if let ControlKind::ProviderGallery { role, .. } = &control.kind
                && let Some(provider) = provider_value_for_control(document, control)
            {
                providers.insert(role.clone(), provider);
            }
        }
    }
    providers
}

fn geometry_fingerprint(output: &FoundryCompilationOutput) -> String {
    output.build_stamp.geometry_input_fingerprint.0.to_hex()
}

fn artifact_fingerprint(output: &FoundryCompilationOutput) -> String {
    output.build_stamp.artifact_fingerprint.0.to_hex()
}

fn artifact_mesh_summary(output: &FoundryCompilationOutput) -> MeshSummary {
    MeshSummary {
        vertices: output.artifact.combined_preview.mesh.positions.len(),
        triangles: output.artifact.statistics.triangle_count as usize,
        parts: output.artifact.statistics.part_count,
    }
}

fn image_delta(left: &RenderedImage, right: &RenderedImage) -> u64 {
    if left.width != right.width || left.height != right.height {
        return u64::MAX;
    }
    left.rgba8
        .iter()
        .zip(&right.rgba8)
        .map(|(left, right)| u64::from(left.abs_diff(*right)))
        .sum()
}

fn control_value_order(left: &ControlValue, right: &ControlValue) -> std::cmp::Ordering {
    control_value_rank(left)
        .cmp(&control_value_rank(right))
        .then_with(|| match (left, right) {
            (ControlValue::Scalar(left), ControlValue::Scalar(right)) => left.total_cmp(right),
            (ControlValue::Integer(left), ControlValue::Integer(right)) => left.cmp(right),
            (ControlValue::Toggle(left), ControlValue::Toggle(right)) => left.cmp(right),
            (ControlValue::Choice(left), ControlValue::Choice(right))
            | (ControlValue::Provider(left), ControlValue::Provider(right)) => left.cmp(right),
            _ => std::cmp::Ordering::Equal,
        })
}

fn control_value_rank(value: &ControlValue) -> u8 {
    match value {
        ControlValue::Scalar(_) => 0,
        ControlValue::Integer(_) => 1,
        ControlValue::Toggle(_) => 2,
        ControlValue::Choice(_) => 3,
        ControlValue::Provider(_) => 4,
    }
}

fn control_value_label(control: &CustomizerControl, value: &ControlValue) -> String {
    match (&control.kind, value) {
        (ControlKind::ChoiceGallery { options }, ControlValue::Choice(value)) => options
            .iter()
            .find(|option| option.value == *value)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| value.clone()),
        (ControlKind::ProviderGallery { options, .. }, ControlValue::Provider(value)) => options
            .iter()
            .find(|option| option.provider_id == *value)
            .map(|option| option.label.clone())
            .unwrap_or_else(|| value.clone()),
        (_, ControlValue::Scalar(value)) => format!("{value:.3}"),
        (_, ControlValue::Integer(value)) => value.to_string(),
        (_, ControlValue::Toggle(value)) => value.to_string(),
        (_, ControlValue::Choice(value) | ControlValue::Provider(value)) => value.clone(),
    }
}

fn control_kind_name(control: &CustomizerControl) -> String {
    match control.kind {
        ControlKind::ContinuousAxis { .. } => "continuous_axis",
        ControlKind::IntegerStepper { .. } => "integer_stepper",
        ControlKind::Toggle { .. } => "toggle",
        ControlKind::ChoiceGallery { .. } => "choice_gallery",
        ControlKind::ProviderGallery { .. } => "provider_gallery",
    }
    .to_owned()
}

fn topology_behavior_name(value: ControlTopologyBehavior) -> String {
    match value {
        ControlTopologyBehavior::TopologyPreserving => "topology_preserving",
        ControlTopologyBehavior::TopologyChanging => "topology_changing",
        ControlTopologyBehavior::RuntimeOnly => "runtime_only",
    }
    .to_owned()
}

fn gallery_kind(control: &CustomizerControl) -> String {
    match control.kind {
        ControlKind::ChoiceGallery { .. } => "choice",
        ControlKind::ProviderGallery { .. } => "provider",
        _ => "unknown",
    }
    .to_owned()
}

fn provider_control_role(control: &CustomizerControl) -> Option<String> {
    match &control.kind {
        ControlKind::ProviderGallery { role, .. } => Some(role.clone()),
        _ => None,
    }
}

impl From<&FoundryChangedRoleOverlay> for RoleChangeSummary {
    fn from(value: &FoundryChangedRoleOverlay) -> Self {
        Self {
            role: value.role.clone(),
            previous_provider: value.previous_provider.clone(),
            current_provider: value.current_provider.clone(),
            changed_controls: value.changed_controls.clone(),
        }
    }
}

impl From<&OrbitCamera> for CameraSummary {
    fn from(camera: &OrbitCamera) -> Self {
        let camera = camera.clamped();
        Self {
            target: camera.target.to_array(),
            yaw_degrees: camera.yaw_degrees,
            pitch_degrees: camera.pitch_degrees,
            distance: camera.distance,
            vertical_fov_degrees: camera.vertical_fov_degrees,
        }
    }
}

impl From<FoundryPreviewCacheStats> for PreviewCacheSummary {
    fn from(value: FoundryPreviewCacheStats) -> Self {
        Self {
            len: value.len,
            capacity: value.capacity,
            hits: value.hits,
            misses: value.misses,
            evictions: value.evictions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_benchmark_profiles_cover_canonical_slugs_and_aliases() {
        for slug in [
            "roman-bridge",
            "roman-bridge-hq",
            "sci-fi-crate",
            "stylized-lamp",
            "market-stall",
            "sci-fi-door",
            "storage-barrel",
            "signpost",
            "workshop-chair",
            "handcart",
            "stylized-tree",
        ] {
            let profile = FoundryVisualBenchmarkProfile::from_str(slug, false)
                .unwrap_or_else(|error| panic!("{slug} should parse: {error}"));
            assert_eq!(profile.slug(), slug);
            assert_eq!(profile.fixture().slug, slug);
        }

        let compact_crate = FoundryVisualBenchmarkProfile::from_str("scifi-crate", false)
            .expect("compact sci-fi crate alias should parse");
        assert_eq!(compact_crate, FoundryVisualBenchmarkProfile::ScifiCrate);
        assert_eq!(compact_crate.slug(), "sci-fi-crate");
        assert_eq!(compact_crate.fixture().slug, "sci-fi-crate");
    }
}
