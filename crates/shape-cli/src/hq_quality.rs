use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::ValueEnum;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use shape_compile::export::{verify_model_package, write_model_package};
use shape_compile::validation::{
    ModelValidationReport, ValidationLimits, validate_model,
    validation_config_from_recipe_with_limits,
};
use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlValue, CustomizerControl,
    FoundryCompilationOutput, compile_foundry_document, default_control_value,
    effective_control_domain,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels};
use shape_render::{
    RenderSettings, RenderedImage, fit_camera_to_bounds_from_angles, render_mesh,
    visual_descriptor_for_mesh,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateRequest, generate_foundry_candidate_plans,
};

use crate::{render_mesh_from_triangles, save_contact_sheet, save_png, write_json};

pub const HQ_QUALITY_REPORT_SCHEMA_VERSION: u32 = 1;
const DEFAULT_QUALITY_SEED: u64 = 42;
const DEFAULT_TRIANGLE_BUDGET: u64 = 80_000;
const DEFAULT_DIRECTION_COUNT: usize = 6;
const DEFAULT_PROPOSAL_COUNT: usize = 72;
const HQ_IMAGE_SIZE: u32 = 512;

#[derive(Debug, clap::Args)]
pub struct HqQualityBenchmarkArgs {
    /// Built-in Visual Foundry profile slug, or `all`.
    #[arg(long)]
    profile: String,
    /// Output directory. With `--profile all`, profile subdirectories are written under this path.
    #[arg(long)]
    out_dir: PathBuf,
    /// Highest quality tier requested by this run.
    #[arg(long, value_enum, default_value_t = HqQualityTier::Usable)]
    quality_tier: HqQualityTier,
    /// Verify deterministic package export and package reopen checks.
    #[arg(long)]
    verify_export: bool,
    /// Mark the run as manually approved by a reviewer.
    #[arg(long)]
    human_approved: bool,
    /// Mark the run as having passed adversarial visual review.
    #[arg(long)]
    adversarial_reviewed: bool,
    /// Optional path to human review notes.
    #[arg(long)]
    manual_notes: Option<PathBuf>,
    /// Print the generated quality report JSON to stdout.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HqQualityTier {
    Draft,
    Prototype,
    Usable,
    Showcase,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HqEvidenceStatus {
    Available,
    Verified,
    NotRun,
    Unsupported,
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HqHumanApprovalStatus {
    Pending,
    Approved,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqQualityReport {
    pub schema_version: u32,
    pub generated_at: String,
    pub deterministic_timestamp_policy: String,
    pub profile_id: String,
    pub profile_label: String,
    pub kit_id: Option<String>,
    pub style_id: Option<String>,
    pub quality_tier_requested: HqQualityTier,
    pub quality_tier_achieved: HqQualityTier,
    pub quality_tier_blockers: Vec<String>,
    pub clay_preview_available: bool,
    pub contact_sheet_available: bool,
    pub front_view_available: bool,
    pub three_quarter_view_available: bool,
    pub side_view_available: bool,
    pub back_view_available: bool,
    pub wireframe_available: bool,
    pub silhouette_available: bool,
    pub silhouette_readability_metric: Option<f32>,
    pub silhouette_manual_review_required: bool,
    pub mesh_validity_summary: HqMeshValiditySummary,
    pub triangle_count: u64,
    pub triangle_budget: Option<u64>,
    pub semantic_part_inventory: HqSemanticPartInventory,
    pub required_role_coverage: HqRequiredRoleCoverage,
    pub provider_attachment_validity: HqProviderAttachmentValidity,
    pub candidate_survival_count: usize,
    pub six_direction_availability: bool,
    pub primary_control_count: usize,
    pub visible_control_difference_evidence: HqVisibleControlDifferenceEvidence,
    pub advanced_recipe_required: bool,
    pub export_status: HqEvidenceStatus,
    pub reopen_status: HqEvidenceStatus,
    pub unsupported_outputs: Vec<HqUnsupportedOutput>,
    pub placeholder_thumbnail_detected: bool,
    pub human_review_required: bool,
    pub human_approval_status: HqHumanApprovalStatus,
    pub adversarial_review_status: HqHumanApprovalStatus,
    pub manual_notes_path: Option<String>,
    pub novice_catalog_exposure_allowed_by_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqMeshValiditySummary {
    pub compile_valid: bool,
    pub model_valid: bool,
    pub issue_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub provenance_coverage: f32,
    pub manifold_closed_part_fraction: f32,
    pub accidental_intersection_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqSemanticPartInventory {
    pub part_count: u64,
    pub parts: Vec<HqSemanticPartRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqSemanticPartRow {
    pub instance_id: u64,
    pub definition_id: u64,
    pub name: String,
    pub source_recipe_instance: bool,
    pub generated_by: Option<u64>,
    pub triangle_count: usize,
    pub region_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqRequiredRoleCoverage {
    pub required_role_count: usize,
    pub covered_required_role_count: usize,
    pub missing_required_roles: Vec<String>,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqProviderAttachmentValidity {
    pub provider_override_count: usize,
    pub final_conformance_accepted: bool,
    pub required_attachment_issue_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqVisibleControlDifferenceEvidence {
    pub primary_controls_checked: usize,
    pub changed_control_count: usize,
    pub controls: Vec<HqControlDifferenceRow>,
}

impl HqVisibleControlDifferenceEvidence {
    #[must_use]
    pub fn all_primary_controls_changed(&self) -> bool {
        self.primary_controls_checked > 0
            && self.changed_control_count == self.primary_controls_checked
            && self.controls.iter().all(|control| control.changed_geometry)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqControlDifferenceRow {
    pub control_id: String,
    pub label: String,
    pub control_kind: String,
    pub current_value: String,
    pub sampled_value: Option<String>,
    pub changed_geometry: bool,
    pub visual_delta_from_parent: u64,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqUnsupportedOutput {
    pub output: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqMeshStatsFile {
    pub profile_id: String,
    pub part_count: u64,
    pub polygon_vertex_count: u64,
    pub polygon_face_count: u64,
    pub triangle_count: u64,
    pub triangle_budget: Option<u64>,
    pub used_sdf_or_remeshing: bool,
    pub model_validation: HqMeshValiditySummary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqCandidateReport {
    pub mode: String,
    pub seed: u64,
    pub proposal_count: usize,
    pub requested_count: usize,
    pub returned_count: usize,
    pub candidate_survival_count: usize,
    pub six_direction_availability: bool,
    pub failure: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqExportReopenReport {
    pub export_status: HqEvidenceStatus,
    pub reopen_status: HqEvidenceStatus,
    pub package_dir: Option<String>,
    pub manifest: Option<String>,
    pub checksums_match: Option<bool>,
    pub topology_matches_manifest: Option<bool>,
    pub finite_numeric_payloads: Option<bool>,
    pub not_run_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HqQualityGateEvidence {
    pub compile_valid: bool,
    pub clay_preview_available: bool,
    pub contact_sheet_available: bool,
    pub front_view_available: bool,
    pub three_quarter_view_available: bool,
    pub side_view_available: bool,
    pub back_view_available: bool,
    pub wireframe_available: bool,
    pub silhouette_available: bool,
    pub visible_control_difference_evidence: bool,
    pub advanced_recipe_required: bool,
    pub export_verified: bool,
    pub reopen_verified: bool,
    pub candidate_survival_count: usize,
    pub placeholder_thumbnails: bool,
    pub human_approved: bool,
    pub adversarial_reviewed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HqQualityTierDecision {
    pub achieved: HqQualityTier,
    pub blockers: Vec<String>,
}

pub fn run_hq_quality_benchmark(args: HqQualityBenchmarkArgs) -> anyhow::Result<()> {
    let profiles = resolve_benchmark_profiles(&args.profile)?;
    let multi_profile = profiles.len() > 1;
    let mut reports = Vec::with_capacity(profiles.len());
    for (label, fixture) in profiles {
        let out_dir = if multi_profile {
            args.out_dir.join(&fixture.slug)
        } else {
            args.out_dir.clone()
        };
        fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
        let report = benchmark_one_profile(&args, label, fixture, &out_dir)?;
        reports.push(report);
    }

    if args.json {
        if reports.len() == 1 {
            println!("{}", serde_json::to_string_pretty(&reports[0])?);
        } else {
            println!("{}", serde_json::to_string_pretty(&reports)?);
        }
    } else {
        for report in &reports {
            println!(
                "HQ quality benchmark {}: requested={:?} achieved={:?} blockers={}",
                report.profile_id,
                report.quality_tier_requested,
                report.quality_tier_achieved,
                report.quality_tier_blockers.len()
            );
        }
    }

    Ok(())
}

#[must_use]
pub fn benchmark_profile_slugs() -> Vec<String> {
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect()
}

#[must_use]
pub fn novice_catalog_exposure_allowed(
    tier: HqQualityTier,
    prototype_opt_in: bool,
    human_review_approved: bool,
) -> bool {
    match tier {
        HqQualityTier::Draft => false,
        HqQualityTier::Prototype => prototype_opt_in,
        HqQualityTier::Usable | HqQualityTier::Showcase => human_review_approved,
    }
}

#[must_use]
pub fn evaluate_quality_tier(
    requested: HqQualityTier,
    evidence: &HqQualityGateEvidence,
) -> HqQualityTierDecision {
    let mut blockers = Vec::new();
    let prototype_ok = evidence.compile_valid && evidence.clay_preview_available;
    if !evidence.compile_valid {
        blockers.push("profile must compile before it can exceed Draft".to_owned());
    }
    if !evidence.clay_preview_available {
        blockers.push("at least one clay preview is required before Prototype".to_owned());
    }

    let usable_ok = prototype_ok
        && evidence.contact_sheet_available
        && evidence.front_view_available
        && evidence.three_quarter_view_available
        && evidence.side_view_available
        && evidence.back_view_available
        && evidence.visible_control_difference_evidence
        && !evidence.advanced_recipe_required
        && evidence.export_verified
        && evidence.reopen_verified
        && evidence.candidate_survival_count >= DEFAULT_DIRECTION_COUNT
        && !evidence.placeholder_thumbnails;

    if requested >= HqQualityTier::Usable {
        if !evidence.contact_sheet_available {
            blockers.push("Usable requires contact sheet evidence".to_owned());
        }
        if !evidence.front_view_available
            || !evidence.three_quarter_view_available
            || !evidence.side_view_available
            || !evidence.back_view_available
        {
            blockers.push("Usable requires front, three-quarter, side, and back views".to_owned());
        }
        if !evidence.visible_control_difference_evidence {
            blockers.push("Usable requires visible primary-control difference evidence".to_owned());
        }
        if evidence.advanced_recipe_required {
            blockers.push("Usable cannot require Advanced Recipe for the novice task".to_owned());
        }
        if !evidence.export_verified || !evidence.reopen_verified {
            blockers.push("Usable requires export and reopen verification".to_owned());
        }
        if evidence.candidate_survival_count < DEFAULT_DIRECTION_COUNT {
            blockers.push(
                "Usable requires six surviving direction candidates or an approved exception"
                    .to_owned(),
            );
        }
        if evidence.placeholder_thumbnails {
            blockers.push("Usable cannot rely on placeholder thumbnails".to_owned());
        }
    }

    let showcase_ok = usable_ok
        && evidence.contact_sheet_available
        && evidence.wireframe_available
        && evidence.silhouette_available
        && evidence.human_approved
        && evidence.adversarial_reviewed;

    if requested >= HqQualityTier::Showcase {
        if !evidence.wireframe_available || !evidence.silhouette_available {
            blockers.push(
                "Showcase requires all required visual views, including wireframe and silhouette"
                    .to_owned(),
            );
        }
        if !evidence.human_approved {
            blockers.push("Showcase requires a human approval marker".to_owned());
        }
        if !evidence.adversarial_reviewed {
            blockers.push("Showcase requires adversarial visual review".to_owned());
        }
    }

    let strongest = if showcase_ok {
        HqQualityTier::Showcase
    } else if usable_ok {
        HqQualityTier::Usable
    } else if prototype_ok {
        HqQualityTier::Prototype
    } else {
        HqQualityTier::Draft
    };
    let achieved = strongest.min(requested);

    HqQualityTierDecision { achieved, blockers }
}

fn benchmark_one_profile(
    args: &HqQualityBenchmarkArgs,
    label: &'static str,
    fixture: FoundryFixtureCatalog,
    out_dir: &Path,
) -> anyhow::Result<HqQualityReport> {
    let output = compile_foundry_document(&fixture.document, &fixture).map_err(|error| {
        anyhow::anyhow!(
            "foundry profile {} failed to compile: {error:#?}",
            fixture.slug
        )
    })?;
    let model_validation = validate_compiled_output(&output);
    let mesh = render_mesh_from_triangles(&output.artifact.combined_preview);

    let front = render_hq_view(&mesh, 0.0, 14.0, false)?;
    let three_quarter = render_hq_view(&mesh, 42.0, 18.0, false)?;
    let side = render_hq_view(&mesh, 90.0, 14.0, false)?;
    let back = render_hq_view(&mesh, 180.0, 14.0, false)?;
    let wireframe = render_hq_view(&mesh, 42.0, 18.0, true)?;
    let silhouette = render_silhouette_view(&mesh, 42.0, 18.0)?;
    let placeholder_thumbnail_detected = [
        &front,
        &three_quarter,
        &side,
        &back,
        &wireframe,
        &silhouette,
    ]
    .into_iter()
    .any(is_placeholder_image);

    save_png(&front, out_dir.join("front.png"))?;
    save_png(&three_quarter, out_dir.join("three-quarter.png"))?;
    save_png(&side, out_dir.join("side.png"))?;
    save_png(&back, out_dir.join("back.png"))?;
    save_png(&wireframe, out_dir.join("wireframe.png"))?;
    save_png(&silhouette, out_dir.join("silhouette.png"))?;
    save_contact_sheet(
        &front,
        &[&three_quarter, &side, &back],
        out_dir.join("contact-sheet.png"),
    )?;

    let mesh_validity_summary = mesh_validity_summary(&output, &model_validation);
    let semantic_part_inventory = semantic_part_inventory(&output);
    let required_role_coverage = required_role_coverage(&output);
    let provider_attachment_validity = provider_attachment_validity(&output, &model_validation);
    let candidate_report = candidate_report(&fixture);
    let visible_control_difference_evidence =
        visible_control_difference_evidence(&fixture, &output)?;
    let export_reopen = export_reopen_report(args.verify_export, &output, out_dir)?;
    let silhouette_metric = silhouette_readability_metric(&mesh);
    let unsupported_outputs = unsupported_outputs();

    let gate_evidence = HqQualityGateEvidence {
        compile_valid: output.artifact.validation_report.is_valid() && model_validation.is_valid(),
        clay_preview_available: true,
        contact_sheet_available: true,
        front_view_available: true,
        three_quarter_view_available: true,
        side_view_available: true,
        back_view_available: true,
        wireframe_available: true,
        silhouette_available: true,
        visible_control_difference_evidence: visible_control_difference_evidence
            .all_primary_controls_changed(),
        advanced_recipe_required: false,
        export_verified: export_reopen.export_status == HqEvidenceStatus::Verified,
        reopen_verified: export_reopen.reopen_status == HqEvidenceStatus::Verified,
        candidate_survival_count: candidate_report.candidate_survival_count,
        placeholder_thumbnails: placeholder_thumbnail_detected,
        human_approved: args.human_approved,
        adversarial_reviewed: args.adversarial_reviewed,
    };
    let tier = evaluate_quality_tier(args.quality_tier, &gate_evidence);

    let mesh_stats = HqMeshStatsFile {
        profile_id: fixture.slug.clone(),
        part_count: output.artifact.statistics.part_count,
        polygon_vertex_count: output.artifact.statistics.polygon_vertex_count,
        polygon_face_count: output.artifact.statistics.polygon_face_count,
        triangle_count: output.artifact.statistics.triangle_count,
        triangle_budget: Some(DEFAULT_TRIANGLE_BUDGET),
        used_sdf_or_remeshing: output.artifact.statistics.used_sdf_or_remeshing,
        model_validation: mesh_validity_summary.clone(),
    };
    write_json(out_dir.join("mesh-stats.json"), &mesh_stats)?;
    write_json(
        out_dir.join("semantic-parts.json"),
        &semantic_part_inventory,
    )?;
    write_json(out_dir.join("candidate-report.json"), &candidate_report)?;
    write_json(
        out_dir.join("controls-visibility-report.json"),
        &visible_control_difference_evidence,
    )?;
    write_json(out_dir.join("export-reopen-report.json"), &export_reopen)?;

    let report = HqQualityReport {
        schema_version: HQ_QUALITY_REPORT_SCHEMA_VERSION,
        generated_at: "deterministic-no-wall-clock".to_owned(),
        deterministic_timestamp_policy:
            "quality reports intentionally omit wall-clock timestamps for reproducibility"
                .to_owned(),
        profile_id: fixture.slug,
        profile_label: label.to_owned(),
        kit_id: Some(output.catalog.style_kit.id.clone()),
        style_id: Some(output.final_conformance.style_kit_id.clone()),
        quality_tier_requested: args.quality_tier,
        quality_tier_achieved: tier.achieved,
        quality_tier_blockers: tier.blockers,
        clay_preview_available: true,
        contact_sheet_available: true,
        front_view_available: true,
        three_quarter_view_available: true,
        side_view_available: true,
        back_view_available: true,
        wireframe_available: true,
        silhouette_available: true,
        silhouette_readability_metric: silhouette_metric,
        silhouette_manual_review_required: true,
        mesh_validity_summary,
        triangle_count: output.artifact.statistics.triangle_count,
        triangle_budget: Some(DEFAULT_TRIANGLE_BUDGET),
        semantic_part_inventory,
        required_role_coverage,
        provider_attachment_validity,
        candidate_survival_count: candidate_report.candidate_survival_count,
        six_direction_availability: candidate_report.six_direction_availability,
        primary_control_count: visible_control_difference_evidence.primary_controls_checked,
        visible_control_difference_evidence,
        advanced_recipe_required: false,
        export_status: export_reopen.export_status,
        reopen_status: export_reopen.reopen_status,
        unsupported_outputs,
        placeholder_thumbnail_detected,
        human_review_required: true,
        human_approval_status: if args.human_approved {
            HqHumanApprovalStatus::Approved
        } else {
            HqHumanApprovalStatus::Pending
        },
        adversarial_review_status: if args.adversarial_reviewed {
            HqHumanApprovalStatus::Approved
        } else {
            HqHumanApprovalStatus::Pending
        },
        manual_notes_path: args
            .manual_notes
            .as_ref()
            .map(|path| path.display().to_string()),
        novice_catalog_exposure_allowed_by_default: novice_catalog_exposure_allowed(
            tier.achieved,
            false,
            args.human_approved,
        ),
    };
    write_json(out_dir.join("quality-report.json"), &report)?;

    Ok(report)
}

fn resolve_benchmark_profiles(
    profile: &str,
) -> anyhow::Result<Vec<(&'static str, FoundryFixtureCatalog)>> {
    let normalized = normalize_profile_slug(profile);
    let profiles = built_in_fixture_catalogs_with_labels();
    let known_slugs = benchmark_profile_slugs();
    if normalized == "all" {
        return Ok(profiles);
    }
    if !known_slugs
        .iter()
        .any(|slug| normalize_profile_slug(slug) == normalized)
    {
        bail_unknown_profile(profile, &known_slugs)?;
    }
    profiles
        .into_iter()
        .find(|(_, fixture)| normalize_profile_slug(&fixture.slug) == normalized)
        .map(|profile| vec![profile])
        .with_context(|| format!("unknown HQ quality benchmark profile '{profile}'"))
}

fn bail_unknown_profile(profile: &str, known_slugs: &[String]) -> anyhow::Result<()> {
    anyhow::bail!(
        "unknown HQ quality benchmark profile '{}'; expected one of: {}",
        profile,
        known_slugs.join(", ")
    )
}

fn normalize_profile_slug(profile: &str) -> String {
    match profile.trim().to_ascii_lowercase().as_str() {
        "scifi-crate" => "sci-fi-crate".to_owned(),
        "scifi-door" => "sci-fi-door".to_owned(),
        "storybook-tree" => "stylized-tree".to_owned(),
        other => other.to_owned(),
    }
}

fn validate_compiled_output(output: &FoundryCompilationOutput) -> ModelValidationReport {
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    validate_model(&output.artifact, &config)
}

fn render_hq_view(
    mesh: &shape_mesh::TriangleMesh,
    yaw_degrees: f32,
    pitch_degrees: f32,
    wireframe: bool,
) -> anyhow::Result<RenderedImage> {
    let camera = fit_camera_to_bounds_from_angles(mesh.bounds, yaw_degrees, pitch_degrees, 1.0);
    let settings = RenderSettings {
        width: HQ_IMAGE_SIZE,
        height: HQ_IMAGE_SIZE,
        wireframe,
        ..RenderSettings::default()
    };
    render_mesh(mesh, &camera, &settings).context("rendering HQ quality view")
}

fn render_silhouette_view(
    mesh: &shape_mesh::TriangleMesh,
    yaw_degrees: f32,
    pitch_degrees: f32,
) -> anyhow::Result<RenderedImage> {
    let camera = fit_camera_to_bounds_from_angles(mesh.bounds, yaw_degrees, pitch_degrees, 1.0);
    let settings = RenderSettings {
        width: HQ_IMAGE_SIZE,
        height: HQ_IMAGE_SIZE,
        background: [0, 0, 0, 0],
        ambient: 1.0,
        ..RenderSettings::default()
    };
    let source = render_mesh(mesh, &camera, &settings).context("rendering HQ silhouette source")?;
    let mut image = RgbaImage::from_pixel(source.width, source.height, Rgba([0, 0, 0, 255]));
    for y in 0..source.height {
        for x in 0..source.width {
            if let Some(pixel) = source.pixel(x, y)
                && pixel[3] > 0
            {
                image.put_pixel(x, y, Rgba([244, 246, 248, 255]));
            }
        }
    }
    Ok(RenderedImage {
        width: source.width,
        height: source.height,
        rgba8: image.into_raw(),
    })
}

fn silhouette_readability_metric(mesh: &shape_mesh::TriangleMesh) -> Option<f32> {
    let descriptor = visual_descriptor_for_mesh(mesh).ok()?;
    let average = descriptor.silhouette_occupancy.iter().copied().sum::<f32>()
        / descriptor.silhouette_occupancy.len() as f32;
    Some((average * 1000.0).round() / 1000.0)
}

fn mesh_validity_summary(
    output: &FoundryCompilationOutput,
    model_validation: &ModelValidationReport,
) -> HqMeshValiditySummary {
    let error_count = model_validation
        .issues
        .iter()
        .filter(|issue| issue.severity == shape_compile::validation::ValidationSeverity::Error)
        .count();
    let warning_count = model_validation
        .issues
        .iter()
        .filter(|issue| issue.severity == shape_compile::validation::ValidationSeverity::Warning)
        .count();
    HqMeshValiditySummary {
        compile_valid: output.artifact.validation_report.is_valid(),
        model_valid: model_validation.is_valid(),
        issue_count: model_validation.issues.len(),
        error_count,
        warning_count,
        provenance_coverage: model_validation.metrics.provenance_coverage,
        manifold_closed_part_fraction: model_validation.metrics.manifold_closed_part_fraction,
        accidental_intersection_count: model_validation.metrics.accidental_intersection_count,
    }
}

fn semantic_part_inventory(output: &FoundryCompilationOutput) -> HqSemanticPartInventory {
    let parts = output
        .artifact
        .compiled_parts
        .iter()
        .map(|part| {
            let region_count = output
                .recipe
                .definitions
                .get(&part.definition_id)
                .map_or(0, |definition| definition.regions.len());
            HqSemanticPartRow {
                instance_id: part.instance_id.0,
                definition_id: part.definition_id.0,
                name: part.instance_name.clone(),
                source_recipe_instance: part.source_recipe_instance,
                generated_by: part.generated_by.map(|operation| operation.0),
                triangle_count: part.triangulated_world.mesh.indices.len() / 3,
                region_count,
            }
        })
        .collect::<Vec<_>>();
    HqSemanticPartInventory {
        part_count: output.artifact.statistics.part_count,
        parts,
    }
}

fn required_role_coverage(output: &FoundryCompilationOutput) -> HqRequiredRoleCoverage {
    let missing_required_roles = output
        .final_conformance
        .roles
        .iter()
        .filter(|role| role.expected.min > 0 && role.status.rejects_required())
        .map(|role| role.role.clone())
        .collect::<Vec<_>>();
    let required_role_count = output
        .final_conformance
        .roles
        .iter()
        .filter(|role| role.expected.min > 0)
        .count();
    HqRequiredRoleCoverage {
        required_role_count,
        covered_required_role_count: required_role_count
            .saturating_sub(missing_required_roles.len()),
        missing_required_roles,
        accepted: output.final_conformance.is_accepted(),
    }
}

fn provider_attachment_validity(
    output: &FoundryCompilationOutput,
    model_validation: &ModelValidationReport,
) -> HqProviderAttachmentValidity {
    let required_attachment_issue_count = model_validation
        .issues
        .iter()
        .filter(|issue| issue.code.contains("attachment") || issue.code.contains("relationship"))
        .count();
    HqProviderAttachmentValidity {
        provider_override_count: output.provider_override_reports.len(),
        final_conformance_accepted: output.final_conformance.is_accepted(),
        required_attachment_issue_count,
    }
}

fn candidate_report(fixture: &FoundryFixtureCatalog) -> HqCandidateReport {
    let request = FoundryCandidateRequest {
        seed: DEFAULT_QUALITY_SEED,
        proposal_count: DEFAULT_PROPOSAL_COUNT,
        result_count: DEFAULT_DIRECTION_COUNT,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
    };
    match generate_foundry_candidate_plans(&fixture.document, fixture, &request) {
        Ok(output) => {
            let survived = output
                .candidates
                .iter()
                .filter(|candidate| {
                    candidate_passes_quality_survival(&candidate.document, fixture).unwrap_or(false)
                })
                .count();
            HqCandidateReport {
                mode: "explore".to_owned(),
                seed: request.seed,
                proposal_count: request.proposal_count,
                requested_count: request.result_count,
                returned_count: output.candidates.len(),
                candidate_survival_count: survived,
                six_direction_availability: survived >= DEFAULT_DIRECTION_COUNT,
                failure: None,
            }
        }
        Err(error) => HqCandidateReport {
            mode: "explore".to_owned(),
            seed: request.seed,
            proposal_count: request.proposal_count,
            requested_count: request.result_count,
            returned_count: 0,
            candidate_survival_count: 0,
            six_direction_availability: false,
            failure: Some(error.to_string()),
        },
    }
}

fn candidate_passes_quality_survival(
    document: &shape_foundry::FoundryAssetDocument,
    fixture: &FoundryFixtureCatalog,
) -> anyhow::Result<bool> {
    let compiled = compile_foundry_document(document, fixture)
        .map_err(|error| anyhow::anyhow!("candidate compile failed: {error:#?}"))?;
    if !compiled.artifact.validation_report.is_valid() {
        return Ok(false);
    }
    let model_validation = validate_compiled_output(&compiled);
    if !model_validation.is_valid() {
        return Ok(false);
    }
    let mesh = render_mesh_from_triangles(&compiled.artifact.combined_preview);
    let preview = render_hq_view(&mesh, 42.0, 18.0, false)?;
    Ok(!is_placeholder_image(&preview))
}

fn visible_control_difference_evidence(
    fixture: &FoundryFixtureCatalog,
    output: &FoundryCompilationOutput,
) -> anyhow::Result<HqVisibleControlDifferenceEvidence> {
    let parent_mesh = render_mesh_from_triangles(&output.artifact.combined_preview);
    let parent_image = render_hq_view(&parent_mesh, 42.0, 18.0, false)?;
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    let mut controls = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .collect::<Vec<_>>();
    controls.sort_by(|left, right| left.id.cmp(&right.id));

    let rows = controls
        .iter()
        .map(|control| {
            let current = current_control_value(&output.document, control, context);
            let current_label = current
                .as_ref()
                .map(control_value_label)
                .unwrap_or_else(|error| format!("unavailable: {error}"));
            let Some(sampled) = current
                .ok()
                .and_then(|value| alternate_control_value(control, context, &value))
            else {
                return Ok(HqControlDifferenceRow {
                    control_id: control.id.clone(),
                    label: control.label.clone(),
                    control_kind: control_kind_label(control),
                    current_value: current_label,
                    sampled_value: None,
                    changed_geometry: false,
                    visual_delta_from_parent: 0,
                    failure: Some("no alternate available control value".to_owned()),
                });
            };

            let mut document = output.document.clone();
            document.catalog_lock = None;
            document.build_stamp = None;
            document
                .control_state
                .insert(control.id.clone(), sampled.clone());
            match compile_foundry_document(&document, fixture) {
                Ok(sampled_output) => {
                    let sampled_mesh =
                        render_mesh_from_triangles(&sampled_output.artifact.combined_preview);
                    let sampled_image = render_hq_view(&sampled_mesh, 42.0, 18.0, false)?;
                    let visual_delta = image_delta(&parent_image, &sampled_image);
                    Ok(HqControlDifferenceRow {
                        control_id: control.id.clone(),
                        label: control.label.clone(),
                        control_kind: control_kind_label(control),
                        current_value: current_label,
                        sampled_value: Some(control_value_label(&sampled)),
                        changed_geometry: visual_delta > 0,
                        visual_delta_from_parent: visual_delta,
                        failure: None,
                    })
                }
                Err(error) => Ok(HqControlDifferenceRow {
                    control_id: control.id.clone(),
                    label: control.label.clone(),
                    control_kind: control_kind_label(control),
                    current_value: current_label,
                    sampled_value: Some(control_value_label(&sampled)),
                    changed_geometry: false,
                    visual_delta_from_parent: 0,
                    failure: Some(format!("{error:#?}")),
                }),
            }
        })
        .collect::<anyhow::Result<Vec<_>>>()?;
    let changed_control_count = rows.iter().filter(|row| row.changed_geometry).count();
    Ok(HqVisibleControlDifferenceEvidence {
        primary_controls_checked: rows.len(),
        changed_control_count,
        controls: rows,
    })
}

fn current_control_value(
    document: &shape_foundry::FoundryAssetDocument,
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> anyhow::Result<ControlValue> {
    document
        .control_state
        .get(&control.id)
        .cloned()
        .map(Ok)
        .unwrap_or_else(|| {
            default_control_value(control, context)
                .map_err(|error| anyhow::anyhow!("default value failed: {error:?}"))
        })
}

fn alternate_control_value(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
    current: &ControlValue,
) -> Option<ControlValue> {
    let domain =
        effective_control_domain(control, context).unwrap_or_else(|_| control.domain.clone());
    match (&control.kind, current) {
        (ControlKind::ContinuousAxis { .. }, ControlValue::Scalar(value)) => {
            let mut candidates = domain
                .continuous_intervals
                .iter()
                .flat_map(|interval| [interval.minimum, interval.maximum])
                .filter(|candidate| {
                    candidate.is_finite() && (*candidate - *value).abs() > f32::EPSILON
                })
                .map(ControlValue::Scalar)
                .filter(|candidate| domain.contains_available_value(candidate))
                .collect::<Vec<_>>();
            candidates.sort_by_key(control_value_label);
            candidates.pop()
        }
        _ => domain
            .discrete_values
            .iter()
            .find(|candidate| *candidate != current && domain.contains_available_value(candidate))
            .cloned(),
    }
}

fn export_reopen_report(
    verify_export: bool,
    output: &FoundryCompilationOutput,
    out_dir: &Path,
) -> anyhow::Result<HqExportReopenReport> {
    if !verify_export {
        return Ok(HqExportReopenReport {
            export_status: HqEvidenceStatus::NotRun,
            reopen_status: HqEvidenceStatus::NotRun,
            package_dir: None,
            manifest: None,
            checksums_match: None,
            topology_matches_manifest: None,
            finite_numeric_payloads: None,
            not_run_reason: Some("run with --verify-export to create package evidence".to_owned()),
        });
    }

    let package_dir = out_dir.join("model-package");
    let package_paths = write_model_package(&output.recipe, &output.artifact, &package_dir)
        .with_context(|| format!("writing model package {}", package_dir.display()))?;
    let verification = verify_model_package(&package_dir)
        .with_context(|| format!("verifying model package {}", package_dir.display()))?;
    let verified = verification.checksums_match
        && verification.topology_matches_manifest
        && verification.finite_numeric_payloads;
    Ok(HqExportReopenReport {
        export_status: if verified {
            HqEvidenceStatus::Verified
        } else {
            HqEvidenceStatus::Failed
        },
        reopen_status: if verified {
            HqEvidenceStatus::Verified
        } else {
            HqEvidenceStatus::Failed
        },
        package_dir: Some(package_paths.directory.display().to_string()),
        manifest: Some(package_paths.manifest.display().to_string()),
        checksums_match: Some(verification.checksums_match),
        topology_matches_manifest: Some(verification.topology_matches_manifest),
        finite_numeric_payloads: Some(verification.finite_numeric_payloads),
        not_run_reason: None,
    })
}

fn unsupported_outputs() -> Vec<HqUnsupportedOutput> {
    [
        ("photoreal_render", "photoreal output is not product truth"),
        (
            "uv_layout",
            "UV generation is outside the current Shape Lab scope",
        ),
        ("materials", "materials are not implemented"),
        ("textures", "textures are not implemented"),
        ("rigging", "rigging is not implemented"),
        ("animation", "animation is not implemented"),
        (
            "marketplace_package",
            "marketplace publishing packages are not implemented",
        ),
    ]
    .into_iter()
    .map(|(output, reason)| HqUnsupportedOutput {
        output: output.to_owned(),
        reason: reason.to_owned(),
    })
    .collect()
}

fn control_kind_label(control: &CustomizerControl) -> String {
    match control.kind {
        ControlKind::ContinuousAxis { .. } => "continuous".to_owned(),
        ControlKind::IntegerStepper { .. } => "integer".to_owned(),
        ControlKind::Toggle { .. } => "toggle".to_owned(),
        ControlKind::ChoiceGallery { .. } => "choice_gallery".to_owned(),
        ControlKind::ProviderGallery { .. } => "provider_gallery".to_owned(),
    }
}

fn control_value_label(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => format!("{value:.3}"),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
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

fn is_placeholder_image(image: &RenderedImage) -> bool {
    if image.width <= 1 || image.height <= 1 || image.rgba8.len() <= 4 {
        return true;
    }
    let Some(first) = image.rgba8.get(0..4) else {
        return true;
    };
    image.rgba8.chunks_exact(4).all(|pixel| pixel == first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_evidence() -> HqQualityGateEvidence {
        HqQualityGateEvidence {
            compile_valid: true,
            clay_preview_available: true,
            contact_sheet_available: true,
            front_view_available: true,
            three_quarter_view_available: true,
            side_view_available: true,
            back_view_available: true,
            wireframe_available: true,
            silhouette_available: true,
            visible_control_difference_evidence: true,
            advanced_recipe_required: false,
            export_verified: true,
            reopen_verified: true,
            candidate_survival_count: DEFAULT_DIRECTION_COUNT,
            placeholder_thumbnails: false,
            human_approved: true,
            adversarial_reviewed: true,
        }
    }

    #[test]
    fn hq_quality_tier_cannot_exceed_requested_tier() {
        let decision = evaluate_quality_tier(HqQualityTier::Prototype, &full_evidence());
        assert_eq!(decision.achieved, HqQualityTier::Prototype);
    }

    #[test]
    fn hq_quality_exposure_rules_hide_draft_and_default_prototype() {
        assert!(!novice_catalog_exposure_allowed(
            HqQualityTier::Draft,
            true,
            true
        ));
        assert!(!novice_catalog_exposure_allowed(
            HqQualityTier::Prototype,
            false,
            true
        ));
        assert!(novice_catalog_exposure_allowed(
            HqQualityTier::Prototype,
            true,
            false
        ));
        assert!(!novice_catalog_exposure_allowed(
            HqQualityTier::Usable,
            false,
            false
        ));
        assert!(novice_catalog_exposure_allowed(
            HqQualityTier::Usable,
            false,
            true
        ));
    }

    #[test]
    fn hq_quality_usable_requires_export_reopen_and_control_evidence() {
        let mut evidence = full_evidence();
        evidence.export_verified = false;
        evidence.reopen_verified = false;
        let decision = evaluate_quality_tier(HqQualityTier::Usable, &evidence);
        assert_eq!(decision.achieved, HqQualityTier::Prototype);
        assert!(
            decision
                .blockers
                .iter()
                .any(|blocker| blocker.contains("export and reopen"))
        );

        let mut evidence = full_evidence();
        evidence.visible_control_difference_evidence = false;
        let decision = evaluate_quality_tier(HqQualityTier::Usable, &evidence);
        assert_eq!(decision.achieved, HqQualityTier::Prototype);
    }

    #[test]
    fn hq_quality_usable_rejects_advanced_recipe_dependency_and_placeholder_thumbnails() {
        let mut evidence = full_evidence();
        evidence.advanced_recipe_required = true;
        let decision = evaluate_quality_tier(HqQualityTier::Usable, &evidence);
        assert_eq!(decision.achieved, HqQualityTier::Prototype);

        let mut evidence = full_evidence();
        evidence.placeholder_thumbnails = true;
        let decision = evaluate_quality_tier(HqQualityTier::Usable, &evidence);
        assert_eq!(decision.achieved, HqQualityTier::Prototype);
    }

    #[test]
    fn hq_quality_showcase_requires_human_and_adversarial_markers() {
        let mut evidence = full_evidence();
        evidence.human_approved = false;
        evidence.adversarial_reviewed = false;
        let decision = evaluate_quality_tier(HqQualityTier::Showcase, &evidence);
        assert_eq!(decision.achieved, HqQualityTier::Usable);
        assert!(
            decision
                .blockers
                .iter()
                .any(|blocker| blocker.contains("human approval"))
        );
        assert!(
            decision
                .blockers
                .iter()
                .any(|blocker| blocker.contains("adversarial"))
        );
    }

    #[test]
    fn hq_quality_builtin_profile_list_is_enumerable() {
        let slugs = benchmark_profile_slugs();
        assert_eq!(slugs.len(), 11);
        assert!(slugs.contains(&"roman-bridge".to_owned()));
        assert!(slugs.contains(&"roman-bridge-hq".to_owned()));
        assert!(slugs.contains(&"stylized-tree".to_owned()));
    }

    #[test]
    fn roman_bridge_hq_explore_candidates_survive_quality_validation() {
        let fixture = shape_foundry_catalog::roman_bridge::hq_fixture_catalog();
        let request = FoundryCandidateRequest {
            seed: DEFAULT_QUALITY_SEED,
            proposal_count: DEFAULT_PROPOSAL_COUNT,
            result_count: DEFAULT_DIRECTION_COUNT,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
        };
        let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &request)
            .expect("HQ bridge candidates should generate");
        assert_eq!(output.candidates.len(), DEFAULT_DIRECTION_COUNT);

        let mut survived = 0_usize;
        let mut failures = Vec::new();
        for candidate in &output.candidates {
            match compile_foundry_document(&candidate.document, &fixture) {
                Ok(compiled) => {
                    let model_validation = validate_compiled_output(&compiled);
                    if compiled.artifact.validation_report.is_valid() && model_validation.is_valid()
                    {
                        survived += 1;
                    } else {
                        failures.push(format!(
                            "{} controls={:?} artifact_valid={} model_issues={:?}",
                            candidate.id.0,
                            candidate.changed_controls,
                            compiled.artifact.validation_report.is_valid(),
                            model_validation.issues
                        ));
                    }
                }
                Err(error) => failures.push(format!(
                    "{} controls={:?} compile_error={error:#?}",
                    candidate.id.0, candidate.changed_controls
                )),
            }
        }

        assert_eq!(
            survived,
            DEFAULT_DIRECTION_COUNT,
            "candidate failures:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn hq_quality_placeholder_detection_rejects_blank_images() {
        let tiny = RenderedImage {
            width: 1,
            height: 1,
            rgba8: vec![0, 0, 0, 255],
        };
        assert!(is_placeholder_image(&tiny));

        let blank = RenderedImage {
            width: 2,
            height: 2,
            rgba8: vec![
                24, 26, 28, 255, 24, 26, 28, 255, 24, 26, 28, 255, 24, 26, 28, 255,
            ],
        };
        assert!(is_placeholder_image(&blank));

        let non_blank = RenderedImage {
            width: 2,
            height: 2,
            rgba8: vec![
                24, 26, 28, 255, 244, 246, 248, 255, 24, 26, 28, 255, 24, 26, 28, 255,
            ],
        };
        assert!(!is_placeholder_image(&non_blank));
    }

    #[test]
    fn hq_quality_report_schema_serializes_required_fields() {
        let report = HqQualityReport {
            schema_version: HQ_QUALITY_REPORT_SCHEMA_VERSION,
            generated_at: "deterministic-no-wall-clock".to_owned(),
            deterministic_timestamp_policy: "no wall clock".to_owned(),
            profile_id: "roman-bridge".to_owned(),
            profile_label: "Roman Timber Bridge".to_owned(),
            kit_id: Some("roman_timber_engineering".to_owned()),
            style_id: Some("roman_timber_engineering".to_owned()),
            quality_tier_requested: HqQualityTier::Usable,
            quality_tier_achieved: HqQualityTier::Prototype,
            quality_tier_blockers: vec![
                "Usable requires export and reopen verification".to_owned(),
            ],
            clay_preview_available: true,
            contact_sheet_available: true,
            front_view_available: true,
            three_quarter_view_available: true,
            side_view_available: true,
            back_view_available: true,
            wireframe_available: true,
            silhouette_available: true,
            silhouette_readability_metric: Some(0.25),
            silhouette_manual_review_required: true,
            mesh_validity_summary: HqMeshValiditySummary {
                compile_valid: true,
                model_valid: true,
                issue_count: 0,
                error_count: 0,
                warning_count: 0,
                provenance_coverage: 1.0,
                manifold_closed_part_fraction: 1.0,
                accidental_intersection_count: 0,
            },
            triangle_count: 100,
            triangle_budget: Some(DEFAULT_TRIANGLE_BUDGET),
            semantic_part_inventory: HqSemanticPartInventory {
                part_count: 1,
                parts: vec![HqSemanticPartRow {
                    instance_id: 1,
                    definition_id: 1,
                    name: "part".to_owned(),
                    source_recipe_instance: true,
                    generated_by: None,
                    triangle_count: 12,
                    region_count: 0,
                }],
            },
            required_role_coverage: HqRequiredRoleCoverage {
                required_role_count: 1,
                covered_required_role_count: 1,
                missing_required_roles: Vec::new(),
                accepted: true,
            },
            provider_attachment_validity: HqProviderAttachmentValidity {
                provider_override_count: 0,
                final_conformance_accepted: true,
                required_attachment_issue_count: 0,
            },
            candidate_survival_count: DEFAULT_DIRECTION_COUNT,
            six_direction_availability: true,
            primary_control_count: 1,
            visible_control_difference_evidence: HqVisibleControlDifferenceEvidence {
                primary_controls_checked: 1,
                changed_control_count: 1,
                controls: Vec::new(),
            },
            advanced_recipe_required: false,
            export_status: HqEvidenceStatus::NotRun,
            reopen_status: HqEvidenceStatus::NotRun,
            unsupported_outputs: unsupported_outputs(),
            placeholder_thumbnail_detected: false,
            human_review_required: true,
            human_approval_status: HqHumanApprovalStatus::Pending,
            adversarial_review_status: HqHumanApprovalStatus::Pending,
            manual_notes_path: None,
            novice_catalog_exposure_allowed_by_default: false,
        };

        let value = serde_json::to_value(&report).expect("quality report serializes");
        assert_eq!(value["schema_version"], HQ_QUALITY_REPORT_SCHEMA_VERSION);
        assert_eq!(value["quality_tier_requested"], "usable");
        assert_eq!(value["quality_tier_achieved"], "prototype");
        assert_eq!(
            value["unsupported_outputs"][0]["output"],
            "photoreal_render"
        );
    }
}
