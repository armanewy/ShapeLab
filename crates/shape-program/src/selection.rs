//! Semantic selection IR helpers and strict accounting.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    ExplicitSelectionTarget, OperationPayloadKind, SemanticAdmissibilityPolicy,
    SemanticBoundaryLoopId, SemanticPartId, SemanticRegionId, SemanticSelection,
    SemanticSelectionId, SemanticSelectionPayload, SpatialPrimitiveSelection,
};

/// Canonical accounting for one encoded semantic selection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncodedSemanticSelection {
    /// Stable selection ID.
    pub selection_id: SemanticSelectionId,
    /// Payload category used by strict admissibility.
    pub payload_kind: OperationPayloadKind,
    /// Canonical serialized byte size of the full selection record.
    pub encoded_bytes: usize,
    /// Compact semantic parameter count represented by this selection.
    pub semantic_parameter_count: usize,
    /// Number of explicitly listed element IDs.
    pub explicit_index_count: usize,
    /// Whether this selection is represented by the compact semantic vocabulary.
    pub compact_semantic: bool,
}

/// Selection IR validation errors.
#[derive(Debug, Error)]
pub enum SemanticSelectionError {
    /// Selection serialization failed.
    #[error("failed to serialize semantic selection: {0}")]
    Serialize(#[from] serde_json::Error),
    /// Explicit index lists above the strict threshold are not semantic explanations.
    #[error(
        "explicit selection `{selection_id}` contains {explicit_index_count} indices; strict semantic explanations allow at most {maximum_explicit_index_count}"
    )]
    ExplicitIndexListTooLarge {
        /// Stable selection ID.
        selection_id: String,
        /// Number of explicit IDs present.
        explicit_index_count: usize,
        /// Maximum explicit IDs allowed by policy.
        maximum_explicit_index_count: usize,
    },
}

impl SemanticSelection {
    /// Construct a selection from an existing payload.
    #[must_use]
    pub fn new(id: impl Into<String>, payload: SemanticSelectionPayload) -> Self {
        Self {
            id: SemanticSelectionId(id.into()),
            payload,
        }
    }

