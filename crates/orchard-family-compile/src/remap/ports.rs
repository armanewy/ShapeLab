//! Fragment port remap and attachment binding boundary.

use std::collections::{BTreeMap, BTreeSet};

use glam::{EulerRot, Quat};
use orchard_asset::{
    AssetRecipe, AttachmentMode, AttachmentSpec, PartInstanceId, SocketId, Transform3,
};
use orchard_family::{
    AssetFamilySchema, AttachmentRule, FamilyValidationIssue, FamilyValidationReport, PartRole,
};
use serde::{Deserialize, Serialize};

use crate::{
    FamilyImplementation, FragmentAttachmentPairing, FragmentSocketPort, RecipeFragment,
    RigidOffset,
};

use super::{FragmentRemap, FragmentRemapError};

/// A selected fragment after its local IDs have been remapped into a target recipe.
#[derive(Debug, Clone, Copy)]
pub struct SelectedFragmentPorts<'a> {
    /// Family role this selected fragment provides.
    pub role: &'a str,
    /// Source fragment contract.
    pub fragment: &'a RecipeFragment,
    /// Typed ID remap used when this fragment was merged.
    pub remap: &'a FragmentRemap,
}

/// Deterministic report for generated cross-fragment attachments.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FragmentAttachmentBindingReport {
    /// Applied attachment bindings in deterministic order.
    pub attachments: Vec<FragmentAttachmentApplication>,
}

/// One generated concrete attachment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FragmentAttachmentApplication {
    /// Index of the family implementation attachment binding that produced this row.
    pub binding_index: usize,
    /// Family attachment rule implemented by the binding.
    pub family_attachment_rule: String,
    /// Parent family role.
    pub parent_role: String,
    /// Child family role.
    pub child_role: String,
    /// Remapped child occurrence.
    pub child_instance: PartInstanceId,
    /// Remapped parent occurrence.
    pub parent_instance: PartInstanceId,
    /// Remapped child socket.
    pub child_socket: SocketId,
    /// Remapped parent socket.
    pub parent_socket: SocketId,
}

/// Validate and apply family fragment attachment bindings to a remapped recipe.
///
/// The binding direction is explicit: `parent_role`/`parent_port` receives the
/// concrete child occurrence from `child_role`/`child_port`.
pub fn apply_family_attachment_bindings(
    recipe: &mut AssetRecipe,
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    selected_fragments: &[SelectedFragmentPorts<'_>],
) -> Result<FragmentAttachmentBindingReport, FamilyValidationReport> {
    let mut report = FamilyValidationReport::default();
    let mut pending = resolve_family_attachment_bindings(
        recipe,
        family,
        family_impl,
        selected_fragments,
        &mut report,
    );
    pending.sort();
    pending.dedup();
    validate_pending_attachments(recipe, &pending, &mut report);
    if !report.is_valid() {
        return Err(report);
    }

    for attachment in &pending {
        let Some(child) = recipe.instances.get_mut(&attachment.child_instance) else {
            continue;
        };
        child.parent = Some(attachment.parent_instance);
        child.attachment = Some(attachment_spec(attachment));
    }
    let attached_children = pending
        .iter()
        .map(|attachment| attachment.child_instance)
        .collect::<BTreeSet<_>>();
    recipe
        .root_instances
        .retain(|root| !attached_children.contains(root));

    Ok(FragmentAttachmentBindingReport {
        attachments: pending
            .into_iter()
            .map(FragmentAttachmentApplication::from)
            .collect(),
    })
}

/// Validate that port remapping is intentionally routed through this module.
pub fn unsupported_port_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "ports".to_owned(),
        reason: reason.to_owned(),
    }
}

#[derive(Debug, Clone, Copy)]
struct SelectedFragmentIndex<'a> {
    selection: SelectedFragmentPorts<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PortOccurrence {
    ordinal: usize,
    local_root: PartInstanceId,
    instance: PartInstanceId,
    socket: SocketId,
}

#[derive(Debug, Clone)]
struct ResolvedPort {
    compatibility_tags: Vec<String>,
    occurrences: Vec<PortOccurrence>,
}

#[derive(Debug, Clone)]
struct PendingAttachment {
    binding_index: usize,
    family_attachment_rule: String,
    parent_role: String,
    child_role: String,
    child_instance: PartInstanceId,
    parent_instance: PartInstanceId,
    child_socket: SocketId,
    parent_socket: SocketId,
    rigid_offset: RigidOffset,
    attachment_mode: AttachmentMode,
}

