#![forbid(unsafe_code)]

//! Motion artifact readiness contracts.
//!
//! This module validates motion evidence only. It does not generate animation
//! curves, solve locomotion, or retarget humanoid clips.

use serde::{Deserialize, Serialize};

use crate::lineage::{
    GameAssetLayerKind, GameAssetLayerRef, motion_binds_to_rig, validate_game_asset_layer_ref,
};

/// Current motion artifact schema version.
pub const MOTION_ARTIFACT_SCHEMA_VERSION: u32 = 1;

/// Motion clip evidence status.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionClipEvidenceStatus {
    /// No clip evidence exists.
    Missing,
    /// Descriptor-only placeholder.
    DescriptorOnly,
    /// Authored clip evidence exists.
    AuthoredClipEvidence,
}

/// Overall motion readiness.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionReadinessStatus {
    /// Motion does not apply to this asset.
    NotApplicable,
    /// Motion descriptors exist, but clips are not ready.
    DescriptorOnly,
    /// Rig and clip evidence are present.
    Ready,
    /// One or more blockers prevent motion readiness.
    Blocked,
}

/// One motion clip descriptor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionClipDescriptor {
    /// Stable clip ID.
    pub clip_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Source note, for example authored package or capture provenance.
    pub source: String,
    /// Duration in seconds.
    pub duration_seconds: f32,
    /// Number of frames in the evidence clip.
    pub frame_count: u32,
    /// Clip evidence reference, when present.
    #[serde(default)]
    pub evidence_ref: Option<String>,
    /// Clip evidence status.
    pub evidence_status: MotionClipEvidenceStatus,
}

/// One motion artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MotionArtifact {
    /// Motion artifact schema version.
    pub schema_version: u32,
    /// Stable artifact ID.
    pub artifact_id: String,
    /// Product-facing label.
    pub display_name: String,
    /// Motion layer lineage.
    pub motion_layer: GameAssetLayerRef,
    /// Rig artifact reference this motion is authored against.
    pub rig_artifact_ref: String,
    /// True when validated rig evidence is available to this motion artifact.
    pub rig_evidence_ready: bool,
    /// Clip descriptors.
    pub clips: Vec<MotionClipDescriptor>,
    /// True would overclaim generated curve support and is rejected.
    #[serde(default)]
    pub generated_animation_curve_claim: bool,
    /// True would overclaim humanoid retargeting support and is rejected.
    #[serde(default)]
    pub humanoid_retargeter_claim: bool,
    /// Whether this artifact attempts a motion-ready claim.
    #[serde(default)]
    pub motion_ready_claim: bool,
}

/// Motion validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MotionValidationReport {
    /// Overall status.
    pub status: MotionReadinessStatus,
    /// Stable passed check codes.
    pub passed_checks: Vec<String>,
    /// Stable blocker codes.
    pub blockers: Vec<String>,
}

impl MotionValidationReport {
    /// Return true only when motion-ready evidence is complete.
    #[must_use]
    pub fn is_motion_ready(&self) -> bool {
        self.status == MotionReadinessStatus::Ready && self.blockers.is_empty()
    }
}

/// Validate motion artifact evidence without generating animation data.
#[must_use]
pub fn validate_motion_artifact(artifact: &MotionArtifact) -> MotionValidationReport {
    let mut passed_checks = Vec::new();
    let mut blockers = Vec::new();

    if artifact.schema_version == MOTION_ARTIFACT_SCHEMA_VERSION {
        passed_checks.push("motion_schema_version_supported".to_owned());
    } else {
        blockers.push("unsupported_motion_artifact_schema".to_owned());
    }
    require_non_empty(
        &artifact.artifact_id,
        "missing_motion_artifact_id",
        &mut blockers,
    );
    require_non_empty(
        &artifact.display_name,
        "missing_motion_display_name",
        &mut blockers,
    );
    require_non_empty(
        &artifact.rig_artifact_ref,
        "missing_motion_rig_artifact_ref",
        &mut blockers,
    );

    let lineage_report = validate_game_asset_layer_ref(&artifact.motion_layer);
    if !lineage_report.valid {
        blockers.extend(lineage_report.issue_codes);
    } else if artifact.motion_layer.layer != GameAssetLayerKind::Motion {
        blockers.push("motion_layer_kind_invalid".to_owned());
    } else if !motion_binds_to_rig(&artifact.motion_layer, &artifact.rig_artifact_ref) {
        blockers.push("motion_layer_not_bound_to_rig".to_owned());
    } else {
        passed_checks.push("motion_rig_lineage_valid".to_owned());
    }

    if artifact.generated_animation_curve_claim {
        blockers.push("animation_curve_generation_not_supported".to_owned());
    }
    if artifact.humanoid_retargeter_claim {
        blockers.push("humanoid_retargeting_not_supported".to_owned());
    }
    if artifact.motion_ready_claim && !artifact.rig_evidence_ready {
        blockers.push("motion_ready_requires_rig_evidence".to_owned());
    }

    validate_clips(
        &artifact.clips,
        artifact.motion_ready_claim,
        &mut passed_checks,
        &mut blockers,
    );

    let has_authored_clip = artifact
        .clips
        .iter()
        .any(|clip| clip.evidence_status == MotionClipEvidenceStatus::AuthoredClipEvidence);
    let status = if !artifact.motion_ready_claim && artifact.clips.is_empty() && blockers.is_empty()
    {
        MotionReadinessStatus::NotApplicable
    } else if blockers.is_empty()
        && artifact.motion_ready_claim
        && artifact.rig_evidence_ready
        && has_authored_clip
    {
        MotionReadinessStatus::Ready
    } else if blockers.is_empty() && !artifact.clips.is_empty() {
        MotionReadinessStatus::DescriptorOnly
    } else {
        MotionReadinessStatus::Blocked
    };

    MotionValidationReport {
        status,
        passed_checks,
        blockers,
    }
}

