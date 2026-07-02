//! Export requirement conformance report contracts and metadata evaluation.

use orchard_asset::{AssetRecipe, Frame3, SurfaceRole};
use orchard_compile::AssetArtifact;
use orchard_family::{ExportRequirement, RuntimeMetadataRequirement};
use serde::{Deserialize, Serialize};

use super::ConformanceStatus;

/// Availability of runtime/export metadata in a compiled asset.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportMetadataAvailability {
    /// Metadata is present in the compiled package.
    Available,
    /// Metadata is expected to be supplied by an adapter after compilation.
    AdapterDeferred,
    /// Required metadata is absent.
    Missing,
}

/// Conformance row for one metadata requirement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportMetadataConformance {
    /// Metadata category.
    pub requirement: RuntimeMetadataRequirement,
    /// Availability result.
    pub availability: ExportMetadataAvailability,
    /// Deterministic issue codes attached to this requirement.
    pub issue_codes: Vec<String>,
}

/// Conformance row for one export profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportRequirementConformance {
    /// Export profile key.
    pub profile: String,
    /// Metadata rows.
    pub metadata: Vec<ExportMetadataConformance>,
    /// Optional triangle budget hint from the family contract.
    pub triangle_budget_hint: Option<u32>,
    /// Actual triangle count when an artifact is available.
    pub actual_triangle_count: Option<u32>,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this export profile.
    pub issue_codes: Vec<String>,
}

/// Evaluate export profile requirements against a recipe and optional artifact.
#[must_use]
pub fn evaluate_export_requirements(
    requirements: &[ExportRequirement],
    recipe: &AssetRecipe,
    artifact: Option<&AssetArtifact>,
) -> Vec<ExportRequirementConformance> {
    requirements
        .iter()
        .map(|requirement| evaluate_export_requirement(requirement, recipe, artifact))
        .collect()
}

/// Evaluate one export profile requirement.
#[must_use]
pub fn evaluate_export_requirement(
    requirement: &ExportRequirement,
    recipe: &AssetRecipe,
    artifact: Option<&AssetArtifact>,
) -> ExportRequirementConformance {
    let metadata = requirement
        .required_metadata
        .iter()
        .map(|metadata| evaluate_metadata_requirement(metadata, recipe, artifact))
        .collect::<Vec<_>>();
    let actual_triangle_count =
        artifact.map(|artifact| artifact.statistics.triangle_count.min(u32::MAX as u64) as u32);

    let mut issue_codes = Vec::new();
    if metadata
        .iter()
        .any(|row| row.availability == ExportMetadataAvailability::Missing)
    {
        issue_codes.push("missing_required_export_metadata".to_owned());
    }
    if requirement.triangle_budget_hint.is_some() && actual_triangle_count.is_none() {
        issue_codes.push("missing_artifact_triangle_count".to_owned());
    }
    if let Some((budget, actual)) = requirement.triangle_budget_hint.zip(actual_triangle_count)
        && actual > budget
    {
        issue_codes.push("export_triangle_budget_exceeded".to_owned());
    }

    let status = if issue_codes.iter().any(|code| {
        code == "missing_required_export_metadata" || code == "missing_artifact_triangle_count"
    }) {
        ConformanceStatus::Missing
    } else if issue_codes
        .iter()
        .any(|code| code == "export_triangle_budget_exceeded")
    {
        ConformanceStatus::Failed
    } else {
        ConformanceStatus::Passed
    };

    ExportRequirementConformance {
        profile: requirement.profile.clone(),
        metadata,
        triangle_budget_hint: requirement.triangle_budget_hint,
        actual_triangle_count,
        status,
        issue_codes,
    }
}

