//! Deterministic assembly evaluation for part instances, sockets, mirrors, and arrays.

use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{
    AssetRecipe, AttachmentMode, Frame3, ModelingOperationSpec, OperationId, PartDefinition,
    PartDefinitionId, PartInstance, PartInstanceId, PatternContract, PatternEvaluation,
    PatternEvaluationError, RegionId, SocketId, SocketSpec, Transform3,
};
use orchard_poly::{
    ElementId, FaceMetadata, MeshBounds, PolyError, PolygonMesh, TriangulatedPolygonMesh,
    bounds_from_positions, combine_polygon_meshes, compute_topology_signature,
    triangulate_polygon_mesh,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{GeneratedPart, GeneratorContext, GeometryGenerator, ModelingError, generate_geometry};

include!("assembly/planning_transform.rs");
include!("assembly/evaluation_state.rs");
include!("assembly/validation_transform_helpers.rs");
