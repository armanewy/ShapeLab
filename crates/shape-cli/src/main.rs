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
use shape_mesh::{MeshSettings, TriangleMesh, mesh_field, read_obj_from_path, write_obj_to_path};
use shape_modeling_assets::{BenchmarkAsset, benchmark_assets};
use shape_poly::{EdgeKey, PolygonMesh, TriangulatedPolygonMesh};
use shape_presets::{PresetId, build_preset, list_presets};
use shape_project::Project;
use shape_render::{
    MeshVisualDescriptor, RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh,
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
use shape_search::{ExplorationMode, SearchRequest, TargetScope, generate_candidates};

const DEFAULT_PRESET: &str = "desk-lamp";
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
    #[arg(long, default_value_t = 96)]
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
        Some(BenchmarkAsset::IndustrialCrate) => 30_000,
        Some(BenchmarkAsset::MultiCutPanel) => 18_000,
        Some(BenchmarkAsset::ExplicitDeskLamp) => 25_000,
        Some(BenchmarkAsset::StylizedStool) => 20_000,
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

fn save_png(image: &RenderedImage, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let buffer = RgbaImage::from_raw(image.width, image.height, image.rgba8.clone())
        .context("rendered image buffer length does not match dimensions")?;
    buffer
        .save(path)
        .with_context(|| format!("saving PNG to {}", path.display()))?;
    Ok(())
}

fn save_contact_sheet(
    parent: &RenderedImage,
    candidates: &[&RenderedImage],
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
    draw_card(&mut sheet, 0, "PARENT", parent)?;
    for (index, image) in candidates.iter().enumerate() {
        draw_card(&mut sheet, index + 1, &format!("CAND {index:02}"), image)?;
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

fn render_mesh_from_triangles(mesh: &TriangulatedPolygonMesh) -> TriangleMesh {
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

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> anyhow::Result<()> {
    let path = path.as_ref();
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("writing JSON to {}", path.display()))?;
    Ok(())
}

fn clamp_usize(value: usize, min: usize, max: usize) -> usize {
    value.clamp(min, max)
}
