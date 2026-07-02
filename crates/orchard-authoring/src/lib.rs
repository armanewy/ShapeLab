#![forbid(unsafe_code)]

//! Canonical semantic authoring operation log for Object Orchard.
//!
//! This crate defines the replay boundary for product-visible edits. UI tools
//! should emit typed `AuthoringOp`s instead of mutating recipe state directly.
//! v0 implements `SetProperty` as the first meaningful operation and keeps the
//! other operation families as replayable shell operations.

use orchard_asset::{
    AssetRecipe, AuthoringOpId, ParameterId, PartInstanceId, ReviewTier, RevisionId, set_scalar,
    validate_asset_recipe,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable authoring operation log.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoringOpLog {
    /// Stable log identifier.
    pub log_id: String,
    /// Optional source recipe revision.
    pub source_revision: Option<RevisionId>,
    /// Ordered log entries.
    pub entries: Vec<AuthoringOpLogEntry>,
}

impl AuthoringOpLog {
    /// Create an empty log.
    #[must_use]
    pub fn new(log_id: impl Into<String>) -> Self {
        Self {
            log_id: log_id.into(),
            source_revision: None,
            entries: Vec::new(),
        }
    }
}

/// One replayable authoring operation entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoringOpLogEntry {
    /// Stable authoring operation ID.
    pub op_id: AuthoringOpId,
    /// Monotonic sequence number in the log.
    pub sequence: u64,
    /// Source that produced the operation.
    pub source: AuthoringOpSource,
    /// Operation payload.
    pub op: AuthoringOp,
    /// Expected effect class.
    pub effect: AuthoringEffect,
    /// Hash before applying the operation.
    pub before_hash: RecipeHash,
    /// Hash after applying the operation.
    pub after_hash: RecipeHash,
    /// Operation outcome.
    pub outcome: AuthoringOutcome,
}

/// Source of an authoring operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthoringOpSource {
    /// Product UI control.
    UserControl,
    /// Offline ObjectPlan draft.
    ObjectPlanDraft,
    /// Internal tool.
    InternalTool,
    /// Replay process.
    Replay,
}

/// High-level effect class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthoringEffect {
    /// Property value changed.
    SetProperty,
    /// Property value reset.
    ResetProperty,
    /// Node graph shell operation.
    NodeGraphShell,
    /// Relationship shell operation.
    RelationshipShell,
    /// Placement policy shell operation.
    PlacementPolicyShell,
    /// Scale policy shell operation.
    ScalePolicyShell,
    /// Orientation policy shell operation.
    OrientationPolicyShell,
    /// Review tier changed.
    SetReviewTier,
}

/// Canonical authoring operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthoringOp {
    /// Set a scalar property through a stable descriptor path.
    SetProperty {
        /// Target parameter.
        parameter: ParameterId,
        /// Stable scalar path.
        path: String,
        /// New scalar value.
        value: f32,
    },
    /// Reset a scalar property to an authored default.
    ResetProperty {
        /// Target parameter.
        parameter: ParameterId,
        /// Stable scalar path.
        path: String,
        /// Authored default value.
        default_value: f32,
    },
    /// Shell operation for future node creation.
    AddNode {
        /// Product-safe label.
        label: String,
    },
    /// Shell operation for future node removal.
    RemoveNode {
        /// Target instance.
        instance: PartInstanceId,
    },
    /// Shell operation for future relationship attachment.
    AttachNode {
        /// Parent instance.
        parent: PartInstanceId,
        /// Child instance.
        child: PartInstanceId,
    },
    /// Shell operation for future relationship detach.
    DetachNode {
        /// Child instance.
        child: PartInstanceId,
    },
    /// Shell operation for future placement policy changes.
    SetPlacementPolicy {
        /// Policy label.
        policy: String,
    },
    /// Shell operation for future scale policy changes.
    SetScalePolicy {
        /// Policy label.
        policy: String,
    },
    /// Shell operation for future orientation policy changes.
    SetOrientationPolicy {
        /// Policy label.
        policy: String,
    },
    /// Set the review tier while validation keeps publishing blocked.
    SetReviewTier {
        /// Target tier.
        tier: ReviewTier,
    },
}

