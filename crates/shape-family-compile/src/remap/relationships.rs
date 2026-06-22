//! Relationship and attachment-policy remap boundary.

use std::collections::BTreeSet;

use shape_asset::{
    AssetConstraint, AssetPartSelector, AssetRecipe, AssetRelationshipPolicy, OperationId,
    ParameterId, PartDefinitionId, PartInstanceId, RelationshipPairing, SocketId,
};

use super::{FragmentRemap, FragmentRemapError};

/// Validate that relationship remapping is intentionally routed through this module.
pub fn unsupported_relationship_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "relationships".to_owned(),
        reason: reason.to_owned(),
    }
}

/// Remapped relationship metadata and lock sets for one recipe fragment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemappedFragmentRelationships {
    /// Remapped asset-level constraints.
    pub constraints: Vec<AssetConstraint>,
    /// Remapped authored geometric relationships.
    pub relationships: Vec<AssetRelationshipPolicy>,
    /// Remapped scalar parameter locks.
    pub parameter_locks: BTreeSet<ParameterId>,
    /// Remapped direct instance locks.
    pub instance_locks: BTreeSet<PartInstanceId>,
    /// Remapped subtree locks.
    pub subtree_locks: BTreeSet<PartInstanceId>,
    /// Remapped topology locks.
    pub topology_locks: BTreeSet<PartDefinitionId>,
}

