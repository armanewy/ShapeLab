//! Optional LLM-facing adapter contracts for Foundry commands.
//!
//! This module does not parse natural language, call an LLM, or generate
//! geometry. It accepts a small structured intent enum that an external adapter
//! may produce, exposes only visible customizer controls, and returns validated
//! [`FoundryCommand`] values for the same command surface used by the UI.

use serde::{Deserialize, Serialize};

use crate::{
    ClosedInterval, ControlKind, ControlValue, CustomizerControl, CustomizerProfile,
    FeasibleControlDomain, FoundryAssetDocument, FoundryCandidateId, FoundryCandidateStatus,
    FoundryCandidateSummary, FoundryCommand, FoundryLock, FoundryLockMode, FoundryLockTarget,
    FoundryValidationReport, GenerateCandidatesRequest, validate_foundry_command,
};

/// Current schema version for structured LLM adapter contracts.
pub const FOUNDRY_LLM_ADAPTER_SCHEMA_VERSION: u32 = 1;

/// Bounded adapter limits for commands that can trigger runtime work.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FoundryLlmAdapterLimits {
    /// Maximum candidate count a single LLM intent may request.
    pub max_candidate_count: u32,
}

impl Default for FoundryLlmAdapterLimits {
    fn default() -> Self {
        Self {
            max_candidate_count: 6,
        }
    }
}

/// Runtime context used to adapt a structured LLM intent.
#[derive(Debug, Clone, Copy)]
pub struct FoundryLlmAdapterContext<'a> {
    /// Current semantic source document.
    pub document: &'a FoundryAssetDocument,
    /// Current visible customizer profile.
    pub profile: &'a CustomizerProfile,
    /// Candidate rows currently available to the host.
    pub candidates: &'a [FoundryCandidateSummary],
    /// Export profiles explicitly allowed by the host.
    pub export_profiles: &'a [String],
    /// Adapter limits.
    pub limits: FoundryLlmAdapterLimits,
}

impl<'a> FoundryLlmAdapterContext<'a> {
    /// Create an adapter context with no candidates or export profiles.
    #[must_use]
    pub fn new(document: &'a FoundryAssetDocument, profile: &'a CustomizerProfile) -> Self {
        Self {
            document,
            profile,
            candidates: &[],
            export_profiles: &[],
            limits: FoundryLlmAdapterLimits::default(),
        }
    }

    /// Attach candidate rows used to validate candidate commands.
    #[must_use]
    pub fn with_candidates(mut self, candidates: &'a [FoundryCandidateSummary]) -> Self {
        self.candidates = candidates;
        self
    }

    /// Attach host-approved export profiles.
    #[must_use]
    pub fn with_export_profiles(mut self, export_profiles: &'a [String]) -> Self {
        self.export_profiles = export_profiles;
        self
    }

    /// Override adapter limits.
    #[must_use]
    pub fn with_limits(mut self, limits: FoundryLlmAdapterLimits) -> Self {
        self.limits = limits;
        self
    }
}

/// Structured intent accepted from an optional external LLM adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case")]
pub enum FoundryLlmIntent {
    /// List visible controls and their safe values/options.
    ListControls,
    /// Return an LLM-safe visible state summary.
    DescribeState,
    /// Set one visible customizer control.
    SetControl {
        /// Visible control ID.
        control_id: String,
        /// New value.
        value: ControlValue,
    },
    /// Select a provider through a visible provider-gallery control.
    SelectProvider {
        /// Visible provider-gallery control ID.
        control_id: String,
        /// Provider option ID exposed by that control.
        provider_id: String,
    },
    /// Generate deterministic candidate directions.
    GenerateCandidates {
        /// Optional candidate strategy ID.
        strategy_id: Option<String>,
        /// Requested candidate count.
        count: u32,
        /// Optional explicit seed. Defaults to the document seed.
        seed: Option<u64>,
    },
    /// Lock or unlock one visible customizer control.
    LockControl {
        /// Visible control ID.
        control_id: String,
        /// Whether to lock or clear the lock.
        locked: bool,
    },
    /// Accept one currently proposed candidate.
    AcceptCandidate {
        /// Candidate ID exposed by the host.
        candidate_id: FoundryCandidateId,
    },
    /// Export through a host-approved export profile.
    Export {
        /// Export profile key.
        profile: String,
    },
}

