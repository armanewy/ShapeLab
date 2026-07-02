//! Build stamp, compiled-output, and foundry document compilation contracts.
#![allow(clippy::result_large_err)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetEditProgram, AssetRecipe, AssetValidationReport, ContactPolicy, ExportRealizationPolicy,
    OrientationPolicy, ParameterId, PartInstanceId, PlacementPolicy, PositionRule,
    RelationshipContract, RelationshipType, ScalePolicy, validate_asset_recipe,
};
use shape_compile::{AssetArtifact, CompileError, compile_asset};
use shape_family::{
    AssetFamilySchema, FamilyDefaultValue, FamilyRuleExecutionPolicy, RoleProvision,
};
use shape_family_compile::{
    FamilyCompileError, FamilyImplementation, FamilyInstantiationReport,
    FamilyInstantiationRequest, FamilyValue, SHAPE_FAMILY_COMPILE_CRATE_VERSION,
    StyleImplementation,
    conformance::{
        ConformanceIssue, ConformanceStatus, ConstraintBindingMap, FamilyConformanceReport,
        evaluate_attachment_conformance, evaluate_export_requirements,
        evaluate_geometric_constraints, evaluate_operation_conformance, evaluate_role_conformance,
    },
    identity::{
        ArtifactFingerprint, BuildFingerprint, FingerprintError, GeometryInputFingerprint,
        RecipeFingerprint, fingerprint_serializable,
    },
    instantiate_family,
    remap::ports::SelectedFragmentPorts,
};

use crate::{
    ControlDivergence, ControlKind, ControlValue, DroppedLocalOverride, FoundryCatalogError,
    FoundryCatalogResolver, FoundryConformanceSummary, FoundryResolvedCatalog, LocalRecipeOverride,
    LocalRecipeOverrideId, OverrideSurvivalPolicy, ResponseCurve, SHAPE_FOUNDRY_CRATE_VERSION,
    TouchedSemanticTarget, catalog::resolve_foundry_catalog, control_divergence_state,
    validate_foundry_document,
};

include!("compile/contracts_and_entrypoints.rs");
include!("compile/family_requests.rs");
include!("compile/overrides.rs");
include!("compile/conformance.rs");
include!("compile/fingerprints.rs");
