use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Subcommand;
use shape_foundry::{
    FoundationDraftValidationReport, FoundryFoundationDraft, foundation_adversarial_report,
    foundation_draft_template, materialize_foundation_draft_package, suggest_foundation_repairs,
    validate_foundation_draft, weapon_armor_foundation_batch_summary,
    weapon_armor_foundation_draft_batch,
};

use crate::write_json;

/// Commands for SDK-free Foundry foundation drafts.
#[derive(Debug, clap::Args)]
pub struct FoundryFoundationArgs {
    /// Foundation draft operation.
    #[command(subcommand)]
    pub command: FoundryFoundationCommand,
}

/// Foundry foundation draft subcommands.
#[derive(Debug, Subcommand)]
pub enum FoundryFoundationCommand {
    /// Create a deterministic foundation draft.
    New {
        /// Product category.
        #[arg(long)]
        category: String,
        /// Family ID.
        #[arg(long)]
        family: String,
        /// Output draft JSON path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate a foundation draft JSON file.
    Validate {
        /// Draft JSON path.
        draft: PathBuf,
    },
    /// Materialize a foundation draft into an internal kit package draft.
    Materialize {
        /// Draft JSON path.
        draft: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Write a deterministic adversarial report.
    AdversarialReport {
        /// Draft JSON path.
        draft: PathBuf,
        /// Output report JSON path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Suggest deterministic repairs from a validation report.
    SuggestRepair {
        /// Draft JSON path.
        draft: PathBuf,
        /// Validation report JSON path.
        #[arg(long)]
        validation_report: PathBuf,
        /// Output repair JSON path.
        #[arg(long)]
        out: PathBuf,
    },
    /// Export the deterministic Wave 37 weapon/armor foundation batch.
    Batch {
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// Run a Foundry foundation CLI command.
pub fn run_foundry_foundation(args: FoundryFoundationArgs) -> anyhow::Result<()> {
    match args.command {
        FoundryFoundationCommand::New {
            category,
            family,
            out,
        } => run_new(&category, &family, &out),
        FoundryFoundationCommand::Validate { draft } => run_validate(&draft),
        FoundryFoundationCommand::Materialize { draft, out_dir } => {
            run_materialize(&draft, &out_dir)
        }
        FoundryFoundationCommand::AdversarialReport { draft, out } => {
            run_adversarial_report(&draft, &out)
        }
        FoundryFoundationCommand::SuggestRepair {
            draft,
            validation_report,
            out,
        } => run_suggest_repair(&draft, &validation_report, &out),
        FoundryFoundationCommand::Batch { out_dir } => run_batch(&out_dir),
    }
}

fn run_new(category: &str, family: &str, out: &Path) -> anyhow::Result<()> {
    let draft = foundation_draft_template(category, family);
    ensure_parent_dir(out)?;
    write_json(out, &draft)?;
    println!("Wrote Foundry foundation draft to {}", out.display());
    Ok(())
}

fn run_validate(path: &Path) -> anyhow::Result<()> {
    let draft = load_draft(path)?;
    let report = validate_foundation_draft(&draft);
    print_report(&draft, &report);
    if report.is_valid() {
        Ok(())
    } else {
        bail!(
            "Foundry foundation draft validation failed with {} issue(s)",
            report.issues.len()
        )
    }
}

fn run_materialize(path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let draft = load_draft(path)?;
    let report = validate_foundation_draft(&draft);
    write_json(out_dir.join("foundation-validation.json"), &report)?;
    if !report.is_valid() {
        bail!(
            "Foundry foundation draft validation failed with {} issue(s)",
            report.issues.len()
        );
    }
    let package = materialize_foundation_draft_package(&draft)
        .map_err(|report| anyhow::anyhow!("materialization validation failed: {report:#?}"))?;
    write_json(out_dir.join("foundation-draft.json"), &draft)?;
    write_json(out_dir.join("foundry-kit-package.json"), &package)?;
    write_json(out_dir.join("kit-manifest.json"), &package.kit)?;
    write_json(
        out_dir.join("family-blueprint.json"),
        &package.family_blueprint,
    )?;
    write_json(out_dir.join("provider-pack.json"), &package.provider_pack)?;
    write_json(out_dir.join("style-pack.json"), &package.style_pack)?;
    write_json(
        out_dir.join("control-profile.json"),
        &package.control_profile,
    )?;
    write_json(
        out_dir.join("candidate-strategy-pack.json"),
        &package.candidate_strategy_pack,
    )?;
    write_json(
        out_dir.join("quality-gate-profile.json"),
        &package.quality_gate_profile,
    )?;
    write_json(
        out_dir.join("compatibility-matrix.json"),
        &package.compatibility_matrix,
    )?;
    write_json(
        out_dir.join("review-manifest.json"),
        &package.review_manifest,
    )?;
    write_json(
        out_dir.join("kit-catalog-manifest.json"),
        &package.catalog_manifest,
    )?;
    println!(
        "Materialized Foundry foundation draft {} into {}",
        draft.draft_id,
        out_dir.display()
    );
    Ok(())
}

fn run_adversarial_report(path: &Path, out: &Path) -> anyhow::Result<()> {
    let draft = load_draft(path)?;
    let report = foundation_adversarial_report(&draft);
    ensure_parent_dir(out)?;
    write_json(out, &report)?;
    println!("Wrote foundation adversarial report to {}", out.display());
    Ok(())
}

fn run_suggest_repair(
    draft_path: &Path,
    validation_report_path: &Path,
    out: &Path,
) -> anyhow::Result<()> {
    let draft = load_draft(draft_path)?;
    let report = load_validation_report(validation_report_path)?;
    let repair = suggest_foundation_repairs(
        &draft,
        validation_report_path.display().to_string(),
        &report,
    );
    ensure_parent_dir(out)?;
    write_json(out, &repair)?;
    println!("Wrote foundation repair suggestions to {}", out.display());
    Ok(())
}

fn run_batch(out_dir: &Path) -> anyhow::Result<()> {
    let draft_dir = out_dir.join("drafts");
    let validation_dir = out_dir.join("validation");
    let adversarial_dir = out_dir.join("adversarial");
    fs::create_dir_all(&draft_dir).with_context(|| format!("creating {}", draft_dir.display()))?;
    fs::create_dir_all(&validation_dir)
        .with_context(|| format!("creating {}", validation_dir.display()))?;
    fs::create_dir_all(&adversarial_dir)
        .with_context(|| format!("creating {}", adversarial_dir.display()))?;

    let drafts = weapon_armor_foundation_draft_batch();
    for draft in &drafts {
        let family_id = &draft.family_blueprint.family_id;
        let report = validate_foundation_draft(draft);
        let adversarial = foundation_adversarial_report(draft);
        write_json(
            draft_dir.join(format!("{family_id}.foundation-draft.json")),
            draft,
        )?;
        write_json(
            validation_dir.join(format!("{family_id}.validation.json")),
            &report,
        )?;
        write_json(
            adversarial_dir.join(format!("{family_id}.adversarial-report.json")),
            &adversarial,
        )?;
        if !report.is_valid() {
            bail!(
                "Wave 37 foundation draft {} failed validation with {} issue(s)",
                draft.draft_id,
                report.issues.len()
            );
        }
    }
    write_json(
        out_dir.join("foundation-batch-summary.json"),
        &weapon_armor_foundation_batch_summary(),
    )?;
    println!(
        "Wrote {} Wave 37 foundation drafts to {}",
        drafts.len(),
        out_dir.display()
    );
    Ok(())
}

fn load_draft(path: &Path) -> anyhow::Result<FoundryFoundationDraft> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn load_validation_report(path: &Path) -> anyhow::Result<FoundationDraftValidationReport> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("parsing {}", path.display()))
}

fn print_report(draft: &FoundryFoundationDraft, report: &FoundationDraftValidationReport) {
    println!("Foundry foundation draft {}", draft.draft_id);
    println!(
        "  status: {}",
        if report.is_valid() {
            "valid"
        } else {
            "invalid"
        }
    );
    for issue in &report.issues {
        eprintln!("  [{}] {}: {}", issue.code, issue.subject, issue.message);
    }
}

fn ensure_parent_dir(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    Ok(())
}
