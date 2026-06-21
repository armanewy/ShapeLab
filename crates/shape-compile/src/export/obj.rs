use std::fmt::Write;

use serde::{Deserialize, Serialize};
use shape_asset::AssetRecipe;

use crate::AssetArtifact;

use super::{ExportError, part_regions, safe_part_name};

/// OBJ export plus provenance sidecar text and exact count report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GroupedObjExport {
    /// Wavefront OBJ text.
    pub obj: String,
    /// Pretty JSON provenance sidecar.
    pub provenance_json: String,
    /// Exact OBJ export report.
    pub report: GroupedObjReport,
}

/// Exact OBJ export counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupedObjReport {
    /// Source recipe hash from the compiled artifact.
    pub source_recipe_hash: u64,
    /// Number of exported OBJ objects/groups.
    pub object_count: u64,
    /// Number of OBJ `v` records.
    pub vertex_count: u64,
    /// Number of OBJ `vn` records.
    pub normal_count: u64,
    /// Number of OBJ `f` records.
    pub face_count: u64,
    /// Per-object counts in OBJ order.
    pub objects: Vec<GroupedObjObjectReport>,
}

/// Per-object OBJ count report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroupedObjObjectReport {
    /// Stable object/group name.
    pub name: String,
    /// Source part instance ID.
    pub instance_id: u64,
    /// Source part definition ID.
    pub definition_id: u64,
    /// Number of OBJ `v` records for this object.
    pub vertex_count: u64,
    /// Number of OBJ `vn` records for this object.
    pub normal_count: u64,
    /// Number of OBJ `f` records for this object.
    pub face_count: u64,
}

/// Write deterministic grouped OBJ and provenance sidecar text.
pub fn write_grouped_obj_export(
    artifact: &AssetArtifact,
    recipe: Option<&AssetRecipe>,
) -> Result<GroupedObjExport, ExportError> {
    let mut obj = String::new();
    writeln!(&mut obj, "# Shape Lab explicit model export").map_err(|_| ExportError::Format)?;
    writeln!(
        &mut obj,
        "# source_recipe_hash {}",
        artifact.source_recipe_hash
    )
    .map_err(|_| ExportError::Format)?;

    let mut vertex_offset = 1_u32;
    let mut report = GroupedObjReport {
        source_recipe_hash: artifact.source_recipe_hash,
        object_count: 0,
        vertex_count: 0,
        normal_count: 0,
        face_count: 0,
        objects: Vec::new(),
    };

    for part in super::ordered_parts(artifact) {
        let name = safe_part_name(part);
        let vertex_count = part.triangulated_world.mesh.positions.len() as u64;
        let normal_count = part.triangulated_world.mesh.normals.len() as u64;
        let face_count = (part.triangulated_world.mesh.indices.len() / 3) as u64;
        writeln!(&mut obj, "o {name}").map_err(|_| ExportError::Format)?;
        writeln!(&mut obj, "g {name}").map_err(|_| ExportError::Format)?;
        writeln!(&mut obj, "# part_instance_id {}", part.instance_id.0)
            .map_err(|_| ExportError::Format)?;
        writeln!(&mut obj, "# part_definition_id {}", part.definition_id.0)
            .map_err(|_| ExportError::Format)?;
        if let Some(recipe) = recipe {
            for (region, region_name, region_face_count) in part_regions(recipe, part) {
                writeln!(
                    &mut obj,
                    "# region {} {} faces={}",
                    region.0, region_name, region_face_count
                )
                .map_err(|_| ExportError::Format)?;
            }
        }
        writeln!(
            &mut obj,
            "# counts vertices={} normals={} faces={}",
            vertex_count, normal_count, face_count
        )
        .map_err(|_| ExportError::Format)?;
        for position in &part.triangulated_world.mesh.positions {
            writeln!(
                &mut obj,
                "v {:.9} {:.9} {:.9}",
                position[0], position[1], position[2]
            )
            .map_err(|_| ExportError::Format)?;
        }
        for normal in &part.triangulated_world.mesh.normals {
            writeln!(
                &mut obj,
                "vn {:.9} {:.9} {:.9}",
                normal[0], normal[1], normal[2]
            )
            .map_err(|_| ExportError::Format)?;
        }
        for triangle in part.triangulated_world.mesh.indices.chunks_exact(3) {
            let a = triangle[0]
                .checked_add(vertex_offset)
                .ok_or(ExportError::Format)?;
            let b = triangle[1]
                .checked_add(vertex_offset)
                .ok_or(ExportError::Format)?;
            let c = triangle[2]
                .checked_add(vertex_offset)
                .ok_or(ExportError::Format)?;
            writeln!(&mut obj, "f {a}//{a} {b}//{b} {c}//{c}").map_err(|_| ExportError::Format)?;
        }
        let add = u32::try_from(part.triangulated_world.mesh.positions.len())
            .map_err(|_| ExportError::Format)?;
        vertex_offset = vertex_offset.checked_add(add).ok_or(ExportError::Format)?;

        report.object_count += 1;
        report.vertex_count += vertex_count;
        report.normal_count += normal_count;
        report.face_count += face_count;
        report.objects.push(GroupedObjObjectReport {
            name,
            instance_id: part.instance_id.0,
            definition_id: part.definition_id.0,
            vertex_count,
            normal_count,
            face_count,
        });
    }

    writeln!(
        &mut obj,
        "# total_counts objects={} vertices={} normals={} faces={}",
        report.object_count, report.vertex_count, report.normal_count, report.face_count
    )
    .map_err(|_| ExportError::Format)?;

    Ok(GroupedObjExport {
        obj,
        provenance_json: serde_json::to_string_pretty(&artifact.provenance_report)?,
        report,
    })
}