impl PartialEq for PendingAttachment {
    fn eq(&self, other: &Self) -> bool {
        self.binding_index == other.binding_index
            && self.family_attachment_rule == other.family_attachment_rule
            && self.parent_role == other.parent_role
            && self.child_role == other.child_role
            && self.child_instance == other.child_instance
            && self.parent_instance == other.parent_instance
            && self.child_socket == other.child_socket
            && self.parent_socket == other.parent_socket
            && self.rigid_offset == other.rigid_offset
            && self.attachment_mode == other.attachment_mode
    }
}

impl Eq for PendingAttachment {}

impl PartialOrd for PendingAttachment {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingAttachment {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.binding_index
            .cmp(&other.binding_index)
            .then_with(|| self.child_instance.cmp(&other.child_instance))
            .then_with(|| self.parent_instance.cmp(&other.parent_instance))
            .then_with(|| self.child_socket.cmp(&other.child_socket))
            .then_with(|| self.parent_socket.cmp(&other.parent_socket))
            .then_with(|| cmp_rigid_offset(self.rigid_offset, other.rigid_offset))
            .then_with(|| self.attachment_mode.cmp(&other.attachment_mode))
    }
}

impl From<PendingAttachment> for FragmentAttachmentApplication {
    fn from(value: PendingAttachment) -> Self {
        Self {
            binding_index: value.binding_index,
            family_attachment_rule: value.family_attachment_rule,
            parent_role: value.parent_role,
            child_role: value.child_role,
            child_instance: value.child_instance,
            parent_instance: value.parent_instance,
            child_socket: value.child_socket,
            parent_socket: value.parent_socket,
        }
    }
}

fn resolve_family_attachment_bindings(
    recipe: &AssetRecipe,
    family: &AssetFamilySchema,
    family_impl: &FamilyImplementation,
    selected_fragments: &[SelectedFragmentPorts<'_>],
    report: &mut FamilyValidationReport,
) -> Vec<PendingAttachment> {
    let roles = family
        .part_roles
        .iter()
        .map(|role| (role.id.as_str(), role))
        .collect::<BTreeMap<_, _>>();
    let rules = family
        .attachment_rules
        .iter()
        .map(|rule| (rule.id.as_str(), rule))
        .collect::<BTreeMap<_, _>>();
    let selected = selected_fragment_index(selected_fragments, report);
    let mut pending = Vec::new();

    for (index, binding) in family_impl.attachment_bindings.iter().enumerate() {
        let subject = format!("family_implementation.attachment_bindings.{index}");
        validate_identifier(
            report,
            Some(format!("{subject}.parent_port")),
            &binding.parent_port,
            "invalid_fragment_attachment_port",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.child_port")),
            &binding.child_port,
            "invalid_fragment_attachment_port",
        );
        validate_rigid_offset(report, &subject, &binding.rigid_offset);
        if binding.attachment_mode != AttachmentMode::RigidSeparate {
            push_issue(
                report,
                Some(format!("{subject}.attachment_mode")),
                "unsupported_fragment_attachment_mode",
                "Fragment attachment bindings currently support only RigidSeparate.",
            );
        }

        let parent_role = validate_role(
            report,
            &roles,
            &subject,
            "parent_role",
            &binding.parent_role,
            "unknown_fragment_attachment_parent_role",
        );
        let child_role = validate_role(
            report,
            &roles,
            &subject,
            "child_role",
            &binding.child_role,
            "unknown_fragment_attachment_child_role",
        );
        let rule = validate_rule(report, &rules, &subject, binding);
        if let (Some(rule), Some(parent_role), Some(child_role)) = (rule, parent_role, child_role) {
            validate_rule_roles(report, &subject, rule, parent_role, child_role);
        }

        let parent = resolve_selected_role(
            report,
            &selected,
            &subject,
            "parent_role",
            &binding.parent_role,
        );
        let child = resolve_selected_role(
            report,
            &selected,
            &subject,
            "child_role",
            &binding.child_role,
        );
        let (Some(parent), Some(child)) = (parent, child) else {
            continue;
        };
        let parent_port = resolve_socket_port(
            recipe,
            parent.selection,
            &binding.parent_port,
            &format!("{subject}.parent_port"),
            report,
        );
        let child_port = resolve_socket_port(
            recipe,
            child.selection,
            &binding.child_port,
            &format!("{subject}.child_port"),
            report,
        );
        let (Some(parent_port), Some(child_port)) = (parent_port, child_port) else {
            continue;
        };
        if let Some(rule) = rule {
            validate_compatibility(
                report,
                &subject,
                rule,
                &parent_port.compatibility_tags,
                &child_port.compatibility_tags,
            );
        }
        let pairs = pairing_pairs(
            recipe,
            index,
            &binding.pairing,
            &child_port.occurrences,
            &parent_port.occurrences,
            &subject,
            report,
        );
        pending.extend(pairs.into_iter().map(|(child, parent)| PendingAttachment {
            binding_index: index,
            family_attachment_rule: binding.family_attachment_rule.clone(),
            parent_role: binding.parent_role.clone(),
            child_role: binding.child_role.clone(),
            child_instance: child.instance,
            parent_instance: parent.instance,
            child_socket: child.socket,
            parent_socket: parent.socket,
            rigid_offset: binding.rigid_offset,
            attachment_mode: binding.attachment_mode,
        }));
    }

    pending
}

fn selected_fragment_index<'a>(
    selected_fragments: &'a [SelectedFragmentPorts<'a>],
    report: &mut FamilyValidationReport,
) -> BTreeMap<&'a str, SelectedFragmentIndex<'a>> {
    let mut selected = BTreeMap::new();
    for selection in selected_fragments {
        if selection.fragment.provided_role != selection.role {
            push_issue(
                report,
                Some(format!("selected_fragments.{}", selection.fragment.id)),
                "selected_fragment_role_mismatch",
                "Selected fragment role must match the fragment's provided role.",
            );
        }
        if selected
            .insert(
                selection.role,
                SelectedFragmentIndex {
                    selection: *selection,
                },
            )
            .is_some()
        {
            push_issue(
                report,
                Some(format!("selected_fragments.{}", selection.role)),
                "duplicate_selected_fragment_role",
                "Only one remapped provider fragment can be selected for each role.",
            );
        }
    }
    selected
}

