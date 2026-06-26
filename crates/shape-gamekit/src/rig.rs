#![forbid(unsafe_code)]

//! Rig artifact readiness contracts.
//!
//! These contracts describe rig evidence. They do not implement arbitrary mesh
//! auto-rigging, skin-weight generation, or solver behavior.

use serde::{Deserialize, Serialize};

use crate::lineage::{GameAssetLayerKind, GameAssetLayerRef, validate_game_asset_layer_ref};

/// Current rig artifact schema version.
pub const RIG_ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Type of asset the rig artifact describes.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RigTargetKind {
    /// Static props do not require a rig.
    StaticProp,
    /// Mechanical props may expose pivots or attachment sockets.
    MechanicalProp,
    /// Prepared templates that have authored skeleton and skin evidence.
    PreparedHumanoidTemplate,
    /// Arbitrary meshes are not auto-rigged by Shape Lab.
    ArbitraryMesh,
}

/// Skin binding readiness for a rig artifact.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SkinBindingStatus {
    /// Skinning does not apply to this artifact.
    NotApplicable,
    /// No skin binding evidence exists.
    Missing,
    /// Mechanical descriptor exists but no deforming skin is claimed.
    DescriptorOnlyNoSkinning,
    /// Bind pose and skin weights are present.
    Complete,
}

/// Readiness class returned by rig validation.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RigReadinessStatus {
    /// Rigging does not apply.
    NotApplicable,
    /// Descriptor-only sockets or pivots are valid, but not deforming rig-ready.
    DescriptorOnly,
    /// Full rig evidence is present.
    Ready,
    /// One or more blockers prevent a rig-ready claim.
    Blocked,
}

/// One joint in a skeleton template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JointDescriptor {
    /// Stable joint ID.
    pub joint_id: String,
    /// Product-facing joint label.
    pub display_name: String,
    /// Parent joint ID, or none for the root.
    #[serde(default)]
    pub parent_joint_id: Option<String>,
    /// Bind-pose translation.
    pub bind_translation: [f32; 3],
    /// Bind-pose rotation as `[x, y, z, w]`.
    pub bind_rotation_xyzw: [f32; 4],
}

/// Skeleton template contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkeletonTemplate {
    /// Stable skeleton template ID.
    pub template_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Joints in deterministic parent-before-child order.
    pub joints: Vec<JointDescriptor>,
}

/// Descriptor-only attachment socket.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttachmentSocketDescriptor {
    /// Stable socket ID.
    pub socket_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Optional joint this socket follows.
    #[serde(default)]
    pub parent_joint_id: Option<String>,
    /// Local transform translation.
    pub local_translation: [f32; 3],
    /// Local transform rotation as `[x, y, z, w]`.
    pub local_rotation_xyzw: [f32; 4],
}

/// One rig artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RigArtifact {
    /// Rig artifact schema version.
    pub schema_version: u32,
    /// Stable artifact ID.
    pub artifact_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Asset kind this rig describes.
    pub target_kind: RigTargetKind,
    /// Lineage binding to the frozen mesh/topology.
    pub mesh_layer: GameAssetLayerRef,
    /// Skeleton template, when one exists.
    #[serde(default)]
    pub skeleton: Option<SkeletonTemplate>,
    /// Descriptor-only sockets.
    #[serde(default)]
    pub attachment_sockets: Vec<AttachmentSocketDescriptor>,
    /// Skin binding readiness.
    pub skin_binding_status: SkinBindingStatus,
    /// True only when bind pose evidence is present.
    pub bind_pose_evidence: bool,
    /// True only when skin weight evidence is present.
    pub skin_weight_evidence: bool,
    /// Any arbitrary auto-rigging claim must be rejected.
    #[serde(default)]
    pub arbitrary_auto_rig_claim: bool,
}

