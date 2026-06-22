//! Variation metadata remap boundary.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AuthoredVariationMetadata, OperationId, ParameterDescriptor, ParameterId, PartDefinitionId,
    PartInstanceId, ReplacementGroupHint, SemanticCutGroupHint,
};

use super::{FragmentRemap, FragmentRemapError};

/// Remapped parameter and non-authoritative variation metadata for one fragment.
#[derive(Debug, Clone, PartialEq)]
pub struct RemappedVariationMetadata {
    /// Editable parameter descriptors keyed by remapped parameter ID.
    pub parameters: BTreeMap<ParameterId, ParameterDescriptor>,
    /// Locked parameter IDs.
    pub parameter_locks: BTreeSet<ParameterId>,
    /// Locked instance IDs.
    pub instance_locks: BTreeSet<PartInstanceId>,
    /// Locked instance subtree roots.
    pub subtree_locks: BTreeSet<PartInstanceId>,
    /// Locked topology definition IDs.
    pub topology_locks: BTreeSet<PartDefinitionId>,
    /// Authored variation metadata.
    pub variation: AuthoredVariationMetadata,
}

/// Borrowed source metadata consumed by [`remap_fragment_variation_metadata`].
#[derive(Debug, Copy, Clone)]
pub struct VariationMetadataSource<'a> {
    /// Editable parameter descriptors keyed by source parameter ID.
    pub parameters: &'a BTreeMap<ParameterId, ParameterDescriptor>,
    /// Locked source parameter IDs.
    pub parameter_locks: &'a BTreeSet<ParameterId>,
    /// Locked source instance IDs.
    pub instance_locks: &'a BTreeSet<PartInstanceId>,
    /// Locked source instance subtree roots.
    pub subtree_locks: &'a BTreeSet<PartInstanceId>,
    /// Topology-locked source definition IDs.
    pub topology_locks: &'a BTreeSet<PartDefinitionId>,
    /// Source authored variation metadata.
    pub variation: &'a AuthoredVariationMetadata,
}

/// Validate that variation remapping is intentionally routed through this module.
pub fn unsupported_variation_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "variation".to_owned(),
        reason: reason.to_owned(),
    }
}

/// Remap all parameter, lock, and authored variation metadata owned by this stage.
pub fn remap_fragment_variation_metadata(
    fragment: &str,
    source: VariationMetadataSource<'_>,
    remap: &FragmentRemap,
) -> Result<RemappedVariationMetadata, FragmentRemapError> {
    Ok(RemappedVariationMetadata {
        parameters: remap_parameter_descriptors(fragment, source.parameters, remap)?,
        parameter_locks: remap_parameter_locks(fragment, source.parameter_locks, remap)?,
        instance_locks: remap_instance_locks(fragment, source.instance_locks, remap)?,
        subtree_locks: remap_subtree_locks(fragment, source.subtree_locks, remap)?,
        topology_locks: remap_topology_locks(fragment, source.topology_locks, remap)?,
        variation: remap_variation_metadata(fragment, source.variation, remap)?,
    })
}

/// Remap parameter descriptors and their typed scalar paths.
pub fn remap_parameter_descriptors(
    fragment: &str,
    parameters: &BTreeMap<ParameterId, ParameterDescriptor>,
    remap: &FragmentRemap,
) -> Result<BTreeMap<ParameterId, ParameterDescriptor>, FragmentRemapError> {
    let mut remapped = BTreeMap::new();
    for (source_id, descriptor) in parameters {
        let target_key = remap_parameter_id(fragment, *source_id, remap)?;
        let descriptor = remap_parameter_descriptor(fragment, descriptor, remap)?;
        if target_key != descriptor.id {
            return Err(unsupported_variation_remap(
                fragment,
                "parameter descriptor ID and map key remap to different targets",
            ));
        }
        insert_unique(
            fragment,
            &mut remapped,
            descriptor.id,
            descriptor,
            "parameter",
        )?;
    }
    Ok(remapped)
}