fn validate_role<'a>(
    report: &mut FamilyValidationReport,
    roles: &'a BTreeMap<&str, &PartRole>,
    subject: &str,
    field: &str,
    role: &str,
    code: &'static str,
) -> Option<&'a PartRole> {
    roles.get(role).copied().or_else(|| {
        push_issue(
            report,
            Some(format!("{subject}.{field}")),
            code,
            "Fragment attachment bindings must reference declared family roles.",
        );
        None
    })
}

fn validate_rule<'a>(
    report: &mut FamilyValidationReport,
    rules: &'a BTreeMap<&str, &AttachmentRule>,
    subject: &str,
    binding: &crate::FragmentAttachmentBinding,
) -> Option<&'a AttachmentRule> {
    rules
        .get(binding.family_attachment_rule.as_str())
        .copied()
        .or_else(|| {
            push_issue(
                report,
                Some(format!("{subject}.family_attachment_rule")),
                "unknown_fragment_attachment_rule",
                "Fragment attachment bindings must implement a declared family attachment rule.",
            );
            None
        })
}

fn validate_rule_roles(
    report: &mut FamilyValidationReport,
    subject: &str,
    rule: &AttachmentRule,
    parent_role: &PartRole,
    child_role: &PartRole,
) {
    if rule.from_role != child_role.id || rule.to_role != parent_role.id {
        push_issue(
            report,
            Some(format!("{subject}.family_attachment_rule")),
            "fragment_attachment_rule_role_mismatch",
            "Fragment attachment binding child/parent roles must match the family attachment rule direction.",
        );
    }
}

fn resolve_selected_role<'a>(
    report: &mut FamilyValidationReport,
    selected: &'a BTreeMap<&str, SelectedFragmentIndex<'a>>,
    subject: &str,
    field: &str,
    role: &str,
) -> Option<&'a SelectedFragmentIndex<'a>> {
    selected.get(role).or_else(|| {
        push_issue(
            report,
            Some(format!("{subject}.{field}")),
            "unselected_fragment_attachment_role",
            "Fragment attachment bindings can only target roles selected for this instantiation.",
        );
        None
    })
}

