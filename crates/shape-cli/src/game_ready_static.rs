use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::ValueEnum;
use glam::Vec3;
use image::{Rgba, RgbaImage};
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
use shape_gamekit::gltf::{StaticPropGlbMetadata, write_static_prop_glb};
use shape_gamekit::surface::{
    MaterialRecipe, MaterialSlotBinding, MaterialStylePack, SURFACE_LAB_PACKAGE_SCHEMA_VERSION,
    SurfaceEvidence, SurfaceLabPackage, SurfaceUvLayoutPolicy, TextureChannel, TextureRequirement,
    TextureRequirementSet, UnsupportedSurfaceOutput, UnsupportedSurfaceOutputReport,
    validate_surface_lab_package_with_root,
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
const GROUPED_OBJ_REPORT_FILE: &str = "grouped-obj-report.json";
const SOURCE_DOCUMENT_FILE: &str = "source-document.json";
const SOURCE_RECIPE_FILE: &str = "source-recipe.json";
const SURFACE_LAB_PACKAGE_FILE: &str = "surface-lab-package.json";
const SURFACE_LAB_VALIDATION_REPORT_FILE: &str = "surface-lab-validation-report.json";
const MATERIAL_PACK_FILE: &str = "material-pack.json";
const TEXTURE_REQUIREMENTS_FILE: &str = "texture-requirements.json";
const UNSUPPORTED_TEXTURE_REPORT_FILE: &str = "unsupported-texture-report.json";
const SURFACE_SWATCH_SHEET_FILE: &str = "surface-evidence/swatch-sheet.png";
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
    fs::create_dir_all(out_dir.join("surface-evidence"))
        .with_context(|| format!("creating {}", out_dir.join("surface-evidence").display()))?;

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

    let glb_report = write_static_prop_glb(
        &mesh,
        &StaticPropGlbMetadata {
            profile_id: profile.slug().to_owned(),
            display_name: "Sci-Fi Crate".to_owned(),
            material_slots: static_crate_material_slot_assignments()
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

    let surface_lab_package = surface_lab_package_for_static_crate();
    write_swatch_sheet(
        &surface_lab_package.material_pack,
        out_dir.join(SURFACE_SWATCH_SHEET_FILE),
    )?;
    write_json(
        out_dir.join(MATERIAL_PACK_FILE),
        &surface_lab_package.material_pack,
    )?;
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
        glb_report.valid,
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
    glb_valid: bool,
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
            status: StaticPropFeatureStatus::NotImplemented,
            required_for_game_ready: true,
            blocker_code: Some("uv_layout_not_implemented".to_owned()),
            explanation: "UV unwrapping is not implemented in this Shape Lab package; the package records the blocker instead of claiming texture-ready output.".to_owned(),
        },
        collision: StaticPropCollision {
            source: "compiled preview bounds expanded into a simple box proxy".to_owned(),
            proxies: vec![collision_proxy],
        },
        handoff: StaticPropHandoff {
            primary_package_format: "shape-lab-canonical-model-package".to_owned(),
            blender_handoff_script: format!("{MODEL_PACKAGE_DIR}/blender_reconstruct.py"),
            blender_status: StaticPropFeatureStatus::Ready,
            glb_artifact: Some(GLB_FILE.to_owned()),
            glb_status: if glb_valid {
                StaticPropFeatureStatus::Ready
            } else {
                StaticPropFeatureStatus::ManualRequired
            },
            glb_blocker_code: (!glb_valid).then(|| "glb_validation_failed".to_owned()),
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
            policy: "Primary painted hard-surface body slot. No material graph or texture payload is emitted.".to_owned(),
            material_payload_ready: false,
        },
        MaterialSlotAssignment {
            slot_id: "dark_recessed_panels".to_owned(),
            display_name: "Dark Recessed Panels".to_owned(),
            semantic_roles: vec!["recessed-panel".to_owned(), "inset".to_owned()],
            policy: "Dark inset panels and recessed shapes. Procedural policy only.".to_owned(),
            material_payload_ready: false,
        },
        MaterialSlotAssignment {
            slot_id: "vents".to_owned(),
            display_name: "Vents".to_owned(),
            semantic_roles: vec!["vent".to_owned(), "grille".to_owned()],
            policy: "Vent and grille details use the dark industrial detail policy.".to_owned(),
            material_payload_ready: false,
        },
        MaterialSlotAssignment {
            slot_id: "handles".to_owned(),
            display_name: "Handles".to_owned(),
            semantic_roles: vec!["handle".to_owned(), "side-handle".to_owned()],
            policy: "Handle hardware uses exposed edge metal policy until final materials exist.".to_owned(),
            material_payload_ready: false,
        },
        MaterialSlotAssignment {
            slot_id: "fasteners".to_owned(),
            display_name: "Fasteners".to_owned(),
            semantic_roles: vec!["fastener".to_owned(), "bolt".to_owned(), "rivet".to_owned()],
            policy: "Fasteners and small hardware use exposed metal policy.".to_owned(),
            material_payload_ready: false,
        },
        MaterialSlotAssignment {
            slot_id: "edge_trim".to_owned(),
            display_name: "Edge Trim".to_owned(),
            semantic_roles: vec!["edge-trim".to_owned(), "bevel".to_owned(), "rail".to_owned()],
            policy: "Trim and rails reserve a warning/edge-wear policy. Texture payloads are not emitted.".to_owned(),
            material_payload_ready: false,
        },
    ]
}

