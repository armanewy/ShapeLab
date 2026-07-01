use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use serde::Serialize;
use shape_foundry::{
    DirectKitDraft, DirectKitEvidenceKind, DirectKitEvidenceStatus, DirectKitPropertyExposure,
    DirectKitSourceKind, PrimitivePropertyDomain, PrimitivePropertyValue, direct_kit_user_summary,
    validate_direct_kit_draft,
};

use crate::write_json;

/// Test Direct Kits deterministically.
#[derive(Debug, clap::Args)]
pub struct DirectKitArgs {
    /// Direct Kit operation.
    #[command(subcommand)]
    pub command: DirectKitCommand,
}

/// Direct Kit CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum DirectKitCommand {
    /// Run deterministic Direct Kit tests.
    Test {
        /// Direct Kit JSON file.
        #[arg(long)]
        kit: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// Run a Direct Kit command.
pub fn run_direct_kit(args: DirectKitArgs) -> anyhow::Result<()> {
    match args.command {
        DirectKitCommand::Test { kit, out_dir } => run_test(&kit, &out_dir),
    }
}

fn run_test(kit_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let bytes =
        fs::read(kit_path).with_context(|| format!("reading Direct Kit {}", kit_path.display()))?;
    let kit: DirectKitDraft = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing Direct Kit {}", kit_path.display()))?;
    let validation = validate_direct_kit_draft(&kit);
    let property_report = property_endpoint_report(&kit);
    let capability_results = capability_results(&property_report);
    let preset_report = (!kit.included_presets.is_empty()).then(|| preset_evidence_report(&kit));
    let object_plan_report = (kit.source_kind == DirectKitSourceKind::ObjectPlan)
        .then(|| object_plan_evidence_report(&kit));

    let mut suggested_repairs = Vec::new();
    if !validation.errors.is_empty() {
        suggested_repairs.push("Fix invalid kit properties before testing again.".to_owned());
    }
    if validation
        .warnings
        .iter()
        .any(|issue| issue.code == "direct_kit_missing_evidence")
    {
        suggested_repairs.push("Add review evidence when available.".to_owned());
    }
    if preset_report
        .as_ref()
        .is_some_and(|report| report.status == DirectKitTestStatus::Warnings)
    {
        suggested_repairs.push("Add preset contact sheet evidence when available.".to_owned());
    }
    if object_plan_report
        .as_ref()
        .is_some_and(|report| report.status == DirectKitTestStatus::Warnings)
    {
        suggested_repairs.push("Add ObjectPlan review evidence when available.".to_owned());
    }

    let failed_capabilities = capability_results
        .results
        .iter()
        .filter(|result| result.status == DirectKitTestStatus::Failed)
        .count();
    let warned_capabilities = capability_results
        .results
        .iter()
        .filter(|result| result.status == DirectKitTestStatus::Warnings)
        .count()
        + usize::from(
            preset_report
                .as_ref()
                .is_some_and(|report| report.status == DirectKitTestStatus::Warnings),
        )
        + usize::from(
            object_plan_report
                .as_ref()
                .is_some_and(|report| report.status == DirectKitTestStatus::Warnings),
        );
    let status = if !validation.errors.is_empty() || failed_capabilities > 0 {
        DirectKitTestStatus::Failed
    } else if !validation.warnings.is_empty() || warned_capabilities > 0 {
        DirectKitTestStatus::Warnings
    } else {
        DirectKitTestStatus::Passed
    };
    let tested_capabilities = capability_results.results.len();
    let report = DirectKitTestReport {
        status,
        kit_id: kit.kit_id.clone(),
        tested_capabilities,
        passed_capabilities: capability_results
            .results
            .iter()
            .filter(|result| result.status == DirectKitTestStatus::Passed)
            .count(),
        warned_capabilities,
        failed_capabilities,
        suggested_repairs,
        human_review_required: true,
        approved: false,
        publish_allowed: false,
    };

    write_json(
        out_dir.join("property-endpoint-report.json"),
        &property_report,
    )?;
    write_json(out_dir.join("capability-results.json"), &capability_results)?;
    if let Some(preset_report) = &preset_report {
        write_json(out_dir.join("preset-evidence-report.json"), preset_report)?;
    }
    if let Some(object_plan_report) = &object_plan_report {
        write_json(
            out_dir.join("object-plan-evidence-report.json"),
            object_plan_report,
        )?;
    }
    write_json(out_dir.join("direct-kit-test-report.json"), &report)?;
    fs::write(out_dir.join("user-summary.md"), user_summary(&kit, &report))?;

    if report.status == DirectKitTestStatus::Failed {
        anyhow::bail!("Direct Kit test failed");
    }
    println!(
        "Tested Direct Kit {} into {}",
        kit.kit_id,
        out_dir.display()
    );
    Ok(())
}

fn capability_results(property_report: &PropertyEndpointReport) -> DirectKitCapabilityResults {
    let results = property_report
        .properties
        .iter()
        .map(|property| DirectKitCapabilityResult {
            capability_id: property.property_id.clone(),
            display_name: property.display_name.clone(),
            status: property.status,
            message: if property.status == DirectKitTestStatus::Passed {
                format!("{} can be adjusted.", property.display_name)
            } else {
                format!("{} needs a valid property domain.", property.display_name)
            },
            visible_test_required: true,
        })
        .collect::<Vec<_>>();
    DirectKitCapabilityResults { results }
}

fn property_endpoint_report(kit: &DirectKitDraft) -> PropertyEndpointReport {
    let properties = kit
        .changeable_properties
        .iter()
        .map(property_endpoint_result)
        .collect::<Vec<_>>();
    let failed = properties
        .iter()
        .filter(|property| property.status == DirectKitTestStatus::Failed)
        .count();
    PropertyEndpointReport {
        kit_id: kit.kit_id.clone(),
        status: if failed > 0 {
            DirectKitTestStatus::Failed
        } else {
            DirectKitTestStatus::Passed
        },
        properties,
        evidence_exists: kit.evidence_refs.iter().any(|evidence| {
            evidence.evidence_kind == DirectKitEvidenceKind::PropertyEndpointSheet
                && evidence.status == DirectKitEvidenceStatus::Passed
        }),
    }
}

fn property_endpoint_result(property: &DirectKitPropertyExposure) -> PropertyEndpointResult {
    let mut endpoints = Vec::new();
    let mut valid = true;
    match &property.domain {
        PrimitivePropertyDomain::Length {
            minimum, maximum, ..
        } => {
            endpoints.push(format!("minimum={minimum}"));
            endpoints.push(format!("maximum={maximum}"));
            valid &= matches!(property.current_value, PrimitivePropertyValue::Length(_));
            valid &= matches!(property.default_value, PrimitivePropertyValue::Length(_));
        }
        PrimitivePropertyDomain::Ratio {
            minimum, maximum, ..
        } => {
            endpoints.push(format!("minimum={minimum}"));
            endpoints.push(format!("maximum={maximum}"));
            valid &= matches!(property.current_value, PrimitivePropertyValue::Ratio(_));
            valid &= matches!(property.default_value, PrimitivePropertyValue::Ratio(_));
        }
        PrimitivePropertyDomain::Boolean => {
            endpoints.push("false".to_owned());
            endpoints.push("true".to_owned());
            valid &= matches!(property.current_value, PrimitivePropertyValue::Boolean(_));
            valid &= matches!(property.default_value, PrimitivePropertyValue::Boolean(_));
        }
        PrimitivePropertyDomain::Choice { options } => {
            endpoints.extend(options.iter().map(|option| option.display_name.clone()));
            valid &= matches!(property.current_value, PrimitivePropertyValue::Choice(_));
            valid &= matches!(property.default_value, PrimitivePropertyValue::Choice(_));
        }
        PrimitivePropertyDomain::Angle {
            minimum_degrees,
            maximum_degrees,
            ..
        } => {
            endpoints.push(format!("minimum={minimum_degrees}"));
            endpoints.push(format!("maximum={maximum_degrees}"));
            valid &= matches!(property.current_value, PrimitivePropertyValue::Angle(_));
            valid &= matches!(property.default_value, PrimitivePropertyValue::Angle(_));
        }
    }
    PropertyEndpointResult {
        property_id: property.property_id.clone(),
        display_name: property.display_name.clone(),
        status: if valid {
            DirectKitTestStatus::Passed
        } else {
            DirectKitTestStatus::Failed
        },
        endpoints,
        default_checked: true,
    }
}

fn preset_evidence_report(kit: &DirectKitDraft) -> DirectKitEvidenceTestReport {
    let evidence_exists = kit.evidence_refs.iter().any(|evidence| {
        evidence.evidence_kind == DirectKitEvidenceKind::PresetContactSheet
            && evidence.status == DirectKitEvidenceStatus::Passed
    });
    DirectKitEvidenceTestReport {
        kit_id: kit.kit_id.clone(),
        status: if evidence_exists {
            DirectKitTestStatus::Passed
        } else {
            DirectKitTestStatus::Warnings
        },
        evidence_exists,
        message: if evidence_exists {
            "Preset contact sheet is linked.".to_owned()
        } else {
            "No contact sheet was generated for this kit yet.".to_owned()
        },
    }
}

fn object_plan_evidence_report(kit: &DirectKitDraft) -> DirectKitEvidenceTestReport {
    let evidence_exists = kit.evidence_refs.iter().any(|evidence| {
        evidence.evidence_kind == DirectKitEvidenceKind::ObjectPlanRenderEvidence
            && evidence.status == DirectKitEvidenceStatus::Passed
    });
    DirectKitEvidenceTestReport {
        kit_id: kit.kit_id.clone(),
        status: if evidence_exists {
            DirectKitTestStatus::Passed
        } else {
            DirectKitTestStatus::Warnings
        },
        evidence_exists,
        message: if evidence_exists {
            "Draft review images are linked.".to_owned()
        } else {
            "No draft review images were linked for this kit yet.".to_owned()
        },
    }
}

fn user_summary(kit: &DirectKitDraft, report: &DirectKitTestReport) -> String {
    let summary = direct_kit_user_summary(kit);
    let mut lines = vec![
        format!("# {}", summary.title),
        String::new(),
        summary.what_this_is,
        String::new(),
        format!("Test status: {:?}", report.status),
        "This kit is a Draft and needs review.".to_owned(),
        "No automatic approval or sharing is included.".to_owned(),
        String::new(),
        "## What Can Change".to_owned(),
    ];
    lines.extend(
        summary
            .can_change
            .into_iter()
            .map(|line| format!("- {line}")),
    );
    if !report.suggested_repairs.is_empty() {
        lines.push(String::new());
        lines.push("## Next Steps".to_owned());
        lines.extend(
            report
                .suggested_repairs
                .iter()
                .map(|repair| format!("- {repair}")),
        );
    }
    lines.push(String::new());
    lines.join("\n")
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
enum DirectKitTestStatus {
    Passed,
    Warnings,
    Failed,
}

#[derive(Debug, Serialize)]
struct DirectKitTestReport {
    status: DirectKitTestStatus,
    kit_id: String,
    tested_capabilities: usize,
    passed_capabilities: usize,
    warned_capabilities: usize,
    failed_capabilities: usize,
    suggested_repairs: Vec<String>,
    human_review_required: bool,
    approved: bool,
    publish_allowed: bool,
}

#[derive(Debug, Serialize)]
struct DirectKitCapabilityResults {
    results: Vec<DirectKitCapabilityResult>,
}

#[derive(Debug, Serialize)]
struct DirectKitCapabilityResult {
    capability_id: String,
    display_name: String,
    status: DirectKitTestStatus,
    message: String,
    visible_test_required: bool,
}

#[derive(Debug, Serialize)]
struct PropertyEndpointReport {
    kit_id: String,
    status: DirectKitTestStatus,
    properties: Vec<PropertyEndpointResult>,
    evidence_exists: bool,
}

#[derive(Debug, Serialize)]
struct PropertyEndpointResult {
    property_id: String,
    display_name: String,
    status: DirectKitTestStatus,
    endpoints: Vec<String>,
    default_checked: bool,
}

#[derive(Debug, Serialize)]
struct DirectKitEvidenceTestReport {
    kit_id: String,
    status: DirectKitTestStatus,
    evidence_exists: bool,
    message: String,
}