/// Rig validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RigValidationReport {
    /// Readiness status.
    pub status: RigReadinessStatus,
    /// Stable check codes that passed.
    pub passed_checks: Vec<String>,
    /// Stable blocker codes.
    pub blockers: Vec<String>,
}

impl RigValidationReport {
    /// Return true only for full deforming rig readiness.
    #[must_use]
    pub fn is_rig_ready(&self) -> bool {
        self.status == RigReadinessStatus::Ready && self.blockers.is_empty()
    }
}

/// Validate rig artifact evidence without generating rig data.
#[must_use]
pub fn validate_rig_artifact(artifact: &RigArtifact) -> RigValidationReport {
    let mut passed_checks = Vec::new();
    let mut blockers = Vec::new();

    if artifact.schema_version == RIG_ARTIFACT_SCHEMA_VERSION {
        passed_checks.push("rig_schema_version_supported".to_owned());
    } else {
        blockers.push("unsupported_rig_artifact_schema".to_owned());
    }
    require_non_empty(
        &artifact.artifact_id,
        "missing_rig_artifact_id",
        &mut blockers,
    );
    require_non_empty(
        &artifact.display_name,
        "missing_rig_display_name",
        &mut blockers,
    );

    let lineage_report = validate_game_asset_layer_ref(&artifact.mesh_layer);
    if !lineage_report.valid {
        blockers.extend(lineage_report.issue_codes);
    } else if artifact.mesh_layer.layer == GameAssetLayerKind::Mesh {
        passed_checks.push("rig_mesh_lineage_valid".to_owned());
    } else {
        blockers.push("rig_must_bind_to_mesh_layer".to_owned());
    }

    if artifact.arbitrary_auto_rig_claim || artifact.target_kind == RigTargetKind::ArbitraryMesh {
        blockers.push("arbitrary_mesh_auto_rig_not_supported".to_owned());
    }

    validate_skeleton(
        artifact.skeleton.as_ref(),
        &mut passed_checks,
        &mut blockers,
    );
    validate_sockets(
        &artifact.attachment_sockets,
        &mut passed_checks,
        &mut blockers,
    );

    let status = match artifact.target_kind {
        RigTargetKind::StaticProp => {
            if artifact.skin_binding_status != SkinBindingStatus::NotApplicable {
                blockers.push("static_prop_skin_binding_must_be_not_applicable".to_owned());
            }
            if blockers.is_empty() {
                RigReadinessStatus::NotApplicable
            } else {
                RigReadinessStatus::Blocked
            }
        }
        RigTargetKind::MechanicalProp => {
            if artifact.skin_binding_status != SkinBindingStatus::DescriptorOnlyNoSkinning {
                blockers.push("mechanical_prop_must_be_descriptor_only_no_skinning".to_owned());
            }
            if artifact.attachment_sockets.is_empty() && artifact.skeleton.is_none() {
                blockers.push("mechanical_descriptor_missing".to_owned());
            }
            if blockers.is_empty() {
                RigReadinessStatus::DescriptorOnly
            } else {
                RigReadinessStatus::Blocked
            }
        }
        RigTargetKind::PreparedHumanoidTemplate => {
            if artifact.skeleton.is_none() {
                blockers.push("prepared_humanoid_skeleton_missing".to_owned());
            }
            if !artifact.bind_pose_evidence {
                blockers.push("prepared_humanoid_bind_pose_missing".to_owned());
            }
            if artifact.skin_binding_status != SkinBindingStatus::Complete
                || !artifact.skin_weight_evidence
            {
                blockers.push("prepared_humanoid_skin_weights_missing".to_owned());
            }
            if blockers.is_empty() {
                RigReadinessStatus::Ready
            } else {
                RigReadinessStatus::Blocked
            }
        }
        RigTargetKind::ArbitraryMesh => RigReadinessStatus::Blocked,
    };

    RigValidationReport {
        status,
        passed_checks,
        blockers,
    }
}

