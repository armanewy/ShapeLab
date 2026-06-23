//! Deterministic contracts for topology-construction operations.
//!
//! These contracts describe the semantic topology behavior expected from the
//! canonical evaluator. They do not execute mesh edits.

use serde::{Deserialize, Serialize};

use crate::ModelingOperationKind;

/// Operator families covered by the topology-construction contract registry.
pub const TOPOLOGY_OPERATOR_KINDS: [ModelingOperationKind; 16] = [
    ModelingOperationKind::PrimitiveCreate,
    ModelingOperationKind::RegionExtrude,
    ModelingOperationKind::RegionInset,
    ModelingOperationKind::LoopCut,
    ModelingOperationKind::BridgeLoops,
    ModelingOperationKind::Merge,
    ModelingOperationKind::Split,
    ModelingOperationKind::Dissolve,
    ModelingOperationKind::Separate,
    ModelingOperationKind::Join,
    ModelingOperationKind::Mirror,
    ModelingOperationKind::Array,
    ModelingOperationKind::Subdivide,
    ModelingOperationKind::Bevel,
    ModelingOperationKind::ShellSolidify,
    ModelingOperationKind::ConstrainedBoolean,
];

/// Complete deterministic contract for one topology construction operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopologyOperatorContract {
    /// Operation kind in the shared modeling-program vocabulary.
    pub kind: ModelingOperationKind,
    /// Declared topology effect.
    pub topology_effect: TopologyEffect,
    /// Stable provenance behavior for retained and generated topology.
    pub stable_provenance: StableProvenanceBehavior,
    /// Required semantic selections.
    pub selection_requirements: SelectionRequirements,
    /// Exact replay behavior expected from the canonical evaluator.
    pub exact_replay: ExactReplayBehavior,
    /// Number of semantic parameters required by the compact operator.
    pub semantic_parameter_count: usize,
    /// Hints an inverse solver may use to infer this operation.
    pub inference_hints: Vec<InferenceHint>,
}

/// Topological edit class for an operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyEffect {
    /// Creates a new primitive topology root.
    CreatePrimitive,
    /// Creates an offset copy of a region plus side-wall topology.
    ExtrudeRegion,
    /// Inserts an inner boundary and a new center region.
    InsetRegion,
    /// Inserts one or more edge loops through an existing strip.
    InsertLoopCut,
    /// Connects compatible boundary loops with new faces.
    BridgeBoundaryLoops,
    /// Coalesces vertices, edges, faces, regions, or parts.
    MergeElements,
    /// Splits a region, edge, vertex, or part into separate topology elements.
    SplitElements,
    /// Removes selected topology while preserving the declared surrounding surface.
    DissolveElements,
    /// Moves selected topology into a separate semantic part or object.
    SeparatePart,
    /// Combines parts or objects into one semantic topology root.
    JoinParts,
    /// Duplicates topology through a reflection transform and optional weld plane.
    MirrorTopology,
    /// Duplicates topology into a deterministic ordered instance sequence.
    ArrayTopology,
    /// Refines faces or regions into deterministic sub-elements.
    SubdivideTopology,
    /// Replaces corners or edges with bevel faces and support loops.
    BevelTopology,
    /// Creates offset inner or outer shell topology with rim closure.
    ShellSolidifyTopology,
    /// Applies a closed-volume boolean using constrained intersection topology.
    ConstrainedBooleanTopology,
}

/// Stable provenance behavior for an operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StableProvenanceBehavior {
    /// Rule for topology that survives from input selections.
    pub retained_input: RetainedInputRule,
    /// Rule for newly generated topology IDs.
    pub generated: GeneratedTopologyRule,
    /// Rule for relationships between source and generated topology.
    pub relationship: ProvenanceRelationship,
}

/// How existing topology IDs are retained.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetainedInputRule {
    /// No input topology is consumed.
    None,
    /// Input topology is retained unless directly replaced by the operator.
    PreserveUnmodified,
    /// Input boundary topology is retained and interior topology may be replaced.
    PreserveBoundary,
    /// Selected topology is consumed into a new semantic element.
    ConsumeSelected,
    /// Inputs are retained as parents for duplicate topology.
    PreserveAsSource,
}

