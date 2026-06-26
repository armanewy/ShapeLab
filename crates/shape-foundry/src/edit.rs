//! Local override, semantic edit, and command-application contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_asset::{
    AssetEditProgram, BoundaryLoopId, OperationId, ParameterId, PartDefinitionId, PartInstanceId,
    RegionId, SocketId,
};
use shape_family_compile::{
    FamilyImplementation, StyleImplementation, identity::GeometryInputFingerprint,
};

/// Stable local override ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalRecipeOverrideId(pub String);

/// What happens to a local override when style/provider inputs change.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverrideSurvivalPolicy {
    /// Keep only when the base geometry fingerprint is unchanged.
    Pinned,
    /// Replay and validate against the changed recipe.
    Revalidate,
    /// Drop when the style/provider changes.
    DropOnStyleChange,
}

/// Semantic target touched by a local override.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TouchedSemanticTarget {
    /// Family parameter slot.
    FamilySlot(String),
    /// Asset parameter.
    Parameter(ParameterId),
    /// Part definition.
    PartDefinition(PartDefinitionId),
    /// Part occurrence.
    PartInstance(PartInstanceId),
    /// Modeling operation.
    Operation(OperationId),
    /// Surface region.
    Region(RegionId),
    /// Boundary loop.
    BoundaryLoop(BoundaryLoopId),
    /// Socket.
    Socket(SocketId),
    /// Pack-authored semantic target.
    Custom(String),
}

/// Local recipe override applied after base family instantiation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalRecipeOverride {
    /// Stable override ID.
    pub id: LocalRecipeOverrideId,
    /// Geometry fingerprint of the recipe this override was authored against.
    pub base_geometry_fingerprint: GeometryInputFingerprint,
    /// Ordered semantic edit program.
    pub edit_program: AssetEditProgram,
    /// Semantic targets the edit touches.
    pub touched_targets: Vec<TouchedSemanticTarget>,
    /// Survival policy when upstream inputs change.
    pub survival_policy: OverrideSurvivalPolicy,
}

/// Replayable foundry edit program row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryEdit {
    /// Human-facing edit label.
    pub label: String,
    /// Commands applied by this edit.
    pub commands: Vec<crate::command::FoundryCommand>,
}

/// A local override row dropped during command/style application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DroppedLocalOverride {
    /// Dropped override ID.
    pub id: LocalRecipeOverrideId,
    /// Deterministic reason code.
    pub reason: String,
}

/// Report produced by a style-reference change.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryStyleChangeReport {
    /// Control-state keys not available in the new profile/domain.
    pub dropped_controls: Vec<String>,
    /// Provider override roles not available in the new implementation set.
    pub dropped_provider_overrides: Vec<String>,
    /// Local recipe overrides dropped by survival policy.
    pub dropped_local_overrides: Vec<DroppedLocalOverride>,
}

/// Optional resolved contracts used to preserve compatible state across style changes.
#[derive(Debug, Copy, Clone, Default)]
pub struct FoundryStyleChangeContext<'a> {
    /// New customizer profile, if available.
    pub profile: Option<&'a crate::CustomizerProfile>,
    /// Effective family implementation, if available.
    pub family_implementation: Option<&'a FamilyImplementation>,
    /// New style implementation, if available.
    pub style_implementation: Option<&'a StyleImplementation>,
}

/// Result of applying a document command.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FoundryCommandApplicationReport {
    /// Controls dropped while applying a style change.
    pub dropped_controls: Vec<String>,
    /// Provider overrides dropped while applying a style change.
    pub dropped_provider_overrides: Vec<String>,
    /// Local overrides dropped while applying a style change.
    pub dropped_local_overrides: Vec<DroppedLocalOverride>,
}

/// Result of replaying a command list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryCommandReplay {
    /// Document after every supported command was applied.
    pub document: crate::FoundryAssetDocument,
    /// Per-command reports in input order.
    pub reports: Vec<FoundryCommandApplicationReport>,
}

/// Command application failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCommandApplicationError {
    /// Candidate, revision, export, and pack commands need session/project hosts.
    UnsupportedCommand {
        /// Stable command kind.
        command: String,
    },
    /// A command attempted to edit a locked target.
    LockedTarget {
        /// Stable target description.
        target: String,
    },
    /// A provider control value did not name an available provider option.
    UnknownProviderOption {
        /// Family role.
        role: String,
        /// Provider ID.
        provider_id: String,
    },
}

