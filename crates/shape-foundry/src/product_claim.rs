//! Product claim and report include validation.

use serde::{Deserialize, Serialize};

use crate::{GeometryExportReport, GeometryExportStatus};

/// Capability include/exclude summary shared by export and proof reports.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductClaimIncludes {
    /// Geometry is included.
    pub includes_geometry: bool,
    /// UV data is included.
    pub includes_uvs: bool,
    /// Texture files are included.
    pub includes_textures: bool,
    /// Material looks are included.
    pub includes_material_looks: bool,
    /// Collision data is included.
    pub includes_collision: bool,
    /// Gameplay metadata is included.
    pub includes_gameplay_metadata: bool,
    /// Rig data is included.
    pub includes_rig: bool,
    /// Skinning data is included.
    pub includes_skinning: bool,
    /// Animation data is included.
    pub includes_animation: bool,
    /// Terrain collision is included.
    pub includes_terrain_collision: bool,
    /// Godot scene output is included.
    pub includes_godot_scene: bool,
    /// Output is game-ready.
    pub game_ready: bool,
    /// Human review remains required.
    pub human_review_required: bool,
}

impl Default for ProductClaimIncludes {
    fn default() -> Self {
        Self {
            includes_geometry: false,
            includes_uvs: false,
            includes_textures: false,
            includes_material_looks: false,
            includes_collision: false,
            includes_gameplay_metadata: false,
            includes_rig: false,
            includes_skinning: false,
            includes_animation: false,
            includes_terrain_collision: false,
            includes_godot_scene: false,
            game_ready: false,
            human_review_required: true,
        }
    }
}

impl From<&GeometryExportReport> for ProductClaimIncludes {
    fn from(report: &GeometryExportReport) -> Self {
        Self {
            includes_geometry: report.status == GeometryExportStatus::Passed,
            includes_uvs: report.includes_uvs,
            includes_textures: report.includes_textures,
            includes_material_looks: report.includes_material_looks,
            includes_collision: report.includes_collision,
            includes_gameplay_metadata: false,
            includes_rig: report.includes_rig,
            includes_skinning: false,
            includes_animation: report.includes_animation,
            includes_terrain_collision: false,
            includes_godot_scene: false,
            game_ready: report.game_ready,
            human_review_required: report.human_review_required,
        }
    }
}

/// Product claim validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductClaimIssue {
    /// Subject path.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Message.
    pub message: String,
}

/// Product claim validation report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProductClaimValidationReport {
    /// Issues discovered during validation.
    pub issues: Vec<ProductClaimIssue>,
}

impl ProductClaimValidationReport {
    /// Return true when no issues were discovered.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(
        &mut self,
        subject: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.issues.push(ProductClaimIssue {
            subject: subject.into(),
            code: code.into(),
            message: message.into(),
        });
    }
}

/// Validate include flags for a report that has not passed later phase gates.
#[must_use]
pub fn validate_product_claim_includes(
    subject: impl AsRef<str>,
    includes: &ProductClaimIncludes,
) -> ProductClaimValidationReport {
    let subject = subject.as_ref();
    let mut report = ProductClaimValidationReport::default();

    for (enabled, suffix, code, message) in [
        (
            includes.includes_uvs,
            "includes_uvs",
            "product_claim_uvs_forbidden",
            "UV support is not approved.",
        ),
        (
            includes.includes_textures,
            "includes_textures",
            "product_claim_textures_forbidden",
            "Texture output is not approved.",
        ),
        (
            includes.includes_material_looks,
            "includes_material_looks",
            "product_claim_material_looks_forbidden",
            "Material looks are not approved.",
        ),
        (
            includes.includes_collision,
            "includes_collision",
            "product_claim_collision_forbidden",
            "Collision output is not approved.",
        ),
        (
            includes.includes_gameplay_metadata,
            "includes_gameplay_metadata",
            "product_claim_gameplay_metadata_forbidden",
            "Gameplay metadata is not approved.",
        ),
        (
            includes.includes_rig,
            "includes_rig",
            "product_claim_rig_forbidden",
            "Rig output is not approved.",
        ),
        (
            includes.includes_skinning,
            "includes_skinning",
            "product_claim_skinning_forbidden",
            "Skinning output is not approved.",
        ),
        (
            includes.includes_animation,
            "includes_animation",
            "product_claim_animation_forbidden",
            "Animation output is not approved.",
        ),
        (
            includes.includes_terrain_collision,
            "includes_terrain_collision",
            "product_claim_terrain_collision_forbidden",
            "Terrain collision output is not approved.",
        ),
        (
            includes.includes_godot_scene,
            "includes_godot_scene",
            "product_claim_godot_scene_forbidden",
            "Godot scene output is not approved.",
        ),
    ] {
        if enabled {
            report.push(format!("{subject}.{suffix}"), code, message);
        }
    }

    if includes.game_ready {
        report.push(
            format!("{subject}.game_ready"),
            "product_claim_game_ready_forbidden",
            "Game-ready status is not approved.",
        );
    }
    if !includes.human_review_required {
        report.push(
            format!("{subject}.human_review_required"),
            "product_claim_human_review_required",
            "Human review must remain required.",
        );
    }

    report
}

