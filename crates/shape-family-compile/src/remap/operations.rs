//! Modeling-operation remap boundary.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    BoundaryLoopId, CountRangeHint, ModelingOperationSpec, OperationId, PartDefinitionId,
    PartInstanceId, RegionId, SemanticCutGroupHint, SocketId,
};

use super::{FragmentRemap, FragmentRemapError};

/// Validate that operation remapping is intentionally routed through this module.
pub fn unsupported_operation_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "operations".to_owned(),
        reason: reason.to_owned(),
    }
}

/// Remap an ordered operation list without changing order or phase.
pub fn remap_modeling_operations(
    fragment: &str,
    operations: &[ModelingOperationSpec],
    remap: &FragmentRemap,
) -> Result<Vec<ModelingOperationSpec>, FragmentRemapError> {
    let mut remapped = Vec::with_capacity(operations.len());
    let mut seen_targets = BTreeSet::new();
    for operation in operations {
        let source_operation = operation.operation_id();
        let remapped_operation = remap_modeling_operation(fragment, operation, remap)?;
        let target_operation = remapped_operation.operation_id();
        if !seen_targets.insert(target_operation) {
            return Err(duplicate_mapping(
                fragment,
                source_operation,
                "operation target",
                target_operation.0,
            ));
        }
        remapped.push(remapped_operation);
    }
    Ok(remapped)
}

/// Remap one deterministic modeling operation specification.
pub fn remap_modeling_operation(
    fragment: &str,
    operation: &ModelingOperationSpec,
    remap: &FragmentRemap,
) -> Result<ModelingOperationSpec, FragmentRemapError> {
    let source_operation = operation.operation_id();
    let remapped_operation = remap_operation_in_context(
        fragment,
        source_operation,
        source_operation,
        remap,
        "operation",
    )?;
    match operation {
        ModelingOperationSpec::TransformGeometry { transform, .. } => {
            Ok(ModelingOperationSpec::TransformGeometry {
                operation: remapped_operation,
                transform: transform.clone(),
            })
        }
        ModelingOperationSpec::SetBevelProfile {
            radius, segments, ..
        } => Ok(ModelingOperationSpec::SetBevelProfile {
            operation: remapped_operation,
            radius: *radius,
            segments: *segments,
        }),
        ModelingOperationSpec::AddPanel {
            region,
            inset,
            depth,
            ..
        } => Ok(ModelingOperationSpec::AddPanel {
            operation: remapped_operation,
            region: remap_region_in_context(fragment, source_operation, *region, remap)?,
            inset: *inset,
            depth: *depth,
        }),
        ModelingOperationSpec::AddTrim {
            region,
            width,
            height,
            ..
        } => Ok(ModelingOperationSpec::AddTrim {
            operation: remapped_operation,
            region: remap_region_in_context(fragment, source_operation, *region, remap)?,
            width: *width,
            height: *height,
        }),
        ModelingOperationSpec::RecessedPanelCut {
            region,
            face,
            center,
            size,
            depth,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            floor_loop,
            outer_region,
            rim_region,
            wall_region,
            floor_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::RecessedPanelCut {
            operation: remapped_operation,
            region: remap_region_in_context(fragment, source_operation, *region, remap)?,
            face: *face,
            center: *center,
            size: *size,
            depth: *depth,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *entry_loop,
                remap,
            )?,
            floor_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *floor_loop,
                remap,
            )?,
            outer_region: remap_region_in_context(
                fragment,
                source_operation,
                *outer_region,
                remap,
            )?,
            rim_region: remap_region_in_context(fragment, source_operation, *rim_region, remap)?,
            wall_region: remap_region_in_context(fragment, source_operation, *wall_region, remap)?,
            floor_region: remap_region_in_context(
                fragment,
                source_operation,
                *floor_region,
                remap,
            )?,
            edge_treatment: *edge_treatment,
        }),
        ModelingOperationSpec::RectangularThroughCut {
            region,
            face,
            center,
            size,
            corner_radius,
            rim_width,
            corner_segments,
            entry_loop,
            exit_loop,
            outer_region,
            rim_region,
            wall_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::RectangularThroughCut {
            operation: remapped_operation,
            region: remap_region_in_context(fragment, source_operation, *region, remap)?,
            face: *face,
            center: *center,
            size: *size,
            corner_radius: *corner_radius,
            rim_width: *rim_width,
            corner_segments: *corner_segments,
            entry_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *entry_loop,
                remap,
            )?,
            exit_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *exit_loop,
                remap,
            )?,
            outer_region: remap_region_in_context(
                fragment,
                source_operation,
                *outer_region,
                remap,
            )?,
            rim_region: remap_region_in_context(fragment, source_operation, *rim_region, remap)?,
            wall_region: remap_region_in_context(fragment, source_operation, *wall_region, remap)?,
            edge_treatment: *edge_treatment,
        }),
        ModelingOperationSpec::CircularThroughCut {
            region,
            face,
            center,
            radius,
            radial_segments,
            rim_width,
            entry_loop,
            exit_loop,
            outer_region,
            rim_region,
            wall_region,
            edge_treatment,
            ..
        } => Ok(ModelingOperationSpec::CircularThroughCut {
            operation: remapped_operation,
            region: remap_region_in_context(fragment, source_operation, *region, remap)?,
            face: *face,
            center: *center,
            radius: *radius,
            radial_segments: *radial_segments,
            rim_width: *rim_width,
            entry_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *entry_loop,
                remap,
            )?,
            exit_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *exit_loop,
                remap,
            )?,
            outer_region: remap_region_in_context(
                fragment,
                source_operation,
                *outer_region,
                remap,
            )?,
            rim_region: remap_region_in_context(fragment, source_operation, *rim_region, remap)?,
            wall_region: remap_region_in_context(fragment, source_operation, *wall_region, remap)?,
            edge_treatment: *edge_treatment,
        }),
        ModelingOperationSpec::BevelBoundaryLoop {
            target_loop,
            width,
            segments,
            profile,
            bevel_region,
            outer_replacement_loop,
            inner_replacement_loop,
            ..
        } => Ok(ModelingOperationSpec::BevelBoundaryLoop {
            operation: remapped_operation,
            target_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *target_loop,
                remap,
            )?,
            width: *width,
            segments: *segments,
            profile: *profile,
            bevel_region: remap_region_in_context(
                fragment,
                source_operation,
                *bevel_region,
                remap,
            )?,
            outer_replacement_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *outer_replacement_loop,
                remap,
            )?,
            inner_replacement_loop: remap_boundary_loop_in_context(
                fragment,
                source_operation,
                *inner_replacement_loop,
                remap,
            )?,
        }),
        ModelingOperationSpec::MirrorInstances {
            plane_normal,
            plane_offset,
            ..
        } => Ok(ModelingOperationSpec::MirrorInstances {
            operation: remapped_operation,
            plane_normal: *plane_normal,
            plane_offset: *plane_offset,
        }),
        ModelingOperationSpec::LinearArray { count, offset, .. } => {
            Ok(ModelingOperationSpec::LinearArray {
                operation: remapped_operation,
                count: *count,
                offset: *offset,
            })
        }
        ModelingOperationSpec::RadialArray {
            count,
            axis,
            angle_degrees,
            ..
        } => Ok(ModelingOperationSpec::RadialArray {
            operation: remapped_operation,
            count: *count,
            axis: *axis,
            angle_degrees: *angle_degrees,
        }),
        ModelingOperationSpec::ReservedBoolean { label, .. } => {
            Ok(ModelingOperationSpec::ReservedBoolean {
                operation: remapped_operation,
                label: label.clone(),
            })
        }
        ModelingOperationSpec::ReservedDeformationProgram { label, .. } => {
            Ok(ModelingOperationSpec::ReservedDeformationProgram {
                operation: remapped_operation,
                label: label.clone(),
            })
        }
    }
}

