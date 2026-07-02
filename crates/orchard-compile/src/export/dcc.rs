use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::{
    ASSET_MANIFEST_FILE, AssetManifest, ExportError, PartManifest, RECIPE_FILE,
    validate_package_relative_path, write_json, write_text,
};

/// Schema version for the DCC output adapter sidecars.
pub const DCC_ADAPTER_SCHEMA_VERSION: u32 = 1;
/// DCC adapter manifest sidecar.
pub const DCC_ADAPTER_MANIFEST_FILE: &str = "dcc-adapter.json";
/// DCC adapter rebuild helper.
pub const DCC_REBUILD_SCRIPT_FILE: &str = "dcc_rebuild.py";
/// DCC adapter verification sidecar.
pub const DCC_VERIFICATION_FILE: &str = "dcc-verification.json";

/// Options for DCC adapter sidecar generation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccAdapterOptions {
    /// Variant controls exposed to downstream DCC tools as metadata only.
    pub variant_controls: Vec<DccVariantControl>,
}

/// Variant-control metadata projected to DCC tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccVariantControl {
    /// Stable control ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Display value.
    pub value: String,
    /// Whether this control was locked in the Shape Lab source.
    pub locked: bool,
}

/// Files owned by the DCC adapter projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccAdapterFiles {
    /// Canonical package manifest used as source of truth.
    pub asset_manifest: String,
    /// Serialized Shape Lab source recipe.
    pub recipe: String,
    /// DCC rebuild helper.
    pub rebuild_script: String,
    /// Verification sidecar.
    pub verification: String,
}

impl Default for DccAdapterFiles {
    fn default() -> Self {
        Self {
            asset_manifest: ASSET_MANIFEST_FILE.to_owned(),
            recipe: RECIPE_FILE.to_owned(),
            rebuild_script: DCC_REBUILD_SCRIPT_FILE.to_owned(),
            verification: DCC_VERIFICATION_FILE.to_owned(),
        }
    }
}

/// Source-of-truth boundary for DCC projections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccSourceOfTruth {
    /// Canonical source package.
    pub canonical_package: String,
    /// Whether an exported DCC scene is authoritative.
    pub dcc_scene_is_source_of_truth: bool,
    /// Whether importing a modified DCC scene is supported.
    pub external_scene_import_supported: bool,
}

impl Default for DccSourceOfTruth {
    fn default() -> Self {
        Self {
            canonical_package: ASSET_MANIFEST_FILE.to_owned(),
            dcc_scene_is_source_of_truth: false,
            external_scene_import_supported: false,
        }
    }
}

/// One semantic part projected to a DCC adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccSemanticPart {
    /// Stable package part ID.
    pub part_id: String,
    /// DCC object name.
    pub object_name: String,
    /// Source instance ID.
    pub instance_id: u64,
    /// Source definition ID.
    pub definition_id: u64,
    /// Source parent instance, when present.
    pub parent_instance_id: Option<u64>,
    /// Topology signature from the canonical package.
    pub topology_signature: u64,
    /// Semantic region labels.
    pub regions: Vec<String>,
    /// Custom metadata fields to attach to the DCC object.
    pub metadata: Vec<DccMetadataField>,
}

/// One DCC collection/group projection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccCollection {
    /// Stable collection ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Part IDs in deterministic order.
    pub part_ids: Vec<String>,
}

/// One metadata field projected to DCC objects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccMetadataField {
    /// Field key.
    pub key: String,
    /// Field value encoded as stable text.
    pub value: String,
}

/// DCC adapter manifest. This is output metadata, not a source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccAdapterManifest {
    /// DCC adapter schema version.
    pub schema_version: u32,
    /// Source-of-truth boundary.
    pub source_of_truth: DccSourceOfTruth,
    /// Source recipe hash from the canonical package.
    pub source_recipe_hash: u64,
    /// Adapter-owned files.
    pub files: DccAdapterFiles,
    /// DCC collections/groups.
    pub collections: Vec<DccCollection>,
    /// Semantic part projections.
    pub semantic_parts: Vec<DccSemanticPart>,
    /// Variant controls projected as metadata only.
    pub variant_controls: Vec<DccVariantControl>,
}

