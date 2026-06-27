use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::ValueEnum;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use shape_character::prepared::prepared_hero_template_v1;
use shape_compile::export::{verify_model_package, write_model_package};
use shape_compile::validation::{
    ModelValidationReport, ValidationLimits, validate_model,
    validation_config_from_recipe_with_limits,
};
use shape_foundry::{
    ControlEvaluationContext, ControlKind, ControlValue, CustomizerControl, FoundryAssetDocument,
    FoundryCompilationOutput, FoundryDocumentId, FoundryPackDocument, FoundryPackExportProfile,
    compile_foundry_document, compile_foundry_pack, default_control_value,
    effective_control_domain,
};
use shape_foundry_catalog::moba_hero::MOBA_HERO_CLAY_SLUG;
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
const PREPARED_HERO_TEMPLATE_PROFILE: &str = "prepared-hero-template-v1";

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

#[derive(Debug, clap::Args)]
pub struct HqAdversarialReviewArgs {
    /// Directory containing an HQ benchmark quality-report.json and evidence files.
    #[arg(long)]
    benchmark_dir: PathBuf,
    /// Output adversarial-review.json path.
    #[arg(long)]
    out: PathBuf,
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

pub const HQ_ADVERSARIAL_REVIEW_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqAdversarialReviewReport {
    pub schema_version: u32,
    pub profile_id: String,
    pub reviewed_quality_report: Option<String>,
    pub visual_questions: Vec<HqAdversarialReviewQuestion>,
    pub mesh_questions: Vec<HqAdversarialReviewQuestion>,
    #[serde(rename = "UX_questions")]
    pub ux_questions: Vec<HqAdversarialReviewQuestion>,
    pub blocker_findings: Vec<String>,
    pub non_blocking_findings: Vec<String>,
    pub tier_recommendation: HqQualityTier,
    pub required_followups: Vec<String>,
    pub human_review_required: bool,
    pub human_reviewer_status: HqHumanApprovalStatus,
    pub cannot_automatically_judge_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HqAdversarialReviewQuestion {
    pub question: String,
    pub manual_review_required: bool,
    pub evidence_refs: Vec<String>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqHeroPackReport {
    pub schema_version: u32,
    pub profile_id: String,
    pub pack_id: String,
    pub source_template_id: String,
    pub source_template_fingerprint: String,
    pub pack_report_fingerprint: String,
    pub shared_style: String,
    pub shared_controls: Vec<String>,
    pub members: Vec<HqHeroPackMemberReport>,
    pub total_triangle_count: u64,
    pub semantic_part_inventory: HqSemanticPartInventory,
    pub conformance_status: Vec<String>,
    pub export_status: HqEvidenceStatus,
    pub reopen_status: HqEvidenceStatus,
    pub exported_member_package_count: usize,
    pub pack_export_dir: String,
    pub dcc_sidecar_status: HqEvidenceStatus,
    pub dcc_sidecar_reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HqHeroPackMemberReport {
    pub name: String,
    pub role: String,
    pub differing_controls: Vec<String>,
    pub triangle_count: u64,
    pub semantic_part_count: u64,
    pub conformance_accepted: bool,
}

type HqHeroPackMemberControls = Vec<(&'static str, ControlValue)>;
type HqHeroPackMemberSpec = (
    &'static str,
    &'static str,
    &'static str,
    HqHeroPackMemberControls,
);

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

pub fn run_hq_adversarial_review(args: HqAdversarialReviewArgs) -> anyhow::Result<()> {
    let report = build_hq_adversarial_review(&args.benchmark_dir);
    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    write_json(&args.out, &report)?;
    println!(
        "HQ adversarial review {}: recommendation={:?} blockers={}",
        report.profile_id,
        report.tier_recommendation,
        report.blocker_findings.len()
    );
    Ok(())
}

#[must_use]
pub fn build_hq_adversarial_review(benchmark_dir: &Path) -> HqAdversarialReviewReport {
    let inferred_profile_id = benchmark_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown-profile")
        .to_owned();
    let quality_report_path = benchmark_dir.join("quality-report.json");
    let mut blocker_findings = Vec::new();
    let mut non_blocking_findings = Vec::new();
    let mut required_followups = Vec::new();
    let mut reviewed_quality_report = None;
    let mut profile_id = inferred_profile_id.clone();
    let mut tier_recommendation = HqQualityTier::Draft;
    let mut human_reviewer_status = HqHumanApprovalStatus::Pending;

    if !benchmark_dir.is_dir() {
        blocker_findings.push(format!(
            "missing_benchmark_dir: {} does not exist",
            benchmark_dir.display()
        ));
        required_followups.push("Run the HQ quality benchmark before claiming a tier.".to_owned());
    } else if !quality_report_path.is_file() {
        blocker_findings.push(format!(
            "missing_quality_report: {} is missing",
            quality_report_path.display()
        ));
        required_followups.push(
            "Generate quality-report.json before adversarial review can assess evidence."
                .to_owned(),
        );
    } else {
        reviewed_quality_report = Some(quality_report_path.display().to_string());
        match fs::read_to_string(&quality_report_path)
            .ok()
            .and_then(|json| serde_json::from_str::<HqQualityReport>(&json).ok())
        {
            Some(quality) => {
                profile_id = quality.profile_id.clone();
                human_reviewer_status = quality.human_approval_status;
                let correction = adversarial_tier_correction(
                    &quality,
                    benchmark_dir,
                    &mut blocker_findings,
                    &mut non_blocking_findings,
                    &mut required_followups,
                );
                tier_recommendation = correction;
            }
            None => {
                blocker_findings.push(format!(
                    "unreadable_quality_report: {} could not be parsed",
                    quality_report_path.display()
                ));
                required_followups
                    .push("Regenerate quality-report.json with the current schema.".to_owned());
            }
        }
    }

    HqAdversarialReviewReport {
        schema_version: HQ_ADVERSARIAL_REVIEW_SCHEMA_VERSION,
        profile_id,
        reviewed_quality_report,
        visual_questions: adversarial_visual_questions(),
        mesh_questions: adversarial_mesh_questions(),
        ux_questions: adversarial_ux_questions(),
        blocker_findings,
        non_blocking_findings,
        tier_recommendation,
        required_followups,
        human_review_required: true,
        human_reviewer_status,
        cannot_automatically_judge_fields: cannot_automatically_judge_fields(),
    }
}

fn adversarial_tier_correction(
    quality: &HqQualityReport,
    benchmark_dir: &Path,
    blocker_findings: &mut Vec<String>,
    non_blocking_findings: &mut Vec<String>,
    required_followups: &mut Vec<String>,
) -> HqQualityTier {
    for blocker in &quality.quality_tier_blockers {
        blocker_findings.push(format!("quality_report_blocker: {blocker}"));
    }

    let mut recommendation = quality.quality_tier_achieved;
    if !quality.quality_tier_blockers.is_empty()
        && quality.quality_tier_achieved == quality.quality_tier_requested
    {
        let downgraded = downgrade_one_tier(recommendation);
        if downgraded < recommendation {
            blocker_findings.push(format!(
                "tier_overclaim.report_blockers: claimed {:?} while quality blockers are present",
                recommendation
            ));
            recommendation = downgraded;
        }
    }
    let recomputed = evaluate_quality_tier(
        quality.quality_tier_requested,
        &gate_evidence_from_report(quality),
    );
    if recomputed.achieved < recommendation {
        blocker_findings.push(format!(
            "tier_overclaim.recomputed_evidence: quality report achieved {:?}, but evidence recomputes to {:?}",
            recommendation, recomputed.achieved
        ));
        required_followups.push(
            "Regenerate the quality report or correct the tier before public demo review."
                .to_owned(),
        );
        recommendation = recomputed.achieved;
    }
    for blocker in recomputed.blockers {
        let finding = format!("recomputed_quality_blocker: {blocker}");
        if !blocker_findings.contains(&finding) {
            blocker_findings.push(finding);
        }
    }
    let claimed_or_recommended_usable = quality.quality_tier_achieved >= HqQualityTier::Usable
        || recommendation >= HqQualityTier::Usable;
    let required_files = [
        (
            "contact_sheet",
            "contact-sheet.png",
            quality.contact_sheet_available,
        ),
        ("front_view", "front.png", quality.front_view_available),
        (
            "three_quarter_view",
            "three-quarter.png",
            quality.three_quarter_view_available,
        ),
        ("side_view", "side.png", quality.side_view_available),
        ("back_view", "back.png", quality.back_view_available),
        (
            "wireframe_view",
            "wireframe.png",
            quality.wireframe_available,
        ),
        (
            "silhouette_view",
            "silhouette.png",
            quality.silhouette_available,
        ),
        ("mesh_stats", "mesh-stats.json", true),
        ("semantic_parts", "semantic-parts.json", true),
        ("candidate_report", "candidate-report.json", true),
        (
            "controls_visibility_report",
            "controls-visibility-report.json",
            true,
        ),
        ("export_reopen_report", "export-reopen-report.json", true),
    ];
    let mut missing_required_file = false;
    for (label, file, report_claims_available) in required_files {
        if !report_claims_available || !benchmark_dir.join(file).is_file() {
            let finding = format!("missing_evidence.{label}: {file}");
            if claimed_or_recommended_usable {
                blocker_findings.push(finding);
                missing_required_file = true;
            } else {
                non_blocking_findings.push(finding);
            }
        }
    }

    if quality.advanced_recipe_required && claimed_or_recommended_usable {
        blocker_findings.push(
            "tier_overclaim.advanced_recipe: Usable cannot require Advanced Recipe".to_owned(),
        );
        required_followups
            .push("Keep this profile below Usable until the novice path no longer depends on Advanced Recipe.".to_owned());
        recommendation = recommendation.min(HqQualityTier::Prototype);
    }
    if (quality.export_status != HqEvidenceStatus::Verified
        || quality.reopen_status != HqEvidenceStatus::Verified)
        && claimed_or_recommended_usable
    {
        blocker_findings.push(
            "tier_overclaim.export_reopen: Usable requires verified export and reopen".to_owned(),
        );
        required_followups
            .push("Regenerate package export/reopen evidence before claiming Usable.".to_owned());
        recommendation = recommendation.min(HqQualityTier::Prototype);
    }
    if quality.candidate_survival_count < DEFAULT_DIRECTION_COUNT && claimed_or_recommended_usable {
        blocker_findings.push(format!(
            "tier_overclaim.candidates: expected {DEFAULT_DIRECTION_COUNT} surviving candidates, found {}",
            quality.candidate_survival_count
        ));
        recommendation = recommendation.min(HqQualityTier::Prototype);
    }
    if !quality
        .visible_control_difference_evidence
        .all_primary_controls_changed()
        && claimed_or_recommended_usable
    {
        blocker_findings.push(
            "tier_overclaim.controls: every primary control needs visible whole-model evidence"
                .to_owned(),
        );
        recommendation = recommendation.min(HqQualityTier::Prototype);
    }
    if quality.placeholder_thumbnail_detected && claimed_or_recommended_usable {
        blocker_findings.push(
            "tier_overclaim.placeholder_thumbnail: placeholder thumbnails cannot support Usable"
                .to_owned(),
        );
        recommendation = recommendation.min(HqQualityTier::Prototype);
    }
    if missing_required_file && claimed_or_recommended_usable {
        recommendation = recommendation.min(HqQualityTier::Prototype);
        required_followups
            .push("Fill missing benchmark evidence files before public demo review.".to_owned());
    }
    if quality.quality_tier_achieved == HqQualityTier::Showcase
        && (quality.human_approval_status != HqHumanApprovalStatus::Approved
            || quality.adversarial_review_status != HqHumanApprovalStatus::Approved)
    {
        blocker_findings.push(
            "tier_overclaim.showcase_review: Showcase requires human/pro approval and adversarial visual review"
                .to_owned(),
        );
        recommendation = recommendation.min(HqQualityTier::Usable);
    }
    if matches!(
        recommendation,
        HqQualityTier::Draft | HqQualityTier::Prototype
    ) && quality.novice_catalog_exposure_allowed_by_default
    {
        blocker_findings.push(
            "visibility_policy: Draft and Prototype profiles must stay hidden from the default novice catalog"
                .to_owned(),
        );
        required_followups
            .push("Remove default novice exposure for Draft/Prototype content.".to_owned());
    }

    if quality.human_approval_status != HqHumanApprovalStatus::Approved {
        non_blocking_findings.push(
            "manual_review_pending: automatic evidence is not a human art approval".to_owned(),
        );
    }
    if quality.profile_id.contains("hero") {
        non_blocking_findings
            .push("hero_boundary: clay hero output remains clay mesh only; no UV/material/rig/animation claim".to_owned());
    }
    recommendation
}

fn downgrade_one_tier(tier: HqQualityTier) -> HqQualityTier {
    match tier {
        HqQualityTier::Showcase => HqQualityTier::Usable,
        HqQualityTier::Usable => HqQualityTier::Prototype,
        HqQualityTier::Prototype => HqQualityTier::Draft,
        HqQualityTier::Draft => HqQualityTier::Draft,
    }
}

fn gate_evidence_from_report(report: &HqQualityReport) -> HqQualityGateEvidence {
    HqQualityGateEvidence {
        compile_valid: report.mesh_validity_summary.compile_valid
            && report.mesh_validity_summary.model_valid,
        clay_preview_available: report.clay_preview_available,
        contact_sheet_available: report.contact_sheet_available,
        front_view_available: report.front_view_available,
        three_quarter_view_available: report.three_quarter_view_available,
        side_view_available: report.side_view_available,
        back_view_available: report.back_view_available,
        wireframe_available: report.wireframe_available,
        silhouette_available: report.silhouette_available,
        visible_control_difference_evidence: report
            .visible_control_difference_evidence
            .all_primary_controls_changed(),
        advanced_recipe_required: report.advanced_recipe_required,
        export_verified: report.export_status == HqEvidenceStatus::Verified,
        reopen_verified: report.reopen_status == HqEvidenceStatus::Verified,
        candidate_survival_count: report.candidate_survival_count,
        placeholder_thumbnails: report.placeholder_thumbnail_detected,
        human_approved: report.human_approval_status == HqHumanApprovalStatus::Approved,
        adversarial_reviewed: report.adversarial_review_status == HqHumanApprovalStatus::Approved,
    }
}

fn adversarial_visual_questions() -> Vec<HqAdversarialReviewQuestion> {
    [
        (
            "Does this look like a toy?",
            ["contact-sheet.png", "three-quarter.png"].as_slice(),
        ),
        (
            "Does the silhouette read at 128px?",
            ["silhouette.png"].as_slice(),
        ),
        (
            "Do variants preserve identity?",
            ["candidate-report.json", "contact-sheet.png"].as_slice(),
        ),
        (
            "Do armor/bridge/gear pieces look attached or pasted on?",
            ["wireframe.png", "three-quarter.png"].as_slice(),
        ),
        (
            "Do all generated candidates look art-directed?",
            ["candidate-report.json", "contact-sheet.png"].as_slice(),
        ),
        (
            "Would this embarrass us next to a private clay-render reference board?",
            ["contact-sheet.png"].as_slice(),
        ),
        (
            "Does any output look like procedural filler?",
            ["contact-sheet.png", "candidate-report.json"].as_slice(),
        ),
        (
            "Would a curated Blender/Houdini kit beat this today?",
            ["contact-sheet.png", "export-reopen-report.json"].as_slice(),
        ),
    ]
    .into_iter()
    .map(|(question, refs)| manual_question(question, refs))
    .collect()
}

fn adversarial_mesh_questions() -> Vec<HqAdversarialReviewQuestion> {
    [
        (
            "Are there visible mesh seams, accidental intersections, or pasted-on parts?",
            ["wireframe.png", "mesh-stats.json", "semantic-parts.json"].as_slice(),
        ),
        (
            "Are primary controls visibly meaningful?",
            ["controls-visibility-report.json"].as_slice(),
        ),
        (
            "Does the quality tier overclaim the evidence?",
            ["quality-report.json", "export-reopen-report.json"].as_slice(),
        ),
    ]
    .into_iter()
    .map(|(question, refs)| manual_question(question, refs))
    .collect()
}

fn adversarial_ux_questions() -> Vec<HqAdversarialReviewQuestion> {
    [
        (
            "Are there too many choices for a noob?",
            ["quality-report.json", "controls-visibility-report.json"].as_slice(),
        ),
        (
            "Are candidates coherent or just random combinations?",
            ["candidate-report.json", "contact-sheet.png"].as_slice(),
        ),
        (
            "Is Visual Foundry still simpler than traditional modeling for the task?",
            ["quality-report.json", "contact-sheet.png"].as_slice(),
        ),
    ]
    .into_iter()
    .map(|(question, refs)| manual_question(question, refs))
    .collect()
}

fn manual_question(question: &str, evidence_refs: &[&str]) -> HqAdversarialReviewQuestion {
    HqAdversarialReviewQuestion {
        question: question.to_owned(),
        manual_review_required: true,
        evidence_refs: evidence_refs
            .iter()
            .map(|item| (*item).to_owned())
            .collect(),
    }
}

fn cannot_automatically_judge_fields() -> Vec<String> {
    [
        "toy_like_art_quality",
        "silhouette_readability_at_128px",
        "variant_identity_preservation",
        "attachment_visual_believability",
        "candidate_art_direction",
        "private_reference_board_comparison",
        "procedural_filler_impression",
        "curated_blender_or_houdini_comparison",
        "novice_simplicity",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn run_hq_quality_benchmark(args: HqQualityBenchmarkArgs) -> anyhow::Result<()> {
    let normalized_profile = normalize_profile_slug(&args.profile);
    let include_prepared_hero =
        normalized_profile == "all" || normalized_profile == PREPARED_HERO_TEMPLATE_PROFILE;
    let profiles = if normalized_profile == PREPARED_HERO_TEMPLATE_PROFILE {
        Vec::new()
    } else {
        resolve_benchmark_profiles(&args.profile)?
    };
    let multi_profile = profiles.len() + usize::from(include_prepared_hero) > 1;
    let mut reports = Vec::with_capacity(profiles.len() + usize::from(include_prepared_hero));
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
    if include_prepared_hero {
        let out_dir = if multi_profile {
            args.out_dir.join(PREPARED_HERO_TEMPLATE_PROFILE)
        } else {
            args.out_dir.clone()
        };
        fs::create_dir_all(&out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
        reports.push(benchmark_prepared_hero_template(&args, &out_dir)?);
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
    let mut slugs = built_in_fixture_catalogs_with_labels()
        .into_iter()
        .map(|(_, fixture)| fixture.slug)
        .collect::<Vec<_>>();
    slugs.push(PREPARED_HERO_TEMPLATE_PROFILE.to_owned());
    slugs
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
    let verify_export = args.verify_export
        || shape_foundry_catalog::showcase_gear::is_showcase_gear_slug(&fixture.slug)
        || fixture.slug == MOBA_HERO_CLAY_SLUG;
    let export_reopen = export_reopen_report(verify_export, &output, out_dir)?;
    if fixture.slug == MOBA_HERO_CLAY_SLUG {
        write_moba_hero_evidence(&fixture, &three_quarter, out_dir)?;
    }
    let silhouette_metric = silhouette_readability_metric(&mesh);
    let unsupported_outputs = unsupported_outputs_for_profile(&fixture.slug);

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

fn write_moba_hero_evidence(
    fixture: &FoundryFixtureCatalog,
    parent_preview: &RenderedImage,
    out_dir: &Path,
) -> anyhow::Result<()> {
    write_candidate_mode_contact_sheet(
        fixture,
        parent_preview,
        FoundryCandidateMode::Explore,
        "explore",
        out_dir.join("explore-contact-sheet.png"),
    )?;
    write_candidate_mode_contact_sheet(
        fixture,
        parent_preview,
        FoundryCandidateMode::Silhouette,
        "silhouette",
        out_dir.join("silhouette-contact-sheet.png"),
    )?;
    write_candidate_mode_contact_sheet(
        fixture,
        parent_preview,
        FoundryCandidateMode::Structure,
        "armor_gear",
        out_dir.join("gear-contact-sheet.png"),
    )?;
    let hero_pack_report = moba_hero_pack_report(fixture, out_dir)?;
    write_json(out_dir.join("hero-pack-report.json"), &hero_pack_report)?;
    Ok(())
}

fn write_candidate_mode_contact_sheet(
    fixture: &FoundryFixtureCatalog,
    parent_preview: &RenderedImage,
    mode: FoundryCandidateMode,
    strategy_id: &str,
    path: PathBuf,
) -> anyhow::Result<()> {
    let request = FoundryCandidateRequest {
        seed: DEFAULT_QUALITY_SEED,
        proposal_count: DEFAULT_PROPOSAL_COUNT,
        result_count: DEFAULT_DIRECTION_COUNT,
        mode,
        strategy_id: Some(strategy_id.to_owned()),
        preference_profile: None,
        variation_intent: shape_foundry::VariationIntent::default(),
    };
    let output = generate_foundry_candidate_plans(&fixture.document, fixture, &request)
        .map_err(|error| anyhow::anyhow!("{strategy_id} candidates failed: {error}"))?;
    anyhow::ensure!(
        output.candidates.len() >= DEFAULT_DIRECTION_COUNT,
        "{strategy_id} produced {} candidates, expected {}",
        output.candidates.len(),
        DEFAULT_DIRECTION_COUNT
    );
    let mut images = Vec::with_capacity(DEFAULT_DIRECTION_COUNT);
    for candidate in output.candidates.iter().take(DEFAULT_DIRECTION_COUNT) {
        let compiled = compile_foundry_document(&candidate.document, fixture).map_err(|error| {
            anyhow::anyhow!("{strategy_id} candidate failed to compile: {error:#?}")
        })?;
        let mesh = render_mesh_from_triangles(&compiled.artifact.combined_preview);
        images.push(render_hq_view(&mesh, 42.0, 18.0, false)?);
    }
    let refs = images.iter().collect::<Vec<_>>();
    save_contact_sheet(parent_preview, &refs, path)?;
    Ok(())
}

fn moba_hero_pack_report(
    fixture: &FoundryFixtureCatalog,
    out_dir: &Path,
) -> anyhow::Result<HqHeroPackReport> {
    let source_template = prepared_hero_template_v1();
    let pack = moba_hero_pack_document(fixture);
    let pack_output = compile_foundry_pack(&pack, fixture)
        .map_err(|error| anyhow::anyhow!("moba hero pack failed to compile: {error:#?}"))?;

    let pack_export_dir = out_dir.join("hero-pack-model-package");
    fs::create_dir_all(&pack_export_dir)
        .with_context(|| format!("creating {}", pack_export_dir.display()))?;
    let pack_document_path = pack_export_dir.join("pack-document.json");
    write_json(&pack_document_path, &pack_output.pack)?;
    write_json(
        pack_export_dir.join("pack-report.json"),
        &pack_output.report,
    )?;
    let serialized_pack = serde_json::from_str::<FoundryPackDocument>(
        &fs::read_to_string(&pack_document_path)
            .with_context(|| format!("reading {}", pack_document_path.display()))?,
    )
    .with_context(|| format!("parsing {}", pack_document_path.display()))?;
    let reopened_pack = compile_foundry_pack(&serialized_pack, fixture).map_err(|error| {
        anyhow::anyhow!(
            "moba hero serialized pack {} failed to reopen: {error:#?}",
            pack_document_path.display()
        )
    })?;
    let reopened_matches =
        reopened_pack.report.report_fingerprint == pack_output.report.report_fingerprint;

    let mut exported_member_package_count = 0_usize;
    let mut member_exports_verified = true;
    for (member_id, member_output) in &pack_output.member_outputs {
        let member_dir = pack_export_dir.join(member_id);
        write_model_package(&member_output.recipe, &member_output.artifact, &member_dir)
            .with_context(|| format!("writing pack member package {}", member_dir.display()))?;
        let verification = verify_model_package(&member_dir)
            .with_context(|| format!("verifying pack member package {}", member_dir.display()))?;
        let verified = verification.checksums_match
            && verification.topology_matches_manifest
            && verification.finite_numeric_payloads;
        if verified {
            exported_member_package_count += 1;
        }
        member_exports_verified &= verified;
    }

    let member_specs = moba_hero_pack_members();
    let mut members = Vec::new();
    let mut part_rows = Vec::new();
    let mut conformance_status = Vec::new();
    for (member_index, (member_id, name, role, controls)) in member_specs.iter().enumerate() {
        let output = pack_output
            .member_outputs
            .get(*member_id)
            .ok_or_else(|| anyhow::anyhow!("missing pack member output {member_id}"))?;
        let inventory = semantic_part_inventory(output);
        for mut row in inventory.parts {
            row.instance_id += (member_index as u64) * 10_000;
            row.name = format!("{name} {}", row.name);
            part_rows.push(row);
        }
        let member_report = pack_output
            .report
            .members
            .iter()
            .find(|member| member.member_id == *member_id)
            .ok_or_else(|| anyhow::anyhow!("missing pack member report {member_id}"))?;
        conformance_status.push(format!(
            "{name}: {}",
            if member_report.conformance.accepted {
                "accepted"
            } else {
                "rejected"
            }
        ));
        members.push(HqHeroPackMemberReport {
            name: (*name).to_owned(),
            role: (*role).to_owned(),
            differing_controls: controls
                .iter()
                .map(|(control, _)| product_control_label(control).to_owned())
                .collect(),
            triangle_count: member_report.triangle_count,
            semantic_part_count: inventory.part_count,
            conformance_accepted: member_report.conformance.accepted,
        });
    }
    Ok(HqHeroPackReport {
        schema_version: 2,
        profile_id: MOBA_HERO_CLAY_SLUG.to_owned(),
        pack_id: "moba-hero-clay-demo-pack".to_owned(),
        source_template_id: source_template.template_id,
        source_template_fingerprint: source_template.base_topology.base_library_fingerprint.0,
        pack_report_fingerprint: pack_output.report.report_fingerprint.to_hex(),
        shared_style: "MOBA Heroic Clay".to_owned(),
        shared_controls: vec![
            "Hero Archetype".to_owned(),
            "Body Proportions".to_owned(),
            "Silhouette".to_owned(),
            "Armor Mass".to_owned(),
            "Head & Face".to_owned(),
            "Hair / Headgear".to_owned(),
            "Weapon / Accessory".to_owned(),
        ],
        members,
        total_triangle_count: pack_output.report.triangle_totals.total,
        semantic_part_inventory: HqSemanticPartInventory {
            part_count: part_rows.len() as u64,
            parts: part_rows,
        },
        conformance_status,
        export_status: if member_exports_verified
            && exported_member_package_count == pack_output.member_outputs.len()
        {
            HqEvidenceStatus::Verified
        } else {
            HqEvidenceStatus::Failed
        },
        reopen_status: if reopened_matches {
            HqEvidenceStatus::Verified
        } else {
            HqEvidenceStatus::Failed
        },
        exported_member_package_count,
        pack_export_dir: pack_export_dir.display().to_string(),
        dcc_sidecar_status: HqEvidenceStatus::Unsupported,
        dcc_sidecar_reason: "No DCC sidecar adapter is implemented for clay hero packs yet."
            .to_owned(),
    })
}

fn moba_hero_pack_document(fixture: &FoundryFixtureCatalog) -> FoundryPackDocument {
    let mut pack = FoundryPackDocument::new(
        "moba-hero-clay-demo-pack",
        fixture.document.family_content_ref.clone(),
        fixture.document.style_content_ref.clone(),
        FoundryPackExportProfile {
            profile: "canonical-model-package".to_owned(),
            require_all_members: true,
        },
    );
    for (member_id, name, _, controls) in moba_hero_pack_members() {
        pack.members.insert(
            member_id.to_owned(),
            moba_hero_member_document(fixture, name, &controls),
        );
    }
    pack
}

fn moba_hero_member_document(
    fixture: &FoundryFixtureCatalog,
    member_name: &str,
    controls: &[(&'static str, ControlValue)],
) -> FoundryAssetDocument {
    let mut document = fixture.document.clone();
    document.document_id = FoundryDocumentId(format!(
        "{}-{}",
        MOBA_HERO_CLAY_SLUG,
        member_name.to_ascii_lowercase().replace([' ', '/'], "-")
    ));
    document.catalog_lock = None;
    document.build_stamp = None;
    for (control, value) in controls {
        document
            .control_state
            .insert((*control).to_owned(), value.clone());
    }
    document
}

fn moba_hero_pack_members() -> [HqHeroPackMemberSpec; 3] {
    [
        (
            "duelist-vanguard",
            "Duelist Vanguard",
            "Main",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("armored_duelist".to_owned()),
                ),
                (
                    "armor_mass",
                    ControlValue::Choice("duelist_mail".to_owned()),
                ),
                (
                    "weapon_accessory",
                    ControlValue::Choice("blade_and_scabbard".to_owned()),
                ),
            ],
        ),
        (
            "arcane-ranger",
            "Arcane Ranger",
            "Variant",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("arcane_ranger".to_owned()),
                ),
                ("head_face", ControlValue::Choice("arcane_mask".to_owned())),
                (
                    "hair_headgear",
                    ControlValue::Choice("swept_hair".to_owned()),
                ),
                (
                    "weapon_accessory",
                    ControlValue::Choice("staff_and_cloak".to_owned()),
                ),
            ],
        ),
        (
            "monster-hunter",
            "Monster Hunter",
            "Variant",
            vec![
                (
                    "hero_archetype",
                    ControlValue::Choice("monster_hunter".to_owned()),
                ),
                (
                    "armor_mass",
                    ControlValue::Choice("hunter_leathers".to_owned()),
                ),
                (
                    "hair_headgear",
                    ControlValue::Choice("horned_hood".to_owned()),
                ),
                (
                    "weapon_accessory",
                    ControlValue::Choice("axe_and_trophy".to_owned()),
                ),
            ],
        ),
    ]
}

fn product_control_label(control_id: &str) -> &'static str {
    match control_id {
        "hero_archetype" => "Hero Archetype",
        "body_proportions" => "Body Proportions",
        "silhouette" => "Silhouette",
        "armor_mass" => "Armor Mass",
        "head_face" => "Head & Face",
        "hair_headgear" => "Hair / Headgear",
        "weapon_accessory" => "Weapon / Accessory",
        _ => "Unknown Control",
    }
}

fn benchmark_prepared_hero_template(
    args: &HqQualityBenchmarkArgs,
    out_dir: &Path,
) -> anyhow::Result<HqQualityReport> {
    let template = prepared_hero_template_v1();
    let validation = template.validate();
    let validation_ok = validation.is_ok();
    let validation_error = validation.err().map(|error| error.to_string());

    write_json(out_dir.join("prepared-template-contract.json"), &template)?;

    let semantic_part_inventory = HqSemanticPartInventory {
        part_count: template.semantic_regions.len() as u64,
        parts: template
            .semantic_regions
            .iter()
            .enumerate()
            .map(|(index, region)| HqSemanticPartRow {
                instance_id: index as u64 + 1,
                definition_id: index as u64 + 1,
                name: region.label.clone(),
                source_recipe_instance: false,
                generated_by: None,
                triangle_count: 0,
                region_count: 1,
            })
            .collect(),
    };
    let visible_control_difference_evidence = HqVisibleControlDifferenceEvidence {
        primary_controls_checked: template.control_profile.controls.len(),
        changed_control_count: 0,
        controls: template
            .control_profile
            .controls
            .iter()
            .map(|control| HqControlDifferenceRow {
                control_id: control.control_id.0.clone(),
                label: control.label.clone(),
                control_kind: "prepared_template".to_owned(),
                current_value: "default".to_owned(),
                sampled_value: None,
                changed_geometry: false,
                visual_delta_from_parent: 0,
                failure: Some(
                    "prepared hero template v1 has no rendered clay mesh output yet".to_owned(),
                ),
            })
            .collect(),
    };
    let candidate_report = HqCandidateReport {
        mode: "prepared-template".to_owned(),
        seed: DEFAULT_QUALITY_SEED,
        proposal_count: 0,
        requested_count: DEFAULT_DIRECTION_COUNT,
        returned_count: 0,
        candidate_survival_count: 0,
        six_direction_availability: false,
        failure: Some(
            "prepared hero template v1 needs authored whole-character preview generation"
                .to_owned(),
        ),
    };
    let export_reopen = HqExportReopenReport {
        export_status: HqEvidenceStatus::Unsupported,
        reopen_status: HqEvidenceStatus::Unsupported,
        package_dir: None,
        manifest: None,
        checksums_match: None,
        topology_matches_manifest: None,
        finite_numeric_payloads: None,
        not_run_reason: Some(
            "prepared hero template v1 does not produce a mesh package yet".to_owned(),
        ),
    };
    let mesh_validity_summary = HqMeshValiditySummary {
        compile_valid: validation_ok,
        model_valid: false,
        issue_count: 1,
        error_count: usize::from(!validation_ok),
        warning_count: usize::from(validation_ok),
        provenance_coverage: 0.0,
        manifold_closed_part_fraction: 0.0,
        accidental_intersection_count: 0,
    };
    let mesh_stats = HqMeshStatsFile {
        profile_id: PREPARED_HERO_TEMPLATE_PROFILE.to_owned(),
        part_count: semantic_part_inventory.part_count,
        polygon_vertex_count: 0,
        polygon_face_count: 0,
        triangle_count: 0,
        triangle_budget: Some(DEFAULT_TRIANGLE_BUDGET),
        used_sdf_or_remeshing: false,
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

    let mut unsupported_outputs = unsupported_outputs();
    unsupported_outputs.extend([
        HqUnsupportedOutput {
            output: "prepared_hero_clay_mesh_preview".to_owned(),
            reason: "prepared hero v1 validates a template contract but has no clay mesh renderer"
                .to_owned(),
        },
        HqUnsupportedOutput {
            output: "prepared_hero_contact_sheet".to_owned(),
            reason: "contact sheets require rendered whole-character previews".to_owned(),
        },
        HqUnsupportedOutput {
            output: "prepared_hero_export_package".to_owned(),
            reason: "export/reopen requires generated mesh artifacts".to_owned(),
        },
        HqUnsupportedOutput {
            output: "arbitrary_mesh_import".to_owned(),
            reason: "prepared hero v1 only accepts known-base authored templates".to_owned(),
        },
        HqUnsupportedOutput {
            output: "dota_ip_reconstruction".to_owned(),
            reason:
                "the template is a generic stylized hero base, not a third-party reconstruction"
                    .to_owned(),
        },
        HqUnsupportedOutput {
            output: "materials_uvs_rigging_animation".to_owned(),
            reason: "materials, UVs, rigging, and animation remain out of current scope".to_owned(),
        },
    ]);

    let gate_evidence = HqQualityGateEvidence {
        compile_valid: validation_ok,
        clay_preview_available: false,
        contact_sheet_available: false,
        front_view_available: false,
        three_quarter_view_available: false,
        side_view_available: false,
        back_view_available: false,
        wireframe_available: false,
        silhouette_available: false,
        visible_control_difference_evidence: false,
        advanced_recipe_required: false,
        export_verified: false,
        reopen_verified: false,
        candidate_survival_count: 0,
        placeholder_thumbnails: false,
        human_approved: args.human_approved,
        adversarial_reviewed: args.adversarial_reviewed,
    };
    let tier = evaluate_quality_tier(args.quality_tier, &gate_evidence);
    let mut blockers = tier.blockers;
    for blocker in [
        "prepared hero template v1 has no clay mesh renderer yet",
        "prepared hero template v1 has no contact sheet evidence yet",
        "prepared hero template v1 has no export/reopen mesh package yet",
    ] {
        if !blockers.iter().any(|existing| existing == blocker) {
            blockers.push(blocker.to_owned());
        }
    }
    if let Some(error) = validation_error {
        blockers.push(format!("prepared hero template validation failed: {error}"));
    }

    let required_role_coverage = HqRequiredRoleCoverage {
        required_role_count: template.provider_slots.len(),
        covered_required_role_count: 0,
        missing_required_roles: template
            .provider_slots
            .iter()
            .map(|slot| slot.label.clone())
            .collect(),
        accepted: false,
    };
    let provider_attachment_validity = HqProviderAttachmentValidity {
        provider_override_count: 0,
        final_conformance_accepted: false,
        required_attachment_issue_count: template.provider_slots.len(),
    };
    let report = HqQualityReport {
        schema_version: HQ_QUALITY_REPORT_SCHEMA_VERSION,
        generated_at: "deterministic-no-wall-clock".to_owned(),
        deterministic_timestamp_policy:
            "quality reports intentionally omit wall-clock timestamps for reproducibility"
                .to_owned(),
        profile_id: PREPARED_HERO_TEMPLATE_PROFILE.to_owned(),
        profile_label: template.display_name,
        kit_id: None,
        style_id: None,
        quality_tier_requested: args.quality_tier,
        quality_tier_achieved: tier.achieved,
        quality_tier_blockers: blockers,
        clay_preview_available: false,
        contact_sheet_available: false,
        front_view_available: false,
        three_quarter_view_available: false,
        side_view_available: false,
        back_view_available: false,
        wireframe_available: false,
        silhouette_available: false,
        silhouette_readability_metric: None,
        silhouette_manual_review_required: true,
        mesh_validity_summary,
        triangle_count: 0,
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
        placeholder_thumbnail_detected: false,
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
        novice_catalog_exposure_allowed_by_default: false,
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
        "fantasy_sword" => "fantasy-sword".to_owned(),
        "round_shield" => "round-shield".to_owned(),
        "hero_helmet" => "hero-helmet".to_owned(),
        "pauldron_pair" => "pauldron-pair".to_owned(),
        "chest_armor" => "chest-armor".to_owned(),
        "prepared_hero_template_v1" | "prepared-hero-template" | "hero-template-v1" => {
            PREPARED_HERO_TEMPLATE_PROFILE.to_owned()
        }
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
        variation_intent: shape_foundry::VariationIntent::default(),
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

fn unsupported_outputs_for_profile(profile_id: &str) -> Vec<HqUnsupportedOutput> {
    let mut outputs = unsupported_outputs();
    if profile_id == MOBA_HERO_CLAY_SLUG {
        outputs.extend([
            HqUnsupportedOutput {
                output: "dota_ip_reconstruction".to_owned(),
                reason:
                    "the clay hero profile is authored original content, not Dota/IP reconstruction"
                        .to_owned(),
            },
            HqUnsupportedOutput {
                output: "arbitrary_mesh_import".to_owned(),
                reason:
                    "arbitrary imported mesh editability is outside the current Shape Lab scope"
                        .to_owned(),
            },
            HqUnsupportedOutput {
                output: "llm_mesh_generation".to_owned(),
                reason: "LLM text-to-geometry generation is not implemented".to_owned(),
            },
        ]);
    }
    outputs
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

    fn sample_quality_report(profile_id: &str, tier: HqQualityTier) -> HqQualityReport {
        HqQualityReport {
            schema_version: HQ_QUALITY_REPORT_SCHEMA_VERSION,
            generated_at: "deterministic".to_owned(),
            deterministic_timestamp_policy: "fixed-test-value".to_owned(),
            profile_id: profile_id.to_owned(),
            profile_label: profile_id.to_owned(),
            kit_id: None,
            style_id: None,
            quality_tier_requested: tier,
            quality_tier_achieved: tier,
            quality_tier_blockers: Vec::new(),
            clay_preview_available: true,
            contact_sheet_available: true,
            front_view_available: true,
            three_quarter_view_available: true,
            side_view_available: true,
            back_view_available: true,
            wireframe_available: true,
            silhouette_available: true,
            silhouette_readability_metric: Some(0.8),
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
            triangle_count: 1024,
            triangle_budget: Some(DEFAULT_TRIANGLE_BUDGET),
            semantic_part_inventory: HqSemanticPartInventory {
                part_count: 1,
                parts: Vec::new(),
            },
            required_role_coverage: HqRequiredRoleCoverage {
                required_role_count: 1,
                covered_required_role_count: 1,
                missing_required_roles: Vec::new(),
                accepted: true,
            },
            provider_attachment_validity: HqProviderAttachmentValidity {
                provider_override_count: 1,
                final_conformance_accepted: true,
                required_attachment_issue_count: 0,
            },
            candidate_survival_count: DEFAULT_DIRECTION_COUNT,
            six_direction_availability: true,
            primary_control_count: 7,
            visible_control_difference_evidence: HqVisibleControlDifferenceEvidence {
                primary_controls_checked: 7,
                changed_control_count: 7,
                controls: Vec::new(),
            },
            advanced_recipe_required: false,
            export_status: HqEvidenceStatus::Verified,
            reopen_status: HqEvidenceStatus::Verified,
            unsupported_outputs: unsupported_outputs(),
            placeholder_thumbnail_detected: false,
            human_review_required: true,
            human_approval_status: HqHumanApprovalStatus::Pending,
            adversarial_review_status: HqHumanApprovalStatus::Pending,
            manual_notes_path: None,
            novice_catalog_exposure_allowed_by_default: false,
        }
    }

    fn write_sample_benchmark(dir: &Path, report: &HqQualityReport) {
        fs::create_dir_all(dir).expect("benchmark dir");
        write_json(dir.join("quality-report.json"), report).expect("quality report");
        for file in [
            "contact-sheet.png",
            "front.png",
            "three-quarter.png",
            "side.png",
            "back.png",
            "wireframe.png",
            "silhouette.png",
            "mesh-stats.json",
            "semantic-parts.json",
            "candidate-report.json",
            "controls-visibility-report.json",
            "export-reopen-report.json",
        ] {
            fs::write(dir.join(file), b"evidence").expect("evidence file");
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
    fn hq_adversarial_review_schema_serializes_required_fields() {
        let report = HqAdversarialReviewReport {
            schema_version: HQ_ADVERSARIAL_REVIEW_SCHEMA_VERSION,
            profile_id: "test-profile".to_owned(),
            reviewed_quality_report: Some("quality-report.json".to_owned()),
            visual_questions: adversarial_visual_questions(),
            mesh_questions: adversarial_mesh_questions(),
            ux_questions: adversarial_ux_questions(),
            blocker_findings: Vec::new(),
            non_blocking_findings: vec!["manual_review_pending".to_owned()],
            tier_recommendation: HqQualityTier::Usable,
            required_followups: Vec::new(),
            human_review_required: true,
            human_reviewer_status: HqHumanApprovalStatus::Pending,
            cannot_automatically_judge_fields: cannot_automatically_judge_fields(),
        };

        let json = serde_json::to_string(&report).expect("serialize adversarial review");
        assert!(json.contains("\"schema_version\""));
        assert!(json.contains("\"reviewed_quality_report\""));
        assert!(json.contains("\"UX_questions\""));
        assert!(json.contains("\"cannot_automatically_judge_fields\""));
    }

    #[test]
    fn hq_adversarial_missing_benchmark_dir_reports_missing_evidence() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let missing = temp_dir.path().join("missing-profile");
        let review = build_hq_adversarial_review(&missing);
        assert_eq!(review.profile_id, "missing-profile");
        assert_eq!(review.tier_recommendation, HqQualityTier::Draft);
        assert!(review.reviewed_quality_report.is_none());
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("missing_benchmark_dir"))
        );
    }

    #[test]
    fn hq_adversarial_missing_quality_report_is_not_passed() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("empty-profile");
        fs::create_dir_all(&benchmark_dir).expect("benchmark dir");
        let review = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(review.tier_recommendation, HqQualityTier::Draft);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("missing_quality_report"))
        );
    }

    #[test]
    fn hq_adversarial_subjective_fields_are_manual_required() {
        let review = build_hq_adversarial_review(Path::new("target/nonexistent-adversarial-test"));
        assert!(review.human_review_required);
        assert!(!review.cannot_automatically_judge_fields.is_empty());
        assert!(
            review
                .visual_questions
                .iter()
                .chain(review.mesh_questions.iter())
                .chain(review.ux_questions.iter())
                .all(|question| question.manual_review_required)
        );
    }

    #[test]
    fn hq_adversarial_tier_downgrades_usable_with_advanced_recipe() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("advanced-profile");
        let mut quality = sample_quality_report("advanced-profile", HqQualityTier::Usable);
        quality.advanced_recipe_required = true;
        write_sample_benchmark(&benchmark_dir, &quality);

        let review = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(review.tier_recommendation, HqQualityTier::Prototype);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("advanced_recipe"))
        );
    }

    #[test]
    fn hq_adversarial_report_blockers_lower_claimed_tier() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("blocked-usable-profile");
        let mut quality = sample_quality_report("blocked-usable-profile", HqQualityTier::Usable);
        quality
            .quality_tier_blockers
            .push("stale blocker should not keep usable".to_owned());
        write_sample_benchmark(&benchmark_dir, &quality);

        let review = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(review.tier_recommendation, HqQualityTier::Prototype);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("report_blockers"))
        );
    }

