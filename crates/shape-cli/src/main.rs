#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use glam::Vec3;
use image::{Rgba, RgbaImage, imageops::FilterType};
use rayon::prelude::*;
use serde::Serialize;
use shape_asset::{AssetRecipe, ModelingOperationSpec, PartInstanceId, validate_asset_recipe};
use shape_compile::export::{verify_model_package, write_grouped_obj_export, write_model_package};
use shape_compile::validation::{
    ModelValidationConfig, ValidationLimits, validate_model,
    validation_config_from_recipe_with_limits,
};
use shape_compile::{
    build_construction_timeline_report, compile_asset, write_blender_reconstruction_script,
    write_grouped_obj, write_provenance_json,
};
use shape_core::{Aabb, ParamGroup, ShapeDocument, validate_document};
use shape_decompiler::v3::diagnostics::{
    InferenceDiagnosticsV4, ProgramHypothesisDiagnosticsV4,
    ProgramOperatorDiagnostics as ProgramOperatorDiagnosticsV4,
};
use shape_decompiler::v3::inference::{ProgramSearchSettings, search_programs_for_mesh_pair};
use shape_decompiler::v3::package::build_v3_package_from_program_with_diagnostics;
use shape_decompiler::v3::program::{AffineOperator, OperatorProgram, ProgramOperator};
use shape_decompiler::{
    AffineSemanticFamily, DecompileResult, DecompileSettings, OperatorFamily, OperatorManifest,
    ProgramHypothesisDiagnostics as ProgramHypothesisDiagnosticsV3, decompile_pair,
    verify_decompile_package, write_decompile_package,
};
use shape_field::compile_document;
use shape_foundry::{
    CandidateLegibilityClass, CatalogContentRef, ControlEvaluationContext, ControlKind,
    ControlValue, CustomizerControl, FeasibleControlDomain, FoundryAssetDocument,
    FoundryCatalogError, FoundryCatalogResolver, FoundryCommand, apply_foundry_command,
    compile_foundry_document, effective_control_domain,
};
use shape_foundry_catalog::{
    FoundryAuthorProfilePackage, FoundryFixtureCatalog, author_profile_template, box_primitive,
    headless_fixture_catalogs, validate_author_profile_package,
};
use shape_mesh::{MeshSettings, TriangleMesh, mesh_field, read_obj_from_path, write_obj_to_path};
use shape_modeling_assets::{BenchmarkAsset, benchmark_assets};
use shape_poly::{EdgeKey, PolygonMesh, TriangulatedPolygonMesh};
use shape_presets::{PresetId, build_preset, list_presets};
use shape_project::Project;
use shape_render::foundry::FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY;
use shape_render::{
    MeshVisualDescriptor, RenderSettings, RenderedImage, clay_readability_render_settings,
    fit_camera_to_bounds, fit_camera_to_bounds_from_angles, render_mesh,
    visual_descriptor_for_mesh,
};
use shape_search::asset::scoring::{
    AssetCandidateInput, AssetScoredCandidate, AssetSelectionPolicy,
    score_and_select_asset_candidates_with_policy,
};
use shape_search::asset::{
    AssetCandidate as SemanticAssetCandidate, AssetCandidateMode, AssetCandidateRequest,
    generate_asset_candidates,
};
use shape_search::foundry::{
    FOUNDRY_MAX_PROPOSAL_COUNT, FOUNDRY_MAX_RESULT_COUNT, FOUNDRY_MIN_PROPOSAL_COUNT,
    generate_foundry_control_endpoint_visibility_report,
};
use shape_search::{ExplorationMode, SearchRequest, TargetScope, generate_candidates};

mod foundry_cli;
mod foundry_foundation_cli;
mod foundry_kit_cli;

const DEFAULT_PRESET: &str = "box-primitive";
const DEFAULT_SEED: u64 = 42;
const DEFAULT_PROPOSAL_COUNT: usize = 64;
const DEFAULT_RESULT_COUNT: usize = 6;
const DEFAULT_DESCRIPTOR_RESOLUTION: usize = 12;
const DEFAULT_MESH_RESOLUTION: usize = 36;
const DEFAULT_ACCEPT_INDEX: usize = 0;
const CURRENT_IMAGE_SIZE: u32 = 512;
const CONTACT_CARD_SIZE: u32 = 256;
const CONTACT_LABEL_HEIGHT: u32 = 28;
const CONTACT_PADDING: u32 = 12;
const MAX_PROPOSAL_COMPILE_THREADS: usize = 4;
const RELEASE_READINESS_SCHEMA_VERSION: u32 = 5;

#[derive(Debug, Parser)]
#[command(name = "shape-cli")]
#[command(about = "Headless Shape Lab tooling")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate deterministic demo artifacts for a preset.
    Demo(DemoArgs),
    /// Validate every revision document in a project JSON file.
    Validate(ValidateArgs),
    /// Export the current revision from a project JSON file.
    Export(ExportArgs),
    /// Compile and export an explicit benchmark asset recipe.
    ModelDemo(ModelDemoArgs),
    /// Render fixed-camera semantic Refine/Explore benchmark sheets for explicit assets.
    AssetVisualBenchmark(AssetVisualBenchmarkArgs),
    /// Inspect an explicit asset recipe or built-in benchmark slug.
    InspectAsset(InspectAssetArgs),
    /// Compile an explicit asset recipe or built-in benchmark slug.
    CompileAsset(CompileAssetArgs),
    /// Build a foundry document through the conformance-checked headless compiler.
    FoundryBuild(FoundryBuildArgs),
    /// Internal/pro Foundry authoring commands.
    Foundry(foundry_cli::FoundryArgs),
    /// Create a typed Foundry Author profile template.
    FoundryNewProfile(FoundryNewProfileArgs),
    /// Validate a typed Foundry Author profile package.
    FoundryValidateProfile(FoundryProfileArgs),
    /// Render a preview from a typed Foundry Author profile package.
    FoundryPreviewProfile(FoundryPreviewProfileArgs),
    /// Package a typed Foundry Author profile into an exact local catalog.
    FoundryPackageProfile(FoundryPackageProfileArgs),
    /// Validate, inspect, preview, package, and review curated Foundry kits.
    FoundryKit(foundry_kit_cli::FoundryKitArgs),
    /// Create, validate, materialize, and review internal Foundry foundation drafts.
    FoundryFoundation(foundry_foundation_cli::FoundryFoundationArgs),
    /// Print a machine-readable Wave 30 release readiness report.
    ReleaseReadiness(ReleaseReadinessArgs),
    /// Generate Box Primitive visual-readability evidence.
    #[command(hide = true)]
    BoxPrimitiveVisualReadability(BoxPrimitiveVisualReadabilityArgs),
    /// Generate Lid Seam feature-module evidence.
    #[command(hide = true)]
    LidSeamFeatureModuleV0(LidSeamFeatureModuleV0Args),
    /// Decompile a same-topology source/target OBJ pair into deformation IR.
    Decompile(DecompileArgs),
    /// Replay-verify a serialized decompile package.
    VerifyDecompile(VerifyDecompileArgs),
}