/// DCC adapter verification sidecar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DccAdapterVerificationReport {
    /// DCC adapter schema version.
    pub schema_version: u32,
    /// Source recipe hash from the canonical package.
    pub source_recipe_hash: u64,
    /// Whether the canonical package is the verified source.
    pub canonical_package_verified: bool,
    /// Whether an exported DCC scene is authoritative.
    pub dcc_scene_is_source_of_truth: bool,
    /// Whether importing a modified DCC scene is supported.
    pub external_scene_import_supported: bool,
    /// Number of semantic part rows.
    pub semantic_part_count: u64,
    /// Number of collection rows.
    pub collection_count: u64,
    /// Number of variant controls.
    pub variant_control_count: u64,
    /// Stable fingerprint of variant-control metadata.
    pub variant_control_fingerprint: u64,
    /// Deterministic issue codes.
    pub issues: Vec<String>,
}

/// Build a deterministic DCC adapter manifest from the canonical package manifest.
#[must_use]
pub fn dcc_adapter_manifest(
    manifest: &AssetManifest,
    options: &DccAdapterOptions,
) -> DccAdapterManifest {
    let semantic_parts = manifest
        .parts
        .iter()
        .map(dcc_semantic_part)
        .collect::<Vec<_>>();
    DccAdapterManifest {
        schema_version: DCC_ADAPTER_SCHEMA_VERSION,
        source_of_truth: DccSourceOfTruth::default(),
        source_recipe_hash: manifest.source_recipe_hash,
        files: DccAdapterFiles::default(),
        collections: dcc_collections(&manifest.parts),
        semantic_parts,
        variant_controls: options.variant_controls.clone(),
    }
}

/// Build the DCC adapter verification report.
#[must_use]
pub fn dcc_adapter_verification_report(
    adapter: &DccAdapterManifest,
) -> DccAdapterVerificationReport {
    DccAdapterVerificationReport {
        schema_version: DCC_ADAPTER_SCHEMA_VERSION,
        source_recipe_hash: adapter.source_recipe_hash,
        canonical_package_verified: true,
        dcc_scene_is_source_of_truth: false,
        external_scene_import_supported: false,
        semantic_part_count: adapter.semantic_parts.len() as u64,
        collection_count: adapter.collections.len() as u64,
        variant_control_count: adapter.variant_controls.len() as u64,
        variant_control_fingerprint: variant_control_fingerprint(&adapter.variant_controls),
        issues: Vec::new(),
    }
}

/// Write DCC adapter sidecars into a canonical model package directory.
pub fn write_dcc_adapter_files(
    out_dir: &Path,
    manifest: &AssetManifest,
    options: &DccAdapterOptions,
) -> Result<(DccAdapterManifest, DccAdapterVerificationReport), ExportError> {
    let adapter = dcc_adapter_manifest(manifest, options);
    let verification = dcc_adapter_verification_report(&adapter);
    write_text(&out_dir.join(DCC_REBUILD_SCRIPT_FILE), dcc_rebuild_script())?;
    write_json(&out_dir.join(DCC_ADAPTER_MANIFEST_FILE), &adapter)?;
    write_json(&out_dir.join(DCC_VERIFICATION_FILE), &verification)?;
    Ok((adapter, verification))
}

/// Validate DCC adapter sidecars against the canonical package manifest.
pub fn validate_dcc_adapter(
    package_manifest: &AssetManifest,
    adapter: &DccAdapterManifest,
    verification: &DccAdapterVerificationReport,
    context: &Path,
) -> Result<(), ExportError> {
    validate_dcc_manifest(package_manifest, adapter, context)?;
    validate_dcc_verification(package_manifest, adapter, verification, context)
}

/// Return the deterministic DCC rebuild helper written into packages.
#[must_use]
pub fn dcc_rebuild_script() -> &'static str {
    r####"# Generated by Shape Lab DCC output adapter.
import argparse
import json
from pathlib import Path
import sys


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-dir", default=str(Path(__file__).resolve().parent))
    parser.add_argument("--out-dir", default=None)
    args = sys.argv[1:]
    if "--" in args:
        args = args[args.index("--") + 1 :]
    return parser.parse_args(args)


