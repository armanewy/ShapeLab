use std::collections::{BTreeMap, BTreeSet};
use std::f32::consts::{FRAC_PI_2, PI};

use shape_asset::{
    BoundaryLoopId, CutEdgeTreatment, Frame3, GeometrySource, ModelingOperationSpec, OperationId,
    PartDefinition, PlanarCutFace, RegionId, SocketId, SocketSpec, SurfaceRegionSpec, SurfaceRole,
};
use shape_poly::{
    BoundaryRole, EdgeClassification, EdgeKey, EdgeMetadata, ElementId, FaceMetadata, PolygonFace,
    PolygonMesh, bounds_from_positions, compute_topology_signature,
};

use crate::{GeneratedPart, GeneratorContext, ModelingError};

include!("basic/contracts_and_entrypoints.rs");
include!("basic/rounded_box_cuts.rs");
include!("basic/plate_cuts.rs");
include!("basic/frustum_mesh.rs");
include!("basic/plate_cut_helpers.rs");
include!("basic/shell_and_loop_helpers.rs");
include!("basic/mesh_and_regions.rs");
include!("basic/validation_math.rs");