impl AuthoringOp {
    fn effect(&self) -> AuthoringEffect {
        match self {
            Self::SetProperty { .. } => AuthoringEffect::SetProperty,
            Self::ResetProperty { .. } => AuthoringEffect::ResetProperty,
            Self::AddNode { .. } | Self::RemoveNode { .. } => AuthoringEffect::NodeGraphShell,
            Self::AttachNode { .. } | Self::DetachNode { .. } => AuthoringEffect::RelationshipShell,
            Self::SetPlacementPolicy { .. } => AuthoringEffect::PlacementPolicyShell,
            Self::SetScalePolicy { .. } => AuthoringEffect::ScalePolicyShell,
            Self::SetOrientationPolicy { .. } => AuthoringEffect::OrientationPolicyShell,
            Self::SetReviewTier { .. } => AuthoringEffect::SetReviewTier,
        }
    }
}

/// Outcome for one operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthoringOutcome {
    /// Operation changed recipe state.
    Applied,
    /// Operation replayed as a shell/no-op in v0.
    AppliedShellNoOp,
}

/// Stable recipe hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeHash {
    /// Hex digest when known.
    pub digest: Option<String>,
    /// Whether the digest covers the complete serialized recipe.
    pub complete: bool,
}

impl RecipeHash {
    /// Compute a deterministic hash of a recipe.
    pub fn complete(recipe: &AssetRecipe) -> Result<Self, AuthoringError> {
        let json = serde_json::to_vec(recipe).map_err(AuthoringError::SerializeRecipe)?;
        Ok(Self {
            digest: Some(blake3::hash(&json).to_hex().to_string()),
            complete: true,
        })
    }
}

/// Replay validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayValidationReport {
    /// Whether replay passed validation.
    pub accepted: bool,
    /// Validation or replay issues.
    pub issues: Vec<ReplayValidationIssue>,
}

impl ReplayValidationReport {
    fn accepted() -> Self {
        Self {
            accepted: true,
            issues: Vec::new(),
        }
    }

    fn rejected(issue: ReplayValidationIssue) -> Self {
        Self {
            accepted: false,
            issues: vec![issue],
        }
    }
}

/// Replay validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayValidationIssue {
    /// Stable issue code.
    pub code: String,
    /// Product-safe message.
    pub message: String,
}

/// Successful replay result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoringReplayOutcome {
    /// Replayed recipe.
    pub recipe: AssetRecipe,
    /// Replay report.
    pub report: ReplayValidationReport,
}

/// Single operation result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthoringApplyOutcome {
    /// Edited recipe.
    pub recipe: AssetRecipe,
    /// Log entry for the operation.
    pub entry: AuthoringOpLogEntry,
    /// Validation report.
    pub report: ReplayValidationReport,
}

/// Rejected authoring operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoringRejection {
    /// Rejection report.
    pub report: ReplayValidationReport,
}

/// Error while authoring.
#[derive(Debug, Error)]
pub enum AuthoringError {
    /// Recipe serialization failed.
    #[error("failed to serialize recipe for hashing")]
    SerializeRecipe(serde_json::Error),
}

/// Drag samples reserved for later UI coalescing.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct DragSample {
    /// Sample time in milliseconds.
    pub t_millis: u64,
    /// Sample value.
    pub value: f32,
}

/// Coalesced drag operation reserved for later UI wiring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CoalescedDragOperation {
    /// First sample.
    pub first: DragSample,
    /// Last sample.
    pub last: DragSample,
    /// Operation produced by the final sample.
    pub op: AuthoringOp,
}

/// Coalesce drag samples into one final SetProperty operation.
pub fn coalesce_set_property_drag(
    parameter: ParameterId,
    path: impl Into<String>,
    samples: &[DragSample],
) -> Option<CoalescedDragOperation> {
    let first = *samples.first()?;
    let last = *samples.last()?;
    Some(CoalescedDragOperation {
        first,
        last,
        op: AuthoringOp::SetProperty {
            parameter,
            path: path.into(),
            value: last.value,
        },
    })
}