def package_path(root, relative):
    path = Path(relative)
    if path.is_absolute() or ".." in path.parts or not path.parts:
        raise ValueError(f"unsafe package-relative path: {relative}")
    resolved_root = root.resolve()
    resolved = (root / path).resolve()
    if resolved_root not in (resolved, *resolved.parents):
        raise ValueError(f"package path escapes root: {relative}")
    return resolved


def main():
    args = parse_args()
    package_root = Path(args.package_dir)
    out_dir = Path(args.out_dir) if args.out_dir is not None else package_root
    adapter = json.loads(package_path(package_root, "dcc-adapter.json").read_text(encoding="utf-8"))
    manifest = json.loads(package_path(package_root, "asset-manifest.json").read_text(encoding="utf-8"))
    verification = json.loads(package_path(package_root, "dcc-verification.json").read_text(encoding="utf-8"))
    expected_files = {
        "asset_manifest": "asset-manifest.json",
        "recipe": "recipe.json",
        "rebuild_script": "dcc_rebuild.py",
        "verification": "dcc-verification.json",
    }
    if adapter["files"] != expected_files:
        raise RuntimeError("DCC adapter files must use canonical package paths")
    if adapter["source_of_truth"].get("canonical_package") != "asset-manifest.json":
        raise RuntimeError("DCC adapter canonical package mismatch")
    if adapter["source_of_truth"]["dcc_scene_is_source_of_truth"]:
        raise RuntimeError("DCC scene cannot be source of truth")
    if adapter["source_of_truth"]["external_scene_import_supported"]:
        raise RuntimeError("DCC scene import is not supported")
    if adapter["source_recipe_hash"] != manifest["source_recipe_hash"]:
        raise RuntimeError("DCC adapter hash does not match canonical manifest")
    expected_parts = semantic_parts(manifest["parts"])
    expected_collections = collections(manifest["parts"])
    if adapter["semantic_parts"] != expected_parts:
        raise RuntimeError("DCC semantic part projection does not match canonical manifest")
    if adapter["collections"] != expected_collections:
        raise RuntimeError("DCC collection projection does not match canonical manifest")
    if verification["semantic_part_count"] != len(expected_parts):
        raise RuntimeError("DCC verification part count mismatch")
    if verification["collection_count"] != len(expected_collections):
        raise RuntimeError("DCC verification collection count mismatch")
    if verification["variant_control_count"] != len(adapter.get("variant_controls", [])):
        raise RuntimeError("DCC verification variant count mismatch")
    out_dir.mkdir(parents=True, exist_ok=True)
    report = {
        "schema_version": adapter["schema_version"],
        "source_recipe_hash": adapter["source_recipe_hash"],
        "semantic_parts": len(adapter["semantic_parts"]),
        "collections": len(adapter["collections"]),
        "variant_controls": len(adapter.get("variant_controls", [])),
        "dcc_scene_is_source_of_truth": False,
        "external_scene_import_supported": False,
    }
    report_path = out_dir / "dcc_projection_report.json"
    report_path.write_text(json.dumps(report, sort_keys=True, indent=2), encoding="utf-8")
    print(json.dumps(report, sort_keys=True))


def metadata_field(key, value):
    return {"key": key, "value": str(value)}


def semantic_part(part):
    metadata = [
        metadata_field("shape_lab_part_id", part["part_id"]),
        metadata_field("shape_lab_instance_id", part["instance_id"]),
        metadata_field("shape_lab_definition_id", part["definition_id"]),
        metadata_field("shape_lab_topology_signature", part["topology_signature"]),
        metadata_field("shape_lab_source_recipe_instance", str(part["source_recipe_instance"]).lower()),
    ]
    if part.get("parent_instance_id") is not None:
        metadata.append(metadata_field("shape_lab_parent_instance_id", part["parent_instance_id"]))
    if part.get("prototype_instance_id") is not None:
        metadata.append(metadata_field("shape_lab_prototype_instance_id", part["prototype_instance_id"]))
    if part.get("generated_by") is not None:
        metadata.append(metadata_field("shape_lab_generated_by", part["generated_by"]))
    return {
        "part_id": part["part_id"],
        "object_name": part["object_name"],
        "instance_id": part["instance_id"],
        "definition_id": part["definition_id"],
        "parent_instance_id": part.get("parent_instance_id"),
        "topology_signature": part["topology_signature"],
        "regions": [f"{region['id']}:{region['name']}" for region in part.get("regions", [])],
        "metadata": metadata,
    }


