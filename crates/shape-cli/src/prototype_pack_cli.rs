use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use serde::Serialize;
use shape_foundry::{
    AssetRequest, OBJECT_PLAN_SCHEMA_VERSION, ObjectPlan, ObjectPlanAttachment,
    ObjectPlanCreatedBy, ObjectPlanNode, ObjectPlanProvenance, ObjectPlanReviewTier,
    ObjectPlanValidationPolicy, PrimitiveAttachmentOffsetPolicy,
    PrimitiveAttachmentOrientationPolicy, PrimitiveAttachmentScalePolicy, PrimitiveKind,
    PrototypePackBrief, PrototypePackCapability, PrototypePackCompositionKind,
    PrototypePackValidationReport, box_primitive_property_schema,
    flat_panel_primitive_property_schema, primitive_default_property_values,
    prototype_pack_brief_summary, sphere_primitive_property_schema, validate_object_plan,
    validate_prototype_pack_brief,
};

use crate::write_json;

/// Plan Draft ObjectPlan batches from Prototype Pack briefs.
#[derive(Debug, clap::Args)]
pub struct PrototypePackArgs {
    /// Prototype Pack operation.
    #[command(subcommand)]
    pub command: PrototypePackCommand,
}

/// Prototype Pack CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum PrototypePackCommand {
    /// Turn a small brief into Draft ObjectPlan files.
    Plan {
        /// Prototype Pack brief JSON file.
        #[arg(long)]
        brief: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// Run a Prototype Pack command.
pub fn run_prototype_pack(args: PrototypePackArgs) -> anyhow::Result<()> {
    match args.command {
        PrototypePackCommand::Plan { brief, out_dir } => run_plan(&brief, &out_dir),
    }
}

fn run_plan(brief_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let bytes =
        fs::read(brief_path).with_context(|| format!("reading brief {}", brief_path.display()))?;
    let brief: PrototypePackBrief = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing brief {}", brief_path.display()))?;
    fs::create_dir_all(out_dir.join("object-plans"))
        .with_context(|| format!("creating {}", out_dir.display()))?;

    let validation_report = validate_prototype_pack_brief(&brief);
    let mut generated_plans = Vec::new();
    let mut request_reports = Vec::new();

    for request in &brief.asset_requests {
        let result = plans_for_request(request);
        match result {
            PlanRequestResult::Plans(plans) => {
                for plan in plans {
                    let file_name = format!("{}.json", plan.plan_id);
                    let relative_path = format!("object-plans/{file_name}");
                    write_json(out_dir.join(&relative_path), &plan)?;
                    let validation = validate_object_plan(&plan);
                    request_reports.push(PrototypePackRequestPlanReport {
                        request_id: request.request_id.clone(),
                        display_name: request.display_name.clone(),
                        status: if validation.is_valid() {
                            PrototypePackPlanStatus::Passed
                        } else {
                            PrototypePackPlanStatus::Failed
                        },
                        generated_plan_ids: vec![plan.plan_id.clone()],
                        blocked_reasons: Vec::new(),
                        validation_issue_count: validation.issues.len(),
                        approved: false,
                        publish_allowed: false,
                    });
                    generated_plans.push(relative_path);
                }
            }
            PlanRequestResult::Blocked(reason) => {
                request_reports.push(PrototypePackRequestPlanReport {
                    request_id: request.request_id.clone(),
                    display_name: request.display_name.clone(),
                    status: PrototypePackPlanStatus::Blocked,
                    generated_plan_ids: Vec::new(),
                    blocked_reasons: vec![reason],
                    validation_issue_count: 0,
                    approved: false,
                    publish_allowed: false,
                });
            }
        }
    }

    generated_plans.sort();
    let batch = PrototypePackObjectPlanBatchJson {
        batch_id: format!("{}_object_plan_batch", brief.brief_id),
        display_name: format!("{} ObjectPlan Batch", brief.display_name),
        plans: generated_plans.clone(),
        review_policy: PrototypePackObjectPlanBatchReviewPolicy {
            human_review_required: true,
        },
        output_policy: PrototypePackObjectPlanBatchOutputPolicy {
            contact_sheet: true,
        },
    };
    write_json(out_dir.join("object-plan-batch.json"), &batch)?;

    let generated_count = generated_plans.len();
    let blocked_count = request_reports
        .iter()
        .filter(|report| report.status == PrototypePackPlanStatus::Blocked)
        .count();
    let status = if generated_count > 0 && blocked_count == 0 && validation_report.is_valid() {
        PrototypePackPlanStatus::Passed
    } else if generated_count > 0 {
        PrototypePackPlanStatus::Partial
    } else {
        PrototypePackPlanStatus::Blocked
    };
    let report = PrototypePackPlanReport {
        status,
        brief_id: brief.brief_id.clone(),
        generated_plan_count: generated_count,
        blocked_request_count: blocked_count,
        object_plan_batch_path: "object-plan-batch.json".to_owned(),
        object_plan_dir: "object-plans".to_owned(),
        validation_report,
        request_reports,
        human_review_required: true,
        approved: false,
        publish_allowed: false,
        runtime_llm_used: false,
        public_catalog_publishing: false,
        game_ready: false,
        batch_review_path: None,
    };
    write_json(out_dir.join("prototype-pack-plan-report.json"), &report)?;
    fs::write(
        out_dir.join("user-summary.md"),
        user_summary(&brief, &report),
    )?;
    println!(
        "Generated {} Draft ObjectPlan(s) from Prototype Pack brief {} into {}",
        generated_count,
        brief.brief_id,
        out_dir.display()
    );
    Ok(())
}

enum PlanRequestResult {
    Plans(Vec<ObjectPlan>),
    Blocked(String),
}

fn plans_for_request(request: &AssetRequest) -> PlanRequestResult {
    if request
        .must_have_capabilities
        .iter()
        .any(|capability| !supported_planning_capability(*capability))
    {
        return PlanRequestResult::Blocked(
            "Request needs a future capability outside Draft ObjectPlan planning.".to_owned(),
        );
    }
    if request
        .allowed_compositions
        .contains(&PrototypePackCompositionKind::PanelWithKnob)
    {
        return PlanRequestResult::Plans(numbered_plans(request, panel_with_knob_plan));
    }
    if request
        .allowed_primitives
        .contains(&PrimitiveKind::BoxPrimitive)
    {
        return PlanRequestResult::Plans(numbered_plans(request, |request, index| {
            primitive_plan(request, index, PrimitiveKind::BoxPrimitive)
        }));
    }
    if request
        .allowed_primitives
        .contains(&PrimitiveKind::FlatPanelPrimitive)
    {
        return PlanRequestResult::Plans(numbered_plans(request, |request, index| {
            primitive_plan(request, index, PrimitiveKind::FlatPanelPrimitive)
        }));
    }
    if request
        .allowed_primitives
        .contains(&PrimitiveKind::SpherePrimitive)
    {
        return PlanRequestResult::Plans(numbered_plans(request, |request, index| {
            primitive_plan(request, index, PrimitiveKind::SpherePrimitive)
        }));
    }
    PlanRequestResult::Blocked("No supported primitive or composition mapping exists.".to_owned())
}

fn numbered_plans(
    request: &AssetRequest,
    build: impl Fn(&AssetRequest, u32) -> ObjectPlan,
) -> Vec<ObjectPlan> {
    (0..request.desired_count.max(1))
        .map(|index| build(request, index + 1))
        .collect()
}

fn primitive_plan(request: &AssetRequest, index: u32, primitive_kind: PrimitiveKind) -> ObjectPlan {
    let node_id = match primitive_kind {
        PrimitiveKind::BoxPrimitive => "box",
        PrimitiveKind::FlatPanelPrimitive => "panel",
        PrimitiveKind::SpherePrimitive => "round_form",
        PrimitiveKind::CylinderPrimitive => "unsupported",
    };
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: plan_id(request, index),
        display_name: plan_display_name(request, index),
        intent_summary: request.intended_use.clone(),
        nodes: vec![ObjectPlanNode {
            node_id: node_id.to_owned(),
            primitive_kind,
            display_name: request.display_name.clone(),
            property_values: default_values(primitive_kind),
            role_hint: request.intended_use.clone(),
            locked: false,
        }],
        attachments: Vec::new(),
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: provenance(request),
    }
}

