//! Role conformance report contracts.

use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{AssetRecipe, PartInstanceId};
use orchard_family::{AssetFamilySchema, PartRole, RoleMultiplicity};
use serde::{Deserialize, Serialize};

use super::ConformanceStatus;
use crate::remap::ports::SelectedFragmentPorts;

/// Inclusive role occurrence expectation resolved from family multiplicity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleMultiplicityExpectation {
    /// Minimum accepted occurrence count.
    pub min: u32,
    /// Maximum accepted occurrence count, or `None` for unbounded repeated roles.
    pub max: Option<u32>,
}

/// Conformance row for one family role.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleConformance {
    /// Family role ID.
    pub role: String,
    /// Expected occurrence range.
    pub expected: RoleMultiplicityExpectation,
    /// Actual exported occurrence count.
    pub actual_occurrences: u32,
    /// Whether provider selection and presence controls left this role enabled.
    pub effective_enabled: bool,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this role.
    pub issue_codes: Vec<String>,
}

/// Evaluate exported role occurrence multiplicity for selected fragments.
#[must_use]
pub fn evaluate_role_conformance(
    family: &AssetFamilySchema,
    recipe: &AssetRecipe,
    selected_fragments: &[SelectedFragmentPorts<'_>],
) -> Vec<RoleConformance> {
    let exported = remapped_role_occurrence_roots(selected_fragments);
    let mut rows = family
        .part_roles
        .iter()
        .map(|role| role_conformance_row(role, recipe, &exported))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.role.cmp(&right.role));
    rows
}

pub(crate) fn remapped_role_occurrence_roots(
    selected_fragments: &[SelectedFragmentPorts<'_>],
) -> BTreeMap<String, Vec<PartInstanceId>> {
    let mut by_role = BTreeMap::<String, BTreeSet<PartInstanceId>>::new();
    for selection in selected_fragments {
        let occurrences = by_role.entry(selection.role.to_owned()).or_default();
        for local_root in &selection.fragment.exports.role_occurrence_roots {
            if let Some(instance) = selection.remap.instances.get(local_root) {
                occurrences.insert(*instance);
            }
        }
    }
    by_role
        .into_iter()
        .map(|(role, occurrences)| (role, occurrences.into_iter().collect()))
        .collect()
}

pub(crate) fn is_effectively_enabled(recipe: &AssetRecipe, instance: PartInstanceId) -> bool {
    let mut current = Some(instance);
    let mut seen = BTreeSet::new();
    while let Some(instance_id) = current {
        if !seen.insert(instance_id) {
            return false;
        }
        let Some(part) = recipe.instances.get(&instance_id) else {
            return false;
        };
        if !part.enabled {
            return false;
        }
        current = part.parent;
    }
    true
}

fn role_conformance_row(
    role: &PartRole,
    recipe: &AssetRecipe,
    exported: &BTreeMap<String, Vec<PartInstanceId>>,
) -> RoleConformance {
    let expected = role_multiplicity_expectation(role);
    let exported_occurrences = exported.get(&role.id).map(Vec::as_slice).unwrap_or(&[]);
    let enabled_occurrences = exported_occurrences
        .iter()
        .filter(|instance| is_effectively_enabled(recipe, **instance))
        .count() as u32;
    let effective_enabled = enabled_occurrences > 0;
    let mut issue_codes = role_issue_codes(
        role,
        &expected,
        exported_occurrences.len() as u32,
        enabled_occurrences,
        effective_enabled,
    );
    issue_codes.sort();
    issue_codes.dedup();
    let status = if issue_codes.is_empty() {
        ConformanceStatus::Passed
    } else if enabled_occurrences == 0 && expected.min > 0 {
        ConformanceStatus::Missing
    } else {
        ConformanceStatus::Failed
    };

    RoleConformance {
        role: role.id.clone(),
        expected,
        actual_occurrences: exported_occurrences.len() as u32,
        effective_enabled,
        status,
        issue_codes,
    }
}

fn role_multiplicity_expectation(role: &PartRole) -> RoleMultiplicityExpectation {
    match &role.multiplicity {
        RoleMultiplicity::Single => RoleMultiplicityExpectation {
            min: 1,
            max: Some(1),
        },
        RoleMultiplicity::Optional => RoleMultiplicityExpectation {
            min: u32::from(role.required),
            max: Some(1),
        },
        RoleMultiplicity::Range { min, max } => RoleMultiplicityExpectation {
            min: *min,
            max: Some(*max),
        },
        RoleMultiplicity::Repeated => RoleMultiplicityExpectation {
            min: u32::from(role.required),
            max: None,
        },
    }
}

fn role_issue_codes(
    role: &PartRole,
    expected: &RoleMultiplicityExpectation,
    exported_occurrences: u32,
    enabled_occurrences: u32,
    effective_enabled: bool,
) -> Vec<String> {
    let mut issue_codes = Vec::new();
    if enabled_occurrences < expected.min {
        if exported_occurrences == 0 {
            issue_codes.push("missing_required_role".to_owned());
        } else if !effective_enabled {
            issue_codes.push("required_role_disabled".to_owned());
        } else {
            issue_codes.push("role_multiplicity_underflow".to_owned());
        }
    }
    if let Some(max) = expected.max
        && enabled_occurrences > max
    {
        issue_codes.push("role_multiplicity_overflow".to_owned());
    }
    if role.required && exported_occurrences > 0 && !effective_enabled {
        issue_codes.push("required_role_disabled".to_owned());
    }
    issue_codes
}
