#![forbid(unsafe_code)]

//! Minimal static-prop glTF/GLB handoff for runtime-neutral packages.
//!
//! This is intentionally narrow: it exports one static mesh into a portable,
//! geometry-only GLB with deterministic JSON, one node, one mesh primitive, and
//! material slot IDs recorded as metadata. It does not claim UVs, textures,
//! slot-assigned primitives, skinning, animation, or engine-native import.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use shape_mesh::TriangleMesh;

use crate::surface::{SurfaceArtifact, SurfaceReviewStatus};

/// GLB handoff schema version for Shape Lab static props.
pub const STATIC_PROP_GLB_HANDOFF_SCHEMA_VERSION: u32 = 1;

/// Metadata embedded in a generated static-prop GLB.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticPropGlbMetadata {
    /// Source profile slug.
    pub profile_id: String,
    /// Human-facing asset name.
    pub display_name: String,
    /// Stable material slot IDs recorded as metadata only.
    pub material_slots: Vec<String>,
}

/// Lightweight GLB validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StaticPropGlbValidationReport {
    /// Report schema version.
    pub schema_version: u32,
    /// Whether the bytes look like a GLB 2.0 payload.
    pub valid: bool,
    /// Stable issue codes.
    pub issue_codes: Vec<String>,
    /// Parsed GLB version, when readable.
    pub version: Option<u32>,
    /// Payload length declared by the GLB header.
    pub declared_length: Option<u32>,
}

/// GLB export failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticPropGlbError {
    /// Mesh data is not exportable.
    InvalidMesh(String),
    /// Metadata is not exportable.
    InvalidMetadata(String),
    /// JSON encoding failed.
    Json(String),
    /// File I/O failed.
    Io(String),
}

impl fmt::Display for StaticPropGlbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMesh(message) => write!(formatter, "invalid mesh: {message}"),
            Self::InvalidMetadata(message) => write!(formatter, "invalid metadata: {message}"),
            Self::Json(message) => write!(formatter, "json error: {message}"),
            Self::Io(message) => write!(formatter, "io error: {message}"),
        }
    }
}

impl std::error::Error for StaticPropGlbError {}

impl From<std::io::Error> for StaticPropGlbError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

/// Write a deterministic GLB file for one static prop mesh.
pub fn write_static_prop_glb(
    mesh: &TriangleMesh,
    metadata: &StaticPropGlbMetadata,
    path: impl AsRef<Path>,
) -> Result<StaticPropGlbValidationReport, StaticPropGlbError> {
    let bytes = encode_static_prop_glb(mesh, metadata)?;
    fs::write(path, &bytes)?;
    Ok(validate_static_prop_glb(&bytes))
}

/// Write a deterministic surface-aware GLB file for one static prop mesh.
///
/// This handoff includes POSITION, NORMAL, and TEXCOORD_0. Texture files remain
/// package sidecars referenced by the SurfaceArtifact; this function does not
/// claim engine-native material import.
pub fn write_static_prop_surface_glb(
    mesh: &TriangleMesh,
    surface: &SurfaceArtifact,
    surface_artifact_ref: &str,
    path: impl AsRef<Path>,
) -> Result<StaticPropGlbValidationReport, StaticPropGlbError> {
    let bytes = encode_static_prop_surface_glb(mesh, surface, surface_artifact_ref)?;
    fs::write(path, &bytes)?;
    Ok(validate_static_prop_surface_glb(&bytes))
}

/// Encode one static prop mesh as a deterministic GLB 2.0 payload.
pub fn encode_static_prop_glb(
    mesh: &TriangleMesh,
    metadata: &StaticPropGlbMetadata,
) -> Result<Vec<u8>, StaticPropGlbError> {
    validate_mesh_for_glb(mesh)?;
    validate_metadata(metadata)?;

    let positions_offset = 0_u32;
    let positions_length = checked_byte_len(mesh.positions.len(), 12, "positions")?;
    let normals_offset = align4(positions_offset + positions_length);
    let normals_length = checked_byte_len(mesh.normals.len(), 12, "normals")?;
    let indices_offset = align4(normals_offset + normals_length);
    let indices_length = checked_byte_len(mesh.indices.len(), 4, "indices")?;
    let bin_length = align4(indices_offset + indices_length);

    let mut bin = vec![0_u8; bin_length as usize];
    write_f32_triplets(&mut bin, positions_offset as usize, &mesh.positions);
    write_f32_triplets(&mut bin, normals_offset as usize, &mesh.normals);
    write_u32_values(&mut bin, indices_offset as usize, &mesh.indices);

    let min = mesh.bounds.min.to_array();
    let max = mesh.bounds.max.to_array();
    let gltf = json!({
        "asset": {
            "version": "2.0",
            "generator": "Shape Lab static-prop GLB handoff v0"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{
            "name": metadata.display_name,
            "mesh": 0,
            "extras": {
                "shapeLab": {
                    "schemaVersion": STATIC_PROP_GLB_HANDOFF_SCHEMA_VERSION,
                    "profileId": metadata.profile_id,
                    "claim": "geometry-only-portable-static-mesh-handoff",
                    "materialSlotBinding": "metadata-only",
                    "materialSlotIds": metadata.material_slots,
                    "notIncluded": [
                        "uv-layout",
                        "textures",
                        "slot-assigned-primitives",
                        "skin",
                        "animation",
                        "engine-native-import"
                    ]
                }
            }
        }],
        "meshes": [{
            "name": format!("{} Mesh", metadata.display_name),
            "primitives": [{
                "attributes": {
                    "POSITION": 0,
                    "NORMAL": 1
                },
                "indices": 2,
                "material": 0,
                "mode": 4
            }]
        }],
        "materials": [{
            "name": "Geometry Placeholder Material",
            "extras": {
                "shapeLabMaterialBinding": "metadata-only"
            },
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.72, 0.74, 0.69, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.72
            }
        }],
        "buffers": [{
            "byteLength": bin_length
        }],
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
                "componentType": 5126,
                "count": mesh.positions.len(),
                "type": "VEC3",
                "min": min,
                "max": max
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": mesh.normals.len(),
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ],
        "extras": {
            "shapeLabProfileId": metadata.profile_id,
            "shapeLabDisplayName": metadata.display_name,
            "shapeLabMaterialSlotIds": metadata.material_slots,
            "shapeLabMaterialBinding": "metadata-only"
        }
    });

    encode_glb_json_and_bin(gltf, bin_length, bin)
}

