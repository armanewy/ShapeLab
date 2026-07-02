
/// Editable generator dimensions and segment counts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeneratorDimensionEdit {
    /// Replace rounded-box half extents.
    RoundedBoxHalfExtents([f32; 3]),
    /// Replace rounded-box corner radius.
    RoundedBoxRadius(f32),
    /// Replace cylinder radius.
    CylinderRadius(f32),
    /// Replace cylinder height.
    CylinderHeight(f32),
    /// Replace cylinder radial segment count.
    CylinderRadialSegments(u32),
    /// Replace frustum bottom radius.
    FrustumBottomRadius(f32),
    /// Replace frustum top radius.
    FrustumTopRadius(f32),
    /// Replace frustum height.
    FrustumHeight(f32),
    /// Replace frustum radial segment count.
    FrustumRadialSegments(u32),
    /// Replace plate size.
    PlateSize([f32; 2]),
    /// Replace plate thickness.
    PlateThickness(f32),
    /// Replace lathe segment count.
    LatheSegments(u32),
}

impl GeneratorDimensionEdit {
    /// Return true when this edit can change generated topology.
    #[must_use]
    pub fn topology_changing(&self) -> bool {
        matches!(
            self,
            Self::CylinderRadialSegments(_)
                | Self::FrustumRadialSegments(_)
                | Self::LatheSegments(_)
        )
    }
}

/// Editable spacing fields on array operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArraySpacingEdit {
    /// Replace linear array offset.
    LinearOffset([f32; 3]),
    /// Replace radial array axis.
    RadialAxis([f32; 3]),
    /// Replace radial array total angle in degrees.
    RadialAngleDegrees(f32),
}

/// Plane used when mirroring an instance into a new instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MirrorInstanceSpec {
    /// Mirror plane normal.
    pub plane_normal: [f32; 3],
    /// Signed offset from the asset origin along the normal.
    pub plane_offset: f32,
}

/// Structural or scalar asset edit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AssetEdit {
    /// Set an editable scalar parameter.
    SetScalar {
        /// Target parameter.
        parameter: ParameterId,
        /// New scalar value.
        value: f32,
    },
    /// Set one scalar field on a modeling operation without a parameter descriptor.
    SetOperationScalar {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to mutate.
        operation: OperationId,
        /// Operation-relative scalar field path, such as `circular_through_cut.radius`.
        field: String,
        /// New scalar value.
        value: f32,
    },
    /// Replace an instance transform.
    SetTransform {
        /// Target instance.
        instance: PartInstanceId,
        /// New transform.
        transform: Transform3,
    },
    /// Enable or disable an instance.
    SetEnabled {
        /// Target instance.
        instance: PartInstanceId,
        /// New enabled state.
        enabled: bool,
    },
    /// Enable or disable an authored optional part instance.
    SetOptionalPartEnabled {
        /// Target optional instance.
        instance: PartInstanceId,
        /// New enabled state.
        enabled: bool,
    },
    /// Set a generator dimension or segment count.
    SetGeneratorDimension {
        /// Definition containing the generator.
        definition: PartDefinitionId,
        /// Dimension edit.
        dimension: GeneratorDimensionEdit,
    },
    /// Replace a definition's base geometry source.
    ReplaceGeometrySource {
        /// Definition containing the generator.
        definition: PartDefinitionId,
        /// Replacement source.
        source: GeometrySource,
    },
    /// Set radius and/or segment count on a bevel operation.
    SetBevelSettings {
        /// Definition containing the bevel operation.
        definition: PartDefinitionId,
        /// Bevel operation.
        operation: OperationId,
        /// Optional radius replacement.
        radius: Option<f32>,
        /// Optional segment-count replacement.
        segments: Option<u32>,
    },
    /// Replace one sweep profile point.
    SetSweepProfilePoint {
        /// Definition containing the sweep source.
        definition: PartDefinitionId,
        /// Profile point index.
        index: usize,
        /// Replacement profile point.
        point: [f32; 2],
    },
    /// Replace one sweep path frame.
    SetSweepPathFrame {
        /// Definition containing the sweep source.
        definition: PartDefinitionId,
        /// Path frame index.
        index: usize,
        /// Replacement frame.
        frame: Frame3,
    },
    /// Replace one lathe profile point.
    SetLatheProfilePoint {
        /// Definition containing the lathe source.
        definition: PartDefinitionId,
        /// Profile point index.
        index: usize,
        /// Replacement profile point.
        point: [f32; 2],
    },
    /// Add a new instance.
    AddInstance {
        /// Instance to add.
        instance: PartInstance,
    },
    /// Remove an existing instance.
    RemoveInstance {
        /// Instance to remove.
        instance: PartInstanceId,
    },
    /// Replace or insert a part definition.
    ReplaceDefinition {
        /// New definition payload.
        definition: PartDefinition,
    },
    /// Insert one modeling operation into a definition's ordered local history.
    InsertModelingOperation {
        /// Definition receiving the operation.
        definition: PartDefinitionId,
        /// Insertion index in the ordered operation list.
        index: usize,
        /// Operation payload to insert.
        operation: ModelingOperationSpec,
    },
    /// Remove one modeling operation from a definition's ordered local history.
    RemoveModelingOperation {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to remove.
        operation: OperationId,
        /// How to handle descriptors and authored hints that reference the operation.
        #[serde(default)]
        policy: OperationRemovalPolicy,
    },
    /// Duplicate an authored cut operation with fresh semantic IDs.
    DuplicateCutOperation {
        /// Definition containing the source cut.
        definition: PartDefinitionId,
        /// Source cut operation.
        source: OperationId,
        /// Stable ID for the duplicated operation.
        operation: OperationId,
        /// Entry boundary loop for the duplicate.
        entry_loop: BoundaryLoopId,
        /// Floor or exit boundary loop for the duplicate.
        secondary_loop: BoundaryLoopId,
        /// Rim region for the duplicate.
        rim_region: RegionId,
        /// Wall region for the duplicate.
        wall_region: RegionId,
        /// Floor region for duplicated recessed cuts.
        floor_region: Option<RegionId>,
        /// Offset applied to the duplicate cut center in face-local coordinates.
        center_offset: [f32; 2],
        /// How the duplicated operation joins authored semantic cut groups.
        #[serde(default)]
        group_membership: DuplicateCutGroupMembership,
        /// Dependent boundary-treatment operations to copy with explicit fresh IDs.
        #[serde(default)]
        dependent_bevels: Vec<DuplicateBoundaryBevelSpec>,
    },
    /// Move one modeling operation to a new ordered index.
    MoveModelingOperation {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Operation to move.
        operation: OperationId,
        /// Destination index after removing the operation from its old position.
        new_index: usize,
    },
    /// Replace an instance with another definition from a compatible variant group.
    ReplaceInstanceDefinition {
        /// Instance to retarget.
        instance: PartInstanceId,
        /// Replacement definition.
        definition: PartDefinitionId,
    },
    /// Set the count on a deterministic array operation.
    SetArrayCount {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Array operation.
        operation: OperationId,
        /// New count.
        count: u32,
    },
    /// Set spacing fields on a deterministic array operation.
    SetArraySpacing {
        /// Definition containing the operation.
        definition: PartDefinitionId,
        /// Array operation.
        operation: OperationId,
        /// New spacing field.
        spacing: ArraySpacingEdit,
    },
    /// Duplicate one leaf instance under the same parent.
    DuplicateInstance {
        /// Source instance.
        source: PartInstanceId,
        /// Stable ID for the duplicated instance.
        instance: PartInstanceId,
        /// Optional replacement name.
        name: Option<String>,
        /// Optional replacement transform.
        transform: Option<Transform3>,
    },
    /// Mirror one leaf instance into a new instance.
    MirrorInstance {
        /// Source instance.
        source: PartInstanceId,
        /// Stable ID for the mirrored instance.
        instance: PartInstanceId,
        /// Mirror plane.
        plane: MirrorInstanceSpec,
        /// Optional replacement name.
        name: Option<String>,
    },
    /// Attach an instance to a parent socket.
    Attach {
        /// Target child instance.
        instance: PartInstanceId,
        /// Attachment specification.
        attachment: AttachmentSpec,
    },
    /// Remove an instance attachment.
    Detach {
        /// Target instance.
        instance: PartInstanceId,
    },
    /// Change a parameter lock.
    SetLock {
        /// Target parameter.
        parameter: ParameterId,
        /// New lock state.
        locked: bool,
    },
    /// Change a part instance lock.
    SetInstanceLock {
        /// Target instance.
        instance: PartInstanceId,
        /// New lock state.
        locked: bool,
    },
    /// Change a subtree lock.
    SetSubtreeLock {
        /// Target subtree root.
        instance: PartInstanceId,
        /// New lock state.
        locked: bool,
    },
    /// Change a definition topology lock.
    SetTopologyLock {
        /// Target definition.
        definition: PartDefinitionId,
        /// New lock state.
        locked: bool,
    },
    /// Request a harmless child order change.
    ReorderChildInstances {
        /// Parent instance, or `None` for roots.
        parent: Option<PartInstanceId>,
        /// Requested child order.
        ordered_children: Vec<PartInstanceId>,
    },
}