def semantic_parts(parts):
    return [semantic_part(part) for part in parts]


def collections(parts):
    rows = [
        {
            "id": "shape_lab_asset",
            "label": "Shape Lab Asset",
            "part_ids": [part["part_id"] for part in parts],
        }
    ]
    by_definition = {}
    source_parts = []
    generated_parts = []
    for part in parts:
        by_definition.setdefault(part["definition_id"], []).append(part["part_id"])
        if part["source_recipe_instance"]:
            source_parts.append(part["part_id"])
        else:
            generated_parts.append(part["part_id"])
    for definition in sorted(by_definition):
        rows.append(
            {
                "id": f"definition_{definition}",
                "label": f"Definition {definition}",
                "part_ids": by_definition[definition],
            }
        )
    if source_parts:
        rows.append(
            {
                "id": "source_recipe_instances",
                "label": "Source Recipe Instances",
                "part_ids": source_parts,
            }
        )
    if generated_parts:
        rows.append(
            {
                "id": "generated_instances",
                "label": "Generated Instances",
                "part_ids": generated_parts,
            }
        )
    return rows


if __name__ == "__main__":
    main()
"####
}

fn dcc_semantic_part(part: &PartManifest) -> DccSemanticPart {
    let mut metadata = vec![
        metadata_field("shape_lab_part_id", &part.part_id),
        metadata_field("shape_lab_instance_id", part.instance_id.to_string()),
        metadata_field("shape_lab_definition_id", part.definition_id.to_string()),
        metadata_field(
            "shape_lab_topology_signature",
            part.topology_signature.to_string(),
        ),
        metadata_field(
            "shape_lab_source_recipe_instance",
            part.source_recipe_instance.to_string(),
        ),
    ];
    if let Some(parent) = part.parent_instance_id {
        metadata.push(metadata_field(
            "shape_lab_parent_instance_id",
            parent.to_string(),
        ));
    }
    if let Some(prototype) = part.prototype_instance_id {
        metadata.push(metadata_field(
            "shape_lab_prototype_instance_id",
            prototype.to_string(),
        ));
    }
    if let Some(operation) = part.generated_by {
        metadata.push(metadata_field(
            "shape_lab_generated_by",
            operation.to_string(),
        ));
    }
    DccSemanticPart {
        part_id: part.part_id.clone(),
        object_name: part.object_name.clone(),
        instance_id: part.instance_id,
        definition_id: part.definition_id,
        parent_instance_id: part.parent_instance_id,
        topology_signature: part.topology_signature,
        regions: part
            .regions
            .iter()
            .map(|region| format!("{}:{}", region.id, region.name))
            .collect(),
        metadata,
    }
}

fn dcc_collections(parts: &[PartManifest]) -> Vec<DccCollection> {
    let mut collections = vec![DccCollection {
        id: "shape_lab_asset".to_owned(),
        label: "Shape Lab Asset".to_owned(),
        part_ids: parts.iter().map(|part| part.part_id.clone()).collect(),
    }];

    let mut by_definition = BTreeMap::<u64, Vec<String>>::new();
    let mut source_parts = Vec::new();
    let mut generated_parts = Vec::new();
    for part in parts {
        by_definition
            .entry(part.definition_id)
            .or_default()
            .push(part.part_id.clone());
        if part.source_recipe_instance {
            source_parts.push(part.part_id.clone());
        } else {
            generated_parts.push(part.part_id.clone());
        }
    }
    for (definition, part_ids) in by_definition {
        collections.push(DccCollection {
            id: format!("definition_{definition}"),
            label: format!("Definition {definition}"),
            part_ids,
        });
    }
    if !source_parts.is_empty() {
        collections.push(DccCollection {
            id: "source_recipe_instances".to_owned(),
            label: "Source Recipe Instances".to_owned(),
            part_ids: source_parts,
        });
    }
    if !generated_parts.is_empty() {
        collections.push(DccCollection {
            id: "generated_instances".to_owned(),
            label: "Generated Instances".to_owned(),
            part_ids: generated_parts,
        });
    }
    collections
}