/// High-level control kind exposed to adapter clients.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundryLlmControlKind {
    /// Continuous scalar control.
    Scalar,
    /// Integer stepper control.
    Integer,
    /// Boolean control.
    Toggle,
    /// Symbolic choice gallery.
    Choice,
    /// Provider gallery, exposed through provider option IDs.
    Provider,
}

/// One visible option exposed for a choice or provider control.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryLlmControlOption {
    /// Option value/provider ID.
    pub value: String,
    /// Human-facing label.
    pub label: String,
    /// Whether the option is currently available.
    pub available: bool,
    /// Deterministic reason when unavailable.
    pub unavailable_reason: Option<String>,
}

/// One visible control descriptor safe to show to an LLM adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryLlmControlDescriptor {
    /// Stable control ID.
    pub id: String,
    /// Human-facing label.
    pub label: String,
    /// Whether this is part of the novice primary control surface.
    pub primary: bool,
    /// Adapter-facing control kind.
    pub kind: FoundryLlmControlKind,
    /// Current value, falling back to the authored visible default.
    pub current_value: ControlValue,
    /// Available interval(s) for scalar controls.
    pub continuous_intervals: Vec<ClosedInterval>,
    /// Choice/provider options safe to show to adapter clients.
    pub options: Vec<FoundryLlmControlOption>,
    /// Whether the control is explicitly locked.
    pub locked: bool,
    /// Human-facing reason when locked.
    pub locked_reason: Option<String>,
}

/// Candidate descriptor safe to expose through the LLM adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryLlmCandidateDescriptor {
    /// Candidate ID.
    pub id: FoundryCandidateId,
    /// Human-facing label.
    pub label: String,
    /// Candidate status.
    pub status: FoundryCandidateStatus,
    /// Visible controls changed by this candidate.
    pub changed_controls: Vec<String>,
    /// Optional preview ID.
    pub preview_id: Option<String>,
}

/// LLM-safe state summary. This intentionally omits the source document and
/// local recipe overrides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryLlmStateSummary {
    /// Visible controls.
    pub controls: Vec<FoundryLlmControlDescriptor>,
    /// Current candidates whose changed controls remain visible.
    pub candidates: Vec<FoundryLlmCandidateDescriptor>,
    /// Host-approved export profiles.
    pub export_profiles: Vec<String>,
}

/// Adapter response after planning one intent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "response", rename_all = "snake_case")]
pub enum FoundryLlmAdapterResponse {
    /// Read-only visible control list.
    ControlList {
        /// Visible controls.
        controls: Vec<FoundryLlmControlDescriptor>,
    },
    /// Read-only LLM-safe state summary.
    StateSummary {
        /// State summary safe for an optional command adapter.
        state: FoundryLlmStateSummary,
    },
    /// Validated command for host preview/confirmation/execution.
    Command {
        /// Command to run through the normal foundry command surface.
        command: FoundryCommand,
    },
}

/// Safety metadata attached to every adapter plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryLlmSafetyEnvelope {
    /// Adapter never permits direct recipe mutation.
    pub direct_recipe_mutation_allowed: bool,
    /// Adapter exposes visible control IDs/options, not hidden scalar paths.
    pub hidden_scalar_paths_exposed: bool,
    /// Commands must validate before a host may commit them.
    pub command_validated: bool,
    /// Mutating commands require host preview before commit.
    pub preview_required_before_commit: bool,
    /// Mutating commands require a host undo checkpoint.
    pub undo_checkpoint_required: bool,
    /// Host-side effects such as export require explicit host confirmation.
    pub host_confirmation_required: bool,
}