/// Encode one static prop mesh and its primary UV set as GLB 2.0.
pub fn encode_static_prop_surface_glb(
    mesh: &TriangleMesh,
    surface: &SurfaceArtifact,
    surface_artifact_ref: &str,
) -> Result<Vec<u8>, StaticPropGlbError> {
    validate_mesh_for_glb(mesh)?;
    validate_surface_for_glb(mesh, surface, surface_artifact_ref)?;

    let uv_set = surface
        .uv_sets
        .iter()
        .find(|set| set.channel_index == 0)
        .or_else(|| surface.uv_sets.first())
        .ok_or_else(|| {
            StaticPropGlbError::InvalidMetadata("surface UV set is required".to_owned())
        })?;
    let positions_offset = 0_u32;
    let positions_length = checked_byte_len(mesh.positions.len(), 12, "positions")?;
    let normals_offset = align4(positions_offset + positions_length);
    let normals_length = checked_byte_len(mesh.normals.len(), 12, "normals")?;
    let texcoords_offset = align4(normals_offset + normals_length);
    let texcoords_length = checked_byte_len(uv_set.coordinates.len(), 8, "texcoords")?;
    let indices_offset = align4(texcoords_offset + texcoords_length);
    let indices_length = checked_byte_len(mesh.indices.len(), 4, "indices")?;
    let bin_length = align4(indices_offset + indices_length);

    let mut bin = vec![0_u8; bin_length as usize];
    write_f32_triplets(&mut bin, positions_offset as usize, &mesh.positions);
    write_f32_triplets(&mut bin, normals_offset as usize, &mesh.normals);
    write_f32_pairs(&mut bin, texcoords_offset as usize, &uv_set.coordinates);
    write_u32_values(&mut bin, indices_offset as usize, &mesh.indices);

    let min = mesh.bounds.min.to_array();
    let max = mesh.bounds.max.to_array();
    let material_slot_ids = surface
        .material_slots
        .iter()
        .map(|slot| slot.slot_id.as_str())
        .collect::<Vec<_>>();
    let gltf = json!({
        "asset": {
            "version": "2.0",
            "generator": "Shape Lab static-prop surface GLB handoff v1"
        },
        "scene": 0,
        "scenes": [{ "nodes": [0] }],
        "nodes": [{
            "name": surface.display_name,
            "mesh": 0,
            "extras": {
                "shapeLab": {
                    "schemaVersion": STATIC_PROP_GLB_HANDOFF_SCHEMA_VERSION,
                    "profileId": surface.profile_id,
                    "claim": "portable-static-mesh-handoff-with-texcoord0",
                    "surfaceArtifactRef": surface_artifact_ref,
                    "surfaceReviewStatus": match surface.manual_review {
                        SurfaceReviewStatus::NotReviewed => "not_reviewed",
                        SurfaceReviewStatus::AutomatedReady => "automated_ready",
                        SurfaceReviewStatus::ManualRequired => "manual_required",
                        SurfaceReviewStatus::Approved => "approved",
                        SurfaceReviewStatus::Rejected => "rejected",
                    },
                    "materialSlotBinding": "triangle-bindings-in-surface-artifact",
                    "materialSlotIds": material_slot_ids,
                    "textureBinding": "package-sidecars-not-embedded",
                    "notIncluded": [
                        "engine-native-import",
                        "skin",
                        "animation",
                        "human-approved-material-art-quality"
                    ]
                }
            }
        }],
        "meshes": [{
            "name": format!("{} Surface Mesh", surface.display_name),
            "primitives": [{
                "attributes": {
                    "POSITION": 0,
                    "NORMAL": 1,
                    "TEXCOORD_0": 2
                },
                "indices": 3,
                "material": 0,
                "mode": 4
            }]
        }],
        "materials": [surface.material_slots.first().map(|slot| {
            json!({
                "name": slot.display_name,
                "extras": {
                    "shapeLabMaterialSlotId": slot.slot_id,
                    "shapeLabMaterialBinding": "surface-artifact-triangle-bindings"
                },
                "pbrMetallicRoughness": {
                    "baseColorFactor": [0.72, 0.74, 0.69, 1.0],
                    "metallicFactor": 0.0,
                    "roughnessFactor": 0.72
                }
            })
        }).unwrap_or_else(|| json!({
            "name": "Surface Placeholder Material",
            "pbrMetallicRoughness": {
                "baseColorFactor": [0.72, 0.74, 0.69, 1.0],
                "metallicFactor": 0.0,
                "roughnessFactor": 0.72
            }
        }))],
        "buffers": [{
            "byteLength": bin_length
        }],
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
                "byteOffset": texcoords_offset,
                "byteLength": texcoords_length,
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
                "componentType": 5126,
                "count": mesh.positions.len(),
                "type": "VEC3",
                "min": min,
                "max": max
            },
            {
                "bufferView": 1,
                "componentType": 5126,
                "count": mesh.normals.len(),
                "type": "VEC3"
            },
            {
                "bufferView": 2,
                "componentType": 5126,
                "count": uv_set.coordinates.len(),
                "type": "VEC2"
            },
            {
                "bufferView": 3,
                "componentType": 5125,
                "count": mesh.indices.len(),
                "type": "SCALAR"
            }
        ],
        "extras": {
            "shapeLabProfileId": surface.profile_id,
            "shapeLabDisplayName": surface.display_name,
            "shapeLabMaterialSlotIds": material_slot_ids,
            "shapeLabSurfaceArtifact": surface_artifact_ref,
            "shapeLabTextureBinding": "sidecar-texture-files",
            "shapeLabNoSkin": true,
            "shapeLabNoAnimation": true
        }
    });

    encode_glb_json_and_bin(gltf, bin_length, bin)
}