    #[test]
    fn hq_adversarial_visibility_policy_uses_downgraded_tier() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("downgraded-visible-profile");
        let mut quality =
            sample_quality_report("downgraded-visible-profile", HqQualityTier::Usable);
        quality.advanced_recipe_required = true;
        quality.novice_catalog_exposure_allowed_by_default = true;
        write_sample_benchmark(&benchmark_dir, &quality);

        let review = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(review.tier_recommendation, HqQualityTier::Prototype);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("visibility_policy"))
        );
    }

    #[test]
    fn hq_adversarial_showcase_requires_human_approval() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("showcase-profile");
        let quality = sample_quality_report("showcase-profile", HqQualityTier::Showcase);
        write_sample_benchmark(&benchmark_dir, &quality);

        let review = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(review.tier_recommendation, HqQualityTier::Usable);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("showcase_review"))
        );
    }

    #[test]
    fn hq_adversarial_draft_and_prototype_stay_hidden_from_novice_catalog() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("prototype-profile");
        let mut quality = sample_quality_report("prototype-profile", HqQualityTier::Prototype);
        quality.novice_catalog_exposure_allowed_by_default = true;
        write_sample_benchmark(&benchmark_dir, &quality);

        let review = build_hq_adversarial_review(&benchmark_dir);
        assert!(
            review
                .blocker_findings
                .iter()
                .any(|finding| finding.contains("Draft and Prototype"))
        );
    }

    #[test]
    fn hq_adversarial_review_results_are_deterministic() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let benchmark_dir = temp_dir.path().join("deterministic-profile");
        let quality = sample_quality_report("deterministic-profile", HqQualityTier::Usable);
        write_sample_benchmark(&benchmark_dir, &quality);

        let first = build_hq_adversarial_review(&benchmark_dir);
        let second = build_hq_adversarial_review(&benchmark_dir);
        assert_eq!(first, second);
    }

    #[test]
    fn hq_adversarial_public_product_strings_do_not_claim_dota_or_ip_output() {
        for text in [
            include_str!("../../../docs/PRODUCT_POSITIONING_BOUNDARY.md"),
            include_str!("../../../docs/MOBA_HERO_FOUNDRY_MVP.md"),
            include_str!("../../../docs/WAVE40_MOBA_HERO_FOUNDRY_REPORT.md"),
        ] {
            let lower = text.to_ascii_lowercase();
            for forbidden_claim in [
                "can create dota",
                "dota reconstruction is supported",
                "ip reconstruction is supported",
                "public dota claim",
            ] {
                assert!(
                    !lower.contains(forbidden_claim),
                    "forbidden public output claim found: {forbidden_claim}"
                );
            }
        }
    }

    #[test]
    fn hq_quality_builtin_profile_list_is_enumerable() {
        let slugs = benchmark_profile_slugs();
        assert_eq!(slugs.len(), 18);
        assert!(slugs.contains(&"roman-bridge".to_owned()));
        assert!(slugs.contains(&"roman-bridge-hq".to_owned()));
        assert!(slugs.contains(&"stylized-tree".to_owned()));
        assert!(slugs.contains(&"fantasy-sword".to_owned()));
        assert!(slugs.contains(&"chest-armor".to_owned()));
        assert!(slugs.contains(&MOBA_HERO_CLAY_SLUG.to_owned()));
        assert!(slugs.contains(&PREPARED_HERO_TEMPLATE_PROFILE.to_owned()));
    }

    #[test]
    fn hq_quality_prepared_hero_template_profile_emits_honest_unsupported_report() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let args = HqQualityBenchmarkArgs {
            profile: PREPARED_HERO_TEMPLATE_PROFILE.to_owned(),
            out_dir: temp_dir.path().to_path_buf(),
            quality_tier: HqQualityTier::Usable,
            verify_export: true,
            human_approved: false,
            adversarial_reviewed: false,
            manual_notes: None,
            json: false,
        };

        run_hq_quality_benchmark(args).expect("prepared hero benchmark writes report");

        let report_path = temp_dir.path().join("quality-report.json");
        let report = serde_json::from_str::<HqQualityReport>(
            &fs::read_to_string(&report_path).expect("quality report exists"),
        )
        .expect("quality report json");
        assert_eq!(report.profile_id, PREPARED_HERO_TEMPLATE_PROFILE);
        assert_eq!(report.quality_tier_requested, HqQualityTier::Usable);
        assert_eq!(report.quality_tier_achieved, HqQualityTier::Draft);
        assert!(!report.clay_preview_available);
        assert!(!report.contact_sheet_available);
        assert_eq!(report.export_status, HqEvidenceStatus::Unsupported);
        assert_eq!(report.reopen_status, HqEvidenceStatus::Unsupported);
        assert_eq!(report.triangle_count, 0);
        assert_eq!(report.primary_control_count, 6);
        assert!(report.mesh_validity_summary.compile_valid);
        assert!(!report.mesh_validity_summary.model_valid);
        assert!(!report.novice_catalog_exposure_allowed_by_default);
        assert!(
            report
                .quality_tier_blockers
                .iter()
                .any(|blocker| { blocker.contains("no clay mesh renderer") })
        );
        for unsupported in [
            "prepared_hero_clay_mesh_preview",
            "prepared_hero_contact_sheet",
            "prepared_hero_export_package",
            "arbitrary_mesh_import",
            "dota_ip_reconstruction",
            "materials_uvs_rigging_animation",
        ] {
            assert!(
                report
                    .unsupported_outputs
                    .iter()
                    .any(|output| output.output == unsupported),
                "missing unsupported output {unsupported}"
            );
        }
        for name in [
            "prepared-template-contract.json",
            "mesh-stats.json",
            "semantic-parts.json",
            "candidate-report.json",
            "controls-visibility-report.json",
            "export-reopen-report.json",
            "quality-report.json",
        ] {
            let path = temp_dir.path().join(name);
            assert!(path.exists(), "prepared hero benchmark should write {name}");
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "prepared hero benchmark {name} is empty"
            );
        }
    }

    #[test]
    fn moba_hero_pack_report_reopens_serialized_pack_and_exports_members() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let fixture = shape_foundry_catalog::moba_hero::fixture_catalog();
        let report =
            moba_hero_pack_report(&fixture, temp_dir.path()).expect("moba hero pack evidence");

        assert_eq!(report.schema_version, 2);
        assert_eq!(report.profile_id, MOBA_HERO_CLAY_SLUG);
        assert_eq!(report.source_template_id, PREPARED_HERO_TEMPLATE_PROFILE);
        assert!(!report.source_template_fingerprint.is_empty());
        assert!(!report.pack_report_fingerprint.is_empty());
        assert_eq!(report.members.len(), 3);
        assert_eq!(report.exported_member_package_count, 3);
        assert_eq!(report.export_status, HqEvidenceStatus::Verified);
        assert_eq!(report.reopen_status, HqEvidenceStatus::Verified);
        assert_eq!(report.semantic_part_inventory.part_count, 51);

        for name in [
            "hero-pack-model-package/pack-document.json",
            "hero-pack-model-package/pack-report.json",
            "hero-pack-model-package/duelist-vanguard/asset-manifest.json",
            "hero-pack-model-package/arcane-ranger/asset-manifest.json",
            "hero-pack-model-package/monster-hunter/asset-manifest.json",
        ] {
            let path = temp_dir.path().join(name);
            assert!(path.exists(), "moba hero pack evidence should write {name}");
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "moba hero pack evidence {name} is empty"
            );
        }
    }

    #[test]
    #[ignore = "Wave 40 hero benchmark compiles candidates, mode sheets, and pack member exports"]
    fn moba_hero_hq_benchmark_emits_pack_and_boundary_evidence() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let args = HqQualityBenchmarkArgs {
            profile: MOBA_HERO_CLAY_SLUG.to_owned(),
            out_dir: temp_dir.path().to_path_buf(),
            quality_tier: HqQualityTier::Usable,
            verify_export: false,
            human_approved: false,
            adversarial_reviewed: false,
            manual_notes: None,
            json: false,
        };

        run_hq_quality_benchmark(args).expect("moba hero benchmark writes report");

        let report = serde_json::from_str::<HqQualityReport>(
            &fs::read_to_string(temp_dir.path().join("quality-report.json"))
                .expect("quality report exists"),
        )
        .expect("quality report json");
        assert_eq!(report.profile_id, MOBA_HERO_CLAY_SLUG);
        assert_eq!(report.quality_tier_achieved, HqQualityTier::Usable);
        assert!(report.quality_tier_blockers.is_empty());
        assert_eq!(report.primary_control_count, 7);
        assert_eq!(
            report
                .visible_control_difference_evidence
                .changed_control_count,
            7
        );
        assert_eq!(report.candidate_survival_count, DEFAULT_DIRECTION_COUNT);
        assert!(report.six_direction_availability);
        assert_eq!(report.export_status, HqEvidenceStatus::Verified);
        assert_eq!(report.reopen_status, HqEvidenceStatus::Verified);
        assert!(!report.novice_catalog_exposure_allowed_by_default);
        for unsupported in [
            "dota_ip_reconstruction",
            "arbitrary_mesh_import",
            "llm_mesh_generation",
            "materials",
            "textures",
            "uv_layout",
            "rigging",
            "animation",
            "marketplace_package",
        ] {
            assert!(
                report
                    .unsupported_outputs
                    .iter()
                    .any(|output| output.output == unsupported),
                "missing unsupported output {unsupported}"
            );
        }

        let hero_pack = serde_json::from_str::<HqHeroPackReport>(
            &fs::read_to_string(temp_dir.path().join("hero-pack-report.json"))
                .expect("hero pack report exists"),
        )
        .expect("hero pack report json");
        assert_eq!(hero_pack.schema_version, 2);
        assert_eq!(hero_pack.source_template_id, PREPARED_HERO_TEMPLATE_PROFILE);
        assert!(!hero_pack.source_template_fingerprint.is_empty());
        assert!(!hero_pack.pack_report_fingerprint.is_empty());
        assert_eq!(hero_pack.members.len(), 3);
        assert_eq!(hero_pack.exported_member_package_count, 3);
        assert_eq!(hero_pack.export_status, HqEvidenceStatus::Verified);
        assert_eq!(hero_pack.reopen_status, HqEvidenceStatus::Verified);
        assert_eq!(hero_pack.semantic_part_inventory.part_count, 51);
        assert!(temp_dir.path().join("hero-pack-model-package").exists());

        for name in [
            "explore-contact-sheet.png",
            "silhouette-contact-sheet.png",
            "gear-contact-sheet.png",
            "hero-pack-report.json",
            "hero-pack-model-package/pack-document.json",
            "hero-pack-model-package/pack-report.json",
            "hero-pack-model-package/duelist-vanguard/asset-manifest.json",
            "hero-pack-model-package/arcane-ranger/asset-manifest.json",
            "hero-pack-model-package/monster-hunter/asset-manifest.json",
        ] {
            let path = temp_dir.path().join(name);
            assert!(path.exists(), "moba hero benchmark should write {name}");
            assert!(
                path.metadata().expect("metadata").len() > 0,
                "moba hero benchmark {name} is empty"
            );
        }
    }

    #[test]
    fn roman_bridge_hq_explore_candidates_survive_quality_validation() {
        let fixture = shape_foundry_catalog::roman_bridge::hq_fixture_catalog();
        assert_all_explore_candidates_survive("roman-bridge-hq", fixture);
    }

    #[test]
    fn fantasy_sword_explore_candidates_survive_quality_validation() {
        let fixture = shape_foundry_catalog::showcase_gear::fantasy_sword_fixture_catalog();
        assert_all_explore_candidates_survive("fantasy-sword", fixture);
    }

    #[test]
    fn moba_hero_explore_candidates_survive_quality_validation() {
        let fixture = shape_foundry_catalog::moba_hero::fixture_catalog();
        assert_all_explore_candidates_survive("moba-hero-clay", fixture);
    }

    #[test]
    fn showcase_gear_defaults_pass_model_validation() {
        let mut failures = Vec::new();
        for (slug, fixture) in showcase_gear_fixtures() {
            let compiled = compile_foundry_document(&fixture.document, &fixture)
                .unwrap_or_else(|error| panic!("{slug} should compile: {error:#?}"));
            let model_validation = validate_compiled_output(&compiled);
            if !model_validation.is_valid() {
                failures.push(format!("{slug} issues={:#?}", model_validation.issues));
            }
        }
        assert!(
            failures.is_empty(),
            "showcase gear default validation failures:\n{}",
            failures.join("\n")
        );
    }

    #[test]
    fn showcase_gear_hq_benchmarks_reach_usable_with_export_evidence() {
        let temp_dir = tempfile::tempdir().expect("temp dir");
        for (slug, fixture) in showcase_gear_fixtures() {
            let out_dir = temp_dir.path().join(slug);
            fs::create_dir_all(&out_dir).expect("create benchmark output dir");
            let args = HqQualityBenchmarkArgs {
                profile: slug.to_owned(),
                out_dir: out_dir.clone(),
                quality_tier: HqQualityTier::Usable,
                verify_export: false,
                human_approved: false,
                adversarial_reviewed: false,
                manual_notes: None,
                json: false,
            };
            let report = benchmark_one_profile(&args, slug, fixture, &out_dir)
                .unwrap_or_else(|error| panic!("{slug} HQ benchmark should pass: {error:#?}"));
            assert_eq!(report.quality_tier_achieved, HqQualityTier::Usable);
            assert!(report.quality_tier_blockers.is_empty());
            assert_eq!(report.candidate_survival_count, DEFAULT_DIRECTION_COUNT);
            assert!(report.six_direction_availability);
            assert_eq!(report.export_status, HqEvidenceStatus::Verified);
            assert_eq!(report.reopen_status, HqEvidenceStatus::Verified);
            assert!(report.mesh_validity_summary.model_valid);
            assert_eq!(
                report
                    .visible_control_difference_evidence
                    .changed_control_count,
                report.primary_control_count
            );
            assert!(!report.placeholder_thumbnail_detected);
            for name in [
                "contact-sheet.png",
                "front.png",
                "three-quarter.png",
                "side.png",
                "back.png",
                "wireframe.png",
                "silhouette.png",
                "mesh-stats.json",
                "semantic-parts.json",
                "candidate-report.json",
                "controls-visibility-report.json",
                "export-reopen-report.json",
                "quality-report.json",
            ] {
                let path = out_dir.join(name);
                assert!(path.exists(), "{slug} HQ benchmark should write {name}");
                assert!(
                    path.metadata().expect("metadata").len() > 0,
                    "{slug} HQ benchmark {name} is empty"
                );
            }
        }
    }

    fn showcase_gear_fixtures() -> [(&'static str, shape_foundry_catalog::FoundryFixtureCatalog); 5]
    {
        [
            (
                "fantasy-sword",
                shape_foundry_catalog::showcase_gear::fantasy_sword_fixture_catalog(),
            ),
            (
                "round-shield",
                shape_foundry_catalog::showcase_gear::round_shield_fixture_catalog(),
            ),
            (
                "hero-helmet",
                shape_foundry_catalog::showcase_gear::hero_helmet_fixture_catalog(),
            ),
            (
                "pauldron-pair",
                shape_foundry_catalog::showcase_gear::pauldron_pair_fixture_catalog(),
            ),
            (
                "chest-armor",
                shape_foundry_catalog::showcase_gear::chest_armor_fixture_catalog(),
            ),
        ]
    }

    fn assert_all_explore_candidates_survive(
        slug: &str,
        fixture: shape_foundry_catalog::FoundryFixtureCatalog,
    ) {
        let request = FoundryCandidateRequest {
            seed: DEFAULT_QUALITY_SEED,
            proposal_count: DEFAULT_PROPOSAL_COUNT,
            result_count: DEFAULT_DIRECTION_COUNT,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: shape_foundry::VariationIntent::default(),
        };
        let output = generate_foundry_candidate_plans(&fixture.document, &fixture, &request)
            .unwrap_or_else(|error| panic!("{slug} candidates should generate: {error:#?}"));
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
            "{slug} candidate failures:\n{}",
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
