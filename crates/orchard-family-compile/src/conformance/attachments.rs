//! Attachment conformance report contracts.

use std::collections::{BTreeMap, BTreeSet};

use orchard_asset::{AssetRecipe, Frame3, PartInstanceId, SocketId, SocketSpec};
use orchard_family::{AssetFamilySchema, AttachmentRule, FamilyRuleExecutionPolicy};
use serde::{Deserialize, Serialize};

use super::ConformanceStatus;
use super::roles::is_effectively_enabled;
use crate::remap::ports::SelectedFragmentPorts;
use crate::{FamilyImplementation, FragmentAttachmentPairing};

/// Concrete part/socket endpoint used by an attachment conformance row.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AttachmentEndpointConformance {
    /// Concrete part occurrence.
    pub instance: PartInstanceId,
    /// Socket on the occurrence definition.
    pub socket: SocketId,
}

/// Concrete pair evaluated for one attachment rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentPairConformance {
    /// Parent endpoint.
    pub parent: AttachmentEndpointConformance,
    /// Child endpoint.
    pub child: AttachmentEndpointConformance,
    /// Whether socket tags and frames are compatible.
    pub socket_compatible: bool,
    /// Whether the expected relationship was found.
    pub connected: bool,
}

/// Coverage summary for repeated attachment rules.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AttachmentCoverageConformance {
    /// From-role endpoints not covered by any evaluated pair.
    pub unmatched_first: Vec<AttachmentEndpointConformance>,
    /// To-role endpoints not covered by any evaluated pair.
    pub unmatched_second: Vec<AttachmentEndpointConformance>,
    /// Whether pairing produced at least one row.
    pub produced_pairs: bool,
}

/// Conformance row for one family attachment rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentConformance {
    /// Family attachment-rule ID.
    pub rule_id: String,
    /// Family rule `from_role`, the child/dependent side of the attachment.
    pub from_role: String,
    /// Family rule `to_role`, the parent/destination side of the attachment.
    pub to_role: String,
    /// Rule policy.
    pub policy: FamilyRuleExecutionPolicy,
    /// Evaluated pairs.
    pub pairs: Vec<AttachmentPairConformance>,
    /// Pairing coverage summary.
    pub coverage: AttachmentCoverageConformance,
    /// Row status.
    pub status: ConformanceStatus,
    /// Deterministic issue codes attached to this rule.
    pub issue_codes: Vec<String>,
}