fn encode_glb_json_and_bin(
    gltf: Value,
    bin_length: u32,
    bin: Vec<u8>,
) -> Result<Vec<u8>, StaticPropGlbError> {
    let mut json_bytes =
        serde_json::to_vec(&gltf).map_err(|error| StaticPropGlbError::Json(error.to_string()))?;
    pad_json(&mut json_bytes);
    let json_length = u32::try_from(json_bytes.len()).map_err(|_| {
        StaticPropGlbError::Json("glTF JSON chunk exceeds u32 byte length".to_owned())
    })?;

    let total_length = 12_u32
        .checked_add(8)
        .and_then(|value| value.checked_add(json_length))
        .and_then(|value| value.checked_add(8))
        .and_then(|value| value.checked_add(bin_length))
        .ok_or_else(|| StaticPropGlbError::InvalidMesh("GLB payload is too large".to_owned()))?;

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
    Ok(glb)
}

/// Validate the GLB header, chunk table, JSON payload, BIN payload, and the
/// minimal static-prop primitive contract without performing full scene import.
#[must_use]
pub fn validate_static_prop_glb(bytes: &[u8]) -> StaticPropGlbValidationReport {
    let mut issue_codes = Vec::new();
    if bytes.len() < 20 {
        issue_codes.push("glb_too_short".to_owned());
        return StaticPropGlbValidationReport {
            schema_version: STATIC_PROP_GLB_HANDOFF_SCHEMA_VERSION,
            valid: false,
            issue_codes,
            version: None,
            declared_length: None,
        };
    }
    if &bytes[0..4] != b"glTF" {
        issue_codes.push("glb_magic_invalid".to_owned());
    }
    let version = read_u32(bytes, 4);
    if version != Some(2) {
        issue_codes.push("glb_version_invalid".to_owned());
    }
    let declared_length = read_u32(bytes, 8);
    if declared_length.map(|length| length as usize) != Some(bytes.len()) {
        issue_codes.push("glb_length_mismatch".to_owned());
    }
    if read_u32(bytes, 12).is_none() || bytes.get(16..20) != Some(b"JSON".as_slice()) {
        issue_codes.push("glb_json_chunk_missing".to_owned());
    }
    let Some(json_length) = read_u32(bytes, 12).map(|value| value as usize) else {
        return finish_validation_report(issue_codes, version, declared_length);
    };
    let json_start = 20_usize;
    let Some(json_end) = json_start.checked_add(json_length) else {
        issue_codes.push("glb_json_chunk_overflow".to_owned());
        return finish_validation_report(issue_codes, version, declared_length);
    };
    if json_length == 0 || !json_length.is_multiple_of(4) {
        issue_codes.push("glb_json_chunk_length_invalid".to_owned());
    }
    if json_end > bytes.len() {
        issue_codes.push("glb_json_chunk_out_of_bounds".to_owned());
        return finish_validation_report(issue_codes, version, declared_length);
    }
    let json_value = match std::str::from_utf8(&bytes[json_start..json_end])
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(text.trim_end()).ok())
    {
        Some(value) => Some(value),
        None => {
            issue_codes.push("glb_json_invalid".to_owned());
            None
        }
    };

    let bin_header_start = json_end;
    if bin_header_start
        .checked_add(8)
        .is_none_or(|end| end > bytes.len())
    {
        issue_codes.push("glb_bin_chunk_missing".to_owned());
        return finish_validation_report(issue_codes, version, declared_length);
    }
    let bin_length = read_u32(bytes, bin_header_start).unwrap_or(0) as usize;
    if bytes.get(bin_header_start + 4..bin_header_start + 8) != Some(b"BIN\0".as_slice()) {
        issue_codes.push("glb_bin_chunk_missing".to_owned());
    }
    let bin_start = bin_header_start + 8;
    let Some(bin_end) = bin_start.checked_add(bin_length) else {
        issue_codes.push("glb_bin_chunk_overflow".to_owned());
        return finish_validation_report(issue_codes, version, declared_length);
    };
    if bin_length == 0 || !bin_length.is_multiple_of(4) || bin_end > bytes.len() {
        issue_codes.push("glb_bin_chunk_length_invalid".to_owned());
    }
    if bin_end != bytes.len() {
        issue_codes.push("glb_unexpected_extra_bytes".to_owned());
    }

    if let Some(value) = json_value.as_ref() {
        validate_static_prop_gltf_json(value, bin_length, &mut issue_codes);
    }

    finish_validation_report(issue_codes, version, declared_length)
}

