use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use shape_foundry::{
    ObjectPlan, ObjectPlanReviewTier, ObjectPlanValidationReport, PrimitiveKind,
    object_plan_user_summary, validate_object_plan,
};

use crate::write_json;

/// Validate and inspect structured offline ObjectPlans.
#[derive(Debug, clap::Args)]
pub struct ObjectPlanArgs {
    /// ObjectPlan operation.
    #[command(subcommand)]
    pub command: ObjectPlanCommand,
}

/// ObjectPlan CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum ObjectPlanCommand {
    /// Validate one ObjectPlan JSON file.
    Validate {
        /// ObjectPlan JSON file.
        plan: PathBuf,
    },
    /// Validate and prepare deterministic offline render artifacts.
    Render {
        /// ObjectPlan JSON file.
        #[arg(long)]
        plan: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Validate an ObjectPlan and optionally request contact-sheet evidence.
    Run {
        /// ObjectPlan JSON file.
        #[arg(long)]
        plan: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Request contact-sheet evidence when renderable bindings exist.
        #[arg(long)]
        contact_sheet: bool,
    },
    /// Run a directory or batch JSON of ObjectPlans for offline review.
    BatchRun {
        /// Directory of ObjectPlan JSON files or an ObjectPlanBatch JSON file.
        #[arg(long)]
        input: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// Run an ObjectPlan CLI command.
pub fn run_object_plan(args: ObjectPlanArgs) -> anyhow::Result<()> {
    match args.command {
        ObjectPlanCommand::Validate { plan } => run_validate(&plan),
        ObjectPlanCommand::Render { plan, out_dir } => run_render(&plan, &out_dir),
        ObjectPlanCommand::Run {
            plan,
            out_dir,
            contact_sheet,
        } => run_prepare(&plan, &out_dir, contact_sheet),
        ObjectPlanCommand::BatchRun { input, out_dir } => run_batch(&input, &out_dir),
    }
}

fn run_validate(plan_path: &Path) -> anyhow::Result<()> {
    let plan = read_object_plan(plan_path)?;
    let report = validate_object_plan(&plan);
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.is_valid() {
        Ok(())
    } else {
        anyhow::bail!(
            "ObjectPlan validation failed with {} issue(s)",
            report.issues.len()
        )
    }
}

fn run_render(plan_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    run_prepare(plan_path, out_dir, false)
}

fn run_prepare(
    plan_path: &Path,
    out_dir: &Path,
    contact_sheet_requested: bool,
) -> anyhow::Result<()> {
    let outcome = write_plan_outputs(plan_path, out_dir, contact_sheet_requested)?;
    if outcome.validation_passed {
        println!(
            "Validated ObjectPlan {} into {}",
            outcome.plan_id.as_deref().unwrap_or("unknown-plan"),
            out_dir.display()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "ObjectPlan validation failed with {} issue(s)",
            outcome.validation_issue_count
        )
    }
}

fn write_plan_outputs(
    plan_path: &Path,
    out_dir: &Path,
    contact_sheet_requested: bool,
) -> anyhow::Result<ObjectPlanRunOutcome> {
    let plan = read_object_plan(plan_path)?;
    let report = validate_object_plan(&plan);
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let user_summary = object_plan_user_summary(&plan);
    let primitive_summary = primitive_summary(&plan, &report);
    let renderability_report = object_plan_renderability_report(&plan, &report);
    let visual_evidence_report = ObjectPlanVisualEvidenceReport::from_renderability(
        &renderability_report,
        contact_sheet_requested,
    );
    let rendering_report = ObjectPlanRenderingReport::from_reports(
        &renderability_report,
        &visual_evidence_report,
        report.is_valid(),
    );

    write_json(out_dir.join("validation-report.json"), &report)?;
    write_json(out_dir.join("primitive-summary.json"), &primitive_summary)?;
    write_json(out_dir.join("normalized-object-plan.json"), &plan)?;
    write_json(
        out_dir.join("renderability-report.json"),
        &renderability_report,
    )?;
    if contact_sheet_requested {
        write_json(
            out_dir.join("visual-evidence-report.json"),
            &visual_evidence_report,
        )?;
    }
    write_json(out_dir.join("rendering-report.json"), &rendering_report)?;
    fs::write(
        out_dir.join("plan-user-summary.md"),
        user_summary_markdown(&user_summary, &rendering_report),
    )
    .with_context(|| format!("writing {}", out_dir.join("plan-user-summary.md").display()))?;

    Ok(ObjectPlanRunOutcome {
        plan_id: Some(plan.plan_id),
        display_name: Some(plan.display_name),
        validation_passed: report.is_valid(),
        validation_issue_count: report.issues.len(),
        rendered: visual_evidence_report.rendered,
        renderable: renderability_report.renderable,
    })
}

fn object_plan_renderability_report(
    plan: &ObjectPlan,
    validation_report: &ObjectPlanValidationReport,
) -> ObjectPlanRenderabilityReport {
    let unsupported_primitives = plan
        .nodes
        .iter()
        .filter(|node| matches!(node.primitive_kind, PrimitiveKind::CylinderPrimitive))
        .map(|node| format!("{}: unsupported primitive kind", node.node_id))
        .collect::<Vec<_>>();
    if !validation_report.is_valid() {
        return ObjectPlanRenderabilityReport {
            plan_id: plan.plan_id.clone(),
            renderable: false,
            unsupported_primitives,
            unsupported_attachments: Vec::new(),
            missing_preview_bindings: Vec::new(),
            reason: "Validation failed before rendering.".to_owned(),
        };
    }

    ObjectPlanRenderabilityReport {
        plan_id: plan.plan_id.clone(),
        renderable: false,
        unsupported_primitives,
        unsupported_attachments: Vec::new(),
        missing_preview_bindings: plan
            .nodes
            .iter()
            .map(|node| {
                format!(
                    "{}: ObjectPlan preview materialization is not wired yet.",
                    node.node_id
                )
            })
            .collect(),
        reason: "ObjectPlan rendering is blocked until plan materialization is implemented."
            .to_owned(),
    }
}

fn run_batch(input_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let batch = resolve_batch_input(input_path)?;
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let plans_dir = out_dir.join("plans");
    fs::create_dir_all(&plans_dir).with_context(|| format!("creating {}", plans_dir.display()))?;

    let mut plan_reports = Vec::new();
    for (index, plan_input) in batch.plans.iter().enumerate() {
        let slug = unique_plan_slug(index, &plan_input.source_ref);
        let relative_out_dir = format!("plans/{slug}");
        let plan_out_dir = out_dir.join(&relative_out_dir);
        let outcome = write_plan_outputs(
            &plan_input.path,
            &plan_out_dir,
            batch.output_policy.contact_sheet,
        );
        plan_reports.push(plan_report_from_outcome(
            plan_input,
            &relative_out_dir,
            outcome,
        ));
    }

    let passed_validation = plan_reports
        .iter()
        .filter(|plan| plan.validation_status == "Passed")
        .count();
    let failed_validation = plan_reports
        .iter()
        .filter(|plan| plan.validation_status == "Failed")
        .count();
    let rendered = plan_reports.iter().filter(|plan| plan.rendered).count();
    let unsupported = plan_reports.iter().filter(|plan| plan.unsupported).count();
    let _ = batch.review_policy.human_review_required;
    let report = ObjectPlanBatchValidationReport {
        batch_id: batch.batch_id,
        display_name: batch.display_name,
        total_plans: plan_reports.len(),
        passed_validation,
        failed_validation,
        rendered,
        unsupported,
        human_review_required: true,
        approved: false,
        plans: plan_reports,
    };

    write_json(out_dir.join("batch-validation-report.json"), &report)?;
    fs::write(
        out_dir.join("keep-regenerate-simplify.md"),
        keep_regenerate_simplify_markdown(&report),
    )
    .with_context(|| {
        format!(
            "writing {}",
            out_dir.join("keep-regenerate-simplify.md").display()
        )
    })?;
    fs::write(
        out_dir.join("batch-user-summary.md"),
        batch_user_summary_markdown(&report),
    )
    .with_context(|| {
        format!(
            "writing {}",
            out_dir.join("batch-user-summary.md").display()
        )
    })?;

    println!(
        "Ran ObjectPlan batch {} with {} plan(s) into {}",
        report.batch_id,
        report.total_plans,
        out_dir.display()
    );
    Ok(())
}

fn resolve_batch_input(input_path: &Path) -> anyhow::Result<ObjectPlanBatchInput> {
    if input_path.is_dir() {
        let mut paths = fs::read_dir(input_path)
            .with_context(|| format!("reading {}", input_path.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .with_context(|| format!("reading {}", input_path.display()))?;
        paths.retain(|path| {
            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        });
        paths.sort();
        let plans = paths
            .into_iter()
            .map(|path| {
                let source_ref = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("object-plan.json")
                    .to_owned();
                ObjectPlanBatchPlanInput { source_ref, path }
            })
            .collect::<Vec<_>>();
        return Ok(ObjectPlanBatchInput {
            batch_id: input_path
                .file_name()
                .and_then(|name| name.to_str())
                .map(safe_identifier)
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| "object_plan_batch".to_owned()),
            display_name: "ObjectPlan Batch".to_owned(),
            review_policy: ObjectPlanBatchReviewPolicy::default(),
            output_policy: ObjectPlanBatchOutputPolicy::default(),
            plans,
        });
    }

    let bytes =
        fs::read(input_path).with_context(|| format!("reading {}", input_path.display()))?;
    let batch: ObjectPlanBatch =
        serde_json::from_slice(&bytes).with_context(|| "parsing ObjectPlanBatch JSON")?;
    let base_dir = input_path.parent().unwrap_or_else(|| Path::new("."));
    let plans = batch
        .plans
        .iter()
        .map(|plan_ref| {
            let raw_path = PathBuf::from(plan_ref);
            let path = if raw_path.is_absolute() {
                raw_path.clone()
            } else {
                base_dir.join(&raw_path)
            };
            ObjectPlanBatchPlanInput {
                source_ref: persisted_source_ref(plan_ref, &raw_path),
                path,
            }
        })
        .collect::<Vec<_>>();
    Ok(ObjectPlanBatchInput {
        batch_id: safe_identifier(&batch.batch_id),
        display_name: batch.display_name,
        review_policy: batch.review_policy,
        output_policy: batch.output_policy,
        plans,
    })
}

fn plan_report_from_outcome(
    plan_input: &ObjectPlanBatchPlanInput,
    relative_out_dir: &str,
    outcome: anyhow::Result<ObjectPlanRunOutcome>,
) -> ObjectPlanBatchPlanReport {
    match outcome {
        Ok(outcome) => {
            let recommendation = if outcome.validation_passed && outcome.rendered {
                BatchReviewRecommendation::Keep
            } else if outcome.validation_passed {
                BatchReviewRecommendation::Simplify
            } else {
                BatchReviewRecommendation::Regenerate
            };
            ObjectPlanBatchPlanReport {
                source_ref: plan_input.source_ref.clone(),
                output_dir: relative_out_dir.to_owned(),
                plan_id: outcome.plan_id,
                display_name: outcome.display_name,
                validation_status: if outcome.validation_passed {
                    "Passed".to_owned()
                } else {
                    "Failed".to_owned()
                },
                validation_issue_count: outcome.validation_issue_count,
                rendered: outcome.rendered,
                unsupported: !outcome.renderable,
                recommendation,
                errors: Vec::new(),
            }
        }
        Err(_error) => ObjectPlanBatchPlanReport {
            source_ref: plan_input.source_ref.clone(),
            output_dir: relative_out_dir.to_owned(),
            plan_id: None,
            display_name: None,
            validation_status: "Failed".to_owned(),
            validation_issue_count: 1,
            rendered: false,
            unsupported: true,
            recommendation: BatchReviewRecommendation::Blocked,
            errors: vec!["Plan could not be read or parsed.".to_owned()],
        },
    }
}

fn keep_regenerate_simplify_markdown(report: &ObjectPlanBatchValidationReport) -> String {
    let mut markdown = "# Keep / Regenerate / Simplify\n\n".to_owned();
    markdown.push_str("Recommendations are review labels only. They do not publish plans.\n\n");
    for plan in &report.plans {
        markdown.push_str("- ");
        markdown.push_str(&plan.source_ref);
        markdown.push_str(": ");
        markdown.push_str(plan.recommendation.label());
        markdown.push_str(" - ");
        markdown.push_str(recommendation_reason(plan));
        markdown.push('\n');
    }
    markdown
}

fn batch_user_summary_markdown(report: &ObjectPlanBatchValidationReport) -> String {
    format!(
        "# {}\n\nTotal plans: {}\n\nPassed validation: {}\n\nFailed validation: {}\n\nRendered: {}\n\nHuman review required: true\n\nApproved: false\n",
        report.display_name,
        report.total_plans,
        report.passed_validation,
        report.failed_validation,
        report.rendered
    )
}

fn recommendation_reason(plan: &ObjectPlanBatchPlanReport) -> &'static str {
    match plan.recommendation {
        BatchReviewRecommendation::Keep => "rendered evidence is available for review",
        BatchReviewRecommendation::Regenerate => "validation failed",
        BatchReviewRecommendation::Simplify => "validation passed but preview output is blocked",
        BatchReviewRecommendation::Blocked => "the plan could not be read",
    }
}

fn unique_plan_slug(index: usize, source_ref: &str) -> String {
    let stem = Path::new(source_ref)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(safe_identifier)
        .filter(|stem| !stem.is_empty())
        .unwrap_or_else(|| "object_plan".to_owned());
    format!("{index:03}-{stem}")
}

fn persisted_source_ref(plan_ref: &str, path: &Path) -> String {
    if path.is_absolute() {
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("object-plan.json")
            .to_owned()
    } else {
        plan_ref.replace('\\', "/")
    }
}

fn safe_identifier(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else if ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

fn read_object_plan(path: &Path) -> anyhow::Result<ObjectPlan> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing ObjectPlan {}", path.display()))
}

fn primitive_summary(
    plan: &ObjectPlan,
    report: &ObjectPlanValidationReport,
) -> ObjectPlanPrimitiveSummary {
    ObjectPlanPrimitiveSummary {
        plan_id: plan.plan_id.clone(),
        display_name: plan.display_name.clone(),
        valid: report.is_valid(),
        issue_count: report.issues.len(),
        review_tier: plan.review_tier,
        primitives: plan
            .nodes
            .iter()
            .map(|node| ObjectPlanPrimitiveSummaryNode {
                node_id: node.node_id.clone(),
                display_name: node.display_name.clone(),
                primitive_kind: node.primitive_kind,
                property_count: node.property_values.len(),
                locked: node.locked,
            })
            .collect(),
        attachments: plan
            .attachments
            .iter()
            .map(|attachment| ObjectPlanAttachmentSummary {
                attachment_id: attachment.attachment_id.clone(),
                parent_node_id: attachment.parent_node_id.clone(),
                parent_anchor_id: attachment.parent_anchor_id.clone(),
                child_node_id: attachment.child_node_id.clone(),
                child_anchor_id: attachment.child_anchor_id.clone(),
            })
            .collect(),
    }
}

fn user_summary_markdown(
    summary: &shape_foundry::ObjectPlanUserSummary,
    rendering_report: &ObjectPlanRenderingReport,
) -> String {
    let mut markdown = String::new();
    markdown.push_str("# ");
    markdown.push_str(&summary.display_name);
    markdown.push_str("\n\n");
    markdown.push_str(&summary.intent_summary);
    markdown.push_str("\n\n");
    markdown.push_str("Review: ");
    markdown.push_str(&summary.review_summary);
    markdown.push_str("\n\n## Primitives\n\n");
    for primitive in &summary.primitives_used {
        markdown.push_str("- ");
        markdown.push_str(primitive);
        markdown.push('\n');
    }
    markdown.push_str("\n## Adjustable Properties\n\n");
    for property in &summary.adjustable_properties {
        markdown.push_str("- ");
        markdown.push_str(property);
        markdown.push('\n');
    }
    markdown.push_str("\n## Attachments\n\n");
    if summary.attachments.is_empty() {
        markdown.push_str("- No attachments.\n");
    } else {
        for attachment in &summary.attachments {
            markdown.push_str("- ");
            markdown.push_str(attachment);
            markdown.push('\n');
        }
    }
    markdown.push_str("\n## Rendering\n\n");
    markdown.push_str("- Status: ");
    markdown.push_str(&rendering_report.status);
    markdown.push('\n');
    markdown.push_str("- Reason: ");
    markdown.push_str(&rendering_report.reason);
    markdown.push('\n');
    markdown
}

#[derive(Debug, Deserialize)]
struct ObjectPlanBatch {
    batch_id: String,
    display_name: String,
    plans: Vec<String>,
    #[serde(default)]
    review_policy: ObjectPlanBatchReviewPolicy,
    #[serde(default)]
    output_policy: ObjectPlanBatchOutputPolicy,
}

#[derive(Debug, Deserialize)]
struct ObjectPlanBatchReviewPolicy {
    #[serde(default = "default_true")]
    human_review_required: bool,
}

impl Default for ObjectPlanBatchReviewPolicy {
    fn default() -> Self {
        Self {
            human_review_required: true,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ObjectPlanBatchOutputPolicy {
    #[serde(default = "default_true")]
    contact_sheet: bool,
}

impl Default for ObjectPlanBatchOutputPolicy {
    fn default() -> Self {
        Self {
            contact_sheet: true,
        }
    }
}

#[derive(Debug)]
struct ObjectPlanBatchInput {
    batch_id: String,
    display_name: String,
    review_policy: ObjectPlanBatchReviewPolicy,
    output_policy: ObjectPlanBatchOutputPolicy,
    plans: Vec<ObjectPlanBatchPlanInput>,
}

#[derive(Debug)]
struct ObjectPlanBatchPlanInput {
    source_ref: String,
    path: PathBuf,
}

#[derive(Debug)]
struct ObjectPlanRunOutcome {
    plan_id: Option<String>,
    display_name: Option<String>,
    validation_passed: bool,
    validation_issue_count: usize,
    rendered: bool,
    renderable: bool,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchValidationReport {
    batch_id: String,
    display_name: String,
    total_plans: usize,
    passed_validation: usize,
    failed_validation: usize,
    rendered: usize,
    unsupported: usize,
    human_review_required: bool,
    approved: bool,
    plans: Vec<ObjectPlanBatchPlanReport>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchPlanReport {
    source_ref: String,
    output_dir: String,
    plan_id: Option<String>,
    display_name: Option<String>,
    validation_status: String,
    validation_issue_count: usize,
    rendered: bool,
    unsupported: bool,
    recommendation: BatchReviewRecommendation,
    errors: Vec<String>,
}

#[derive(Debug, Copy, Clone, Serialize)]
enum BatchReviewRecommendation {
    Keep,
    Regenerate,
    Simplify,
    Blocked,
}

impl BatchReviewRecommendation {
    fn label(self) -> &'static str {
        match self {
            Self::Keep => "Keep",
            Self::Regenerate => "Regenerate",
            Self::Simplify => "Simplify",
            Self::Blocked => "Blocked",
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct ObjectPlanPrimitiveSummary {
    plan_id: String,
    display_name: String,
    valid: bool,
    issue_count: usize,
    review_tier: ObjectPlanReviewTier,
    primitives: Vec<ObjectPlanPrimitiveSummaryNode>,
    attachments: Vec<ObjectPlanAttachmentSummary>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanPrimitiveSummaryNode {
    node_id: String,
    display_name: String,
    primitive_kind: PrimitiveKind,
    property_count: usize,
    locked: bool,
}

#[derive(Debug, Serialize)]
struct ObjectPlanAttachmentSummary {
    attachment_id: String,
    parent_node_id: String,
    parent_anchor_id: String,
    child_node_id: String,
    child_anchor_id: String,
}

#[derive(Debug, Serialize)]
struct ObjectPlanRenderabilityReport {
    plan_id: String,
    renderable: bool,
    unsupported_primitives: Vec<String>,
    unsupported_attachments: Vec<String>,
    missing_preview_bindings: Vec<String>,
    reason: String,
}

#[derive(Debug, Serialize)]
struct ObjectPlanVisualEvidenceReport {
    plan_id: String,
    rendered: bool,
    preview_count: usize,
    contact_sheet_path: Option<String>,
    warnings: Vec<String>,
    user_review_required: bool,
    approved: bool,
}

impl ObjectPlanVisualEvidenceReport {
    fn from_renderability(
        renderability_report: &ObjectPlanRenderabilityReport,
        contact_sheet_requested: bool,
    ) -> Self {
        let warnings = if contact_sheet_requested && !renderability_report.renderable {
            vec![renderability_report.reason.clone()]
        } else {
            Vec::new()
        };
        Self {
            plan_id: renderability_report.plan_id.clone(),
            rendered: false,
            preview_count: 0,
            contact_sheet_path: None,
            warnings,
            user_review_required: true,
            approved: false,
        }
    }
}

#[derive(Debug, Serialize)]
struct ObjectPlanRenderingReport {
    plan_id: String,
    status: String,
    validation_passed: bool,
    contact_sheet_written: bool,
    reason: String,
}

impl ObjectPlanRenderingReport {
    fn from_reports(
        renderability_report: &ObjectPlanRenderabilityReport,
        visual_evidence_report: &ObjectPlanVisualEvidenceReport,
        validation_passed: bool,
    ) -> Self {
        Self {
            plan_id: renderability_report.plan_id.clone(),
            status: if visual_evidence_report.rendered {
                "rendered".to_owned()
            } else {
                "blocked".to_owned()
            },
            validation_passed,
            contact_sheet_written: visual_evidence_report.rendered,
            reason: renderability_report.reason.clone(),
        }
    }
}
