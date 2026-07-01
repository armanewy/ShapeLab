use std::collections::BTreeMap;
use std::f32::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Subcommand, ValueEnum};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use shape_core::Aabb;
use shape_foundry::{
    GeometryExportFormat, GeometryExportPolicy, GeometryExportReport, GeometryExportRequest,
    GeometryExportSourceKind, GeometryExportStatus, GeometryExportUserSummary,
    MaterializationPolicy, MaterializationStatus, MaterializedObjectDraft,
    MaterializedObjectNextAction, MaterializedPrimitiveInstance, ObjectPlan,
    ObjectPlanMaterializationOutputMode, ObjectPlanMaterializationRequest, ObjectPlanReviewTier,
    ObjectPlanValidationReport, PrimitiveAttachmentOffsetPolicy, PrimitiveKind,
    PrimitivePropertyValue, geometry_export_blockers_for_materialized_draft,
    geometry_export_user_summary, materialize_object_plan, materialized_object_summary,
    object_plan_user_summary, validate_geometry_export_report, validate_geometry_export_request,
    validate_object_plan,
};
use shape_mesh::TriangleMesh;
use shape_render::{
    RenderedImage, clay_readability_render_settings, fit_camera_to_bounds_from_angles, render_mesh,
};

use crate::{save_contact_sheet, save_png, write_json};

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
    /// Materialize an ObjectPlan into a review-required draft graph.
    Materialize {
        /// ObjectPlan JSON file.
        #[arg(long)]
        plan: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Write real preview PNG evidence for supported materialized drafts.
        #[arg(long)]
        render_evidence: bool,
    },
    /// Export a supported ObjectPlan as a geometry-only GLB draft.
    ExportGeometry {
        /// ObjectPlan JSON file.
        #[arg(long)]
        plan: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Export format.
        #[arg(long, value_enum, default_value = "glb")]
        format: ObjectPlanGeometryExportFormatArg,
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

/// Supported ObjectPlan geometry export formats.
#[derive(Debug, Copy, Clone, ValueEnum)]
pub enum ObjectPlanGeometryExportFormatArg {
    /// Binary glTF 2.0 package.
    Glb,
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
        ObjectPlanCommand::Materialize {
            plan,
            out_dir,
            render_evidence,
        } => run_materialize(&plan, &out_dir, render_evidence),
        ObjectPlanCommand::ExportGeometry {
            plan,
            out_dir,
            format,
        } => run_export_geometry(&plan, &out_dir, format),
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

fn run_materialize(plan_path: &Path, out_dir: &Path, render_evidence: bool) -> anyhow::Result<()> {
    let outcome = write_materialization_outputs(plan_path, out_dir, render_evidence)?;
    println!(
        "Materialized ObjectPlan {} into {}",
        outcome.plan_id,
        out_dir.display()
    );
    if outcome.status == MaterializationStatus::Failed {
        anyhow::bail!("ObjectPlan materialization failed");
    }
    Ok(())
}

fn run_export_geometry(
    plan_path: &Path,
    out_dir: &Path,
    format: ObjectPlanGeometryExportFormatArg,
) -> anyhow::Result<()> {
    let outcome = write_geometry_export_outputs(plan_path, out_dir, format)?;
    if outcome.status == GeometryExportStatus::Passed {
        println!(
            "Exported ObjectPlan {} geometry into {}",
            outcome.plan_id,
            out_dir.display()
        );
        Ok(())
    } else {
        anyhow::bail!("ObjectPlan geometry export did not pass")
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
        validation_passed: report.is_valid(),
        validation_issue_count: report.issues.len(),
    })
}

fn write_materialization_outputs(
    plan_path: &Path,
    out_dir: &Path,
    render_evidence: bool,
) -> anyhow::Result<ObjectPlanMaterializationOutcome> {
    let plan = read_object_plan(plan_path)?;
    write_materialization_outputs_for_plan(plan, out_dir, render_evidence)
}

fn write_materialization_outputs_for_plan(
    plan: ObjectPlan,
    out_dir: &Path,
    render_evidence: bool,
) -> anyhow::Result<ObjectPlanMaterializationOutcome> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let request = ObjectPlanMaterializationRequest {
        plan: plan.clone(),
        materialization_policy: MaterializationPolicy::default(),
        target_preview_profile: "clay-review".to_owned(),
        output_mode: ObjectPlanMaterializationOutputMode::DraftReview,
    };
    let draft = materialize_object_plan(request);
    let summary = materialized_object_summary(&plan, &draft);
    let report = materialization_report(&plan, &draft);

    write_json(out_dir.join("materialized-object-draft.json"), &draft)?;
    write_json(out_dir.join("materialization-report.json"), &report)?;
    write_json(out_dir.join("normalized-object-plan.json"), &plan)?;
    if !draft.unresolved_nodes.is_empty() {
        write_json(
            out_dir.join("unresolved-nodes.json"),
            &draft.unresolved_nodes,
        )?;
    }
    if !draft.unresolved_attachments.is_empty() {
        write_json(
            out_dir.join("unresolved-attachments.json"),
            &draft.unresolved_attachments,
        )?;
    }
    fs::write(
        out_dir.join("materialized-user-summary.md"),
        materialized_user_summary_markdown(&summary, &report),
    )
    .with_context(|| {
        format!(
            "writing {}",
            out_dir.join("materialized-user-summary.md").display()
        )
    })?;
    let render_evidence_report = if render_evidence {
        let render_report = write_render_evidence_outputs(out_dir, &draft)?;
        if !render_report.rendered {
            println!(
                "ObjectPlan render evidence blocked for {}",
                render_report.plan_id
            );
        }
        Some(render_report)
    } else {
        None
    };

    Ok(ObjectPlanMaterializationOutcome {
        plan_id: plan.plan_id,
        display_name: plan.display_name,
        status: draft.status,
        validation_passed: draft.validation_report.is_valid(),
        validation_issue_count: draft.validation_report.issues.len(),
        materialization_report: report,
        render_evidence_report,
        draft,
    })
}

