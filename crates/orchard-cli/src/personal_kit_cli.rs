use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::Subcommand;
use orchard_foundry::{
    DirectKitDraft, list_personal_kits, save_direct_kit, validate_personal_kit_store,
};

use crate::write_json;

/// Save, list, and validate local/private Personal Kits.
#[derive(Debug, clap::Args)]
pub struct PersonalKitArgs {
    /// Personal Kit operation.
    #[command(subcommand)]
    pub command: PersonalKitCommand,
}

/// Personal Kit CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum PersonalKitCommand {
    /// Save a Direct Kit as a local/private kit.
    Save {
        /// Direct Kit JSON file.
        #[arg(long)]
        kit: PathBuf,
        /// Store base directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// List saved local/private kits.
    List {
        /// Store base directory.
        #[arg(long)]
        store: PathBuf,
    },
    /// Validate a local/private kit store.
    Validate {
        /// Store base directory.
        #[arg(long)]
        store: PathBuf,
    },
}

/// Run a Personal Kit command.
pub fn run_personal_kit(args: PersonalKitArgs) -> anyhow::Result<()> {
    match args.command {
        PersonalKitCommand::Save { kit, out_dir } => run_save(&kit, &out_dir),
        PersonalKitCommand::List { store } => run_list(&store),
        PersonalKitCommand::Validate { store } => run_validate(&store),
    }
}

fn run_save(kit_path: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let bytes =
        fs::read(kit_path).with_context(|| format!("reading Direct Kit {}", kit_path.display()))?;
    let kit: DirectKitDraft = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing Direct Kit {}", kit_path.display()))?;
    let stored = save_direct_kit(out_dir, &kit).map_err(|error| anyhow::anyhow!(error))?;
    write_json(
        out_dir.join("personal-kit-save-report.json"),
        &serde_json::json!({
            "status": "Passed",
            "kit_id": stored.kit_id,
            "visibility": stored.visibility,
            "novice_visible": stored.novice_visible,
            "public_catalog_visible": stored.public_catalog_visible,
            "store": "personal-kits"
        }),
    )?;
    println!(
        "Saved Personal Kit {} into {}",
        kit.kit_id,
        out_dir.display()
    );
    Ok(())
}

fn run_list(store: &Path) -> anyhow::Result<()> {
    let manifest = list_personal_kits(store).map_err(|error| anyhow::anyhow!(error))?;
    println!("{}", serde_json::to_string_pretty(&manifest)?);
    Ok(())
}

fn run_validate(store: &Path) -> anyhow::Result<()> {
    let report = validate_personal_kit_store(store);
    println!("{}", serde_json::to_string_pretty(&report)?);
    if report.is_valid() {
        Ok(())
    } else {
        anyhow::bail!(
            "Personal Kit store validation failed with {} error(s)",
            report.errors.len()
        )
    }
}