impl FoundryLlmSafetyEnvelope {
    fn read_only() -> Self {
        Self {
            direct_recipe_mutation_allowed: false,
            hidden_scalar_paths_exposed: false,
            command_validated: false,
            preview_required_before_commit: false,
            undo_checkpoint_required: false,
            host_confirmation_required: false,
        }
    }

    fn previewable_command() -> Self {
        Self {
            direct_recipe_mutation_allowed: false,
            hidden_scalar_paths_exposed: false,
            command_validated: true,
            preview_required_before_commit: true,
            undo_checkpoint_required: true,
            host_confirmation_required: false,
        }
    }

    fn host_side_effect() -> Self {
        Self {
            direct_recipe_mutation_allowed: false,
            hidden_scalar_paths_exposed: false,
            command_validated: true,
            preview_required_before_commit: true,
            undo_checkpoint_required: false,
            host_confirmation_required: true,
        }
    }
}

/// Planned adapter result. The host still owns preview, undo, and execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryLlmAdapterPlan {
    /// Adapter schema version.
    pub schema_version: u32,
    /// Planned response.
    pub response: FoundryLlmAdapterResponse,
    /// Deterministic human-facing summary.
    pub summary: String,
    /// Safety metadata.
    pub safety: FoundryLlmSafetyEnvelope,
}

/// Rejection reason for unsafe or invalid adapter intents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum FoundryLlmAdapterError {
    /// The control is not present in the current profile.
    UnknownVisibleControl {
        /// Requested control ID.
        control_id: String,
    },
    /// The control exists but is not visible to adapter clients.
    HiddenControl {
        /// Requested control ID.
        control_id: String,
    },
    /// The requested control has the wrong kind for the intent.
    WrongControlKind {
        /// Requested control ID.
        control_id: String,
        /// Expected kind.
        expected: String,
    },
    /// The control is locked.
    LockedControl {
        /// Requested control ID.
        control_id: String,
        /// Human-facing lock reason.
        reason: Option<String>,
    },
    /// The provider option is not authored for that visible control.
    UnknownProviderOption {
        /// Requested control ID.
        control_id: String,
        /// Requested provider option.
        provider_id: String,
    },
    /// A lower-level provider override already selects a different provider.
    ExistingProviderOverrideConflict {
        /// Requested control ID.
        control_id: String,
        /// Provider role.
        role: String,
        /// Existing provider ID from the document override.
        existing_provider_id: String,
        /// Requested provider ID.
        requested_provider_id: String,
    },
    /// A lower-level provider override is not in the visible provider options.
    ExistingProviderOverrideOutsideVisibleOptions {
        /// Requested control ID.
        control_id: String,
        /// Provider role.
        role: String,
    },
    /// LLM candidate generation requires an explicit visible strategy.
    CandidateStrategyRequired,
    /// Candidate generation requested zero candidates.
    EmptyCandidateRequest,
    /// Candidate generation exceeded the adapter limit.
    CandidateRequestTooLarge {
        /// Requested count.
        requested: u32,
        /// Maximum allowed count.
        maximum: u32,
    },
    /// The candidate ID is not currently known to the host.
    UnknownCandidate {
        /// Requested candidate ID.
        candidate_id: FoundryCandidateId,
    },
    /// The candidate exists but is not currently proposed.
    CandidateNotProposed {
        /// Requested candidate ID.
        candidate_id: FoundryCandidateId,
        /// Current status.
        status: FoundryCandidateStatus,
    },
    /// A candidate touches a control that is not visible to the adapter.
    CandidateTouchesHiddenControl {
        /// Requested candidate ID.
        candidate_id: FoundryCandidateId,
        /// Hidden or unknown control ID.
        control_id: String,
    },
    /// A candidate touches a currently locked or search-protected control.
    CandidateTouchesLockedControl {
        /// Requested candidate ID.
        candidate_id: FoundryCandidateId,
        /// Locked control ID.
        control_id: String,
        /// Human-facing lock reason.
        reason: Option<String>,
    },
    /// A candidate strategy touches a control that is not visible to the adapter.
    CandidateStrategyTouchesHiddenControl {
        /// Candidate strategy ID.
        strategy_id: String,
        /// Hidden or unknown control ID.
        control_id: String,
    },
    /// A candidate strategy touches a currently locked or search-protected control.
    CandidateStrategyTouchesLockedControl {
        /// Candidate strategy ID.
        strategy_id: String,
        /// Locked control ID.
        control_id: String,
        /// Human-facing lock reason.
        reason: Option<String>,
    },
    /// Export intent was received without a host allow-list.
    ExportProfilesRequired,
    /// The profile is not on the host-approved export allow-list.
    UnknownExportProfile {
        /// Requested export profile.
        profile: String,
    },
    /// The translated command failed foundry command validation.
    InvalidCommand {
        /// Validation report.
        report: FoundryValidationReport,
    },
}

