use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;
use clap::Subcommand;
use serde::Serialize;

use crate::write_json;

/// Godot proof commands.
#[derive(Debug, clap::Args)]
pub struct GodotProofArgs {
    /// Godot proof operation.
    #[command(subcommand)]
    pub command: GodotProofCommand,
}

/// Godot proof subcommands.
#[derive(Debug, Subcommand)]
pub enum GodotProofCommand {
    /// Prove whether a geometry-only GLB imports into Godot.
    GeometryImport {
        /// Source GLB.
        #[arg(long)]
        glb: PathBuf,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
        /// Optional Godot binary path.
        #[arg(long)]
        godot_bin: Option<PathBuf>,
    },
}

/// Run a Godot proof command.
pub fn run_godot_proof(args: GodotProofArgs) -> anyhow::Result<()> {
    match args.command {
        GodotProofCommand::GeometryImport {
            glb,
            out_dir,
            godot_bin,
        } => run_geometry_import(&glb, &out_dir, godot_bin.as_deref()),
    }
}

fn run_geometry_import(
    glb_path: &Path,
    out_dir: &Path,
    godot_bin: Option<&Path>,
) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    if !glb_path.is_file() {
        let report = base_report(
            GodotImportProofStatus::Failed,
            false,
            None,
            glb_path,
            vec!["Source GLB path does not exist or is not a file.".to_owned()],
            Vec::new(),
        );
        write_json(out_dir.join("godot-import-proof-report.json"), &report)?;
        anyhow::bail!("source GLB path is invalid");
    }

    let Some(godot) = discover_godot_bin(godot_bin) else {
        let report = base_report(
            GodotImportProofStatus::Blocked,
            false,
            None,
            glb_path,
            vec!["Godot binary was not found; import proof was not run.".to_owned()],
            Vec::new(),
        );
        write_json(out_dir.join("godot-import-proof-report.json"), &report)?;
        println!(
            "Godot geometry import proof blocked for {}",
            glb_path.display()
        );
        return Ok(());
    };

    let version = godot_version(&godot, out_dir)?;
    let project_dir = out_dir.join("godot-project");
    fs::create_dir_all(&project_dir)
        .with_context(|| format!("creating {}", project_dir.display()))?;
    fs::write(
        project_dir.join("project.godot"),
        "; Engine configuration file.\nconfig_version=5\n\n[application]\nconfig/name=\"Object Orchard Geometry Import Proof\"\n",
    )
    .with_context(|| format!("writing {}", project_dir.join("project.godot").display()))?;
    fs::copy(glb_path, project_dir.join("asset.glb"))
        .with_context(|| format!("copying GLB into {}", project_dir.display()))?;

    let import_output = Command::new(&godot)
        .arg("--headless")
        .arg("--path")
        .arg(&project_dir)
        .arg("--import")
        .arg("--quit")
        .output()
        .with_context(|| format!("running Godot import with {}", godot.display()))?;
    fs::write(out_dir.join("stdout.log"), &import_output.stdout)
        .with_context(|| format!("writing {}", out_dir.join("stdout.log").display()))?;
    fs::write(out_dir.join("stderr.log"), &import_output.stderr)
        .with_context(|| format!("writing {}", out_dir.join("stderr.log").display()))?;

    let imported_files = imported_asset_files(&project_dir)?;
    let mesh_imported = import_output.status.success() && !imported_files.is_empty();
    let imported_asset_report = ImportedAssetReport {
        source_glb: persisted_glb_ref(glb_path),
        imported_files,
        mesh_imported,
        hierarchy_checked: false,
        imported_node_count: 0,
        imported_mesh_count: if mesh_imported { 1 } else { 0 },
        relationship_hierarchy_preserved: false,
        material_imported: false,
        collision_imported: false,
        rig_imported: false,
        animation_imported: false,
        game_ready: false,
        hierarchy_notes: vec!["Godot hierarchy inspection is not implemented in V0.".to_owned()],
    };
    write_json(
        out_dir.join("imported-asset-report.json"),
        &imported_asset_report,
    )?;

    let status = if mesh_imported {
        GodotImportProofStatus::Passed
    } else if import_output.status.success() {
        GodotImportProofStatus::Blocked
    } else {
        GodotImportProofStatus::Failed
    };
    let blockers = match status {
        GodotImportProofStatus::Passed => Vec::new(),
        GodotImportProofStatus::Blocked => {
            vec!["Godot ran, but no imported mesh resource was found.".to_owned()]
        }
        GodotImportProofStatus::Failed => vec!["Godot import command failed.".to_owned()],
    };
    let report = GodotImportProofReport {
        status,
        godot_available: true,
        godot_version: version,
        source_glb: persisted_glb_ref(glb_path),
        mesh_imported,
        hierarchy_checked: false,
        imported_node_count: 0,
        imported_mesh_count: if mesh_imported { 1 } else { 0 },
        relationship_hierarchy_preserved: false,
        material_imported: false,
        collision_imported: false,
        rig_imported: false,
        animation_imported: false,
        game_ready: false,
        blockers,
        logs: vec!["stdout.log".to_owned(), "stderr.log".to_owned()],
        hierarchy_notes: vec!["Godot hierarchy inspection is not implemented in V0.".to_owned()],
    };
    write_json(out_dir.join("godot-import-proof-report.json"), &report)?;

    if status == GodotImportProofStatus::Failed {
        anyhow::bail!("Godot geometry import proof failed");
    }
    Ok(())
}