/// Evaluate family attachment rules through selected fragment socket ports.
#[must_use]
pub fn evaluate_attachment_conformance(
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    recipe: &AssetRecipe,
    selected_fragments: &[SelectedFragmentPorts<'_>],
) -> Vec<AttachmentConformance> {
    let selected = selected_fragment_index(selected_fragments);
    let bindings = bindings_by_rule(family_impl);
    let mut rows = family
        .attachment_rules
        .iter()
        .map(|rule| {
            attachment_rule_conformance_row(
                rule,
                bindings
                    .get(rule.id.as_str())
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
                recipe,
                &selected,
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.rule_id.cmp(&right.rule_id));
    rows
}

#[derive(Debug, Clone)]
struct EndpointOccurrence {
    ordinal: usize,
    endpoint: AttachmentEndpointConformance,
    port_compatibility_tags: BTreeSet<String>,
    socket_compatibility_tags: BTreeSet<String>,
    frame_valid: bool,
}

#[derive(Debug, Clone)]
struct ResolvedAttachmentPort {
    occurrences: Vec<EndpointOccurrence>,
}

fn selected_fragment_index<'a>(
    selected_fragments: &'a [SelectedFragmentPorts<'a>],
) -> BTreeMap<&'a str, SelectedFragmentPorts<'a>> {
    let mut selected = BTreeMap::new();
    for selection in selected_fragments {
        selected.entry(selection.role).or_insert(*selection);
    }
    selected
}

fn bindings_by_rule(
    family_impl: &FamilyImplementation,
) -> BTreeMap<&str, Vec<(usize, &crate::FragmentAttachmentBinding)>> {
    let mut bindings = BTreeMap::<&str, Vec<(usize, &crate::FragmentAttachmentBinding)>>::new();
    for (index, binding) in family_impl.attachment_bindings.iter().enumerate() {
        bindings
            .entry(binding.family_attachment_rule.as_str())
            .or_default()
            .push((index, binding));
    }
    bindings
}

fn attachment_rule_conformance_row(
    rule: &AttachmentRule,
    bindings: &[(usize, &crate::FragmentAttachmentBinding)],
    recipe: &AssetRecipe,
    selected_fragments: &BTreeMap<&str, SelectedFragmentPorts<'_>>,
) -> AttachmentConformance {
    if rule.execution_policy == FamilyRuleExecutionPolicy::RuntimeOnly {
        return AttachmentConformance {
            rule_id: rule.id.clone(),
            from_role: rule.from_role.clone(),
            to_role: rule.to_role.clone(),
            policy: rule.execution_policy,
            pairs: Vec::new(),
            coverage: AttachmentCoverageConformance::default(),
            status: ConformanceStatus::Deferred,
            issue_codes: vec!["runtime_only_attachment_rule_deferred".to_owned()],
        };
    }

    let mut issue_codes = Vec::<String>::new();
    let mut pairs = Vec::<AttachmentPairConformance>::new();
    let mut first_endpoints = BTreeSet::<AttachmentEndpointConformance>::new();
    let mut second_endpoints = BTreeSet::<AttachmentEndpointConformance>::new();
    let mut covered_first = BTreeSet::<AttachmentEndpointConformance>::new();
    let mut covered_second = BTreeSet::<AttachmentEndpointConformance>::new();

    if bindings.is_empty() {
        issue_codes.push("missing_attachment_binding".to_owned());
    }

    for (binding_index, binding) in bindings {
        if binding.child_role != rule.from_role || binding.parent_role != rule.to_role {
            issue_codes.push("attachment_binding_rule_mismatch".to_owned());
            continue;
        }
        let parent = resolve_attachment_port(
            recipe,
            selected_fragments,
            &binding.parent_role,
            &binding.parent_port,
            &mut issue_codes,
        );
        let child = resolve_attachment_port(
            recipe,
            selected_fragments,
            &binding.child_role,
            &binding.child_port,
            &mut issue_codes,
        );
        let (Some(parent), Some(child)) = (parent, child) else {
            continue;
        };
        first_endpoints.extend(
            child
                .occurrences
                .iter()
                .map(|occurrence| occurrence.endpoint),
        );
        second_endpoints.extend(
            parent
                .occurrences
                .iter()
                .map(|occurrence| occurrence.endpoint),
        );

        let endpoint_pairs = pairing_pairs(
            recipe,
            *binding_index,
            &binding.pairing,
            &child.occurrences,
            &parent.occurrences,
            &mut issue_codes,
        );
        for (child, parent) in endpoint_pairs {
            let socket_compatible = socket_compatible(rule, &child, &parent);
            let connected = concrete_attachment_exists(recipe, child.endpoint, parent.endpoint);
            if !socket_compatible {
                issue_codes.push("incompatible_attachment_socket".to_owned());
            }
            if !connected {
                issue_codes.push("missing_required_attachment".to_owned());
                if rule.execution_policy == FamilyRuleExecutionPolicy::Required || rule.required {
                    issue_codes.push("disconnected_required_role".to_owned());
                }
            }
            covered_first.insert(child.endpoint);
            covered_second.insert(parent.endpoint);
            pairs.push(AttachmentPairConformance {
                parent: parent.endpoint,
                child: child.endpoint,
                socket_compatible,
                connected,
            });
        }
    }

    let mut coverage = AttachmentCoverageConformance {
        unmatched_first: first_endpoints
            .difference(&covered_first)
            .copied()
            .collect::<Vec<_>>(),
        unmatched_second: second_endpoints
            .difference(&covered_second)
            .copied()
            .collect::<Vec<_>>(),
        produced_pairs: !pairs.is_empty(),
    };
    coverage.unmatched_first.sort();
    coverage.unmatched_second.sort();
    if !coverage.unmatched_first.is_empty() || !coverage.unmatched_second.is_empty() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
        if rule.execution_policy == FamilyRuleExecutionPolicy::Required || rule.required {
            issue_codes.push("disconnected_required_role".to_owned());
        }
    }

    pairs.sort_by(|left, right| {
        left.child
            .cmp(&right.child)
            .then_with(|| left.parent.cmp(&right.parent))
            .then_with(|| left.socket_compatible.cmp(&right.socket_compatible))
            .then_with(|| left.connected.cmp(&right.connected))
    });
    issue_codes.sort();
    issue_codes.dedup();
    let status = attachment_status(&issue_codes);

    AttachmentConformance {
        rule_id: rule.id.clone(),
        from_role: rule.from_role.clone(),
        to_role: rule.to_role.clone(),
        policy: rule.execution_policy,
        pairs,
        coverage,
        status,
        issue_codes,
    }
}