fn panel_with_knob_plan(request: &AssetRequest, index: u32) -> ObjectPlan {
    ObjectPlan {
        schema_version: OBJECT_PLAN_SCHEMA_VERSION,
        plan_id: plan_id(request, index),
        display_name: plan_display_name(request, index),
        intent_summary: request.intended_use.clone(),
        nodes: vec![
            ObjectPlanNode {
                node_id: "panel".to_owned(),
                primitive_kind: PrimitiveKind::FlatPanelPrimitive,
                display_name: "Panel".to_owned(),
                property_values: default_values(PrimitiveKind::FlatPanelPrimitive),
                role_hint: "Base panel".to_owned(),
                locked: false,
            },
            ObjectPlanNode {
                node_id: "knob".to_owned(),
                primitive_kind: PrimitiveKind::SpherePrimitive,
                display_name: "Knob-like form".to_owned(),
                property_values: default_values(PrimitiveKind::SpherePrimitive),
                role_hint: "Rounded attached form".to_owned(),
                locked: false,
            },
        ],
        attachments: vec![ObjectPlanAttachment {
            attachment_id: "panel_knob_attachment".to_owned(),
            parent_node_id: "panel".to_owned(),
            parent_anchor_id: "front_handle_zone".to_owned(),
            child_node_id: "knob".to_owned(),
            child_anchor_id: "back_mount_point".to_owned(),
            offset: PrimitiveAttachmentOffsetPolicy::BoundedNormalized {
                x: 0.25,
                y: 0.0,
                minimum_x: -0.6,
                maximum_x: 0.6,
                minimum_y: -0.5,
                maximum_y: 0.5,
            },
            orientation_policy: PrimitiveAttachmentOrientationPolicy::AlignChildToParentNormal,
            scale_policy: PrimitiveAttachmentScalePolicy::KeepChildScale,
        }],
        validation_policy: ObjectPlanValidationPolicy::default(),
        review_tier: ObjectPlanReviewTier::Draft,
        provenance: provenance(request),
    }
}