/// Plan one structured LLM intent against the current Foundry context.
pub fn plan_foundry_llm_intent(
    intent: FoundryLlmIntent,
    context: FoundryLlmAdapterContext<'_>,
) -> Result<FoundryLlmAdapterPlan, FoundryLlmAdapterError> {
    match intent {
        FoundryLlmIntent::ListControls => Ok(FoundryLlmAdapterPlan {
            schema_version: FOUNDRY_LLM_ADAPTER_SCHEMA_VERSION,
            response: FoundryLlmAdapterResponse::ControlList {
                controls: foundry_llm_visible_controls(context.document, context.profile),
            },
            summary: "List visible Foundry controls.".to_owned(),
            safety: FoundryLlmSafetyEnvelope::read_only(),
        }),
        FoundryLlmIntent::DescribeState => Ok(FoundryLlmAdapterPlan {
            schema_version: FOUNDRY_LLM_ADAPTER_SCHEMA_VERSION,
            response: FoundryLlmAdapterResponse::StateSummary {
                state: foundry_llm_state_summary(context),
            },
            summary: "Describe the current Foundry state through visible adapter fields."
                .to_owned(),
            safety: FoundryLlmSafetyEnvelope::read_only(),
        }),
        FoundryLlmIntent::SetControl { control_id, value } => {
            let control = visible_control(context.profile, &control_id)?;
            reject_locked_control(context.document, control)?;
            if let ControlKind::ProviderGallery { role, .. } = &control.kind
                && let ControlValue::Provider(provider_id) = &value
            {
                reject_conflicting_provider_override(
                    context.document,
                    control,
                    &control_id,
                    role,
                    provider_id,
                )?;
            }
            command_plan(
                FoundryCommand::SetControl {
                    control_id: control_id.clone(),
                    value,
                },
                context,
                format!("Set visible control `{control_id}`."),
                FoundryLlmSafetyEnvelope::previewable_command(),
            )
        }
        FoundryLlmIntent::SelectProvider {
            control_id,
            provider_id,
        } => {
            let control = visible_control(context.profile, &control_id)?;
            reject_locked_control(context.document, control)?;
            let ControlKind::ProviderGallery { role, options } = &control.kind else {
                return Err(FoundryLlmAdapterError::WrongControlKind {
                    control_id,
                    expected: "provider".to_owned(),
                });
            };
            if !options
                .iter()
                .any(|option| option.provider_id == provider_id)
            {
                return Err(FoundryLlmAdapterError::UnknownProviderOption {
                    control_id,
                    provider_id,
                });
            }
            reject_conflicting_provider_override(
                context.document,
                control,
                &control_id,
                role,
                &provider_id,
            )?;
            command_plan(
                FoundryCommand::SetControl {
                    control_id: control_id.clone(),
                    value: ControlValue::Provider(provider_id.clone()),
                },
                context,
                format!("Select provider `{provider_id}` through visible control `{control_id}`."),
                FoundryLlmSafetyEnvelope::previewable_command(),
            )
        }
        FoundryLlmIntent::GenerateCandidates {
            strategy_id,
            count,
            seed,
        } => {
            if count == 0 {
                return Err(FoundryLlmAdapterError::EmptyCandidateRequest);
            }
            if count > context.limits.max_candidate_count {
                return Err(FoundryLlmAdapterError::CandidateRequestTooLarge {
                    requested: count,
                    maximum: context.limits.max_candidate_count,
                });
            }
            let strategy_id =
                strategy_id.ok_or(FoundryLlmAdapterError::CandidateStrategyRequired)?;
            ensure_candidate_strategy_is_llm_safe(&strategy_id, context)?;
            command_plan(
                FoundryCommand::GenerateCandidates(GenerateCandidatesRequest {
                    strategy_id: Some(strategy_id),
                    count,
                    seed: seed.unwrap_or(context.document.seed),
                }),
                context,
                format!("Generate {count} Foundry candidate directions."),
                FoundryLlmSafetyEnvelope::previewable_command(),
            )
        }
        FoundryLlmIntent::LockControl { control_id, locked } => {
            let control = visible_control(context.profile, &control_id)?;
            let command = if locked {
                FoundryCommand::SetLock {
                    lock: FoundryLock {
                        target: FoundryLockTarget::Control(control_id.clone()),
                        mode: FoundryLockMode::Locked,
                        reason: Some("Locked through command adapter".to_owned()),
                    },
                }
            } else {
                FoundryCommand::ClearLock {
                    target: control_lock_target_to_clear(context.document, control)
                        .unwrap_or_else(|| FoundryLockTarget::Control(control_id.clone())),
                }
            };
            let verb = if locked { "Lock" } else { "Unlock" };
            command_plan(
                command,
                context,
                format!("{verb} visible control `{control_id}`."),
                FoundryLlmSafetyEnvelope::previewable_command(),
            )
        }
        FoundryLlmIntent::AcceptCandidate { candidate_id } => {
            let candidate = context
                .candidates
                .iter()
                .find(|candidate| candidate.id == candidate_id)
                .ok_or_else(|| FoundryLlmAdapterError::UnknownCandidate {
                    candidate_id: candidate_id.clone(),
                })?;
            if candidate.status != FoundryCandidateStatus::Proposed {
                return Err(FoundryLlmAdapterError::CandidateNotProposed {
                    candidate_id,
                    status: candidate.status,
                });
            }
            ensure_candidate_is_llm_safe(candidate, context)?;
            command_plan(
                FoundryCommand::AcceptCandidate {
                    candidate_id: candidate_id.clone(),
                },
                context,
                format!("Accept candidate `{}`.", candidate_id.0),
                FoundryLlmSafetyEnvelope::previewable_command(),
            )
        }
        FoundryLlmIntent::Export { profile } => {
            if context.export_profiles.is_empty() {
                return Err(FoundryLlmAdapterError::ExportProfilesRequired);
            }
            if !context
                .export_profiles
                .iter()
                .any(|allowed| allowed == &profile)
            {
                return Err(FoundryLlmAdapterError::UnknownExportProfile { profile });
            }
            command_plan(
                FoundryCommand::Export {
                    profile: profile.clone(),
                    out_dir: None,
                },
                context,
                format!("Export through profile `{profile}`."),
                FoundryLlmSafetyEnvelope::host_side_effect(),
            )
        }
    }
}