fn resolve_socket_port(
    recipe: &AssetRecipe,
    selection: SelectedFragmentPorts<'_>,
    port_id: &str,
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Option<ResolvedPort> {
    let Some(port) = selection
        .fragment
        .exports
        .socket_ports
        .iter()
        .find(|port| port.id == port_id)
    else {
        let code = if selection
            .fragment
            .exports
            .surface_ports
            .iter()
            .any(|port| port.id == port_id)
        {
            "unsupported_fragment_attachment_surface_port"
        } else {
            "unknown_fragment_attachment_port"
        };
        push_issue(
            report,
            Some(subject.to_owned()),
            code,
            "Fragment attachment bindings must reference exported socket ports on the selected fragment.",
        );
        return None;
    };
    if !selection
        .fragment
        .exports
        .role_occurrence_roots
        .contains(&port.local_occurrence_root)
    {
        push_issue(
            report,
            Some(subject.to_owned()),
            "fragment_attachment_port_not_on_role_occurrence",
            "Fragment attachment ports must be exported from a role occurrence root.",
        );
    }
    let Some(socket) = selection.remap.sockets.get(&port.local_socket).copied() else {
        push_issue(
            report,
            Some(subject.to_owned()),
            "missing_fragment_socket_remap",
            "Fragment attachment socket ports must have a typed socket remap.",
        );
        return None;
    };

    let mut resolver = PortOccurrenceResolver {
        recipe,
        selection,
        port,
        socket,
        subject,
        report,
    };
    let occurrences = selection
        .fragment
        .exports
        .role_occurrence_roots
        .iter()
        .enumerate()
        .filter_map(|(ordinal, local_root)| resolver.resolve(ordinal, *local_root))
        .collect::<Vec<_>>();

    Some(ResolvedPort {
        compatibility_tags: port.compatibility_tags.clone(),
        occurrences,
    })
}

struct PortOccurrenceResolver<'a, 'b> {
    recipe: &'a AssetRecipe,
    selection: SelectedFragmentPorts<'a>,
    port: &'a FragmentSocketPort,
    socket: SocketId,
    subject: &'a str,
    report: &'b mut FamilyValidationReport,
}

impl PortOccurrenceResolver<'_, '_> {
    fn resolve(&mut self, ordinal: usize, local_root: PartInstanceId) -> Option<PortOccurrence> {
        let Some(local_instance) = self.selection.fragment.recipe.instances.get(&local_root) else {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "unknown_fragment_attachment_occurrence",
                "Role occurrence roots must exist inside the source fragment recipe.",
            );
            return None;
        };
        if local_instance.generated_by.is_some() {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "unsupported_fragment_attachment_generated_occurrence",
                "Fragment attachment ports support explicit role occurrence roots only; generated array or mirror occurrence expansion is not supported in this milestone.",
            );
            return None;
        }
        let Some(local_definition) = self
            .selection
            .fragment
            .recipe
            .definitions
            .get(&local_instance.definition)
        else {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "fragment_attachment_external_definition",
                "Role occurrence roots must reference definitions inside the source fragment.",
            );
            return None;
        };
        if !local_definition
            .sockets
            .contains_key(&self.port.local_socket)
        {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "missing_fragment_attachment_occurrence_socket",
                "Every role occurrence paired by an attachment binding must expose the selected socket.",
            );
            return None;
        }
        let Some(instance) = self.selection.remap.instances.get(&local_root).copied() else {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "missing_fragment_instance_remap",
                "Fragment attachment occurrence roots must have typed instance remaps.",
            );
            return None;
        };
        let Some(remapped_instance) = self.recipe.instances.get(&instance) else {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "unknown_remapped_fragment_instance",
                "Fragment attachment occurrence roots must resolve to instances in the target recipe.",
            );
            return None;
        };
        let Some(remapped_definition) = self.recipe.definitions.get(&remapped_instance.definition)
        else {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "unknown_remapped_fragment_definition",
                "Fragment attachment occurrence roots must resolve to definitions in the target recipe.",
            );
            return None;
        };
        if !remapped_definition.sockets.contains_key(&self.socket) {
            push_issue(
                self.report,
                Some(self.subject.to_owned()),
                "unknown_remapped_fragment_socket",
                "Fragment attachment sockets must resolve to sockets in the target recipe.",
            );
            return None;
        }
        Some(PortOccurrence {
            ordinal,
            local_root,
            instance,
            socket: self.socket,
        })
    }
}