fn write_geometry_export_outputs(
    plan_path: &Path,
    out_dir: &Path,
    format: ObjectPlanGeometryExportFormatArg,
) -> anyhow::Result<ObjectPlanGeometryExportOutcome> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;

    let plan_bytes = match fs::read(plan_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            let report = blocked_geometry_export_report(
                None,
                0,
                vec![format!("ObjectPlan could not be read: {error}")],
                GeometryExportStatus::Failed,
            );
            write_geometry_export_report_outputs(out_dir, &report)?;
            return Ok(ObjectPlanGeometryExportOutcome {
                plan_id: "unknown-plan".to_owned(),
                status: report.status,
            });
        }
    };
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&plan_bytes) {
        let blocked_capabilities = geometry_export_blocked_capability_fields(&value);
        if !blocked_capabilities.is_empty() {
            let source_plan_id = json_plan_id(&value);
            let report = blocked_geometry_export_report(
                source_plan_id.clone(),
                json_node_count(&value),
                blocked_capabilities,
                GeometryExportStatus::Blocked,
            );
            write_geometry_export_report_outputs(out_dir, &report)?;
            return Ok(ObjectPlanGeometryExportOutcome {
                plan_id: source_plan_id.unwrap_or_else(|| "unknown-plan".to_owned()),
                status: report.status,
            });
        }
    }

    let plan = match serde_json::from_slice::<ObjectPlan>(&plan_bytes) {
        Ok(plan) => plan,
        Err(error) => {
            let report = blocked_geometry_export_report(
                None,
                0,
                vec![format!("ObjectPlan could not be parsed: {error}")],
                GeometryExportStatus::Failed,
            );
            write_geometry_export_report_outputs(out_dir, &report)?;
            return Ok(ObjectPlanGeometryExportOutcome {
                plan_id: "unknown-plan".to_owned(),
                status: report.status,
            });
        }
    };

    let request = GeometryExportRequest {
        source_kind: GeometryExportSourceKind::ObjectPlan,
        source_ref: persisted_source_ref(
            plan_path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("object-plan.json"),
            plan_path,
        ),
        export_format: geometry_export_format(format),
        export_policy: GeometryExportPolicy::default(),
        output_name: "asset".to_owned(),
        output_dir: ".".to_owned(),
    };
    let request_report = validate_geometry_export_request(&request);
    if !request_report.is_valid() {
        let report = blocked_geometry_export_report(
            Some(plan.plan_id.clone()),
            plan.nodes.len(),
            request_report
                .issues
                .into_iter()
                .map(|issue| issue.message)
                .collect(),
            GeometryExportStatus::Blocked,
        );
        write_json(out_dir.join("normalized-object-plan.json"), &plan)?;
        write_geometry_export_report_outputs(out_dir, &report)?;
        return Ok(ObjectPlanGeometryExportOutcome {
            plan_id: plan.plan_id,
            status: report.status,
        });
    }

    let outcome = write_materialization_outputs_for_plan(plan.clone(), out_dir, true)?;
    let mut blockers = geometry_export_blockers_for_materialized_draft(&outcome.draft);
    let Some(mesh) = materialized_draft_mesh(&outcome.draft)? else {
        blockers.push("Source draft did not produce exportable geometry.".to_owned());
        let report = blocked_geometry_export_report(
            Some(plan.plan_id.clone()),
            outcome.draft.primitive_instances.len(),
            blockers,
            GeometryExportStatus::Blocked,
        );
        write_geometry_export_report_outputs(out_dir, &report)?;
        return Ok(ObjectPlanGeometryExportOutcome {
            plan_id: plan.plan_id,
            status: report.status,
        });
    };

    if !blockers.is_empty() {
        let report = blocked_geometry_export_report(
            Some(plan.plan_id.clone()),
            outcome.draft.primitive_instances.len(),
            blockers,
            GeometryExportStatus::Blocked,
        );
        write_geometry_export_report_outputs(out_dir, &report)?;
        return Ok(ObjectPlanGeometryExportOutcome {
            plan_id: plan.plan_id,
            status: report.status,
        });
    }

    let asset_path = out_dir.join("asset.glb");
    write_geometry_only_glb(&mesh, &asset_path)?;
    let report = GeometryExportReport {
        status: GeometryExportStatus::Passed,
        output_files: vec!["asset.glb".to_owned()],
        source_plan_id: Some(plan.plan_id.clone()),
        primitive_count: outcome.draft.primitive_instances.len(),
        mesh_count: 1,
        triangle_count: mesh.indices.len() / 3,
        warning_count: 0,
        blockers: Vec::new(),
        includes_uvs: false,
        includes_textures: false,
        includes_material_looks: false,
        includes_collision: false,
        includes_rig: false,
        includes_animation: false,
        game_ready: false,
        human_review_required: true,
    };
    let validation_report = validate_geometry_export_report(&report);
    if !validation_report.is_valid() {
        anyhow::bail!(
            "geometry export report failed validation: {:?}",
            validation_report.issues
        );
    }
    write_geometry_export_report_outputs(out_dir, &report)?;

    Ok(ObjectPlanGeometryExportOutcome {
        plan_id: plan.plan_id,
        status: report.status,
    })
}

fn write_geometry_export_report_outputs(
    out_dir: &Path,
    report: &GeometryExportReport,
) -> anyhow::Result<()> {
    write_json(out_dir.join("geometry-export-report.json"), report)?;
    fs::write(
        out_dir.join("geometry-export-user-summary.md"),
        geometry_export_user_summary_markdown(&geometry_export_user_summary(report), report),
    )
    .with_context(|| {
        format!(
            "writing {}",
            out_dir.join("geometry-export-user-summary.md").display()
        )
    })?;
    Ok(())
}

fn blocked_geometry_export_report(
    source_plan_id: Option<String>,
    primitive_count: usize,
    blockers: Vec<String>,
    status: GeometryExportStatus,
) -> GeometryExportReport {
    GeometryExportReport {
        status,
        output_files: Vec::new(),
        source_plan_id,
        primitive_count,
        mesh_count: 0,
        triangle_count: 0,
        warning_count: 0,
        blockers,
        includes_uvs: false,
        includes_textures: false,
        includes_material_looks: false,
        includes_collision: false,
        includes_rig: false,
        includes_animation: false,
        game_ready: false,
        human_review_required: true,
    }
}

fn geometry_export_format(format: ObjectPlanGeometryExportFormatArg) -> GeometryExportFormat {
    match format {
        ObjectPlanGeometryExportFormatArg::Glb => GeometryExportFormat::Glb,
    }
}

fn geometry_export_blocked_capability_fields(value: &serde_json::Value) -> Vec<String> {
    let mut blockers = Vec::new();
    collect_geometry_export_blocked_capability_fields(value, &mut blockers);
    blockers.sort();
    blockers.dedup();
    blockers
}

fn collect_geometry_export_blocked_capability_fields(
    value: &serde_json::Value,
    blockers: &mut Vec<String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                if let Some(blocker) = geometry_export_capability_blocker(key) {
                    blockers.push(blocker.to_owned());
                }
                collect_geometry_export_blocked_capability_fields(child, blockers);
            }
        }
        serde_json::Value::Array(values) => {
            for child in values {
                collect_geometry_export_blocked_capability_fields(child, blockers);
            }
        }
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {}
    }
}