/// How new topology IDs are derived.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedTopologyRule {
    /// IDs derive from primitive kind, operation ID, and canonical primitive order.
    PrimitiveRoot,
    /// IDs derive from selected region and canonical boundary traversal.
    RegionBoundaryOrder,
    /// IDs derive from selected loops and loop-pair traversal.
    LoopPairOrder,
    /// IDs derive from selected elements and canonical split order.
    SplitOrder,
    /// IDs derive from selected elements and canonical merge survivor order.
    MergeOrder,
    /// IDs derive from source element ID and deterministic instance index.
    InstanceIndex,
    /// IDs derive from source element ID and reflection side.
    MirrorSide,
    /// IDs derive from intersecting operands and canonical boolean cell order.
    BooleanCellOrder,
}

/// Stable relationship recorded between inputs and outputs.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceRelationship {
    /// Operation creates a new root without input parents.
    NewRoot,
    /// Generated topology records the selected topology as its parent.
    ParentChild,
    /// Generated topology records source and counterpart pairs.
    SourceCounterpart,
    /// Generated topology records source and ordered instance index.
    SourceInstance,
    /// Generated topology records all merged sources and canonical survivor.
    MergeSurvivor,
    /// Generated topology records split siblings from one source.
    SplitSiblings,
    /// Generated topology records boolean operand and cell provenance.
    BooleanOperands,
}

/// Selection contract for one topology operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectionRequirements {
    /// Number of selections required.
    pub count: SelectionCount,
    /// Accepted semantic selection subjects.
    pub accepted_subjects: Vec<SelectionSubject>,
    /// Whether selection order changes the deterministic result.
    pub ordered: bool,
    /// Whether an empty selection list is valid.
    pub allows_empty: bool,
}

/// Required selection count.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionCount {
    /// No selection is accepted.
    ExactlyZero,
    /// Exactly one selection is required.
    ExactlyOne,
    /// Exactly two selections are required.
    ExactlyTwo,
    /// One or more selections are required.
    OneOrMore,
    /// Two or more selections are required.
    TwoOrMore,
}

/// Semantic subject accepted by a topology operator.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectionSubject {
    /// Whole semantic part.
    Part,
    /// Whole semantic object or joined root.
    Object,
    /// Semantic face region.
    Region,
    /// Boundary loop.
    BoundaryLoop,
    /// Edge loop or edge class.
    EdgeLoop,
    /// Edge set.
    EdgeSet,
    /// Vertex set.
    VertexSet,
    /// Boolean cutter operand.
    BooleanOperand,
}

/// Exact replay contract for one operator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExactReplayBehavior {
    /// Canonical replay action.
    pub action: ReplayAction,
    /// How the evaluator resolves ties.
    pub tie_breaker: TieBreaker,
    /// How serialization order must be produced after replay.
    pub serialization_order: SerializationOrderRule,
    /// Required behavior for invalid selections or parameter domains.
    pub invalid_input: InvalidInputBehavior,
}

/// Deterministic action used by exact replay.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayAction {
    /// Build the named primitive from canonical templates and parameters.
    InstantiateCanonicalPrimitive,
    /// Apply the operation over canonical region traversal.
    TraverseSelectedRegion,
    /// Apply the operation over canonical loop traversal.
    TraverseSelectedLoops,
    /// Apply the operation over sorted selected element IDs.
    TraverseSortedSelection,
    /// Apply the operation over sorted source parts.
    TraverseSortedParts,
    /// Apply reflection and weld decisions in canonical plane order.
    ReflectThenCanonicalWeld,
    /// Apply instances in increasing index order.
    EmitInstancesInIndexOrder,
    /// Apply boolean classification and cell emission in canonical order.
    ClassifyBooleanCells,
}

/// Tie-break behavior for deterministic replay.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TieBreaker {
    /// Primitive-local template order.
    PrimitiveTemplateOrder,
    /// Stable semantic ID lexical order.
    StableIdOrder,
    /// Boundary loop winding, then stable ID order.
    WindingThenStableId,
    /// Distance along canonical edge-loop parameter, then stable ID order.
    LoopParameterThenStableId,
    /// Operand role, then stable ID order.
    OperandRoleThenStableId,
    /// Instance index, then source stable ID order.
    InstanceIndexThenStableId,
}