/// Remap all authored relationship metadata from one source fragment recipe.
pub fn remap_fragment_relationships(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<RemappedFragmentRelationships, FragmentRemapError> {
    Ok(RemappedFragmentRelationships {
        constraints: remap_constraints(fragment, source, remap)?,
        relationships: remap_relationship_policies(fragment, source, remap)?,
        parameter_locks: remap_parameter_locks(fragment, source, remap)?,
        instance_locks: remap_instance_locks(fragment, source, remap)?,
        subtree_locks: remap_subtree_locks(fragment, source, remap)?,
        topology_locks: remap_topology_locks(fragment, source, remap)?,
    })
}

/// Remap relationship metadata and append it to a target recipe.
pub fn append_remapped_fragment_relationships(
    target: &mut AssetRecipe,
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<RemappedFragmentRelationships, FragmentRemapError> {
    let remapped = remap_fragment_relationships(fragment, source, remap)?;

    target
        .constraints
        .extend(remapped.constraints.iter().cloned());
    target
        .relationships
        .extend(remapped.relationships.iter().cloned());
    target
        .locks
        .extend(remapped.parameter_locks.iter().copied());
    target
        .instance_locks
        .extend(remapped.instance_locks.iter().copied());
    target
        .subtree_locks
        .extend(remapped.subtree_locks.iter().copied());
    target
        .topology_locks
        .extend(remapped.topology_locks.iter().copied());

    Ok(remapped)
}

/// Remap asset-level constraints.
pub fn remap_constraints(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<Vec<AssetConstraint>, FragmentRemapError> {
    source
        .constraints
        .iter()
        .map(|constraint| remap_asset_constraint(fragment, source, remap, constraint))
        .collect()
}

/// Remap one asset-level constraint.
pub fn remap_asset_constraint(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    constraint: &AssetConstraint,
) -> Result<AssetConstraint, FragmentRemapError> {
    Ok(match constraint {
        AssetConstraint::RequireInstance { instance } => AssetConstraint::RequireInstance {
            instance: remap_instance(fragment, source, remap, *instance)?,
        },
        AssetConstraint::MutuallyExclusiveTags { first, second } => {
            AssetConstraint::MutuallyExclusiveTags {
                first: first.clone(),
                second: second.clone(),
            }
        }
        AssetConstraint::Custom { code, message } => AssetConstraint::Custom {
            code: code.clone(),
            message: message.clone(),
        },
    })
}

/// Remap authored geometric relationship policies.
pub fn remap_relationship_policies(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<Vec<AssetRelationshipPolicy>, FragmentRemapError> {
    source
        .relationships
        .iter()
        .map(|policy| remap_asset_relationship_policy(fragment, source, remap, policy))
        .collect()
}

/// Remap one authored geometric relationship policy.
pub fn remap_asset_relationship_policy(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    policy: &AssetRelationshipPolicy,
) -> Result<AssetRelationshipPolicy, FragmentRemapError> {
    Ok(match policy {
        AssetRelationshipPolicy::MayOverlap {
            first,
            second,
            pairing,
            reason,
        } => AssetRelationshipPolicy::MayOverlap {
            first: remap_part_selector(fragment, source, remap, first)?,
            second: remap_part_selector(fragment, source, remap, second)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
            reason: reason.clone(),
        },
        AssetRelationshipPolicy::MustNotIntersect {
            first,
            second,
            pairing,
        } => AssetRelationshipPolicy::MustNotIntersect {
            first: remap_part_selector(fragment, source, remap, first)?,
            second: remap_part_selector(fragment, source, remap, second)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
        },
        AssetRelationshipPolicy::MustTouch {
            first,
            second,
            pairing,
            max_clearance,
        } => AssetRelationshipPolicy::MustTouch {
            first: remap_part_selector(fragment, source, remap, first)?,
            second: remap_part_selector(fragment, source, remap, second)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
            max_clearance: *max_clearance,
        },
        AssetRelationshipPolicy::MustContain {
            container,
            contained,
            pairing,
        } => AssetRelationshipPolicy::MustContain {
            container: remap_part_selector(fragment, source, remap, container)?,
            contained: remap_part_selector(fragment, source, remap, contained)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
        },
        AssetRelationshipPolicy::MinimumClearance {
            first,
            second,
            pairing,
            clearance,
        } => AssetRelationshipPolicy::MinimumClearance {
            first: remap_part_selector(fragment, source, remap, first)?,
            second: remap_part_selector(fragment, source, remap, second)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
            clearance: *clearance,
        },
        AssetRelationshipPolicy::SocketAttached {
            parent,
            child,
            pairing,
            parent_socket,
            child_socket,
            max_origin_distance,
            max_axis_angle_degrees,
            max_clearance,
        } => AssetRelationshipPolicy::SocketAttached {
            parent: remap_part_selector(fragment, source, remap, parent)?,
            child: remap_part_selector(fragment, source, remap, child)?,
            pairing: remap_relationship_pairing(fragment, source, remap, pairing)?,
            parent_socket: remap_socket(fragment, source, remap, *parent_socket)?,
            child_socket: remap_socket(fragment, source, remap, *child_socket)?,
            max_origin_distance: *max_origin_distance,
            max_axis_angle_degrees: *max_axis_angle_degrees,
            max_clearance: *max_clearance,
        },
    })
}

/// Remap a relationship endpoint selector.
pub fn remap_part_selector(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    selector: &AssetPartSelector,
) -> Result<AssetPartSelector, FragmentRemapError> {
    Ok(match selector {
        AssetPartSelector::SpecificInstance { instance } => AssetPartSelector::SpecificInstance {
            instance: remap_instance(fragment, source, remap, *instance)?,
        },
        AssetPartSelector::GeneratedByOperation { operation } => {
            AssetPartSelector::GeneratedByOperation {
                operation: remap_operation(fragment, source, remap, *operation)?,
            }
        }
        AssetPartSelector::PrototypeAndGeneratedOccurrences { prototype } => {
            AssetPartSelector::PrototypeAndGeneratedOccurrences {
                prototype: remap_instance(fragment, source, remap, *prototype)?,
            }
        }
        AssetPartSelector::PartTag { tag } => AssetPartSelector::PartTag { tag: tag.clone() },
        AssetPartSelector::DefinitionRole { role } => {
            AssetPartSelector::DefinitionRole { role: role.clone() }
        }
    })
}

/// Remap relationship pairing metadata.
pub fn remap_relationship_pairing(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    pairing: &RelationshipPairing,
) -> Result<RelationshipPairing, FragmentRemapError> {
    Ok(match pairing {
        RelationshipPairing::AllPairs => RelationshipPairing::AllPairs,
        RelationshipPairing::ByOccurrenceIndex => RelationshipPairing::ByOccurrenceIndex,
        RelationshipPairing::ByPrototypeLineage => RelationshipPairing::ByPrototypeLineage,
        RelationshipPairing::NearestOneToOne => RelationshipPairing::NearestOneToOne,
        RelationshipPairing::Explicit(pairs) => RelationshipPairing::Explicit(
            pairs
                .iter()
                .map(|(first, second)| {
                    Ok((
                        remap_instance(fragment, source, remap, *first)?,
                        remap_instance(fragment, source, remap, *second)?,
                    ))
                })
                .collect::<Result<Vec<_>, FragmentRemapError>>()?,
        ),
    })
}

/// Remap scalar parameter locks.
pub fn remap_parameter_locks(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<BTreeSet<ParameterId>, FragmentRemapError> {
    source
        .locks
        .iter()
        .map(|parameter| remap_parameter(fragment, source, remap, *parameter))
        .collect()
}

/// Remap direct instance locks.
pub fn remap_instance_locks(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartInstanceId>, FragmentRemapError> {
    source
        .instance_locks
        .iter()
        .map(|instance| remap_instance(fragment, source, remap, *instance))
        .collect()
}

/// Remap subtree locks.
pub fn remap_subtree_locks(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartInstanceId>, FragmentRemapError> {
    source
        .subtree_locks
        .iter()
        .map(|instance| remap_instance(fragment, source, remap, *instance))
        .collect()
}

/// Remap topology locks.
pub fn remap_topology_locks(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
) -> Result<BTreeSet<PartDefinitionId>, FragmentRemapError> {
    source
        .topology_locks
        .iter()
        .map(|definition| remap_definition(fragment, source, remap, *definition))
        .collect()
}

fn remap_parameter(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    parameter: ParameterId,
) -> Result<ParameterId, FragmentRemapError> {
    remap_typed_id(
        fragment,
        "parameter",
        parameter.0.to_string(),
        source.parameters.contains_key(&parameter),
        remap.parameters.get(&parameter).copied(),
    )
}

fn remap_instance(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    instance: PartInstanceId,
) -> Result<PartInstanceId, FragmentRemapError> {
    remap_typed_id(
        fragment,
        "part instance",
        instance.0.to_string(),
        source.instances.contains_key(&instance),
        remap.instances.get(&instance).copied(),
    )
}

fn remap_definition(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    definition: PartDefinitionId,
) -> Result<PartDefinitionId, FragmentRemapError> {
    remap_typed_id(
        fragment,
        "part definition",
        definition.0.to_string(),
        source.definitions.contains_key(&definition),
        remap.definitions.get(&definition).copied(),
    )
}

fn remap_operation(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    operation: OperationId,
) -> Result<OperationId, FragmentRemapError> {
    let exists = source
        .definitions
        .values()
        .flat_map(|definition| &definition.geometry.operations)
        .any(|candidate| candidate.operation_id() == operation);
    remap_typed_id(
        fragment,
        "operation",
        operation.0.to_string(),
        exists,
        remap.operations.get(&operation).copied(),
    )
}

fn remap_socket(
    fragment: &str,
    source: &AssetRecipe,
    remap: &FragmentRemap,
    socket: SocketId,
) -> Result<SocketId, FragmentRemapError> {
    let exists = source
        .definitions
        .values()
        .any(|definition| definition.sockets.contains_key(&socket));
    remap_typed_id(
        fragment,
        "socket",
        socket.0.to_string(),
        exists,
        remap.sockets.get(&socket).copied(),
    )
}

fn remap_typed_id<T>(
    fragment: &str,
    id_kind: &str,
    id: String,
    exists_in_fragment: bool,
    remapped: Option<T>,
) -> Result<T, FragmentRemapError> {
    if !exists_in_fragment {
        return Err(FragmentRemapError::ExternalReference {
            fragment: fragment.to_owned(),
            id_kind: id_kind.to_owned(),
            id,
        });
    }
    remapped.ok_or_else(|| FragmentRemapError::MissingMapping {
        fragment: fragment.to_owned(),
        id_kind: id_kind.to_owned(),
        id,
    })
}