/// Explicit policy for removing authored modeling operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OperationRemovalPolicy {
    /// Fail when parameters or variation metadata still reference the operation.
    #[default]
    RejectIfReferenced,
    /// Remove metadata owned by the operation before deleting it.
    CascadeOwnedMetadata,
    /// Remove the operation, owned metadata, and operations depending on its generated loops.
    CascadeDependentOperations,
}

/// Policy for semantic cut-group membership when duplicating a cut operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DuplicateCutGroupMembership {
    /// Add the duplicate to every semantic cut group containing the source operation.
    #[default]
    PreserveSource,
    /// Leave the duplicate outside any semantic cut group.
    Ungrouped,
    /// Add the duplicate to one explicit semantic cut group.
    AddTo(String),
}

/// Explicit remap for duplicating a boundary-loop bevel dependent on a copied cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicateBoundaryBevelSpec {
    /// Source bevel operation to copy.
    pub source: OperationId,
    /// Fresh operation ID for the copied bevel.
    pub operation: OperationId,
    /// Fresh bevel-band region for the copied bevel.
    pub bevel_region: RegionId,
    /// Fresh outer replacement loop for the copied bevel.
    pub outer_replacement_loop: BoundaryLoopId,
    /// Fresh inner replacement loop for the copied bevel.
    pub inner_replacement_loop: BoundaryLoopId,
}

/// Coarse execution phase for ordered modeling operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OperationPhase {
    /// Controls that configure the base source before local topology is generated.
    SourceConfiguration,
    /// Operations that create or remove local topology.
    LocalTopology,
    /// Operations that consume existing local boundaries or alter boundary treatment.
    BoundaryTreatment,
    /// Operations that move local generated geometry without assembly fan-out.
    LocalTransform,
    /// Operations that generate assembly occurrences such as arrays and mirrors.
    AssemblyGeneration,
}

/// Ordered edit program with a deterministic seed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEditProgram {
    /// Human-facing edit label.
    pub label: String,
    /// Deterministic seed associated with this edit program.
    pub seed: u64,
    /// Ordered edit operations.
    pub operations: Vec<AssetEdit>,
}