/// Apply a non-candidate foundry command to a document.
pub fn apply_foundry_command(
    document: &mut crate::FoundryAssetDocument,
    command: &crate::FoundryCommand,
) -> Result<FoundryCommandApplicationReport, FoundryCommandApplicationError> {
    apply_foundry_command_with_style_context(
        document,
        command,
        FoundryStyleChangeContext::default(),
    )
}

/// Apply a foundry command, using style-change context when the command switches style refs.
pub fn apply_foundry_command_with_style_context(
    document: &mut crate::FoundryAssetDocument,
    command: &crate::FoundryCommand,
    style_context: FoundryStyleChangeContext<'_>,
) -> Result<FoundryCommandApplicationReport, FoundryCommandApplicationError> {
    match command {
        crate::FoundryCommand::SetControl { control_id, value } => {
            ensure_unlocked_control(document, control_id)?;
            document
                .control_state
                .insert(control_id.clone(), value.clone());
            document.build_stamp = None;
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::ResetControl { control_id } => {
            ensure_unlocked_control(document, control_id)?;
            document.control_state.remove(control_id);
            document.build_stamp = None;
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SelectProvider { role, provider_ref } => {
            ensure_unlocked_provider(document, role)?;
            document.provider_overrides.insert(
                role.clone(),
                crate::ProviderOverride {
                    role: role.clone(),
                    provider_ref: provider_ref.clone(),
                },
            );
            document.catalog_lock = None;
            document.build_stamp = None;
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetRolePresence { role, enabled } => {
            ensure_unlocked_role(document, role)?;
            document
                .control_state
                .insert(role.clone(), crate::ControlValue::Toggle(*enabled));
            document.build_stamp = None;
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetStyle {
            style_content_ref,
            style_implementation_ref,
        } => {
            let report = apply_foundry_style_change(
                document,
                style_content_ref.clone(),
                style_implementation_ref.clone(),
                style_context,
            );
            Ok(FoundryCommandApplicationReport {
                dropped_controls: report.dropped_controls,
                dropped_provider_overrides: report.dropped_provider_overrides,
                dropped_local_overrides: report.dropped_local_overrides,
            })
        }
        crate::FoundryCommand::SetLock { lock } => {
            if let Some(existing) = document
                .foundry_locks
                .iter_mut()
                .find(|existing| existing.target == lock.target)
            {
                *existing = lock.clone();
            } else {
                document.foundry_locks.push(lock.clone());
            }
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::ClearLock { target } => {
            document
                .foundry_locks
                .retain(|existing| existing.target != *target);
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetVariationIntent { intent } => {
            document.variation_state.intent = intent.clone().normalized();
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetVariationScope { scope } => {
            ensure_unlocked_variation_scope(document, scope)?;
            document.variation_state.intent.scope = scope.clone();
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetVariationChannels { channels } => {
            ensure_unlocked_variation_channels(document, channels)?;
            document.variation_state.intent.channels = if channels.is_empty() {
                vec![crate::VariationChannel::CompleteLook]
            } else {
                channels.clone()
            };
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::ClearVariationFocus | crate::FoundryCommand::ClearFocusPartGroup => {
            document.variation_state.intent.scope = crate::VariationScope::WholeAsset;
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::SetFocusPartGroup { group_id } => {
            let scope = crate::VariationScope::SemanticPartGroup {
                group_id: group_id.clone(),
                display_name: humanize_identifier(group_id),
            };
            ensure_unlocked_variation_scope(document, &scope)?;
            document.variation_state.intent.scope = scope;
            document.variation_state.intent = document.variation_state.intent.clone().normalized();
            Ok(FoundryCommandApplicationReport::default())
        }
        crate::FoundryCommand::GenerateCandidates(_)
        | crate::FoundryCommand::GenerateFocusedPartCandidates { .. }
        | crate::FoundryCommand::AcceptCandidate { .. }
        | crate::FoundryCommand::RejectCandidate { .. }
        | crate::FoundryCommand::Undo
        | crate::FoundryCommand::SwitchRevision { .. }
        | crate::FoundryCommand::Export { .. }
        | crate::FoundryCommand::AddCurrentToPack { .. } => {
            Err(FoundryCommandApplicationError::UnsupportedCommand {
                command: command_kind(command).to_owned(),
            })
        }
    }
}

/// Replay supported foundry commands from an initial document.
pub fn replay_foundry_commands(
    mut document: crate::FoundryAssetDocument,
    commands: &[crate::FoundryCommand],
) -> Result<FoundryCommandReplay, FoundryCommandApplicationError> {
    let mut reports = Vec::with_capacity(commands.len());
    for command in commands {
        reports.push(apply_foundry_command(&mut document, command)?);
    }
    Ok(FoundryCommandReplay { document, reports })
}

/// Apply a style switch and preserve only state that remains compatible.
pub fn apply_foundry_style_change(
    document: &mut crate::FoundryAssetDocument,
    style_content_ref: crate::CatalogContentRef,
    style_implementation_ref: crate::CatalogContentRef,
    context: FoundryStyleChangeContext<'_>,
) -> FoundryStyleChangeReport {
    document.style_content_ref = style_content_ref;
    document.style_implementation_ref = style_implementation_ref;
    document.catalog_lock = None;
    document.build_stamp = None;

    let mut report = FoundryStyleChangeReport::default();
    if let Some(profile) = context.profile {
        let controls = profile
            .controls
            .iter()
            .map(|control| (control.id.as_str(), control))
            .collect::<BTreeMap<_, _>>();
        document.control_state.retain(|control_id, value| {
            let keep = controls
                .get(control_id.as_str())
                .is_some_and(|control| control_value_is_available(control, value));
            if !keep {
                report.dropped_controls.push(control_id.clone());
            }
            keep
        });
    }

    if context.family_implementation.is_some() || context.style_implementation.is_some() {
        document.provider_overrides.retain(|role, override_row| {
            let keep = provider_override_is_available(
                role,
                &override_row.provider_ref.stable_id,
                context.family_implementation,
                context.style_implementation,
            );
            if !keep {
                report.dropped_provider_overrides.push(role.clone());
            }
            keep
        });
    }

    document.local_recipe_overrides.retain(|override_row| {
        let keep = override_row.survival_policy != OverrideSurvivalPolicy::DropOnStyleChange;
        if !keep {
            report.dropped_local_overrides.push(DroppedLocalOverride {
                id: override_row.id.clone(),
                reason: "drop_on_style_change".to_owned(),
            });
        }
        keep
    });

    report
}

fn ensure_unlocked_control(
    document: &crate::FoundryAssetDocument,
    control_id: &str,
) -> Result<(), FoundryCommandApplicationError> {
    if has_locked_target(
        document,
        &crate::FoundryLockTarget::Control(control_id.to_owned()),
    ) {
        return Err(FoundryCommandApplicationError::LockedTarget {
            target: format!("control:{control_id}"),
        });
    }
    Ok(())
}

fn ensure_unlocked_role(
    document: &crate::FoundryAssetDocument,
    role: &str,
) -> Result<(), FoundryCommandApplicationError> {
    if has_locked_target(document, &crate::FoundryLockTarget::Role(role.to_owned())) {
        return Err(FoundryCommandApplicationError::LockedTarget {
            target: format!("role:{role}"),
        });
    }
    Ok(())
}

fn ensure_unlocked_provider(
    document: &crate::FoundryAssetDocument,
    role: &str,
) -> Result<(), FoundryCommandApplicationError> {
    if has_locked_target(
        document,
        &crate::FoundryLockTarget::Provider(role.to_owned()),
    ) {
        return Err(FoundryCommandApplicationError::LockedTarget {
            target: format!("provider:{role}"),
        });
    }
    Ok(())
}

fn ensure_unlocked_variation_scope(
    document: &crate::FoundryAssetDocument,
    scope: &crate::VariationScope,
) -> Result<(), FoundryCommandApplicationError> {
    if has_locked_target(
        document,
        &crate::FoundryLockTarget::VariationScope(scope.clone()),
    ) {
        return Err(FoundryCommandApplicationError::LockedTarget {
            target: "variation scope".to_owned(),
        });
    }
    if let Some(group_id) = scope.semantic_part_group_id()
        && has_locked_target(
            document,
            &crate::FoundryLockTarget::FocusPartGroup(group_id.to_owned()),
        )
    {
        return Err(FoundryCommandApplicationError::LockedTarget {
            target: "focus part".to_owned(),
        });
    }
    Ok(())
}

fn ensure_unlocked_variation_channels(
    document: &crate::FoundryAssetDocument,
    channels: &[crate::VariationChannel],
) -> Result<(), FoundryCommandApplicationError> {
    for channel in channels {
        if has_locked_target(
            document,
            &crate::FoundryLockTarget::VariationChannel(channel.clone()),
        ) {
            return Err(FoundryCommandApplicationError::LockedTarget {
                target: "variation channel".to_owned(),
            });
        }
    }
    Ok(())
}

fn has_locked_target(
    document: &crate::FoundryAssetDocument,
    target: &crate::FoundryLockTarget,
) -> bool {
    document
        .foundry_locks
        .iter()
        .any(|lock| lock.target == *target && lock.mode == crate::FoundryLockMode::Locked)
}

fn provider_override_is_available(
    role: &str,
    provider_id: &str,
    family_implementation: Option<&FamilyImplementation>,
    style_implementation: Option<&StyleImplementation>,
) -> bool {
    let family_match = family_implementation.is_some_and(|implementation| {
        implementation
            .fragments
            .get(provider_id)
            .is_some_and(|fragment| fragment.provided_role == role)
    });
    let style_match = style_implementation.is_some_and(|implementation| {
        implementation
            .prototypes
            .get(provider_id)
            .is_some_and(|fragment| fragment.provided_role == role)
    });
    family_match || style_match
}

fn control_value_is_available(
    control: &crate::CustomizerControl,
    value: &crate::ControlValue,
) -> bool {
    if !control_value_matches_kind(&control.kind, value) {
        return false;
    }
    if control
        .domain
        .discrete_values
        .iter()
        .any(|allowed| allowed == value)
    {
        return true;
    }
    match value {
        crate::ControlValue::Scalar(value) => control
            .domain
            .continuous_intervals
            .iter()
            .any(|interval| (*value >= interval.minimum) && (*value <= interval.maximum)),
        _ => control.domain.discrete_values.is_empty(),
    }
}

fn control_value_matches_kind(kind: &crate::ControlKind, value: &crate::ControlValue) -> bool {
    matches!(
        (kind, value),
        (
            crate::ControlKind::ContinuousAxis { .. },
            crate::ControlValue::Scalar(_)
        ) | (
            crate::ControlKind::IntegerStepper { .. },
            crate::ControlValue::Integer(_)
        ) | (
            crate::ControlKind::Toggle { .. },
            crate::ControlValue::Toggle(_)
        ) | (
            crate::ControlKind::ChoiceGallery { .. },
            crate::ControlValue::Choice(_)
        ) | (
            crate::ControlKind::ProviderGallery { .. },
            crate::ControlValue::Provider(_)
        )
    )
}

fn command_kind(command: &crate::FoundryCommand) -> &'static str {
    match command {
        crate::FoundryCommand::SetControl { .. } => "set_control",
        crate::FoundryCommand::ResetControl { .. } => "reset_control",
        crate::FoundryCommand::SelectProvider { .. } => "select_provider",
        crate::FoundryCommand::SetRolePresence { .. } => "set_role_presence",
        crate::FoundryCommand::SetStyle { .. } => "set_style",
        crate::FoundryCommand::SetLock { .. } => "set_lock",
        crate::FoundryCommand::ClearLock { .. } => "clear_lock",
        crate::FoundryCommand::SetVariationIntent { .. } => "set_variation_intent",
        crate::FoundryCommand::SetVariationScope { .. } => "set_variation_scope",
        crate::FoundryCommand::SetVariationChannels { .. } => "set_variation_channels",
        crate::FoundryCommand::ClearVariationFocus => "clear_variation_focus",
        crate::FoundryCommand::ClearFocusPartGroup => "clear_focus_part_group",
        crate::FoundryCommand::SetFocusPartGroup { .. } => "set_focus_part_group",
        crate::FoundryCommand::GenerateFocusedPartCandidates { .. } => {
            "generate_focused_part_candidates"
        }
        crate::FoundryCommand::GenerateCandidates(_) => "generate_candidates",
        crate::FoundryCommand::AcceptCandidate { .. } => "accept_candidate",
        crate::FoundryCommand::RejectCandidate { .. } => "reject_candidate",
        crate::FoundryCommand::Undo => "undo",
        crate::FoundryCommand::SwitchRevision { .. } => "switch_revision",
        crate::FoundryCommand::Export { .. } => "export",
        crate::FoundryCommand::AddCurrentToPack { .. } => "add_current_to_pack",
    }
}

fn humanize_identifier(identifier: &str) -> String {
    let words = identifier
        .split(|character: char| character == '_' || character == '-' || character.is_whitespace())
        .filter(|word| !word.trim().is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let mut output = first.to_uppercase().collect::<String>();
                    output.push_str(&chars.as_str().to_ascii_lowercase());
                    output
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>();
    if words.is_empty() {
        "Focused Part".to_owned()
    } else {
        words.join(" ")
    }
}
