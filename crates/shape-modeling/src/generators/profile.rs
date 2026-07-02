//! Profile-driven sweep and lathe generators.
//!
//! The public specs in this module model the richer profile-generator controls
//! before those controls are represented directly in `shape-asset`.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    Frame3, PartDefinitionId, PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec,
    SurfaceRole,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, FaceMetadata, PolygonMesh,
    build_adjacency, polygon_mesh_from_faces,
};

use crate::{GeneratedPart, GeneratorContext, ModelingError};

include!("profile/contracts_sweep_lathe.rs");
include!("profile/profile_frames.rs");
include!("profile/lathe_mesh.rs");