fn discover_godot_bin(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return executable_path(path);
    }
    if let Ok(value) = env::var("GODOT_BIN")
        && let Some(path) = executable_path(Path::new(&value))
    {
        return Some(path);
    }
    for candidate in ["godot", "godot4"] {
        if let Some(path) = find_on_path(candidate) {
            return Some(path);
        }
    }
    None
}

fn executable_path(path: &Path) -> Option<PathBuf> {
    if path.is_file() {
        Some(path.to_path_buf())
    } else {
        None
    }
}

fn find_on_path(program: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn godot_version(godot: &Path, out_dir: &Path) -> anyhow::Result<Option<String>> {
    let output = Command::new(godot).arg("--version").output();
    match output {
        Ok(output) => {
            fs::write(out_dir.join("godot-version-stdout.log"), &output.stdout).with_context(
                || {
                    format!(
                        "writing {}",
                        out_dir.join("godot-version-stdout.log").display()
                    )
                },
            )?;
            fs::write(out_dir.join("godot-version-stderr.log"), &output.stderr).with_context(
                || {
                    format!(
                        "writing {}",
                        out_dir.join("godot-version-stderr.log").display()
                    )
                },
            )?;
            let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
            Ok((!version.is_empty()).then_some(version))
        }
        Err(_) => Ok(None),
    }
}

fn imported_asset_files(project_dir: &Path) -> anyhow::Result<Vec<String>> {
    let imported_dir = project_dir.join(".godot").join("imported");
    if !imported_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    collect_imported_asset_files(&imported_dir, &imported_dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_imported_asset_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<String>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry
            .with_context(|| format!("reading entry in {}", dir.display()))?
            .path();
        if path.is_dir() {
            collect_imported_asset_files(root, &path, files)?;
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.contains("asset"))
        {
            files.push(relative_path_string(root, &path));
        }
    }
    Ok(())
}

fn base_report(
    status: GodotImportProofStatus,
    godot_available: bool,
    godot_version: Option<String>,
    glb_path: &Path,
    blockers: Vec<String>,
    logs: Vec<String>,
) -> GodotImportProofReport {
    GodotImportProofReport {
        status,
        godot_available,
        godot_version,
        source_glb: persisted_glb_ref(glb_path),
        mesh_imported: false,
        hierarchy_checked: false,
        imported_node_count: 0,
        imported_mesh_count: 0,
        relationship_hierarchy_preserved: false,
        material_imported: false,
        collision_imported: false,
        rig_imported: false,
        animation_imported: false,
        game_ready: false,
        blockers,
        logs,
        hierarchy_notes: vec!["Godot hierarchy inspection was not run.".to_owned()],
    }
}

fn persisted_glb_ref(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("asset.glb")
        .to_owned()
}

fn relative_path_string(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
enum GodotImportProofStatus {
    Passed,
    Blocked,
    Failed,
}

#[derive(Debug, Serialize)]
struct GodotImportProofReport {
    status: GodotImportProofStatus,
    godot_available: bool,
    godot_version: Option<String>,
    source_glb: String,
    mesh_imported: bool,
    hierarchy_checked: bool,
    imported_node_count: usize,
    imported_mesh_count: usize,
    relationship_hierarchy_preserved: bool,
    material_imported: bool,
    collision_imported: bool,
    rig_imported: bool,
    animation_imported: bool,
    game_ready: bool,
    blockers: Vec<String>,
    logs: Vec<String>,
    hierarchy_notes: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ImportedAssetReport {
    source_glb: String,
    imported_files: Vec<String>,
    mesh_imported: bool,
    hierarchy_checked: bool,
    imported_node_count: usize,
    imported_mesh_count: usize,
    relationship_hierarchy_preserved: bool,
    material_imported: bool,
    collision_imported: bool,
    rig_imported: bool,
    animation_imported: bool,
    game_ready: bool,
    hierarchy_notes: Vec<String>,
}
