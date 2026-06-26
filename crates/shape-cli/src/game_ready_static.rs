use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::ValueEnum;
use glam::Vec3;
use image::{Rgba, RgbaImage, imageops::FilterType};
use serde::{Deserialize, Serialize};
use shape_asset::Frame3;
use shape_compile::export::{verify_model_package, write_grouped_obj_export, write_model_package};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_core::Aabb;
use shape_foundry::{FoundryCompilationOutput, compile_foundry_document};
use shape_foundry_catalog::{FoundryFixtureCatalog, scifi_crate};
use shape_gamekit::export::{
    FrozenMeshArtifact, ManualReviewMarker, ManualReviewStatus, MaterialSlotAssignment,
    STATIC_PROP_GAME_READY_PACKAGE_SCHEMA_VERSION, StaticPropCollision, StaticPropFeatureStatus,
    StaticPropGameReadyPackage, StaticPropHandoff, StaticPropLodLevel, StaticPropLodPolicy,
    StaticPropVisualEvidence, UvPolicy, validate_static_prop_game_ready_package_with_root,
};
use shape_gamekit::gltf::{
    StaticPropGlbMetadata, write_static_prop_glb, write_static_prop_surface_glb,
};
use shape_gamekit::surface::{
    MaterialRecipe, MaterialSlotBinding, MaterialStylePack, SURFACE_ARTIFACT_SCHEMA_VERSION,
    SURFACE_CAPABILITIES_SCHEMA_VERSION, SURFACE_LAB_PACKAGE_SCHEMA_VERSION, SurfaceArtifact,
    SurfaceArtifactEvidence, SurfaceCapabilities, SurfaceEvidence, SurfaceLabPackage,
    SurfaceMaterialSlot, SurfaceMaterialVariantCandidate, SurfaceMaterialVariantCandidateSet,
    SurfaceReviewStatus, SurfaceTextureFile, SurfaceTextureSet, SurfaceTriangleBinding,
    SurfaceUvLayoutPolicy, SurfaceUvSet, SurfaceVariationChannels, SurfaceVisualDeltaEvidence,
    TextureChannel, TextureRequirement, TextureRequirementSet, UnsupportedSurfaceOutputReport,
    surface_visual_delta_report, validate_surface_artifact_with_root,
    validate_surface_lab_package_with_root, validate_surface_material_variant_candidate_set,
};
use shape_gamekit::{
    CellBounds, CollisionProxy, ConstructionPhase, ConstructionProfile, ExportProfile,
    FixedCameraProfile, GAME_ASSET_PACK_SCHEMA_VERSION, GameAssetDefinition, GameAssetPack,
    GameplayTag, GridRotation, LayerBounds, LogicalFootprint, ModuleSemantics,
    MonotonicVisibilityPolicy, ReadabilityProfile, RotationSymmetry, SnapAnchor, SnapAnchorRole,
    SnapRelationship, SupportRole, SupportSurface, SurfaceShape, TriangleBudget,
    validate_game_asset_pack,
};
use shape_mesh::{TriangleMesh, write_obj_to_path};
use shape_render::surface_preview::{
    SurfacePreviewMaterialBinding, SurfacePreviewOutput, SurfacePreviewRequest,
    SurfacePreviewTexture, SurfacePreviewTextureChannel, SurfacePreviewTriangleBinding,
    TextureSampling, default_surface_preview_request, render_material_slot_overlay,
    render_surface_preview, surface_preview_contact_sheet,
};
use shape_render::{RenderSettings, RenderedImage, fit_camera_to_bounds_from_angles, render_mesh};

use crate::{render_mesh_from_triangles, save_contact_sheet, save_png, write_json};

const SCI_FI_CRATE_PROFILE: &str = "sci-fi-crate";
const STATIC_PROP_PACKAGE_FILE: &str = "static-prop-package.json";
const VALIDATION_REPORT_FILE: &str = "validation-report.json";
const GAME_ASSET_PACK_FILE: &str = "game-asset-pack.json";
const MODEL_PACKAGE_DIR: &str = "model-package";
const PACKAGE_VERIFICATION_FILE: &str = "package-verification.json";
const FROZEN_OBJ_FILE: &str = "frozen.obj";
const GLB_FILE: &str = "sci-fi-crate-static-prop.glb";
const GLB_VALIDATION_REPORT_FILE: &str = "glb-validation-report.json";
const SURFACE_GLB_FILE: &str = "sci-fi-crate-surface-static-prop.glb";
const SURFACE_GLB_VALIDATION_REPORT_FILE: &str = "surface-glb-validation-report.json";
const GROUPED_OBJ_REPORT_FILE: &str = "grouped-obj-report.json";
const SOURCE_DOCUMENT_FILE: &str = "source-document.json";
const SOURCE_RECIPE_FILE: &str = "source-recipe.json";
const SURFACE_LAB_PACKAGE_FILE: &str = "surface-lab-package.json";
const SURFACE_LAB_VALIDATION_REPORT_FILE: &str = "surface-lab-validation-report.json";
const SURFACE_ARTIFACT_FILE: &str = "surface/surface-artifact.json";
const SURFACE_VALIDATION_REPORT_FILE: &str = "surface/surface-validation-report.json";
const SURFACE_CAPABILITIES_FILE: &str = "surface/surface-capabilities.json";
const MATERIAL_PACK_FILE: &str = "surface/material-pack.json";
const TEXTURE_REQUIREMENTS_FILE: &str = "surface/texture-requirements.json";
const UNSUPPORTED_TEXTURE_REPORT_FILE: &str = "surface/unsupported-texture-report.json";
const SURFACE_UV_LAYOUT_FILE: &str = "surface/uv-layout.png";
const SURFACE_SWATCH_SHEET_FILE: &str = "surface/material-swatch-sheet.png";
const SURFACE_TEXTURE_CONTACT_SHEET_FILE: &str = "surface/texture-contact-sheet.png";
const SURFACE_TEXTURED_PREVIEW_FILE: &str = "surface/textured-preview.png";
const SURFACE_TEXTURED_CONTACT_SHEET_FILE: &str = "surface/textured-contact-sheet.png";
const SURFACE_MATERIAL_SLOT_OVERLAY_FILE: &str = "surface/material-slot-overlay.png";
const SURFACE_PREVIEW_REPORT_FILE: &str = "surface/surface-preview-report.json";
const SURFACE_VARIANTS_DIR: &str = "surface/variants";
const SURFACE_VARIANTS_CANDIDATES_FILE: &str = "surface/variants/candidates.json";
const SURFACE_VARIANTS_CONTACT_SHEET_FILE: &str = "surface/variants/contact-sheet.png";
const SURFACE_TRIANGLE_COVERAGE_FILE: &str = "surface/triangle-slot-coverage.json";
const SURFACE_TEXTURE_DIR: &str = "surface/textures";
const SURFACE_TEXTURE_SIZE: u32 = 256;
const LOD1_PROXY_FILE: &str = "lods/lod1-proxy.obj";
const LOD2_COLLISION_FILE: &str = "lods/lod2-collision.obj";
const FRONT_PREVIEW_FILE: &str = "visual-evidence/front.png";
const THREE_QUARTER_PREVIEW_FILE: &str = "visual-evidence/three-quarter.png";
const SIDE_PREVIEW_FILE: &str = "visual-evidence/side.png";
const WIREFRAME_PREVIEW_FILE: &str = "visual-evidence/wireframe.png";
const CONTACT_SHEET_FILE: &str = "visual-evidence/contact-sheet.png";

/// Emit a deterministic static-prop game-readiness package.
#[derive(Debug, clap::Args)]
pub struct GameReadyStaticPropArgs {
    /// Static prop profile to package. Only sci-fi-crate is implemented in this milestone.
    #[arg(long, value_enum, default_value = "sci-fi-crate")]
    profile: GameReadyStaticPropProfile,
    /// Output package directory.
    #[arg(long)]
    out_dir: PathBuf,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, ValueEnum)]
enum GameReadyStaticPropProfile {
    #[value(name = "sci-fi-crate", alias = "scifi-crate")]
    SciFiCrate,
}

impl GameReadyStaticPropProfile {
    fn slug(self) -> &'static str {
        match self {
            Self::SciFiCrate => SCI_FI_CRATE_PROFILE,
        }
    }

    fn fixture(self) -> FoundryFixtureCatalog {
        match self {
            Self::SciFiCrate => scifi_crate::fixture_catalog(),
        }
    }
}

/// Summary printed by the CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameReadyStaticPropSummary {
    /// Profile slug.
    pub profile_id: String,
    /// Portable package manifest.
    pub package_manifest: String,
    /// Readiness validation report.
    pub validation_report: String,
    /// Runtime-neutral game asset pack.
    pub game_asset_pack: String,
    /// Whether the full game-ready claim passed.
    pub game_ready: bool,
    /// Blocking issue codes.
    pub blocker_codes: Vec<String>,
}

pub fn run_game_ready_static_prop(args: GameReadyStaticPropArgs) -> anyhow::Result<()> {
    let summary = generate_static_prop_package(args.profile, &args.out_dir)?;
    println!("{}", serde_json::to_string_pretty(&summary)?);
    Ok(())
}