/// Return visible controls safe to expose to an external LLM adapter.
#[must_use]
pub fn foundry_llm_visible_controls(
    document: &FoundryAssetDocument,
    profile: &CustomizerProfile,
) -> Vec<FoundryLlmControlDescriptor> {
    profile
        .controls
        .iter()
        .filter(|control| control.visible)
        .map(|control| visible_control_descriptor(document, control))
        .collect()
}

/// Return an LLM-safe summary of visible state.
#[must_use]
pub fn foundry_llm_state_summary(context: FoundryLlmAdapterContext<'_>) -> FoundryLlmStateSummary {
    FoundryLlmStateSummary {
        controls: foundry_llm_visible_controls(context.document, context.profile),
        candidates: context
            .candidates
            .iter()
            .filter_map(|candidate| llm_candidate_descriptor(candidate, context.profile))
            .collect(),
        export_profiles: context.export_profiles.to_vec(),
    }
}

fn command_plan(
    command: FoundryCommand,
    context: FoundryLlmAdapterContext<'_>,
    summary: String,
    safety: FoundryLlmSafetyEnvelope,
) -> Result<FoundryLlmAdapterPlan, FoundryLlmAdapterError> {
    let report = validate_foundry_command(&command, Some(context.document), Some(context.profile));
    if !report.is_valid() {
        return Err(FoundryLlmAdapterError::InvalidCommand { report });
    }
    Ok(FoundryLlmAdapterPlan {
        schema_version: FOUNDRY_LLM_ADAPTER_SCHEMA_VERSION,
        response: FoundryLlmAdapterResponse::Command { command },
        summary,
        safety,
    })
}

