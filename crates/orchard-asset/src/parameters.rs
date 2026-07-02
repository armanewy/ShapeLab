//! Beginner-facing parameter reflection.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ParameterDescriptor, ParameterId, PartInstanceId};

/// Beginner-facing parameter groups returned in deterministic order.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum BeginnerParameterGroup {
    /// Overall dimensions.
    Size,
    /// Relative proportions and scale.
    Proportions,
    /// Placement and orientation.
    Placement,
    /// Curved profile controls.
    Curvature,
    /// Bevel and soft-edge controls.
    EdgeSoftness,
    /// Repeated part and pattern controls.
    Repetition,
    /// Optional part visibility controls.
    PartPresence,
    /// Segment counts and authored detail density.
    DetailDensity,
}

impl BeginnerParameterGroup {
    /// Return all groups in beginner-facing display order.
    #[must_use]
    pub fn all() -> [Self; 8] {
        [
            Self::Size,
            Self::Proportions,
            Self::Placement,
            Self::Curvature,
            Self::EdgeSoftness,
            Self::Repetition,
            Self::PartPresence,
            Self::DetailDensity,
        ]
    }

    /// Return the display label for this group.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Proportions => "Proportions",
            Self::Placement => "Placement",
            Self::Curvature => "Curvature",
            Self::EdgeSoftness => "Edge Softness",
            Self::Repetition => "Repetition",
            Self::PartPresence => "Part Presence",
            Self::DetailDensity => "Detail Density",
        }
    }
}

/// One beginner-facing reflection group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeginnerParameterGroupReflection {
    /// Stable group identifier.
    pub group: BeginnerParameterGroup,
    /// Display label.
    pub label: String,
    /// Scalar parameters in deterministic order.
    pub parameters: Vec<ReflectedParameter>,
    /// Optional part presence controls.
    pub part_presence: Vec<ReflectedPartPresence>,
}

/// Reflected scalar parameter with current value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReflectedParameter {
    /// Stable parameter ID.
    pub id: ParameterId,
    /// Display label.
    pub label: String,
    /// Canonical scalar path.
    pub path: String,
    /// Current scalar value.
    pub value: f32,
    /// Minimum permitted scalar.
    pub minimum: f32,
    /// Maximum permitted scalar.
    pub maximum: f32,
    /// Suggested UI step.
    pub step: f32,
    /// Whether the parameter is currently locked.
    pub locked: bool,
    /// Whether changing this parameter can change topology.
    pub topology_changing: bool,
    /// Beginner-facing explanation.
    pub beginner_description: String,
}

/// Reflected optional part presence control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflectedPartPresence {
    /// Optional instance ID.
    pub instance: PartInstanceId,
    /// Display label.
    pub label: String,
    /// Current enabled state.
    pub enabled: bool,
    /// Whether the instance cannot currently be edited.
    pub locked: bool,
}

/// Return beginner-facing parameter groups.
#[must_use]
pub fn reflect_beginner_parameters(
    recipe: &crate::AssetRecipe,
) -> Vec<BeginnerParameterGroupReflection> {
    let mut groups = BeginnerParameterGroup::all()
        .into_iter()
        .map(|group| {
            (
                group,
                BeginnerParameterGroupReflection {
                    group,
                    label: group.label().to_owned(),
                    parameters: Vec::new(),
                    part_presence: Vec::new(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for parameter in crate::enumerate_parameters(recipe) {
        let group = beginner_group_for_parameter(&parameter);
        let Ok(value) = crate::get_scalar(recipe, &parameter.path) else {
            continue;
        };
        let reflected = ReflectedParameter {
            id: parameter.id,
            label: parameter.label,
            path: parameter.path,
            value,
            minimum: parameter.minimum,
            maximum: parameter.maximum,
            step: parameter.step,
            locked: recipe.locks.contains(&parameter.id),
            topology_changing: parameter.topology_changing,
            beginner_description: parameter.beginner_description,
        };
        groups
            .get_mut(&group)
            .expect("group should be initialized")
            .parameters
            .push(reflected);
    }

    let presence_group = groups
        .get_mut(&BeginnerParameterGroup::PartPresence)
        .expect("group should be initialized");
    for instance_id in &recipe.variation.optional_instances {
        let Some(instance) = recipe.instances.get(instance_id) else {
            continue;
        };
        presence_group.part_presence.push(ReflectedPartPresence {
            instance: *instance_id,
            label: instance.name.clone(),
            enabled: instance.enabled,
            locked: crate::edits::ensure_instance_editable(recipe, *instance_id).is_err(),
        });
    }

    BeginnerParameterGroup::all()
        .into_iter()
        .map(|group| groups.remove(&group).expect("group should be initialized"))
        .collect()
}

/// Return true when a path is safe to expose to beginner-facing parameter UIs.
#[must_use]
pub(crate) fn is_beginner_safe_parameter_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    if lower.contains("literal_mesh") {
        return false;
    }
    !lower.split('.').any(|part| {
        matches!(
            part,
            "vertex" | "vertices" | "face" | "faces" | "positions" | "indices"
        )
    })
}

fn beginner_group_for_parameter(parameter: &ParameterDescriptor) -> BeginnerParameterGroup {
    if let Some(group) = group_from_label(&parameter.group) {
        return group;
    }

    let lower = parameter.path.to_ascii_lowercase();
    if lower.contains(".transform.translation") || lower.contains(".transform.rotation") {
        BeginnerParameterGroup::Placement
    } else if lower.contains(".transform.scale") {
        BeginnerParameterGroup::Proportions
    } else if lower.contains("linear_array") || lower.contains("radial_array") {
        BeginnerParameterGroup::Repetition
    } else if lower.contains("segments") {
        BeginnerParameterGroup::DetailDensity
    } else if lower.contains("bevel") || lower.contains("radius") {
        BeginnerParameterGroup::EdgeSoftness
    } else if lower.contains("sweep") || lower.contains("lathe.profile") {
        BeginnerParameterGroup::Curvature
    } else if lower.contains("half_extents")
        || lower.contains("size")
        || lower.contains("height")
        || lower.contains("width")
        || lower.contains("thickness")
    {
        BeginnerParameterGroup::Size
    } else {
        BeginnerParameterGroup::Proportions
    }
}

fn group_from_label(label: &str) -> Option<BeginnerParameterGroup> {
    match label {
        "Size" => Some(BeginnerParameterGroup::Size),
        "Proportions" => Some(BeginnerParameterGroup::Proportions),
        "Placement" => Some(BeginnerParameterGroup::Placement),
        "Curvature" => Some(BeginnerParameterGroup::Curvature),
        "Edge Softness" => Some(BeginnerParameterGroup::EdgeSoftness),
        "Repetition" => Some(BeginnerParameterGroup::Repetition),
        "Part Presence" => Some(BeginnerParameterGroup::PartPresence),
        "Detail Density" => Some(BeginnerParameterGroup::DetailDensity),
        _ => None,
    }
}
