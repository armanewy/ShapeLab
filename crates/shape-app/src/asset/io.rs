//! Asset project file and export I/O helpers for the explicit asset app surface.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use shape_project::asset::{
    ASSET_PROJECT_FILE_SUFFIX, AssetAutosaveSnapshot, AssetGroupedObjReport,
    AssetModelExportPackagePaths, AssetProjectError, AssetProjectFile, autosave_snapshot_path,
    ensure_asset_project_path, has_asset_project_suffix,
};

/// Dialog filter label for explicit asset projects.
pub(crate) const ASSET_PROJECT_DIALOG_LABEL: &str = "Shape Lab Asset Project";

/// Return true when `path` points at a `.shapelab-asset.json` project.
pub(crate) fn is_asset_project_path(path: impl AsRef<Path>) -> bool {
    has_asset_project_suffix(path)
}

/// Normalize a title into a suggested asset project filename.
pub(crate) fn suggested_asset_project_file_name(title: &str) -> String {
    format!(
        "{}{}",
        conservative_file_stem(title, "asset-project"),
        ASSET_PROJECT_FILE_SUFFIX
    )
}

/// Save to the existing asset project path.
pub(crate) fn save_asset_project(file: &mut AssetProjectFile) -> Result<(), AssetProjectError> {
    file.save()
}

/// Save to a new asset project path.
pub(crate) fn save_asset_project_as(
    file: &mut AssetProjectFile,
    path: PathBuf,
) -> Result<(), AssetProjectError> {
    ensure_asset_project_path(&path)?;
    file.save_as(path)
}

/// Load an explicit asset project from disk.
pub(crate) fn load_asset_project(path: PathBuf) -> Result<AssetProjectFile, AssetProjectError> {
    AssetProjectFile::load(path)
}

/// Write an autosave recovery snapshot next to the current project path.
pub(crate) fn save_recovery_snapshot(
    file: &AssetProjectFile,
) -> Result<AssetAutosaveSnapshot, AssetProjectError> {
    let path = file
        .path
        .as_ref()
        .map(autosave_snapshot_path)
        .ok_or(AssetProjectError::MissingSavePath)?;
    file.save_autosave_snapshot(path)
}

/// Export the current asset model package without changing dirty state.
pub(crate) fn export_current_model_package(
    file: &AssetProjectFile,
    out_dir: PathBuf,
) -> Result<AssetModelExportPackagePaths, AssetProjectError> {
    file.export_current_model_package(out_dir)
}

/// Export the current asset model as grouped OBJ without changing dirty state.
pub(crate) fn export_current_obj(
    file: &AssetProjectFile,
    path: PathBuf,
) -> Result<AssetGroupedObjReport, AssetProjectError> {
    file.export_current_obj(path)
}

fn conservative_file_stem(title: &str, fallback: &str) -> String {
    let mut stem = String::new();
    let mut pending_separator = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_separator && !stem.is_empty() {
                stem.push('-');
            }
            stem.push(character.to_ascii_lowercase());
            pending_separator = false;
        } else if !stem.is_empty() {
            pending_separator = true;
        }
        if stem.len() >= 48 {
            break;
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() || is_windows_reserved_name(stem) {
        fallback.to_owned()
    } else {
        stem.to_owned()
    }
}

fn is_windows_reserved_name(stem: &str) -> bool {
    let upper = stem.to_ascii_uppercase();
    matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