fn ensure_candidate_strategy_is_llm_safe(
    strategy_id: &str,
    context: FoundryLlmAdapterContext<'_>,
) -> Result<(), FoundryLlmAdapterError> {
    let Some(strategy) = context
        .profile
        .candidate_strategies
        .iter()
        .find(|strategy| strategy.id == strategy_id)
    else {
        return Ok(());
    };
    for control_id in &strategy.control_ids {
        let control = visible_control(context.profile, control_id).map_err(|_| {
            FoundryLlmAdapterError::CandidateStrategyTouchesHiddenControl {
                strategy_id: strategy_id.to_owned(),
                control_id: control_id.clone(),
            }
        })?;
        if let Some(lock) = control_candidate_protection(context.document, control) {
            return Err(
                FoundryLlmAdapterError::CandidateStrategyTouchesLockedControl {
                    strategy_id: strategy_id.to_owned(),
                    control_id: control_id.clone(),
                    reason: lock.reason.clone(),
                },
            );
        }
    }
    Ok(())
}

fn ensure_candidate_is_llm_safe(
    candidate: &FoundryCandidateSummary,
    context: FoundryLlmAdapterContext<'_>,
) -> Result<(), FoundryLlmAdapterError> {
    for control_id in &candidate.changed_controls {
        let control = visible_control(context.profile, control_id).map_err(|_| {
            FoundryLlmAdapterError::CandidateTouchesHiddenControl {
                candidate_id: candidate.id.clone(),
                control_id: control_id.clone(),
            }
        })?;
        if let Some(lock) = control_candidate_protection(context.document, control) {
            return Err(FoundryLlmAdapterError::CandidateTouchesLockedControl {
                candidate_id: candidate.id.clone(),
                control_id: control_id.clone(),
                reason: lock.reason.clone(),
            });
        }
    }
    Ok(())
}

fn visible_control<'a>(
    profile: &'a CustomizerProfile,
    control_id: &str,
) -> Result<&'a CustomizerControl, FoundryLlmAdapterError> {
    let control = profile
        .controls
        .iter()
        .find(|control| control.id == control_id);
    match control {
        Some(control) if control.visible => Ok(control),
        Some(_) => Err(FoundryLlmAdapterError::HiddenControl {
            control_id: control_id.to_owned(),
        }),
        None => Err(FoundryLlmAdapterError::UnknownVisibleControl {
            control_id: control_id.to_owned(),
        }),
    }
}