fn generate_static_prop_package(
    profile: GameReadyStaticPropProfile,
    out_dir: &Path,
) -> anyhow::Result<GameReadyStaticPropSummary> {
    if profile.slug() != SCI_FI_CRATE_PROFILE {
        bail!("only the sci-fi-crate static prop profile is implemented");
    }
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    fs::create_dir_all(out_dir.join("lods"))
        .with_context(|| format!("creating {}", out_dir.join("lods").display()))?;
    fs::create_dir_all(out_dir.join("visual-evidence"))
        .with_context(|| format!("creating {}", out_dir.join("visual-evidence").display()))?;
    fs::create_dir_all(out_dir.join("surface"))
        .with_context(|| format!("creating {}", out_dir.join("surface").display()))?;
    fs::create_dir_all(out_dir.join(SURFACE_TEXTURE_DIR))
        .with_context(|| format!("creating {}", out_dir.join(SURFACE_TEXTURE_DIR).display()))?;
    fs::create_dir_all(out_dir.join(SURFACE_VARIANTS_DIR))
        .with_context(|| format!("creating {}", out_dir.join(SURFACE_VARIANTS_DIR).display()))?;

    let fixture = profile.fixture();
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("static prop profile failed to compile: {error:#?}"))?;

    write_json(out_dir.join(SOURCE_DOCUMENT_FILE), &output.document)?;
    write_json(out_dir.join(SOURCE_RECIPE_FILE), &output.recipe)?;

    let model_report = validate_model_for_output(&output);
    let model_package_dir = out_dir.join(MODEL_PACKAGE_DIR);
    let package_paths = write_model_package(&output.recipe, &output.artifact, &model_package_dir)
        .with_context(|| format!("writing {}", model_package_dir.display()))?;
    let package_verification = verify_model_package(&model_package_dir)
        .with_context(|| format!("verifying {}", model_package_dir.display()))?;
    write_json(
        out_dir.join(PACKAGE_VERIFICATION_FILE),
        &package_verification,
    )?;

    let grouped_obj = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
        .context("writing grouped OBJ export")?;
    fs::write(out_dir.join(FROZEN_OBJ_FILE), grouped_obj.obj)
        .with_context(|| format!("writing {}", out_dir.join(FROZEN_OBJ_FILE).display()))?;
    write_json(out_dir.join(GROUPED_OBJ_REPORT_FILE), &grouped_obj.report)?;

    let mesh = render_mesh_from_triangles(&output.artifact.combined_preview);
    let collision_proxy = collision_proxy_for_bounds(mesh.bounds);
    write_obj_to_path(
        &box_mesh_from_bounds(mesh.bounds.expanded(0.015)),
        out_dir.join(LOD1_PROXY_FILE),
    )
    .with_context(|| format!("writing {}", out_dir.join(LOD1_PROXY_FILE).display()))?;
    write_obj_to_path(
        &box_mesh_from_collision(&collision_proxy),
        out_dir.join(LOD2_COLLISION_FILE),
    )
    .with_context(|| format!("writing {}", out_dir.join(LOD2_COLLISION_FILE).display()))?;

    let material_slots = static_crate_material_slot_assignments();
    let glb_report = write_static_prop_glb(
        &mesh,
        &StaticPropGlbMetadata {
            profile_id: profile.slug().to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            material_slots: material_slots
                .iter()
                .map(|slot| slot.slot_id.clone())
                .collect(),
        },
        out_dir.join(GLB_FILE),
    )
    .context("writing static prop GLB handoff")?;
    write_json(out_dir.join(GLB_VALIDATION_REPORT_FILE), &glb_report)?;

    let front = render_static_view(&mesh, 0.0, 12.0, false)?;
    let three_quarter = render_static_view(&mesh, 42.0, 18.0, false)?;
    let side = render_static_view(&mesh, 90.0, 12.0, false)?;
    let wireframe = render_static_view(&mesh, 42.0, 18.0, true)?;
    save_png(&front, out_dir.join(FRONT_PREVIEW_FILE))?;
    save_png(&three_quarter, out_dir.join(THREE_QUARTER_PREVIEW_FILE))?;
    save_png(&side, out_dir.join(SIDE_PREVIEW_FILE))?;
    save_png(&wireframe, out_dir.join(WIREFRAME_PREVIEW_FILE))?;
    save_contact_sheet(
        &front,
        &[&three_quarter, &side, &wireframe],
        out_dir.join(CONTACT_SHEET_FILE),
    )?;

    let material_pack = material_pack_for_static_crate(&material_slots);
    let texture_sets = write_static_crate_texture_payloads(&material_pack, out_dir)?;
    let surface_artifact =
        surface_artifact_for_static_crate(&output, &mesh, &material_pack, texture_sets);
    write_uv_layout(
        &surface_artifact,
        &mesh,
        out_dir.join(SURFACE_UV_LAYOUT_FILE),
    )?;
    write_swatch_sheet(&material_pack, out_dir.join(SURFACE_SWATCH_SHEET_FILE))?;
    write_texture_contact_sheet(
        &surface_artifact.texture_sets,
        out_dir,
        out_dir.join(SURFACE_TEXTURE_CONTACT_SHEET_FILE),
    )?;
    write_json(
        out_dir.join(SURFACE_TRIANGLE_COVERAGE_FILE),
        &triangle_slot_coverage_report(&surface_artifact),
    )?;
    let surface_artifact_report = validate_surface_artifact_with_root(&surface_artifact, out_dir);
    write_json(out_dir.join(SURFACE_ARTIFACT_FILE), &surface_artifact)?;
    write_json(
        out_dir.join(SURFACE_VALIDATION_REPORT_FILE),
        &surface_artifact_report,
    )?;
    let surface_preview = write_surface_preview_evidence(&mesh, &surface_artifact, out_dir)
        .context("writing textured surface preview evidence")?;
    let surface_variant_candidates = write_static_crate_material_variants(
        &mesh,
        &surface_artifact,
        &material_pack,
        out_dir,
        &surface_preview.image,
    )
    .context("writing static crate material-only surface variants")?;
    let surface_variant_report =
        validate_surface_material_variant_candidate_set(&surface_variant_candidates);
    if !surface_variant_report.is_ready() {
        bail!(
            "generated material-only surface variants failed validation: {:#?}",
            surface_variant_report.blockers
        );
    }
    write_json(
        out_dir.join(SURFACE_CAPABILITIES_FILE),
        &surface_capabilities_for_static_crate(&surface_artifact),
    )?;
    let surface_glb_report = write_static_prop_surface_glb(
        &mesh,
        &surface_artifact,
        SURFACE_ARTIFACT_FILE,
        out_dir.join(SURFACE_GLB_FILE),
    )
    .context("writing static prop surface GLB handoff")?;
    write_json(
        out_dir.join(SURFACE_GLB_VALIDATION_REPORT_FILE),
        &surface_glb_report,
    )?;

    let surface_lab_package = surface_lab_package_for_static_crate(&material_pack);
    write_json(out_dir.join(MATERIAL_PACK_FILE), &material_pack)?;
    write_json(
        out_dir.join(TEXTURE_REQUIREMENTS_FILE),
        &surface_lab_package.texture_requirements,
    )?;
    write_json(
        out_dir.join(UNSUPPORTED_TEXTURE_REPORT_FILE),
        &surface_lab_package.unsupported_outputs,
    )?;
    let surface_lab_report = validate_surface_lab_package_with_root(&surface_lab_package, out_dir);
    write_json(out_dir.join(SURFACE_LAB_PACKAGE_FILE), &surface_lab_package)?;
    write_json(
        out_dir.join(SURFACE_LAB_VALIDATION_REPORT_FILE),
        &surface_lab_report,
    )?;

    let game_asset_pack = game_asset_pack_for_static_crate(&output, collision_proxy.clone());
    let game_asset_report = validate_game_asset_pack(&game_asset_pack);
    if !game_asset_report.is_valid() {
        bail!(
            "generated static prop game asset pack failed validation: {:#?}",
            game_asset_report.issues
        );
    }
    write_json(out_dir.join(GAME_ASSET_PACK_FILE), &game_asset_pack)?;

    let package = static_prop_package_for_output(
        &output,
        &model_report,
        &package_paths.blender_reconstruct,
        collision_proxy,
        surface_glb_report.valid,
        surface_artifact_report
            .blockers
            .iter()
            .all(|issue| issue.code == "surface_manual_review_required"),
    );
    let validation_report = validate_static_prop_game_ready_package_with_root(&package, out_dir);
    write_json(out_dir.join(STATIC_PROP_PACKAGE_FILE), &package)?;
    write_json(out_dir.join(VALIDATION_REPORT_FILE), &validation_report)?;

    Ok(GameReadyStaticPropSummary {
        profile_id: profile.slug().to_owned(),
        package_manifest: STATIC_PROP_PACKAGE_FILE.to_owned(),
        validation_report: VALIDATION_REPORT_FILE.to_owned(),
        game_asset_pack: GAME_ASSET_PACK_FILE.to_owned(),
        game_ready: validation_report.is_ready(),
        blocker_codes: validation_report
            .blockers
            .iter()
            .map(|issue| issue.code.clone())
            .collect(),
    })
}

fn validate_model_for_output(
    output: &FoundryCompilationOutput,
) -> shape_compile::validation::ModelValidationReport {
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    validate_model(&output.artifact, &config)
}