fn validate_compatibility(
    report: &mut FamilyValidationReport,
    subject: &str,
    rule: &AttachmentRule,
    child_tags: &[String],
    parent_tags: &[String],
) {
    let compatible = if rule.compatibility_tags.is_empty() {
        tags_overlap(child_tags, parent_tags)
    } else {
        rule.compatibility_tags
            .iter()
            .any(|tag| child_tags.contains(tag) && parent_tags.contains(tag))
    };
    if !compatible {
        push_issue(
            report,
            Some(format!("{subject}.family_attachment_rule")),
            "fragment_attachment_tag_mismatch",
            "Attachment binding ports must share a required compatibility tag.",
        );
    }
}

fn tags_overlap(left: &[String], right: &[String]) -> bool {
    left.iter().any(|tag| right.contains(tag))
}

fn pairing_pairs(
    recipe: &AssetRecipe,
    binding_index: usize,
    pairing: &FragmentAttachmentPairing,
    children: &[PortOccurrence],
    parents: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    match pairing {
        FragmentAttachmentPairing::AllPairs => all_pairs(children, parents, subject, report),
        FragmentAttachmentPairing::ByOccurrenceIndex => {
            occurrence_index_pairs(children, parents, subject, report)
        }
        FragmentAttachmentPairing::NearestOneToOne => {
            nearest_one_to_one_pairs(recipe, binding_index, children, parents, subject, report)
        }
        FragmentAttachmentPairing::ExplicitOrdinalPairs(pairs) => {
            explicit_ordinal_pairs(children, parents, pairs, subject, report)
        }
    }
}

fn all_pairs(
    children: &[PortOccurrence],
    parents: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if children.is_empty() || parents.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    if parents.len() > 1 {
        push_issue(
            report,
            Some(format!("{subject}.pairing")),
            "fragment_attachment_all_pairs_multiple_parents",
            "AllPairs is invalid for parent/child attachments when it would give one child occurrence multiple parents.",
        );
        return Vec::new();
    }
    children
        .iter()
        .flat_map(|child| parents.iter().map(move |parent| (*child, *parent)))
        .collect()
}

fn occurrence_index_pairs(
    children: &[PortOccurrence],
    parents: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if children.len() != parents.len() || children.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    children
        .iter()
        .zip(parents.iter())
        .map(|(child, parent)| (*child, *parent))
        .collect()
}

fn nearest_one_to_one_pairs(
    recipe: &AssetRecipe,
    binding_index: usize,
    children: &[PortOccurrence],
    parents: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if children.len() != parents.len() || children.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for child in children {
        let Some(child_position) = instance_position(recipe, child.instance) else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "unresolved_fragment_attachment_position",
                "NearestOneToOne attachment pairing requires resolvable finite occurrence positions.",
            );
            return Vec::new();
        };
        for parent in parents {
            let Some(parent_position) = instance_position(recipe, parent.instance) else {
                push_issue(
                    report,
                    Some(format!("{subject}.pairing")),
                    "unresolved_fragment_attachment_position",
                    "NearestOneToOne attachment pairing requires resolvable finite occurrence positions.",
                );
                return Vec::new();
            };
            candidates.push(NearestCandidate {
                distance_bits: distance_squared(child_position, parent_position).to_bits(),
                binding_index,
                child: *child,
                parent: *parent,
            });
        }
    }
    candidates.sort();
    let mut used_children = BTreeSet::new();
    let mut used_parents = BTreeSet::new();
    let mut pairs = Vec::new();
    for candidate in candidates {
        if used_children.insert(candidate.child.ordinal)
            && used_parents.insert(candidate.parent.ordinal)
        {
            pairs.push((candidate.child, candidate.parent));
        }
    }
    if pairs.len() != children.len() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    pairs.sort_by_key(|left| left.0.ordinal);
    pairs
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct NearestCandidate {
    distance_bits: u32,
    binding_index: usize,
    child: PortOccurrence,
    parent: PortOccurrence,
}

impl PartialOrd for PortOccurrence {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PortOccurrence {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.ordinal
            .cmp(&other.ordinal)
            .then_with(|| self.local_root.cmp(&other.local_root))
            .then_with(|| self.instance.cmp(&other.instance))
            .then_with(|| self.socket.cmp(&other.socket))
    }
}