/// Serialization order produced by exact replay.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerializationOrderRule {
    /// Emits primitive template order.
    PrimitiveTemplate,
    /// Preserves retained input order and appends generated topology.
    PreserveThenAppendGenerated,
    /// Rewrites the affected local span in canonical order.
    CanonicalLocalRewrite,
    /// Emits sorted joined parts followed by generated connectors.
    SortedPartsThenGenerated,
    /// Emits source topology followed by ordered duplicates.
    SourceThenDuplicates,
    /// Emits boolean cells in classified canonical order.
    BooleanCellOrder,
}

/// Required invalid-input response.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidInputBehavior {
    /// Operation must fail without partial topology emission.
    RejectWithoutMutation,
    /// Operation may become a deterministic no-op when the selection is valid but empty.
    DeterministicNoOp,
}

/// Inference hint used by inverse reconstruction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceHint {
    PrimitiveClass,
    RegionNormal,
    RegionScale,
    BoundaryLoopOffset,
    EdgeFlowContinuity,
    BoundaryLoopPairing,
    CoincidentElements,
    PlanarCut,
    SurfaceContinuity,
    PartConnectivity,
    SymmetryPlane,
    RepetitionVector,
    SubdivisionPattern,
    EdgeSharpness,
    Thickness,
    ClosedVolumeOverlap,
}

/// Return true when the operation kind has a topology-construction contract.
#[must_use]
pub fn has_topology_contract(kind: ModelingOperationKind) -> bool {
    TOPOLOGY_OPERATOR_KINDS.contains(&kind)
}