fn default_values(
    primitive_kind: PrimitiveKind,
) -> BTreeMap<String, shape_foundry::PrimitivePropertyValue> {
    match primitive_kind {
        PrimitiveKind::BoxPrimitive => {
            primitive_default_property_values(&box_primitive_property_schema())
        }
        PrimitiveKind::FlatPanelPrimitive => {
            primitive_default_property_values(&flat_panel_primitive_property_schema())
        }
        PrimitiveKind::SpherePrimitive => {
            primitive_default_property_values(&sphere_primitive_property_schema())
        }
        PrimitiveKind::CylinderPrimitive => BTreeMap::new(),
    }
}

fn provenance(request: &AssetRequest) -> ObjectPlanProvenance {
    ObjectPlanProvenance {
        created_by: ObjectPlanCreatedBy::InternalTool,
        source_prompt_hash: None,
        source_seed_refs: vec![request.request_id.clone()],
        created_at: "1970-01-01T00:00:00Z".to_owned(),
    }
}

fn plan_id(request: &AssetRequest, index: u32) -> String {
    if request.desired_count <= 1 {
        request.request_id.clone()
    } else {
        format!("{}_{index:02}", request.request_id)
    }
}

fn plan_display_name(request: &AssetRequest, index: u32) -> String {
    if request.desired_count <= 1 {
        request.display_name.clone()
    } else {
        format!("{} {index}", request.display_name)
    }
}

const fn supported_planning_capability(capability: PrototypePackCapability) -> bool {
    matches!(
        capability,
        PrototypePackCapability::ObjectPlanDraft
            | PrototypePackCapability::ReviewImage
            | PrototypePackCapability::GeometryOnlyExport
    )
}

fn user_summary(brief: &PrototypePackBrief, report: &PrototypePackPlanReport) -> String {
    let summary = prototype_pack_brief_summary(brief);
    let mut lines = vec![
        format!("# {}", summary.title),
        String::new(),
        format!(
            "Created {} Draft ObjectPlan file(s).",
            report.generated_plan_count
        ),
        "Outputs need review and are not approved.".to_owned(),
        "No public publishing, runtime LLM, or game-ready status is included.".to_owned(),
    ];
    if report.blocked_request_count > 0 {
        lines.push(format!(
            "{} request(s) were blocked by unsupported scope.",
            report.blocked_request_count
        ));
    }
    lines.push(String::new());
    lines.push("## Requested Assets".to_owned());
    lines.extend(
        summary
            .requested_assets
            .into_iter()
            .map(|line| format!("- {line}")),
    );
    lines.push(String::new());
    lines.join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
enum PrototypePackPlanStatus {
    Passed,
    Partial,
    Blocked,
    Failed,
}

#[derive(Debug, Serialize)]
struct PrototypePackPlanReport {
    status: PrototypePackPlanStatus,
    brief_id: String,
    generated_plan_count: usize,
    blocked_request_count: usize,
    object_plan_batch_path: String,
    object_plan_dir: String,
    validation_report: PrototypePackValidationReport,
    request_reports: Vec<PrototypePackRequestPlanReport>,
    human_review_required: bool,
    approved: bool,
    publish_allowed: bool,
    runtime_llm_used: bool,
    public_catalog_publishing: bool,
    game_ready: bool,
    batch_review_path: Option<String>,
}

#[derive(Debug, Serialize)]
struct PrototypePackRequestPlanReport {
    request_id: String,
    display_name: String,
    status: PrototypePackPlanStatus,
    generated_plan_ids: Vec<String>,
    blocked_reasons: Vec<String>,
    validation_issue_count: usize,
    approved: bool,
    publish_allowed: bool,
}

#[derive(Debug, Serialize)]
struct PrototypePackObjectPlanBatchJson {
    batch_id: String,
    display_name: String,
    plans: Vec<String>,
    review_policy: PrototypePackObjectPlanBatchReviewPolicy,
    output_policy: PrototypePackObjectPlanBatchOutputPolicy,
}

#[derive(Debug, Serialize)]
struct PrototypePackObjectPlanBatchReviewPolicy {
    human_review_required: bool,
}

#[derive(Debug, Serialize)]
struct PrototypePackObjectPlanBatchOutputPolicy {
    contact_sheet: bool,
}