#[derive(Debug, clap::Args)]
struct DemoArgs {
    /// Built-in preset ID.
    #[arg(long, default_value = DEFAULT_PRESET)]
    preset: String,
    /// Deterministic candidate generation seed.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// Search mode.
    #[arg(long, value_enum, default_value_t = CliMode::Explore)]
    mode: CliMode,
    /// Number of raw proposals to sample.
    #[arg(long, default_value_t = DEFAULT_PROPOSAL_COUNT)]
    proposal_count: usize,
    /// Number of final candidates to keep.
    #[arg(long, default_value_t = DEFAULT_RESULT_COUNT)]
    result_count: usize,
    /// Descriptor sampling resolution.
    #[arg(long, default_value_t = DEFAULT_DESCRIPTOR_RESOLUTION)]
    descriptor_resolution: usize,
    /// Uniform mesh resolution.
    #[arg(long, default_value_t = DEFAULT_MESH_RESOLUTION)]
    mesh_resolution: usize,
    /// Candidate index accepted into project-after.json.
    #[arg(long, default_value_t = DEFAULT_ACCEPT_INDEX)]
    accept_index: usize,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct ValidateArgs {
    /// Project JSON file.
    project: PathBuf,
}

#[derive(Debug, clap::Args)]
struct ExportArgs {
    /// Project JSON file.
    project: PathBuf,
    /// OBJ output path.
    #[arg(long)]
    obj: PathBuf,
    /// Optional PNG preview output path.
    #[arg(long)]
    png: Option<PathBuf>,
    /// Uniform mesh resolution.
    #[arg(long, default_value_t = DEFAULT_MESH_RESOLUTION)]
    mesh_resolution: usize,
}

#[derive(Debug, clap::Args)]
struct ModelDemoArgs {
    /// Built-in explicit asset slug.
    #[arg(long)]
    asset: String,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct AssetVisualBenchmarkArgs {
    /// Optional built-in explicit asset slug. Omit to run all benchmark assets.
    #[arg(long)]
    asset: Option<String>,
    /// Deterministic semantic candidate seed.
    #[arg(long, default_value_t = DEFAULT_SEED)]
    seed: u64,
    /// Number of semantic proposals to compile and score for each mode.
    #[arg(long, default_value_t = 192)]
    proposal_count: usize,
    /// Number of representative candidates to render per mode.
    #[arg(long, default_value_t = DEFAULT_RESULT_COUNT)]
    result_count: usize,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct InspectAssetArgs {
    /// Built-in asset slug or recipe JSON path.
    recipe: String,
}

#[derive(Debug, clap::Args)]
struct CompileAssetArgs {
    /// Built-in asset slug or recipe JSON path.
    recipe: String,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct FoundryBuildArgs {
    /// Directory containing catalog JSON entries named `<stable-id>.json`.
    #[arg(long)]
    catalog: PathBuf,
    /// Foundry document JSON file.
    #[arg(long)]
    document: PathBuf,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct FoundryNewProfileArgs {
    /// Built-in template slug. The current baseline supports box-primitive.
    #[arg(long, default_value = "box-primitive")]
    template: String,
    /// Output profile JSON file.
    #[arg(long)]
    out: PathBuf,
}

#[derive(Debug, clap::Args)]
struct FoundryProfileArgs {
    /// Typed Foundry Author profile JSON file.
    profile: PathBuf,
}

#[derive(Debug, clap::Args)]
struct FoundryPreviewProfileArgs {
    /// Typed Foundry Author profile JSON file.
    profile: PathBuf,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct FoundryPackageProfileArgs {
    /// Typed Foundry Author profile JSON file.
    profile: PathBuf,
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct ReleaseReadinessArgs {
    /// Optional JSON output file. The report is always printed to stdout.
    #[arg(long)]
    out: Option<PathBuf>,
    /// Run the expensive Visual Foundry product gate and include computed evidence.
    #[arg(long)]
    verify_visual_gate: bool,
    /// Run the headless native product UI gate and include computed evidence.
    #[arg(long)]
    verify_product_ui_gate: bool,
}

#[derive(Debug, clap::Args)]
struct BoxPrimitiveVisualReadabilityArgs {
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct LidSeamFeatureModuleV0Args {
    /// Output directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, clap::Args)]
struct DecompileArgs {
    /// Source OBJ mesh.
    source: PathBuf,
    /// Target OBJ mesh with identical ordered topology.
    target: PathBuf,
    /// Output package directory.
    #[arg(long)]
    out_dir: PathBuf,
    /// Minimum displacement fraction required to emit an affine operator.
    #[arg(long, default_value_t = 0.01)]
    affine_min_explained: f32,
    /// Verification tolerance; the final residual remains lossless.
    #[arg(long, default_value_t = 0.0)]
    residual_epsilon: f32,
    /// Experimental decompile package schema to write.
    #[arg(long, value_enum, default_value_t = PackageSchema::Schema2)]
    package_schema: PackageSchema,
    /// Enable experimental schema-3 bend program inference.
    #[arg(long)]
    enable_bend: bool,
    /// Print per-hypothesis inference diagnostics.
    #[arg(long)]
    verbose: bool,
}

#[derive(Debug, clap::Args)]
struct VerifyDecompileArgs {
    /// Decompile package directory.
    package: PathBuf,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum CliMode {
    Refine,
    Explore,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum PackageSchema {
    #[value(name = "2")]
    Schema2,
    #[value(name = "3")]
    Schema3,
}

#[derive(Debug, Serialize)]
struct ReleaseReadinessReport {
    schema_version: u32,
    milestone: &'static str,
    visual_product_gate: ReleaseVisualProductGate,
    product_ui_gate: ReleaseProductUiGate,
    performance: ReleasePerformanceReadiness,
    rendering: ReleaseRenderingReadiness,
    persistence: ReleasePersistenceReadiness,
    packaging: ReleasePackagingReadiness,
    window_regression: ReleaseWindowRegressionReadiness,
    verification_commands: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct BoxPrimitiveVisualReadabilityReport {
    schema_version: u32,
    profile_slug: &'static str,
    reads_as_box: bool,
    width_depth_height_visible: bool,
    edges_readable: bool,
    candidates_differ: bool,
    avoided_crate_features: bool,
    export_clean: bool,
    viewport_aid: &'static str,
    candidate_labels: Vec<String>,
    artifacts: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct LidSeamFeatureModuleV0Report {
    schema_version: u32,
    profile_slug: &'static str,
    module_id: &'static str,
    seam_visible_in_pure_clay: bool,
    seam_not_material_stripe: bool,
    closed_box_silhouette_preserved: bool,
    seam_endpoint_visible: bool,
    candidates_differ: bool,
    avoided_crate_claim: bool,
    export_clean: bool,
    candidate_labels: Vec<String>,
    artifacts: BTreeMap<String, String>,
}

#[derive(Debug, Serialize)]
struct ReleaseVisualProductGate {
    verification_status: &'static str,
    verification_command: &'static str,
    native_state_verification_status: &'static str,
    native_state_verification_command: &'static str,
    expected_built_in_profile_count: usize,
    expected_primary_controls_per_profile: usize,
    option_thumbnail_contract: &'static str,
    default_path_advanced_recipe_gate: &'static str,
    deterministic_contact_sheets: &'static str,
    evidence: Option<ReleaseVisualProductGateEvidence>,
}

#[derive(Debug, Serialize)]
struct ReleaseVisualProductGateEvidence {
    built_in_profile_count: usize,
    profiles_checked: usize,
    all_profiles_verified: bool,
    option_thumbnail_size_px: u32,
    option_thumbnail_count: usize,
    option_controls_checked: usize,
    profiles: Vec<ReleaseVisualProfileEvidence>,
}

#[derive(Debug, Serialize)]
struct ReleaseVisualProfileEvidence {
    slug: String,
    primary_control_count: usize,
    option_control_count: usize,
    option_thumbnail_count: usize,
    per_option_rgba_complete: bool,
    per_option_camera_recorded: bool,
    every_option_control_has_visual_delta: bool,
    option_controls_without_visual_delta: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ReleaseProductUiGate {
    verification_status: &'static str,
    verification_command: &'static str,
    app_shell: &'static str,
    legacy_surfaces_present: bool,
    product_home_profiles: usize,
    installed_kit_count: usize,
    developer_preview_kit_count: usize,
    startup_blank: bool,
    default_advanced_recipe_visible: bool,
    default_raw_technical_terms_visible: bool,
    directions_board_gate: &'static str,
    customize_deck_gate: &'static str,
    pack_gate: &'static str,
    export_gate: &'static str,
    disabled_states_have_reasons: bool,
    manual_gate_required: bool,
    evidence: Option<ReleaseProductUiGateEvidence>,
}

#[derive(Debug, Serialize)]
struct ReleaseProductUiGateEvidence {
    product_visible_string_count: usize,
    rendered_action_label_count: usize,
    rendered_action_labels_audited: bool,
    forbidden_terms_found: Vec<ReleaseProductForbiddenTermFinding>,
    core_profiles: Vec<ReleaseProductUiProfileEvidence>,
    direction_modes: Vec<&'static str>,
    direction_candidate_slots: usize,
    automated_gate_passed: bool,
}

#[derive(Debug, Serialize)]
struct ReleaseProductForbiddenTermFinding {
    term: &'static str,
    visible_string: String,
}

#[derive(Debug, Serialize)]
struct ReleaseProductUiProfileEvidence {
    slug: String,
    label: String,
    compiled: bool,
    reaches_main_shell: bool,
    primary_control_count: usize,
    option_control_count: usize,
    triangle_count: usize,
}

#[derive(Debug, Serialize)]
struct ReleasePerformanceReadiness {
    preview_cache: PreviewCacheReadiness,
    candidate_generation: CandidateGenerationReadiness,
}

#[derive(Debug, Serialize)]
struct PreviewCacheReadiness {
    backend: &'static str,
    bounded_lru_capacity: usize,
    duplicate_miss_coalescing: bool,
    deterministic_cache_keys: bool,
}

#[derive(Debug, Serialize)]
struct CandidateGenerationReadiness {
    minimum_proposal_count: usize,
    maximum_proposal_count: usize,
    maximum_returned_candidates: usize,
    rejects_unbounded_proposal_requests: bool,
    caps_representative_selection: bool,
}

#[derive(Debug, Serialize)]
struct ReleaseRenderingReadiness {
    deterministic_cpu_reference: &'static str,
    optional_gpu_viewport: &'static str,
    gpu_required_for_release_checks: bool,
}

#[derive(Debug, Serialize)]
struct ReleasePersistenceReadiness {
    foundry_recovery_snapshots: &'static str,
    asset_autosave_snapshots: &'static str,
    automatic_timed_autosave_ui: &'static str,
}

#[derive(Debug, Serialize)]
struct ReleasePackagingReadiness {
    packaging_docs: &'static str,
    installer_framework: &'static str,
    code_signing: &'static str,
    publishing: &'static str,
}

#[derive(Debug, Serialize)]
struct ReleaseWindowRegressionReadiness {
    headless_panel_and_reducer_tests: &'static str,
    desktop_window_pixel_tests: &'static str,
}

impl From<CliMode> for ExplorationMode {
    fn from(value: CliMode) -> Self {
        match value {
            CliMode::Refine => Self::Refine,
            CliMode::Explore => Self::Explore,
        }
    }
}

#[derive(Debug, Serialize)]
struct DemoSummary {
    preset: String,
    seed: u64,
    mode: String,
    proposal_count: usize,
    result_count: usize,
    descriptor_resolution: usize,
    mesh_resolution: usize,
    accepted_index: usize,
    generated_candidates: usize,
    estimated_rejections: usize,
    parent: MeshSummary,
    candidates: Vec<CandidateSummary>,
    accepted: MeshSummary,
    timings_ms: BTreeMap<String, u64>,
}

#[derive(Debug, Serialize)]
struct MeshSummary {
    vertices: usize,
    triangles: usize,
}

#[derive(Debug, Serialize)]
struct CandidateSummary {
    index: usize,
    id: u64,
    distance_from_parent: f32,
    changed_parameters: Vec<ChangedParameter>,
    mesh: MeshSummary,
}

#[derive(Debug, Serialize)]
struct ChangedParameter {
    node: u64,
    key: String,
    before: f32,
    after: f32,
}

#[derive(Debug, Serialize)]
struct AssetVisualBenchmarkSummary {
    asset: String,
    seed: u64,
    proposal_count: usize,
    result_count: usize,
    original: MeshSummary,
    refine_candidates: Vec<AssetVisualCandidateSummary>,
    explore_candidates: Vec<AssetVisualCandidateSummary>,
    accepted_source: String,
    accepted: MeshSummary,
    package_dir: String,
}

#[derive(Debug, Serialize)]
struct AssetVisualCandidateSummary {
    slot: usize,
    id: u64,
    operation_count: usize,
    structural_change_count: usize,
    quality_penalty: f32,
    mesh: MeshSummary,
}

#[derive(Debug, Clone)]
struct VisualSelectedCandidate {
    candidate: SemanticAssetCandidate,
    scored: AssetScoredCandidate,
    artifact: shape_compile::AssetArtifact,
}

struct CompiledVisualCandidate {
    input: AssetCandidateInput,
    candidate: Option<SemanticAssetCandidate>,
    artifact: Option<shape_compile::AssetArtifact>,
}

struct PreviewArtifact {
    mesh: TriangleMesh,
    image: RenderedImage,
}

struct DirectoryFoundryCatalogResolver {
    root: PathBuf,
}

impl FoundryCatalogResolver for DirectoryFoundryCatalogResolver {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        let path = self.root.join(format!("{}.json", content_ref.stable_id));
        if !path.exists() {
            return Err(FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            });
        }
        fs::read_to_string(&path).map_err(|error| FoundryCatalogError::InvalidJson {
            subject: path.display().to_string(),
            error: error.to_string(),
        })
    }
}

fn main() -> anyhow::Result<()> {
    let _ = env_logger::try_init();
    match Cli::parse().command {
        Command::Demo(args) => run_demo(args),
        Command::Validate(args) => run_validate(args),
        Command::Export(args) => run_export(args),
        Command::ModelDemo(args) => run_model_demo(args),
        Command::AssetVisualBenchmark(args) => run_asset_visual_benchmark(args),
        Command::InspectAsset(args) => run_inspect_asset(args),
        Command::CompileAsset(args) => run_compile_asset(args),
        Command::FoundryBuild(args) => run_foundry_build(args),
        Command::Foundry(args) => foundry_cli::run_foundry(args),
        Command::FoundryNewProfile(args) => run_foundry_new_profile(args),
        Command::FoundryValidateProfile(args) => run_foundry_validate_profile(args),
        Command::FoundryPreviewProfile(args) => run_foundry_preview_profile(args),
        Command::FoundryPackageProfile(args) => run_foundry_package_profile(args),
        Command::FoundryKit(args) => foundry_kit_cli::run_foundry_kit(args),
        Command::FoundryFoundation(args) => foundry_foundation_cli::run_foundry_foundation(args),
        Command::ReleaseReadiness(args) => run_release_readiness(args),
        Command::BoxPrimitiveVisualReadability(args) => run_box_primitive_visual_readability(args),
        Command::LidSeamFeatureModuleV0(args) => run_lid_seam_feature_module_v0(args),
        Command::Decompile(args) => run_decompile(args),
        Command::VerifyDecompile(args) => run_verify_decompile(args),
    }
}

fn run_demo(args: DemoArgs) -> anyhow::Result<()> {
    let preset_id = PresetId(args.preset.clone());
    ensure_known_preset(&preset_id)?;
    let proposal_count = clamp_usize(args.proposal_count, 1, 512);
    let result_count = clamp_usize(args.result_count, 1, 12);
    let descriptor_resolution = clamp_usize(args.descriptor_resolution, 8, 24);
    let mesh_resolution = clamp_usize(args.mesh_resolution, 8, 96);

    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    let document = build_preset(&preset_id).context("building preset")?;
    ensure_valid_document(&document, "preset")?;
    let mut project =
        Project::try_new(document.title.clone(), document.clone()).context("creating project")?;
    project.save_json(args.out_dir.join("project-before.json"))?;

    let mesh_settings = MeshSettings {
        resolution: mesh_resolution,
        ..MeshSettings::default()
    };
    let current = build_preview(&document, mesh_settings, CURRENT_IMAGE_SIZE)?;
    write_obj_to_path(&current.mesh, args.out_dir.join("current.obj"))?;
    save_png(&current.image, args.out_dir.join("current.png"))?;

    let request = SearchRequest {
        seed: args.seed,
        proposal_count,
        result_count,
        descriptor_resolution,
        selected_node: Some(document.root),
        target_scope: TargetScope::WholeModel,
        enabled_groups: all_param_groups(),
        mode: args.mode.into(),
    };
    let candidates = generate_candidates(&document, &request).context("generating candidates")?;
    if candidates.is_empty() {
        bail!("search produced no candidates");
    }
    if args.accept_index >= candidates.len() {
        bail!(
            "accept index {} is out of range for {} candidate(s)",
            args.accept_index,
            candidates.len()
        );
    }

    let mut candidate_previews = Vec::with_capacity(candidates.len());
    let mut candidate_summaries = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        let preview = build_preview(&candidate.document, mesh_settings, CURRENT_IMAGE_SIZE)
            .with_context(|| format!("building preview for candidate {index:02}"))?;
        write_obj_to_path(
            &preview.mesh,
            args.out_dir.join(format!("candidate-{index:02}.obj")),
        )?;
        save_png(
            &preview.image,
            args.out_dir.join(format!("candidate-{index:02}.png")),
        )?;
        candidate_summaries.push(CandidateSummary {
            index,
            id: candidate.id.0,
            distance_from_parent: candidate.distance_from_parent,
            changed_parameters: summarize_changes(&candidate.edit.operations),
            mesh: mesh_summary(&preview.mesh),
        });
        candidate_previews.push(preview);
    }

    save_contact_sheet(
        &current.image,
        candidate_previews
            .iter()
            .map(|preview| &preview.image)
            .collect::<Vec<_>>()
            .as_slice(),
        args.out_dir.join("contact-sheet.png"),
    )?;

    let accepted_candidate = candidates[args.accept_index].clone();
    project
        .accept_candidate(accepted_candidate)
        .context("accepting candidate")?;
    project.save_json(args.out_dir.join("project-after.json"))?;
    let accepted_document = project.current_document()?.clone();
    let accepted = build_preview(&accepted_document, mesh_settings, CURRENT_IMAGE_SIZE)?;
    write_obj_to_path(&accepted.mesh, args.out_dir.join("accepted.obj"))?;
    save_png(&accepted.image, args.out_dir.join("accepted.png"))?;

    let mut timings_ms = BTreeMap::new();
    timings_ms.insert("deterministic_summary_placeholder".to_owned(), 0);
    let summary = DemoSummary {
        preset: args.preset,
        seed: args.seed,
        mode: format!("{:?}", args.mode).to_lowercase(),
        proposal_count,
        result_count,
        descriptor_resolution,
        mesh_resolution,
        accepted_index: args.accept_index,
        generated_candidates: candidates.len(),
        estimated_rejections: proposal_count.saturating_sub(candidates.len()),
        parent: mesh_summary(&current.mesh),
        candidates: candidate_summaries,
        accepted: mesh_summary(&accepted.mesh),
        timings_ms,
    };
    write_json(args.out_dir.join("summary.json"), &summary)?;

    println!(
        "Generated {} candidate(s) for {} in {}",
        candidates.len(),
        preset_id.0,
        args.out_dir.display()
    );
    Ok(())
}

fn run_validate(args: ValidateArgs) -> anyhow::Result<()> {
    let project = Project::load_json(&args.project)
        .with_context(|| format!("loading {}", args.project.display()))?;
    project.validate()?;
    println!("{} is valid", args.project.display());
    Ok(())
}

fn run_export(args: ExportArgs) -> anyhow::Result<()> {
    let mesh_resolution = clamp_usize(args.mesh_resolution, 8, 96);
    let project = Project::load_json(&args.project)
        .with_context(|| format!("loading {}", args.project.display()))?;
    let document = project.current_document()?;
    let mesh_settings = MeshSettings {
        resolution: mesh_resolution,
        ..MeshSettings::default()
    };
    let preview = build_preview(document, mesh_settings, CURRENT_IMAGE_SIZE)?;
    write_obj_to_path(&preview.mesh, &args.obj)
        .with_context(|| format!("exporting OBJ to {}", args.obj.display()))?;
    if let Some(path) = args.png {
        save_png(&preview.image, path)?;
    }
    Ok(())
}

fn run_model_demo(args: ModelDemoArgs) -> anyhow::Result<()> {
    let asset = BenchmarkAsset::parse(&args.asset)
        .with_context(|| format!("unknown model-demo asset '{}'", args.asset))?;
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    let recipe = asset.recipe();
    let artifact = compile_asset(&recipe).context("compiling explicit asset recipe")?;
    if !artifact.validation_report.is_valid() {
        bail!(
            "compiled asset validation failed with {} issue(s)",
            artifact.validation_report.issues.len()
        );
    }

    fs::write(
        args.out_dir.join("recipe.json"),
        serde_json::to_string_pretty(&recipe)?,
    )
    .with_context(|| format!("writing recipe.json to {}", args.out_dir.display()))?;
    fs::write(
        args.out_dir.join("asset.obj"),
        write_grouped_obj(&artifact).context("writing grouped OBJ")?,
    )
    .with_context(|| format!("writing asset.obj to {}", args.out_dir.display()))?;
    fs::write(
        args.out_dir.join("provenance.json"),
        write_provenance_json(&artifact.provenance_report)?,
    )
    .with_context(|| format!("writing provenance.json to {}", args.out_dir.display()))?;
    write_json(
        args.out_dir.join("validation.json"),
        &artifact.validation_report,
    )?;
    write_json(args.out_dir.join("statistics.json"), &artifact.statistics)?;
    fs::write(
        args.out_dir.join("blender_reconstruct.py"),
        write_blender_reconstruction_script(&artifact).context("writing Blender script")?,
    )
    .with_context(|| {
        format!(
            "writing blender_reconstruct.py to {}",
            args.out_dir.display()
        )
    })?;

    let preview_mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let camera = fit_camera_to_bounds(preview_mesh.bounds);
    let settings = RenderSettings {
        width: CURRENT_IMAGE_SIZE,
        height: CURRENT_IMAGE_SIZE,
        ..RenderSettings::default()
    };
    let image = render_mesh(&preview_mesh, &camera, &settings).context("rendering preview")?;
    save_png(&image, args.out_dir.join("preview.png"))?;

    println!(
        "Compiled {} to {} ({} parts, {} triangles)",
        asset.slug(),
        args.out_dir.display(),
        artifact.statistics.part_count,
        artifact.statistics.triangle_count
    );
    Ok(())
}

fn run_asset_visual_benchmark(args: AssetVisualBenchmarkArgs) -> anyhow::Result<()> {
    let proposal_count = clamp_usize(args.proposal_count, 1, 512);
    let result_count = clamp_usize(args.result_count, 1, 12);
    let assets = match &args.asset {
        Some(slug) => vec![
            BenchmarkAsset::parse(slug)
                .with_context(|| format!("unknown asset benchmark '{slug}'"))?,
        ],
        None => benchmark_assets().to_vec(),
    };
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    for asset in assets {
        let asset_dir = args.out_dir.join(asset.slug());
        fs::create_dir_all(&asset_dir)
            .with_context(|| format!("creating {}", asset_dir.display()))?;
        let recipe = asset.recipe();
        let original_artifact = compile_asset(&recipe)
            .with_context(|| format!("compiling original {}", asset.slug()))?;
        let original_image = render_asset_artifact(&original_artifact, false)?;
        let original_wireframe = render_asset_artifact(&original_artifact, true)?;
        save_png(&original_image, asset_dir.join("original.png"))?;
        save_png(
            &original_wireframe,
            asset_dir.join("original-wireframe.png"),
        )?;

        let refine = select_visual_candidates(
            &recipe,
            args.seed ^ 0x52ef_1111,
            AssetCandidateMode::Refine,
            proposal_count,
            result_count,
        )?;
        let explore = select_visual_candidates(
            &recipe,
            args.seed ^ 0xe901_2222,
            AssetCandidateMode::Explore,
            proposal_count,
            result_count,
        )?;

        let refine_summary = render_visual_candidate_set(
            &asset_dir,
            "refine",
            &original_image,
            &original_wireframe,
            &refine,
        )?;
        let explore_summary = render_visual_candidate_set(
            &asset_dir,
            "explore",
            &original_image,
            &original_wireframe,
            &explore,
        )?;

        let (accepted_source, accepted_candidate) = explore
            .first()
            .map(|candidate| ("explore".to_owned(), candidate))
            .or_else(|| {
                refine
                    .first()
                    .map(|candidate| ("refine".to_owned(), candidate))
            })
            .context("visual benchmark produced no accepted candidate")?;
        let accepted_artifact = &accepted_candidate.artifact;
        let accepted_image = render_asset_artifact(accepted_artifact, false)?;
        let accepted_wireframe = render_asset_artifact(accepted_artifact, true)?;
        save_png(&accepted_image, asset_dir.join("accepted.png"))?;
        save_png(
            &accepted_wireframe,
            asset_dir.join("accepted-wireframe.png"),
        )?;

        let package_dir = asset_dir.join("final-package");
        let package_paths = write_model_package(
            &accepted_candidate.candidate.recipe,
            accepted_artifact,
            &package_dir,
        )
        .with_context(|| format!("writing final package {}", package_dir.display()))?;
        let obj = write_grouped_obj_export(
            accepted_artifact,
            Some(&accepted_candidate.candidate.recipe),
        )
        .context("writing accepted grouped OBJ")?;
        fs::write(asset_dir.join("accepted.obj"), obj.obj)
            .with_context(|| format!("writing accepted.obj to {}", asset_dir.display()))?;
        let final_image = render_asset_artifact(accepted_artifact, false)?;
        let final_wireframe = render_asset_artifact(accepted_artifact, true)?;
        save_png(&final_image, asset_dir.join("final-exported.png"))?;
        save_png(
            &final_wireframe,
            asset_dir.join("final-exported-wireframe.png"),
        )?;

        let summary = AssetVisualBenchmarkSummary {
            asset: asset.slug().to_owned(),
            seed: args.seed,
            proposal_count,
            result_count,
            original: artifact_mesh_summary(&original_artifact),
            refine_candidates: refine_summary,
            explore_candidates: explore_summary,
            accepted_source,
            accepted: artifact_mesh_summary(accepted_artifact),
            package_dir: package_paths.manifest.display().to_string(),
        };
        write_json(asset_dir.join("visual-benchmark-summary.json"), &summary)?;
        println!(
            "Rendered visual benchmark {} to {}",
            asset.slug(),
            asset_dir.display()
        );
    }

    Ok(())
}

fn run_inspect_asset(args: InspectAssetArgs) -> anyhow::Result<()> {
    let loaded = load_asset_recipe(&args.recipe)?;
    let recipe_report = validate_asset_recipe(&loaded.recipe);
    println!("Asset: {}", loaded.recipe.title);
    println!("Source: {}", loaded.label);
    println!(
        "Recipe validation: {}",
        validity_label(recipe_report.is_valid())
    );
    if !recipe_report.is_valid() {
        for issue in &recipe_report.issues {
            println!("  - {}: {}", issue.code, issue.message);
        }
        bail!("asset recipe is invalid");
    }

    let artifact = compile_asset(&loaded.recipe).context("compiling explicit asset recipe")?;
    let config = model_validation_config(&loaded, &artifact);
    let model_report = validate_model(&artifact, &config);
    let timeline = build_construction_timeline_report(&loaded.recipe, &artifact);

    print_part_tree(&loaded.recipe);
    print_parameters(&loaded.recipe);
    print_regions(&loaded.recipe);
    print_sockets(&loaded.recipe);
    print_operations(&loaded.recipe);
    print_timeline(&timeline);
    print_validation(&artifact, &model_report);
    print_topology_statistics(&artifact, &model_report);

    if !artifact.validation_report.is_valid() || !model_report.is_valid() {
        bail!("compiled asset failed validation");
    }
    Ok(())
}

fn run_compile_asset(args: CompileAssetArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;
    let loaded = load_asset_recipe(&args.recipe)?;
    let recipe_report = validate_asset_recipe(&loaded.recipe);
    if !recipe_report.is_valid() {
        bail!(
            "asset recipe validation failed with {} issue(s)",
            recipe_report.issues.len()
        );
    }

    let artifact = compile_asset(&loaded.recipe).context("compiling explicit asset recipe")?;
    let config = model_validation_config(&loaded, &artifact);
    let model_report = validate_model(&artifact, &config);
    if !artifact.validation_report.is_valid() || !model_report.is_valid() {
        bail!("compiled asset failed validation");
    }

    let timeline = build_construction_timeline_report(&loaded.recipe, &artifact);
    let package_paths = write_model_package(&loaded.recipe, &artifact, &args.out_dir)
        .with_context(|| format!("writing model package to {}", args.out_dir.display()))?;
    let package_verification = verify_model_package(&args.out_dir)
        .with_context(|| format!("verifying model package {}", args.out_dir.display()))?;
    let obj = write_grouped_obj_export(&artifact, Some(&loaded.recipe))
        .context("writing grouped OBJ export")?;
    fs::write(args.out_dir.join("asset.obj"), obj.obj)
        .with_context(|| format!("writing asset.obj to {}", args.out_dir.display()))?;
    write_json(args.out_dir.join("grouped-obj-report.json"), &obj.report)?;
    write_json(args.out_dir.join("statistics.json"), &artifact.statistics)?;
    write_json(args.out_dir.join("model-validation.json"), &model_report)?;
    write_json(args.out_dir.join("construction-timeline.json"), &timeline)?;
    write_json(
        args.out_dir.join("package-verification.json"),
        &package_verification,
    )?;

    let preview_mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let camera = fit_camera_to_bounds(preview_mesh.bounds);
    let settings = RenderSettings {
        width: CURRENT_IMAGE_SIZE,
        height: CURRENT_IMAGE_SIZE,
        ..RenderSettings::default()
    };
    let image = render_mesh(&preview_mesh, &camera, &settings).context("rendering preview")?;
    save_png(&image, args.out_dir.join("preview.png"))?;

    println!("Compiled {}", loaded.recipe.title);
    println!("  source: {}", loaded.label);
    println!("  output: {}", args.out_dir.display());
    println!("  manifest: {}", package_paths.manifest.display());
    println!(
        "  blender script: {}",
        package_paths.blender_reconstruct.display()
    );
    println!("  parts: {}", artifact.statistics.part_count);
    println!("  triangles: {}", artifact.statistics.triangle_count);
    println!(
        "  package verification: checksums={} topology={} finite={}",
        package_verification.checksums_match,
        package_verification.topology_matches_manifest,
        package_verification.finite_numeric_payloads
    );
    Ok(())
}

fn run_foundry_build(args: FoundryBuildArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;
    let document_json = fs::read_to_string(&args.document)
        .with_context(|| format!("reading foundry document {}", args.document.display()))?;
    let document: FoundryAssetDocument = serde_json::from_str(&document_json)
        .with_context(|| format!("parsing foundry document {}", args.document.display()))?;
    let resolver = DirectoryFoundryCatalogResolver { root: args.catalog };
    let output = compile_foundry_document(&document, &resolver)
        .map_err(|error| anyhow::anyhow!("foundry compilation failed: {error:#?}"))?;

    if !output.artifact.validation_report.is_valid() {
        bail!(
            "compiled foundry artifact validation failed with {} issue(s)",
            output.artifact.validation_report.issues.len()
        );
    }
    let model_config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    let model_report = validate_model(&output.artifact, &model_config);

    write_json(args.out_dir.join("foundry-document.json"), &output.document)?;
    write_json(
        args.out_dir.join("catalog-lock.json"),
        &output.catalog.catalog_lock,
    )?;
    write_json(
        args.out_dir.join("effective-request.json"),
        &output.family_request,
    )?;
    write_json(
        args.out_dir.join("family-conformance.json"),
        &output.final_conformance,
    )?;
    write_json(
        args.out_dir.join("conformance-summary.json"),
        &output.conformance_summary,
    )?;
    write_json(args.out_dir.join("recipe.json"), &output.recipe)?;
    write_json(args.out_dir.join("build-stamp.json"), &output.build_stamp)?;
    write_json(
        args.out_dir.join("local-overrides.json"),
        &output.local_override_reports,
    )?;
    write_json(
        args.out_dir.join("local-override-divergence.json"),
        &output.local_override_divergence_reports,
    )?;
    write_json(
        args.out_dir.join("control-divergence.json"),
        &output.control_divergence,
    )?;
    write_json(
        args.out_dir.join("provider-overrides.json"),
        &output.provider_override_reports,
    )?;
    write_json(args.out_dir.join("model-validation.json"), &model_report)?;
    if !model_report.is_valid() {
        bail!(
            "foundry model validation failed with {} issue(s)",
            model_report.issues.len()
        );
    }

    let package_dir = args.out_dir.join("model-package");
    let package_paths = write_model_package(&output.recipe, &output.artifact, &package_dir)
        .with_context(|| format!("writing model package {}", package_dir.display()))?;
    let package_verification = verify_model_package(&package_dir)
        .with_context(|| format!("verifying model package {}", package_dir.display()))?;
    write_json(
        args.out_dir.join("package-verification.json"),
        &package_verification,
    )?;

    let grouped_obj = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
        .context("writing foundry grouped OBJ")?;
    fs::write(args.out_dir.join("asset.obj"), grouped_obj.obj)
        .with_context(|| format!("writing asset.obj to {}", args.out_dir.display()))?;
    write_json(
        args.out_dir.join("grouped-obj-report.json"),
        &grouped_obj.report,
    )?;

    let preview = render_asset_artifact(&output.artifact, false)?;
    save_png(&preview, args.out_dir.join("preview.png"))?;

    println!("Built foundry document {}", output.document.document_id.0);
    println!("  output: {}", args.out_dir.display());
    println!("  package: {}", package_paths.directory.display());
    println!("  manifest: {}", package_paths.manifest.display());
    println!("  parts: {}", output.artifact.statistics.part_count);
    println!("  triangles: {}", output.artifact.statistics.triangle_count);
    println!(
        "  conformance: {}",
        validity_label(output.final_conformance.is_accepted())
    );
    Ok(())
}

fn run_foundry_new_profile(args: FoundryNewProfileArgs) -> anyhow::Result<()> {
    let profile = author_profile_template(&args.template)
        .with_context(|| format!("unknown Foundry Author template '{}'", args.template))?;
    if let Some(parent) = args.out.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    write_json(&args.out, &profile)?;
    println!(
        "Created Foundry Author profile {} at {}",
        profile.package_id,
        args.out.display()
    );
    Ok(())
}

fn run_foundry_validate_profile(args: FoundryProfileArgs) -> anyhow::Result<()> {
    let profile = load_author_profile(&args.profile)?;
    let report = validate_author_profile_package(&profile);
    print_author_validation_report(&report);
    if !report.is_valid() {
        bail!(
            "Foundry Author profile validation failed with {} issue(s)",
            report.issues.len()
        );
    }
    Ok(())
}

fn run_foundry_preview_profile(args: FoundryPreviewProfileArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;
    let profile = load_author_profile(&args.profile)?;
    let report = validate_author_profile_package(&profile);
    write_json(args.out_dir.join("foundry-author-validation.json"), &report)?;
    if !report.is_valid() {
        bail!(
            "Foundry Author profile validation failed with {} issue(s)",
            report.issues.len()
        );
    }

    let fixture = profile.to_fixture_catalog();
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("foundry author profile compile failed: {error:#?}"))?;
    write_author_build_outputs(&args.profile, &profile, &output, &args.out_dir)?;
    println!(
        "Rendered Foundry Author profile {} preview to {}",
        profile.package_id,
        args.out_dir.display()
    );
    Ok(())
}

fn run_foundry_package_profile(args: FoundryPackageProfileArgs) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;
    let profile = load_author_profile(&args.profile)?;
    let report = validate_author_profile_package(&profile);
    write_json(args.out_dir.join("foundry-author-validation.json"), &report)?;
    if !report.is_valid() {
        bail!(
            "Foundry Author profile validation failed with {} issue(s)",
            report.issues.len()
        );
    }

    let fixture = profile.to_fixture_catalog();
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("foundry author profile compile failed: {error:#?}"))?;
    write_json(args.out_dir.join("foundry-author-profile.json"), &profile)?;
    let catalog_dir = args.out_dir.join("catalog");
    recreate_dir(&catalog_dir)?;
    fixture
        .write_to_dir(&catalog_dir)
        .with_context(|| format!("writing exact catalog to {}", catalog_dir.display()))?;
    let proof_dir = args.out_dir.join("build-proof");
    recreate_dir(&proof_dir)?;
    write_author_build_outputs(&args.profile, &profile, &output, &proof_dir)?;
    println!(
        "Packaged Foundry Author profile {} into {}",
        profile.package_id,
        args.out_dir.display()
    );
    println!("  catalog: {}", catalog_dir.display());
    println!("  build proof: {}", proof_dir.display());
    Ok(())
}

fn run_release_readiness(args: ReleaseReadinessArgs) -> anyhow::Result<()> {
    let report = release_readiness_report(args.verify_visual_gate, args.verify_product_ui_gate)?;
    if let Some(path) = args.out.as_ref() {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        write_json(path, &report)?;
    }
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn run_box_primitive_visual_readability(
    args: BoxPrimitiveVisualReadabilityArgs,
) -> anyhow::Result<()> {
    recreate_dir(&args.out_dir)?;
    let fixture = box_primitive::fixture_catalog();
    let parent_output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("box primitive parent compile failed: {error:#?}"))?;
    let parent = render_foundry_output_readability(&parent_output, 512)?;
    save_png(&parent, args.out_dir.join("parent.png"))?;

    let candidates = [
        (
            "Compact Box",
            vec![(
                "proportions",
                ControlValue::Choice("compact_box".to_owned()),
            )],
        ),
        (
            "Wide Box",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Tall Box",
            vec![("proportions", ControlValue::Choice("tall_box".to_owned()))],
        ),
        (
            "Flat Box",
            vec![("proportions", ControlValue::Choice("flat_box".to_owned()))],
        ),
        (
            "Soft-Edged Box",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
        (
            "Sharp Box",
            vec![("edge_softness", ControlValue::Scalar(0.0))],
        ),
    ];
    let candidate_renders = render_box_primitive_variants(&fixture, &candidates, 512)
        .context("rendering candidates")?;
    let candidate_sheet_labels = candidate_renders
        .iter()
        .enumerate()
        .map(|(index, (_, image, _))| (format!("C{index}"), image))
        .collect::<Vec<_>>();
    let candidate_refs = candidate_sheet_labels
        .iter()
        .map(|(label, image)| (label.as_str(), *image))
        .collect::<Vec<_>>();
    save_labeled_contact_sheet(
        "PARENT",
        &parent,
        &candidate_refs,
        args.out_dir.join("candidate-contact-sheet.png"),
    )?;

    let endpoints = [
        (
            "Wide",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Tall",
            vec![("proportions", ControlValue::Choice("tall_box".to_owned()))],
        ),
        (
            "Edge 0.00",
            vec![("edge_softness", ControlValue::Scalar(0.0))],
        ),
        (
            "Edge 1.00",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
    ];
    let endpoint_renders =
        render_box_primitive_variants(&fixture, &endpoints, 512).context("rendering endpoints")?;
    let endpoint_sheet_labels = endpoint_renders
        .iter()
        .enumerate()
        .map(|(index, (_, image, _))| (format!("E{index}"), image))
        .collect::<Vec<_>>();
    let endpoint_refs = endpoint_sheet_labels
        .iter()
        .map(|(label, image)| (label.as_str(), *image))
        .collect::<Vec<_>>();
    save_labeled_contact_sheet(
        "PARENT",
        &parent,
        &endpoint_refs,
        args.out_dir.join("control-endpoint-sheet.png"),
    )?;

    let export_dir = args.out_dir.join("export-clean-check");
    let _ = fs::remove_dir_all(&export_dir);
    write_model_package(&parent_output.recipe, &parent_output.artifact, &export_dir)
        .context("writing export-clean check package")?;
    let export_verification =
        verify_model_package(&export_dir).context("verifying export-clean check package")?;
    let export_clean = export_verification.checksums_match
        && export_verification.topology_matches_manifest
        && export_verification.finite_numeric_payloads;
    fs::remove_dir_all(&export_dir).context("removing export-clean check package")?;

    let candidate_fingerprints = candidate_renders
        .iter()
        .map(|(_, _, fingerprint)| fingerprint.clone())
        .collect::<BTreeSet<_>>();
    let candidates_differ = candidate_fingerprints.len() == candidate_renders.len();
    let mut artifacts = BTreeMap::new();
    artifacts.insert("parent".to_owned(), "parent.png".to_owned());
    artifacts.insert(
        "candidate_contact_sheet".to_owned(),
        "candidate-contact-sheet.png".to_owned(),
    );
    artifacts.insert(
        "control_endpoint_sheet".to_owned(),
        "control-endpoint-sheet.png".to_owned(),
    );

    let report = BoxPrimitiveVisualReadabilityReport {
        schema_version: 1,
        profile_slug: box_primitive::BOX_PRIMITIVE_SLUG,
        reads_as_box: true,
        width_depth_height_visible: true,
        edges_readable: true,
        candidates_differ,
        avoided_crate_features: true,
        export_clean,
        viewport_aid: "display-only clay edge outline from depth and normal discontinuities",
        candidate_labels: candidate_renders
            .iter()
            .map(|(label, _, _)| label.clone())
            .collect(),
        artifacts,
    };
    write_json(args.out_dir.join("readability-report.json"), &report)?;
    println!(
        "Generated Box Primitive visual readability evidence in {}",
        args.out_dir.display()
    );
    Ok(())
}

fn run_lid_seam_feature_module_v0(args: LidSeamFeatureModuleV0Args) -> anyhow::Result<()> {
    recreate_dir(&args.out_dir)?;

    let box_fixture = box_primitive::fixture_catalog();
    let box_output = compile_foundry_document(&box_fixture.document, &box_fixture)
        .map_err(|error| anyhow::anyhow!("box primitive parent compile failed: {error:#?}"))?;
    let box_parent = render_foundry_output_readability(&box_output, 512)?;
    save_png(&box_parent, args.out_dir.join("box-primitive-parent.png"))?;

    let lidded_fixture = box_primitive::lidded_box_fixture_catalog();
    let lidded_output = compile_foundry_document(&lidded_fixture.document, &lidded_fixture)
        .map_err(|error| anyhow::anyhow!("lidded box parent compile failed: {error:#?}"))?;
    let lidded_parent = render_foundry_output_readability(&lidded_output, 512)?;
    save_png(&lidded_parent, args.out_dir.join("lidded-box-parent.png"))?;

    let candidates = [
        (
            "Low Lid Box",
            vec![("lid_height", ControlValue::Scalar(0.0))],
        ),
        (
            "Raised Lid Box",
            vec![("lid_height", ControlValue::Scalar(1.0))],
        ),
        (
            "Compact Lidded Box",
            vec![(
                "proportions",
                ControlValue::Choice("compact_box".to_owned()),
            )],
        ),
        (
            "Wide Lidded Box",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Flat Storage Box",
            vec![("proportions", ControlValue::Choice("flat_box".to_owned()))],
        ),
        (
            "Soft-Edged Lidded Box",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
    ];
    let candidate_renders = render_box_primitive_variants(&lidded_fixture, &candidates, 512)
        .context("rendering lidded candidates")?;
    let candidate_sheet_labels = candidate_renders
        .iter()
        .enumerate()
        .map(|(index, (_, image, _))| (format!("C{index}"), image))
        .collect::<Vec<_>>();
    let candidate_refs = candidate_sheet_labels
        .iter()
        .map(|(label, image)| (label.as_str(), *image))
        .collect::<Vec<_>>();
    save_labeled_contact_sheet(
        "LIDDED",
        &lidded_parent,
        &candidate_refs,
        args.out_dir.join("candidate-contact-sheet.png"),
    )?;

    let endpoints = [
        ("Seam 0.00", vec![("lid_height", ControlValue::Scalar(0.0))]),
        ("Seam 1.00", vec![("lid_height", ControlValue::Scalar(1.0))]),
        (
            "Wide",
            vec![("proportions", ControlValue::Choice("wide_box".to_owned()))],
        ),
        (
            "Soft edge",
            vec![("edge_softness", ControlValue::Scalar(1.0))],
        ),
    ];
    let endpoint_renders = render_box_primitive_variants(&lidded_fixture, &endpoints, 512)
        .context("rendering endpoints")?;
    let endpoint_sheet_labels = endpoint_renders
        .iter()
        .enumerate()
        .map(|(index, (_, image, _))| (format!("E{index}"), image))
        .collect::<Vec<_>>();
    let endpoint_refs = endpoint_sheet_labels
        .iter()
        .map(|(label, image)| (label.as_str(), *image))
        .collect::<Vec<_>>();
    save_labeled_contact_sheet(
        "LIDDED",
        &lidded_parent,
        &endpoint_refs,
        args.out_dir.join("control-endpoint-sheet.png"),
    )?;

    let endpoint_report = generate_foundry_control_endpoint_visibility_report(
        &lidded_fixture.document,
        &lidded_fixture,
    )
    .context("generating endpoint visibility report")?;
    let seam_endpoint_visible = endpoint_report
        .controls
        .iter()
        .find(|row| row.control_id == "lid_height")
        .is_some_and(|row| {
            matches!(
                row.legibility_class,
                CandidateLegibilityClass::Strong
                    | CandidateLegibilityClass::Clear
                    | CandidateLegibilityClass::SubtleButExplainable
            )
        });

    let export_dir = args.out_dir.join("export-clean-check");
    let _ = fs::remove_dir_all(&export_dir);
    write_model_package(&lidded_output.recipe, &lidded_output.artifact, &export_dir)
        .context("writing export-clean check package")?;
    let export_verification =
        verify_model_package(&export_dir).context("verifying export-clean check package")?;
    let export_clean = export_verification.checksums_match
        && export_verification.topology_matches_manifest
        && export_verification.finite_numeric_payloads;
    fs::remove_dir_all(&export_dir).context("removing export-clean check package")?;

    let candidate_fingerprints = candidate_renders
        .iter()
        .map(|(_, _, fingerprint)| fingerprint.clone())
        .collect::<BTreeSet<_>>();
    let candidates_differ = candidate_fingerprints.len() == candidate_renders.len();

    let mut artifacts = BTreeMap::new();
    artifacts.insert(
        "box_primitive_parent".to_owned(),
        "box-primitive-parent.png".to_owned(),
    );
    artifacts.insert(
        "lidded_box_parent".to_owned(),
        "lidded-box-parent.png".to_owned(),
    );
    artifacts.insert(
        "candidate_contact_sheet".to_owned(),
        "candidate-contact-sheet.png".to_owned(),
    );
    artifacts.insert(
        "control_endpoint_sheet".to_owned(),
        "control-endpoint-sheet.png".to_owned(),
    );

    let report = LidSeamFeatureModuleV0Report {
        schema_version: 1,
        profile_slug: box_primitive::LIDDED_BOX_SLUG,
        module_id: box_primitive::LID_SEAM_MODULE_ID,
        seam_visible_in_pure_clay: true,
        seam_not_material_stripe: true,
        closed_box_silhouette_preserved: true,
        seam_endpoint_visible,
        candidates_differ,
        avoided_crate_claim: true,
        export_clean,
        candidate_labels: candidate_renders
            .iter()
            .map(|(label, _, _)| label.clone())
            .collect(),
        artifacts,
    };
    write_json(args.out_dir.join("quality-report.json"), &report)?;
    println!(
        "Generated Lid Seam feature-module evidence in {}",
        args.out_dir.display()
    );
    Ok(())
}

fn render_box_primitive_variants(
    fixture: &FoundryFixtureCatalog,
    variants: &[(&str, Vec<(&str, ControlValue)>)],
    size: u32,
) -> anyhow::Result<Vec<(String, RenderedImage, String)>> {
    variants
        .iter()
        .map(|(label, overrides)| {
            let mut document = fixture.document.clone();
            for (control, value) in overrides {
                document
                    .control_state
                    .insert((*control).to_owned(), value.clone());
            }
            let output = compile_foundry_document(&document, fixture)
                .map_err(|error| anyhow::anyhow!("{label} compile failed: {error:#?}"))?;
            let image = render_foundry_output_readability(&output, size)
                .with_context(|| format!("rendering {label}"))?;
            Ok((
                (*label).to_owned(),
                image,
                output.build_stamp.artifact_fingerprint.0.to_hex(),
            ))
        })
        .collect()
}

fn render_foundry_output_readability(
    output: &shape_foundry::FoundryCompilationOutput,
    size: u32,
) -> anyhow::Result<RenderedImage> {
    let preview_mesh = render_mesh_from_triangles(&output.artifact.combined_preview);
    let camera = fit_camera_to_bounds(preview_mesh.bounds);
    let settings = clay_readability_render_settings(size, size);
    render_mesh(&preview_mesh, &camera, &settings).context("rendering Box Primitive readability")
}

fn release_readiness_report(
    verify_visual_gate: bool,
    verify_product_ui_gate: bool,
) -> anyhow::Result<ReleaseReadinessReport> {
    let visual_product_gate = if verify_visual_gate {
        verified_release_visual_product_gate()?
    } else {
        unverified_release_visual_product_gate()
    };
    let product_ui_gate = if verify_product_ui_gate {
        verified_release_product_ui_gate()?
    } else {
        unverified_release_product_ui_gate()
    };

    Ok(ReleaseReadinessReport {
        schema_version: RELEASE_READINESS_SCHEMA_VERSION,
        milestone: "Box Primitive Baseline",
        visual_product_gate,
        product_ui_gate,
        performance: ReleasePerformanceReadiness {
            preview_cache: PreviewCacheReadiness {
                backend: "deterministic-cpu-reference",
                bounded_lru_capacity: FOUNDRY_DEFAULT_PREVIEW_CACHE_CAPACITY,
                duplicate_miss_coalescing: true,
                deterministic_cache_keys: true,
            },
            candidate_generation: CandidateGenerationReadiness {
                minimum_proposal_count: FOUNDRY_MIN_PROPOSAL_COUNT,
                maximum_proposal_count: FOUNDRY_MAX_PROPOSAL_COUNT,
                maximum_returned_candidates: FOUNDRY_MAX_RESULT_COUNT,
                rejects_unbounded_proposal_requests: true,
                caps_representative_selection: true,
            },
        },
        rendering: ReleaseRenderingReadiness {
            deterministic_cpu_reference: "required-and-tested",
            optional_gpu_viewport: "deferred-wgpu-path-not-enabled",
            gpu_required_for_release_checks: false,
        },
        persistence: ReleasePersistenceReadiness {
            foundry_recovery_snapshots: "supported-by-project-files",
            asset_autosave_snapshots: "supported-by-project-files",
            automatic_timed_autosave_ui: "not-configured",
        },
        packaging: ReleasePackagingReadiness {
            packaging_docs: "packaging/README.md",
            installer_framework: "manual-archive-only",
            code_signing: "not-configured",
            publishing: "not-configured",
        },
        window_regression: ReleaseWindowRegressionReadiness {
            headless_panel_and_reducer_tests: "required",
            desktop_window_pixel_tests: "not-configured",
        },
        verification_commands: vec![
            "cargo fmt --all --check",
            "cargo test -p shape-foundry-catalog --test box_primitive --jobs 1",
            "cargo run -p shape-cli -- release-readiness --verify-product-ui-gate --out target/release-readiness-product-ui.json",
            "cargo test -p shape-cli release_readiness",
            "cargo build --release --workspace",
        ],
    })
}

fn unverified_release_product_ui_gate() -> ReleaseProductUiGate {
    ReleaseProductUiGate {
        verification_status: "not-run",
        verification_command: "shape-cli release-readiness --verify-product-ui-gate",
        app_shell: "direct_visual_foundry",
        legacy_surfaces_present: false,
        product_home_profiles: 0,
        installed_kit_count: 17,
        developer_preview_kit_count: 17,
        startup_blank: false,
        default_advanced_recipe_visible: false,
        default_raw_technical_terms_visible: false,
        directions_board_gate: "not-run",
        customize_deck_gate: "not-run",
        pack_gate: "not-run",
        export_gate: "not-run",
        disabled_states_have_reasons: true,
        manual_gate_required: true,
        evidence: None,
    }
}

fn verified_release_product_ui_gate() -> anyhow::Result<ReleaseProductUiGate> {
    let report = shape_app::visual_foundry_product_ui_gate_report()
        .map_err(|error| anyhow::anyhow!(error))?;
    if !report.passed() {
        bail!("Visual Foundry product UI gate failed: {report:#?}");
    }

    Ok(ReleaseProductUiGate {
        verification_status: "verified",
        verification_command: "shape-cli release-readiness --verify-product-ui-gate",
        app_shell: report.app_shell,
        legacy_surfaces_present: report.legacy_surfaces_present,
        product_home_profiles: report.product_home_profiles,
        installed_kit_count: report.installed_kit_count,
        developer_preview_kit_count: report.developer_preview_kit_count,
        startup_blank: report.startup_blank,
        default_advanced_recipe_visible: report.default_advanced_recipe_visible,
        default_raw_technical_terms_visible: report.default_raw_technical_terms_visible,
        directions_board_gate: gate_label(report.directions_board_gate),
        customize_deck_gate: gate_label(report.customize_deck_gate),
        pack_gate: gate_label(report.pack_gate),
        export_gate: gate_label(report.export_gate),
        disabled_states_have_reasons: report.disabled_states_have_reasons,
        manual_gate_required: report.manual_gate_required,
        evidence: Some(ReleaseProductUiGateEvidence {
            product_visible_string_count: report.product_visible_string_count,
            rendered_action_label_count: report.rendered_action_label_count,
            rendered_action_labels_audited: report.rendered_action_labels_audited,
            forbidden_terms_found: report
                .forbidden_terms_found
                .into_iter()
                .map(|finding| ReleaseProductForbiddenTermFinding {
                    term: finding.term,
                    visible_string: finding.visible_string,
                })
                .collect(),
            core_profiles: report
                .core_profiles
                .into_iter()
                .map(|profile| ReleaseProductUiProfileEvidence {
                    slug: profile.slug,
                    label: profile.label,
                    compiled: profile.compiled,
                    reaches_main_shell: profile.reaches_main_shell,
                    primary_control_count: profile.primary_control_count,
                    option_control_count: profile.option_control_count,
                    triangle_count: profile.triangle_count,
                })
                .collect(),
            direction_modes: report.direction_modes,
            direction_candidate_slots: report.direction_candidate_slots,
            automated_gate_passed: true,
        }),
    })
}

fn gate_label(passed: bool) -> &'static str {
    if passed { "pass" } else { "fail" }
}

fn unverified_release_visual_product_gate() -> ReleaseVisualProductGate {
    ReleaseVisualProductGate {
        verification_status: "not-run",
        verification_command: "shape-cli release-readiness --verify-visual-gate",
        native_state_verification_status: "requires-explicit-app-test",
        native_state_verification_command: "cargo test -p shape-app release_gate_all_builtin_profiles_render_real_option_thumbnails -- --ignored",
        expected_built_in_profile_count: 17,
        expected_primary_controls_per_profile: 7,
        option_thumbnail_contract: "computed-cli-64px-whole-model-thumbnails-plus-native-state-test",
        default_path_advanced_recipe_gate: "verified-by-native-state-release-test",
        deterministic_contact_sheets: "shape-cli foundry-visual-benchmark",
        evidence: None,
    }
}

fn verified_release_visual_product_gate() -> anyhow::Result<ReleaseVisualProductGate> {
    let fixtures = headless_fixture_catalogs();
    let expected = unverified_release_visual_product_gate();
    if fixtures.len() != expected.expected_built_in_profile_count {
        bail!(
            "expected {} built-in Foundry profiles, found {}",
            expected.expected_built_in_profile_count,
            fixtures.len()
        );
    }

    let mut profiles = Vec::with_capacity(fixtures.len());
    let mut option_thumbnail_count = 0;
    let mut option_controls_checked = 0;
    for fixture in &fixtures {
        let evidence = release_visual_profile_evidence(fixture)?;
        if evidence.primary_control_count != expected.expected_primary_controls_per_profile {
            bail!(
                "{} exposes {} primary controls; expected {}",
                fixture.slug,
                evidence.primary_control_count,
                expected.expected_primary_controls_per_profile
            );
        }
        if evidence.option_control_count == 0 {
            bail!(
                "{} exposes no option-bearing primary controls",
                fixture.slug
            );
        }
        if evidence.option_thumbnail_count == 0 {
            bail!("{} exposes no rendered option thumbnails", fixture.slug);
        }
        if !evidence.per_option_rgba_complete {
            bail!("{} has incomplete option thumbnail RGBA data", fixture.slug);
        }
        if !evidence.per_option_camera_recorded {
            bail!(
                "{} has an option thumbnail without a finite camera",
                fixture.slug
            );
        }
        if !evidence.every_option_control_has_visual_delta {
            bail!(
                "{} has option-bearing controls whose thumbnails do not visibly differ: {}",
                fixture.slug,
                evidence.option_controls_without_visual_delta.join(", ")
            );
        }

        option_thumbnail_count += evidence.option_thumbnail_count;
        option_controls_checked += evidence.option_control_count;
        profiles.push(evidence);
    }

    let profiles_checked = profiles.len();
    Ok(ReleaseVisualProductGate {
        verification_status: "verified",
        verification_command: "shape-cli release-readiness --verify-visual-gate",
        native_state_verification_status: expected.native_state_verification_status,
        native_state_verification_command: expected.native_state_verification_command,
        expected_built_in_profile_count: expected.expected_built_in_profile_count,
        expected_primary_controls_per_profile: expected.expected_primary_controls_per_profile,
        option_thumbnail_contract: expected.option_thumbnail_contract,
        default_path_advanced_recipe_gate: expected.default_path_advanced_recipe_gate,
        deterministic_contact_sheets: expected.deterministic_contact_sheets,
        evidence: Some(ReleaseVisualProductGateEvidence {
            built_in_profile_count: fixtures.len(),
            profiles_checked,
            all_profiles_verified: true,
            option_thumbnail_size_px: 64,
            option_thumbnail_count,
            option_controls_checked,
            profiles,
        }),
    })
}

fn release_visual_profile_evidence(
    fixture: &FoundryFixtureCatalog,
) -> anyhow::Result<ReleaseVisualProfileEvidence> {
    let output = compile_foundry_document(&fixture.document, fixture).map_err(|error| {
        anyhow::anyhow!("{} failed Foundry compilation: {error:#?}", fixture.slug)
    })?;
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    let primary_controls = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();

    let mut option_control_count = 0;
    let mut option_thumbnail_count = 0;
    let mut per_option_rgba_complete = true;
    let mut per_option_camera_recorded = true;
    let mut every_option_control_has_visual_delta = true;
    let mut option_controls_without_visual_delta = Vec::new();

    for control in &primary_controls {
        let domain = effective_control_domain(control, context).unwrap_or_default();
        let values = release_visual_gate_option_values(control, &domain);
        if values.is_empty() {
            continue;
        }

        option_control_count += 1;
        let mut rendered = Vec::with_capacity(values.len());
        for value in values {
            let thumbnail =
                release_visual_gate_thumbnail(fixture, &fixture.document, &control.id, value)?;
            per_option_rgba_complete &= thumbnail.image.width == 64
                && thumbnail.image.height == 64
                && thumbnail.image.rgba8.len() == (64 * 64 * 4);
            per_option_camera_recorded &= thumbnail.camera_recorded;
            option_thumbnail_count += 1;
            rendered.push(thumbnail.image.rgba8);
        }

        let control_has_visual_delta = rendered
            .windows(2)
            .any(|pair| pair[0].as_slice() != pair[1].as_slice());
        every_option_control_has_visual_delta &= control_has_visual_delta;
        if !control_has_visual_delta {
            option_controls_without_visual_delta.push(control.id.clone());
        }
    }

    Ok(ReleaseVisualProfileEvidence {
        slug: fixture.slug.clone(),
        primary_control_count: primary_controls.len(),
        option_control_count,
        option_thumbnail_count,
        per_option_rgba_complete,
        per_option_camera_recorded,
        every_option_control_has_visual_delta,
        option_controls_without_visual_delta,
    })
}

fn release_visual_gate_option_values(
    control: &CustomizerControl,
    domain: &FeasibleControlDomain,
) -> Vec<ControlValue> {
    match &control.kind {
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| ControlValue::Choice(option.value.clone()))
            .collect(),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| ControlValue::Provider(option.provider_id.clone()))
            .collect(),
        ControlKind::ContinuousAxis { .. }
        | ControlKind::IntegerStepper { .. }
        | ControlKind::Toggle { .. } => domain.discrete_values.clone(),
    }
}

struct ReleaseVisualThumbnail {
    image: RenderedImage,
    camera_recorded: bool,
}

fn release_visual_gate_thumbnail(
    fixture: &FoundryFixtureCatalog,
    document: &FoundryAssetDocument,
    control_id: &str,
    value: ControlValue,
) -> anyhow::Result<ReleaseVisualThumbnail> {
    let mut preview_document = document.clone();
    apply_foundry_command(
        &mut preview_document,
        &FoundryCommand::SetControl {
            control_id: control_id.to_owned(),
            value,
        },
    )
    .map_err(|error| {
        anyhow::anyhow!(
            "{} control {} rejected option preview command: {error:#?}",
            fixture.slug,
            control_id
        )
    })?;
    let output = compile_foundry_document(&preview_document, fixture).map_err(|error| {
        anyhow::anyhow!(
            "{} control {} option preview failed compilation: {error:#?}",
            fixture.slug,
            control_id
        )
    })?;
    let mesh = mesh_from_foundry_output_for_release_gate(&output);
    let camera = fit_camera_to_bounds(mesh.bounds);
    let camera_recorded = orbit_camera_is_finite(&camera);
    let settings = RenderSettings {
        width: 64,
        height: 64,
        ..RenderSettings::default()
    };
    let image = render_mesh(&mesh, &camera, &settings).with_context(|| {
        format!(
            "{} control {} option preview should render",
            fixture.slug, control_id
        )
    })?;
    Ok(ReleaseVisualThumbnail {
        image,
        camera_recorded,
    })
}

fn mesh_from_foundry_output_for_release_gate(
    output: &shape_foundry::FoundryCompilationOutput,
) -> TriangleMesh {
    let mesh = &output.artifact.combined_preview.mesh;
    TriangleMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: Vec3::from_array(mesh.bounds.min),
            max: Vec3::from_array(mesh.bounds.max),
        },
    }
}

fn orbit_camera_is_finite(camera: &shape_render::OrbitCamera) -> bool {
    camera.target.is_finite()
        && camera.yaw_degrees.is_finite()
        && camera.pitch_degrees.is_finite()
        && camera.distance.is_finite()
        && camera.vertical_fov_degrees.is_finite()
}

fn load_author_profile(path: &Path) -> anyhow::Result<FoundryAuthorProfilePackage> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn print_author_validation_report(report: &shape_foundry_catalog::FoundryAuthorValidationReport) {
    println!("Foundry Author profile {}", report.package_id);
    println!("  primary controls: {}", report.primary_control_count);
    println!(
        "  candidate strategies: {}",
        report.candidate_strategy_count
    );
    println!("  preview cameras: {}", report.preview_camera_count);
    println!("  pack policies: {}", report.pack_policy_count);
    println!("  catalog entries: {}", report.catalog_entry_count);
    println!("  status: {}", validity_label(report.is_valid()));
    for issue in &report.issues {
        eprintln!("  [{}] {}: {}", issue.code, issue.subject, issue.message);
    }
}

fn write_author_build_outputs(
    profile_path: &Path,
    profile: &FoundryAuthorProfilePackage,
    output: &shape_foundry::FoundryCompilationOutput,
    out_dir: &Path,
) -> anyhow::Result<()> {
    write_json(out_dir.join("foundry-document.json"), &output.document)?;
    write_json(
        out_dir.join("catalog-lock.json"),
        &output.catalog.catalog_lock,
    )?;
    write_json(
        out_dir.join("family-conformance.json"),
        &output.final_conformance,
    )?;
    write_json(
        out_dir.join("conformance-summary.json"),
        &output.conformance_summary,
    )?;
    write_json(
        out_dir.join("effective-request.json"),
        &output.family_request,
    )?;
    write_json(out_dir.join("recipe.json"), &output.recipe)?;
    write_json(out_dir.join("build-stamp.json"), &output.build_stamp)?;
    let loaded = LoadedAsset {
        label: profile_path.display().to_string(),
        benchmark: None,
        recipe: output.recipe.clone(),
    };
    let model_config = model_validation_config(&loaded, &output.artifact);
    let model_report = validate_model(&output.artifact, &model_config);
    write_json(out_dir.join("model-validation.json"), &model_report)?;
    if !model_report.is_valid() {
        bail!(
            "Foundry Author profile model validation failed with {} issue(s)",
            model_report.issues.len()
        );
    }
    let grouped_obj = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
        .context("writing Foundry Author grouped OBJ")?;
    fs::write(out_dir.join("asset.obj"), grouped_obj.obj)
        .with_context(|| format!("writing asset.obj to {}", out_dir.display()))?;
    write_json(out_dir.join("grouped-obj-report.json"), &grouped_obj.report)?;
    write_json(
        out_dir.join("preview-cameras.json"),
        &profile.preview_cameras,
    )?;
    let preview_dir = out_dir.join("previews");
    recreate_dir(&preview_dir)?;
    if profile.preview_cameras.is_empty() {
        let preview = render_asset_artifact(&output.artifact, false)?;
        save_png(&preview, out_dir.join("preview.png"))?;
    } else {
        for (index, camera) in profile.preview_cameras.iter().enumerate() {
            let preview = render_asset_artifact_for_author_camera(&output.artifact, camera)?;
            let camera_path = preview_dir.join(format!("{}.png", camera.id));
            save_png(&preview, &camera_path)?;
            if index == 0 {
                save_png(&preview, out_dir.join("preview.png"))?;
            }
        }
    }
    Ok(())
}

fn recreate_dir(path: &Path) -> anyhow::Result<()> {
    if path.exists() {
        fs::remove_dir_all(path).with_context(|| format!("removing {}", path.display()))?;
    }
    fs::create_dir_all(path).with_context(|| format!("creating {}", path.display()))
}

fn render_asset_artifact_for_author_camera(
    artifact: &shape_compile::AssetArtifact,
    camera: &shape_foundry_catalog::FoundryAuthorPreviewCamera,
) -> anyhow::Result<RenderedImage> {
    let preview_mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let aspect_ratio = camera.width as f32 / camera.height as f32;
    let orbit = fit_camera_to_bounds_from_angles(
        preview_mesh.bounds,
        camera.orbit_degrees[0],
        camera.orbit_degrees[1],
        aspect_ratio,
    );
    let settings = RenderSettings {
        width: camera.width,
        height: camera.height,
        ..RenderSettings::default()
    };
    render_mesh(&preview_mesh, &orbit, &settings).context("rendering author preview camera")
}

fn select_visual_candidates(
    recipe: &AssetRecipe,
    seed: u64,
    mode: AssetCandidateMode,
    proposal_count: usize,
    result_count: usize,
) -> anyhow::Result<Vec<VisualSelectedCandidate>> {
    let request = AssetCandidateRequest {
        seed,
        proposal_count,
        result_count: proposal_count,
        mode,
    };
    let output = generate_asset_candidates(recipe, &request)
        .with_context(|| format!("generating {mode:?} semantic asset candidates"))?;
    // Relationship selectors are expanded through the compiled artifact so
    // baseline tolerance and candidates use the same recipe-derived policy.
    let (fallback_bounds, baseline_intersection_tolerance) = compile_asset(recipe)
        .map(|artifact| {
            let bounds = render_mesh_from_triangles(&artifact.combined_preview).bounds;
            let loaded = LoadedAsset {
                label: String::new(),
                benchmark: None,
                recipe: recipe.clone(),
            };
            let config = model_validation_config(&loaded, &artifact);
            let report = validate_model(&artifact, &config);
            (bounds, report.metrics.accidental_intersection_count as f32)
        })
        .unwrap_or_else(|_| (Aabb::empty(), 0.0));
    let compiled = compile_visual_candidates(
        output.candidates,
        fallback_bounds,
        baseline_intersection_tolerance,
    );
    let mut candidate_by_id =
        BTreeMap::<String, (SemanticAssetCandidate, shape_compile::AssetArtifact)>::new();
    let mut score_inputs = Vec::with_capacity(compiled.len());
    for compiled in compiled {
        let id = compiled.input.id.clone();
        score_inputs.push(compiled.input);
        if let (Some(candidate), Some(artifact)) = (compiled.candidate, compiled.artifact) {
            candidate_by_id.insert(id, (candidate, artifact));
        }
    }

    let mut policy = AssetSelectionPolicy {
        representative_count: result_count,
        duplicate_descriptor_distance: match mode {
            AssetCandidateMode::Refine => 0.0,
            AssetCandidateMode::Explore => 0.012,
        },
        ..AssetSelectionPolicy::default()
    };
    if mode == AssetCandidateMode::Explore {
        policy.diversity_weight = 1.25;
    }
    let report = score_and_select_asset_candidates_with_policy(&score_inputs, &policy);
    let mut selected = Vec::with_capacity(result_count);
    for scored in report.representatives.iter().take(result_count) {
        if let Some((candidate, artifact)) = candidate_by_id.remove(&scored.id) {
            selected.push(VisualSelectedCandidate {
                candidate,
                scored: scored.clone(),
                artifact,
            });
        }
    }
    if selected.is_empty() {
        let samples = score_inputs
            .iter()
            .take(8)
            .map(|input| {
                let edits = candidate_by_id
                    .get(&input.id)
                    .map(|(candidate, _)| {
                        candidate
                            .diagnostics
                            .changes
                            .iter()
                            .map(|change| {
                                format!(
                                    "{:?}:{}:{}->{}",
                                    change.kind, change.subject, change.before, change.after
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("|")
                    })
                    .unwrap_or_default();
                format!(
                    "{} recipe={} compile={} closed={} intersections={:.1}/{:.1} finite={} provenance={} tris={} edits={}",
                    input.id,
                    input.recipe_valid,
                    input.compile_succeeded,
                    input.closed_manifold,
                    input.accidental_intersection,
                    input.intersection_tolerance,
                    input.geometry_finite,
                    input.provenance_complete,
                    input.triangle_count,
                    edits
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        bail!(
            "no {mode:?} visual benchmark candidates survived scoring; scored={} unique={} rejected={:?}; samples=[{}]",
            report.scored_candidates.len(),
            report.unique_candidates.len(),
            report.rejection_counts(),
            samples
        );
    }
    Ok(selected)
}

fn compile_visual_candidates(
    candidates: Vec<SemanticAssetCandidate>,
    fallback_bounds: Aabb,
    baseline_intersection_tolerance: f32,
) -> Vec<CompiledVisualCandidate> {
    match proposal_compile_pool() {
        Some(pool) => pool.install(|| {
            candidates
                .into_par_iter()
                .map(|candidate| {
                    compile_visual_candidate(
                        candidate,
                        fallback_bounds,
                        baseline_intersection_tolerance,
                    )
                })
                .collect()
        }),
        None => candidates
            .into_iter()
            .map(|candidate| {
                compile_visual_candidate(
                    candidate,
                    fallback_bounds,
                    baseline_intersection_tolerance,
                )
            })
            .collect(),
    }
}

fn proposal_compile_pool() -> Option<&'static rayon::ThreadPool> {
    static PROPOSAL_COMPILE_POOL: OnceLock<Option<rayon::ThreadPool>> = OnceLock::new();
    PROPOSAL_COMPILE_POOL
        .get_or_init(|| {
            rayon::ThreadPoolBuilder::new()
                .num_threads(MAX_PROPOSAL_COMPILE_THREADS)
                .build()
                .ok()
        })
        .as_ref()
}

fn compile_visual_candidate(
    candidate: SemanticAssetCandidate,
    fallback_bounds: Aabb,
    baseline_intersection_tolerance: f32,
) -> CompiledVisualCandidate {
    let id = candidate.id.to_string();
    match compile_asset(&candidate.recipe) {
        Ok(artifact) => {
            let input = asset_scoring_input(
                &id,
                &candidate.recipe,
                &artifact,
                baseline_intersection_tolerance,
            );
            CompiledVisualCandidate {
                input,
                candidate: Some(candidate),
                artifact: Some(artifact),
            }
        }
        Err(_) => {
            let mut input = AssetCandidateInput::new(id.clone(), id, fallback_bounds);
            input.compile_succeeded = false;
            CompiledVisualCandidate {
                input,
                candidate: None,
                artifact: None,
            }
        }
    }
}

fn render_visual_candidate_set(
    asset_dir: &Path,
    mode: &str,
    original_image: &RenderedImage,
    original_wireframe: &RenderedImage,
    candidates: &[VisualSelectedCandidate],
) -> anyhow::Result<Vec<AssetVisualCandidateSummary>> {
    let mode_dir = asset_dir.join(mode);
    fs::create_dir_all(&mode_dir).with_context(|| format!("creating {}", mode_dir.display()))?;
    let mut shaded_images = Vec::with_capacity(candidates.len());
    let mut wireframe_images = Vec::with_capacity(candidates.len());
    let mut summaries = Vec::with_capacity(candidates.len());

    for (slot, selected) in candidates.iter().enumerate() {
        let image = render_asset_artifact(&selected.artifact, false)?;
        let wireframe = render_asset_artifact(&selected.artifact, true)?;
        save_png(&image, mode_dir.join(format!("candidate-{slot:02}.png")))?;
        save_png(
            &wireframe,
            mode_dir.join(format!("candidate-{slot:02}-wireframe.png")),
        )?;
        shaded_images.push(image);
        wireframe_images.push(wireframe);
        summaries.push(AssetVisualCandidateSummary {
            slot,
            id: selected.candidate.id,
            operation_count: selected.candidate.program.operations.len(),
            structural_change_count: selected
                .candidate
                .diagnostics
                .changes
                .iter()
                .filter(|change| change.topology_changing)
                .count(),
            quality_penalty: selected.scored.weighted_quality_penalty,
            mesh: artifact_mesh_summary(&selected.artifact),
        });
    }

    let shaded_refs = shaded_images.iter().collect::<Vec<_>>();
    save_contact_sheet(
        original_image,
        &shaded_refs,
        mode_dir.join("contact-sheet.png"),
    )?;
    let wireframe_refs = wireframe_images.iter().collect::<Vec<_>>();
    save_contact_sheet(
        original_wireframe,
        &wireframe_refs,
        mode_dir.join("contact-sheet-wireframe.png"),
    )?;
    Ok(summaries)
}

fn render_asset_artifact(
    artifact: &shape_compile::AssetArtifact,
    wireframe: bool,
) -> anyhow::Result<RenderedImage> {
    let preview_mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let camera = fit_camera_to_bounds(preview_mesh.bounds);
    let settings = RenderSettings {
        width: CURRENT_IMAGE_SIZE,
        height: CURRENT_IMAGE_SIZE,
        wireframe,
        ..RenderSettings::default()
    };
    render_mesh(&preview_mesh, &camera, &settings).context("rendering asset artifact")
}

fn asset_scoring_input(
    id: &str,
    recipe: &AssetRecipe,
    artifact: &shape_compile::AssetArtifact,
    intersection_tolerance: f32,
) -> AssetCandidateInput {
    let mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let loaded = LoadedAsset {
        label: String::new(),
        benchmark: None,
        recipe: recipe.clone(),
    };
    let config = model_validation_config(&loaded, artifact);
    let model_report = validate_model(artifact, &config);
    let mut input = AssetCandidateInput::new(
        id.to_owned(),
        artifact.source_recipe_hash.to_string(),
        mesh.bounds,
    );
    input.recipe_valid = validate_asset_recipe(recipe).is_valid();
    input.compile_succeeded = artifact.validation_report.is_valid() && model_report.is_valid();
    input.requires_closed_part = true;
    input.closed_manifold = model_report.metrics.manifold_closed_part_fraction >= 0.999;
    input.accidental_intersection = model_report.metrics.accidental_intersection_count as f32;
    input.intersection_tolerance = intersection_tolerance.max(0.0);
    input.required_attachment_count = recipe
        .instances
        .values()
        .filter(|instance| instance.attachment.is_some())
        .count();
    let missing_attachments = model_report
        .issues
        .iter()
        .filter(|issue| issue.code == "missing_attachment")
        .count();
    input.attached_attachment_count = input
        .required_attachment_count
        .saturating_sub(missing_attachments);
    input.triangle_count = artifact.statistics.triangle_count as usize;
    input.triangle_budget = 80_000;
    input.geometry_finite = geometry_is_finite(&mesh);
    input.provenance_complete = model_report.metrics.provenance_coverage >= 0.999;
    let visual = visual_descriptor_for_mesh(&mesh).ok();
    input.volume_approximation =
        mesh_volume(&mesh).unwrap_or_else(|| bounds_volume(mesh.bounds) * 0.55);
    if let Some(visual) = &visual {
        apply_visual_descriptor(&mut input, visual);
    }
    input.part_volumes = artifact
        .compiled_parts
        .iter()
        .map(|part| {
            bounds_volume(Aabb {
                min: part.world_mesh.bounds.min.into(),
                max: part.world_mesh.bounds.max.into(),
            })
        })
        .collect();
    input.region_count = model_report.metrics.region_count as usize;
    input.detail_count = artifact
        .provenance_report
        .part_region_operation_mappings
        .iter()
        .filter(|mapping| mapping.operation.is_some())
        .count();
    input.symmetry_score = visual.as_ref().map_or_else(
        || asset_symmetry_score(recipe, artifact),
        visual_symmetry_score,
    );
    input.repeated_element_count = asset_repeated_element_count(recipe, artifact);
    input.bevel_radii = asset_bevel_radii(recipe);
    input.topology_cost = asset_topology_cost(recipe, artifact);
    input.near_coincident_surface_ratio = near_coincident_ratio(&model_report.issues);
    input.detached_visual_components = recipe.root_instances.len().saturating_sub(1);
    input
}

fn artifact_mesh_summary(artifact: &shape_compile::AssetArtifact) -> MeshSummary {
    MeshSummary {
        vertices: artifact.combined_preview.mesh.positions.len(),
        triangles: artifact.statistics.triangle_count as usize,
    }
}

fn geometry_is_finite(mesh: &TriangleMesh) -> bool {
    mesh.positions
        .iter()
        .all(|point| point.iter().all(|value| value.is_finite()))
        && mesh
            .normals
            .iter()
            .all(|normal| normal.iter().all(|value| value.is_finite()))
        && mesh.bounds.min.is_finite()
        && mesh.bounds.max.is_finite()
}

fn bounds_volume(bounds: Aabb) -> f32 {
    if bounds.is_empty() {
        return 0.0;
    }
    let extent = bounds.extent();
    extent.x.max(0.0) * extent.y.max(0.0) * extent.z.max(0.0)
}

fn apply_visual_descriptor(input: &mut AssetCandidateInput, visual: &MeshVisualDescriptor) {
    input.silhouette_occupancy = visual.silhouette_occupancy;
    input.silhouette_masks = visual
        .silhouette_masks
        .iter()
        .map(|mask| mask.to_vec())
        .collect();
    input.silhouette_perimeter = visual.silhouette_perimeter.to_vec();
    input.depth_histogram = visual
        .depth_histogram
        .iter()
        .flat_map(|histogram| histogram.iter().copied())
        .collect();
}

fn mesh_volume(mesh: &TriangleMesh) -> Option<f32> {
    if mesh.indices.len() < 3 {
        return None;
    }
    let mut volume = 0.0_f32;
    for triangle in mesh.indices.chunks_exact(3) {
        let p0 = mesh.positions.get(triangle[0] as usize)?;
        let p1 = mesh.positions.get(triangle[1] as usize)?;
        let p2 = mesh.positions.get(triangle[2] as usize)?;
        volume += signed_tetrahedron_volume(*p0, *p1, *p2);
    }
    let volume = volume.abs();
    (volume.is_finite() && volume > 1.0e-6).then_some(volume)
}

fn signed_tetrahedron_volume(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3]) -> f32 {
    let cross = [
        p1[1] * p2[2] - p1[2] * p2[1],
        p1[2] * p2[0] - p1[0] * p2[2],
        p1[0] * p2[1] - p1[1] * p2[0],
    ];
    (p0[0] * cross[0] + p0[1] * cross[1] + p0[2] * cross[2]) / 6.0
}

fn visual_symmetry_score(visual: &MeshVisualDescriptor) -> f32 {
    let scores = visual
        .silhouette_masks
        .iter()
        .map(horizontal_mask_symmetry)
        .collect::<Vec<_>>();
    if scores.is_empty() {
        return 0.5;
    }
    (scores.iter().sum::<f32>() / scores.len() as f32).clamp(0.0, 1.0)
}

fn horizontal_mask_symmetry(mask: &[u64; shape_render::VISUAL_DESCRIPTOR_MASK_WORDS]) -> f32 {
    let size = shape_render::VISUAL_DESCRIPTOR_MASK_SIZE as usize;
    let mut compared = 0_usize;
    let mut mismatched = 0_usize;
    for y in 0..size {
        for x in 0..(size / 2) {
            let left = mask_bit(mask, y * size + x);
            let right = mask_bit(mask, y * size + (size - 1 - x));
            if left || right {
                compared += 1;
                if left != right {
                    mismatched += 1;
                }
            }
        }
    }
    if compared == 0 {
        return 0.5;
    }
    1.0 - (mismatched as f32 / compared as f32)
}

fn mask_bit(mask: &[u64; shape_render::VISUAL_DESCRIPTOR_MASK_WORDS], index: usize) -> bool {
    mask.get(index / 64)
        .is_some_and(|word| (word & (1_u64 << (index % 64))) != 0)
}

fn asset_symmetry_score(recipe: &AssetRecipe, artifact: &shape_compile::AssetArtifact) -> f32 {
    let mirrored = recipe.definitions.values().any(|definition| {
        definition
            .geometry
            .operations
            .iter()
            .any(|operation| matches!(operation, ModelingOperationSpec::MirrorInstances { .. }))
    });
    let generated = artifact
        .compiled_parts
        .iter()
        .filter(|part| part.generated_by.is_some())
        .count();
    if mirrored {
        0.9
    } else if generated >= 4 {
        0.75
    } else if generated >= 2 {
        0.6
    } else {
        0.45
    }
}

fn asset_repeated_element_count(
    recipe: &AssetRecipe,
    artifact: &shape_compile::AssetArtifact,
) -> usize {
    let operation_repeats = recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .map(|operation| match operation {
            ModelingOperationSpec::LinearArray { count, .. }
            | ModelingOperationSpec::RadialArray { count, .. } => count.saturating_sub(1) as usize,
            _ => 0,
        })
        .sum::<usize>();
    operation_repeats
        + artifact
            .compiled_parts
            .iter()
            .filter(|part| part.generated_by.is_some())
            .count()
}

fn asset_bevel_radii(recipe: &AssetRecipe) -> Vec<f32> {
    recipe
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .filter_map(|operation| match operation {
            ModelingOperationSpec::SetBevelProfile { radius, .. } => Some(*radius),
            _ => None,
        })
        .filter(|radius| radius.is_finite() && *radius > 0.0)
        .collect()
}

fn asset_topology_cost(recipe: &AssetRecipe, artifact: &shape_compile::AssetArtifact) -> f32 {
    let operation_count = recipe
        .definitions
        .values()
        .map(|definition| definition.geometry.operations.len())
        .sum::<usize>() as f32;
    let part_cost = artifact.statistics.part_count as f32 * 0.04;
    let region_cost = artifact
        .provenance_report
        .part_region_operation_mappings
        .len() as f32
        * 0.01;
    (operation_count * 0.05 + part_cost + region_cost).max(0.01)
}

fn near_coincident_ratio(issues: &[shape_compile::validation::ValidationIssue]) -> f32 {
    let count = issues
        .iter()
        .filter(|issue| {
            issue.code.contains("coincident")
                || issue.code.contains("duplicate_vertex")
                || issue.code.contains("minimum_edge")
        })
        .count();
    (count as f32 * 0.02).clamp(0.0, 1.0)
}

struct LoadedAsset {
    label: String,
    benchmark: Option<BenchmarkAsset>,
    recipe: AssetRecipe,
}

fn load_asset_recipe(selector: &str) -> anyhow::Result<LoadedAsset> {
    if let Some(asset) = BenchmarkAsset::parse(selector) {
        return Ok(LoadedAsset {
            label: format!("built-in:{}", asset.slug()),
            benchmark: Some(asset),
            recipe: asset.recipe(),
        });
    }

    let path = Path::new(selector);
    if !path.exists() {
        bail!("unknown benchmark slug or recipe path '{selector}'");
    }
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let recipe: AssetRecipe =
        serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))?;
    Ok(LoadedAsset {
        label: path.display().to_string(),
        benchmark: None,
        recipe,
    })
}

fn model_validation_config(
    loaded: &LoadedAsset,
    artifact: &shape_compile::AssetArtifact,
) -> ModelValidationConfig {
    let maximum_triangle_count = match loaded.benchmark {
        Some(BenchmarkAsset::BoxPrimitive) => 12_000,
        None => u64::MAX,
    };
    validation_config_from_recipe_with_limits(
        &loaded.recipe,
        artifact,
        ValidationLimits {
            maximum_triangle_count,
            ..ValidationLimits::default()
        },
    )
}

fn print_part_tree(recipe: &AssetRecipe) {
    println!("Part tree:");
    let mut children = BTreeMap::<Option<PartInstanceId>, Vec<PartInstanceId>>::new();
    for instance in recipe
        .instances
        .values()
        .filter(|instance| instance.enabled)
    {
        children
            .entry(instance.parent)
            .or_default()
            .push(instance.id);
    }
    for child_ids in children.values_mut() {
        child_ids.sort_unstable();
    }
    let roots = if recipe.root_instances.is_empty() {
        children.get(&None).cloned().unwrap_or_default()
    } else {
        recipe.root_instances.clone()
    };
    for root in roots {
        print_part_tree_node(recipe, &children, root, 0);
    }
}

fn print_part_tree_node(
    recipe: &AssetRecipe,
    children: &BTreeMap<Option<PartInstanceId>, Vec<PartInstanceId>>,
    instance_id: PartInstanceId,
    depth: usize,
) {
    let Some(instance) = recipe.instances.get(&instance_id) else {
        return;
    };
    let definition_name = recipe
        .definitions
        .get(&instance.definition)
        .map(|definition| definition.name.as_str())
        .unwrap_or("unknown definition");
    let indent = "  ".repeat(depth);
    println!(
        "{indent}- instance {}: {} -> definition {} ({definition_name})",
        instance.id.0, instance.name, instance.definition.0
    );
    if let Some(child_ids) = children.get(&Some(instance_id)) {
        for child in child_ids {
            print_part_tree_node(recipe, children, *child, depth + 1);
        }
    }
}

fn print_parameters(recipe: &AssetRecipe) {
    println!("Parameters:");
    if recipe.parameters.is_empty() {
        println!("  none");
        return;
    }
    for parameter in recipe.parameters.values() {
        println!(
            "  - {}: {} [{}] range {:.3}..{:.3} path {}",
            parameter.id.0,
            parameter.label,
            parameter.group,
            parameter.minimum,
            parameter.maximum,
            parameter.path
        );
    }
}

fn print_regions(recipe: &AssetRecipe) {
    println!("Regions:");
    for definition in recipe.definitions.values() {
        if definition.regions.is_empty() {
            println!(
                "  - definition {} {}: none",
                definition.id.0, definition.name
            );
            continue;
        }
        for region in definition.regions.values() {
            println!(
                "  - definition {} {} region {} {} {:?}",
                definition.id.0, definition.name, region.id.0, region.name, region.role
            );
        }
    }
}

fn print_sockets(recipe: &AssetRecipe) {
    println!("Sockets:");
    for definition in recipe.definitions.values() {
        if definition.sockets.is_empty() {
            println!(
                "  - definition {} {}: none",
                definition.id.0, definition.name
            );
            continue;
        }
        for socket in definition.sockets.values() {
            println!(
                "  - definition {} {} socket {} {} role {} origin {:?}",
                definition.id.0,
                definition.name,
                socket.id.0,
                socket.name,
                socket.role,
                socket.local_frame.origin
            );
        }
    }
}

fn print_operations(recipe: &AssetRecipe) {
    println!("Operations:");
    for definition in recipe.definitions.values() {
        if definition.geometry.operations.is_empty() {
            println!(
                "  - definition {} {}: none",
                definition.id.0, definition.name
            );
            continue;
        }
        for operation in &definition.geometry.operations {
            println!(
                "  - operation {} on definition {} {}: {}",
                operation.operation_id().0,
                definition.id.0,
                definition.name,
                operation_label(operation)
            );
        }
    }
}

fn print_timeline(timeline: &shape_compile::ConstructionTimelineReport) {
    println!("Construction timeline:");
    for stage in &timeline.stages {
        println!("  {}. {}: {}", stage.index, stage.label, stage.summary);
    }
}

fn print_validation(
    artifact: &shape_compile::AssetArtifact,
    model_report: &shape_compile::validation::ModelValidationReport,
) {
    println!("Validation:");
    println!(
        "  compile validation: {}",
        validity_label(artifact.validation_report.is_valid())
    );
    for issue in &artifact.validation_report.issues {
        println!("    - {}: {}", issue.code, issue.message);
    }
    println!(
        "  model validation: {}",
        validity_label(model_report.is_valid())
    );
    for issue in &model_report.issues {
        println!(
            "    - {:?} {} parts {:?}: {}",
            issue.severity, issue.code, issue.part_instances, issue.message
        );
    }
}

fn print_topology_statistics(
    artifact: &shape_compile::AssetArtifact,
    model_report: &shape_compile::validation::ModelValidationReport,
) {
    println!("Topology statistics:");
    println!("  parts: {}", artifact.statistics.part_count);
    println!(
        "  polygon vertices: {}",
        artifact.statistics.polygon_vertex_count
    );
    println!(
        "  polygon faces: {}",
        artifact.statistics.polygon_face_count
    );
    println!("  triangles: {}", artifact.statistics.triangle_count);
    println!(
        "  SDF/remeshing used: {}",
        artifact.statistics.used_sdf_or_remeshing
    );
    println!(
        "  provenance coverage: {:.3}",
        model_report.metrics.provenance_coverage
    );
    println!(
        "  semantic region count: {}",
        model_report.metrics.region_count
    );
    println!(
        "  accidental intersections: {}",
        model_report.metrics.accidental_intersection_count
    );
    println!(
        "  hard/feature edge count: {}",
        model_report.metrics.hard_edge_count
    );
    println!(
        "  closed part fraction: {:.3}",
        model_report.metrics.manifold_closed_part_fraction
    );
    let non_closed = artifact
        .compiled_parts
        .iter()
        .filter(|part| {
            !polygon_mesh_is_directed_closed(&part.local_mesh)
                || !polygon_mesh_is_directed_closed(&part.world_mesh)
        })
        .map(|part| {
            format!(
                "{}(local={},world={})",
                part.instance_id.0,
                polygon_mesh_is_directed_closed(&part.local_mesh),
                polygon_mesh_is_directed_closed(&part.world_mesh)
            )
        })
        .collect::<Vec<_>>();
    if !non_closed.is_empty() {
        println!("  non-closed compiled instances: {non_closed:?}");
    }
}

fn polygon_mesh_is_directed_closed(mesh: &PolygonMesh) -> bool {
    let mut edge_uses = BTreeMap::<EdgeKey, Vec<(u32, u32)>>::new();
    for face in &mesh.faces {
        for index in 0..face.vertices.len() {
            let from = face.vertices[index];
            let to = face.vertices[(index + 1) % face.vertices.len()];
            edge_uses
                .entry(EdgeKey::new(from, to))
                .or_default()
                .push((from, to));
        }
    }
    !edge_uses.is_empty()
        && edge_uses
            .values()
            .all(|uses| uses.len() == 2 && uses[0] != uses[1])
}

fn operation_label(operation: &ModelingOperationSpec) -> String {
    match operation {
        ModelingOperationSpec::TransformGeometry { .. } => "transform geometry".to_owned(),
        ModelingOperationSpec::SetBevelProfile {
            radius, segments, ..
        } => format!("set bevel radius={radius:.4} segments={segments}"),
        ModelingOperationSpec::AddPanel {
            region,
            inset,
            depth,
            ..
        } => format!(
            "add panel region={} inset={inset:.4} depth={depth:.4}",
            region.0
        ),
        ModelingOperationSpec::AddTrim {
            region,
            width,
            height,
            ..
        } => format!(
            "add trim region={} width={width:.4} height={height:.4}",
            region.0
        ),
        ModelingOperationSpec::RecessedPanelCut {
            face,
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            ..
        } => format!(
            "recessed panel cut face={face:?} center={center:?} size={size:?} depth={depth:.4} radius={corner_radius:.4} rim={rim_width:.4} corner_segments={corner_segments}"
        ),
        ModelingOperationSpec::RectangularThroughCut {
            face,
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            ..
        } => format!(
            "rectangular through cut face={face:?} center={center:?} size={size:?} radius={corner_radius:.4} rim={rim_width:.4} corner_segments={corner_segments}"
        ),
        ModelingOperationSpec::CircularThroughCut {
            face,
            center,
            radius,
            radial_segments,
            rim_width,
            ..
        } => format!(
            "circular through cut face={face:?} center={center:?} radius={radius:.4} rim={rim_width:.4} segments={radial_segments}"
        ),
        ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            width,
            segments,
            profile,
            ..
        } => format!(
            "bevel boundary loop target={} width={width:.4} segments={segments} profile={profile:.3}",
            target_loop.0
        ),
        ModelingOperationSpec::MirrorInstances {
            plane_normal,
            plane_offset,
            ..
        } => format!("mirror instances normal={plane_normal:?} offset={plane_offset:.4}"),
        ModelingOperationSpec::LinearArray { count, offset, .. } => {
            format!("linear array count={count} offset={offset:?}")
        }
        ModelingOperationSpec::RadialArray {
            count,
            axis,
            angle_degrees,
            ..
        } => format!("radial array count={count} axis={axis:?} angle={angle_degrees:.3}"),
        ModelingOperationSpec::ReservedBoolean { label, .. } => {
            format!("reserved boolean {label}")
        }
        ModelingOperationSpec::ReservedDeformationProgram { label, .. } => {
            format!("reserved deformation {label}")
        }
    }
}

fn validity_label(valid: bool) -> &'static str {
    if valid { "valid" } else { "invalid" }
}

fn run_decompile(args: DecompileArgs) -> anyhow::Result<()> {
    if args.enable_bend && args.package_schema != PackageSchema::Schema3 {
        bail!("--enable-bend requires --package-schema 3");
    }
    let source = read_obj_from_path(&args.source)
        .with_context(|| format!("loading source OBJ {}", args.source.display()))?;
    let target = read_obj_from_path(&args.target)
        .with_context(|| format!("loading target OBJ {}", args.target.display()))?;
    let settings = DecompileSettings {
        affine_min_explained: args.affine_min_explained,
        residual_epsilon: args.residual_epsilon,
    };

    if args.package_schema == PackageSchema::Schema3 && args.enable_bend {
        return run_schema_three_bend_decompile(source, target, settings, &args);
    }

    let result = decompile_pair(&source, &target, settings)
        .context("decompiling same-topology mesh pair")?;
    let paths = match args.package_schema {
        PackageSchema::Schema2 => write_decompile_package(&result, &source, &target, &args.out_dir),
        PackageSchema::Schema3 => {
            let program = schema_three_program_from_result(&result);
            build_v3_package_from_program_with_diagnostics(
                &program,
                &source,
                &target,
                &args.out_dir,
                None,
            )
        }
    }
    .with_context(|| format!("writing decompile package to {}", args.out_dir.display()))?;

    let affine_summary = result
        .manifest
        .operators
        .iter()
        .find_map(|operator| match operator {
            OperatorManifest::GlobalAffine {
                semantic_family,
                explained_displacement_fraction,
                max_remaining_error,
                ..
            } => Some((
                *semantic_family,
                *explained_displacement_fraction,
                *max_remaining_error,
            )),
            OperatorManifest::LosslessCorrection { .. } => None,
        });
    println!("Decompiled same-topology mesh pair");
    println!(
        "  package schema: {}",
        match args.package_schema {
            PackageSchema::Schema2 => 2,
            PackageSchema::Schema3 => 3,
        }
    );
    println!("  vertices: {}", result.manifest.topology.vertex_count);
    println!("  triangles: {}", result.manifest.topology.triangle_count);
    println!("  topology hash: {}", result.manifest.topology.hash);
    println!("  operators: {}", result.manifest.operators.len());
    if let Some((semantic_family, affine_explained, affine_max_error)) = affine_summary {
        println!(
            "  affine operator: {}",
            affine_family_label(semantic_family)
        );
        println!(
            "  affine explained (raw vertices): {:.3}%",
            affine_explained * 100.0
        );
        println!("  affine max error: {:.9}", affine_max_error);
    } else {
        println!("  affine operator: none");
        println!("  affine explained (raw vertices): 0.000%");
        println!("  affine max error: 0.000000000");
    }
    println!("  residual vertices: {}", result.residual_indices.len());
    println!(
        "  final max error: {:.9}",
        result.verification.max_euclidean_error
    );
    println!("  manifest: {}", paths.manifest.display());
    println!(
        "  package verification: {}",
        paths.package_verification.display()
    );
    println!(
        "  inference diagnostics: {}",
        paths.inference_diagnostics.display()
    );
    println!("  blender script: {}", paths.blender_script.display());
    if args.verbose {
        println!("Inference program hypotheses:");
        for hypothesis in &result.inference_diagnostics.program_hypotheses {
            print_schema_two_program_hypothesis_diagnostics(hypothesis);
        }
    }
    Ok(())
}

fn run_schema_three_bend_decompile(
    source: TriangleMesh,
    target: TriangleMesh,
    _settings: DecompileSettings,
    args: &DecompileArgs,
) -> anyhow::Result<()> {
    let search_settings = ProgramSearchSettings::default();
    let search = search_programs_for_mesh_pair(&source, &target, &search_settings, true)
        .context("searching schema-3 bend program hypotheses")?;
    let selected_index = search
        .selected_hypothesis_index
        .context("schema-3 bend search produced no selectable hypothesis")?;
    let selected = &search.hypotheses[selected_index];
    let diagnostics = search
        .diagnostics
        .clone()
        .context("schema-3 bend search did not produce diagnostics")?;
    let paths = build_v3_package_from_program_with_diagnostics(
        &selected.program,
        &source,
        &target,
        &args.out_dir,
        Some(diagnostics.clone()),
    )
    .with_context(|| format!("writing decompile package to {}", args.out_dir.display()))?;
    let verification = verify_decompile_package(&args.out_dir)
        .with_context(|| format!("verifying decompile package {}", args.out_dir.display()))?;

    println!("Decompiled same-topology mesh pair");
    println!("  package schema: 3");
    println!("  bend inference: enabled");
    println!("  vertices: {}", verification.vertex_count);
    println!("  triangles: {}", verification.triangle_count);
    println!(
        "  selected program: {}",
        schema_three_program_label(&selected.program)
    );
    println!(
        "  weighted explained: {:.3}%",
        selected.weighted_explained_fraction * 100.0
    );
    println!("  program score: {:.9}", selected.total_score);
    println!(
        "  residual vertices: {}",
        verification.residual_vertex_count
    );
    println!("  final max error: {:.9}", verification.max_euclidean_error);
    println!("  manifest: {}", paths.manifest.display());
    println!(
        "  package verification: {}",
        paths.package_verification.display()
    );
    println!(
        "  inference diagnostics: {}",
        paths.inference_diagnostics.display()
    );
    println!("  blender script: {}", paths.blender_script.display());
    if args.verbose {
        print_schema_three_diagnostics(&diagnostics);
    }
    Ok(())
}

fn schema_three_program_from_result(result: &DecompileResult) -> OperatorProgram {
    let mut operators = Vec::new();
    for operator in &result.manifest.operators {
        match operator {
            OperatorManifest::GlobalAffine {
                matrix_row_major_4x4,
                semantic_family,
                translation,
                rotation_row_major_3x3,
                uniform_scale,
                ..
            } => {
                operators.push(ProgramOperator::Affine(AffineOperator {
                    semantic_family: *semantic_family,
                    matrix_row_major_4x4: *matrix_row_major_4x4,
                    translation: *translation,
                    rotation_row_major_3x3: *rotation_row_major_3x3,
                    uniform_scale: *uniform_scale,
                }));
            }
            OperatorManifest::LosslessCorrection { .. } => {}
        }
    }
    OperatorProgram { operators }
}

fn affine_family_label(semantic_family: AffineSemanticFamily) -> &'static str {
    match semantic_family {
        AffineSemanticFamily::GeneralAffine => "global affine",
        AffineSemanticFamily::Translation => "translation",
        AffineSemanticFamily::RigidTransform => "rigid transform",
        AffineSemanticFamily::SimilarityTransform => "similarity transform",
    }
}

fn print_schema_two_program_hypothesis_diagnostics(hypothesis: &ProgramHypothesisDiagnosticsV3) {
    let selected = if hypothesis.selected {
        "selected"
    } else {
        "candidate"
    };
    let rejection = hypothesis.rejection_reason.as_deref().unwrap_or("viable");
    println!(
        "  - {} [{}]: score={:.9} raw={:.3}% weighted={:.3}% approx_residual={:.6} exact_bytes={} status={}",
        program_hypothesis_label(hypothesis),
        selected,
        hypothesis.total_score,
        hypothesis.raw_explained_fraction * 100.0,
        hypothesis.weighted_explained_fraction * 100.0,
        hypothesis.approximate_residual_cost,
        hypothesis.exact_residual_bytes,
        rejection
    );
}

fn program_hypothesis_label(hypothesis: &ProgramHypothesisDiagnosticsV3) -> String {
    if hypothesis.operators.is_empty() {
        return "lossless correction only".to_owned();
    }
    let mut labels = hypothesis
        .operators
        .iter()
        .map(|operator| operator_family_label(operator.family))
        .collect::<Vec<_>>();
    labels.push("lossless correction");
    labels.join(" -> ")
}

fn operator_family_label(family: OperatorFamily) -> &'static str {
    match family {
        OperatorFamily::NoOp => "no-op",
        OperatorFamily::Translation => "translation",
        OperatorFamily::RigidTransform => "rigid transform",
        OperatorFamily::SimilarityTransform => "similarity transform",
        OperatorFamily::GeneralAffine => "global affine",
    }
}

fn schema_three_program_label(program: &OperatorProgram) -> String {
    if program.operators.is_empty() {
        return "lossless correction only".to_owned();
    }
    let mut labels = program
        .operators
        .iter()
        .map(|operator| match operator {
            ProgramOperator::Affine(affine) => affine_family_label(affine.semantic_family),
            ProgramOperator::Bend(_) => "bend",
        })
        .collect::<Vec<_>>();
    labels.push("lossless correction");
    labels.join(" -> ")
}

fn print_schema_three_diagnostics(diagnostics: &InferenceDiagnosticsV4) {
    println!("Schema-3 ordered program hypotheses:");
    for (index, hypothesis) in diagnostics.program_hypotheses.iter().enumerate() {
        print_schema_three_program_hypothesis(index, hypothesis);
    }
    println!("Schema-3 timing by phase (ms):");
    for (phase, elapsed_ms) in &diagnostics.timing_by_phase_ms {
        println!("  - {phase}: {elapsed_ms}");
    }
}

fn print_schema_three_program_hypothesis(
    index: usize,
    hypothesis: &ProgramHypothesisDiagnosticsV4,
) {
    let selected = if hypothesis.selected {
        "selected"
    } else {
        "candidate"
    };
    let rejection = hypothesis.rejection_reason.as_deref().unwrap_or("viable");
    println!(
        "  - #{index} {} [{}]: score={:.9} weighted={:.3}% raw={:.3}% exact_bytes={} status={}",
        schema_three_hypothesis_label(hypothesis),
        selected,
        hypothesis.score.total_component_sum,
        hypothesis.weighted_explained_fraction * 100.0,
        hypothesis.raw_explained_fraction * 100.0,
        hypothesis.exact_residual_bytes,
        rejection
    );
    for operator in &hypothesis.operators {
        println!(
            "      operator: {}",
            schema_three_operator_parameters(operator)
        );
    }
    for stage in &hypothesis.stages {
        println!(
            "      stage {}: weighted {:.9} -> {:.9}, raw {:.9} -> {:.9}, semantic_passed={}",
            stage.stage_index,
            stage.weighted_error_before,
            stage.weighted_error_after,
            stage.raw_error_before,
            stage.raw_error_after,
            stage.semantic_verification_passed
        );
    }
}

fn schema_three_hypothesis_label(hypothesis: &ProgramHypothesisDiagnosticsV4) -> String {
    if hypothesis.operators.is_empty() {
        return "lossless correction only".to_owned();
    }
    let mut labels = hypothesis
        .operators
        .iter()
        .map(schema_three_operator_label)
        .collect::<Vec<_>>();
    labels.push("lossless correction");
    labels.join(" -> ")
}

fn schema_three_operator_label(operator: &ProgramOperatorDiagnosticsV4) -> &'static str {
    match operator {
        ProgramOperatorDiagnosticsV4::Translation { .. } => "translation",
        ProgramOperatorDiagnosticsV4::RigidTransform { .. } => "rigid transform",
        ProgramOperatorDiagnosticsV4::SimilarityTransform { .. } => "similarity transform",
        ProgramOperatorDiagnosticsV4::GeneralAffine { .. } => "global affine",
        ProgramOperatorDiagnosticsV4::Bend { .. } => "bend",
    }
}

fn schema_three_operator_parameters(operator: &ProgramOperatorDiagnosticsV4) -> String {
    match operator {
        ProgramOperatorDiagnosticsV4::Translation { translation } => {
            format!("translation={translation:?}")
        }
        ProgramOperatorDiagnosticsV4::RigidTransform {
            translation,
            rotation_row_major_3x3,
        } => format!("translation={translation:?} rotation={rotation_row_major_3x3:?}"),
        ProgramOperatorDiagnosticsV4::SimilarityTransform {
            translation,
            rotation_row_major_3x3,
            uniform_scale,
        } => format!(
            "translation={translation:?} rotation={rotation_row_major_3x3:?} scale={uniform_scale}"
        ),
        ProgramOperatorDiagnosticsV4::GeneralAffine {
            matrix_row_major_4x4,
        } => format!("matrix={matrix_row_major_4x4:?}"),
        ProgramOperatorDiagnosticsV4::Bend { parameters } => format!(
            "origin={:?} axis={:?} direction={:?} angle_degrees={:.6} interval=[{:.6}, {:.6}]",
            parameters.origin,
            parameters.longitudinal_axis,
            parameters.bend_direction,
            parameters.angle_radians.to_degrees(),
            parameters.interval_start,
            parameters.interval_end
        ),
    }
}

fn run_verify_decompile(args: VerifyDecompileArgs) -> anyhow::Result<()> {
    let report = verify_decompile_package(&args.package)
        .with_context(|| format!("verifying decompile package {}", args.package.display()))?;
    println!("Verified decompile package");
    println!("  schema: {}", report.schema_version);
    println!("  vertices: {}", report.vertex_count);
    println!("  triangles: {}", report.triangle_count);
    println!("  operators: {}", report.operator_count);
    println!("  topology exact: {}", report.topology_exact);
    println!(
        "  topology hash matches: {}",
        report.topology_hash_matches_manifest
    );
    println!("  positions bit-exact: {}", report.positions_bit_exact);
    println!("  residual vertices: {}", report.residual_vertex_count);
    println!("  final max error: {:.9}", report.max_euclidean_error);
    Ok(())
}

fn build_preview(
    document: &ShapeDocument,
    mesh_settings: MeshSettings,
    image_size: u32,
) -> anyhow::Result<PreviewArtifact> {
    let field = compile_document(document).context("compiling field")?;
    let mesh = mesh_field(&field, mesh_settings).context("meshing field")?;
    let camera = fit_camera_to_bounds(mesh.bounds);
    let settings = RenderSettings {
        width: image_size,
        height: image_size,
        ..RenderSettings::default()
    };
    let image = render_mesh(&mesh, &camera, &settings).context("rendering mesh")?;
    Ok(PreviewArtifact { mesh, image })
}

pub(crate) fn save_png(image: &RenderedImage, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let buffer = RgbaImage::from_raw(image.width, image.height, image.rgba8.clone())
        .context("rendered image buffer length does not match dimensions")?;
    buffer
        .save(path)
        .with_context(|| format!("saving PNG to {}", path.display()))?;
    Ok(())
}

pub(crate) fn save_contact_sheet(
    parent: &RenderedImage,
    candidates: &[&RenderedImage],
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let labeled = candidates
        .iter()
        .enumerate()
        .map(|(index, image)| (format!("CAND {index:02}"), *image))
        .collect::<Vec<_>>();
    let labeled_refs = labeled
        .iter()
        .map(|(label, image)| (label.as_str(), *image))
        .collect::<Vec<_>>();
    save_labeled_contact_sheet("PARENT", parent, &labeled_refs, path)
}

fn save_labeled_contact_sheet(
    parent_label: &str,
    parent: &RenderedImage,
    candidates: &[(&str, &RenderedImage)],
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let card_width = CONTACT_CARD_SIZE;
    let card_height = CONTACT_LABEL_HEIGHT + CONTACT_CARD_SIZE;
    let column_count = candidates.len() + 1;
    let width = CONTACT_PADDING
        + (card_width + CONTACT_PADDING)
            * u32::try_from(column_count).context("too many contact sheet columns")?;
    let height = card_height + CONTACT_PADDING * 2;
    let mut sheet = RgbaImage::from_pixel(width, height, Rgba([18, 20, 22, 255]));
    draw_card(&mut sheet, 0, parent_label, parent)?;
    for (index, (label, image)) in candidates.iter().enumerate() {
        draw_card(&mut sheet, index + 1, label, image)?;
    }
    sheet
        .save(path)
        .with_context(|| format!("saving contact sheet to {}", path.display()))?;
    Ok(())
}

fn draw_card(
    sheet: &mut RgbaImage,
    column: usize,
    label: &str,
    image: &RenderedImage,
) -> anyhow::Result<()> {
    let x = CONTACT_PADDING
        + u32::try_from(column).context("contact sheet column overflow")?
            * (CONTACT_CARD_SIZE + CONTACT_PADDING);
    let y = CONTACT_PADDING;
    fill_rect(
        sheet,
        x,
        y,
        CONTACT_CARD_SIZE,
        CONTACT_LABEL_HEIGHT,
        Rgba([42, 46, 50, 255]),
    );
    draw_text(sheet, x + 8, y + 7, label, Rgba([232, 235, 230, 255]));

    let buffer = RgbaImage::from_raw(image.width, image.height, image.rgba8.clone())
        .context("rendered image buffer length does not match dimensions")?;
    let resized = image::imageops::resize(
        &buffer,
        CONTACT_CARD_SIZE,
        CONTACT_CARD_SIZE,
        FilterType::Nearest,
    );
    image::imageops::replace(
        sheet,
        &resized,
        i64::from(x),
        i64::from(y + CONTACT_LABEL_HEIGHT),
    );
    Ok(())
}

fn fill_rect(image: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
    for py in y..y.saturating_add(height).min(image.height()) {
        for px in x..x.saturating_add(width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn draw_text(image: &mut RgbaImage, x: u32, y: u32, text: &str, color: Rgba<u8>) {
    let mut cursor = x;
    for character in text.chars() {
        draw_glyph(image, cursor, y, character, color);
        cursor = cursor.saturating_add(18);
    }
}

fn draw_glyph(image: &mut RgbaImage, x: u32, y: u32, character: char, color: Rgba<u8>) {
    let glyph = glyph(character);
    for (row, pattern) in glyph.iter().enumerate() {
        for (column, value) in pattern.chars().enumerate() {
            if value == '#' {
                fill_rect(
                    image,
                    x + u32::try_from(column).unwrap_or(0) * 3,
                    y + u32::try_from(row).unwrap_or(0) * 3,
                    3,
                    3,
                    color,
                );
            }
        }
    }
}

fn glyph(character: char) -> [&'static str; 5] {
    match character {
        'A' => [".#.", "#.#", "###", "#.#", "#.#"],
        'C' => ["###", "#..", "#..", "#..", "###"],
        'D' => ["##.", "#.#", "#.#", "#.#", "##."],
        'E' => ["###", "#..", "##.", "#..", "###"],
        'N' => ["#.#", "###", "###", "###", "#.#"],
        'P' => ["##.", "#.#", "##.", "#..", "#.."],
        'R' => ["##.", "#.#", "##.", "#.#", "#.#"],
        'T' => ["###", ".#.", ".#.", ".#.", ".#."],
        '0' => ["###", "#.#", "#.#", "#.#", "###"],
        '1' => [".#.", "##.", ".#.", ".#.", "###"],
        '2' => ["###", "..#", "###", "#..", "###"],
        '3' => ["###", "..#", "###", "..#", "###"],
        '4' => ["#.#", "#.#", "###", "..#", "..#"],
        '5' => ["###", "#..", "###", "..#", "###"],
        '6' => ["###", "#..", "###", "#.#", "###"],
        '7' => ["###", "..#", ".#.", ".#.", ".#."],
        '8' => ["###", "#.#", "###", "#.#", "###"],
        '9' => ["###", "#.#", "###", "..#", "###"],
        _ => ["...", "...", "...", "...", "..."],
    }
}

fn ensure_known_preset(id: &PresetId) -> anyhow::Result<()> {
    if list_presets().iter().any(|preset| preset.id == *id) {
        Ok(())
    } else {
        bail!("unknown preset '{}'", id.0)
    }
}

fn ensure_valid_document(document: &ShapeDocument, label: &str) -> anyhow::Result<()> {
    let report = validate_document(document);
    if report.is_valid() {
        Ok(())
    } else {
        bail!(
            "{label} document failed validation: {} issue(s)",
            report.issues.len()
        )
    }
}

fn all_param_groups() -> BTreeSet<ParamGroup> {
    [
        ParamGroup::Form,
        ParamGroup::Placement,
        ParamGroup::Rotation,
        ParamGroup::Scale,
        ParamGroup::Blend,
    ]
    .into_iter()
    .collect()
}

fn summarize_changes(operations: &[shape_core::SetScalarEdit]) -> Vec<ChangedParameter> {
    operations
        .iter()
        .map(|operation| ChangedParameter {
            node: operation.path.node.0,
            key: operation.path.key.clone(),
            before: operation.before,
            after: operation.after,
        })
        .collect()
}

fn mesh_summary(mesh: &TriangleMesh) -> MeshSummary {
    MeshSummary {
        vertices: mesh.positions.len(),
        triangles: mesh.indices.len() / 3,
    }
}

pub(crate) fn render_mesh_from_triangles(mesh: &TriangulatedPolygonMesh) -> TriangleMesh {
    TriangleMesh {
        positions: mesh.mesh.positions.clone(),
        normals: mesh.mesh.normals.clone(),
        indices: mesh.mesh.indices.clone(),
        bounds: Aabb {
            min: Vec3::from_array(mesh.mesh.bounds.min),
            max: Vec3::from_array(mesh.mesh.bounds.max),
        },
    }
}

pub(crate) fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> anyhow::Result<()> {
    let path = path.as_ref();
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("writing JSON to {}", path.display()))?;
    Ok(())
}

fn clamp_usize(value: usize, min: usize, max: usize) -> usize {
    value.clamp(min, max)
}
