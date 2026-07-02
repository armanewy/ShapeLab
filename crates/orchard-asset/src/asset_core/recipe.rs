
/// Serializable recipe for one asset and its part hierarchy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssetRecipe {
    /// Schema version for compatibility checks.
    pub schema_version: u32,
    /// Stable asset identifier.
    pub id: AssetId,
    /// Human-facing asset title.
    pub title: String,
    /// Reusable part definitions.
    pub definitions: BTreeMap<PartDefinitionId, PartDefinition>,
    /// Concrete part instances.
    pub instances: BTreeMap<PartInstanceId, PartInstance>,
    /// Root instances in stable display and compile order.
    pub root_instances: Vec<PartInstanceId>,
    /// Editable parameter descriptors.
    pub parameters: BTreeMap<ParameterId, ParameterDescriptor>,
    /// Locked parameters that mutation programs must not edit.
    pub locks: BTreeSet<ParameterId>,
    /// Locked instances that mutation programs must not edit directly.
    #[serde(default)]
    pub instance_locks: BTreeSet<PartInstanceId>,
    /// Locked instance subtrees. The root and every descendant are protected.
    #[serde(default)]
    pub subtree_locks: BTreeSet<PartInstanceId>,
    /// Locked part definition topology. Shape-preserving value edits remain valid.
    #[serde(default)]
    pub topology_locks: BTreeSet<PartDefinitionId>,
    /// Asset-level constraints.
    pub constraints: Vec<AssetConstraint>,
    /// Authored geometric relationship policies between part instances.
    #[serde(default)]
    pub relationships: Vec<AssetRelationshipPolicy>,
    /// Authored variation hints that do not affect hierarchy or generation semantics.
    #[serde(default)]
    pub variation: AuthoredVariationMetadata,
    /// v8 semantic shells reserved for the canonical Orchard asset lane.
    #[serde(default)]
    pub semantic: AssetRecipeSemanticShells,
    /// Next semantic ID counters.
    pub next_ids: AssetIdCounters,
}

#[derive(Deserialize)]
struct AssetRecipeWire {
    schema_version: u32,
    id: AssetId,
    title: String,
    definitions: BTreeMap<PartDefinitionId, PartDefinitionWire>,
    instances: BTreeMap<PartInstanceId, PartInstance>,
    root_instances: Vec<PartInstanceId>,
    parameters: BTreeMap<ParameterId, ParameterDescriptor>,
    locks: BTreeSet<ParameterId>,
    #[serde(default)]
    instance_locks: BTreeSet<PartInstanceId>,
    #[serde(default)]
    subtree_locks: BTreeSet<PartInstanceId>,
    #[serde(default)]
    topology_locks: BTreeSet<PartDefinitionId>,
    constraints: Vec<AssetConstraint>,
    #[serde(default)]
    relationships: Vec<AssetRelationshipPolicy>,
    #[serde(default)]
    variation: AuthoredVariationMetadata,
    #[serde(default)]
    semantic: AssetRecipeSemanticShells,
    next_ids: AssetIdCounters,
}

#[derive(Deserialize)]
struct PartDefinitionWire {
    id: PartDefinitionId,
    name: String,
    tags: BTreeSet<String>,
    geometry: GeometryRecipeWire,
    regions: BTreeMap<RegionId, SurfaceRegionSpec>,
    sockets: BTreeMap<SocketId, SocketSpec>,
    local_pivot: Frame3,
    variant_group: Option<String>,
    production_hints: Option<ProductionHints>,
}

#[derive(Deserialize)]
struct GeometryRecipeWire {
    source: GeometrySource,
    operations: Vec<ModelingOperationSpecWire>,
}

impl AssetRecipeWire {
    fn into_recipe(self) -> Result<AssetRecipe, String> {
        let schema_version = self.schema_version;
        let mut definitions = BTreeMap::new();
        for (id, definition) in self.definitions {
            definitions.insert(id, definition.into_definition(schema_version)?);
        }
        let mut recipe = AssetRecipe {
            schema_version: migrated_asset_recipe_schema_version(schema_version),
            id: self.id,
            title: self.title,
            definitions,
            instances: self.instances,
            root_instances: self.root_instances,
            parameters: self.parameters,
            locks: self.locks,
            instance_locks: self.instance_locks,
            subtree_locks: self.subtree_locks,
            topology_locks: self.topology_locks,
            constraints: self.constraints,
            relationships: self.relationships,
            variation: self.variation,
            semantic: self.semantic,
            next_ids: self.next_ids,
        };
        if schema_version < 4 {
            migrate_legacy_cut_boundary_loops(&mut recipe);
        }
        Ok(recipe)
    }
}