fn resolve_attachment_port(
    recipe: &AssetRecipe,
    selected_fragments: &BTreeMap<&str, SelectedFragmentPorts<'_>>,
    role: &str,
    port_id: &str,
    issue_codes: &mut Vec<String>,
) -> Option<ResolvedAttachmentPort> {
    let Some(selection) = selected_fragments.get(role).copied() else {
        issue_codes.push("missing_attachment_endpoint".to_owned());
        return None;
    };
    let Some(port) = selection
        .fragment
        .exports
        .socket_ports
        .iter()
        .find(|port| port.id == port_id)
    else {
        issue_codes.push("missing_attachment_port".to_owned());
        return None;
    };
    let Some(socket) = selection.remap.sockets.get(&port.local_socket).copied() else {
        issue_codes.push("missing_attachment_socket_remap".to_owned());
        return None;
    };

    let mut occurrences = Vec::new();
    for (ordinal, local_root) in selection
        .fragment
        .exports
        .role_occurrence_roots
        .iter()
        .enumerate()
    {
        let Some(instance) = selection.remap.instances.get(local_root).copied() else {
            issue_codes.push("missing_attachment_instance_remap".to_owned());
            continue;
        };
        if !is_effectively_enabled(recipe, instance) {
            continue;
        }
        let Some(part) = recipe.instances.get(&instance) else {
            issue_codes.push("missing_attachment_endpoint".to_owned());
            continue;
        };
        let Some(definition) = recipe.definitions.get(&part.definition) else {
            issue_codes.push("missing_attachment_endpoint".to_owned());
            continue;
        };
        let Some(socket_spec) = definition.sockets.get(&socket) else {
            issue_codes.push("missing_attachment_socket".to_owned());
            continue;
        };
        occurrences.push(EndpointOccurrence {
            ordinal,
            endpoint: AttachmentEndpointConformance { instance, socket },
            port_compatibility_tags: port.compatibility_tags.iter().cloned().collect(),
            socket_compatibility_tags: socket_spec.tags.iter().cloned().collect(),
            frame_valid: socket_frame_is_valid(socket_spec),
        });
    }
    if occurrences.is_empty() {
        issue_codes.push("missing_attachment_endpoint".to_owned());
    }
    Some(ResolvedAttachmentPort { occurrences })
}

fn pairing_pairs(
    recipe: &AssetRecipe,
    binding_index: usize,
    pairing: &FragmentAttachmentPairing,
    children: &[EndpointOccurrence],
    parents: &[EndpointOccurrence],
    issue_codes: &mut Vec<String>,
) -> Vec<(EndpointOccurrence, EndpointOccurrence)> {
    match pairing {
        FragmentAttachmentPairing::AllPairs => all_pairs(children, parents, issue_codes),
        FragmentAttachmentPairing::ByOccurrenceIndex => {
            occurrence_index_pairs(children, parents, issue_codes)
        }
        FragmentAttachmentPairing::NearestOneToOne => {
            nearest_one_to_one_pairs(recipe, binding_index, children, parents, issue_codes)
        }
        FragmentAttachmentPairing::ExplicitOrdinalPairs(pairs) => {
            explicit_ordinal_pairs(children, parents, pairs, issue_codes)
        }
    }
}