/// Apply one authoring operation to a recipe clone.
pub fn apply_authoring_op(
    recipe: &AssetRecipe,
    op: AuthoringOp,
    source: AuthoringOpSource,
) -> Result<AuthoringApplyOutcome, AuthoringRejection> {
    let before_hash = RecipeHash::complete(recipe).map_err(|error| AuthoringRejection {
        report: ReplayValidationReport::rejected(ReplayValidationIssue {
            code: "recipe_hash_failed".to_owned(),
            message: error.to_string(),
        }),
    })?;
    let mut edited = recipe.clone();
    let op_id = AuthoringOpId(edited.next_ids.authoring_op);
    let outcome = apply_to_recipe(&mut edited, op_id, &op)?;
    let validation = validate_asset_recipe(&edited);
    if !validation.is_valid() {
        return Err(AuthoringRejection {
            report: ReplayValidationReport {
                accepted: false,
                issues: validation
                    .issues
                    .into_iter()
                    .map(|issue| ReplayValidationIssue {
                        code: issue.code,
                        message: issue.message,
                    })
                    .collect(),
            },
        });
    }
    let after_hash = RecipeHash::complete(&edited).map_err(|error| AuthoringRejection {
        report: ReplayValidationReport::rejected(ReplayValidationIssue {
            code: "recipe_hash_failed".to_owned(),
            message: error.to_string(),
        }),
    })?;
    let entry = AuthoringOpLogEntry {
        op_id,
        sequence: 0,
        source,
        effect: op.effect(),
        op,
        before_hash,
        after_hash,
        outcome,
    };
    Ok(AuthoringApplyOutcome {
        recipe: edited,
        entry,
        report: ReplayValidationReport::accepted(),
    })
}

/// Replay an authoring log from a starting recipe.
pub fn replay_authoring_log(
    recipe: &AssetRecipe,
    log: &AuthoringOpLog,
) -> Result<AuthoringReplayOutcome, AuthoringRejection> {
    let mut current = recipe.clone();
    for entry in &log.entries {
        let outcome = apply_authoring_op(&current, entry.op.clone(), AuthoringOpSource::Replay)?;
        current = outcome.recipe;
    }
    Ok(AuthoringReplayOutcome {
        recipe: current,
        report: ReplayValidationReport::accepted(),
    })
}

fn apply_to_recipe(
    recipe: &mut AssetRecipe,
    op_id: AuthoringOpId,
    op: &AuthoringOp,
) -> Result<AuthoringOutcome, AuthoringRejection> {
    match op {
        AuthoringOp::SetProperty {
            parameter,
            path,
            value,
        } => {
            validate_parameter_path(recipe, *parameter, path)?;
            set_scalar(recipe, path, *value)
                .map_err(|error| rejection("set_property_failed", error))?;
            insert_authoring_shell(recipe, op_id, *parameter, None, "Set property");
            Ok(AuthoringOutcome::Applied)
        }
        AuthoringOp::ResetProperty {
            parameter,
            path,
            default_value,
        } => {
            validate_parameter_path(recipe, *parameter, path)?;
            set_scalar(recipe, path, *default_value)
                .map_err(|error| rejection("reset_property_failed", error))?;
            insert_authoring_shell(recipe, op_id, *parameter, None, "Reset property");
            Ok(AuthoringOutcome::Applied)
        }
        AuthoringOp::SetReviewTier { tier } => {
            recipe.semantic.review_state.tier = tier.clone();
            Ok(AuthoringOutcome::Applied)
        }
        AuthoringOp::RemoveNode { instance } | AuthoringOp::DetachNode { child: instance } => {
            validate_instance_exists(recipe, *instance)?;
            Ok(AuthoringOutcome::AppliedShellNoOp)
        }
        AuthoringOp::AttachNode { parent, child } => {
            validate_instance_exists(recipe, *parent)?;
            validate_instance_exists(recipe, *child)?;
            Ok(AuthoringOutcome::AppliedShellNoOp)
        }
        AuthoringOp::AddNode { .. }
        | AuthoringOp::SetPlacementPolicy { .. }
        | AuthoringOp::SetScalePolicy { .. }
        | AuthoringOp::SetOrientationPolicy { .. } => Ok(AuthoringOutcome::AppliedShellNoOp),
    }
}

fn insert_authoring_shell(
    recipe: &mut AssetRecipe,
    id: AuthoringOpId,
    parameter: ParameterId,
    instance: Option<PartInstanceId>,
    label: &str,
) {
    recipe.next_ids.authoring_op = recipe.next_ids.authoring_op.max(id.0.saturating_add(1));
    recipe.semantic.authoring_ops.insert(
        id,
        orchard_asset::AuthoringOpShell {
            id,
            target_parameter: Some(parameter),
            target_instance: instance,
            label: label.to_owned(),
        },
    );
}