fn surface_lab_package_for_static_crate() -> SurfaceLabPackage {
    SurfaceLabPackage {
        schema_version: SURFACE_LAB_PACKAGE_SCHEMA_VERSION,
        profile_id: SCI_FI_CRATE_PROFILE.to_owned(),
        display_name: "Sci-Fi Crate".to_owned(),
        asset_family: "Static Prop Surface Lab".to_owned(),
        uv_layout: SurfaceUvLayoutPolicy {
            status: StaticPropFeatureStatus::NotImplemented,
            required_for_texture_ready: true,
            policy: "No authored unwrap yet; future milestone may start with deterministic box/projected regions for hard-surface slots.".to_owned(),
            blocker_code: Some("uv_layout_not_implemented".to_owned()),
            explanation: "UV layout is required before texture-ready or game-ready surface claims. This package records the missing UV layout as a blocker.".to_owned(),
        },
        material_pack: MaterialStylePack {
            id: "sci-fi-crate-industrial-surface-recipes-v0".to_owned(),
            display_name: "Industrial Sci-Fi Surface Recipes".to_owned(),
            texture_packing_policy: "Future texture set: base color, normal, and ORM packing where occlusion, roughness, and metallic share one map.".to_owned(),
            recipes: vec![
                MaterialRecipe {
                    material_id: "clean-painted-sci-fi-metal".to_owned(),
                    display_name: "Clean Painted Sci-Fi Metal".to_owned(),
                    base_color_srgb: [124, 132, 120],
                    metallic: 0.0,
                    roughness: 0.68,
                    wear_policy: "No wear mask; clean painted finish.".to_owned(),
                    texture_payload_ready: false,
                },
                MaterialRecipe {
                    material_id: "worn-painted-sci-fi-metal".to_owned(),
                    display_name: "Worn Painted Sci-Fi Metal".to_owned(),
                    base_color_srgb: [104, 112, 102],
                    metallic: 0.0,
                    roughness: 0.76,
                    wear_policy: "Reserve edge-wear and corner-chipping masks for later authored textures.".to_owned(),
                    texture_payload_ready: false,
                },
                MaterialRecipe {
                    material_id: "dark-rubber-industrial-detail".to_owned(),
                    display_name: "Dark Rubber / Industrial Detail".to_owned(),
                    base_color_srgb: [28, 31, 32],
                    metallic: 0.0,
                    roughness: 0.84,
                    wear_policy: "Reserve dust and grime masks for recessed details.".to_owned(),
                    texture_payload_ready: false,
                },
                MaterialRecipe {
                    material_id: "exposed-edge-metal".to_owned(),
                    display_name: "Exposed Edge Metal".to_owned(),
                    base_color_srgb: [154, 150, 138],
                    metallic: 0.75,
                    roughness: 0.46,
                    wear_policy: "Reserve bright-edge wear masks for handles and fasteners.".to_owned(),
                    texture_payload_ready: false,
                },
                MaterialRecipe {
                    material_id: "industrial-warning-trim".to_owned(),
                    display_name: "Industrial Warning Trim".to_owned(),
                    base_color_srgb: [198, 148, 44],
                    metallic: 0.0,
                    roughness: 0.7,
                    wear_policy: "Reserve stripe/trim masking and chipped paint for later authored textures.".to_owned(),
                    texture_payload_ready: false,
                },
            ],
            slot_bindings: surface_material_slot_bindings_for_static_crate(
                &static_crate_material_slot_assignments(),
            ),
        },
        texture_requirements: TextureRequirementSet {
            id: "sci-fi-crate-texture-requirements-v0".to_owned(),
            channels: vec![
                TextureRequirement {
                    channel: TextureChannel::BaseColor,
                    resolution_px: 1024,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::NotImplemented,
                    blocker_code: Some("base_color_texture_not_authored".to_owned()),
                    explanation: "A base-color texture or approved procedural bake is required before claiming textured output.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::Normal,
                    resolution_px: 1024,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::NotImplemented,
                    blocker_code: Some("normal_texture_not_authored".to_owned()),
                    explanation: "A normal texture is required before claiming a complete hard-surface texture set.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::MetallicRoughness,
                    resolution_px: 1024,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::NotImplemented,
                    blocker_code: Some("orm_texture_not_authored".to_owned()),
                    explanation: "Packed occlusion/roughness/metallic output is required before runtime texture handoff.".to_owned(),
                },
                TextureRequirement {
                    channel: TextureChannel::Occlusion,
                    resolution_px: 1024,
                    required_for_texture_ready: true,
                    status: StaticPropFeatureStatus::NotImplemented,
                    blocker_code: Some("occlusion_texture_not_authored".to_owned()),
                    explanation: "Ambient occlusion is required for the future static-prop texture evidence bundle.".to_owned(),
                },
            ],
        },
        unsupported_outputs: UnsupportedSurfaceOutputReport {
            outputs: vec![
                UnsupportedSurfaceOutput {
                    output: "authored-uv-layout".to_owned(),
                    blocker_code: "uv_layout_not_implemented".to_owned(),
                    explanation: "No authored or generated UV island layout is emitted yet.".to_owned(),
                },
                UnsupportedSurfaceOutput {
                    output: "baked-texture-set".to_owned(),
                    blocker_code: "texture_baking_not_implemented".to_owned(),
                    explanation: "No base color, normal, ORM, or packed texture files are baked yet.".to_owned(),
                },
                UnsupportedSurfaceOutput {
                    output: "engine-material-graph".to_owned(),
                    blocker_code: "engine_material_graph_not_implemented".to_owned(),
                    explanation: "No Unity, Unreal, Godot, or DCC-specific material graph is emitted.".to_owned(),
                },
            ],
        },
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
        "dark_recessed_panels" | "vents" => "dark-rubber-industrial-detail",
        "handles" | "fasteners" => "exposed-edge-metal",
        "edge_trim" => "industrial-warning-trim",
        _ => "clean-painted-sci-fi-metal",
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
    use shape_gamekit::surface::{SurfaceLabPackage, SurfaceLabStatus, SurfaceLabValidationReport};

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
                .contains(&"uv_layout_not_implemented".to_owned())
        );
        assert!(
            !summary
                .blocker_codes
                .contains(&"glb_export_not_implemented".to_owned())
        );
        assert!(
            summary
                .blocker_codes
                .contains(&"manual_review_pending".to_owned())
        );
        for file in [
            STATIC_PROP_PACKAGE_FILE,
            VALIDATION_REPORT_FILE,
            GAME_ASSET_PACK_FILE,
            FROZEN_OBJ_FILE,
            GLB_FILE,
            GLB_VALIDATION_REPORT_FILE,
            SURFACE_LAB_PACKAGE_FILE,
            SURFACE_LAB_VALIDATION_REPORT_FILE,
            MATERIAL_PACK_FILE,
            TEXTURE_REQUIREMENTS_FILE,
            UNSUPPORTED_TEXTURE_REPORT_FILE,
            SURFACE_SWATCH_SHEET_FILE,
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
        assert_eq!(package.handoff.glb_artifact.as_deref(), Some(GLB_FILE));
        assert_eq!(package.material_slots.len(), 6);
        assert_eq!(
            package.uv_policy.status,
            StaticPropFeatureStatus::NotImplemented
        );

        let surface_report: SurfaceLabValidationReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(SURFACE_LAB_VALIDATION_REPORT_FILE))
                .expect("surface report json"),
        )
        .expect("surface report decodes");
        let surface_blockers = surface_report
            .blockers
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>();
        assert_eq!(surface_report.status, SurfaceLabStatus::Blocked);
        assert!(surface_blockers.contains(&"uv_layout_not_implemented"));
        assert!(surface_blockers.contains(&"texture_payload_policy_only"));
        assert!(surface_blockers.contains(&"base_color_texture_not_authored"));

        let glb_report: StaticPropGlbValidationReport = serde_json::from_str(
            &fs::read_to_string(temp.path().join(GLB_VALIDATION_REPORT_FILE))
                .expect("glb report json"),
        )
        .expect("glb report decodes");
        assert!(glb_report.valid, "{glb_report:#?}");
        assert_eq!(glb_report.version, Some(2));

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
            SURFACE_SWATCH_SHEET_FILE,
        ] {
            let left_bytes = fs::read(left.path().join(file))
                .unwrap_or_else(|error| panic!("read left {file}: {error}"));
            let right_bytes = fs::read(right.path().join(file))
                .unwrap_or_else(|error| panic!("read right {file}: {error}"));
            assert_eq!(left_bytes, right_bytes, "{file} should be deterministic");
        }
    }
}