/// Validate a surface-aware static-prop GLB.
#[must_use]
pub fn validate_static_prop_surface_glb(bytes: &[u8]) -> StaticPropGlbValidationReport {
    let base = validate_static_prop_glb(bytes);
    let mut issue_codes = base.issue_codes;
    if let Some(value) = parse_glb_json(bytes) {
        let Some(attributes) = value
            .pointer("/meshes/0/primitives/0/attributes")
            .and_then(Value::as_object)
        else {
            issue_codes.push("gltf_surface_attributes_missing".to_owned());
            return finish_validation_report(issue_codes, base.version, base.declared_length);
        };
        if !attributes.contains_key("TEXCOORD_0") {
            issue_codes.push("gltf_texcoord0_missing".to_owned());
        } else {
            validate_texcoord0_accessor(&value, bytes, attributes, &mut issue_codes);
        }
        let text = value.to_string();
        for forbidden in ["skins", "animations", "engine-native-import-ready"] {
            if text.contains(forbidden) {
                issue_codes.push(format!("gltf_forbidden_claim_{forbidden}"));
            }
        }
        if value
            .pointer("/extras/shapeLabTextureBinding")
            .and_then(Value::as_str)
            != Some("sidecar-texture-files")
        {
            issue_codes.push("gltf_surface_texture_binding_not_sidecar".to_owned());
        }
        if value
            .pointer("/extras/shapeLabMaterialSlotIds")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
        {
            issue_codes.push("gltf_surface_material_slot_ids_missing".to_owned());
        }
    } else {
        issue_codes.push("glb_json_invalid".to_owned());
    }
    finish_validation_report(issue_codes, base.version, base.declared_length)
}

/// Validate a surface-aware static-prop GLB and require its metadata to point at
/// the expected SurfaceArtifact sidecar.
#[must_use]
pub fn validate_static_prop_surface_glb_with_artifact_ref(
    bytes: &[u8],
    expected_surface_artifact_ref: &str,
) -> StaticPropGlbValidationReport {
    let base = validate_static_prop_surface_glb(bytes);
    let mut issue_codes = base.issue_codes;
    if expected_surface_artifact_ref.trim().is_empty() {
        issue_codes.push("gltf_surface_artifact_ref_expected_missing".to_owned());
        return finish_validation_report(issue_codes, base.version, base.declared_length);
    }
    if let Some(value) = parse_glb_json(bytes) {
        if value
            .pointer("/nodes/0/extras/shapeLab/surfaceArtifactRef")
            .and_then(Value::as_str)
            != Some(expected_surface_artifact_ref)
        {
            issue_codes.push("gltf_surface_artifact_ref_mismatch".to_owned());
        }
        if value
            .pointer("/extras/shapeLabSurfaceArtifact")
            .and_then(Value::as_str)
            != Some(expected_surface_artifact_ref)
        {
            issue_codes.push("gltf_surface_artifact_extra_ref_mismatch".to_owned());
        }
    }
    finish_validation_report(issue_codes, base.version, base.declared_length)
}