impl PartDefinitionWire {
    fn into_definition(self, schema_version: u32) -> Result<PartDefinition, String> {
        let mut operations = Vec::with_capacity(self.geometry.operations.len());
        for operation in self.geometry.operations {
            operations.push(operation.into_operation(schema_version)?);
        }
        Ok(PartDefinition {
            id: self.id,
            name: self.name,
            tags: self.tags,
            geometry: GeometryRecipe {
                source: self.geometry.source,
                operations,
            },
            regions: self.regions,
            sockets: self.sockets,
            local_pivot: self.local_pivot,
            variant_group: self.variant_group,
            production_hints: self.production_hints,
        })
    }
}

impl<'de> Deserialize<'de> for AssetRecipe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        AssetRecipeWire::deserialize(deserializer)?
            .into_recipe()
            .map_err(de::Error::custom)
    }
}

fn migrated_asset_recipe_schema_version(schema_version: u32) -> u32 {
    if matches!(schema_version, 1..=7) {
        ASSET_RECIPE_SCHEMA_VERSION
    } else {
        schema_version
    }
}

fn migrate_legacy_cut_boundary_loops(recipe: &mut AssetRecipe) {
    let mut used = recipe
        .definitions
        .values()
        .flat_map(|definition| definition.geometry.operations.iter())
        .flat_map(ModelingOperationSpec::boundary_loop_ids)
        .filter_map(|id| (id != LEGACY_MISSING_BOUNDARY_LOOP).then_some(id.0))
        .collect::<BTreeSet<_>>();
    let mut next = recipe
        .next_ids
        .boundary_loop
        .max(used.last().copied().unwrap_or_default().saturating_add(1))
        .max(1);

    for definition in recipe.definitions.values_mut() {
        for operation in &mut definition.geometry.operations {
            match operation {
                ModelingOperationSpec::RecessedPanelCut {
                    entry_loop,
                    floor_loop,
                    ..
                } => {
                    if *entry_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *entry_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                    if *floor_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *floor_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                }
                ModelingOperationSpec::RectangularThroughCut {
                    entry_loop,
                    exit_loop,
                    ..
                }
                | ModelingOperationSpec::CircularThroughCut {
                    entry_loop,
                    exit_loop,
                    ..
                } => {
                    if *entry_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *entry_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                    if *exit_loop == LEGACY_MISSING_BOUNDARY_LOOP {
                        *exit_loop = allocate_migrated_boundary_loop(&mut used, &mut next);
                    }
                }
                ModelingOperationSpec::TransformGeometry { .. }
                | ModelingOperationSpec::SetBevelProfile { .. }
                | ModelingOperationSpec::AddPanel { .. }
                | ModelingOperationSpec::AddTrim { .. }
                | ModelingOperationSpec::BevelBoundaryLoop { .. }
                | ModelingOperationSpec::MirrorInstances { .. }
                | ModelingOperationSpec::LinearArray { .. }
                | ModelingOperationSpec::RadialArray { .. }
                | ModelingOperationSpec::ReservedBoolean { .. }
                | ModelingOperationSpec::ReservedDeformationProgram { .. } => {}
            }
        }
    }

    recipe.next_ids.boundary_loop = recipe.next_ids.boundary_loop.max(next);
}

fn allocate_migrated_boundary_loop(used: &mut BTreeSet<u64>, next: &mut u64) -> BoundaryLoopId {
    while *next == 0 || used.contains(&*next) {
        *next = next.saturating_add(1);
    }
    let id = *next;
    used.insert(id);
    *next = next.saturating_add(1);
    BoundaryLoopId(id)
}

