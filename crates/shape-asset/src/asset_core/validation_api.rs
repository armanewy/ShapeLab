
/// Validation issue emitted for asset recipes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetValidationIssue {
    /// Optional stable subject path.
    pub subject: Option<String>,
    /// Stable issue code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

/// Validation report for an asset recipe.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssetValidationReport {
    /// Discovered issues.
    pub issues: Vec<AssetValidationIssue>,
}

impl AssetValidationReport {
    /// Return true when the recipe is valid.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }
}

/// Error type for asset recipe helpers.
#[derive(Debug, Error)]
pub enum AssetError {
    /// The requested part definition does not exist.
    #[error("unknown part definition {0:?}")]
    UnknownDefinition(PartDefinitionId),
    /// The requested part instance does not exist.
    #[error("unknown part instance {0:?}")]
    UnknownInstance(PartInstanceId),
    /// The requested parameter does not exist.
    #[error("unknown parameter {0:?}")]
    UnknownParameter(ParameterId),
    /// The requested scalar path does not exist.
    #[error("unknown scalar path {0}")]
    UnknownScalarPath(String),
    /// A non-finite scalar was supplied.
    #[error("non-finite scalar for {path}: {value}")]
    NonFiniteScalar {
        /// Target path.
        path: String,
        /// Supplied value.
        value: f32,
    },
    /// The scalar cannot be represented by the target field.
    #[error("invalid scalar for {path}: {value} ({reason})")]
    InvalidScalarValue {
        /// Target path.
        path: String,
        /// Supplied value.
        value: f32,
        /// Reason the value cannot be applied.
        reason: &'static str,
    },
    /// The edit attempted to mutate a locked parameter.
    #[error("parameter is locked {0:?}")]
    LockedParameter(ParameterId),
    /// The edit attempted to mutate a locked part instance.
    #[error("part instance is locked {0:?}")]
    LockedInstance(PartInstanceId),
    /// The edit attempted to mutate a part instance inside a locked subtree.
    #[error("part instance {instance:?} is inside locked subtree {root:?}")]
    LockedSubtree {
        /// Locked subtree root.
        root: PartInstanceId,
        /// Mutated instance.
        instance: PartInstanceId,
    },
    /// The edit attempted to change locked topology.
    #[error("topology is locked for definition {0:?}")]
    LockedTopology(PartDefinitionId),
    /// The replacement definition is outside the compatible variant group.
    #[error("incompatible replacement from {from:?} to {to:?}")]
    IncompatibleReplacement {
        /// Existing definition.
        from: PartDefinitionId,
        /// Requested replacement definition.
        to: PartDefinitionId,
    },
    /// An edit cannot be applied to the target.
    #[error("unsupported edit: {0}")]
    UnsupportedEdit(String),
    /// The edited recipe failed validation.
    #[error("asset recipe validation failed")]
    ValidationFailed(AssetValidationReport),
}

/// Build a canonical scalar path for a part definition.
#[must_use]
pub fn definition_scalar_path(definition: PartDefinitionId, key: impl AsRef<str>) -> String {
    format!("definition.{}.{}", definition.0, key.as_ref())
}

/// Build a canonical scalar path for a part instance.
#[must_use]
pub fn instance_scalar_path(instance: PartInstanceId, key: impl AsRef<str>) -> String {
    format!("instance.{}.{}", instance.0, key.as_ref())
}

/// Validate an asset recipe and collect all discoverable issues.
#[must_use]
pub fn validate_asset_recipe(recipe: &AssetRecipe) -> AssetValidationReport {
    let mut report = AssetValidationReport::default();

    if recipe.schema_version != ASSET_RECIPE_SCHEMA_VERSION {
        push_issue(
            &mut report,
            None,
            "unsupported_schema_version",
            "Asset recipe schema version is not supported.",
        );
    }
    if recipe.title.trim().is_empty() {
        push_issue(
            &mut report,
            None,
            "empty_title",
            "Asset recipe title cannot be empty.",
        );
    }

    validate_definitions(recipe, &mut report);
    validate_instances(recipe, &mut report);
    validate_parameters(recipe, &mut report);
    validate_locks(recipe, &mut report);
    validate_constraints(recipe, &mut report);
    validate_relationships(recipe, &mut report);
    validate_variation_metadata(recipe, &mut report);
    validate_semantic_shells(recipe, &mut report);
    validate_next_ids(recipe, &mut report);

    report
}

/// Return editable parameters in deterministic order.
#[must_use]
pub fn enumerate_parameters(recipe: &AssetRecipe) -> Vec<ParameterDescriptor> {
    recipe
        .parameters
        .iter()
        .filter(|entry| {
            let (id, parameter) = *entry;
            parameter_is_reflectable(recipe, *id, parameter)
        })
        .map(|(_, parameter)| parameter.clone())
        .collect()
}