fn validate_texcoord0_accessor(
    value: &Value,
    bytes: &[u8],
    attributes: &serde_json::Map<String, Value>,
    issue_codes: &mut Vec<String>,
) {
    let Some(accessors) = value.get("accessors").and_then(Value::as_array) else {
        return;
    };
    let Some(buffer_views) = value.get("bufferViews").and_then(Value::as_array) else {
        return;
    };
    let Some(texcoord_index) = attributes.get("TEXCOORD_0").and_then(Value::as_u64) else {
        issue_codes.push("gltf_texcoord0_accessor_invalid".to_owned());
        return;
    };
    let Some(texcoord_accessor) = accessors.get(texcoord_index as usize) else {
        issue_codes.push("gltf_texcoord0_accessor_out_of_range".to_owned());
        return;
    };
    if texcoord_accessor
        .get("componentType")
        .and_then(Value::as_u64)
        != Some(5126)
    {
        issue_codes.push("gltf_texcoord0_component_type_invalid".to_owned());
    }
    if texcoord_accessor.get("type").and_then(Value::as_str) != Some("VEC2") {
        issue_codes.push("gltf_texcoord0_type_invalid".to_owned());
    }
    let texcoord_count = texcoord_accessor
        .get("count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if texcoord_count == 0 {
        issue_codes.push("gltf_texcoord0_count_invalid".to_owned());
    }
    let position_count = attributes
        .get("POSITION")
        .and_then(Value::as_u64)
        .and_then(|index| accessors.get(index as usize))
        .and_then(|accessor| accessor.get("count"))
        .and_then(Value::as_u64);
    if position_count.is_some_and(|count| count != texcoord_count) {
        issue_codes.push("gltf_texcoord0_count_mismatch".to_owned());
    }
    let Some(buffer_view_index) = texcoord_accessor.get("bufferView").and_then(Value::as_u64)
    else {
        issue_codes.push("gltf_texcoord0_buffer_view_missing".to_owned());
        return;
    };
    let Some(buffer_view) = buffer_views.get(buffer_view_index as usize) else {
        issue_codes.push("gltf_texcoord0_buffer_view_out_of_range".to_owned());
        return;
    };
    let view_offset = buffer_view
        .get("byteOffset")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let accessor_offset = texcoord_accessor
        .get("byteOffset")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let Some(view_length) = buffer_view.get("byteLength").and_then(Value::as_u64) else {
        issue_codes.push("gltf_texcoord0_buffer_view_length_missing".to_owned());
        return;
    };
    let Some(required_length) = (texcoord_count as usize).checked_mul(8) else {
        issue_codes.push("gltf_texcoord0_byte_length_invalid".to_owned());
        return;
    };
    if accessor_offset
        .checked_add(required_length)
        .is_none_or(|length| length > view_length as usize)
    {
        issue_codes.push("gltf_texcoord0_buffer_view_too_short".to_owned());
        return;
    }
    let Some(bin) = parse_glb_bin(bytes) else {
        issue_codes.push("gltf_texcoord0_bin_missing".to_owned());
        return;
    };
    let Some(start) = view_offset.checked_add(accessor_offset) else {
        issue_codes.push("gltf_texcoord0_byte_range_invalid".to_owned());
        return;
    };
    let Some(end) = start.checked_add(required_length) else {
        issue_codes.push("gltf_texcoord0_byte_range_invalid".to_owned());
        return;
    };
    let Some(slice) = bin.get(start..end) else {
        issue_codes.push("gltf_texcoord0_byte_range_out_of_bounds".to_owned());
        return;
    };
    for pair in slice.chunks_exact(8) {
        let u = f32::from_le_bytes(pair[0..4].try_into().expect("chunk size checked"));
        let v = f32::from_le_bytes(pair[4..8].try_into().expect("chunk size checked"));
        if !u.is_finite() || !v.is_finite() {
            issue_codes.push("gltf_texcoord0_non_finite".to_owned());
            return;
        }
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            issue_codes.push("gltf_texcoord0_out_of_range".to_owned());
            return;
        }
    }
}

fn finish_validation_report(
    issue_codes: Vec<String>,
    version: Option<u32>,
    declared_length: Option<u32>,
) -> StaticPropGlbValidationReport {
    let valid = issue_codes.is_empty();
    StaticPropGlbValidationReport {
        schema_version: STATIC_PROP_GLB_HANDOFF_SCHEMA_VERSION,
        valid,
        issue_codes,
        version,
        declared_length,
    }
}

fn validate_static_prop_gltf_json(value: &Value, bin_length: usize, issue_codes: &mut Vec<String>) {
    if value.pointer("/asset/version").and_then(Value::as_str) != Some("2.0") {
        issue_codes.push("gltf_asset_version_invalid".to_owned());
    }
    let Some(buffers) = value.get("buffers").and_then(Value::as_array) else {
        issue_codes.push("gltf_buffers_missing".to_owned());
        return;
    };
    if buffers.len() != 1
        || buffers[0]
            .get("byteLength")
            .and_then(Value::as_u64)
            .map(|length| length as usize)
            != Some(bin_length)
    {
        issue_codes.push("gltf_buffer_length_invalid".to_owned());
    }
    let Some(buffer_views) = value.get("bufferViews").and_then(Value::as_array) else {
        issue_codes.push("gltf_buffer_views_missing".to_owned());
        return;
    };
    let Some(accessors) = value.get("accessors").and_then(Value::as_array) else {
        issue_codes.push("gltf_accessors_missing".to_owned());
        return;
    };
    for (index, view) in buffer_views.iter().enumerate() {
        let offset = view.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let Some(length) = view.get("byteLength").and_then(Value::as_u64) else {
            issue_codes.push(format!("gltf_buffer_view_{index}_length_missing"));
            continue;
        };
        if offset
            .checked_add(length as usize)
            .is_none_or(|end| end > bin_length)
        {
            issue_codes.push(format!("gltf_buffer_view_{index}_out_of_bounds"));
        }
    }
    for (index, accessor) in accessors.iter().enumerate() {
        let Some(view_index) = accessor.get("bufferView").and_then(Value::as_u64) else {
            issue_codes.push(format!("gltf_accessor_{index}_buffer_view_missing"));
            continue;
        };
        if view_index as usize >= buffer_views.len() {
            issue_codes.push(format!("gltf_accessor_{index}_buffer_view_out_of_range"));
        }
        if accessor.get("count").and_then(Value::as_u64).unwrap_or(0) == 0 {
            issue_codes.push(format!("gltf_accessor_{index}_count_invalid"));
        }
    }
    let material_count = value
        .get("materials")
        .and_then(Value::as_array)
        .map_or(0, |materials| materials.len());
    let Some(primitive) = value
        .pointer("/meshes/0/primitives/0")
        .and_then(Value::as_object)
    else {
        issue_codes.push("gltf_static_prop_primitive_missing".to_owned());
        return;
    };
    let Some(position_accessor) = primitive
        .get("attributes")
        .and_then(|attributes| attributes.get("POSITION"))
        .and_then(Value::as_u64)
    else {
        issue_codes.push("gltf_position_attribute_missing".to_owned());
        return;
    };
    if position_accessor as usize >= accessors.len() {
        issue_codes.push("gltf_position_accessor_out_of_range".to_owned());
    }
    let Some(normal_accessor) = primitive
        .get("attributes")
        .and_then(|attributes| attributes.get("NORMAL"))
        .and_then(Value::as_u64)
    else {
        issue_codes.push("gltf_normal_attribute_missing".to_owned());
        return;
    };
    if normal_accessor as usize >= accessors.len() {
        issue_codes.push("gltf_normal_accessor_out_of_range".to_owned());
    }
    let Some(index_accessor) = primitive.get("indices").and_then(Value::as_u64) else {
        issue_codes.push("gltf_indices_accessor_missing".to_owned());
        return;
    };
    if index_accessor as usize >= accessors.len() {
        issue_codes.push("gltf_indices_accessor_out_of_range".to_owned());
    }
    if primitive
        .get("material")
        .and_then(Value::as_u64)
        .is_some_and(|index| index as usize >= material_count)
    {
        issue_codes.push("gltf_material_index_out_of_range".to_owned());
    }
}