/// Return the deterministic topology contract for an operation kind.
#[must_use]
pub fn topology_contract_for(kind: ModelingOperationKind) -> Option<TopologyOperatorContract> {
    match kind {
        ModelingOperationKind::PrimitiveCreate => Some(contract(
            kind,
            TopologyEffect::CreatePrimitive,
            provenance(
                RetainedInputRule::None,
                GeneratedTopologyRule::PrimitiveRoot,
                ProvenanceRelationship::NewRoot,
            ),
            selections(SelectionCount::ExactlyZero, [], false, true),
            replay(
                ReplayAction::InstantiateCanonicalPrimitive,
                TieBreaker::PrimitiveTemplateOrder,
                SerializationOrderRule::PrimitiveTemplate,
            ),
            8,
            [InferenceHint::PrimitiveClass],
        )),
        ModelingOperationKind::RegionExtrude => Some(contract(
            kind,
            TopologyEffect::ExtrudeRegion,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::RegionBoundaryOrder,
                ProvenanceRelationship::ParentChild,
            ),
            selections(
                SelectionCount::ExactlyOne,
                [SelectionSubject::Region],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSelectedRegion,
                TieBreaker::WindingThenStableId,
                SerializationOrderRule::PreserveThenAppendGenerated,
            ),
            4,
            [
                InferenceHint::RegionNormal,
                InferenceHint::EdgeFlowContinuity,
            ],
        )),
        ModelingOperationKind::RegionInset => Some(contract(
            kind,
            TopologyEffect::InsetRegion,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::RegionBoundaryOrder,
                ProvenanceRelationship::ParentChild,
            ),
            selections(
                SelectionCount::ExactlyOne,
                [SelectionSubject::Region],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSelectedRegion,
                TieBreaker::WindingThenStableId,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            3,
            [
                InferenceHint::BoundaryLoopOffset,
                InferenceHint::RegionScale,
            ],
        )),
        ModelingOperationKind::LoopCut => Some(contract(
            kind,
            TopologyEffect::InsertLoopCut,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::SplitOrder,
                ProvenanceRelationship::SplitSiblings,
            ),
            selections(
                SelectionCount::ExactlyOne,
                [SelectionSubject::EdgeLoop],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSelectedLoops,
                TieBreaker::LoopParameterThenStableId,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            3,
            [InferenceHint::EdgeFlowContinuity, InferenceHint::PlanarCut],
        )),
        ModelingOperationKind::BridgeLoops => Some(contract(
            kind,
            TopologyEffect::BridgeBoundaryLoops,
            provenance(
                RetainedInputRule::PreserveUnmodified,
                GeneratedTopologyRule::LoopPairOrder,
                ProvenanceRelationship::SourceCounterpart,
            ),
            selections(
                SelectionCount::ExactlyTwo,
                [SelectionSubject::BoundaryLoop],
                true,
                false,
            ),
            replay(
                ReplayAction::TraverseSelectedLoops,
                TieBreaker::WindingThenStableId,
                SerializationOrderRule::PreserveThenAppendGenerated,
            ),
            4,
            [
                InferenceHint::BoundaryLoopPairing,
                InferenceHint::SurfaceContinuity,
            ],
        )),
        ModelingOperationKind::Merge => Some(contract(
            kind,
            TopologyEffect::MergeElements,
            provenance(
                RetainedInputRule::ConsumeSelected,
                GeneratedTopologyRule::MergeOrder,
                ProvenanceRelationship::MergeSurvivor,
            ),
            selections(
                SelectionCount::TwoOrMore,
                [
                    SelectionSubject::VertexSet,
                    SelectionSubject::EdgeSet,
                    SelectionSubject::Region,
                    SelectionSubject::Part,
                ],
                true,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedSelection,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            2,
            [
                InferenceHint::CoincidentElements,
                InferenceHint::PartConnectivity,
            ],
        )),
        ModelingOperationKind::Split => Some(contract(
            kind,
            TopologyEffect::SplitElements,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::SplitOrder,
                ProvenanceRelationship::SplitSiblings,
            ),
            selections(
                SelectionCount::ExactlyOne,
                [
                    SelectionSubject::Region,
                    SelectionSubject::EdgeSet,
                    SelectionSubject::VertexSet,
                    SelectionSubject::Part,
                ],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedSelection,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            3,
            [InferenceHint::PlanarCut, InferenceHint::PartConnectivity],
        )),
        ModelingOperationKind::Dissolve => Some(contract(
            kind,
            TopologyEffect::DissolveElements,
            provenance(
                RetainedInputRule::ConsumeSelected,
                GeneratedTopologyRule::MergeOrder,
                ProvenanceRelationship::MergeSurvivor,
            ),
            selections(
                SelectionCount::OneOrMore,
                [
                    SelectionSubject::VertexSet,
                    SelectionSubject::EdgeSet,
                    SelectionSubject::Region,
                ],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedSelection,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            2,
            [
                InferenceHint::SurfaceContinuity,
                InferenceHint::EdgeFlowContinuity,
            ],
        )),
        ModelingOperationKind::Separate => Some(contract(
            kind,
            TopologyEffect::SeparatePart,
            provenance(
                RetainedInputRule::ConsumeSelected,
                GeneratedTopologyRule::SplitOrder,
                ProvenanceRelationship::SplitSiblings,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::Region, SelectionSubject::Part],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedParts,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::SortedPartsThenGenerated,
            ),
            2,
            [InferenceHint::PartConnectivity],
        )),
        ModelingOperationKind::Join => Some(contract(
            kind,
            TopologyEffect::JoinParts,
            provenance(
                RetainedInputRule::ConsumeSelected,
                GeneratedTopologyRule::MergeOrder,
                ProvenanceRelationship::MergeSurvivor,
            ),
            selections(
                SelectionCount::TwoOrMore,
                [SelectionSubject::Part, SelectionSubject::Object],
                true,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedParts,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::SortedPartsThenGenerated,
            ),
            2,
            [InferenceHint::PartConnectivity],
        )),
        ModelingOperationKind::Mirror => Some(contract(
            kind,
            TopologyEffect::MirrorTopology,
            provenance(
                RetainedInputRule::PreserveAsSource,
                GeneratedTopologyRule::MirrorSide,
                ProvenanceRelationship::SourceCounterpart,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::Part, SelectionSubject::Region],
                false,
                false,
            ),
            replay(
                ReplayAction::ReflectThenCanonicalWeld,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::SourceThenDuplicates,
            ),
            6,
            [
                InferenceHint::SymmetryPlane,
                InferenceHint::CoincidentElements,
            ],
        )),
        ModelingOperationKind::Array => Some(contract(
            kind,
            TopologyEffect::ArrayTopology,
            provenance(
                RetainedInputRule::PreserveAsSource,
                GeneratedTopologyRule::InstanceIndex,
                ProvenanceRelationship::SourceInstance,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::Part, SelectionSubject::Region],
                false,
                false,
            ),
            replay(
                ReplayAction::EmitInstancesInIndexOrder,
                TieBreaker::InstanceIndexThenStableId,
                SerializationOrderRule::SourceThenDuplicates,
            ),
            7,
            [InferenceHint::RepetitionVector],
        )),
        ModelingOperationKind::Subdivide => Some(contract(
            kind,
            TopologyEffect::SubdivideTopology,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::SplitOrder,
                ProvenanceRelationship::SplitSiblings,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::Region, SelectionSubject::Part],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedSelection,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            4,
            [
                InferenceHint::SubdivisionPattern,
                InferenceHint::EdgeSharpness,
            ],
        )),
        ModelingOperationKind::Bevel => Some(contract(
            kind,
            TopologyEffect::BevelTopology,
            provenance(
                RetainedInputRule::PreserveBoundary,
                GeneratedTopologyRule::SplitOrder,
                ProvenanceRelationship::ParentChild,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::EdgeSet, SelectionSubject::VertexSet],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSortedSelection,
                TieBreaker::StableIdOrder,
                SerializationOrderRule::CanonicalLocalRewrite,
            ),
            5,
            [
                InferenceHint::EdgeSharpness,
                InferenceHint::SurfaceContinuity,
            ],
        )),
        ModelingOperationKind::ShellSolidify => Some(contract(
            kind,
            TopologyEffect::ShellSolidifyTopology,
            provenance(
                RetainedInputRule::PreserveAsSource,
                GeneratedTopologyRule::RegionBoundaryOrder,
                ProvenanceRelationship::SourceCounterpart,
            ),
            selections(
                SelectionCount::OneOrMore,
                [SelectionSubject::Part, SelectionSubject::Region],
                false,
                false,
            ),
            replay(
                ReplayAction::TraverseSelectedRegion,
                TieBreaker::WindingThenStableId,
                SerializationOrderRule::SourceThenDuplicates,
            ),
            5,
            [InferenceHint::Thickness, InferenceHint::BoundaryLoopOffset],
        )),
        ModelingOperationKind::ConstrainedBoolean => Some(contract(
            kind,
            TopologyEffect::ConstrainedBooleanTopology,
            provenance(
                RetainedInputRule::ConsumeSelected,
                GeneratedTopologyRule::BooleanCellOrder,
                ProvenanceRelationship::BooleanOperands,
            ),
            selections(
                SelectionCount::ExactlyTwo,
                [SelectionSubject::Part, SelectionSubject::BooleanOperand],
                true,
                false,
            ),
            replay(
                ReplayAction::ClassifyBooleanCells,
                TieBreaker::OperandRoleThenStableId,
                SerializationOrderRule::BooleanCellOrder,
            ),
            6,
            [
                InferenceHint::ClosedVolumeOverlap,
                InferenceHint::SurfaceContinuity,
            ],
        )),
        _ => None,
    }
}