fn explicit_ordinal_pairs(
    children: &[PortOccurrence],
    parents: &[PortOccurrence],
    pairs: &[(u32, u32)],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if children.is_empty() || parents.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut covered_children = BTreeSet::new();
    let mut covered_parents = BTreeSet::new();
    for (child_ordinal, parent_ordinal) in pairs {
        let Some(child) = children.get(*child_ordinal as usize).copied() else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "fragment_attachment_pairing_ordinal_out_of_range",
                "Explicit attachment child ordinals must target exported role occurrences.",
            );
            continue;
        };
        let Some(parent) = parents.get(*parent_ordinal as usize).copied() else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "fragment_attachment_pairing_ordinal_out_of_range",
                "Explicit attachment parent ordinals must target exported role occurrences.",
            );
            continue;
        };
        if !covered_children.insert(child.ordinal) || !covered_parents.insert(parent.ordinal) {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "duplicate_fragment_attachment_pairing_ordinal",
                "Explicit attachment ordinals must be one-to-one.",
            );
        }
        result.push((child, parent));
    }
    if covered_children.len() != children.len() || covered_parents.len() != parents.len() {
        push_incomplete_pairing_issue(report, subject);
    }
    result
}

fn push_incomplete_pairing_issue(report: &mut FamilyValidationReport, subject: &str) {
    push_issue(
        report,
        Some(format!("{subject}.pairing")),
        "incomplete_fragment_attachment_pairing",
        "Attachment pairing must cover every child and parent occurrence.",
    );
}

fn validate_pending_attachments(
    recipe: &AssetRecipe,
    pending: &[PendingAttachment],
    report: &mut FamilyValidationReport,
) {
    let mut by_child = BTreeMap::<PartInstanceId, PendingAttachment>::new();
    for attachment in pending {
        if attachment.child_instance == attachment.parent_instance {
            push_issue(
                report,
                Some(format!(
                    "family_implementation.attachment_bindings.{}",
                    attachment.binding_index
                )),
                "fragment_attachment_cycle",
                "Fragment attachment bindings must not attach an occurrence to itself.",
            );
            continue;
        }
        let Some(child) = recipe.instances.get(&attachment.child_instance) else {
            continue;
        };
        if let Some(existing_parent) = child.parent
            && existing_parent != attachment.parent_instance
        {
            push_duplicate_attachment_issue(report, attachment);
        }
        if let Some(existing) = &child.attachment
            && existing != &attachment_spec(attachment)
        {
            push_duplicate_attachment_issue(report, attachment);
        }
        if let Some(existing) = by_child.insert(attachment.child_instance, attachment.clone())
            && existing != *attachment
        {
            push_duplicate_attachment_issue(report, attachment);
        }
    }
    validate_attachment_cycles(recipe, &by_child, report);
}

fn validate_attachment_cycles(
    recipe: &AssetRecipe,
    pending_by_child: &BTreeMap<PartInstanceId, PendingAttachment>,
    report: &mut FamilyValidationReport,
) {
    for attachment in pending_by_child.values() {
        let mut seen = BTreeSet::new();
        let mut current = Some(attachment.parent_instance);
        while let Some(instance) = current {
            if instance == attachment.child_instance {
                push_issue(
                    report,
                    Some(format!(
                        "family_implementation.attachment_bindings.{}",
                        attachment.binding_index
                    )),
                    "fragment_attachment_cycle",
                    "Fragment attachment bindings must not create parent or attachment cycles.",
                );
                break;
            }
            if !seen.insert(instance) {
                break;
            }
            current = pending_by_child
                .get(&instance)
                .map(|pending| pending.parent_instance)
                .or_else(|| {
                    recipe.instances.get(&instance).and_then(|part| {
                        part.attachment
                            .as_ref()
                            .map(|attachment| attachment.parent_instance)
                            .or(part.parent)
                    })
                });
        }
    }
}

fn push_duplicate_attachment_issue(
    report: &mut FamilyValidationReport,
    attachment: &PendingAttachment,
) {
    push_issue(
        report,
        Some(format!(
            "family_implementation.attachment_bindings.{}",
            attachment.binding_index
        )),
        "duplicate_fragment_parent_attachment",
        "Each fragment occurrence can receive only one compatible parent attachment.",
    );
}