fn reject_locked_control(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> Result<(), FoundryLlmAdapterError> {
    if let Some(lock) = control_lock(document, control) {
        return Err(FoundryLlmAdapterError::LockedControl {
            control_id: control.id.clone(),
            reason: lock.reason.clone(),
        });
    }
    Ok(())
}

fn reject_conflicting_provider_override(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
    control_id: &str,
    role: &str,
    provider_id: &str,
) -> Result<(), FoundryLlmAdapterError> {
    let Some(override_row) = document.provider_overrides.get(role) else {
        return Ok(());
    };
    let existing_provider_id = &override_row.provider_ref.stable_id;
    if !visible_provider_option(control, existing_provider_id) {
        return Err(
            FoundryLlmAdapterError::ExistingProviderOverrideOutsideVisibleOptions {
                control_id: control_id.to_owned(),
                role: role.to_owned(),
            },
        );
    }
    if existing_provider_id == provider_id {
        return Ok(());
    }
    Err(FoundryLlmAdapterError::ExistingProviderOverrideConflict {
        control_id: control_id.to_owned(),
        role: role.to_owned(),
        existing_provider_id: existing_provider_id.clone(),
        requested_provider_id: provider_id.to_owned(),
    })
}

fn llm_candidate_descriptor(
    candidate: &FoundryCandidateSummary,
    profile: &CustomizerProfile,
) -> Option<FoundryLlmCandidateDescriptor> {
    if candidate.status != FoundryCandidateStatus::Proposed {
        return None;
    }
    let mut changed_controls = Vec::new();
    for control_id in &candidate.changed_controls {
        visible_control(profile, control_id).ok()?;
        changed_controls.push(control_id.clone());
    }
    Some(FoundryLlmCandidateDescriptor {
        id: candidate.id.clone(),
        label: candidate.label.clone(),
        status: candidate.status,
        changed_controls,
        preview_id: candidate.preview_id.clone(),
    })
}

fn visible_control_descriptor(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> FoundryLlmControlDescriptor {
    let lock = control_lock(document, control);
    FoundryLlmControlDescriptor {
        id: control.id.clone(),
        label: control.label.clone(),
        primary: control.primary,
        kind: llm_control_kind(control),
        current_value: current_control_value(document, control),
        continuous_intervals: control.domain.continuous_intervals.clone(),
        options: llm_control_options(&control.kind, &control.domain),
        locked: lock.is_some(),
        locked_reason: lock.and_then(|lock| lock.reason.clone()),
    }
}

fn current_control_value(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> ControlValue {
    if let ControlKind::ProviderGallery { role, .. } = &control.kind
        && let Some(override_row) = document.provider_overrides.get(role)
        && visible_provider_option(control, &override_row.provider_ref.stable_id)
    {
        return ControlValue::Provider(override_row.provider_ref.stable_id.clone());
    }
    document
        .control_state
        .get(&control.id)
        .cloned()
        .unwrap_or_else(|| authored_default_control_value(control))
}

fn visible_provider_option(control: &CustomizerControl, provider_id: &str) -> bool {
    matches!(
        &control.kind,
        ControlKind::ProviderGallery { options, .. }
            if options.iter().any(|option| option.provider_id == provider_id)
    )
}

fn llm_control_kind(control: &CustomizerControl) -> FoundryLlmControlKind {
    match control.kind {
        ControlKind::ContinuousAxis { .. } => FoundryLlmControlKind::Scalar,
        ControlKind::IntegerStepper { .. } => FoundryLlmControlKind::Integer,
        ControlKind::Toggle { .. } => FoundryLlmControlKind::Toggle,
        ControlKind::ChoiceGallery { .. } => FoundryLlmControlKind::Choice,
        ControlKind::ProviderGallery { .. } => FoundryLlmControlKind::Provider,
    }
}

fn llm_control_options(
    kind: &ControlKind,
    domain: &FeasibleControlDomain,
) -> Vec<FoundryLlmControlOption> {
    match kind {
        ControlKind::Toggle { .. } => [false, true]
            .into_iter()
            .map(|value| {
                let control_value = ControlValue::Toggle(value);
                option_descriptor(value.to_string(), value.to_string(), &control_value, domain)
            })
            .collect(),
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| {
                let value = ControlValue::Choice(option.value.clone());
                option_descriptor(option.value.clone(), option.label.clone(), &value, domain)
            })
            .collect(),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| {
                let value = ControlValue::Provider(option.provider_id.clone());
                option_descriptor(
                    option.provider_id.clone(),
                    option.label.clone(),
                    &value,
                    domain,
                )
            })
            .collect(),
        ControlKind::IntegerStepper { .. } => domain
            .discrete_values
            .iter()
            .filter_map(|value| {
                let ControlValue::Integer(integer) = value else {
                    return None;
                };
                Some(option_descriptor(
                    integer.to_string(),
                    integer.to_string(),
                    value,
                    domain,
                ))
            })
            .collect(),
        ControlKind::ContinuousAxis { .. } => Vec::new(),
    }
}

fn option_descriptor(
    value: String,
    label: String,
    control_value: &ControlValue,
    domain: &FeasibleControlDomain,
) -> FoundryLlmControlOption {
    let option_key = control_value.option_key();
    FoundryLlmControlOption {
        value,
        label,
        available: domain.contains_available_value(control_value),
        unavailable_reason: domain.unavailable_options.get(&option_key).cloned(),
    }
}

fn authored_default_control_value(control: &CustomizerControl) -> ControlValue {
    match &control.kind {
        ControlKind::ContinuousAxis { default } => ControlValue::Scalar(*default),
        ControlKind::IntegerStepper { default } => ControlValue::Integer(*default),
        ControlKind::Toggle { default } => ControlValue::Toggle(*default),
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| ControlValue::Choice(option.value.clone()))
            .find(|value| control.domain.contains_available_value(value))
            .or_else(|| {
                control
                    .domain
                    .discrete_values
                    .iter()
                    .find(|value| matches!(value, ControlValue::Choice(_)))
                    .cloned()
            })
            .unwrap_or_else(|| ControlValue::Choice(String::new())),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| ControlValue::Provider(option.provider_id.clone()))
            .find(|value| control.domain.contains_available_value(value))
            .or_else(|| {
                control
                    .domain
                    .discrete_values
                    .iter()
                    .find(|value| matches!(value, ControlValue::Provider(_)))
                    .cloned()
            })
            .unwrap_or_else(|| ControlValue::Provider(String::new())),
    }
}