fn geometry_export_capability_blocker(key: &str) -> Option<&'static str> {
    match key {
        "raw_mesh_payload" => Some("Raw mesh payloads are outside geometry export V0."),
        "texture_request" | "textures" | "texture_files" => {
            Some("Texture requests are outside geometry export V0.")
        }
        "material_request" | "material_looks" | "surface_request" => {
            Some("Material look requests are outside geometry export V0.")
        }
        "collision_request" | "collision" | "gameplay_metadata" => {
            Some("Collision or gameplay metadata is outside geometry export V0.")
        }
        "rigging_request" | "rig" | "skeleton" => {
            Some("Rigging requests are outside geometry export V0.")
        }
        "animation_request" | "animations" => {
            Some("Animation requests are outside geometry export V0.")
        }
        "game_ready" | "game_ready_request" => {
            Some("Game-ready requests are outside geometry export V0.")
        }
        _ => None,
    }
}

fn json_plan_id(value: &serde_json::Value) -> Option<String> {
    value
        .get("plan_id")
        .and_then(|plan_id| plan_id.as_str())
        .map(str::to_owned)
}

fn json_node_count(value: &serde_json::Value) -> usize {
    value
        .get("nodes")
        .and_then(|nodes| nodes.as_array())
        .map_or(0, Vec::len)
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
    let mut materialization_plan_reports = Vec::new();
    let mut render_plan_reports = Vec::new();
    let mut rendered_preview_paths = Vec::new();
    for (index, plan_input) in batch.plans.iter().enumerate() {
        let slug = unique_plan_slug(index, &plan_input.source_ref);
        let relative_out_dir = format!("plans/{slug}");
        let plan_out_dir = out_dir.join(&relative_out_dir);
        let outcome = write_materialization_outputs(&plan_input.path, &plan_out_dir, true);
        let rendered_preview_path = plan_out_dir.join("plan-preview.png");
        let (plan_report, materialization_report, render_report) =
            batch_reports_from_outcome(plan_input, &relative_out_dir, outcome);
        if render_report.rendered && rendered_preview_path.is_file() {
            rendered_preview_paths.push(rendered_preview_path);
        }
        plan_reports.push(plan_report);
        materialization_plan_reports.push(materialization_report);
        render_plan_reports.push(render_report);
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
    let contact_sheet_path =
        if batch.output_policy.contact_sheet && !rendered_preview_paths.is_empty() {
            write_batch_contact_sheet(
                &rendered_preview_paths,
                &out_dir.join("batch-contact-sheet.png"),
            )?;
            Some("batch-contact-sheet.png".to_owned())
        } else {
            None
        };
    let _ = batch.review_policy.human_review_required;
    let report = ObjectPlanBatchValidationReport {
        batch_id: batch.batch_id.clone(),
        display_name: batch.display_name.clone(),
        total_plans: plan_reports.len(),
        passed_validation,
        failed_validation,
        rendered,
        unsupported,
        human_review_required: true,
        approved: false,
        publish_allowed: false,
        plans: plan_reports,
    };
    let materialization_report = ObjectPlanBatchMaterializationReport {
        batch_id: batch.batch_id.clone(),
        display_name: batch.display_name.clone(),
        total_plans: materialization_plan_reports.len(),
        passed: materialization_plan_reports
            .iter()
            .filter(|plan| plan.status == "Passed")
            .count(),
        partial: materialization_plan_reports
            .iter()
            .filter(|plan| plan.status == "Partial")
            .count(),
        failed: materialization_plan_reports
            .iter()
            .filter(|plan| plan.status == "Failed")
            .count(),
        human_review_required: true,
        approved: false,
        publish_allowed: false,
        plans: materialization_plan_reports,
    };
    let render_evidence_report = ObjectPlanBatchRenderEvidenceReport {
        batch_id: batch.batch_id,
        display_name: batch.display_name,
        total_plans: render_plan_reports.len(),
        rendered,
        blocked: render_plan_reports
            .iter()
            .filter(|plan| !plan.rendered)
            .count(),
        contact_sheet_path,
        human_review_required: true,
        approved: false,
        publish_allowed: false,
        plans: render_plan_reports,
    };

    write_json(out_dir.join("batch-validation-report.json"), &report)?;
    write_json(
        out_dir.join("batch-materialization-report.json"),
        &materialization_report,
    )?;
    write_json(
        out_dir.join("batch-render-evidence-report.json"),
        &render_evidence_report,
    )?;
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

fn batch_reports_from_outcome(
    plan_input: &ObjectPlanBatchPlanInput,
    relative_out_dir: &str,
    outcome: anyhow::Result<ObjectPlanMaterializationOutcome>,
) -> (
    ObjectPlanBatchPlanReport,
    ObjectPlanBatchMaterializationPlanReport,
    ObjectPlanBatchRenderEvidencePlanReport,
) {
    match outcome {
        Ok(outcome) => {
            let render_report = outcome.render_evidence_report.clone().unwrap_or_else(|| {
                ObjectPlanRenderEvidenceReport {
                    rendered: false,
                    materialized: outcome.status == MaterializationStatus::Passed,
                    plan_id: outcome.plan_id.clone(),
                    preview_count: 0,
                    contact_sheet_path: None,
                    unsupported_primitives: Vec::new(),
                    unsupported_attachments: Vec::new(),
                    warnings: vec!["Render evidence was not requested.".to_owned()],
                    user_review_required: true,
                    approved: false,
                }
            });
            let recommendation =
                batch_recommendation(&outcome, render_report.rendered, &render_report);
            let plan_report = ObjectPlanBatchPlanReport {
                source_ref: plan_input.source_ref.clone(),
                output_dir: relative_out_dir.to_owned(),
                plan_id: Some(outcome.plan_id.clone()),
                display_name: Some(outcome.display_name.clone()),
                validation_status: if outcome.validation_passed {
                    "Passed".to_owned()
                } else {
                    "Failed".to_owned()
                },
                validation_issue_count: outcome.validation_issue_count,
                materialization_status: materialization_status_label(outcome.status).to_owned(),
                rendered: render_report.rendered,
                unsupported: !render_report.unsupported_primitives.is_empty()
                    || !render_report.unsupported_attachments.is_empty()
                    || !outcome.validation_passed,
                recommendation,
                errors: Vec::new(),
            };
            let materialization_plan_report = ObjectPlanBatchMaterializationPlanReport {
                source_ref: plan_input.source_ref.clone(),
                output_dir: relative_out_dir.to_owned(),
                plan_id: Some(outcome.plan_id.clone()),
                status: materialization_status_label(outcome.status).to_owned(),
                primitive_count: outcome.materialization_report.primitive_count,
                materialized_primitive_count: outcome
                    .materialization_report
                    .materialized_primitive_count,
                attachment_count: outcome.materialization_report.attachment_count,
                materialized_attachment_count: outcome
                    .materialization_report
                    .materialized_attachment_count,
                unresolved_node_count: outcome.materialization_report.unresolved_nodes.len(),
                unresolved_attachment_count: outcome
                    .materialization_report
                    .unresolved_attachments
                    .len(),
                user_review_required: true,
                publish_allowed: false,
                errors: Vec::new(),
            };
            let render_plan_report = ObjectPlanBatchRenderEvidencePlanReport {
                source_ref: plan_input.source_ref.clone(),
                output_dir: relative_out_dir.to_owned(),
                plan_id: Some(outcome.plan_id),
                rendered: render_report.rendered,
                materialized: render_report.materialized,
                preview_count: render_report.preview_count,
                contact_sheet_path: render_report
                    .contact_sheet_path
                    .map(|path| format!("{relative_out_dir}/{path}")),
                warnings: render_report.warnings,
                user_review_required: true,
                approved: false,
                publish_allowed: false,
                errors: Vec::new(),
            };
            (plan_report, materialization_plan_report, render_plan_report)
        }
        Err(_error) => {
            let errors = vec!["Plan could not be read or parsed.".to_owned()];
            (
                ObjectPlanBatchPlanReport {
                    source_ref: plan_input.source_ref.clone(),
                    output_dir: relative_out_dir.to_owned(),
                    plan_id: None,
                    display_name: None,
                    validation_status: "Failed".to_owned(),
                    validation_issue_count: 1,
                    materialization_status: "Failed".to_owned(),
                    rendered: false,
                    unsupported: true,
                    recommendation: BatchReviewRecommendation::Blocked,
                    errors: errors.clone(),
                },
                ObjectPlanBatchMaterializationPlanReport {
                    source_ref: plan_input.source_ref.clone(),
                    output_dir: relative_out_dir.to_owned(),
                    plan_id: None,
                    status: "Failed".to_owned(),
                    primitive_count: 0,
                    materialized_primitive_count: 0,
                    attachment_count: 0,
                    materialized_attachment_count: 0,
                    unresolved_node_count: 0,
                    unresolved_attachment_count: 0,
                    user_review_required: true,
                    publish_allowed: false,
                    errors: errors.clone(),
                },
                ObjectPlanBatchRenderEvidencePlanReport {
                    source_ref: plan_input.source_ref.clone(),
                    output_dir: relative_out_dir.to_owned(),
                    plan_id: None,
                    rendered: false,
                    materialized: false,
                    preview_count: 0,
                    contact_sheet_path: None,
                    warnings: Vec::new(),
                    user_review_required: true,
                    approved: false,
                    publish_allowed: false,
                    errors,
                },
            )
        }
    }
}

fn batch_recommendation(
    outcome: &ObjectPlanMaterializationOutcome,
    rendered: bool,
    render_report: &ObjectPlanRenderEvidenceReport,
) -> BatchReviewRecommendation {
    if !outcome.validation_passed || !render_report.unsupported_primitives.is_empty() {
        return BatchReviewRecommendation::Blocked;
    }
    if outcome.status != MaterializationStatus::Passed
        || !render_report.unsupported_attachments.is_empty()
    {
        return BatchReviewRecommendation::Simplify;
    }
    if rendered {
        BatchReviewRecommendation::Keep
    } else {
        BatchReviewRecommendation::Regenerate
    }
}

fn write_batch_contact_sheet(preview_paths: &[PathBuf], path: &Path) -> anyhow::Result<()> {
    let images = preview_paths
        .iter()
        .map(|preview_path| read_rendered_png(preview_path))
        .collect::<anyhow::Result<Vec<_>>>()?;
    let Some((parent, candidates)) = images.split_first() else {
        return Ok(());
    };
    let candidate_refs = candidates.iter().collect::<Vec<_>>();
    save_contact_sheet(parent, &candidate_refs, path)
}

fn read_rendered_png(path: &Path) -> anyhow::Result<RenderedImage> {
    let image = image::open(path)
        .with_context(|| format!("reading rendered preview {}", path.display()))?
        .into_rgba8();
    Ok(RenderedImage {
        width: image.width(),
        height: image.height(),
        rgba8: image.into_raw(),
    })
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
        "# {}\n\nTotal plans: {}\n\nPassed validation: {}\n\nFailed validation: {}\n\nRendered: {}\n\nHuman review required: true\n\nApproved: false\n\nPublish allowed: false\n",
        report.display_name,
        report.total_plans,
        report.passed_validation,
        report.failed_validation,
        report.rendered
    )
}

fn recommendation_reason(plan: &ObjectPlanBatchPlanReport) -> &'static str {
    match plan.recommendation {
        BatchReviewRecommendation::Keep => "rendered evidence is available for human review",
        BatchReviewRecommendation::Regenerate => "valid draft needs stronger visual evidence",
        BatchReviewRecommendation::Simplify => "available primitives or anchors are not enough",
        BatchReviewRecommendation::Blocked => "the plan is invalid or unsupported",
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

fn materialization_report(
    plan: &ObjectPlan,
    draft: &MaterializedObjectDraft,
) -> ObjectPlanMaterializationReport {
    ObjectPlanMaterializationReport {
        status: draft.status,
        primitive_count: plan.nodes.len(),
        materialized_primitive_count: draft.primitive_instances.len(),
        attachment_count: plan.attachments.len(),
        materialized_attachment_count: draft.composition_document.attachments.len(),
        unresolved_nodes: draft.unresolved_nodes.clone(),
        unresolved_attachments: draft.unresolved_attachments.clone(),
        user_review_required: true,
        publish_allowed: false,
    }
}

fn materialized_user_summary_markdown(
    summary: &shape_foundry::MaterializedObjectSummary,
    report: &ObjectPlanMaterializationReport,
) -> String {
    format!(
        "# {}\n\nSupported primitives: {}\n\nUnresolved primitives: {}\n\nSupported attachments: {}\n\nUnresolved attachments: {}\n\nHuman review required: true\n\nPublish allowed: false\n\nNext action: {}\n\nStatus: {}\n",
        summary.source_plan_label,
        summary.supported_primitive_count,
        summary.unresolved_primitive_count,
        summary.supported_attachment_count,
        summary.unresolved_attachment_count,
        materialized_next_action_label(summary.next_action),
        materialization_status_label(report.status)
    )
}

fn geometry_export_user_summary_markdown(
    summary: &GeometryExportUserSummary,
    report: &GeometryExportReport,
) -> String {
    let mut markdown = String::new();
    markdown.push_str("# ");
    markdown.push_str(&summary.title);
    markdown.push_str("\n\n");
    markdown.push_str("Status: ");
    markdown.push_str(match report.status {
        GeometryExportStatus::Passed => "Passed",
        GeometryExportStatus::Blocked => "Blocked",
        GeometryExportStatus::Failed => "Failed",
    });
    markdown.push_str("\n\n");
    for line in &summary.lines {
        markdown.push_str("- ");
        markdown.push_str(line);
        markdown.push('\n');
    }
    if !report.blockers.is_empty() {
        markdown.push_str("\n## Blockers\n\n");
        for blocker in &report.blockers {
            markdown.push_str("- ");
            markdown.push_str(blocker);
            markdown.push('\n');
        }
    }
    markdown.push_str("\nHuman review required: true\n");
    markdown
}

fn write_render_evidence_outputs(
    out_dir: &Path,
    draft: &MaterializedObjectDraft,
) -> anyhow::Result<ObjectPlanRenderEvidenceReport> {
    let mut report = blocked_render_evidence_report(draft);
    if draft.status != MaterializationStatus::Passed {
        write_json(out_dir.join("render-evidence-report.json"), &report)?;
        return Ok(report);
    }

    let Some(preview_set) = render_materialized_preview_set(draft)? else {
        write_json(out_dir.join("render-evidence-report.json"), &report)?;
        return Ok(report);
    };

    let node_dir = out_dir.join("node-previews");
    fs::create_dir_all(&node_dir).with_context(|| format!("creating {}", node_dir.display()))?;
    for node_preview in &preview_set.node_previews {
        save_png(
            &node_preview.image,
            node_dir.join(format!("{}.png", safe_identifier(&node_preview.node_id))),
        )?;
    }
    save_png(&preview_set.plan_preview, out_dir.join("plan-preview.png"))?;
    let node_images = preview_set
        .node_previews
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_contact_sheet(
        &preview_set.plan_preview,
        &node_images,
        out_dir.join("contact-sheet.png"),
    )?;

    report.rendered = true;
    report.materialized = true;
    report.preview_count = 1 + preview_set.node_previews.len();
    report.contact_sheet_path = Some("contact-sheet.png".to_owned());
    report.warnings.clear();
    write_json(out_dir.join("render-evidence-report.json"), &report)?;
    Ok(report)
}

fn blocked_render_evidence_report(
    draft: &MaterializedObjectDraft,
) -> ObjectPlanRenderEvidenceReport {
    let mut warnings = Vec::new();
    if draft.status != MaterializationStatus::Passed {
        warnings.push(format!(
            "Render evidence is blocked because materialization status is {}.",
            materialization_status_label(draft.status)
        ));
    }
    warnings.extend(
        draft
            .unresolved_nodes
            .iter()
            .map(|node| format!("{}: {}", node.node_id, node.reason)),
    );
    warnings.extend(
        draft
            .unresolved_attachments
            .iter()
            .map(|attachment| format!("{}: {}", attachment.attachment_id, attachment.reason)),
    );
    ObjectPlanRenderEvidenceReport {
        rendered: false,
        materialized: draft.status == MaterializationStatus::Passed
            && !draft.primitive_instances.is_empty(),
        plan_id: draft.source_plan_id.clone(),
        preview_count: 0,
        contact_sheet_path: None,
        unsupported_primitives: draft
            .unresolved_nodes
            .iter()
            .map(|node| format!("{}: {}", node.node_id, node.reason))
            .collect(),
        unsupported_attachments: draft
            .unresolved_attachments
            .iter()
            .map(|attachment| format!("{}: {}", attachment.attachment_id, attachment.reason))
            .collect(),
        warnings,
        user_review_required: true,
        approved: false,
    }
}

fn render_materialized_preview_set(
    draft: &MaterializedObjectDraft,
) -> anyhow::Result<Option<MaterializedPreviewSet>> {
    if draft.primitive_instances.is_empty()
        || !draft.unresolved_nodes.is_empty()
        || !draft.unresolved_attachments.is_empty()
    {
        return Ok(None);
    }

    let placements = materialized_node_placements(draft);
    let mut node_previews = Vec::new();
    let mut placed_meshes = Vec::new();
    for instance in &draft.primitive_instances {
        let centered_mesh = primitive_instance_mesh(instance, Vec3::ZERO)?;
        let image = render_review_image(&centered_mesh, 256)?;
        node_previews.push(NodePreview {
            node_id: instance.node_id.clone(),
            image,
        });

        let placement = placements
            .get(instance.node_id.as_str())
            .copied()
            .unwrap_or(Vec3::ZERO);
        placed_meshes.push(translate_mesh(&centered_mesh, placement));
    }

    let plan_mesh =
        merge_meshes(&placed_meshes).context("materialized draft has no preview mesh")?;
    let plan_preview = render_review_image(&plan_mesh, 512)?;
    Ok(Some(MaterializedPreviewSet {
        plan_preview,
        node_previews,
    }))
}

fn materialized_draft_mesh(
    draft: &MaterializedObjectDraft,
) -> anyhow::Result<Option<TriangleMesh>> {
    if draft.primitive_instances.is_empty()
        || !draft.unresolved_nodes.is_empty()
        || !draft.unresolved_attachments.is_empty()
    {
        return Ok(None);
    }

    let placements = materialized_node_placements(draft);
    let mut placed_meshes = Vec::new();
    for instance in &draft.primitive_instances {
        let centered_mesh = primitive_instance_mesh(instance, Vec3::ZERO)?;
        let placement = placements
            .get(instance.node_id.as_str())
            .copied()
            .unwrap_or(Vec3::ZERO);
        placed_meshes.push(translate_mesh(&centered_mesh, placement));
    }

    Ok(merge_meshes(&placed_meshes))
}

fn materialized_node_placements(draft: &MaterializedObjectDraft) -> BTreeMap<String, Vec3> {
    let mut placements = draft
        .primitive_instances
        .iter()
        .map(|instance| (instance.node_id.clone(), Vec3::ZERO))
        .collect::<BTreeMap<_, _>>();
    if draft.composition_document.attachments.is_empty() {
        return spread_unattached_placements(&draft.primitive_instances);
    }

    let instances = draft
        .primitive_instances
        .iter()
        .map(|instance| (instance.node_id.as_str(), instance))
        .collect::<BTreeMap<_, _>>();
    for attachment in &draft.composition_document.attachments {
        let Some(parent) = instances.get(attachment.parent_node_id.as_str()).copied() else {
            continue;
        };
        let Some(child) = instances.get(attachment.child_node_id.as_str()).copied() else {
            continue;
        };
        if parent.primitive_kind != PrimitiveKind::FlatPanelPrimitive
            || child.primitive_kind != PrimitiveKind::SpherePrimitive
            || attachment.parent_anchor_id != "right_side_handle_zone"
            || attachment.child_anchor_id != "back_mount_point"
        {
            continue;
        }

        let parent_dimensions = primitive_dimensions(parent);
        let child_dimensions = primitive_dimensions(child);
        let (offset_x, offset_y) = match attachment.offset_policy {
            PrimitiveAttachmentOffsetPolicy::Fixed => (0.0, 0.0),
            PrimitiveAttachmentOffsetPolicy::BoundedNormalized { x, y, .. } => (x, y),
        };
        let x = (0.45 + offset_x) * parent_dimensions.width * 0.5;
        let y = offset_y * parent_dimensions.height * 0.5;
        let z = -parent_dimensions.depth * 0.5 - child_dimensions.depth * 0.5;
        placements.insert(child.node_id.clone(), Vec3::new(x, y, z));
    }
    placements
}

fn spread_unattached_placements(
    instances: &[MaterializedPrimitiveInstance],
) -> BTreeMap<String, Vec3> {
    if instances.len() <= 1 {
        return instances
            .iter()
            .map(|instance| (instance.node_id.clone(), Vec3::ZERO))
            .collect();
    }

    let spacing = instances
        .iter()
        .map(|instance| primitive_dimensions(instance).width)
        .fold(0.0, f32::max)
        + 0.4;
    let center = (instances.len() as f32 - 1.0) * 0.5;
    instances
        .iter()
        .enumerate()
        .map(|(index, instance)| {
            (
                instance.node_id.clone(),
                Vec3::new((index as f32 - center) * spacing, 0.0, 0.0),
            )
        })
        .collect()
}

fn render_review_image(mesh: &TriangleMesh, size: u32) -> anyhow::Result<RenderedImage> {
    let camera = fit_camera_to_bounds_from_angles(mesh.bounds, 35.0, 24.0, 1.0);
    let settings = clay_readability_render_settings(size, size);
    render_mesh(mesh, &camera, &settings).context("rendering ObjectPlan preview")
}

fn primitive_instance_mesh(
    instance: &MaterializedPrimitiveInstance,
    center: Vec3,
) -> anyhow::Result<TriangleMesh> {
    let dimensions = primitive_dimensions(instance);
    match instance.primitive_kind {
        PrimitiveKind::BoxPrimitive | PrimitiveKind::FlatPanelPrimitive => Ok(cuboid_mesh(
            dimensions.width,
            dimensions.height,
            dimensions.depth,
            center,
        )),
        PrimitiveKind::SpherePrimitive => Ok(sphere_mesh(
            dimensions.width,
            dimensions.height,
            dimensions.depth,
            property_ratio(instance, "front_flatten", 0.0),
            property_ratio(instance, "back_flatten", 0.0),
            center,
        )),
        PrimitiveKind::CylinderPrimitive => {
            anyhow::bail!("Cylinder Primitive is not renderable for ObjectPlan evidence v1")
        }
    }
}

fn primitive_dimensions(instance: &MaterializedPrimitiveInstance) -> PrimitiveDimensions {
    match instance.primitive_kind {
        PrimitiveKind::BoxPrimitive => PrimitiveDimensions {
            width: property_length(instance, "width", 2.0),
            height: property_length(instance, "height", 1.0),
            depth: property_length(instance, "depth", 1.4),
        },
        PrimitiveKind::FlatPanelPrimitive => PrimitiveDimensions {
            width: property_length(instance, "width", 1.8),
            height: property_length(instance, "height", 2.6),
            depth: property_length(instance, "thickness", 0.18),
        },
        PrimitiveKind::SpherePrimitive => PrimitiveDimensions {
            width: property_length(instance, "width", 1.0),
            height: property_length(instance, "height", 1.0),
            depth: property_length(instance, "depth", 1.0),
        },
        PrimitiveKind::CylinderPrimitive => PrimitiveDimensions {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        },
    }
}

fn property_length(
    instance: &MaterializedPrimitiveInstance,
    property_id: &str,
    fallback: f32,
) -> f32 {
    match instance.property_values.get(property_id) {
        Some(PrimitivePropertyValue::Length(value)) if value.is_finite() => *value,
        _ => fallback,
    }
}

fn property_ratio(
    instance: &MaterializedPrimitiveInstance,
    property_id: &str,
    fallback: f32,
) -> f32 {
    match instance.property_values.get(property_id) {
        Some(PrimitivePropertyValue::Ratio(value)) if value.is_finite() => *value,
        _ => fallback,
    }
}

fn cuboid_mesh(width: f32, height: f32, depth: f32, center: Vec3) -> TriangleMesh {
    let half = Vec3::new(width.max(0.01), height.max(0.01), depth.max(0.01)) * 0.5;
    let mut builder = MeshBuilder::default();
    let x = half.x;
    let y = half.y;
    let z = half.z;
    builder.add_quad(
        [
            center + Vec3::new(-x, -y, -z),
            center + Vec3::new(-x, y, -z),
            center + Vec3::new(x, y, -z),
            center + Vec3::new(x, -y, -z),
        ],
        Vec3::NEG_Z,
    );
    builder.add_quad(
        [
            center + Vec3::new(-x, -y, z),
            center + Vec3::new(x, -y, z),
            center + Vec3::new(x, y, z),
            center + Vec3::new(-x, y, z),
        ],
        Vec3::Z,
    );
    builder.add_quad(
        [
            center + Vec3::new(x, -y, -z),
            center + Vec3::new(x, y, -z),
            center + Vec3::new(x, y, z),
            center + Vec3::new(x, -y, z),
        ],
        Vec3::X,
    );
    builder.add_quad(
        [
            center + Vec3::new(-x, -y, -z),
            center + Vec3::new(-x, -y, z),
            center + Vec3::new(-x, y, z),
            center + Vec3::new(-x, y, -z),
        ],
        Vec3::NEG_X,
    );
    builder.add_quad(
        [
            center + Vec3::new(-x, y, -z),
            center + Vec3::new(-x, y, z),
            center + Vec3::new(x, y, z),
            center + Vec3::new(x, y, -z),
        ],
        Vec3::Y,
    );
    builder.add_quad(
        [
            center + Vec3::new(-x, -y, -z),
            center + Vec3::new(x, -y, -z),
            center + Vec3::new(x, -y, z),
            center + Vec3::new(-x, -y, z),
        ],
        Vec3::NEG_Y,
    );
    builder.finish()
}

fn sphere_mesh(
    width: f32,
    height: f32,
    depth: f32,
    front_flatten: f32,
    back_flatten: f32,
    center: Vec3,
) -> TriangleMesh {
    const LATITUDE_SEGMENTS: usize = 16;
    const LONGITUDE_SEGMENTS: usize = 32;
    let radii = Vec3::new(width.max(0.01), height.max(0.01), depth.max(0.01)) * 0.5;
    let front_scale = 1.0 - front_flatten.clamp(0.0, 0.8) * 0.75;
    let back_scale = 1.0 - back_flatten.clamp(0.0, 0.8) * 0.75;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    for latitude in 0..=LATITUDE_SEGMENTS {
        let theta = PI * latitude as f32 / LATITUDE_SEGMENTS as f32;
        let y = theta.cos();
        let ring_radius = theta.sin();
        for longitude in 0..=LONGITUDE_SEGMENTS {
            let phi = 2.0 * PI * longitude as f32 / LONGITUDE_SEGMENTS as f32;
            let x = ring_radius * phi.sin();
            let mut z = ring_radius * phi.cos();
            z *= if z < 0.0 { front_scale } else { back_scale };
            let normal = Vec3::new(x, y, z).normalize_or_zero();
            let position = center + Vec3::new(x * radii.x, y * radii.y, z * radii.z);
            positions.push(position.to_array());
            normals.push(normal.to_array());
        }
    }

    let stride = LONGITUDE_SEGMENTS + 1;
    let mut indices = Vec::new();
    for latitude in 0..LATITUDE_SEGMENTS {
        for longitude in 0..LONGITUDE_SEGMENTS {
            let a = (latitude * stride + longitude) as u32;
            let b = a + 1;
            let c = ((latitude + 1) * stride + longitude) as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }

    TriangleMesh {
        positions,
        normals,
        indices,
        bounds: Aabb {
            min: center - radii,
            max: center + radii,
        },
    }
}

fn translate_mesh(mesh: &TriangleMesh, translation: Vec3) -> TriangleMesh {
    if translation == Vec3::ZERO {
        return mesh.clone();
    }
    TriangleMesh {
        positions: mesh
            .positions
            .iter()
            .map(|position| (Vec3::from_array(*position) + translation).to_array())
            .collect(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: mesh.bounds.min + translation,
            max: mesh.bounds.max + translation,
        },
    }
}

fn merge_meshes(meshes: &[TriangleMesh]) -> Option<TriangleMesh> {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    let mut bounds = Aabb::empty();
    for mesh in meshes {
        if mesh.positions.is_empty() || mesh.indices.is_empty() {
            continue;
        }
        let base = u32::try_from(positions.len()).ok()?;
        positions.extend_from_slice(&mesh.positions);
        normals.extend_from_slice(&mesh.normals);
        indices.extend(mesh.indices.iter().map(|index| base + *index));
        bounds = bounds.union(&mesh.bounds);
    }
    if positions.is_empty() || indices.is_empty() {
        return None;
    }
    Some(TriangleMesh {
        positions,
        normals,
        indices,
        bounds,
    })
}

fn write_geometry_only_glb(mesh: &TriangleMesh, path: &Path) -> anyhow::Result<()> {
    validate_export_mesh(mesh)?;
    let normals = export_normals(mesh)?;
    let (min_position, max_position) = position_min_max(&mesh.positions);

    let mut bin = Vec::new();
    let positions_offset = bin.len();
    push_f32_triplets(&mut bin, &mesh.positions);
    pad_to_4(&mut bin, 0);
    let positions_length = bin.len() - positions_offset;

    let normals_offset = bin.len();
    push_f32_triplets(&mut bin, &normals);
    pad_to_4(&mut bin, 0);
    let normals_length = bin.len() - normals_offset;

    let indices_offset = bin.len();
    push_u32_values(&mut bin, &mesh.indices);
    pad_to_4(&mut bin, 0);
    let indices_length = bin.len() - indices_offset;

    let json = serde_json::json!({
        "asset": {
            "version": "2.0",
            "generator": "Shape Lab ObjectPlan geometry export v0"
        },
        "scene": 0,
        "scenes": [
            {"nodes": [0]}
        ],
        "nodes": [
            {"mesh": 0, "name": "GeometryOnlyAsset"}
        ],
        "meshes": [
            {
                "name": "ObjectPlanGeometry",
                "primitives": [
                    {
                        "attributes": {
                            "POSITION": 0,
                            "NORMAL": 1
                        },
                        "indices": 2,
                        "material": 0,
                        "mode": 4
                    }
                ]
            }
        ],
        "materials": [
            {
                "name": "Neutral visibility placeholder",
                "pbrMetallicRoughness": {
                    "baseColorFactor": [0.72, 0.72, 0.70, 1.0],
                    "metallicFactor": 0.0,
                    "roughnessFactor": 1.0
                }
            }
        ],
        "buffers": [
            {"byteLength": bin.len()}
        ],
        "bufferViews": [
            {
                "buffer": 0,
                "byteOffset": positions_offset,
                "byteLength": positions_length,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": normals_offset,
                "byteLength": normals_length,
                "target": 34962
            },
            {
                "buffer": 0,
                "byteOffset": indices_offset,
                "byteLength": indices_length,
                "target": 34963
            }
        ],
        "accessors": [
            {
                "bufferView": 0,
                "byteOffset": 0,
                "componentType": 5126,
                "count": mesh.positions.len(),
                "type": "VEC3",
                "min": min_position,
                "max": max_position
            },
            {
                "bufferView": 1,
                "byteOffset": 0,
                "componentType": 5126,
                "count": normals.len(),
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "byteOffset": 0,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ]
    });
    let mut json_bytes = serde_json::to_vec(&json)?;
    pad_to_4(&mut json_bytes, b' ');

    let total_length = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let total_length = u32::try_from(total_length).context("GLB is too large")?;
    let json_length = u32::try_from(json_bytes.len()).context("GLB JSON chunk is too large")?;
    let bin_length = u32::try_from(bin.len()).context("GLB BIN chunk is too large")?;

    let mut glb = Vec::with_capacity(total_length as usize);
    glb.extend_from_slice(b"glTF");
    glb.extend_from_slice(&2_u32.to_le_bytes());
    glb.extend_from_slice(&total_length.to_le_bytes());
    glb.extend_from_slice(&json_length.to_le_bytes());
    glb.extend_from_slice(b"JSON");
    glb.extend_from_slice(&json_bytes);
    glb.extend_from_slice(&bin_length.to_le_bytes());
    glb.extend_from_slice(b"BIN\0");
    glb.extend_from_slice(&bin);

    fs::write(path, glb).with_context(|| format!("writing GLB {}", path.display()))?;
    Ok(())
}

fn validate_export_mesh(mesh: &TriangleMesh) -> anyhow::Result<()> {
    if mesh.positions.is_empty() {
        anyhow::bail!("geometry export mesh has no positions");
    }
    if mesh.indices.is_empty() {
        anyhow::bail!("geometry export mesh has no indices");
    }
    if !mesh.indices.len().is_multiple_of(3) {
        anyhow::bail!("geometry export mesh indices are not triangles");
    }
    if mesh
        .positions
        .iter()
        .flatten()
        .any(|value| !value.is_finite())
    {
        anyhow::bail!("geometry export mesh positions must be finite");
    }
    if mesh
        .indices
        .iter()
        .any(|index| *index as usize >= mesh.positions.len())
    {
        anyhow::bail!("geometry export mesh indices reference missing positions");
    }
    Ok(())
}

fn export_normals(mesh: &TriangleMesh) -> anyhow::Result<Vec<[f32; 3]>> {
    if mesh.normals.len() == mesh.positions.len()
        && mesh.normals.iter().flatten().all(|value| value.is_finite())
    {
        return Ok(mesh.normals.clone());
    }

    let mut normals = vec![Vec3::ZERO; mesh.positions.len()];
    for triangle in mesh.indices.chunks_exact(3) {
        let a = Vec3::from_array(mesh.positions[triangle[0] as usize]);
        let b = Vec3::from_array(mesh.positions[triangle[1] as usize]);
        let c = Vec3::from_array(mesh.positions[triangle[2] as usize]);
        let face_normal = (b - a).cross(c - a);
        if face_normal.length_squared() <= f32::EPSILON || !face_normal.is_finite() {
            continue;
        }
        for index in triangle {
            normals[*index as usize] += face_normal;
        }
    }

    Ok(normals
        .into_iter()
        .map(|normal| normal.normalize_or(Vec3::Y).to_array())
        .collect())
}

fn position_min_max(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in positions {
        for axis in 0..3 {
            min[axis] = min[axis].min(position[axis]);
            max[axis] = max[axis].max(position[axis]);
        }
    }
    (min, max)
}

fn push_f32_triplets(buffer: &mut Vec<u8>, values: &[[f32; 3]]) {
    for value in values {
        for component in value {
            buffer.extend_from_slice(&component.to_le_bytes());
        }
    }
}

fn push_u32_values(buffer: &mut Vec<u8>, values: &[u32]) {
    for value in values {
        buffer.extend_from_slice(&value.to_le_bytes());
    }
}

fn pad_to_4(buffer: &mut Vec<u8>, byte: u8) {
    while !buffer.len().is_multiple_of(4) {
        buffer.push(byte);
    }
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
    validation_passed: bool,
    validation_issue_count: usize,
}

#[derive(Debug)]
struct ObjectPlanMaterializationOutcome {
    plan_id: String,
    display_name: String,
    status: MaterializationStatus,
    validation_passed: bool,
    validation_issue_count: usize,
    materialization_report: ObjectPlanMaterializationReport,
    render_evidence_report: Option<ObjectPlanRenderEvidenceReport>,
    draft: MaterializedObjectDraft,
}

#[derive(Debug)]
struct ObjectPlanGeometryExportOutcome {
    plan_id: String,
    status: GeometryExportStatus,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectPlanMaterializationReport {
    status: MaterializationStatus,
    primitive_count: usize,
    materialized_primitive_count: usize,
    attachment_count: usize,
    materialized_attachment_count: usize,
    unresolved_nodes: Vec<shape_foundry::UnresolvedObjectPlanNode>,
    unresolved_attachments: Vec<shape_foundry::UnresolvedObjectPlanAttachment>,
    user_review_required: bool,
    publish_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ObjectPlanRenderEvidenceReport {
    rendered: bool,
    materialized: bool,
    plan_id: String,
    preview_count: usize,
    contact_sheet_path: Option<String>,
    unsupported_primitives: Vec<String>,
    unsupported_attachments: Vec<String>,
    warnings: Vec<String>,
    user_review_required: bool,
    approved: bool,
}

#[derive(Debug)]
struct MaterializedPreviewSet {
    plan_preview: RenderedImage,
    node_previews: Vec<NodePreview>,
}

#[derive(Debug)]
struct NodePreview {
    node_id: String,
    image: RenderedImage,
}

#[derive(Debug, Copy, Clone)]
struct PrimitiveDimensions {
    width: f32,
    height: f32,
    depth: f32,
}

#[derive(Debug)]
struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    bounds: Aabb,
}

impl Default for MeshBuilder {
    fn default() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
            bounds: Aabb::empty(),
        }
    }
}

impl MeshBuilder {
    fn add_quad(&mut self, corners: [Vec3; 4], normal: Vec3) {
        let base = self.positions.len() as u32;
        self.positions
            .extend(corners.iter().map(|corner| corner.to_array()));
        self.normals.extend([normal.to_array(); 4]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        for corner in corners {
            self.bounds = self.bounds.union(&Aabb {
                min: corner,
                max: corner,
            });
        }
    }

    fn finish(self) -> TriangleMesh {
        TriangleMesh {
            positions: self.positions,
            normals: self.normals,
            indices: self.indices,
            bounds: self.bounds,
        }
    }
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
    publish_allowed: bool,
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
    materialization_status: String,
    rendered: bool,
    unsupported: bool,
    recommendation: BatchReviewRecommendation,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchMaterializationReport {
    batch_id: String,
    display_name: String,
    total_plans: usize,
    passed: usize,
    partial: usize,
    failed: usize,
    human_review_required: bool,
    approved: bool,
    publish_allowed: bool,
    plans: Vec<ObjectPlanBatchMaterializationPlanReport>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchMaterializationPlanReport {
    source_ref: String,
    output_dir: String,
    plan_id: Option<String>,
    status: String,
    primitive_count: usize,
    materialized_primitive_count: usize,
    attachment_count: usize,
    materialized_attachment_count: usize,
    unresolved_node_count: usize,
    unresolved_attachment_count: usize,
    user_review_required: bool,
    publish_allowed: bool,
    errors: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchRenderEvidenceReport {
    batch_id: String,
    display_name: String,
    total_plans: usize,
    rendered: usize,
    blocked: usize,
    contact_sheet_path: Option<String>,
    human_review_required: bool,
    approved: bool,
    publish_allowed: bool,
    plans: Vec<ObjectPlanBatchRenderEvidencePlanReport>,
}

#[derive(Debug, Serialize)]
struct ObjectPlanBatchRenderEvidencePlanReport {
    source_ref: String,
    output_dir: String,
    plan_id: Option<String>,
    rendered: bool,
    materialized: bool,
    preview_count: usize,
    contact_sheet_path: Option<String>,
    warnings: Vec<String>,
    user_review_required: bool,
    approved: bool,
    publish_allowed: bool,
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

fn materialization_status_label(status: MaterializationStatus) -> &'static str {
    match status {
        MaterializationStatus::Passed => "Passed",
        MaterializationStatus::Partial => "Partial",
        MaterializationStatus::Failed => "Failed",
    }
}

fn materialized_next_action_label(action: MaterializedObjectNextAction) -> &'static str {
    match action {
        MaterializedObjectNextAction::Review => "Review",
        MaterializedObjectNextAction::Simplify => "Simplify",
        MaterializedObjectNextAction::Regenerate => "Regenerate",
        MaterializedObjectNextAction::Blocked => "Blocked",
    }
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