fn static_prop_package_for_output(
    output: &FoundryCompilationOutput,
    model_report: &shape_compile::validation::ModelValidationReport,
    blender_script: &Path,
    collision_proxy: CollisionProxy,
    surface_glb_valid: bool,
    uv_ready: bool,
) -> StaticPropGameReadyPackage {
    let triangle_count = output.artifact.statistics.triangle_count.max(1) as u32;
    StaticPropGameReadyPackage {
        schema_version: STATIC_PROP_GAME_READY_PACKAGE_SCHEMA_VERSION,
        profile_id: SCI_FI_CRATE_PROFILE.to_owned(),
        display_name: "Sci-Fi Crate".to_owned(),
        asset_family: "Static Prop Readiness Package".to_owned(),
        source_recipe_hash: output.artifact.source_recipe_hash,
        artifact_fingerprint: output.build_stamp.artifact_fingerprint.0.to_hex(),
        frozen_mesh: FrozenMeshArtifact {
            canonical_model_package: MODEL_PACKAGE_DIR.to_owned(),
            asset_manifest: format!("{MODEL_PACKAGE_DIR}/asset-manifest.json"),
            package_verification: PACKAGE_VERIFICATION_FILE.to_owned(),
            grouped_obj: FROZEN_OBJ_FILE.to_owned(),
            blender_reconstruct_script: rel_model_package_path(blender_script),
            compile_validation_passed: output.artifact.validation_report.is_valid(),
            model_validation_passed: model_report.is_valid(),
        },
        lod_policy: StaticPropLodPolicy {
            policy: "LOD0 exact canonical package; lower LODs are deterministic proxy OBJs derived from bounds/collision.".to_owned(),
            levels: vec![
                StaticPropLodLevel {
                    index: 0,
                    id: "lod0_exact_model_package".to_owned(),
                    source: "exact_canonical_model_package".to_owned(),
                    artifact: MODEL_PACKAGE_DIR.to_owned(),
                    target_triangle_count: triangle_count,
                    exact_source_geometry: true,
                },
                StaticPropLodLevel {
                    index: 1,
                    id: "lod1_bounds_proxy".to_owned(),
                    source: "compiled_bounds_proxy".to_owned(),
                    artifact: LOD1_PROXY_FILE.to_owned(),
                    target_triangle_count: 24,
                    exact_source_geometry: false,
                },
                StaticPropLodLevel {
                    index: 2,
                    id: "lod2_collision_proxy".to_owned(),
                    source: "collision_proxy".to_owned(),
                    artifact: LOD2_COLLISION_FILE.to_owned(),
                    target_triangle_count: 12,
                    exact_source_geometry: false,
                },
            ],
        },
        material_slots: static_crate_material_slot_assignments(),
        uv_policy: UvPolicy {
            status: if uv_ready {
                StaticPropFeatureStatus::Ready
            } else {
                StaticPropFeatureStatus::ManualRequired
            },
            required_for_game_ready: true,
            blocker_code: (!uv_ready).then(|| "surface_uv_validation_failed".to_owned()),
            explanation: if uv_ready {
                "Deterministic Surface Lab v1 UVs are present for the Sci-Fi Crate static prop.".to_owned()
            } else {
                "Surface Lab v1 UV evidence did not pass validation; no texture-ready claim is made.".to_owned()
            },
        },
        surface_artifact: Some(SURFACE_ARTIFACT_FILE.to_owned()),
        collision: StaticPropCollision {
            source: "compiled preview bounds expanded into a simple box proxy".to_owned(),
            proxies: vec![collision_proxy],
        },
        handoff: StaticPropHandoff {
            primary_package_format: "shape-lab-canonical-model-package".to_owned(),
            blender_handoff_script: format!("{MODEL_PACKAGE_DIR}/blender_reconstruct.py"),
            blender_status: StaticPropFeatureStatus::Ready,
            glb_artifact: Some(SURFACE_GLB_FILE.to_owned()),
            glb_status: if surface_glb_valid {
                StaticPropFeatureStatus::Ready
            } else {
                StaticPropFeatureStatus::ManualRequired
            },
            glb_blocker_code: (!surface_glb_valid).then(|| "surface_glb_validation_failed".to_owned()),
            engine_import_proof: None,
            engine_import_status: StaticPropFeatureStatus::ManualRequired,
            engine_native_package_status: StaticPropFeatureStatus::NotImplemented,
        },
        visual_evidence: StaticPropVisualEvidence {
            front: FRONT_PREVIEW_FILE.to_owned(),
            three_quarter: THREE_QUARTER_PREVIEW_FILE.to_owned(),
            side: SIDE_PREVIEW_FILE.to_owned(),
            wireframe: WIREFRAME_PREVIEW_FILE.to_owned(),
            contact_sheet: CONTACT_SHEET_FILE.to_owned(),
        },
        manual_review: ManualReviewMarker {
            status: ManualReviewStatus::Pending,
            reviewer: None,
            notes: "Manual DCC/runtime import review has not been completed.".to_owned(),
        },
    }
}

fn static_crate_material_slot_assignments() -> Vec<MaterialSlotAssignment> {
    vec![
        MaterialSlotAssignment {
            slot_id: "painted_metal_body".to_owned(),
            display_name: "Painted Metal Body".to_owned(),
            semantic_roles: vec!["body".to_owned(), "shell".to_owned(), "side-panel".to_owned()],
            policy: "Primary painted hard-surface body slot with deterministic procedural texture payload.".to_owned(),
            material_payload_ready: true,
        },
        MaterialSlotAssignment {
            slot_id: "dark_recesses_and_vents".to_owned(),
            display_name: "Dark Recesses and Vents".to_owned(),
            semantic_roles: vec![
                "recessed-panel".to_owned(),
                "inset".to_owned(),
                "vent".to_owned(),
                "grille".to_owned(),
            ],
            policy: "Dark inset panels and vent details with deterministic procedural texture payload.".to_owned(),
            material_payload_ready: true,
        },
        MaterialSlotAssignment {
            slot_id: "exposed_edge_trim".to_owned(),
            display_name: "Exposed Edge Trim".to_owned(),
            semantic_roles: vec!["edge-trim".to_owned(), "bevel".to_owned(), "rail".to_owned()],
            policy: "Trim, bevel highlights, and exposed edges with deterministic procedural texture payload.".to_owned(),
            material_payload_ready: true,
        },
        MaterialSlotAssignment {
            slot_id: "handle_grip".to_owned(),
            display_name: "Handle Grip".to_owned(),
            semantic_roles: vec!["handle".to_owned(), "side-handle".to_owned(), "grip".to_owned()],
            policy: "Handle hardware and grips with deterministic procedural texture payload.".to_owned(),
            material_payload_ready: true,
        },
        MaterialSlotAssignment {
            slot_id: "fasteners_and_mounts".to_owned(),
            display_name: "Fasteners and Mounts".to_owned(),
            semantic_roles: vec!["fastener".to_owned(), "bolt".to_owned(), "rivet".to_owned()],
            policy: "Fasteners and small mounts with deterministic procedural texture payload.".to_owned(),
            material_payload_ready: true,
        },
        MaterialSlotAssignment {
            slot_id: "fallback_hard_surface".to_owned(),
            display_name: "Fallback Hard Surface".to_owned(),
            semantic_roles: vec!["fallback".to_owned(), "hard-surface".to_owned()],
            policy: "Safe fallback slot for triangles with sparse provenance.".to_owned(),
            material_payload_ready: true,
        },
    ]
}

fn material_pack_for_static_crate(slots: &[MaterialSlotAssignment]) -> MaterialStylePack {
    MaterialStylePack {
        id: "sci-fi-crate-industrial-surface-recipes-v1".to_owned(),
        display_name: "Industrial Sci-Fi Surface Recipes".to_owned(),
        texture_packing_policy: "Generated v1 texture set: base color, flat normal, metallic-roughness, and neutral occlusion PNG sidecars.".to_owned(),
        recipes: vec![
            MaterialRecipe {
                material_id: "worn-painted-sci-fi-metal".to_owned(),
                display_name: "Worn Painted Sci-Fi Metal".to_owned(),
                base_color_srgb: [104, 112, 102],
                metallic: 0.0,
                roughness: 0.76,
                wear_policy: "Deterministic subtle edge and panel variation only; no authored wear mask.".to_owned(),
                texture_payload_ready: true,
            },
            MaterialRecipe {
                material_id: "dark-rubber-industrial-detail".to_owned(),
                display_name: "Dark Rubber / Industrial Detail".to_owned(),
                base_color_srgb: [28, 31, 32],
                metallic: 0.0,
                roughness: 0.84,
                wear_policy: "Deterministic low-contrast surface noise for recessed details.".to_owned(),
                texture_payload_ready: true,
            },
            MaterialRecipe {
                material_id: "exposed-edge-metal".to_owned(),
                display_name: "Exposed Edge Metal".to_owned(),
                base_color_srgb: [154, 150, 138],
                metallic: 0.75,
                roughness: 0.46,
                wear_policy: "Deterministic brushed variation; no final artist-authored edge mask.".to_owned(),
                texture_payload_ready: true,
            },
            MaterialRecipe {
                material_id: "industrial-warning-trim".to_owned(),
                display_name: "Industrial Warning Trim".to_owned(),
                base_color_srgb: [198, 148, 44],
                metallic: 0.0,
                roughness: 0.7,
                wear_policy: "Deterministic warning-trim color only; no authored stripe mask.".to_owned(),
                texture_payload_ready: true,
            },
            MaterialRecipe {
                material_id: "neutral-hard-surface-fallback".to_owned(),
                display_name: "Neutral Hard Surface Fallback".to_owned(),
                base_color_srgb: [92, 96, 92],
                metallic: 0.0,
                roughness: 0.72,
                wear_policy: "Fallback material for sparse provenance triangles.".to_owned(),
                texture_payload_ready: true,
            },
        ],
        slot_bindings: surface_material_slot_bindings_for_static_crate(slots),
    }
}

fn surface_lab_package_for_static_crate(material_pack: &MaterialStylePack) -> SurfaceLabPackage {
    SurfaceLabPackage {
        schema_version: SURFACE_LAB_PACKAGE_SCHEMA_VERSION,
        profile_id: SCI_FI_CRATE_PROFILE.to_owned(),
        display_name: "Sci-Fi Crate".to_owned(),
        asset_family: "Static Prop Surface Lab".to_owned(),
        uv_layout: SurfaceUvLayoutPolicy {
            status: StaticPropFeatureStatus::Ready,
            required_for_texture_ready: true,
            policy: "Deterministic normal-aware atlas projection for the Sci-Fi Crate static prop; coordinates are normalized into 0..1.".to_owned(),
            blocker_code: None,
            explanation: "Surface Lab v1 emits deterministic UV coordinates for every exported vertex and one UV binding per triangle.".to_owned(),
        },
        material_pack: material_pack.clone(),
        texture_requirements: TextureRequirementSet {
            id: "sci-fi-crate-texture-requirements-v1".to_owned(),
            channels: vec![
                TextureRequirement {
                    channel: TextureChannel::BaseColor,
                    resolution_px: SURFACE_TEXTURE_SIZE,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::Ready,
                    blocker_code: None,
                    explanation: "A deterministic base-color PNG is emitted for every v1 material recipe.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::Normal,
                    resolution_px: SURFACE_TEXTURE_SIZE,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::Ready,
                    blocker_code: None,
                    explanation: "A deterministic flat normal PNG is emitted for every v1 material recipe.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::MetallicRoughness,
                    resolution_px: SURFACE_TEXTURE_SIZE,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::Ready,
                    blocker_code: None,
                    explanation: "A deterministic packed metallic/roughness PNG is emitted for every v1 material recipe.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::Occlusion,
                    resolution_px: SURFACE_TEXTURE_SIZE,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::Ready,
                    blocker_code: None,
                    explanation: "A deterministic neutral occlusion PNG is emitted for every v1 material recipe.".to_owned(),
                },
            ],
        },
        unsupported_outputs: UnsupportedSurfaceOutputReport { outputs: Vec::new() },
        evidence: SurfaceEvidence {
            swatch_sheet: SURFACE_SWATCH_SHEET_FILE.to_owned(),
            validation_report: SURFACE_LAB_VALIDATION_REPORT_FILE.to_owned(),
        },
    }
}

fn surface_material_slot_bindings_for_static_crate(
    slots: &[MaterialSlotAssignment],
) -> Vec<MaterialSlotBinding> {
    slots
        .iter()
        .map(|slot| MaterialSlotBinding {
            slot_id: slot.slot_id.clone(),
            display_name: slot.display_name.clone(),
            material_id: material_id_for_static_crate_slot(&slot.slot_id).to_owned(),
            semantic_roles: slot.semantic_roles.clone(),
        })
        .collect()
}