fn control_lock<'a>(
    document: &'a FoundryAssetDocument,
    control: &CustomizerControl,
) -> Option<&'a FoundryLock> {
    document
        .foundry_locks
        .iter()
        .find(|lock| lock.mode == FoundryLockMode::Locked && lock_matches_control(lock, control))
}

fn control_lock_target_to_clear(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> Option<FoundryLockTarget> {
    document
        .foundry_locks
        .iter()
        .find(|lock| {
            matches!(
                lock.mode,
                FoundryLockMode::Locked | FoundryLockMode::SearchProtected
            ) && lock_matches_control(lock, control)
        })
        .map(|lock| lock.target.clone())
}

fn control_candidate_protection<'a>(
    document: &'a FoundryAssetDocument,
    control: &CustomizerControl,
) -> Option<&'a FoundryLock> {
    document.foundry_locks.iter().find(|lock| {
        matches!(
            lock.mode,
            FoundryLockMode::Locked | FoundryLockMode::SearchProtected
        ) && lock_matches_control(lock, control)
    })
}

fn lock_matches_control(lock: &FoundryLock, control: &CustomizerControl) -> bool {
    match &lock.target {
        FoundryLockTarget::Control(control_id) => control_id == &control.id,
        FoundryLockTarget::Provider(role) | FoundryLockTarget::Role(role) => {
            matches!(&control.kind, ControlKind::ProviderGallery { role: control_role, .. } if control_role == role)
        }
        _ => false,
    }
}