fn validate_parameter_path(
    recipe: &AssetRecipe,
    parameter: ParameterId,
    path: &str,
) -> Result<(), AuthoringRejection> {
    let descriptor = recipe.parameters.get(&parameter).ok_or_else(|| {
        rejection(
            "unknown_parameter",
            format!("Unknown parameter {}", parameter.0),
        )
    })?;
    if descriptor.path != path {
        return Err(rejection(
            "parameter_path_mismatch",
            "Parameter descriptor path does not match operation path.",
        ));
    }
    Ok(())
}

fn validate_instance_exists(
    recipe: &AssetRecipe,
    instance: PartInstanceId,
) -> Result<(), AuthoringRejection> {
    if recipe.instances.contains_key(&instance) {
        Ok(())
    } else {
        Err(rejection(
            "unknown_instance",
            format!("Unknown instance {}", instance.0),
        ))
    }
}

fn rejection(code: impl Into<String>, message: impl ToString) -> AuthoringRejection {
    AuthoringRejection {
        report: ReplayValidationReport::rejected(ReplayValidationIssue {
            code: code.into(),
            message: message.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use orchard_asset::{
        AssetId, GeometryRecipe, GeometrySource, ParameterDescriptor, PartDefinition,
        PartDefinitionId, PartInstance, Transform3, definition_scalar_path, get_scalar,
    };

    use super::*;

    fn test_recipe() -> AssetRecipe {
        let definition_id = PartDefinitionId(1);
        let instance_id = PartInstanceId(1);
        let parameter_id = ParameterId(1);
        let definition = PartDefinition {
            id: definition_id,
            name: "Box".to_owned(),
            tags: BTreeSet::new(),
            geometry: GeometryRecipe {
                source: GeometrySource::RoundedBox {
                    half_extents: [1.0, 0.5, 0.25],
                    radius: 0.1,
                },
                operations: Vec::new(),
            },
            regions: BTreeMap::new(),
            sockets: BTreeMap::new(),
            local_pivot: Default::default(),
            variant_group: None,
            production_hints: None,
        };
        let instance = PartInstance {
            id: instance_id,
            definition: definition_id,
            name: "Box".to_owned(),
            parent: None,
            local_transform: Transform3::default(),
            attachment: None,
            enabled: true,
            tags: BTreeSet::new(),
            generated_by: None,
        };
        let path = definition_scalar_path(definition_id, "geometry.rounded_box.radius");
        let descriptor = ParameterDescriptor {
            id: parameter_id,
            path,
            label: "Radius".to_owned(),
            group: "Form".to_owned(),
            minimum: 0.0,
            maximum: 1.0,
            step: 0.01,
            mutation_sigma: 0.05,
            topology_changing: false,
            beginner_description: "Corner radius".to_owned(),
        };
        let mut recipe = AssetRecipe::new(AssetId(1), "Authoring");
        recipe.definitions.insert(definition.id, definition);
        recipe.instances.insert(instance.id, instance);
        recipe.root_instances.push(instance_id);
        recipe.parameters.insert(parameter_id, descriptor);
        recipe.next_ids.part_definition = 2;
        recipe.next_ids.part_instance = 2;
        recipe.next_ids.parameter = 2;
        recipe
    }

    #[test]
    fn empty_log_replays() {
        let recipe = test_recipe();
        let outcome = replay_authoring_log(&recipe, &AuthoringOpLog::new("empty")).unwrap();

        assert_eq!(outcome.recipe, recipe);
        assert!(outcome.report.accepted);
    }

    #[test]
    fn set_property_produces_one_log_entry() {
        let recipe = test_recipe();
        let path = definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius");
        let outcome = apply_authoring_op(
            &recipe,
            AuthoringOp::SetProperty {
                parameter: ParameterId(1),
                path: path.clone(),
                value: 0.25,
            },
            AuthoringOpSource::UserControl,
        )
        .unwrap();

        assert_eq!(outcome.entry.sequence, 0);
        assert_eq!(outcome.entry.effect, AuthoringEffect::SetProperty);
        assert_eq!(outcome.entry.outcome, AuthoringOutcome::Applied);
        assert!(outcome.entry.before_hash.complete);
        assert!(outcome.entry.after_hash.complete);
        assert_eq!(get_scalar(&outcome.recipe, path).unwrap(), 0.25);
        assert_eq!(outcome.recipe.semantic.authoring_ops.len(), 1);
    }

    #[test]
    fn replay_is_deterministic() {
        let recipe = test_recipe();
        let path = definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius");
        let first = apply_authoring_op(
            &recipe,
            AuthoringOp::SetProperty {
                parameter: ParameterId(1),
                path,
                value: 0.3,
            },
            AuthoringOpSource::UserControl,
        )
        .unwrap();
        let log = AuthoringOpLog {
            log_id: "one".to_owned(),
            source_revision: None,
            entries: vec![first.entry],
        };

        let replay_a = replay_authoring_log(&recipe, &log).unwrap();
        let replay_b = replay_authoring_log(&recipe, &log).unwrap();

        assert_eq!(replay_a.recipe, replay_b.recipe);
        assert_eq!(
            RecipeHash::complete(&replay_a.recipe).unwrap(),
            RecipeHash::complete(&replay_b.recipe).unwrap()
        );
    }

    #[test]
    fn invalid_target_is_rejected() {
        let recipe = test_recipe();
        let rejection = apply_authoring_op(
            &recipe,
            AuthoringOp::SetProperty {
                parameter: ParameterId(404),
                path: "definition.1.geometry.rounded_box.radius".to_owned(),
                value: 0.5,
            },
            AuthoringOpSource::UserControl,
        )
        .unwrap_err();

        assert!(!rejection.report.accepted);
        assert_eq!(rejection.report.issues[0].code, "unknown_parameter");
    }

    #[test]
    fn shell_operations_replay_without_mutating_geometry() {
        let recipe = test_recipe();
        let before = RecipeHash::complete(&recipe).unwrap();
        let outcome = apply_authoring_op(
            &recipe,
            AuthoringOp::SetPlacementPolicy {
                policy: "future fixed offset".to_owned(),
            },
            AuthoringOpSource::InternalTool,
        )
        .unwrap();

        assert_eq!(outcome.entry.outcome, AuthoringOutcome::AppliedShellNoOp);
        assert_eq!(before, outcome.entry.after_hash);
    }

    #[test]
    fn drag_samples_coalesce_to_one_operation() {
        let path = definition_scalar_path(PartDefinitionId(1), "geometry.rounded_box.radius");
        let coalesced = coalesce_set_property_drag(
            ParameterId(1),
            path.clone(),
            &[
                DragSample {
                    t_millis: 0,
                    value: 0.1,
                },
                DragSample {
                    t_millis: 16,
                    value: 0.2,
                },
            ],
        )
        .unwrap();

        assert_eq!(coalesced.first.value, 0.1);
        assert_eq!(coalesced.last.value, 0.2);
        assert_eq!(
            coalesced.op,
            AuthoringOp::SetProperty {
                parameter: ParameterId(1),
                path,
                value: 0.2
            }
        );
    }

    #[test]
    fn serde_round_trip_is_deterministic() {
        let log = AuthoringOpLog {
            log_id: "serde".to_owned(),
            source_revision: None,
            entries: vec![AuthoringOpLogEntry {
                op_id: AuthoringOpId(1),
                sequence: 0,
                source: AuthoringOpSource::UserControl,
                op: AuthoringOp::SetReviewTier {
                    tier: ReviewTier::ReviewRequired,
                },
                effect: AuthoringEffect::SetReviewTier,
                before_hash: RecipeHash {
                    digest: Some("a".repeat(64)),
                    complete: true,
                },
                after_hash: RecipeHash {
                    digest: Some("b".repeat(64)),
                    complete: true,
                },
                outcome: AuthoringOutcome::Applied,
            }],
        };

        let first = serde_json::to_string(&log).unwrap();
        let round_tripped: AuthoringOpLog = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string(&round_tripped).unwrap();

        assert_eq!(first, second);
        assert_eq!(log, round_tripped);
    }

    #[test]
    fn docs_define_authoring_as_mutation_boundary() {
        let crate_docs = include_str!("lib.rs");
        let report = include_str!("../../../docs/AUTHORING_OP_LOG_V0.md");
        let joined = format!("{crate_docs}\n{report}");

        assert!(joined.contains("mutation boundary"));
        assert!(joined.contains("typed `AuthoringOp`s"));
        assert!(joined.contains("instead of mutating recipe state directly"));
        assert!(joined.contains("no app UI wiring"));
    }
}