fn material_id_for_static_crate_slot(slot_id: &str) -> &'static str {
    match slot_id {
        "painted_metal_body" => "worn-painted-sci-fi-metal",
        "dark_recesses_and_vents" => "dark-rubber-industrial-detail",
        "handle_grip" | "fasteners_and_mounts" => "exposed-edge-metal",
        "exposed_edge_trim" => "industrial-warning-trim",
        _ => "neutral-hard-surface-fallback",
    }
}

fn write_swatch_sheet(pack: &MaterialStylePack, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let count = u32::try_from(pack.recipes.len().max(1)).context("too many material recipes")?;
    let swatch_width = 96;
    let swatch_height = 96;
    let padding = 16;
    let width = padding + count * (swatch_width + padding);
    let height = swatch_height + padding * 2;
    let mut image = RgbaImage::from_pixel(width, height, Rgba([18, 20, 22, 255]));
    for (index, recipe) in pack.recipes.iter().enumerate() {
        let x = padding
            + u32::try_from(index).context("material recipe index overflow")?
                * (swatch_width + padding);
        let color = Rgba([
            recipe.base_color_srgb[0],
            recipe.base_color_srgb[1],
            recipe.base_color_srgb[2],
            255,
        ]);
        fill_swatch_rect(&mut image, x, padding, swatch_width, swatch_height, color);
        fill_swatch_rect(
            &mut image,
            x,
            padding + swatch_height.saturating_sub(8),
            swatch_width,
            8,
            Rgba([220, 225, 218, 255]),
        );
    }
    image
        .save(path)
        .with_context(|| format!("saving surface swatch sheet to {}", path.display()))?;
    Ok(())
}

fn fill_swatch_rect(
    image: &mut RgbaImage,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: Rgba<u8>,
) {
    for py in y..y.saturating_add(height).min(image.height()) {
        for px in x..x.saturating_add(width).min(image.width()) {
            image.put_pixel(px, py, color);
        }
    }
}

fn write_static_crate_texture_payloads(
    pack: &MaterialStylePack,
    out_dir: &Path,
) -> anyhow::Result<Vec<SurfaceTextureSet>> {
    write_static_crate_texture_payloads_in_dir(pack, out_dir, SURFACE_TEXTURE_DIR)
}

fn write_static_crate_texture_payloads_in_dir(
    pack: &MaterialStylePack,
    out_dir: &Path,
    texture_dir: &str,
) -> anyhow::Result<Vec<SurfaceTextureSet>> {
    let mut texture_sets = Vec::with_capacity(pack.recipes.len());
    for (recipe_index, recipe) in pack.recipes.iter().enumerate() {
        let mut files = Vec::new();
        for channel in [
            TextureChannel::BaseColor,
            TextureChannel::MetallicRoughness,
            TextureChannel::Normal,
            TextureChannel::Occlusion,
        ] {
            let relative = format!(
                "{texture_dir}/{}-{}.png",
                recipe.material_id,
                texture_channel_slug(channel)
            );
            write_texture_png(
                recipe,
                recipe_index as u32,
                channel,
                out_dir.join(&relative),
            )?;
            files.push(SurfaceTextureFile {
                channel,
                path: relative,
                width: SURFACE_TEXTURE_SIZE,
                height: SURFACE_TEXTURE_SIZE,
                color_space: match channel {
                    TextureChannel::BaseColor => "sRGB".to_owned(),
                    TextureChannel::Normal
                    | TextureChannel::MetallicRoughness
                    | TextureChannel::Occlusion
                    | TextureChannel::Emissive => "linear".to_owned(),
                },
                required_for_texture_ready: true,
            });
        }
        texture_sets.push(SurfaceTextureSet {
            id: format!("{}-texture-set-v1", recipe.material_id),
            display_name: format!("{} Texture Set", recipe.display_name),
            material_recipe_id: recipe.material_id.clone(),
            files,
            procedural_source: "Shape Lab deterministic Surface Lab v1 hard-surface texture generator; no image-generation model.".to_owned(),
            payload_ready: true,
        });
    }
    Ok(texture_sets)
}

fn write_texture_png(
    recipe: &MaterialRecipe,
    recipe_index: u32,
    channel: TextureChannel,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating texture directory {}", parent.display()))?;
    }
    let mut image = RgbaImage::from_pixel(
        SURFACE_TEXTURE_SIZE,
        SURFACE_TEXTURE_SIZE,
        Rgba([0, 0, 0, 255]),
    );
    for y in 0..SURFACE_TEXTURE_SIZE {
        for x in 0..SURFACE_TEXTURE_SIZE {
            let noise = (((x * 13 + y * 7 + recipe_index * 31) % 19) as i16) - 9;
            let pixel = match channel {
                TextureChannel::BaseColor => {
                    let color = recipe
                        .base_color_srgb
                        .map(|value| (i16::from(value) + noise).clamp(0, 255) as u8);
                    Rgba([color[0], color[1], color[2], 255])
                }
                TextureChannel::MetallicRoughness => Rgba([
                    (recipe.metallic.clamp(0.0, 1.0) * 255.0).round() as u8,
                    (recipe.roughness.clamp(0.0, 1.0) * 255.0).round() as u8,
                    255,
                    255,
                ]),
                TextureChannel::Normal => Rgba([128, 128, 255, 255]),
                TextureChannel::Occlusion => Rgba([235, 235, 235, 255]),
                TextureChannel::Emissive => Rgba([0, 0, 0, 255]),
            };
            image.put_pixel(x, y, pixel);
        }
    }
    image
        .save(path.as_ref())
        .with_context(|| format!("saving texture PNG to {}", path.as_ref().display()))?;
    Ok(())
}

fn surface_artifact_for_static_crate(
    output: &FoundryCompilationOutput,
    mesh: &TriangleMesh,
    material_pack: &MaterialStylePack,
    texture_sets: Vec<SurfaceTextureSet>,
) -> SurfaceArtifact {
    let uv_coordinates = generate_static_crate_uvs(mesh);
    let triangle_count = mesh.indices.len() / 3;
    let mut coverage = material_pack
        .slot_bindings
        .iter()
        .map(|binding| (binding.slot_id.clone(), 0_u32))
        .collect::<BTreeMap<_, _>>();
    let mut triangle_bindings = Vec::with_capacity(triangle_count);
    for triangle_index in 0..triangle_count {
        let slot_id = material_slot_for_triangle(output, mesh, triangle_index).to_owned();
        *coverage.entry(slot_id.clone()).or_default() += 1;
        triangle_bindings.push(SurfaceTriangleBinding {
            triangle_index: triangle_index as u32,
            material_slot_id: slot_id,
            uv_set_id: "uv0_static_crate_box_projection".to_owned(),
            source_part: output
                .artifact
                .combined_preview
                .triangle_to_part
                .get(triangle_index)
                .and_then(|part| part.map(|id| format!("compiled_part_{:03}", id.0))),
            source_region: output
                .artifact
                .combined_preview
                .triangle_to_region
                .get(triangle_index)
                .and_then(|region| region.map(|id| format!("compiled_region_{:03}", id.0))),
            source_operation: output
                .artifact
                .combined_preview
                .triangle_to_operation
                .get(triangle_index)
                .and_then(|operation| {
                    operation.map(|id| format!("compiled_operation_{:03}", id.0))
                }),
        });
    }
    let material_slots = material_pack
        .slot_bindings
        .iter()
        .map(|binding| {
            let count = coverage.get(&binding.slot_id).copied().unwrap_or_default();
            SurfaceMaterialSlot {
                slot_id: binding.slot_id.clone(),
                display_name: binding.display_name.clone(),
                semantic_roles: binding.semantic_roles.clone(),
                recipe_id: binding.material_id.clone(),
                coverage_triangle_count: count,
                coverage_fraction: if triangle_count == 0 {
                    0.0
                } else {
                    count as f32 / triangle_count as f32
                },
            }
        })
        .collect();
    SurfaceArtifact {
        schema_version: SURFACE_ARTIFACT_SCHEMA_VERSION,
        profile_id: SCI_FI_CRATE_PROFILE.to_owned(),
        display_name: "Sci-Fi Crate".to_owned(),
        source_artifact_fingerprint: output.build_stamp.artifact_fingerprint.0.to_hex(),
        source_recipe_hash: output.artifact.source_recipe_hash,
        frozen_mesh_ref: MODEL_PACKAGE_DIR.to_owned(),
        uv_sets: vec![SurfaceUvSet {
            id: "uv0_static_crate_box_projection".to_owned(),
            display_name: "UV0 Static Crate Box Projection".to_owned(),
            channel_index: 0,
            coordinate_count: uv_coordinates.len() as u32,
            coordinates: uv_coordinates,
            source_policy:
                "Normal-aware deterministic box projection into a normalized 0..1 atlas.".to_owned(),
            readiness_status: StaticPropFeatureStatus::Ready,
            tiling_allowed: false,
        }],
        material_slots,
        texture_sets,
        triangle_bindings,
        evidence: SurfaceArtifactEvidence {
            uv_layout: SURFACE_UV_LAYOUT_FILE.to_owned(),
            material_swatch_sheet: SURFACE_SWATCH_SHEET_FILE.to_owned(),
            texture_contact_sheet: SURFACE_TEXTURE_CONTACT_SHEET_FILE.to_owned(),
            triangle_slot_coverage: SURFACE_TRIANGLE_COVERAGE_FILE.to_owned(),
        },
        validation_report_ref: SURFACE_VALIDATION_REPORT_FILE.to_owned(),
        manual_review: SurfaceReviewStatus::AutomatedReady,
    }
}

fn generate_static_crate_uvs(mesh: &TriangleMesh) -> Vec<[f32; 2]> {
    let min = mesh.bounds.min;
    let extent = mesh.bounds.extent().max(Vec3::splat(1.0e-5));
    mesh.positions
        .iter()
        .zip(&mesh.normals)
        .map(|(position, normal)| {
            let p = Vec3::from_array(*position);
            let n = Vec3::from_array(*normal).abs();
            let coordinate = if n.x >= n.y && n.x >= n.z {
                [(p.z - min.z) / extent.z, (p.y - min.y) / extent.y]
            } else if n.y >= n.z {
                [(p.x - min.x) / extent.x, (p.z - min.z) / extent.z]
            } else {
                [(p.x - min.x) / extent.x, (p.y - min.y) / extent.y]
            };
            [coordinate[0].clamp(0.0, 1.0), coordinate[1].clamp(0.0, 1.0)]
        })
        .collect()
}

