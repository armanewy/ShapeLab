use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use serde::Serialize;
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

    if report.is_valid() {
        println!(
            "Validated ObjectPlan {} into {}",
            plan.plan_id,
            out_dir.display()
        );
        Ok(())
    } else {
        anyhow::bail!(
            "ObjectPlan validation failed with {} issue(s)",
            report.issues.len()
        )
    }
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