fn validate_skeleton(
    skeleton: Option<&SkeletonTemplate>,
    passed_checks: &mut Vec<String>,
    blockers: &mut Vec<String>,
) {
    let Some(skeleton) = skeleton else {
        return;
    };
    require_non_empty(
        &skeleton.template_id,
        "missing_skeleton_template_id",
        blockers,
    );
    require_non_empty(
        &skeleton.display_name,
        "missing_skeleton_display_name",
        blockers,
    );
    if skeleton.joints.is_empty() {
        blockers.push("skeleton_joints_missing".to_owned());
        return;
    }
    let mut seen = Vec::<&str>::new();
    let mut root_count = 0_usize;
    for joint in &skeleton.joints {
        require_non_empty(&joint.joint_id, "missing_joint_id", blockers);
        require_non_empty(&joint.display_name, "missing_joint_display_name", blockers);
        if seen.contains(&joint.joint_id.as_str()) {
            blockers.push("duplicate_joint_id".to_owned());
        }
        if let Some(parent) = joint.parent_joint_id.as_deref() {
            if !seen.contains(&parent) {
                blockers.push("joint_parent_must_precede_child".to_owned());
            }
        } else {
            root_count = root_count.saturating_add(1);
        }
        if !array_is_finite(&joint.bind_translation) || !quat_is_finite(&joint.bind_rotation_xyzw) {
            blockers.push("joint_bind_pose_non_finite".to_owned());
        }
        seen.push(&joint.joint_id);
    }
    if root_count != 1 {
        blockers.push("skeleton_must_have_one_root".to_owned());
    } else {
        passed_checks.push("skeleton_joint_hierarchy_valid".to_owned());
    }
}

fn validate_sockets(
    sockets: &[AttachmentSocketDescriptor],
    passed_checks: &mut Vec<String>,
    blockers: &mut Vec<String>,
) {
    let mut ids = Vec::<&str>::new();
    for socket in sockets {
        require_non_empty(&socket.socket_id, "missing_socket_id", blockers);
        require_non_empty(
            &socket.display_name,
            "missing_socket_display_name",
            blockers,
        );
        if ids.contains(&socket.socket_id.as_str()) {
            blockers.push("duplicate_socket_id".to_owned());
        }
        if !array_is_finite(&socket.local_translation)
            || !quat_is_finite(&socket.local_rotation_xyzw)
        {
            blockers.push("socket_transform_non_finite".to_owned());
        }
        ids.push(&socket.socket_id);
    }
    if !sockets.is_empty() {
        passed_checks.push("attachment_socket_descriptors_valid".to_owned());
    }
}

fn require_non_empty(value: &str, issue: &str, blockers: &mut Vec<String>) {
    if value.trim().is_empty() {
        blockers.push(issue.to_owned());
    }
}

fn array_is_finite(values: &[f32; 3]) -> bool {
    values.iter().all(|value| value.is_finite())
}