fn material_slot_for_triangle(
    output: &FoundryCompilationOutput,
    mesh: &TriangleMesh,
    triangle_index: usize,
) -> &'static str {
    if let Some(part_name) = part_name_for_triangle(output, triangle_index) {
        let name = part_name.to_ascii_lowercase();
        if name.contains("fastener") || name.contains("bolt") || name.contains("rivet") {
            return "fasteners_and_mounts";
        }
        if name.contains("handle") || name.contains("grip") {
            return "handle_grip";
        }
        if name.contains("trim") || name.contains("rail") {
            return "exposed_edge_trim";
        }
        if name.contains("panel") || name.contains("vent") || name.contains("recess") {
            return "dark_recesses_and_vents";
        }
    }
    if let Some(operation) = output
        .artifact
        .combined_preview
        .triangle_to_operation
        .get(triangle_index)
        .and_then(|operation| *operation)
    {
        match operation.0 {
            1..=5 => return "dark_recesses_and_vents",
            6..=7 => return "fasteners_and_mounts",
            8..=u64::MAX => return "exposed_edge_trim",
            _ => {}
        }
    }
    if let Some(region) = output
        .artifact
        .combined_preview
        .triangle_to_region
        .get(triangle_index)
        .and_then(|region| *region)
    {
        match region.0 {
            10..=79 => return "dark_recesses_and_vents",
            80..=u64::MAX => return "exposed_edge_trim",
            _ => {}
        }
    }
    if triangle_centroid_is_near_bounds_edge(mesh, triangle_index) {
        "exposed_edge_trim"
    } else if part_name_for_triangle(output, triangle_index).is_some() {
        "painted_metal_body"
    } else {
        "fallback_hard_surface"
    }
}

fn part_name_for_triangle(
    output: &FoundryCompilationOutput,
    triangle_index: usize,
) -> Option<&str> {
    let part = output
        .artifact
        .combined_preview
        .triangle_to_part
        .get(triangle_index)
        .and_then(|part| *part)?;
    output
        .artifact
        .compiled_parts
        .iter()
        .find(|compiled| compiled.instance_id == part)
        .map(|compiled| compiled.instance_name.as_str())
}