fn validate_clips(
    clips: &[MotionClipDescriptor],
    motion_ready_claim: bool,
    passed_checks: &mut Vec<String>,
    blockers: &mut Vec<String>,
) {
    if motion_ready_claim && clips.is_empty() {
        blockers.push("motion_ready_requires_clip_evidence".to_owned());
        return;
    }
    let mut ids = Vec::<&str>::new();
    for clip in clips {
        require_non_empty(&clip.clip_id, "missing_motion_clip_id", blockers);
        require_non_empty(
            &clip.display_name,
            "missing_motion_clip_display_name",
            blockers,
        );
        require_non_empty(&clip.source, "missing_motion_clip_source", blockers);
        if ids.contains(&clip.clip_id.as_str()) {
            blockers.push("duplicate_motion_clip_id".to_owned());
        }
        if !clip.duration_seconds.is_finite() || clip.duration_seconds <= 0.0 {
            blockers.push("invalid_motion_clip_duration".to_owned());
        }
        if clip.frame_count == 0 {
            blockers.push("invalid_motion_clip_frame_count".to_owned());
        }
        if motion_ready_claim
            && (clip.evidence_status != MotionClipEvidenceStatus::AuthoredClipEvidence
                || clip
                    .evidence_ref
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty()))
        {
            blockers.push("motion_ready_requires_clip_evidence".to_owned());
        }
        ids.push(&clip.clip_id);
    }
    if !clips.is_empty() {
        passed_checks.push("motion_clip_descriptors_valid".to_owned());
    }
}

fn require_non_empty(value: &str, issue: &str, blockers: &mut Vec<String>) {
    if value.trim().is_empty() {
        blockers.push(issue.to_owned());
    }
}

#[cfg(test)]
mod tests {
    use crate::lineage::{
        GAME_ASSET_LAYER_REF_SCHEMA_VERSION, GameAssetLayerKind, GameAssetLayerRef,
    };

    use super::*;

    #[test]
    fn motion_ready_requires_rig_and_clip_evidence() {
        let mut artifact = valid_motion_artifact();
        artifact.rig_evidence_ready = false;

        let report = validate_motion_artifact(&artifact);

        assert_eq!(report.status, MotionReadinessStatus::Blocked);
        assert!(
            report
                .blockers
                .contains(&"motion_ready_requires_rig_evidence".to_owned())
        );

        artifact.rig_evidence_ready = true;
        assert!(validate_motion_artifact(&artifact).is_motion_ready());
    }

    #[test]
    fn descriptor_only_motion_does_not_claim_ready() {
        let mut artifact = valid_motion_artifact();
        artifact.motion_ready_claim = false;
        artifact.clips[0].evidence_status = MotionClipEvidenceStatus::DescriptorOnly;
        artifact.clips[0].evidence_ref = None;

        let report = validate_motion_artifact(&artifact);

        assert_eq!(report.status, MotionReadinessStatus::DescriptorOnly);
        assert!(!report.is_motion_ready());
    }

    #[test]
    fn curve_generation_and_retargeter_claims_are_rejected() {
        let mut artifact = valid_motion_artifact();
        artifact.generated_animation_curve_claim = true;
        artifact.humanoid_retargeter_claim = true;

        let report = validate_motion_artifact(&artifact);

        assert!(
            report
                .blockers
                .contains(&"animation_curve_generation_not_supported".to_owned())
        );
        assert!(
            report
                .blockers
                .contains(&"humanoid_retargeting_not_supported".to_owned())
        );
    }

    #[test]
    fn motion_lineage_must_bind_to_rig() {
        let mut artifact = valid_motion_artifact();
        artifact.motion_layer.parent_artifact_ref = Some("rig/other.json".to_owned());

        let report = validate_motion_artifact(&artifact);

        assert!(
            report
                .blockers
                .contains(&"motion_layer_not_bound_to_rig".to_owned())
        );
    }

    fn valid_motion_artifact() -> MotionArtifact {
        MotionArtifact {
            schema_version: MOTION_ARTIFACT_SCHEMA_VERSION,
            artifact_id: "motion:idle".to_owned(),
            display_name: "Idle Motion".to_owned(),
            motion_layer: GameAssetLayerRef {
                schema_version: GAME_ASSET_LAYER_REF_SCHEMA_VERSION,
                layer: GameAssetLayerKind::Motion,
                artifact_ref: "motion/idle.json".to_owned(),
                frozen_mesh_fingerprint: "mesh:abc".to_owned(),
                topology_fingerprint: "topology:abc".to_owned(),
                parent_artifact_ref: Some("rig/rig-artifact.json".to_owned()),
            },
            rig_artifact_ref: "rig/rig-artifact.json".to_owned(),
            rig_evidence_ready: true,
            clips: vec![MotionClipDescriptor {
                clip_id: "idle".to_owned(),
                display_name: "Idle".to_owned(),
                source: "authored test fixture".to_owned(),
                duration_seconds: 1.0,
                frame_count: 30,
                evidence_ref: Some("motion/idle.clip.json".to_owned()),
                evidence_status: MotionClipEvidenceStatus::AuthoredClipEvidence,
            }],
            generated_animation_curve_claim: false,
            humanoid_retargeter_claim: false,
            motion_ready_claim: true,
        }
    }
}