/// Return all topology-construction operator contracts in registry order.
#[must_use]
pub fn all_topology_contracts() -> Vec<TopologyOperatorContract> {
    TOPOLOGY_OPERATOR_KINDS
        .into_iter()
        .filter_map(topology_contract_for)
        .collect()
}

fn contract(
    kind: ModelingOperationKind,
    topology_effect: TopologyEffect,
    stable_provenance: StableProvenanceBehavior,
    selection_requirements: SelectionRequirements,
    exact_replay: ExactReplayBehavior,
    semantic_parameter_count: usize,
    inference_hints: impl IntoIterator<Item = InferenceHint>,
) -> TopologyOperatorContract {
    TopologyOperatorContract {
        kind,
        topology_effect,
        stable_provenance,
        selection_requirements,
        exact_replay,
        semantic_parameter_count,
        inference_hints: inference_hints.into_iter().collect(),
    }
}

fn provenance(
    retained_input: RetainedInputRule,
    generated: GeneratedTopologyRule,
    relationship: ProvenanceRelationship,
) -> StableProvenanceBehavior {
    StableProvenanceBehavior {
        retained_input,
        generated,
        relationship,
    }
}

fn selections(
    count: SelectionCount,
    accepted_subjects: impl IntoIterator<Item = SelectionSubject>,
    ordered: bool,
    allows_empty: bool,
) -> SelectionRequirements {
    SelectionRequirements {
        count,
        accepted_subjects: accepted_subjects.into_iter().collect(),
        ordered,
        allows_empty,
    }
}