    /// Select a semantic part.
    #[must_use]
    pub fn part(id: impl Into<String>, part: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::Part {
                part: SemanticPartId(part.into()),
            },
        )
    }

    /// Select a semantic region.
    #[must_use]
    pub fn region(id: impl Into<String>, region: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::Region {
                region: SemanticRegionId(region.into()),
            },
        )
    }

    /// Select a semantic boundary loop.
    #[must_use]
    pub fn boundary_loop(id: impl Into<String>, boundary_loop: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::BoundaryLoop {
                boundary_loop: SemanticBoundaryLoopId(boundary_loop.into()),
            },
        )
    }

    /// Select an edge class.
    #[must_use]
    pub fn edge_class(id: impl Into<String>, class: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::EdgeClass {
                class: class.into(),
            },
        )
    }

    /// Select a face patch.
    #[must_use]
    pub fn face_patch(id: impl Into<String>, patch: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::FacePatch {
                patch: patch.into(),
            },
        )
    }

    /// Select the symmetry partner of another selection.
    #[must_use]
    pub fn symmetry_partner(id: impl Into<String>, selection: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::SymmetryPartner {
                selection: SemanticSelectionId(selection.into()),
            },
        )
    }

    /// Select a geodesic neighborhood around a seed selection.
    #[must_use]
    pub fn geodesic_neighborhood(
        id: impl Into<String>,
        seed: impl Into<String>,
        radius: f64,
    ) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::GeodesicNeighborhood {
                seed: SemanticSelectionId(seed.into()),
                radius,
            },
        )
    }

    /// Select by a compact spatial primitive.
    #[must_use]
    pub fn spatial_primitive(id: impl Into<String>, shape: SpatialPrimitiveSelection) -> Self {
        Self::new(id, SemanticSelectionPayload::SpatialPrimitive { shape })
    }

    /// Select a boolean operand by semantic operand ID.
    #[must_use]
    pub fn boolean_operand(id: impl Into<String>, operand_id: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::BooleanOperand {
                operand_id: operand_id.into(),
            },
        )
    }

    /// Select by a compact falloff field.
    #[must_use]
    pub fn compact_falloff_field(
        id: impl Into<String>,
        field_id: impl Into<String>,
        parameter_count: usize,
    ) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::CompactFalloffField {
                field_id: field_id.into(),
                parameter_count,
            },
        )
    }

    /// Select a semantic landmark group.
    #[must_use]
    pub fn semantic_landmark_group(id: impl Into<String>, group_id: impl Into<String>) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::SemanticLandmarkGroup {
                group_id: group_id.into(),
            },
        )
    }

    /// Select a small explicit element-ID list.
    #[must_use]
    pub fn explicit_indices(
        id: impl Into<String>,
        target: ExplicitSelectionTarget,
        indices: Vec<u32>,
    ) -> Self {
        Self::new(
            id,
            SemanticSelectionPayload::ExplicitIndices { target, indices },
        )
    }

    /// Return canonical encoded-size accounting for this selection.
    pub fn encoded_semantic_selection(
        &self,
    ) -> Result<EncodedSemanticSelection, SemanticSelectionError> {
        let encoded_bytes = serde_json::to_vec(self)?.len();
        Ok(EncodedSemanticSelection {
            selection_id: self.id.clone(),
            payload_kind: self.payload.operation_payload_kind(),
            encoded_bytes,
            semantic_parameter_count: self.payload.semantic_parameter_count(),
            explicit_index_count: self.payload.explicit_index_count(),
            compact_semantic: self.payload.is_compact_semantic(),
        })
    }

    /// Return strict accounting, rejecting explicit lists too large to be semantic.
    pub fn strict_semantic_selection(
        &self,
        policy: &SemanticAdmissibilityPolicy,
    ) -> Result<EncodedSemanticSelection, SemanticSelectionError> {
        let explicit_index_count = self.payload.explicit_index_count();
        if explicit_index_count > policy.maximum_explicit_selection_payload {
            return Err(SemanticSelectionError::ExplicitIndexListTooLarge {
                selection_id: self.id.0.clone(),
                explicit_index_count,
                maximum_explicit_index_count: policy.maximum_explicit_selection_payload,
            });
        }

        self.encoded_semantic_selection()
    }
}

impl SemanticSelectionPayload {
    /// Number of explicitly encoded element IDs.
    #[must_use]
    pub fn explicit_index_count(&self) -> usize {
        match self {
            Self::ExplicitIndices { indices, .. } => indices.len(),
            _ => 0,
        }
    }

    /// Whether this payload uses compact semantic selection vocabulary.
    #[must_use]
    pub fn is_compact_semantic(&self) -> bool {
        !matches!(self, Self::ExplicitIndices { .. })
    }

    /// Strict payload category represented by this selection.
    #[must_use]
    pub fn operation_payload_kind(&self) -> OperationPayloadKind {
        if self.is_compact_semantic() {
            OperationPayloadKind::CompactSelection
        } else {
            OperationPayloadKind::ExplicitSelectionIndices
        }
    }

    /// Count semantic parameters represented by this payload.
    #[must_use]
    pub fn semantic_parameter_count(&self) -> usize {
        match self {
            Self::Part { .. }
            | Self::Region { .. }
            | Self::BoundaryLoop { .. }
            | Self::EdgeClass { .. }
            | Self::FacePatch { .. }
            | Self::SymmetryPartner { .. }
            | Self::BooleanOperand { .. }
            | Self::SemanticLandmarkGroup { .. } => 1,
            Self::GeodesicNeighborhood { .. } => 2,
            Self::SpatialPrimitive { shape } => shape.semantic_parameter_count(),
            Self::CompactFalloffField {
                parameter_count, ..
            } => *parameter_count,
            Self::ExplicitIndices { indices, .. } => indices.len(),
        }
    }
}