/// Remap one parameter descriptor and its canonical scalar path.
pub fn remap_parameter_descriptor(
    fragment: &str,
    descriptor: &ParameterDescriptor,
    remap: &FragmentRemap,
) -> Result<ParameterDescriptor, FragmentRemapError> {
    let mut descriptor = descriptor.clone();
    descriptor.id = remap_parameter_id(fragment, descriptor.id, remap)?;
    descriptor.path = remap_scalar_path(fragment, &descriptor.path, remap)?;
    Ok(descriptor)
}

/// Remap a typed scalar path.
///
/// The parser accepts only scalar paths that Shape Lab knows how to read and
/// write. It remaps semantic IDs in parsed root segments and operation segments
/// instead of applying arbitrary text replacement.
pub fn remap_scalar_path(
    fragment: &str,
    path: &str,
    remap: &FragmentRemap,
) -> Result<String, FragmentRemapError> {
    ScalarPath::parse(fragment, path)?.remap(fragment, remap)
}

/// Remap locked parameter IDs.
pub fn remap_parameter_locks(
    fragment: &str,
    locks: &BTreeSet<ParameterId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<ParameterId>, FragmentRemapError> {
    remap_parameter_set(fragment, locks, remap)
}

/// Remap locked instance IDs.
pub fn remap_instance_locks(
    fragment: &str,
    locks: &BTreeSet<PartInstanceId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartInstanceId>, FragmentRemapError> {
    remap_instance_set(fragment, locks, remap)
}

/// Remap locked subtree root instance IDs.
pub fn remap_subtree_locks(
    fragment: &str,
    locks: &BTreeSet<PartInstanceId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartInstanceId>, FragmentRemapError> {
    remap_instance_set(fragment, locks, remap)
}

/// Remap topology-locked definition IDs.
pub fn remap_topology_locks(
    fragment: &str,
    locks: &BTreeSet<PartDefinitionId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartDefinitionId>, FragmentRemapError> {
    remap_definition_set(fragment, locks, remap)
}

/// Remap authored variation metadata.
pub fn remap_variation_metadata(
    fragment: &str,
    metadata: &AuthoredVariationMetadata,
    remap: &FragmentRemap,
) -> Result<AuthoredVariationMetadata, FragmentRemapError> {
    let mut replacement_groups = BTreeMap::new();
    for (group, hint) in &metadata.replacement_groups {
        insert_unique(
            fragment,
            &mut replacement_groups,
            group.clone(),
            ReplacementGroupHint {
                definitions: remap_definition_set(fragment, &hint.definitions, remap)?,
            },
            "replacement group",
        )?;
    }

    let mut count_ranges = BTreeMap::new();
    for (operation, range) in &metadata.count_ranges {
        insert_unique(
            fragment,
            &mut count_ranges,
            remap_operation_id(fragment, *operation, remap)?,
            *range,
            "count range",
        )?;
    }

    let mut parameter_range_overrides = BTreeMap::new();
    for (parameter, range) in &metadata.parameter_range_overrides {
        insert_unique(
            fragment,
            &mut parameter_range_overrides,
            remap_parameter_id(fragment, *parameter, remap)?,
            *range,
            "parameter range override",
        )?;
    }

    let mut semantic_cut_groups = BTreeMap::new();
    for (group, hint) in &metadata.semantic_cut_groups {
        insert_unique(
            fragment,
            &mut semantic_cut_groups,
            group.clone(),
            remap_semantic_cut_group(fragment, hint, remap)?,
            "semantic cut group",
        )?;
    }

    Ok(AuthoredVariationMetadata {
        optional_instances: remap_instance_set(fragment, &metadata.optional_instances, remap)?,
        replacement_groups,
        count_ranges,
        parameter_range_overrides,
        semantic_cut_groups,
    })
}

fn remap_semantic_cut_group(
    fragment: &str,
    hint: &SemanticCutGroupHint,
    remap: &FragmentRemap,
) -> Result<SemanticCutGroupHint, FragmentRemapError> {
    Ok(SemanticCutGroupHint {
        label: hint.label.clone(),
        definition: remap_definition_id(fragment, hint.definition, remap)?,
        operations: hint
            .operations
            .iter()
            .map(|operation| remap_operation_id(fragment, *operation, remap))
            .collect::<Result<Vec<_>, _>>()?,
        role: hint.role.clone(),
        count_range: hint.count_range,
    })
}

fn remap_definition_set(
    fragment: &str,
    source: &BTreeSet<PartDefinitionId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartDefinitionId>, FragmentRemapError> {
    let mut target = BTreeSet::new();
    for id in source {
        insert_unique_set(
            fragment,
            &mut target,
            remap_definition_id(fragment, *id, remap)?,
            "part definition",
        )?;
    }
    Ok(target)
}

fn remap_instance_set(
    fragment: &str,
    source: &BTreeSet<PartInstanceId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartInstanceId>, FragmentRemapError> {
    let mut target = BTreeSet::new();
    for id in source {
        insert_unique_set(
            fragment,
            &mut target,
            remap_instance_id(fragment, *id, remap)?,
            "part instance",
        )?;
    }
    Ok(target)
}

fn remap_parameter_set(
    fragment: &str,
    source: &BTreeSet<ParameterId>,
    remap: &FragmentRemap,
) -> Result<BTreeSet<ParameterId>, FragmentRemapError> {
    let mut target = BTreeSet::new();
    for id in source {
        insert_unique_set(
            fragment,
            &mut target,
            remap_parameter_id(fragment, *id, remap)?,
            "parameter",
        )?;
    }
    Ok(target)
}

fn remap_definition_id(
    fragment: &str,
    id: PartDefinitionId,
    remap: &FragmentRemap,
) -> Result<PartDefinitionId, FragmentRemapError> {
    remap
        .definitions
        .get(&id)
        .copied()
        .ok_or_else(|| missing_mapping(fragment, "part definition", id.0))
}

fn remap_instance_id(
    fragment: &str,
    id: PartInstanceId,
    remap: &FragmentRemap,
) -> Result<PartInstanceId, FragmentRemapError> {
    remap
        .instances
        .get(&id)
        .copied()
        .ok_or_else(|| missing_mapping(fragment, "part instance", id.0))
}

fn remap_parameter_id(
    fragment: &str,
    id: ParameterId,
    remap: &FragmentRemap,
) -> Result<ParameterId, FragmentRemapError> {
    remap
        .parameters
        .get(&id)
        .copied()
        .ok_or_else(|| missing_mapping(fragment, "parameter", id.0))
}

fn remap_operation_id(
    fragment: &str,
    id: OperationId,
    remap: &FragmentRemap,
) -> Result<OperationId, FragmentRemapError> {
    remap
        .operations
        .get(&id)
        .copied()
        .ok_or_else(|| missing_mapping(fragment, "operation", id.0))
}

fn missing_mapping(fragment: &str, id_kind: &str, id: u64) -> FragmentRemapError {
    FragmentRemapError::MissingMapping {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id: id.to_string(),
    }
}

fn duplicate_mapping(fragment: &str, id_kind: &str, id: impl Into<String>) -> FragmentRemapError {
    FragmentRemapError::DuplicateMapping {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id: id.into(),
    }
}

fn insert_unique<K, V>(
    fragment: &str,
    target: &mut BTreeMap<K, V>,
    key: K,
    value: V,
    id_kind: &str,
) -> Result<(), FragmentRemapError>
where
    K: Ord + Clone + std::fmt::Debug,
{
    if target.insert(key.clone(), value).is_some() {
        return Err(duplicate_mapping(fragment, id_kind, format!("{key:?}")));
    }
    Ok(())
}

fn insert_unique_set<K>(
    fragment: &str,
    target: &mut BTreeSet<K>,
    key: K,
    id_kind: &str,
) -> Result<(), FragmentRemapError>
where
    K: Ord + Clone + std::fmt::Debug,
{
    if !target.insert(key.clone()) {
        return Err(duplicate_mapping(fragment, id_kind, format!("{key:?}")));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScalarPath {
    DefinitionGeometry {
        definition: PartDefinitionId,
        suffix: ValidPathSuffix,
    },
    DefinitionOperation {
        definition: PartDefinitionId,
        operation: OperationId,
        suffix: ValidPathSuffix,
    },
    InstanceTransform {
        instance: PartInstanceId,
        suffix: ValidPathSuffix,
    },
}

impl ScalarPath {
    fn parse(fragment: &str, path: &str) -> Result<Self, FragmentRemapError> {
        let parts = path.split('.').collect::<Vec<_>>();
        match parts.as_slice() {
            ["definition", id, rest @ ..] if !rest.is_empty() => {
                let definition = PartDefinitionId(parse_numeric_segment(fragment, path, id)?);
                match rest {
                    ["geometry", geometry @ ..] if !geometry.is_empty() => {
                        Ok(Self::DefinitionGeometry {
                            definition,
                            suffix: parse_geometry_scalar_path(fragment, path, geometry)?,
                        })
                    }
                    ["operation", operation, operation_path @ ..] if !operation_path.is_empty() => {
                        Ok(Self::DefinitionOperation {
                            definition,
                            operation: OperationId(parse_numeric_segment(
                                fragment, path, operation,
                            )?),
                            suffix: parse_operation_scalar_path(fragment, path, operation_path)?,
                        })
                    }
                    _ => Err(unknown_scalar_path(fragment, path)),
                }
            }
            ["instance", id, rest @ ..] if !rest.is_empty() => Ok(Self::InstanceTransform {
                instance: PartInstanceId(parse_numeric_segment(fragment, path, id)?),
                suffix: parse_instance_transform_path(fragment, path, rest)?,
            }),
            _ => Err(unknown_scalar_path(fragment, path)),
        }
    }

    fn remap(&self, fragment: &str, remap: &FragmentRemap) -> Result<String, FragmentRemapError> {
        match self {
            Self::DefinitionGeometry { definition, suffix } => Ok(format!(
                "definition.{}.geometry.{}",
                remap_definition_id(fragment, *definition, remap)?.0,
                suffix.as_path()
            )),
            Self::DefinitionOperation {
                definition,
                operation,
                suffix,
            } => Ok(format!(
                "definition.{}.operation.{}.{}",
                remap_definition_id(fragment, *definition, remap)?.0,
                remap_operation_id(fragment, *operation, remap)?.0,
                suffix.as_path()
            )),
            Self::InstanceTransform { instance, suffix } => Ok(format!(
                "instance.{}.{}",
                remap_instance_id(fragment, *instance, remap)?.0,
                suffix.as_path()
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidPathSuffix {
    segments: Vec<String>,
}

impl ValidPathSuffix {
    fn new(segments: &[&str]) -> Self {
        Self {
            segments: segments
                .iter()
                .map(|segment| (*segment).to_owned())
                .collect(),
        }
    }

    fn as_path(&self) -> String {
        self.segments.join(".")
    }
}

fn parse_geometry_scalar_path(
    fragment: &str,
    path: &str,
    segments: &[&str],
) -> Result<ValidPathSuffix, FragmentRemapError> {
    let valid = match segments {
        ["rounded_box", "half_extents", component] => is_axis3(component),
        ["rounded_box", "radius"]
        | ["cylinder", "radius"]
        | ["cylinder", "height"]
        | ["cylinder", "radial_segments"]
        | ["frustum", "bottom_radius"]
        | ["frustum", "top_radius"]
        | ["frustum", "height"]
        | ["frustum", "radial_segments"]
        | ["plate", "thickness"]
        | ["lathe", "segments"] => true,
        ["plate", "size", component] => is_axis2(component),
        ["sweep", "profile", index, component] | ["lathe", "profile", index, component] => {
            parse_index_segment(fragment, path, index)?;
            is_axis2(component)
        }
        ["sweep", "path", index, frame_field, component] => {
            parse_index_segment(fragment, path, index)?;
            is_frame_field(frame_field) && is_axis3(component)
        }
        _ => false,
    };
    if valid {
        Ok(ValidPathSuffix::new(segments))
    } else {
        Err(unknown_scalar_path(fragment, path))
    }
}

fn parse_operation_scalar_path(
    fragment: &str,
    path: &str,
    segments: &[&str],
) -> Result<ValidPathSuffix, FragmentRemapError> {
    let valid = match segments {
        ["bevel", "radius"]
        | ["bevel", "segments"]
        | ["panel", "inset"]
        | ["panel", "depth"]
        | ["trim", "width"]
        | ["trim", "height"]
        | ["recessed_panel_cut", "depth"]
        | ["recessed_panel_cut", "corner_radius"]
        | ["recessed_panel_cut", "rim_width"]
        | ["recessed_panel_cut", "corner_segments"]
        | ["rectangular_through_cut", "corner_radius"]
        | ["rectangular_through_cut", "rim_width"]
        | ["rectangular_through_cut", "corner_segments"]
        | ["circular_through_cut", "radius"]
        | ["circular_through_cut", "radial_segments"]
        | ["circular_through_cut", "rim_width"]
        | ["bevel_boundary_loop", "width"]
        | ["bevel_boundary_loop", "segments"]
        | ["bevel_boundary_loop", "profile"]
        | ["linear_array", "count"]
        | ["radial_array", "count"]
        | ["radial_array", "angle_degrees"] => true,
        ["recessed_panel_cut", "size", component]
        | ["recessed_panel_cut", "center", component]
        | ["rectangular_through_cut", "size", component]
        | ["rectangular_through_cut", "center", component]
        | ["circular_through_cut", "center", component] => is_axis2(component),
        ["linear_array", "offset", component] | ["radial_array", "axis", component] => {
            is_axis3(component)
        }
        _ => false,
    };
    if valid {
        Ok(ValidPathSuffix::new(segments))
    } else {
        Err(unknown_scalar_path(fragment, path))
    }
}

fn parse_instance_transform_path(
    fragment: &str,
    path: &str,
    segments: &[&str],
) -> Result<ValidPathSuffix, FragmentRemapError> {
    let valid = matches!(
        segments,
        ["transform", "translation", component]
            | ["transform", "rotation_degrees", component]
            | ["transform", "scale", component] if is_axis3(component)
    );
    if valid {
        Ok(ValidPathSuffix::new(segments))
    } else {
        Err(unknown_scalar_path(fragment, path))
    }
}

fn parse_numeric_segment(
    fragment: &str,
    path: &str,
    segment: &str,
) -> Result<u64, FragmentRemapError> {
    segment
        .parse::<u64>()
        .map_err(|_| malformed_scalar_path(fragment, path))
}

fn parse_index_segment(
    fragment: &str,
    path: &str,
    segment: &str,
) -> Result<usize, FragmentRemapError> {
    segment
        .parse::<usize>()
        .map_err(|_| malformed_scalar_path(fragment, path))
}

fn is_axis2(segment: &str) -> bool {
    matches!(segment, "x" | "y")
}

fn is_axis3(segment: &str) -> bool {
    matches!(segment, "x" | "y" | "z")
}

fn is_frame_field(segment: &str) -> bool {
    matches!(segment, "origin" | "x_axis" | "y_axis" | "z_axis")
}

fn malformed_scalar_path(fragment: &str, path: &str) -> FragmentRemapError {
    unsupported_variation_remap(fragment, &format!("malformed scalar path `{path}`"))
}

fn unknown_scalar_path(fragment: &str, path: &str) -> FragmentRemapError {
    unsupported_variation_remap(fragment, &format!("unknown scalar path grammar `{path}`"))
}
