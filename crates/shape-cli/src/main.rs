#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::{Parser, Subcommand, ValueEnum};
use image::{Rgba, RgbaImage, imageops::FilterType};
use serde::Serialize;
use shape_core::{ParamGroup, ShapeDocument, validate_document};
use shape_decompiler::{
    AffineSemanticFamily, DecompileSettings, OperatorFamily, OperatorManifest,
    ProgramHypothesisDiagnostics, decompile_pair, verify_decompile_package,
    write_decompile_package,
};
use shape_field::compile_document;
use shape_mesh::{MeshSettings, TriangleMesh, mesh_field, read_obj_from_path, write_obj_to_path};
use shape_presets::{PresetId, build_preset, list_presets};
use shape_project::Project;
use shape_render::{RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh};
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

fn run_decompile(args: DecompileArgs) -> anyhow::Result<()> {
    let source = read_obj_from_path(&args.source)
        .with_context(|| format!("loading source OBJ {}", args.source.display()))?;
    let target = read_obj_from_path(&args.target)
        .with_context(|| format!("loading target OBJ {}", args.target.display()))?;
    let settings = DecompileSettings {
        affine_min_explained: args.affine_min_explained,
        residual_epsilon: args.residual_epsilon,
    };
    let result = decompile_pair(&source, &target, settings)
        .context("decompiling same-topology mesh pair")?;
    let paths = write_decompile_package(&result, &source, &target, &args.out_dir)
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
            print_program_hypothesis_diagnostics(hypothesis);
        }
    }
    Ok(())
}

fn affine_family_label(semantic_family: AffineSemanticFamily) -> &'static str {
    match semantic_family {
        AffineSemanticFamily::GeneralAffine => "global affine",
        AffineSemanticFamily::Translation => "translation",
        AffineSemanticFamily::RigidTransform => "rigid transform",
        AffineSemanticFamily::SimilarityTransform => "similarity transform",
    }
}

fn print_program_hypothesis_diagnostics(hypothesis: &ProgramHypothesisDiagnostics) {
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

fn program_hypothesis_label(hypothesis: &ProgramHypothesisDiagnostics) -> String {
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

fn write_json(path: impl AsRef<Path>, value: &impl Serialize) -> anyhow::Result<()> {
    let path = path.as_ref();
    fs::write(path, serde_json::to_string_pretty(value)?)
        .with_context(|| format!("writing JSON to {}", path.display()))?;
    Ok(())
}

fn clamp_usize(value: usize, min: usize, max: usize) -> usize {
    value.clamp(min, max)
}