fn parse_glb_json(bytes: &[u8]) -> Option<Value> {
    let json_length = read_u32(bytes, 12)? as usize;
    let json_start = 20_usize;
    let json_end = json_start.checked_add(json_length)?;
    let text = std::str::from_utf8(bytes.get(json_start..json_end)?).ok()?;
    serde_json::from_str(text.trim_end()).ok()
}

fn parse_glb_bin(bytes: &[u8]) -> Option<&[u8]> {
    let json_length = read_u32(bytes, 12)? as usize;
    let bin_header = 20_usize.checked_add(json_length)?;
    let bin_length = read_u32(bytes, bin_header)? as usize;
    let bin_start = bin_header.checked_add(8)?;
    let bin_end = bin_start.checked_add(bin_length)?;
    bytes.get(bin_start..bin_end)
}

fn validate_mesh_for_glb(mesh: &TriangleMesh) -> Result<(), StaticPropGlbError> {
    if mesh.positions.is_empty() {
        return Err(StaticPropGlbError::InvalidMesh(
            "GLB export requires at least one position".to_owned(),
        ));
    }
    if mesh.positions.len() != mesh.normals.len() {
        return Err(StaticPropGlbError::InvalidMesh(
            "position and normal counts must match".to_owned(),
        ));
    }
    if !mesh.indices.len().is_multiple_of(3) {
        return Err(StaticPropGlbError::InvalidMesh(
            "indices must form triangles".to_owned(),
        ));
    }
    if mesh
        .indices
        .iter()
        .any(|index| *index as usize >= mesh.positions.len())
    {
        return Err(StaticPropGlbError::InvalidMesh(
            "indices must reference existing vertices".to_owned(),
        ));
    }
    if mesh
        .positions
        .iter()
        .chain(mesh.normals.iter())
        .any(|values| !values.iter().all(|value| value.is_finite()))
    {
        return Err(StaticPropGlbError::InvalidMesh(
            "positions and normals must be finite".to_owned(),
        ));
    }
    if mesh.bounds.is_empty() || !mesh.bounds.min.is_finite() || !mesh.bounds.max.is_finite() {
        return Err(StaticPropGlbError::InvalidMesh(
            "bounds must be finite and non-empty".to_owned(),
        ));
    }
    Ok(())
}

fn validate_metadata(metadata: &StaticPropGlbMetadata) -> Result<(), StaticPropGlbError> {
    if metadata.profile_id.trim().is_empty() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "profile_id is required".to_owned(),
        ));
    }
    if metadata.display_name.trim().is_empty() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "display_name is required".to_owned(),
        ));
    }
    if metadata.material_slots.is_empty()
        || metadata
            .material_slots
            .iter()
            .any(|slot| slot.trim().is_empty())
    {
        return Err(StaticPropGlbError::InvalidMetadata(
            "at least one material slot name is required".to_owned(),
        ));
    }
    Ok(())
}

fn validate_surface_for_glb(
    mesh: &TriangleMesh,
    surface: &SurfaceArtifact,
    surface_artifact_ref: &str,
) -> Result<(), StaticPropGlbError> {
    if surface.profile_id.trim().is_empty() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface profile_id is required".to_owned(),
        ));
    }
    if surface.display_name.trim().is_empty() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface display_name is required".to_owned(),
        ));
    }
    if surface_artifact_ref.trim().is_empty() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface artifact reference is required".to_owned(),
        ));
    }
    let Some(uv_set) = surface
        .uv_sets
        .iter()
        .find(|set| set.channel_index == 0)
        .or_else(|| surface.uv_sets.first())
    else {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface UV set is required".to_owned(),
        ));
    };
    if uv_set.coordinates.len() != mesh.positions.len() {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface UV coordinate count must match mesh position count".to_owned(),
        ));
    }
    if uv_set
        .coordinates
        .iter()
        .any(|coordinate| !coordinate[0].is_finite() || !coordinate[1].is_finite())
    {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface UV coordinates must be finite".to_owned(),
        ));
    }
    if surface.material_slots.is_empty()
        || surface
            .material_slots
            .iter()
            .any(|slot| slot.slot_id.trim().is_empty())
    {
        return Err(StaticPropGlbError::InvalidMetadata(
            "surface material slot IDs are required".to_owned(),
        ));
    }
    Ok(())
}