fn validate_dcc_manifest(
    package_manifest: &AssetManifest,
    adapter: &DccAdapterManifest,
    context: &Path,
) -> Result<(), ExportError> {
    if adapter.schema_version != DCC_ADAPTER_SCHEMA_VERSION {
        return Err(super::invalid_package(
            context,
            format!(
                "unsupported DCC adapter schema {}; expected {DCC_ADAPTER_SCHEMA_VERSION}",
                adapter.schema_version
            ),
        ));
    }
    if adapter.source_recipe_hash != package_manifest.source_recipe_hash {
        return Err(super::invalid_package(
            context,
            "DCC adapter source hash does not match asset-manifest.json",
        ));
    }
    if adapter.source_of_truth != DccSourceOfTruth::default() {
        return Err(super::invalid_package(
            context,
            "DCC adapter must keep Shape Lab package as source of truth",
        ));
    }
    if adapter.files != DccAdapterFiles::default() {
        return Err(super::invalid_package(
            context,
            "DCC adapter files must use canonical package sidecar paths",
        ));
    }
    for path in [
        &adapter.files.asset_manifest,
        &adapter.files.recipe,
        &adapter.files.rebuild_script,
        &adapter.files.verification,
    ] {
        validate_package_relative_path(path, context)?;
    }
    let expected = dcc_adapter_manifest(
        package_manifest,
        &DccAdapterOptions {
            variant_controls: adapter.variant_controls.clone(),
        },
    );
    if adapter.source_recipe_hash != expected.source_recipe_hash
        || adapter.source_of_truth != expected.source_of_truth
        || adapter.files != expected.files
        || adapter.collections != expected.collections
        || adapter.semantic_parts != expected.semantic_parts
    {
        return Err(super::invalid_package(
            context,
            "DCC adapter projection does not match asset-manifest.json",
        ));
    }
    let package_part_ids = package_manifest
        .parts
        .iter()
        .map(|part| part.part_id.as_str())
        .collect::<BTreeSet<_>>();
    for collection in &adapter.collections {
        for part_id in &collection.part_ids {
            if !package_part_ids.contains(part_id.as_str()) {
                return Err(super::invalid_package(
                    context,
                    format!("DCC collection references unknown part `{part_id}`"),
                ));
            }
        }
    }
    Ok(())
}

fn validate_dcc_verification(
    package_manifest: &AssetManifest,
    adapter: &DccAdapterManifest,
    verification: &DccAdapterVerificationReport,
    context: &Path,
) -> Result<(), ExportError> {
    if verification.schema_version != DCC_ADAPTER_SCHEMA_VERSION
        || verification.source_recipe_hash != package_manifest.source_recipe_hash
        || verification.semantic_part_count != adapter.semantic_parts.len() as u64
        || verification.collection_count != adapter.collections.len() as u64
        || verification.variant_control_count != adapter.variant_controls.len() as u64
        || verification.variant_control_fingerprint
            != variant_control_fingerprint(&adapter.variant_controls)
        || !verification.canonical_package_verified
        || verification.dcc_scene_is_source_of_truth
        || verification.external_scene_import_supported
        || !verification.issues.is_empty()
    {
        return Err(super::invalid_package(
            context,
            "DCC verification report does not match adapter manifest",
        ));
    }
    Ok(())
}

fn variant_control_fingerprint(variant_controls: &[DccVariantControl]) -> u64 {
    serde_json::to_vec(variant_controls).map_or(0, |bytes| super::fnv64(&bytes))
}

fn metadata_field(key: impl Into<String>, value: impl Into<String>) -> DccMetadataField {
    DccMetadataField {
        key: key.into(),
        value: value.into(),
    }
}
