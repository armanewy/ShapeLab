//! Fragment port remap and attachment binding boundary.

use std::collections::{BTreeMap, BTreeSet};

use shape_asset::{
    AssetRecipe, AttachmentMode, AttachmentSpec, PartInstanceId, SocketId, Transform3,
};
use shape_family::{
    AssetFamilySchema, AttachmentRule, FamilyValidationIssue, FamilyValidationReport, PartRole,
};

use crate::{FamilyImplementation, FragmentAttachmentPairing, FragmentSocketPort, RecipeFragment};

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FragmentAttachmentBindingReport {
    /// Applied attachment bindings in deterministic order.
    pub attachments: Vec<FragmentAttachmentApplication>,
}

/// One generated concrete attachment.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FragmentAttachmentApplication {
    /// Index of the family implementation attachment binding that produced this row.
    pub binding_index: usize,
    /// Family attachment rule implemented by the binding.
    pub family_attachment_rule: String,
    /// Source family role. This is the attached child side.
    pub source_role: String,
    /// Destination family role. This is the parent side.
    pub destination_role: String,
    /// Remapped child/source occurrence.
    pub child_instance: PartInstanceId,
    /// Remapped parent/destination occurrence.
    pub parent_instance: PartInstanceId,
    /// Remapped child/source socket.
    pub child_socket: SocketId,
    /// Remapped parent/destination socket.
    pub parent_socket: SocketId,
}

/// Validate and apply family fragment attachment bindings to a remapped recipe.
///
/// The binding direction is source role/port as the child side and destination
/// role/port as the parent side, matching [`shape_asset::AttachmentSpec`].
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
    source_role: String,
    destination_role: String,
    child_instance: PartInstanceId,
    parent_instance: PartInstanceId,
    child_socket: SocketId,
    parent_socket: SocketId,
    offset: [f32; 3],
    attachment_mode: AttachmentMode,
}

impl PartialEq for PendingAttachment {
    fn eq(&self, other: &Self) -> bool {
        self.binding_index == other.binding_index
            && self.family_attachment_rule == other.family_attachment_rule
            && self.source_role == other.source_role
            && self.destination_role == other.destination_role
            && self.child_instance == other.child_instance
            && self.parent_instance == other.parent_instance
            && self.child_socket == other.child_socket
            && self.parent_socket == other.parent_socket
            && self.offset == other.offset
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
            .then_with(|| cmp_f32_array(self.offset, other.offset))
            .then_with(|| self.attachment_mode.cmp(&other.attachment_mode))
    }
}