fn checked_byte_len(
    count: usize,
    stride: u32,
    label: &'static str,
) -> Result<u32, StaticPropGlbError> {
    u32::try_from(count)
        .ok()
        .and_then(|value| value.checked_mul(stride))
        .ok_or_else(|| StaticPropGlbError::InvalidMesh(format!("{label} buffer is too large")))
}

fn align4(value: u32) -> u32 {
    (value + 3) & !3
}

fn write_f32_triplets(bytes: &mut [u8], offset: usize, values: &[[f32; 3]]) {
    let mut cursor = offset;
    for triplet in values {
        for value in triplet {
            bytes[cursor..cursor + 4].copy_from_slice(&value.to_le_bytes());
            cursor += 4;
        }
    }
}

fn write_f32_pairs(bytes: &mut [u8], offset: usize, values: &[[f32; 2]]) {
    let mut cursor = offset;
    for pair in values {
        for value in pair {
            bytes[cursor..cursor + 4].copy_from_slice(&value.to_le_bytes());
            cursor += 4;
        }
    }
}

fn write_u32_values(bytes: &mut [u8], offset: usize, values: &[u32]) {
    let mut cursor = offset;
    for value in values {
        bytes[cursor..cursor + 4].copy_from_slice(&value.to_le_bytes());
        cursor += 4;
    }
}

fn pad_json(bytes: &mut Vec<u8>) {
    while !bytes.len().is_multiple_of(4) {
        bytes.push(b' ');
    }
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    bytes
        .get(offset..offset + 4)
        .map(|slice| u32::from_le_bytes(slice.try_into().expect("slice length checked")))
}

#[cfg(test)]
mod tests {
    use glam::Vec3;
    use shape_core::Aabb;

    use crate::surface::{
        SURFACE_ARTIFACT_SCHEMA_VERSION, SurfaceArtifact, SurfaceArtifactEvidence,
        SurfaceMaterialSlot, SurfaceReviewStatus, SurfaceTextureSet, SurfaceTriangleBinding,
        SurfaceUvSet,
    };

    use super::*;

    #[test]
    fn static_prop_glb_is_deterministic_and_valid() {
        let mesh = triangle_mesh();
        let metadata = metadata();

        let first = encode_static_prop_glb(&mesh, &metadata).expect("first glb");
        let second = encode_static_prop_glb(&mesh, &metadata).expect("second glb");

        assert_eq!(first, second);
        let report = validate_static_prop_glb(&first);
        assert!(report.valid, "{report:#?}");
        assert_eq!(report.version, Some(2));
        assert!(
            first
                .windows("painted_metal_body".len())
                .any(|window| { window == b"painted_metal_body" })
        );
    }

    #[test]
    fn static_prop_glb_rejects_invalid_mesh() {
        let mut mesh = triangle_mesh();
        mesh.indices = vec![0, 1];

        let error = encode_static_prop_glb(&mesh, &metadata()).expect_err("invalid mesh");

        assert!(matches!(error, StaticPropGlbError::InvalidMesh(_)));
    }

    #[test]
    fn static_prop_glb_validation_rejects_bad_json_or_missing_bin() {
        let mesh = triangle_mesh();
        let valid = encode_static_prop_glb(&mesh, &metadata()).expect("valid glb");
        let mut bad_json = valid.clone();
        let json_start = 20;
        bad_json[json_start] = b'{';
        bad_json[json_start + 1] = b']';
        let bad_json_report = validate_static_prop_glb(&bad_json);

        assert!(!bad_json_report.valid);
        assert!(
            bad_json_report
                .issue_codes
                .contains(&"glb_json_invalid".to_owned()),
            "{bad_json_report:#?}"
        );

        let declared_without_bin = 20 + read_u32(&valid, 12).expect("json length") as usize;
        let mut missing_bin = valid[..declared_without_bin].to_vec();
        let missing_bin_length = missing_bin.len() as u32;
        missing_bin[8..12].copy_from_slice(&missing_bin_length.to_le_bytes());
        let missing_bin_report = validate_static_prop_glb(&missing_bin);

        assert!(!missing_bin_report.valid);
        assert!(
            missing_bin_report
                .issue_codes
                .contains(&"glb_bin_chunk_missing".to_owned()),
            "{missing_bin_report:#?}"
        );
    }

    #[test]
    fn geometry_glb_remains_geometry_only() {
        let mesh = triangle_mesh();
        let bytes = encode_static_prop_glb(&mesh, &metadata()).expect("geometry glb");
        let text = String::from_utf8_lossy(&bytes);

        assert!(text.contains("geometry-only-portable-static-mesh-handoff"));
        assert!(text.contains("uv-layout"));
        assert!(!text.contains("TEXCOORD_0"));
        assert!(!text.contains("\"skins\""));
        assert!(!text.contains("\"animations\""));
    }