/// Read a scalar parameter by canonical path.
pub fn get_scalar(recipe: &AssetRecipe, path: impl AsRef<str>) -> Result<f32, AssetError> {
    let path = path.as_ref();
    let parts = path.split('.').collect::<Vec<_>>();
    match parts.as_slice() {
        ["definition", id, rest @ ..] => {
            let definition_id = parse_id(id, path).map(PartDefinitionId)?;
            let definition = recipe
                .definitions
                .get(&definition_id)
                .ok_or(AssetError::UnknownDefinition(definition_id))?;
            get_definition_scalar(definition, rest, path)
        }
        ["instance", id, rest @ ..] => {
            let instance_id = parse_id(id, path).map(PartInstanceId)?;
            let instance = recipe
                .instances
                .get(&instance_id)
                .ok_or(AssetError::UnknownInstance(instance_id))?;
            get_transform_scalar(&instance.local_transform, rest, path)
        }
        _ => Err(AssetError::UnknownScalarPath(path.to_owned())),
    }
}

/// Set a scalar parameter by canonical path.
pub fn set_scalar(
    recipe: &mut AssetRecipe,
    path: impl AsRef<str>,
    value: f32,
) -> Result<(), AssetError> {
    let path = path.as_ref();
    if !value.is_finite() {
        return Err(AssetError::NonFiniteScalar {
            path: path.to_owned(),
            value,
        });
    }

    let mut clone = recipe.clone();
    set_scalar_in_place(&mut clone, path, value)?;
    *recipe = clone;
    Ok(())
}

/// Apply an edit program atomically.
pub fn apply_edit_program(
    recipe: &AssetRecipe,
    program: &AssetEditProgram,
) -> Result<AssetRecipe, AssetError> {
    let mut clone = recipe.clone();
    for operation in &program.operations {
        apply_edit(&mut clone, operation)?;
    }
    let report = validate_asset_recipe(&clone);
    if report.is_valid() {
        Ok(clone)
    } else {
        Err(AssetError::ValidationFailed(report))
    }
}

/// Allocate the next part definition ID.
pub fn allocate_part_definition_id(recipe: &mut AssetRecipe) -> PartDefinitionId {
    let id = PartDefinitionId(recipe.next_ids.part_definition);
    recipe.next_ids.part_definition = recipe.next_ids.part_definition.saturating_add(1);
    id
}

/// Allocate the next part instance ID.
pub fn allocate_part_instance_id(recipe: &mut AssetRecipe) -> PartInstanceId {
    let id = PartInstanceId(recipe.next_ids.part_instance);
    recipe.next_ids.part_instance = recipe.next_ids.part_instance.saturating_add(1);
    id
}

/// Allocate the next operation ID.
pub fn allocate_operation_id(recipe: &mut AssetRecipe) -> OperationId {
    let id = OperationId(recipe.next_ids.operation);
    recipe.next_ids.operation = recipe.next_ids.operation.saturating_add(1);
    id
}

/// Allocate the next region ID.
pub fn allocate_region_id(recipe: &mut AssetRecipe) -> RegionId {
    let id = RegionId(recipe.next_ids.region);
    recipe.next_ids.region = recipe.next_ids.region.saturating_add(1);
    id
}

/// Allocate the next boundary loop ID.
pub fn allocate_boundary_loop_id(recipe: &mut AssetRecipe) -> BoundaryLoopId {
    let id = BoundaryLoopId(recipe.next_ids.boundary_loop);
    recipe.next_ids.boundary_loop = recipe.next_ids.boundary_loop.saturating_add(1);
    id
}

/// Allocate the next socket ID.
pub fn allocate_socket_id(recipe: &mut AssetRecipe) -> SocketId {
    let id = SocketId(recipe.next_ids.socket);
    recipe.next_ids.socket = recipe.next_ids.socket.saturating_add(1);
    id
}

/// Allocate the next parameter ID.
pub fn allocate_parameter_id(recipe: &mut AssetRecipe) -> ParameterId {
    let id = ParameterId(recipe.next_ids.parameter);
    recipe.next_ids.parameter = recipe.next_ids.parameter.saturating_add(1);
    id
}

/// Allocate the next revision ID.
pub fn allocate_revision_id(recipe: &mut AssetRecipe) -> RevisionId {
    let id = RevisionId(recipe.next_ids.revision);
    recipe.next_ids.revision = recipe.next_ids.revision.saturating_add(1);
    id
}

/// Return descendants of an instance in stable order.
pub fn descendants_of(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
) -> Result<Vec<PartInstanceId>, AssetError> {
    if !recipe.instances.contains_key(&instance) {
        return Err(AssetError::UnknownInstance(instance));
    }
    let mut result = BTreeSet::new();
    collect_descendants(recipe, instance, &mut result);
    result.remove(&instance);
    Ok(result.into_iter().collect())
}

/// Return instances that reference a definition in stable order.
#[must_use]
pub fn instances_of_definition(
    recipe: &AssetRecipe,
    definition: PartDefinitionId,
) -> Vec<PartInstanceId> {
    recipe
        .instances
        .values()
        .filter(|instance| instance.definition == definition)
        .map(|instance| instance.id)
        .collect()
}