fn attachment_spec(attachment: &PendingAttachment) -> AttachmentSpec {
    AttachmentSpec {
        parent_instance: attachment.parent_instance,
        parent_socket: attachment.parent_socket,
        child_socket: attachment.child_socket,
        local_offset: Transform3 {
            translation: attachment.rigid_offset.translation,
            rotation_degrees: quaternion_to_euler_degrees(attachment.rigid_offset.rotation),
            ..Transform3::default()
        },
        mode: attachment.attachment_mode,
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

fn cmp_rigid_offset(left: RigidOffset, right: RigidOffset) -> std::cmp::Ordering {
    cmp_f32_array(left.translation, right.translation)
        .then_with(|| cmp_f32_array4(left.rotation, right.rotation))
}

fn cmp_f32_array(left: [f32; 3], right: [f32; 3]) -> std::cmp::Ordering {
    left[0]
        .total_cmp(&right[0])
        .then_with(|| left[1].total_cmp(&right[1]))
        .then_with(|| left[2].total_cmp(&right[2]))
}

fn cmp_f32_array4(left: [f32; 4], right: [f32; 4]) -> std::cmp::Ordering {
    left[0]
        .total_cmp(&right[0])
        .then_with(|| left[1].total_cmp(&right[1]))
        .then_with(|| left[2].total_cmp(&right[2]))
        .then_with(|| left[3].total_cmp(&right[3]))
}

fn validate_rigid_offset(report: &mut FamilyValidationReport, subject: &str, offset: &RigidOffset) {
    if offset.translation.iter().any(|value| !value.is_finite()) {
        push_issue(
            report,
            Some(format!("{subject}.rigid_offset.translation")),
            "non_finite_fragment_attachment_translation",
            "Fragment attachment translations must be finite.",
        );
    }
    if offset.rotation.iter().any(|value| !value.is_finite()) {
        push_issue(
            report,
            Some(format!("{subject}.rigid_offset.rotation")),
            "non_finite_fragment_attachment_rotation",
            "Fragment attachment rotations must be finite.",
        );
        return;
    }
    let length_squared = offset
        .rotation
        .iter()
        .map(|value| value * value)
        .sum::<f32>();
    if (length_squared - 1.0).abs() > 1.0e-4 {
        push_issue(
            report,
            Some(format!("{subject}.rigid_offset.rotation")),
            "non_unit_fragment_attachment_rotation",
            "Fragment attachment rotations must be normalized quaternions.",
        );
    }
    if !is_canonical_quaternion(offset.rotation) {
        push_issue(
            report,
            Some(format!("{subject}.rigid_offset.rotation")),
            "non_canonical_fragment_attachment_rotation",
            "Fragment attachment rotations must use the canonical quaternion sign.",
        );
    }
}

fn is_canonical_quaternion(rotation: [f32; 4]) -> bool {
    rotation[3] > 0.0
        || (rotation[3] == 0.0
            && (rotation[2] > 0.0
                || (rotation[2] == 0.0
                    && (rotation[1] > 0.0 || (rotation[1] == 0.0 && rotation[0] >= 0.0)))))
}

fn quaternion_to_euler_degrees(rotation: [f32; 4]) -> [f32; 3] {
    let (x, y, z) =
        Quat::from_xyzw(rotation[0], rotation[1], rotation[2], rotation[3]).to_euler(EulerRot::XYZ);
    [x.to_degrees(), y.to_degrees(), z.to_degrees()]
}

fn validate_identifier(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    value: &str,
    code: &'static str,
) {
    if !stable_identifier_is_valid(value) {
        push_issue(
            report,
            subject,
            code,
            "Stable identifiers must start with a lowercase ASCII letter, end with an alphanumeric character, and use non-repeated lowercase ASCII letters, digits, `_`, `-`, `.`, or `:` separators.",
        );
    }
}

fn stable_identifier_is_valid(value: &str) -> bool {
    if value.trim() != value || value.is_empty() || value == "." || value == ".." {
        return false;
    }
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut previous_was_separator = false;
    let mut last = first;
    for character in std::iter::once(first).chain(chars) {
        if !is_identifier_char(character) {
            return false;
        }
        let is_separator = is_identifier_separator(character);
        if is_separator && previous_was_separator {
            return false;
        }
        previous_was_separator = is_separator;
        last = character;
    }
    last.is_ascii_lowercase() || last.is_ascii_digit()
}

fn is_identifier_char(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '_' | '-' | '.' | ':')
}

fn is_identifier_separator(character: char) -> bool {
    matches!(character, '_' | '-' | '.' | ':')
}

fn push_issue(
    report: &mut FamilyValidationReport,
    subject: Option<impl Into<String>>,
    code: impl Into<String>,
    message: impl Into<String>,
) {
    report.issues.push(FamilyValidationIssue {
        subject: subject.map(Into::into),
        code: code.into(),
        message: message.into(),
    });
}