fn replay(
    action: ReplayAction,
    tie_breaker: TieBreaker,
    serialization_order: SerializationOrderRule,
) -> ExactReplayBehavior {
    ExactReplayBehavior {
        action,
        tie_breaker,
        serialization_order,
        invalid_input: InvalidInputBehavior::RejectWithoutMutation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_required_topology_family_has_a_contract() {
        let contracts = all_topology_contracts();

        assert_eq!(contracts.len(), TOPOLOGY_OPERATOR_KINDS.len());
        for kind in TOPOLOGY_OPERATOR_KINDS {
            let contract = topology_contract_for(kind)
                .unwrap_or_else(|| panic!("{kind:?} should have a topology contract"));

            assert_eq!(contract.kind, kind);
            assert!(contract.semantic_parameter_count > 0);
            assert!(!contract.inference_hints.is_empty());
            assert!(has_topology_contract(kind));
        }
    }

    #[test]
    fn non_topology_operation_kinds_do_not_return_contracts() {
        assert!(!has_topology_contract(ModelingOperationKind::Bend));
        assert!(topology_contract_for(ModelingOperationKind::Bend).is_none());
        assert!(topology_contract_for(ModelingOperationKind::OpaqueResidual).is_none());
    }

    #[test]
    fn selection_contracts_capture_required_operator_arity() {
        let primitive = topology_contract_for(ModelingOperationKind::PrimitiveCreate).unwrap();
        assert_eq!(
            primitive.selection_requirements.count,
            SelectionCount::ExactlyZero
        );
        assert!(primitive.selection_requirements.allows_empty);

        let bridge = topology_contract_for(ModelingOperationKind::BridgeLoops).unwrap();
        assert_eq!(
            bridge.selection_requirements.count,
            SelectionCount::ExactlyTwo
        );
        assert!(bridge.selection_requirements.ordered);
        assert_eq!(
            bridge.selection_requirements.accepted_subjects,
            vec![SelectionSubject::BoundaryLoop]
        );

        let boolean = topology_contract_for(ModelingOperationKind::ConstrainedBoolean).unwrap();
        assert_eq!(
            boolean.selection_requirements.count,
            SelectionCount::ExactlyTwo
        );
        assert!(boolean.selection_requirements.ordered);
    }

    #[test]
    fn expected_semantic_parameter_counts_are_stable() {
        let expected = [
            (ModelingOperationKind::PrimitiveCreate, 8),
            (ModelingOperationKind::RegionExtrude, 4),
            (ModelingOperationKind::RegionInset, 3),
            (ModelingOperationKind::LoopCut, 3),
            (ModelingOperationKind::BridgeLoops, 4),
            (ModelingOperationKind::Merge, 2),
            (ModelingOperationKind::Split, 3),
            (ModelingOperationKind::Dissolve, 2),
            (ModelingOperationKind::Separate, 2),
            (ModelingOperationKind::Join, 2),
            (ModelingOperationKind::Mirror, 6),
            (ModelingOperationKind::Array, 7),
            (ModelingOperationKind::Subdivide, 4),
            (ModelingOperationKind::Bevel, 5),
            (ModelingOperationKind::ShellSolidify, 5),
            (ModelingOperationKind::ConstrainedBoolean, 6),
        ];

        for (kind, count) in expected {
            assert_eq!(
                topology_contract_for(kind)
                    .unwrap()
                    .semantic_parameter_count,
                count,
                "{kind:?} parameter count changed"
            );
        }
    }

    #[test]
    fn contracts_are_serializable_data() {
        let contract = topology_contract_for(ModelingOperationKind::Mirror).unwrap();
        let json = serde_json::to_string(&contract).expect("contract should serialize");
        let round_trip: TopologyOperatorContract =
            serde_json::from_str(&json).expect("contract should deserialize");

        assert_eq!(round_trip, contract);
        assert!(json.contains("mirror_topology"));
        assert!(json.contains("symmetry_plane"));
    }
}