impl AssetRecipe {
    /// Create an empty recipe with deterministic ID counters.
    #[must_use]
    pub fn new(id: AssetId, title: impl Into<String>) -> Self {
        Self {
            schema_version: ASSET_RECIPE_SCHEMA_VERSION,
            id,
            title: title.into(),
            definitions: BTreeMap::new(),
            instances: BTreeMap::new(),
            root_instances: Vec::new(),
            parameters: BTreeMap::new(),
            locks: BTreeSet::new(),
            instance_locks: BTreeSet::new(),
            subtree_locks: BTreeSet::new(),
            topology_locks: BTreeSet::new(),
            constraints: Vec::new(),
            relationships: Vec::new(),
            variation: AuthoredVariationMetadata::default(),
            semantic: AssetRecipeSemanticShells::default(),
            next_ids: AssetIdCounters::default(),
        }
    }

    /// Allocate the next part definition ID.
    pub fn allocate_part_definition_id(&mut self) -> PartDefinitionId {
        allocate_part_definition_id(self)
    }

    /// Allocate the next part instance ID.
    pub fn allocate_part_instance_id(&mut self) -> PartInstanceId {
        allocate_part_instance_id(self)
    }

    /// Allocate the next operation ID.
    pub fn allocate_operation_id(&mut self) -> OperationId {
        allocate_operation_id(self)
    }

    /// Allocate the next region ID.
    pub fn allocate_region_id(&mut self) -> RegionId {
        allocate_region_id(self)
    }

    /// Allocate the next boundary loop ID.
    pub fn allocate_boundary_loop_id(&mut self) -> BoundaryLoopId {
        allocate_boundary_loop_id(self)
    }

    /// Allocate the next socket ID.
    pub fn allocate_socket_id(&mut self) -> SocketId {
        allocate_socket_id(self)
    }

    /// Allocate the next parameter ID.
    pub fn allocate_parameter_id(&mut self) -> ParameterId {
        allocate_parameter_id(self)
    }

    /// Allocate the next revision ID.
    pub fn allocate_revision_id(&mut self) -> RevisionId {
        allocate_revision_id(self)
    }

    /// Allocate the next relationship ID.
    pub fn allocate_relationship_id(&mut self) -> RelationshipId {
        let id = RelationshipId(self.next_ids.relationship);
        self.next_ids.relationship = self.next_ids.relationship.saturating_add(1);
        id
    }
}

/// Next-ID counters stored in an asset recipe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetIdCounters {
    /// Next part definition ID.
    pub part_definition: u64,
    /// Next part instance ID.
    pub part_instance: u64,
    /// Next operation ID.
    pub operation: u64,
    /// Next region ID.
    pub region: u64,
    /// Next boundary loop ID.
    #[serde(default = "default_id_counter")]
    pub boundary_loop: u64,
    /// Next socket ID.
    pub socket: u64,
    /// Next parameter ID.
    pub parameter: u64,
    /// Next revision ID.
    pub revision: u64,
    /// Next relationship contract ID.
    #[serde(default = "default_id_counter")]
    pub relationship: u64,
    /// Next pattern contract ID.
    #[serde(default = "default_id_counter")]
    pub pattern: u64,
    /// Next surface slot ID.
    #[serde(default = "default_id_counter")]
    pub surface_slot: u64,
    /// Next material slot ID.
    #[serde(default = "default_id_counter")]
    pub material_slot: u64,
    /// Next collision body ID.
    #[serde(default = "default_id_counter")]
    pub collision_body: u64,
    /// Next motion channel ID.
    #[serde(default = "default_id_counter")]
    pub motion_channel: u64,
    /// Next terrain patch ID.
    #[serde(default = "default_id_counter")]
    pub terrain_patch: u64,
    /// Next export profile ID.
    #[serde(default = "default_id_counter")]
    pub export_profile: u64,
    /// Next authoring operation ID.
    #[serde(default = "default_id_counter")]
    pub authoring_op: u64,
    /// Next validation report ID.
    #[serde(default = "default_id_counter")]
    pub validation_report: u64,
}

impl Default for AssetIdCounters {
    fn default() -> Self {
        Self {
            part_definition: 1,
            part_instance: 1,
            operation: 1,
            region: 1,
            boundary_loop: 1,
            socket: 1,
            parameter: 1,
            revision: 1,
            relationship: 1,
            pattern: 1,
            surface_slot: 1,
            material_slot: 1,
            collision_body: 1,
            motion_channel: 1,
            terrain_patch: 1,
            export_profile: 1,
            authoring_op: 1,
            validation_report: 1,
        }
    }
}

fn default_id_counter() -> u64 {
    1
}