/// Remap a definition reference through the fragment map.
pub fn remap_definition_reference(
    fragment: &str,
    definition: PartDefinitionId,
    remap: &FragmentRemap,
) -> Result<PartDefinitionId, FragmentRemapError> {
    remap.definitions.get(&definition).copied().ok_or_else(|| {
        missing_mapping(
            fragment,
            None,
            "part definition",
            definition.0,
            "definition reference",
        )
    })
}

/// Remap an instance reference through the fragment map.
pub fn remap_instance_reference(
    fragment: &str,
    instance: PartInstanceId,
    remap: &FragmentRemap,
) -> Result<PartInstanceId, FragmentRemapError> {
    remap.instances.get(&instance).copied().ok_or_else(|| {
        missing_mapping(
            fragment,
            None,
            "part instance",
            instance.0,
            "instance reference",
        )
    })
}

/// Remap an operation reference through the fragment map.
pub fn remap_operation_reference(
    fragment: &str,
    operation: OperationId,
    remap: &FragmentRemap,
) -> Result<OperationId, FragmentRemapError> {
    remap_operation_in_context(fragment, operation, operation, remap, "operation reference")
}

/// Remap a generated-occurrence provenance marker.
pub fn remap_generated_by(
    fragment: &str,
    generated_by: Option<OperationId>,
    remap: &FragmentRemap,
) -> Result<Option<OperationId>, FragmentRemapError> {
    generated_by
        .map(|operation| {
            remap_operation_in_context(
                fragment,
                operation,
                operation,
                remap,
                "generated occurrence provenance",
            )
        })
        .transpose()
}

/// Remap a region reference through the fragment map.
pub fn remap_region_reference(
    fragment: &str,
    operation: OperationId,
    region: RegionId,
    remap: &FragmentRemap,
) -> Result<RegionId, FragmentRemapError> {
    remap_region_in_context(fragment, operation, region, remap)
}

