
macro_rules! id_type {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug,
            Copy,
            Clone,
            Default,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            Hash,
            Serialize,
            Deserialize,
        )]
        pub struct $name(pub u64);
    };
}

id_type!(AssetId, "Stable identifier for an asset recipe.");
id_type!(
    PartDefinitionId,
    "Stable semantic identifier for a reusable part definition."
);
id_type!(
    PartInstanceId,
    "Stable semantic identifier for a concrete part instance."
);
id_type!(
    OperationId,
    "Stable semantic identifier for a modeling operation."
);
id_type!(RegionId, "Stable semantic identifier for a surface region.");
id_type!(
    BoundaryLoopId,
    "Stable semantic identifier for a generated boundary loop."
);
const LEGACY_MISSING_BOUNDARY_LOOP: BoundaryLoopId = BoundaryLoopId(0);
const DEFAULT_RECT_CUT_CORNER_SEGMENTS: u32 = 4;
id_type!(
    SocketId,
    "Stable semantic identifier for an attachment socket."
);
id_type!(
    ParameterId,
    "Stable semantic identifier for an editable parameter."
);
id_type!(
    RevisionId,
    "Stable identifier for an asset recipe revision."
);
id_type!(
    RelationshipId,
    "Stable semantic identifier for an authored relationship contract."
);
id_type!(
    PatternId,
    "Stable semantic identifier for an authored pattern contract."
);
id_type!(
    SurfaceSlotId,
    "Stable semantic identifier for a future surface slot."
);
id_type!(
    MaterialSlotId,
    "Stable semantic identifier for a future material slot."
);
id_type!(
    CollisionBodyId,
    "Stable semantic identifier for a future collision body."
);
id_type!(
    MotionChannelId,
    "Stable semantic identifier for a future motion channel."
);
id_type!(
    TerrainPatchId,
    "Stable semantic identifier for a future terrain patch."
);
id_type!(
    ExportProfileId,
    "Stable semantic identifier for an export profile shell."
);
id_type!(
    AuthoringOpId,
    "Stable semantic identifier for an authoring operation shell."
);
id_type!(
    ValidationReportId,
    "Stable semantic identifier for a validation report shell."
);
