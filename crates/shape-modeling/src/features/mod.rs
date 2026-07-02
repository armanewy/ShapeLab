//! Semantic constructive detail features.
//!
//! These builders create explicit polygon topology for common constructive
//! details while keeping the details separate from their host geometry. They do
//! not perform generic mesh booleans or silently fuse host meshes.

use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{FRAC_PI_2, PI};

use serde::{Deserialize, Serialize};
use shape_asset::{
    Frame3, GeometryRecipe, GeometrySource, OperationId, PartDefinition, PartDefinitionId,
    PartInstance, PartInstanceId, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole,
    Transform3,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, ElementId, FaceMetadata, PolygonFace,
    PolygonMesh, bounds_from_positions, build_adjacency, compute_topology_signature,
    polygon_mesh_from_faces,
};
use thiserror::Error;

use crate::generators::basic::{
    CapMode as BasicCapMode, CylinderParams, FrustumParams, build_cylinder, build_frustum,
};
use crate::{GeneratedPart, GeneratorContext, ModelingError};

include!("contracts_panel_trim.rs");
include!("ribs_fasteners.rs");
include!("frames_instances.rs");
include!("mesh_builder.rs");
include!("geometry_helpers.rs");
