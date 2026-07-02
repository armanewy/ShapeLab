use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use orchard_foundry::{
    archetype_draft_materialization_report, materialize_archetype_foundation_draft,
    validate_foundation_draft,
};

use crate::write_json;

/// Internal/pro Foundry authoring commands.
#[derive(Debug, clap::Args)]
pub struct FoundryArgs {
    /// Foundry operation.
    #[command(subcommand)]
    pub command: FoundryCommand,
}

/// Foundry authoring subcommands.
#[derive(Debug, Subcommand)]
pub enum FoundryCommand {
    /// Materialize an internal foundation draft from one supported archetype.
    MaterializeArchetype {
        /// Archetype ID. v0 supports box-primitive only.
        #[arg(long)]
        archetype: String,
        /// New family ID.
        #[arg(long)]
        family_id: String,
        /// New style ID.
        #[arg(long)]
        style_id: String,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
}

/// Run a Foundry authoring command.
pub fn run_foundry(args: FoundryArgs) -> anyhow::Result<()> {
    match args.command {
        FoundryCommand::MaterializeArchetype {
            archetype,
            family_id,
            style_id,
            out_dir,
        } => run_materialize_archetype(&archetype, &family_id, &style_id, &out_dir),
    }
}

fn run_materialize_archetype(
    archetype: &str,
    family_id: &str,
    style_id: &str,
    out_dir: &Path,
) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let draft = materialize_archetype_foundation_draft(archetype, family_id, style_id)
        .map_err(anyhow::Error::msg)?;
    let validation = validate_foundation_draft(&draft);
    if !validation.is_valid() {
        anyhow::bail!(
            "archetype draft validation failed with {} issue(s): {:#?}",
            validation.issues.len(),
            validation.issues
        );
    }

    let quality_gate = draft
        .quality_gate_profile
        .as_ref()
        .expect("validated archetype draft has a quality gate");
    let generated_files = vec![
        "family-blueprint-draft.json".to_owned(),
        "provider-taxonomy-draft.json".to_owned(),
        "style-pack-draft.json".to_owned(),
        "control-profile-draft.json".to_owned(),
        "candidate-strategy-draft.json".to_owned(),
        "quality-gate-draft.json".to_owned(),
        "test-plan-draft.json".to_owned(),
        "review-checklist.md".to_owned(),
        "materialization-report.json".to_owned(),
    ];
    write_json(
        out_dir.join("family-blueprint-draft.json"),
        &draft.family_blueprint,
    )?;
    write_json(
        out_dir.join("provider-taxonomy-draft.json"),
        &draft.provider_taxonomy,
    )?;
    write_json(out_dir.join("style-pack-draft.json"), &draft.style_pack)?;
    write_json(
        out_dir.join("control-profile-draft.json"),
        &draft.control_profile,
    )?;
    write_json(
        out_dir.join("candidate-strategy-draft.json"),
        &draft.candidate_strategy_pack,
    )?;
    write_json(out_dir.join("quality-gate-draft.json"), quality_gate)?;
    write_json(out_dir.join("test-plan-draft.json"), &draft.test_plan)?;
    fs::write(
        out_dir.join("review-checklist.md"),
        review_checklist_markdown(&draft.review_checklist.items),
    )
    .with_context(|| format!("writing {}", out_dir.join("review-checklist.md").display()))?;
    let report = archetype_draft_materialization_report(&draft, generated_files);
    write_json(out_dir.join("materialization-report.json"), &report)?;
    println!(
        "Materialized {} archetype draft {} into {}",
        archetype,
        draft.family_blueprint.family_id,
        out_dir.display()
    );
    Ok(())
}

fn review_checklist_markdown(items: &[String]) -> String {
    let mut markdown = "# Archetype Draft Review Checklist\n\n".to_owned();
    markdown.push_str("Drafts are internal only and require human review before promotion.\n\n");
    for item in items {
        markdown.push_str("- [ ] ");
        markdown.push_str(item);
        markdown.push('\n');
    }
    markdown
}