impl From<PendingAttachment> for FragmentAttachmentApplication {
    fn from(value: PendingAttachment) -> Self {
        Self {
            binding_index: value.binding_index,
            family_attachment_rule: value.family_attachment_rule,
            source_role: value.source_role,
            destination_role: value.destination_role,
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
            Some(format!("{subject}.source_port")),
            &binding.source_port,
            "invalid_fragment_attachment_port",
        );
        validate_identifier(
            report,
            Some(format!("{subject}.destination_port")),
            &binding.destination_port,
            "invalid_fragment_attachment_port",
        );
        if binding.offset.iter().any(|value| !value.is_finite()) {
            push_issue(
                report,
                Some(format!("{subject}.offset")),
                "non_finite_fragment_attachment_offset",
                "Fragment attachment offsets must be finite.",
            );
        }
        if binding.attachment_mode != AttachmentMode::RigidSeparate {
            push_issue(
                report,
                Some(format!("{subject}.attachment_mode")),
                "unsupported_fragment_attachment_mode",
                "Fragment attachment bindings currently support only RigidSeparate.",
            );
        }

        let source_role = validate_role(
            report,
            &roles,
            &subject,
            "source_role",
            &binding.source_role,
            "unknown_fragment_attachment_source_role",
        );
        let destination_role = validate_role(
            report,
            &roles,
            &subject,
            "destination_role",
            &binding.destination_role,
            "unknown_fragment_attachment_destination_role",
        );
        let rule = validate_rule(report, &rules, &subject, binding);
        if let (Some(rule), Some(source_role), Some(destination_role)) =
            (rule, source_role, destination_role)
        {
            validate_rule_roles(report, &subject, rule, source_role, destination_role);
        }

        let source = resolve_selected_role(
            report,
            &selected,
            &subject,
            "source_role",
            &binding.source_role,
        );
        let destination = resolve_selected_role(
            report,
            &selected,
            &subject,
            "destination_role",
            &binding.destination_role,
        );
        let (Some(source), Some(destination)) = (source, destination) else {
            continue;
        };
        let source_port = resolve_socket_port(
            recipe,
            source.selection,
            &binding.source_port,
            &format!("{subject}.source_port"),
            report,
        );
        let destination_port = resolve_socket_port(
            recipe,
            destination.selection,
            &binding.destination_port,
            &format!("{subject}.destination_port"),
            report,
        );
        let (Some(source_port), Some(destination_port)) = (source_port, destination_port) else {
            continue;
        };
        if let Some(rule) = rule {
            validate_compatibility(
                report,
                &subject,
                rule,
                &source_port.compatibility_tags,
                &destination_port.compatibility_tags,
            );
        }
        let pairs = pairing_pairs(
            recipe,
            index,
            &binding.pairing,
            &source_port.occurrences,
            &destination_port.occurrences,
            &subject,
            report,
        );
        pending.extend(
            pairs
                .into_iter()
                .map(|(source, destination)| PendingAttachment {
                    binding_index: index,
                    family_attachment_rule: binding.family_attachment_rule.clone(),
                    source_role: binding.source_role.clone(),
                    destination_role: binding.destination_role.clone(),
                    child_instance: source.instance,
                    parent_instance: destination.instance,
                    child_socket: source.socket,
                    parent_socket: destination.socket,
                    offset: binding.offset,
                    attachment_mode: binding.attachment_mode,
                }),
        );
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
    source_role: &PartRole,
    destination_role: &PartRole,
) {
    if rule.from_role != source_role.id || rule.to_role != destination_role.id {
        push_issue(
            report,
            Some(format!("{subject}.family_attachment_rule")),
            "fragment_attachment_rule_role_mismatch",
            "Fragment attachment binding roles must match the family attachment rule direction.",
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
    source_tags: &[String],
    destination_tags: &[String],
) {
    let compatible = if rule.compatibility_tags.is_empty() {
        tags_overlap(source_tags, destination_tags)
    } else {
        rule.compatibility_tags
            .iter()
            .any(|tag| source_tags.contains(tag) && destination_tags.contains(tag))
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
    source: &[PortOccurrence],
    destination: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    match pairing {
        FragmentAttachmentPairing::AllPairs => all_pairs(source, destination, subject, report),
        FragmentAttachmentPairing::ByOccurrenceIndex => {
            occurrence_index_pairs(source, destination, subject, report)
        }
        FragmentAttachmentPairing::NearestOneToOne => {
            nearest_one_to_one_pairs(recipe, binding_index, source, destination, subject, report)
        }
        FragmentAttachmentPairing::ExplicitOrdinalPairs(pairs) => {
            explicit_ordinal_pairs(source, destination, pairs, subject, report)
        }
    }
}

fn all_pairs(
    source: &[PortOccurrence],
    destination: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if source.is_empty() || destination.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    source
        .iter()
        .flat_map(|source| {
            destination
                .iter()
                .map(move |destination| (*source, *destination))
        })
        .collect()
}

fn occurrence_index_pairs(
    source: &[PortOccurrence],
    destination: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if source.len() != destination.len() || source.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    source
        .iter()
        .zip(destination.iter())
        .map(|(source, destination)| (*source, *destination))
        .collect()
}

fn nearest_one_to_one_pairs(
    recipe: &AssetRecipe,
    binding_index: usize,
    source: &[PortOccurrence],
    destination: &[PortOccurrence],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if source.len() != destination.len() || source.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    let mut candidates = Vec::new();
    for source in source {
        let Some(source_position) = instance_position(recipe, source.instance) else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "unresolved_fragment_attachment_position",
                "NearestOneToOne attachment pairing requires resolvable finite occurrence positions.",
            );
            return Vec::new();
        };
        for destination in destination {
            let Some(destination_position) = instance_position(recipe, destination.instance) else {
                push_issue(
                    report,
                    Some(format!("{subject}.pairing")),
                    "unresolved_fragment_attachment_position",
                    "NearestOneToOne attachment pairing requires resolvable finite occurrence positions.",
                );
                return Vec::new();
            };
            candidates.push(NearestCandidate {
                distance_bits: distance_squared(source_position, destination_position).to_bits(),
                binding_index,
                source: *source,
                destination: *destination,
            });
        }
    }
    candidates.sort();
    let mut used_sources = BTreeSet::new();
    let mut used_destinations = BTreeSet::new();
    let mut pairs = Vec::new();
    for candidate in candidates {
        if used_sources.insert(candidate.source.ordinal)
            && used_destinations.insert(candidate.destination.ordinal)
        {
            pairs.push((candidate.source, candidate.destination));
        }
    }
    if pairs.len() != source.len() {
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
    source: PortOccurrence,
    destination: PortOccurrence,
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
    source: &[PortOccurrence],
    destination: &[PortOccurrence],
    pairs: &[(u32, u32)],
    subject: &str,
    report: &mut FamilyValidationReport,
) -> Vec<(PortOccurrence, PortOccurrence)> {
    if source.is_empty() || destination.is_empty() {
        push_incomplete_pairing_issue(report, subject);
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut covered_sources = BTreeSet::new();
    let mut covered_destinations = BTreeSet::new();
    for (source_ordinal, destination_ordinal) in pairs {
        let Some(source) = source.get(*source_ordinal as usize).copied() else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "fragment_attachment_pairing_ordinal_out_of_range",
                "Explicit attachment source ordinals must target exported role occurrences.",
            );
            continue;
        };
        let Some(destination) = destination.get(*destination_ordinal as usize).copied() else {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "fragment_attachment_pairing_ordinal_out_of_range",
                "Explicit attachment destination ordinals must target exported role occurrences.",
            );
            continue;
        };
        if !covered_sources.insert(source.ordinal)
            || !covered_destinations.insert(destination.ordinal)
        {
            push_issue(
                report,
                Some(format!("{subject}.pairing")),
                "duplicate_fragment_attachment_pairing_ordinal",
                "Explicit attachment ordinals must be one-to-one.",
            );
        }
        result.push((source, destination));
    }
    if covered_sources.len() != source.len() || covered_destinations.len() != destination.len() {
        push_incomplete_pairing_issue(report, subject);
    }
    result
}

fn push_incomplete_pairing_issue(report: &mut FamilyValidationReport, subject: &str) {
    push_issue(
        report,
        Some(format!("{subject}.pairing")),
        "incomplete_fragment_attachment_pairing",
        "Attachment pairing must cover every source and destination occurrence.",
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
            translation: attachment.offset,
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

fn cmp_f32_array(left: [f32; 3], right: [f32; 3]) -> std::cmp::Ordering {
    left[0]
        .total_cmp(&right[0])
        .then_with(|| left[1].total_cmp(&right[1]))
        .then_with(|| left[2].total_cmp(&right[2]))
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