/// Scan product-facing text for blocked claims.
#[must_use]
pub fn scan_product_claim_text(
    subject: impl AsRef<str>,
    text: impl AsRef<str>,
) -> ProductClaimValidationReport {
    let subject = subject.as_ref();
    let mut report = ProductClaimValidationReport::default();
    for (index, line) in text.as_ref().lines().enumerate() {
        let normalized = line.to_ascii_lowercase();
        for term in BLOCKED_CLAIM_TERMS {
            if normalized.contains(term) && !line_caveats_blocked_claim(&normalized) {
                report.push(
                    format!("{subject}:{}", index + 1),
                    "product_claim_text_overclaim",
                    format!("Blocked claim must be explicitly caveated: {term}"),
                );
            }
        }
    }
    report
}

const BLOCKED_CLAIM_TERMS: &[&str] = &[
    "godot-ready",
    "game-ready",
    "rigged",
    "animated",
    "collision-enabled",
    "textured",
    "uv unwrapped",
    "uv-unwrapped",
    "terrain-ready",
    "public catalog publishing",
    "reviewed kit",
];

fn line_caveats_blocked_claim(line: &str) -> bool {
    [
        "not ",
        "no ",
        "without",
        "blocked",
        "forbidden",
        "excluded",
        "false",
        "must not",
        "does not",
        "do not",
        "required before",
        "until",
        "separate gate",
        "out of scope",
        "unsupported",
    ]
    .iter()
    .any(|marker| line.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GeometryExportReport, GeometryExportStatus};

    fn codes(report: &ProductClaimValidationReport) -> Vec<&str> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect()
    }

    #[test]
    fn geometry_only_export_includes_pass_claim_gate() {
        let export = GeometryExportReport {
            status: GeometryExportStatus::Passed,
            output_files: vec!["asset.glb".to_owned()],
            source_plan_id: Some("box".to_owned()),
            primitive_count: 1,
            mesh_count: 1,
            triangle_count: 12,
            warning_count: 0,
            blockers: Vec::new(),
            includes_uvs: false,
            includes_textures: false,
            includes_material_looks: false,
            includes_collision: false,
            includes_rig: false,
            includes_animation: false,
            game_ready: false,
            human_review_required: true,
        };

        let includes = ProductClaimIncludes::from(&export);

        assert!(includes.includes_geometry);
        assert!(validate_product_claim_includes("geometry_export", &includes).is_valid());
    }

    #[test]
    fn game_ready_true_report_is_rejected() {
        let includes = ProductClaimIncludes {
            includes_geometry: true,
            game_ready: true,
            ..ProductClaimIncludes::default()
        };

        let report = validate_product_claim_includes("report", &includes);

        assert!(codes(&report).contains(&"product_claim_game_ready_forbidden"));
    }

    #[test]
    fn textured_claim_report_is_rejected() {
        let includes = ProductClaimIncludes {
            includes_textures: true,
            includes_material_looks: true,
            ..ProductClaimIncludes::default()
        };

        let report = validate_product_claim_includes("report", &includes);
        let codes = codes(&report);

        assert!(codes.contains(&"product_claim_textures_forbidden"));
        assert!(codes.contains(&"product_claim_material_looks_forbidden"));
    }

    #[test]
    fn blocked_godot_proof_report_passes_claim_gate() {
        let includes = ProductClaimIncludes {
            includes_geometry: true,
            includes_godot_scene: false,
            game_ready: false,
            human_review_required: true,
            ..ProductClaimIncludes::default()
        };

        assert!(validate_product_claim_includes("godot_proof", &includes).is_valid());
    }

    #[test]
    fn text_scan_allows_negative_claims_and_rejects_positive_claims() {
        let safe = "Geometry-only GLB is not game-ready. No rigging included. UV editing is not supported.";
        assert!(scan_product_claim_text("safe", safe).is_valid());

        let unsafe_text = "This is a Godot-ready textured public catalog publishing flow.";
        let report = scan_product_claim_text("unsafe", unsafe_text);

        assert!(!report.is_valid());
        assert!(codes(&report).contains(&"product_claim_text_overclaim"));
    }

    #[test]
    fn product_claim_docs_do_not_overclaim() {
        for (subject, text) in [
            (
                "docs/PRODUCT_CLAIM_GATE.md",
                include_str!("../../../docs/PRODUCT_CLAIM_GATE.md"),
            ),
            (
                "docs/EXPORT_REPORT_INCLUDES_CONTRACT.md",
                include_str!("../../../docs/EXPORT_REPORT_INCLUDES_CONTRACT.md"),
            ),
        ] {
            let report = scan_product_claim_text(subject, text);
            assert!(report.is_valid(), "{subject}: {report:?}");
        }
    }
}