fn triangle_centroid_is_near_bounds_edge(mesh: &TriangleMesh, triangle_index: usize) -> bool {
    let indices = mesh.indices.chunks_exact(3).nth(triangle_index);
    let Some(indices) = indices else {
        return false;
    };
    let centroid = indices
        .iter()
        .filter_map(|index| mesh.positions.get(*index as usize))
        .map(|position| Vec3::from_array(*position))
        .fold(Vec3::ZERO, |acc, point| acc + point)
        / 3.0;
    let min = mesh.bounds.min;
    let max = mesh.bounds.max;
    let extent = mesh.bounds.extent();
    let threshold = extent.max_element() * 0.045;
    [
        centroid.x - min.x,
        max.x - centroid.x,
        centroid.y - min.y,
        max.y - centroid.y,
        centroid.z - min.z,
        max.z - centroid.z,
    ]
    .into_iter()
    .any(|distance| distance <= threshold)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TriangleSlotCoverageReport {
    profile_id: String,
    triangle_count: u32,
    slots: Vec<TriangleSlotCoverageRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct TriangleSlotCoverageRow {
    slot_id: String,
    display_name: String,
    triangle_count: u32,
}

fn triangle_slot_coverage_report(artifact: &SurfaceArtifact) -> TriangleSlotCoverageReport {
    TriangleSlotCoverageReport {
        profile_id: artifact.profile_id.clone(),
        triangle_count: artifact.triangle_bindings.len() as u32,
        slots: artifact
            .material_slots
            .iter()
            .map(|slot| TriangleSlotCoverageRow {
                slot_id: slot.slot_id.clone(),
                display_name: slot.display_name.clone(),
                triangle_count: slot.coverage_triangle_count,
            })
            .collect(),
    }
}

fn surface_capabilities_for_static_crate(artifact: &SurfaceArtifact) -> SurfaceCapabilities {
    SurfaceCapabilities {
        schema_version: SURFACE_CAPABILITIES_SCHEMA_VERSION,
        profile_id: artifact.profile_id.clone(),
        surface_payload_ready: true,
        uv_ready: true,
        material_slots: artifact
            .material_slots
            .iter()
            .map(|slot| slot.slot_id.clone())
            .collect(),
        texture_channels: vec![
            TextureChannel::BaseColor,
            TextureChannel::MetallicRoughness,
            TextureChannel::Normal,
            TextureChannel::Occlusion,
        ],
        variation_channels_supported: SurfaceVariationChannels {
            surface: true,
            wear: false,
        },
        focus_part_surface_ready: false,
        human_label: "Sci-Fi Crate Surface Payload".to_owned(),
        unavailable_reasons: vec![
            "Part-specific surface editing is not implemented.".to_owned(),
            "Wear variation is metadata-only in Surface Lab v1.".to_owned(),
            "Manual DCC/runtime review is still required before a full game-ready claim."
                .to_owned(),
        ],
    }
}

fn write_uv_layout(
    artifact: &SurfaceArtifact,
    mesh: &TriangleMesh,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let size = 512;
    let margin = 24_i32;
    let mut image = RgbaImage::from_pixel(size, size, Rgba([13, 17, 21, 255]));
    let Some(uv_set) = artifact.uv_sets.first() else {
        bail!("surface artifact has no UV set");
    };
    for binding in &artifact.triangle_bindings {
        let color = slot_color(&binding.material_slot_id);
        let Some(indices) = mesh
            .indices
            .chunks_exact(3)
            .nth(binding.triangle_index as usize)
        else {
            continue;
        };
        let points = [indices[0], indices[1], indices[2]].map(|index| {
            let uv = uv_set.coordinates[index as usize];
            uv_to_pixel(uv, size, margin)
        });
        draw_line(&mut image, points[0], points[1], color);
        draw_line(&mut image, points[1], points[2], color);
        draw_line(&mut image, points[2], points[0], color);
    }
    image
        .save(path.as_ref())
        .with_context(|| format!("saving UV layout to {}", path.as_ref().display()))?;
    Ok(())
}

fn write_texture_contact_sheet(
    texture_sets: &[SurfaceTextureSet],
    package_root: &Path,
    path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let cell = 72_u32;
    let padding = 12_u32;
    let columns = 4_u32;
    let rows = u32::try_from(texture_sets.len().max(1)).context("too many texture sets")?;
    let width = padding + columns * (cell + padding);
    let height = padding + rows * (cell + padding);
    let mut sheet = RgbaImage::from_pixel(width, height, Rgba([16, 19, 23, 255]));
    for (row, set) in texture_sets.iter().enumerate() {
        for (column, file) in set.files.iter().enumerate() {
            let source = image::open(package_root.join(&file.path))
                .with_context(|| format!("opening texture {}", file.path))?
                .to_rgba8();
            let resized = image::imageops::resize(&source, cell, cell, FilterType::Nearest);
            image::imageops::replace(
                &mut sheet,
                &resized,
                i64::from(padding + column as u32 * (cell + padding)),
                i64::from(padding + row as u32 * (cell + padding)),
            );
        }
    }
    sheet.save(path.as_ref()).with_context(|| {
        format!(
            "saving texture contact sheet to {}",
            path.as_ref().display()
        )
    })?;
    Ok(())
}

fn write_surface_preview_evidence(
    mesh: &TriangleMesh,
    artifact: &SurfaceArtifact,
    out_dir: &Path,
) -> anyhow::Result<SurfacePreviewOutput> {
    let request = surface_preview_request_for_artifact(mesh, artifact, out_dir, 512)?;
    let preview = render_surface_preview(&request).map_err(|error| anyhow::anyhow!("{error}"))?;
    save_png(&preview.image, out_dir.join(SURFACE_TEXTURED_PREVIEW_FILE))?;

    let mut sheet_images = vec![preview.image.clone()];
    for (yaw, pitch) in [(0.0, 12.0), (90.0, 12.0), (180.0, 18.0)] {
        let mut view_request = request.clone();
        view_request.camera = fit_camera_to_bounds_from_angles(mesh.bounds, yaw, pitch, 1.0);
        let rendered =
            render_surface_preview(&view_request).map_err(|error| anyhow::anyhow!("{error}"))?;
        sheet_images.push(rendered.image);
    }
    let contact_sheet = surface_preview_contact_sheet(&sheet_images);
    save_png(
        &contact_sheet,
        out_dir.join(SURFACE_TEXTURED_CONTACT_SHEET_FILE),
    )?;

    let overlay =
        render_material_slot_overlay(&request).map_err(|error| anyhow::anyhow!("{error}"))?;
    save_png(
        &overlay.image,
        out_dir.join(SURFACE_MATERIAL_SLOT_OVERLAY_FILE),
    )?;
    write_json(out_dir.join(SURFACE_PREVIEW_REPORT_FILE), &preview.report)?;
    Ok(preview)
}

fn surface_preview_request_for_artifact(
    mesh: &TriangleMesh,
    artifact: &SurfaceArtifact,
    package_root: &Path,
    size: u32,
) -> anyhow::Result<SurfacePreviewRequest> {
    let uv_set = artifact
        .uv_sets
        .iter()
        .find(|set| set.channel_index == 0)
        .or_else(|| artifact.uv_sets.first())
        .context("surface artifact has no TEXCOORD_0 set")?;
    let material_bindings = artifact
        .material_slots
        .iter()
        .map(|slot| SurfacePreviewMaterialBinding {
            slot_id: slot.slot_id.clone(),
            display_name: slot.display_name.clone(),
            material_id: slot.recipe_id.clone(),
        })
        .collect::<Vec<_>>();
    let triangle_bindings = artifact
        .triangle_bindings
        .iter()
        .map(|binding| SurfacePreviewTriangleBinding {
            triangle_index: binding.triangle_index,
            material_slot_id: binding.material_slot_id.clone(),
        })
        .collect::<Vec<_>>();
    let mut textures = Vec::new();
    for texture_set in &artifact.texture_sets {
        for file in &texture_set.files {
            let Some(channel) = preview_texture_channel(file.channel) else {
                continue;
            };
            let image = image::open(package_root.join(&file.path))
                .with_context(|| format!("opening preview texture {}", file.path))?
                .to_rgba8();
            textures.push(SurfacePreviewTexture {
                material_id: texture_set.material_recipe_id.clone(),
                channel,
                width: image.width(),
                height: image.height(),
                rgba8: image.into_raw(),
            });
        }
    }
    let mut request = default_surface_preview_request(
        mesh.clone(),
        uv_set.coordinates.clone(),
        material_bindings,
        triangle_bindings,
        textures,
        size,
    );
    request.sampling = TextureSampling::Bilinear;
    Ok(request)
}

fn preview_texture_channel(channel: TextureChannel) -> Option<SurfacePreviewTextureChannel> {
    match channel {
        TextureChannel::BaseColor => Some(SurfacePreviewTextureChannel::BaseColor),
        TextureChannel::Normal => Some(SurfacePreviewTextureChannel::Normal),
        TextureChannel::MetallicRoughness => Some(SurfacePreviewTextureChannel::MetallicRoughness),
        TextureChannel::Occlusion => Some(SurfacePreviewTextureChannel::Occlusion),
        TextureChannel::Emissive => None,
    }
}

fn write_static_crate_material_variants(
    mesh: &TriangleMesh,
    base_artifact: &SurfaceArtifact,
    base_pack: &MaterialStylePack,
    out_dir: &Path,
    base_preview: &RenderedImage,
) -> anyhow::Result<SurfaceMaterialVariantCandidateSet> {
    let specs = material_variant_specs();
    let mut candidates = Vec::with_capacity(specs.len());
    let mut preview_images = Vec::with_capacity(specs.len());

    for spec in specs {
        let variant_dir = format!("{SURFACE_VARIANTS_DIR}/{}", spec.slug);
        fs::create_dir_all(out_dir.join(&variant_dir))
            .with_context(|| format!("creating {}", out_dir.join(&variant_dir).display()))?;
        let material_pack = material_pack_variant(base_pack, &spec);
        let texture_dir = format!("{variant_dir}/textures");
        let texture_sets =
            write_static_crate_texture_payloads_in_dir(&material_pack, out_dir, &texture_dir)?;
        let mut artifact = base_artifact.clone();
        artifact.texture_sets = texture_sets;
        artifact.material_slots = artifact
            .material_slots
            .into_iter()
            .map(|mut slot| {
                if let Some(binding) = material_pack
                    .slot_bindings
                    .iter()
                    .find(|binding| binding.slot_id == slot.slot_id)
                {
                    slot.recipe_id = binding.material_id.clone();
                }
                slot
            })
            .collect();

        let surface_artifact_ref = format!("{variant_dir}/surface-artifact.json");
        let material_pack_ref = format!("{variant_dir}/material-pack.json");
        let textured_preview_ref = format!("{variant_dir}/textured-preview.png");
        let surface_delta_ref = format!("{variant_dir}/surface-delta.json");
        write_json(out_dir.join(&surface_artifact_ref), &artifact)?;
        write_json(out_dir.join(&material_pack_ref), &material_pack)?;

        let request = surface_preview_request_for_artifact(mesh, &artifact, out_dir, 256)?;
        let preview =
            render_surface_preview(&request).map_err(|error| anyhow::anyhow!("{error}"))?;
        save_png(&preview.image, out_dir.join(&textured_preview_ref))?;
        preview_images.push(preview.image.clone());

        let visible_delta = image_pixel_delta(base_preview, &preview.image);
        let delta = surface_visual_delta_report(
            base_artifact,
            &artifact,
            spec.slug,
            SurfaceVisualDeltaEvidence {
                base_frozen_mesh_fingerprint: base_artifact.source_artifact_fingerprint.clone(),
                candidate_frozen_mesh_fingerprint: artifact.source_artifact_fingerprint.clone(),
                base_textured_preview_ref: Some(SURFACE_TEXTURED_PREVIEW_FILE.to_owned()),
                candidate_textured_preview_ref: Some(textured_preview_ref.clone()),
                visible_surface_pixel_delta: Some(visible_delta),
            },
        );
        write_json(out_dir.join(&surface_delta_ref), &delta)?;
        candidates.push(SurfaceMaterialVariantCandidate {
            candidate_id: spec.slug.to_owned(),
            display_name: spec.display_name.to_owned(),
            variant_dir,
            surface_artifact_ref,
            material_pack_ref,
            textured_preview_ref,
            surface_delta_ref,
            frozen_mesh_fingerprint: base_artifact.source_artifact_fingerprint.clone(),
            preserves_frozen_geometry: !delta.shape_delta_leak_detected,
            result_class: delta.result_class,
            diagnostics: delta.diagnostics,
        });
    }

    let contact_sheet = surface_preview_contact_sheet(&preview_images);
    save_png(
        &contact_sheet,
        out_dir.join(SURFACE_VARIANTS_CONTACT_SHEET_FILE),
    )?;
    let set = SurfaceMaterialVariantCandidateSet {
        schema_version: shape_gamekit::surface::SURFACE_MATERIAL_VARIANT_CANDIDATES_SCHEMA_VERSION,
        profile_id: SCI_FI_CRATE_PROFILE.to_owned(),
        base_surface_artifact_ref: SURFACE_ARTIFACT_FILE.to_owned(),
        base_textured_preview_ref: SURFACE_TEXTURED_PREVIEW_FILE.to_owned(),
        candidates,
    };
    write_json(out_dir.join(SURFACE_VARIANTS_CANDIDATES_FILE), &set)?;
    Ok(set)
}

#[derive(Debug, Copy, Clone)]
struct MaterialVariantSpec {
    slug: &'static str,
    display_name: &'static str,
    body: [u8; 3],
    dark: [u8; 3],
    trim: [u8; 3],
    metal: [u8; 3],
    fallback: [u8; 3],
}

fn material_variant_specs() -> Vec<MaterialVariantSpec> {
    vec![
        MaterialVariantSpec {
            slug: "clean-lab-white",
            display_name: "Clean Lab White",
            body: [218, 224, 218],
            dark: [76, 83, 86],
            trim: [170, 190, 184],
            metal: [176, 178, 172],
            fallback: [190, 196, 190],
        },
        MaterialVariantSpec {
            slug: "worn-hazard-yellow",
            display_name: "Worn Hazard Yellow",
            body: [204, 166, 42],
            dark: [42, 39, 32],
            trim: [228, 190, 48],
            metal: [142, 135, 118],
            fallback: [116, 102, 68],
        },
        MaterialVariantSpec {
            slug: "dark-industrial-metal",
            display_name: "Dark Industrial Metal",
            body: [42, 47, 50],
            dark: [18, 20, 21],
            trim: [112, 116, 116],
            metal: [118, 124, 124],
            fallback: [54, 58, 60],
        },
        MaterialVariantSpec {
            slug: "field-blue-utility",
            display_name: "Field Blue Utility",
            body: [55, 94, 140],
            dark: [24, 34, 44],
            trim: [92, 132, 172],
            metal: [136, 146, 150],
            fallback: [58, 78, 102],
        },
        MaterialVariantSpec {
            slug: "graphite-cargo",
            display_name: "Graphite Cargo",
            body: [70, 76, 76],
            dark: [26, 28, 29],
            trim: [108, 116, 110],
            metal: [150, 150, 142],
            fallback: [82, 86, 84],
        },
        MaterialVariantSpec {
            slug: "orange-warning-trim",
            display_name: "Orange Warning Trim",
            body: [92, 98, 96],
            dark: [28, 30, 32],
            trim: [216, 102, 36],
            metal: [156, 148, 136],
            fallback: [104, 90, 78],
        },
    ]
}

fn material_pack_variant(
    base_pack: &MaterialStylePack,
    spec: &MaterialVariantSpec,
) -> MaterialStylePack {
    let mut pack = base_pack.clone();
    pack.id = format!("{}-{}", base_pack.id, spec.slug);
    pack.display_name = format!("{} - {}", base_pack.display_name, spec.display_name);
    for recipe in &mut pack.recipes {
        recipe.base_color_srgb = match recipe.material_id.as_str() {
            "worn-painted-sci-fi-metal" => spec.body,
            "dark-rubber-industrial-detail" => spec.dark,
            "exposed-edge-metal" => spec.metal,
            "industrial-warning-trim" => spec.trim,
            _ => spec.fallback,
        };
        recipe.display_name = format!("{} {}", spec.display_name, recipe.display_name);
    }
    pack
}

fn image_pixel_delta(left: &RenderedImage, right: &RenderedImage) -> f32 {
    let width = left.width.min(right.width);
    let height = left.height.min(right.height);
    if width == 0 || height == 0 {
        return 0.0;
    }
    let mut total = 0.0_f32;
    let mut count = 0_u32;
    for y in 0..height {
        for x in 0..width {
            let Some(left_pixel) = left.pixel(x, y) else {
                continue;
            };
            let Some(right_pixel) = right.pixel(x, y) else {
                continue;
            };
            total += ((f32::from(left_pixel[0]) - f32::from(right_pixel[0])).abs()
                + (f32::from(left_pixel[1]) - f32::from(right_pixel[1])).abs()
                + (f32::from(left_pixel[2]) - f32::from(right_pixel[2])).abs())
                / (255.0 * 3.0);
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        0.0
    } else {
        (total / count as f32).clamp(0.0, 1.0)
    }
}

fn uv_to_pixel(uv: [f32; 2], size: u32, margin: i32) -> [i32; 2] {
    let span = (size as i32 - margin * 2).max(1) as f32;
    [
        margin + (uv[0].clamp(0.0, 1.0) * span).round() as i32,
        margin + ((1.0 - uv[1].clamp(0.0, 1.0)) * span).round() as i32,
    ]
}

fn draw_line(image: &mut RgbaImage, start: [i32; 2], end: [i32; 2], color: Rgba<u8>) {
    let mut x0 = start[0];
    let mut y0 = start[1];
    let x1 = end[0];
    let y1 = end[1];
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    loop {
        if x0 >= 0 && y0 >= 0 && (x0 as u32) < image.width() && (y0 as u32) < image.height() {
            image.put_pixel(x0 as u32, y0 as u32, color);
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * error;
        if e2 >= dy {
            error += dy;
            x0 += sx;
        }
        if e2 <= dx {
            error += dx;
            y0 += sy;
        }
    }
}

fn slot_color(slot_id: &str) -> Rgba<u8> {
    match slot_id {
        "painted_metal_body" => Rgba([112, 148, 128, 255]),
        "dark_recesses_and_vents" => Rgba([82, 96, 112, 255]),
        "exposed_edge_trim" => Rgba([220, 178, 72, 255]),
        "handle_grip" => Rgba([146, 154, 164, 255]),
        "fasteners_and_mounts" => Rgba([190, 190, 178, 255]),
        _ => Rgba([160, 160, 160, 255]),
    }
}

fn texture_channel_slug(channel: TextureChannel) -> &'static str {
    match channel {
        TextureChannel::BaseColor => "base_color",
        TextureChannel::Normal => "normal",
        TextureChannel::MetallicRoughness => "metallic_roughness",
        TextureChannel::Occlusion => "occlusion",
        TextureChannel::Emissive => "emissive",
    }
}

fn game_asset_pack_for_static_crate(
    output: &FoundryCompilationOutput,
    collision_proxy: CollisionProxy,
) -> GameAssetPack {
    let triangle_budget = output
        .artifact
        .statistics
        .triangle_count
        .clamp(1, u64::from(u32::MAX)) as u32;
    GameAssetPack {
        schema_version: GAME_ASSET_PACK_SCHEMA_VERSION,
        id: "sci-fi-crate-static-prop-v1".to_owned(),
        title: "Sci-Fi Crate Static Prop v1".to_owned(),
        assets: vec![GameAssetDefinition {
            id: "asset:sci-fi-crate-static-prop-v1".to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            family: "Static Prop".to_owned(),
            source_recipe: output.recipe.clone(),
            module_semantics: ModuleSemantics {
                runtime_key: "sci_fi_crate_static_prop".to_owned(),
                logical_footprint: LogicalFootprint {
                    cell_bounds: CellBounds {
                        min: [0, 0],
                        max: [1, 1],
                    },
                    vertical_layers: LayerBounds { min: 0, max: 0 },
                    origin_cell: [0, 0],
                    permitted_rotations: vec![
                        GridRotation::R0,
                        GridRotation::R90,
                        GridRotation::R180,
                        GridRotation::R270,
                    ],
                },
                rotation_symmetry: RotationSymmetry::TwoWay,
                instanceable: true,
                snap_anchors: vec![SnapAnchor {
                    id: "center".to_owned(),
                    role: SnapAnchorRole::Center,
                    local_frame: Frame3::default(),
                    compatibility_tags: vec!["static_prop".to_owned(), "crate".to_owned()],
                    relationship: SnapRelationship::Optional,
                }],
                support_surfaces: vec![SupportSurface {
                    id: "top_support".to_owned(),
                    shape: SurfaceShape::Rectangle {
                        center: [1.0, 1.0],
                        size: [1.0, 1.0],
                    },
                    support_role: SupportRole::Custom("static-prop-top".to_owned()),
                    maximum_supported_layer_hint: Some(1),
                }],
                walkable_surfaces: Vec::new(),
                traversal_links: Vec::new(),
                collision_proxies: vec![collision_proxy],
                gameplay_tags: vec![
                    GameplayTag::BlocksMovement,
                    GameplayTag::CoverSource,
                    GameplayTag::Custom("static_prop".to_owned()),
                ],
            },
            construction_profile: ConstructionProfile {
                phases: vec![
                    ConstructionPhase {
                        id: "placed".to_owned(),
                        label: "Placed".to_owned(),
                        progress_threshold: 0.0,
                        visible_part_tags: vec!["body".to_owned()],
                        required_predecessor: None,
                    },
                    ConstructionPhase {
                        id: "complete".to_owned(),
                        label: "Complete".to_owned(),
                        progress_threshold: 1.0,
                        visible_part_tags: vec![
                            "body".to_owned(),
                            "panel".to_owned(),
                            "fastener".to_owned(),
                            "handle".to_owned(),
                            "trim".to_owned(),
                        ],
                        required_predecessor: Some("placed".to_owned()),
                    },
                ],
                optional_damaged_state: None,
                final_phase: "complete".to_owned(),
                monotonic_visibility_policy: MonotonicVisibilityPolicy::Strict,
            },
            readability_profile: ReadabilityProfile {
                fixed_camera_profiles: vec![
                    FixedCameraProfile::Oblique,
                    FixedCameraProfile::Top,
                    FixedCameraProfile::LowOblique,
                ],
                minimum_recognizable_pixel_size: 40,
                silhouette_importance: 0.55,
                maximum_hidden_area_fraction: 0.35,
                orientation_coverage: vec![
                    GridRotation::R0,
                    GridRotation::R90,
                    GridRotation::R180,
                    GridRotation::R270,
                ],
            },
            budgets: TriangleBudget {
                preview_maximum: triangle_budget,
                game_maximum: triangle_budget,
                repeated_instance_maximum: triangle_budget.min(2_000),
            },
            tags: vec![
                "sci-fi".to_owned(),
                "crate".to_owned(),
                "static-prop".to_owned(),
                "internal-dogfood".to_owned(),
            ],
        }],
        export_profile: ExportProfile::internal_dogfood(),
        source_revision: format!(
            "shape-lab:{}:{}",
            SCI_FI_CRATE_PROFILE,
            output.build_stamp.recipe_fingerprint.0.to_hex()
        ),
    }
}

fn collision_proxy_for_bounds(bounds: Aabb) -> CollisionProxy {
    let center = bounds.center();
    let half_extents = bounds.extent() * 0.5 + Vec3::splat(0.015);
    CollisionProxy::Box {
        center: center.to_array(),
        half_extents: half_extents.max(Vec3::splat(0.01)).to_array(),
    }
}

fn box_mesh_from_collision(proxy: &CollisionProxy) -> TriangleMesh {
    match proxy {
        CollisionProxy::Box {
            center,
            half_extents,
        } => box_mesh(Vec3::from_array(*center), Vec3::from_array(*half_extents)),
        _ => box_mesh(Vec3::ZERO, Vec3::splat(0.5)),
    }
}

fn box_mesh_from_bounds(bounds: Aabb) -> TriangleMesh {
    let half_extents = bounds.extent() * 0.5;
    box_mesh(bounds.center(), half_extents.max(Vec3::splat(0.01)))
}

fn box_mesh(center: Vec3, half_extents: Vec3) -> TriangleMesh {
    let min = center - half_extents;
    let max = center + half_extents;
    let positions = vec![
        [min.x, min.y, min.z],
        [max.x, min.y, min.z],
        [max.x, max.y, min.z],
        [min.x, max.y, min.z],
        [min.x, min.y, max.z],
        [max.x, min.y, max.z],
        [max.x, max.y, max.z],
        [min.x, max.y, max.z],
    ];
    let normals = positions
        .iter()
        .map(|position| {
            let n = Vec3::from_array(*position) - center;
            n.try_normalize().unwrap_or(Vec3::Y).to_array()
        })
        .collect::<Vec<_>>();
    let indices = vec![
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ];
    TriangleMesh {
        positions,
        normals,
        indices,
        bounds: Aabb { min, max },
    }
}

fn render_static_view(
    mesh: &TriangleMesh,
    yaw_degrees: f32,
    pitch_degrees: f32,
    wireframe: bool,
) -> anyhow::Result<RenderedImage> {
    let camera = fit_camera_to_bounds_from_angles(mesh.bounds, yaw_degrees, pitch_degrees, 1.0);
    let settings = RenderSettings {
        width: 512,
        height: 512,
        wireframe,
        ..RenderSettings::default()
    };
    render_mesh(mesh, &camera, &settings).context("rendering static prop preview")
}

fn rel_model_package_path(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{MODEL_PACKAGE_DIR}/{name}"))
        .unwrap_or_else(|| format!("{MODEL_PACKAGE_DIR}/blender_reconstruct.py"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_gamekit::export::{
        StaticPropGameReadyPackage, StaticPropReadinessReport, StaticPropReadinessStatus,
    };
    use shape_gamekit::gltf::StaticPropGlbValidationReport;
    use shape_gamekit::surface::{
        SurfaceArtifact, SurfaceCapabilities, SurfaceLabPackage, SurfaceLabStatus,
        SurfaceLabValidationReport, validate_surface_artifact_with_root,
    };

    #[test]
    fn sci_fi_crate_static_prop_bundle_records_blocked_readiness_truthfully() {
        let temp = tempfile::tempdir().expect("tempdir");

        let summary =
            generate_static_prop_package(GameReadyStaticPropProfile::SciFiCrate, temp.path())
                .expect("static prop bundle");

        assert_eq!(summary.profile_id, SCI_FI_CRATE_PROFILE);
        assert!(!summary.game_ready);
        assert!(
            summary
                .blocker_codes
                .contains(&"manual_review_pending".to_owned())
        );
        assert!(
            !summary
                .blocker_codes
                .contains(&"glb_export_not_implemented".to_owned())
        );
        assert!(
            summary
                .blocker_codes
                .contains(&"engine_import_proof_missing".to_owned())
        );
        assert!(
            summary
                .blocker_codes
                .contains(&"engine_native_package_not_implemented".to_owned())
        );
        assert!(
            !summary
                .blocker_codes
                .contains(&"uv_layout_not_implemented".to_owned()),
            "UV-not-implemented blocker should be removed after Surface Lab v1 UV generation"
        );
        for file in [
            STATIC_PROP_PACKAGE_FILE,
            VALIDATION_REPORT_FILE,
            GAME_ASSET_PACK_FILE,
            FROZEN_OBJ_FILE,
            GLB_FILE,
            GLB_VALIDATION_REPORT_FILE,
            SURFACE_GLB_FILE,
            SURFACE_GLB_VALIDATION_REPORT_FILE,
            SURFACE_LAB_PACKAGE_FILE,
            SURFACE_LAB_VALIDATION_REPORT_FILE,
            SURFACE_ARTIFACT_FILE,
            SURFACE_VALIDATION_REPORT_FILE,
            SURFACE_CAPABILITIES_FILE,
            MATERIAL_PACK_FILE,
            TEXTURE_REQUIREMENTS_FILE,
            UNSUPPORTED_TEXTURE_REPORT_FILE,
            SURFACE_UV_LAYOUT_FILE,
            SURFACE_SWATCH_SHEET_FILE,
            SURFACE_TEXTURE_CONTACT_SHEET_FILE,
            SURFACE_TEXTURED_PREVIEW_FILE,
            SURFACE_TEXTURED_CONTACT_SHEET_FILE,
            SURFACE_MATERIAL_SLOT_OVERLAY_FILE,
            SURFACE_PREVIEW_REPORT_FILE,
            SURFACE_VARIANTS_CANDIDATES_FILE,
            SURFACE_VARIANTS_CONTACT_SHEET_FILE,
            SURFACE_TRIANGLE_COVERAGE_FILE,
            LOD1_PROXY_FILE,
            LOD2_COLLISION_FILE,
            FRONT_PREVIEW_FILE,
            THREE_QUARTER_PREVIEW_FILE,
            SIDE_PREVIEW_FILE,
            WIREFRAME_PREVIEW_FILE,
            CONTACT_SHEET_FILE,
        ] {
            assert!(temp.path().join(file).exists(), "missing {file}");
        }

        let report: StaticPropReadinessReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(VALIDATION_REPORT_FILE)).expect("report json"),
        )
        .expect("report decodes");
        assert_eq!(report.status, StaticPropReadinessStatus::Blocked);
        assert!(!report.is_ready());

        let package: StaticPropGameReadyPackage = serde_json::from_str(
            &fs::read_to_string(temp.path().join(STATIC_PROP_PACKAGE_FILE)).expect("package json"),
        )
        .expect("package decodes");
        assert_eq!(package.profile_id, SCI_FI_CRATE_PROFILE);
        assert_eq!(package.handoff.glb_status, StaticPropFeatureStatus::Ready);
        assert_eq!(
            package.handoff.glb_artifact.as_deref(),
            Some(SURFACE_GLB_FILE)
        );
        assert_eq!(package.material_slots.len(), 6);
        assert_eq!(package.uv_policy.status, StaticPropFeatureStatus::Ready);
        assert_eq!(package.uv_policy.blocker_code, None);
        assert_eq!(
            package.surface_artifact.as_deref(),
            Some(SURFACE_ARTIFACT_FILE)
        );

        let surface_report: SurfaceLabValidationReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_LAB_VALIDATION_REPORT_FILE))
                .expect("surface report json"),
        )
        .expect("surface report decodes");
        assert_eq!(surface_report.status, SurfaceLabStatus::Ready);
        assert!(surface_report.is_ready(), "{surface_report:#?}");

        let surface_artifact: SurfaceArtifact = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_ARTIFACT_FILE))
                .expect("surface artifact json"),
        )
        .expect("surface artifact decodes");
        assert_eq!(surface_artifact.profile_id, SCI_FI_CRATE_PROFILE);
        assert_eq!(
            surface_artifact.triangle_bindings.len(),
            output_triangle_count(&surface_artifact)
        );
        assert!(
            surface_artifact
                .uv_sets
                .iter()
                .flat_map(|set| set.coordinates.iter())
                .all(|uv| uv[0].is_finite() && uv[1].is_finite())
        );
        let surface_artifact_report =
            validate_surface_artifact_with_root(&surface_artifact, temp.path());
        assert_eq!(surface_artifact_report.status, SurfaceLabStatus::Blocked);
        assert!(
            surface_artifact_report
                .blockers
                .iter()
                .any(|issue| issue.code == "surface_manual_review_required"),
            "{surface_artifact_report:#?}"
        );
        assert!(
            surface_artifact_report
                .blockers
                .iter()
                .all(|issue| !issue.code.contains("uv")),
            "{surface_artifact_report:#?}"
        );
        let coverage_sum: u32 = surface_artifact
            .material_slots
            .iter()
            .map(|slot| slot.coverage_triangle_count)
            .sum();
        assert_eq!(
            coverage_sum as usize,
            surface_artifact.triangle_bindings.len()
        );
        for texture_set in &surface_artifact.texture_sets {
            assert!(texture_set.payload_ready);
            for file in &texture_set.files {
                assert!(
                    temp.path().join(&file.path).is_file(),
                    "missing {}",
                    file.path
                );
                assert_eq!(
                    image::image_dimensions(temp.path().join(&file.path))
                        .expect("texture dimensions"),
                    (SURFACE_TEXTURE_SIZE, SURFACE_TEXTURE_SIZE)
                );
            }
        }
        assert!(temp.path().join(SURFACE_UV_LAYOUT_FILE).is_file());
        assert!(temp.path().join(SURFACE_SWATCH_SHEET_FILE).is_file());
        assert!(
            temp.path()
                .join(SURFACE_TEXTURE_CONTACT_SHEET_FILE)
                .is_file()
        );
        assert!(temp.path().join(SURFACE_TEXTURED_PREVIEW_FILE).is_file());
        assert!(
            temp.path()
                .join(SURFACE_TEXTURED_CONTACT_SHEET_FILE)
                .is_file()
        );
        assert!(temp.path().join(SURFACE_PREVIEW_REPORT_FILE).is_file());
        assert!(temp.path().join(SURFACE_VARIANTS_CANDIDATES_FILE).is_file());
        assert!(
            temp.path()
                .join(SURFACE_VARIANTS_CONTACT_SHEET_FILE)
                .is_file()
        );
        let variant_candidates: SurfaceMaterialVariantCandidateSet = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_VARIANTS_CANDIDATES_FILE))
                .expect("variant candidates json"),
        )
        .expect("variant candidates decode");
        assert_eq!(variant_candidates.candidates.len(), 6);
        assert!(
            variant_candidates
                .candidates
                .iter()
                .all(|candidate| candidate.preserves_frozen_geometry)
        );
        for candidate in &variant_candidates.candidates {
            assert!(
                temp.path().join(&candidate.textured_preview_ref).is_file(),
                "missing {}",
                candidate.textured_preview_ref
            );
            assert!(
                temp.path().join(&candidate.surface_delta_ref).is_file(),
                "missing {}",
                candidate.surface_delta_ref
            );
        }

        let capabilities: SurfaceCapabilities = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_CAPABILITIES_FILE))
                .expect("capabilities json"),
        )
        .expect("capabilities decode");
        assert!(capabilities.surface_payload_ready);
        assert!(capabilities.uv_ready);
        assert!(!capabilities.focus_part_surface_ready);

        let glb_report: StaticPropGlbValidationReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(GLB_VALIDATION_REPORT_FILE))
                .expect("glb report json"),
        )
        .expect("glb report decodes");
        assert!(glb_report.valid, "{glb_report:#?}");
        assert_eq!(glb_report.version, Some(2));
        let surface_glb_report: StaticPropGlbValidationReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_GLB_VALIDATION_REPORT_FILE))
                .expect("surface glb report json"),
        )
        .expect("surface glb report decodes");
        assert!(surface_glb_report.valid, "{surface_glb_report:#?}");

        let surface_package: SurfaceLabPackage = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_LAB_PACKAGE_FILE))
                .expect("surface package json"),
        )
        .expect("surface package decodes");
        assert_eq!(
            package
                .material_slots
                .iter()
                .map(|slot| (
                    slot.slot_id.as_str(),
                    slot.display_name.as_str(),
                    slot.semantic_roles.as_slice()
                ))
                .collect::<Vec<_>>(),
            surface_package
                .material_pack
                .slot_bindings
                .iter()
                .map(|slot| (
                    slot.slot_id.as_str(),
                    slot.display_name.as_str(),
                    slot.semantic_roles.as_slice()
                ))
                .collect::<Vec<_>>()
        );

        let game_asset_pack: GameAssetPack = serde_json::from_str(
            &fs::read_to_string(temp.path().join(GAME_ASSET_PACK_FILE))
                .expect("game asset pack json"),
        )
        .expect("game asset pack decodes");
        assert!(validate_game_asset_pack(&game_asset_pack).is_valid());
        assert_eq!(game_asset_pack.export_profile.id, "internal-dogfood");
    }

    #[test]
    fn sci_fi_crate_static_prop_manifest_is_deterministic() {
        let left = tempfile::tempdir().expect("left tempdir");
        let right = tempfile::tempdir().expect("right tempdir");

        generate_static_prop_package(GameReadyStaticPropProfile::SciFiCrate, left.path())
            .expect("left bundle");
        generate_static_prop_package(GameReadyStaticPropProfile::SciFiCrate, right.path())
            .expect("right bundle");

        for file in [
            STATIC_PROP_PACKAGE_FILE,
            VALIDATION_REPORT_FILE,
            GAME_ASSET_PACK_FILE,
            GLB_FILE,
            GLB_VALIDATION_REPORT_FILE,
            SURFACE_LAB_PACKAGE_FILE,
            SURFACE_LAB_VALIDATION_REPORT_FILE,
            MATERIAL_PACK_FILE,
            TEXTURE_REQUIREMENTS_FILE,
            UNSUPPORTED_TEXTURE_REPORT_FILE,
            SURFACE_ARTIFACT_FILE,
            SURFACE_VALIDATION_REPORT_FILE,
            SURFACE_CAPABILITIES_FILE,
            SURFACE_UV_LAYOUT_FILE,
            SURFACE_SWATCH_SHEET_FILE,
            SURFACE_TEXTURE_CONTACT_SHEET_FILE,
            SURFACE_TRIANGLE_COVERAGE_FILE,
            SURFACE_TEXTURED_PREVIEW_FILE,
            SURFACE_TEXTURED_CONTACT_SHEET_FILE,
            SURFACE_MATERIAL_SLOT_OVERLAY_FILE,
            SURFACE_PREVIEW_REPORT_FILE,
            SURFACE_VARIANTS_CANDIDATES_FILE,
            SURFACE_VARIANTS_CONTACT_SHEET_FILE,
            SURFACE_GLB_FILE,
            SURFACE_GLB_VALIDATION_REPORT_FILE,
        ] {
            let left_bytes = fs::read(left.path().join(file))
                .unwrap_or_else(|error| panic!("read left {file}: {error}"));
            let right_bytes = fs::read(right.path().join(file))
                .unwrap_or_else(|error| panic!("read right {file}: {error}"));
            assert_eq!(left_bytes, right_bytes, "{file} should be deterministic");
        }
        for texture in deterministic_texture_files(left.path()) {
            let relative = texture.strip_prefix(left.path()).expect("relative texture");
            let right_texture = right.path().join(relative);
            assert_eq!(
                fs::read(&texture).expect("left texture"),
                fs::read(&right_texture).expect("right texture"),
                "{} should be deterministic",
                relative.display()
            );
        }
    }

    fn output_triangle_count(surface_artifact: &SurfaceArtifact) -> usize {
        surface_artifact
            .triangle_bindings
            .iter()
            .map(|binding| binding.triangle_index as usize)
            .max()
            .map_or(0, |max| max + 1)
    }

    fn deterministic_texture_files(root: &Path) -> Vec<PathBuf> {
        let mut files = fs::read_dir(root.join(SURFACE_TEXTURE_DIR))
            .expect("texture dir")
            .map(|entry| entry.expect("texture entry").path())
            .collect::<Vec<_>>();
        files.sort();
        files
    }
}