fn evaluate_metadata_requirement(
    requirement: &RuntimeMetadataRequirement,
    recipe: &AssetRecipe,
    artifact: Option<&AssetArtifact>,
) -> ExportMetadataConformance {
    let availability = metadata_availability(requirement, recipe, artifact);
    let issue_codes = match availability {
        ExportMetadataAvailability::Available => Vec::new(),
        ExportMetadataAvailability::AdapterDeferred => {
            vec!["export_metadata_adapter_deferred".to_owned()]
        }
        ExportMetadataAvailability::Missing => vec!["export_metadata_missing".to_owned()],
    };
    ExportMetadataConformance {
        requirement: requirement.clone(),
        availability,
        issue_codes,
    }
}

fn metadata_availability(
    requirement: &RuntimeMetadataRequirement,
    recipe: &AssetRecipe,
    artifact: Option<&AssetArtifact>,
) -> ExportMetadataAvailability {
    match requirement {
        RuntimeMetadataRequirement::Pivot => availability_if(
            !recipe.definitions.is_empty()
                && recipe
                    .definitions
                    .values()
                    .all(|definition| frame_is_finite(&definition.local_pivot)),
        ),
        RuntimeMetadataRequirement::SnapAnchors => availability_if(
            recipe
                .definitions
                .values()
                .any(|definition| !definition.sockets.is_empty()),
        ),
        RuntimeMetadataRequirement::Footprint => {
            if artifact.is_some_and(|artifact| !artifact.combined_polygon.bounds.is_empty()) {
                ExportMetadataAvailability::Available
            } else {
                ExportMetadataAvailability::AdapterDeferred
            }
        }
        RuntimeMetadataRequirement::WalkableSurfaces => {
            tagged_surface_availability(recipe, ["walkable", "walkable_surface"], [])
        }
        RuntimeMetadataRequirement::SupportSurfaces => tagged_surface_availability(
            recipe,
            ["support", "support_surface"],
            [SurfaceRole::Attachment],
        ),
        RuntimeMetadataRequirement::CollisionProxies
        | RuntimeMetadataRequirement::ConstructionPhases
        | RuntimeMetadataRequirement::Lod => ExportMetadataAvailability::AdapterDeferred,
        RuntimeMetadataRequirement::Previews => {
            if artifact.is_some_and(|artifact| artifact.statistics.triangle_count > 0) {
                ExportMetadataAvailability::Available
            } else {
                ExportMetadataAvailability::AdapterDeferred
            }
        }
        RuntimeMetadataRequirement::Custom(key) => {
            if custom_metadata_hint_available(recipe, key) {
                ExportMetadataAvailability::Available
            } else {
                ExportMetadataAvailability::Missing
            }
        }
    }
}

fn availability_if(available: bool) -> ExportMetadataAvailability {
    if available {
        ExportMetadataAvailability::Available
    } else {
        ExportMetadataAvailability::Missing
    }
}

fn tagged_surface_availability<const T: usize, const R: usize>(
    recipe: &AssetRecipe,
    tags: [&str; T],
    roles: [SurfaceRole; R],
) -> ExportMetadataAvailability {
    if recipe.definitions.values().any(|definition| {
        definition.regions.values().any(|region| {
            roles.contains(&region.role)
                || tags
                    .iter()
                    .any(|tag| region.tags.contains(*tag) || definition.tags.contains(*tag))
        })
    }) {
        ExportMetadataAvailability::Available
    } else {
        ExportMetadataAvailability::AdapterDeferred
    }
}

fn custom_metadata_hint_available(recipe: &AssetRecipe, key: &str) -> bool {
    recipe.definitions.values().any(|definition| {
        definition.production_hints.as_ref().is_some_and(|hints| {
            hints.hints.contains_key(key)
                || hints.hints.contains_key(&format!("metadata.{key}"))
                || hints.preferred_generator.as_deref() == Some(key)
        })
    })
}

fn frame_is_finite(frame: &Frame3) -> bool {
    frame.origin.iter().all(|value| value.is_finite())
        && frame.x_axis.iter().all(|value| value.is_finite())
        && frame.y_axis.iter().all(|value| value.is_finite())
        && frame.z_axis.iter().all(|value| value.is_finite())
}