    #[test]
    fn surface_glb_validation_rejects_geometry_only_glb() {
        let mesh = triangle_mesh();
        let bytes = encode_static_prop_glb(&mesh, &metadata()).expect("geometry glb");

        let report = validate_static_prop_surface_glb(&bytes);

        assert!(!report.valid);
        assert!(
            report
                .issue_codes
                .contains(&"gltf_texcoord0_missing".to_owned()),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_glb_includes_texcoord0_and_preserves_material_slot_ids() {
        let mesh = triangle_mesh();
        let surface = surface_artifact();

        let bytes =
            encode_static_prop_surface_glb(&mesh, &surface, "surface/surface-artifact.json")
                .expect("surface glb");
        let report = validate_static_prop_surface_glb(&bytes);
        let text = String::from_utf8_lossy(&bytes);

        assert!(report.valid, "{report:#?}");
        assert!(text.contains("TEXCOORD_0"));
        assert!(text.contains("surface/surface-artifact.json"));
        assert!(!text.contains("surface/surface-validation-report.json"));
        assert!(text.contains("painted_metal_body"));
        assert!(text.contains("sidecar-texture-files"));
        assert!(!text.contains("\"skins\""));
        assert!(!text.contains("\"animations\""));
    }

    #[test]
    fn surface_glb_validation_rejects_malformed_texcoord0_accessor() {
        let mesh = triangle_mesh();
        let surface = surface_artifact();
        let bytes =
            encode_static_prop_surface_glb(&mesh, &surface, "surface/surface-artifact.json")
                .expect("surface glb");
        let mut value = parse_glb_json(&bytes).expect("surface json");
        value["accessors"][2]["count"] = Value::from(2_u64);
        let bin = parse_glb_bin(&bytes).expect("bin").to_vec();
        let malformed = encode_glb_json_and_bin(value, bin.len() as u32, bin).expect("malformed");

        let report = validate_static_prop_surface_glb(&malformed);

        assert!(!report.valid);
        assert!(
            report
                .issue_codes
                .contains(&"gltf_texcoord0_count_mismatch".to_owned()),
            "{report:#?}"
        );
    }

    #[test]
    fn surface_glb_validation_rejects_wrong_surface_artifact_ref() {
        let mesh = triangle_mesh();
        let surface = surface_artifact();
        let bytes = encode_static_prop_surface_glb(&mesh, &surface, "surface/wrong.json")
            .expect("surface glb");

        let report = validate_static_prop_surface_glb_with_artifact_ref(
            &bytes,
            "surface/surface-artifact.json",
        );

        assert!(!report.valid);
        assert!(
            report
                .issue_codes
                .contains(&"gltf_surface_artifact_ref_mismatch".to_owned()),
            "{report:#?}"
        );
    }

    fn metadata() -> StaticPropGlbMetadata {
        StaticPropGlbMetadata {
            profile_id: "sci-fi-crate".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            material_slots: vec!["painted_metal_body".to_owned()],
        }
    }

    fn triangle_mesh() -> TriangleMesh {
        TriangleMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            indices: vec![0, 1, 2],
            bounds: Aabb {
                min: Vec3::ZERO,
                max: Vec3::new(1.0, 1.0, 0.0),
            },
        }
    }

    fn surface_artifact() -> SurfaceArtifact {
        SurfaceArtifact {
            schema_version: SURFACE_ARTIFACT_SCHEMA_VERSION,
            profile_id: "sci-fi-crate".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            source_artifact_fingerprint: "artifact:abc".to_owned(),
            source_recipe_hash: 42,
            frozen_mesh_ref: "model-package".to_owned(),
            uv_sets: vec![SurfaceUvSet {
                id: "uv0".to_owned(),
                display_name: "UV0".to_owned(),
                channel_index: 0,
                coordinate_count: 3,
                coordinates: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                source_policy: "test projection".to_owned(),
                readiness_status: crate::export::StaticPropFeatureStatus::Ready,
                tiling_allowed: false,
            }],
            material_slots: vec![SurfaceMaterialSlot {
                slot_id: "painted_metal_body".to_owned(),
                display_name: "Painted Metal Body".to_owned(),
                semantic_roles: vec!["body".to_owned()],
                recipe_id: "worn-painted-sci-fi-metal".to_owned(),
                coverage_triangle_count: 1,
                coverage_fraction: 1.0,
            }],
            texture_sets: Vec::<SurfaceTextureSet>::new(),
            triangle_bindings: vec![SurfaceTriangleBinding {
                triangle_index: 0,
                material_slot_id: "painted_metal_body".to_owned(),
                uv_set_id: "uv0".to_owned(),
                source_part: Some("compiled_part_001".to_owned()),
                source_region: None,
                source_operation: None,
            }],
            evidence: SurfaceArtifactEvidence {
                uv_layout: "surface/uv-layout.png".to_owned(),
                material_swatch_sheet: "surface/material-swatch-sheet.png".to_owned(),
                texture_contact_sheet: "surface/texture-contact-sheet.png".to_owned(),
                triangle_slot_coverage: "surface/triangle-slot-coverage.json".to_owned(),
            },
            validation_report_ref: "surface/surface-validation-report.json".to_owned(),
            manual_review: SurfaceReviewStatus::AutomatedReady,
        }
    }
}