/// Remap a boundary-loop reference through the fragment map.
pub fn remap_boundary_loop_reference(
    fragment: &str,
    operation: OperationId,
    boundary_loop: BoundaryLoopId,
    remap: &FragmentRemap,
) -> Result<BoundaryLoopId, FragmentRemapError> {
    remap_boundary_loop_in_context(fragment, operation, boundary_loop, remap)
}

/// Remap a socket reference through the fragment map.
pub fn remap_socket_reference(
    fragment: &str,
    socket: SocketId,
    remap: &FragmentRemap,
) -> Result<SocketId, FragmentRemapError> {
    remap
        .sockets
        .get(&socket)
        .copied()
        .ok_or_else(|| missing_mapping(fragment, None, "socket", socket.0, "socket reference"))
}

/// Remap array count-range hints keyed by operation ID.
pub fn remap_operation_count_ranges(
    fragment: &str,
    count_ranges: &BTreeMap<OperationId, CountRangeHint>,
    remap: &FragmentRemap,
) -> Result<BTreeMap<OperationId, CountRangeHint>, FragmentRemapError> {
    let mut remapped = BTreeMap::new();
    for (operation, range) in count_ranges {
        let target_operation = remap_operation_in_context(
            fragment,
            *operation,
            *operation,
            remap,
            "count range operation",
        )?;
        if remapped.insert(target_operation, *range).is_some() {
            return Err(duplicate_mapping(
                fragment,
                *operation,
                "count range operation target",
                target_operation.0,
            ));
        }
    }
    Ok(remapped)
}

/// Remap one semantic cut-group hint while preserving operation order.
pub fn remap_semantic_cut_group_hint(
    fragment: &str,
    hint: &SemanticCutGroupHint,
    remap: &FragmentRemap,
) -> Result<SemanticCutGroupHint, FragmentRemapError> {
    let mut operations = Vec::with_capacity(hint.operations.len());
    let mut seen_targets = BTreeSet::new();
    for operation in &hint.operations {
        let target_operation = remap_operation_in_context(
            fragment,
            *operation,
            *operation,
            remap,
            "semantic cut-group operation",
        )?;
        if !seen_targets.insert(target_operation) {
            return Err(duplicate_mapping(
                fragment,
                *operation,
                "semantic cut-group operation target",
                target_operation.0,
            ));
        }
        operations.push(target_operation);
    }
    Ok(SemanticCutGroupHint {
        label: hint.label.clone(),
        definition: remap_definition_reference(fragment, hint.definition, remap)?,
        operations,
        role: hint.role.clone(),
        count_range: hint.count_range,
    })
}

fn remap_operation_in_context(
    fragment: &str,
    context_operation: OperationId,
    operation: OperationId,
    remap: &FragmentRemap,
    context: &'static str,
) -> Result<OperationId, FragmentRemapError> {
    remap.operations.get(&operation).copied().ok_or_else(|| {
        missing_mapping(
            fragment,
            Some(context_operation),
            "operation",
            operation.0,
            context,
        )
    })
}

fn remap_region_in_context(
    fragment: &str,
    operation: OperationId,
    region: RegionId,
    remap: &FragmentRemap,
) -> Result<RegionId, FragmentRemapError> {
    remap.regions.get(&region).copied().ok_or_else(|| {
        missing_mapping(
            fragment,
            Some(operation),
            "region",
            region.0,
            "operation region reference",
        )
    })
}

fn remap_boundary_loop_in_context(
    fragment: &str,
    operation: OperationId,
    boundary_loop: BoundaryLoopId,
    remap: &FragmentRemap,
) -> Result<BoundaryLoopId, FragmentRemapError> {
    remap
        .boundary_loops
        .get(&boundary_loop)
        .copied()
        .ok_or_else(|| {
            missing_mapping(
                fragment,
                Some(operation),
                "boundary loop",
                boundary_loop.0,
                "operation boundary-loop reference",
            )
        })
}

fn missing_mapping(
    fragment: &str,
    operation: Option<OperationId>,
    id_kind: &'static str,
    id: u64,
    context: &'static str,
) -> FragmentRemapError {
    let id_kind = match operation {
        Some(operation) => format!("{id_kind} {context} for operation {}", operation.0),
        None => format!("{id_kind} {context}"),
    };
    FragmentRemapError::MissingMapping {
        fragment: fragment.to_owned(),
        id_kind,
        id: id.to_string(),
    }
}

fn duplicate_mapping(
    fragment: &str,
    operation: OperationId,
    id_kind: &'static str,
    id: u64,
) -> FragmentRemapError {
    FragmentRemapError::DuplicateMapping {
        fragment: fragment.to_owned(),
        id_kind: format!("{id_kind} for operation {}", operation.0),
        id: id.to_string(),
    }
}