impl SpatialPrimitiveSelection {
    /// Count scalar semantic parameters represented by this primitive.
    #[must_use]
    pub fn semantic_parameter_count(&self) -> usize {
        match self {
            Self::Sphere { .. } => 4,
            Self::Box { .. } => 6,
            Self::PlaneSlab { .. } => 5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ExplicitSelectionTarget;

    #[test]
    fn compact_selection_constructors_cover_semantic_vocabulary() {
        let selections = vec![
            SemanticSelection::part("sel.part", "part.arm"),
            SemanticSelection::region("sel.region", "region.palm"),
            SemanticSelection::boundary_loop("sel.loop", "loop.wrist"),
            SemanticSelection::edge_class("sel.edge.class", "crease"),
            SemanticSelection::face_patch("sel.patch", "patch.knuckle"),
            SemanticSelection::symmetry_partner("sel.partner", "sel.part"),
            SemanticSelection::geodesic_neighborhood("sel.geo", "sel.region", 2.5),
            SemanticSelection::spatial_primitive(
                "sel.sphere",
                SpatialPrimitiveSelection::Sphere {
                    center: [0.0, 1.0, 2.0],
                    radius: 3.0,
                },
            ),
            SemanticSelection::boolean_operand("sel.boolean", "operand.slot_cutter"),
            SemanticSelection::compact_falloff_field("sel.falloff", "field.soft", 3),
            SemanticSelection::semantic_landmark_group("sel.landmarks", "landmarks.face"),
        ];

        let policy = SemanticAdmissibilityPolicy::strict();
        for selection in selections {
            let accounting = selection
                .strict_semantic_selection(&policy)
                .expect("compact semantic selection should be accepted");
            assert_eq!(
                accounting.payload_kind,
                OperationPayloadKind::CompactSelection
            );
            assert_eq!(accounting.explicit_index_count, 0);
            assert!(accounting.compact_semantic);
            assert!(accounting.encoded_bytes > 0);
            assert!(accounting.semantic_parameter_count > 0);
        }
    }

    #[test]
    fn encoded_size_accounting_matches_canonical_serialization() {
        let selection = SemanticSelection::spatial_primitive(
            "sel.slab",
            SpatialPrimitiveSelection::PlaneSlab {
                normal: [0.0, 1.0, 0.0],
                offset: 1.25,
                half_width: 0.5,
            },
        );

        let accounting = selection
            .encoded_semantic_selection()
            .expect("selection should serialize");

        assert_eq!(
            accounting.encoded_bytes,
            serde_json::to_vec(&selection).unwrap().len()
        );
        assert_eq!(accounting.semantic_parameter_count, 5);
        assert_eq!(
            accounting.payload_kind,
            OperationPayloadKind::CompactSelection
        );
    }

    #[test]
    fn explicit_index_threshold_accepts_boundary_value() {
        let policy = SemanticAdmissibilityPolicy {
            maximum_explicit_selection_payload: 3,
            ..SemanticAdmissibilityPolicy::strict()
        };
        let selection = SemanticSelection::explicit_indices(
            "sel.faces",
            ExplicitSelectionTarget::Face,
            vec![10, 11, 12],
        );

        let accounting = selection
            .strict_semantic_selection(&policy)
            .expect("threshold-sized explicit list should be accepted");

        assert_eq!(
            accounting.payload_kind,
            OperationPayloadKind::ExplicitSelectionIndices
        );
        assert_eq!(accounting.explicit_index_count, 3);
        assert!(!accounting.compact_semantic);
    }

    #[test]
    fn giant_explicit_index_lists_are_rejected_as_strict_semantic_explanations() {
        let policy = SemanticAdmissibilityPolicy {
            maximum_explicit_selection_payload: 3,
            ..SemanticAdmissibilityPolicy::strict()
        };
        let selection = SemanticSelection::explicit_indices(
            "sel.too_many_vertices",
            ExplicitSelectionTarget::Vertex,
            vec![0, 1, 2, 3],
        );

        let error = selection
            .strict_semantic_selection(&policy)
            .expect_err("oversized explicit list should be rejected");

        match error {
            SemanticSelectionError::ExplicitIndexListTooLarge {
                selection_id,
                explicit_index_count,
                maximum_explicit_index_count,
            } => {
                assert_eq!(selection_id, "sel.too_many_vertices");
                assert_eq!(explicit_index_count, 4);
                assert_eq!(maximum_explicit_index_count, 3);
            }
            SemanticSelectionError::Serialize(_) => {
                panic!("expected explicit-list rejection, got serialization error");
            }
        }
    }
}