fn quat_is_finite(values: &[f32; 4]) -> bool {
    values.iter().all(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use crate::lineage::GAME_ASSET_LAYER_REF_SCHEMA_VERSION;

    use super::*;

    #[test]
    fn static_prop_rig_is_not_applicable() {
        let report = validate_rig_artifact(&RigArtifact {
            schema_version: RIG_ARTIFACT_SCHEMA_VERSION,
            artifact_id: "rig:crate".to_owned(),
            display_name: "Crate Rig".to_owned(),
            target_kind: RigTargetKind::StaticProp,
            mesh_layer: mesh_layer(),
            skeleton: None,
            attachment_sockets: Vec::new(),
            skin_binding_status: SkinBindingStatus::NotApplicable,
            bind_pose_evidence: false,
            skin_weight_evidence: false,
            arbitrary_auto_rig_claim: false,
        });

        assert_eq!(report.status, RigReadinessStatus::NotApplicable);
        assert!(!report.is_rig_ready());
    }

    #[test]
    fn mechanical_prop_allows_descriptor_only_sockets_without_skinning() {
        let report = validate_rig_artifact(&RigArtifact {
            schema_version: RIG_ARTIFACT_SCHEMA_VERSION,
            artifact_id: "rig:door".to_owned(),
            display_name: "Door Pivot".to_owned(),
            target_kind: RigTargetKind::MechanicalProp,
            mesh_layer: mesh_layer(),
            skeleton: None,
            attachment_sockets: vec![AttachmentSocketDescriptor {
                socket_id: "hinge".to_owned(),
                display_name: "Hinge".to_owned(),
                parent_joint_id: None,
                local_translation: [0.0, 0.0, 0.0],
                local_rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
            }],
            skin_binding_status: SkinBindingStatus::DescriptorOnlyNoSkinning,
            bind_pose_evidence: false,
            skin_weight_evidence: false,
            arbitrary_auto_rig_claim: false,
        });

        assert_eq!(report.status, RigReadinessStatus::DescriptorOnly);
        assert!(!report.is_rig_ready());
    }

    #[test]
    fn prepared_humanoid_requires_skeleton_bind_pose_and_skin_weights() {
        let mut artifact = prepared_humanoid_rig();
        artifact.skin_weight_evidence = false;

        let report = validate_rig_artifact(&artifact);

        assert_eq!(report.status, RigReadinessStatus::Blocked);
        assert!(
            report
                .blockers
                .contains(&"prepared_humanoid_skin_weights_missing".to_owned())
        );

        artifact.skin_weight_evidence = true;
        assert!(validate_rig_artifact(&artifact).is_rig_ready());
    }

    #[test]
    fn arbitrary_auto_rig_claim_is_rejected() {
        let mut artifact = prepared_humanoid_rig();
        artifact.arbitrary_auto_rig_claim = true;

        let report = validate_rig_artifact(&artifact);

        assert!(
            report
                .blockers
                .contains(&"arbitrary_mesh_auto_rig_not_supported".to_owned())
        );
    }

    fn mesh_layer() -> GameAssetLayerRef {
        GameAssetLayerRef {
            schema_version: GAME_ASSET_LAYER_REF_SCHEMA_VERSION,
            layer: GameAssetLayerKind::Mesh,
            artifact_ref: "model-package".to_owned(),
            frozen_mesh_fingerprint: "mesh:abc".to_owned(),
            topology_fingerprint: "topology:abc".to_owned(),
            parent_artifact_ref: None,
        }
    }

    fn prepared_humanoid_rig() -> RigArtifact {
        RigArtifact {
            schema_version: RIG_ARTIFACT_SCHEMA_VERSION,
            artifact_id: "rig:hero".to_owned(),
            display_name: "Prepared Hero Rig".to_owned(),
            target_kind: RigTargetKind::PreparedHumanoidTemplate,
            mesh_layer: mesh_layer(),
            skeleton: Some(SkeletonTemplate {
                template_id: "skeleton:hero".to_owned(),
                display_name: "Hero Skeleton".to_owned(),
                joints: vec![
                    JointDescriptor {
                        joint_id: "root".to_owned(),
                        display_name: "Root".to_owned(),
                        parent_joint_id: None,
                        bind_translation: [0.0, 0.0, 0.0],
                        bind_rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                    },
                    JointDescriptor {
                        joint_id: "spine".to_owned(),
                        display_name: "Spine".to_owned(),
                        parent_joint_id: Some("root".to_owned()),
                        bind_translation: [0.0, 1.0, 0.0],
                        bind_rotation_xyzw: [0.0, 0.0, 0.0, 1.0],
                    },
                ],
            }),
            attachment_sockets: Vec::new(),
            skin_binding_status: SkinBindingStatus::Complete,
            bind_pose_evidence: true,
            skin_weight_evidence: true,
            arbitrary_auto_rig_claim: false,
        }
    }
}