fn all_pairs(
    children: &[EndpointOccurrence],
    parents: &[EndpointOccurrence],
    issue_codes: &mut Vec<String>,
) -> Vec<(EndpointOccurrence, EndpointOccurrence)> {
    if children.is_empty() || parents.is_empty() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
        return Vec::new();
    }
    if parents.len() > 1 {
        issue_codes.push("invalid_all_pairs_parent_attachment".to_owned());
        return Vec::new();
    }
    children
        .iter()
        .flat_map(|child| {
            parents
                .iter()
                .map(move |parent| (child.clone(), parent.clone()))
        })
        .collect()
}

fn occurrence_index_pairs(
    children: &[EndpointOccurrence],
    parents: &[EndpointOccurrence],
    issue_codes: &mut Vec<String>,
) -> Vec<(EndpointOccurrence, EndpointOccurrence)> {
    if children.is_empty() || parents.is_empty() || children.len() != parents.len() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
    }
    children
        .iter()
        .zip(parents.iter())
        .map(|(child, parent)| (child.clone(), parent.clone()))
        .collect()
}

fn nearest_one_to_one_pairs(
    recipe: &AssetRecipe,
    binding_index: usize,
    children: &[EndpointOccurrence],
    parents: &[EndpointOccurrence],
    issue_codes: &mut Vec<String>,
) -> Vec<(EndpointOccurrence, EndpointOccurrence)> {
    if children.is_empty() || parents.is_empty() || children.len() != parents.len() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
    }
    let mut candidates = Vec::new();
    for child in children {
        let Some(child_position) = instance_position(recipe, child.endpoint.instance) else {
            issue_codes.push("unresolved_attachment_endpoint_position".to_owned());
            return occurrence_index_pairs(children, parents, issue_codes);
        };
        for parent in parents {
            let Some(parent_position) = instance_position(recipe, parent.endpoint.instance) else {
                issue_codes.push("unresolved_attachment_endpoint_position".to_owned());
                return occurrence_index_pairs(children, parents, issue_codes);
            };
            candidates.push(NearestCandidate {
                distance_bits: distance_squared(child_position, parent_position).to_bits(),
                binding_index,
                child_ordinal: child.ordinal,
                parent_ordinal: parent.ordinal,
            });
        }
    }
    candidates.sort();
    let mut used_children = BTreeSet::new();
    let mut used_parents = BTreeSet::new();
    let mut selected = Vec::new();
    for candidate in candidates {
        if used_children.insert(candidate.child_ordinal)
            && used_parents.insert(candidate.parent_ordinal)
        {
            selected.push(candidate);
        }
    }
    selected.sort_by_key(|candidate| candidate.child_ordinal);
    selected
        .into_iter()
        .filter_map(|candidate| {
            let child = children
                .iter()
                .find(|occurrence| occurrence.ordinal == candidate.child_ordinal)?;
            let parent = parents
                .iter()
                .find(|occurrence| occurrence.ordinal == candidate.parent_ordinal)?;
            Some((child.clone(), parent.clone()))
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NearestCandidate {
    distance_bits: u32,
    binding_index: usize,
    child_ordinal: usize,
    parent_ordinal: usize,
}

fn explicit_ordinal_pairs(
    children: &[EndpointOccurrence],
    parents: &[EndpointOccurrence],
    pairs: &[(u32, u32)],
    issue_codes: &mut Vec<String>,
) -> Vec<(EndpointOccurrence, EndpointOccurrence)> {
    if children.is_empty() || parents.is_empty() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut covered_children = BTreeSet::new();
    let mut covered_parents = BTreeSet::new();
    for (child_ordinal, parent_ordinal) in pairs {
        let Some(child) = children.get(*child_ordinal as usize) else {
            issue_codes.push("attachment_pairing_ordinal_out_of_range".to_owned());
            continue;
        };
        let Some(parent) = parents.get(*parent_ordinal as usize) else {
            issue_codes.push("attachment_pairing_ordinal_out_of_range".to_owned());
            continue;
        };
        if !covered_children.insert(child.ordinal) || !covered_parents.insert(parent.ordinal) {
            issue_codes.push("duplicate_attachment_pairing_ordinal".to_owned());
        }
        result.push((child.clone(), parent.clone()));
    }
    if covered_children.len() != children.len() || covered_parents.len() != parents.len() {
        issue_codes.push("incomplete_attachment_pairing".to_owned());
    }
    result
}

fn socket_compatible(
    rule: &AttachmentRule,
    child: &EndpointOccurrence,
    parent: &EndpointOccurrence,
) -> bool {
    child.frame_valid
        && parent.frame_valid
        && if rule.compatibility_tags.is_empty() {
            tags_overlap(
                &child.port_compatibility_tags,
                &parent.port_compatibility_tags,
            ) && tags_overlap(
                &child.socket_compatibility_tags,
                &parent.socket_compatibility_tags,
            )
        } else {
            rule.compatibility_tags.iter().any(|tag| {
                child.port_compatibility_tags.contains(tag)
                    && parent.port_compatibility_tags.contains(tag)
                    && child.socket_compatibility_tags.contains(tag)
                    && parent.socket_compatibility_tags.contains(tag)
            })
        }
}

fn tags_overlap(left: &BTreeSet<String>, right: &BTreeSet<String>) -> bool {
    left.iter().any(|tag| right.contains(tag))
}

fn socket_frame_is_valid(socket: &SocketSpec) -> bool {
    frame_is_finite(&socket.local_frame)
}

fn frame_is_finite(frame: &Frame3) -> bool {
    frame.origin.iter().all(|value| value.is_finite())
        && frame.x_axis.iter().all(|value| value.is_finite())
        && frame.y_axis.iter().all(|value| value.is_finite())
        && frame.z_axis.iter().all(|value| value.is_finite())
}

fn concrete_attachment_exists(
    recipe: &AssetRecipe,
    child: AttachmentEndpointConformance,
    parent: AttachmentEndpointConformance,
) -> bool {
    let Some(child_instance) = recipe.instances.get(&child.instance) else {
        return false;
    };
    child_instance.parent == Some(parent.instance)
        && child_instance
            .attachment
            .as_ref()
            .is_some_and(|attachment| {
                attachment.parent_instance == parent.instance
                    && attachment.parent_socket == parent.socket
                    && attachment.child_socket == child.socket
            })
}

fn attachment_status(issue_codes: &[String]) -> ConformanceStatus {
    if issue_codes.is_empty() {
        return ConformanceStatus::Passed;
    }
    if issue_codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "missing_attachment_binding"
                | "missing_attachment_endpoint"
                | "missing_attachment_port"
                | "missing_attachment_socket"
                | "missing_required_attachment"
        )
    }) {
        ConformanceStatus::Missing
    } else {
        ConformanceStatus::Failed
    }
}

fn instance_position(recipe: &AssetRecipe, instance: PartInstanceId) -> Option<[f32; 3]> {
    let mut chain = Vec::new();
    let mut seen = BTreeSet::new();
    let mut current = Some(instance);
    while let Some(instance_id) = current {
        if !seen.insert(instance_id) {
            return None;
        }
        let part = recipe.instances.get(&instance_id)?;
        chain.push(part.local_transform.clone());
        current = part.parent;
    }
    let mut position = [0.0, 0.0, 0.0];
    for transform in chain.iter().rev() {
        position = transform.transform_point(position);
    }
    position
        .iter()
        .all(|value| value.is_finite())
        .then_some(position)
}

fn distance_squared(left: [f32; 3], right: [f32; 3]) -> f32 {
    (left[0] - right[0]).powi(2) + (left[1] - right[1]).powi(2) + (left[2] - right[2]).powi(2)
}
