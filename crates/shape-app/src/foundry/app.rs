//! Native desktop host for the Foundry workflow state.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, RichText, TextureOptions};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use shape_foundry::{
    CatalogContentRef, DirectKitCreatedFrom, DirectKitDraft, DirectKitPresetRef,
    DirectKitSourceKind, DirectKitVisibility, FoundryAssetDocument, FoundryBuildStamp,
    FoundryCatalogError, FoundryCatalogResolver, FoundryCommand, FoundryCompilationOutput,
    KitCapabilityAvailability, KitCapabilityCard, KitCapabilitySourceKind, ObjectPlanReviewTier,
    PresetSource, PrimitiveKind, VariationIntent, built_in_surface_capability_for_profile,
    compile_foundry_document, direct_kit_property_exposures_for_primitive, direct_kit_user_summary,
    kit_capability_cards_for_panel_with_knob, kit_capability_cards_for_primitive, save_direct_kit,
    validate_direct_kit_draft,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, curated_fixture_catalogs_with_labels, headless_fixture_catalogs,
};
use shape_mesh::TriangleMesh;
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryProject, FoundryProjectFile, ensure_foundry_project_path,
};
use shape_render::{
    Aabb, OrbitCamera, clay_readability_render_settings, fit_camera_to_bounds,
    foundry::FoundryPreviewCache, render_mesh,
};
use shape_search::foundry::{
    FoundryCandidateMode, FoundryCandidateOutput, FoundryCandidateRequest,
};

use crate::foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
    FoundryJobSlot, FoundryPreviewImage,
    kit_view::built_in_kit_card_views,
    panels::{customize, directions, history, pack},
    run_foundry_job,
    state::DEFAULT_PREVIEW_PIXELS,
    trace::MAKE_JOB_TRACE_DIR,
    ui::{
        copy::WORKFLOW_STEPS,
        theme::apply_visual_foundry_theme,
        tokens::VisualFoundryTokens,
        widgets::{
            ActionSpec, BannerTone, ButtonTone, SectionHeaderSpec, StatusBannerSpec,
            StatusPillSpec, StatusTone, action_button, section_header, status_banner, status_pill,
        },
    },
};
mod catalog;
mod customize_ui;
mod direction_ideas;
mod effects;
mod family_studio_lite;
mod feature_gates;
mod history_panel;
mod home;
mod home_thumbnails;
mod job_coordinator;
mod make_actions;
mod make_copy;
mod make_layout;
mod make_state;
mod make_view;
mod material_looks;
mod object_plan_review;
mod pack_export;
mod preview;
mod product_contracts;
mod product_ui;
mod project_io;
mod screenshot;
mod shell;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
use catalog::*;
#[allow(unused_imports)]
use customize_ui::*;
#[allow(unused_imports)]
use direction_ideas::*;
#[allow(unused_imports)]
use effects::*;
#[allow(unused_imports)]
use family_studio_lite::*;
#[allow(unused_imports)]
use feature_gates::*;
#[allow(unused_imports)]
use history_panel::*;
#[allow(unused_imports)]
use home::*;
#[allow(unused_imports)]
use home_thumbnails::*;
#[allow(unused_imports)]
use job_coordinator::*;
#[allow(unused_imports)]
use make_actions::*;
#[allow(unused_imports)]
use make_copy::*;
#[allow(unused_imports)]
use make_layout::*;
#[allow(unused_imports)]
use make_state::*;
#[allow(unused_imports)]
use make_view::*;
#[allow(unused_imports)]
use material_looks::*;
#[allow(unused_imports)]
use object_plan_review::*;
#[allow(unused_imports)]
use pack_export::*;
#[allow(unused_imports)]
use preview::*;
#[allow(unused_imports)]
use product_ui::*;
#[allow(unused_imports)]
use project_io::*;
#[allow(unused_imports)]
use screenshot::*;
#[allow(unused_imports)]
use shell::*;

#[allow(unused_imports)]
pub(crate) use product_contracts::{
    core_make_action_specs_for_default_shell, default_app_launches_on_home,
    default_product_home_profile_count, developer_preview_product_home_profile_count,
    direction_mode_actions_for_panel, direction_variation_mode_actions_for_panel,
    installed_product_kit_count, product_visible_strings_for_default_shell,
    rendered_action_labels_for_default_shell,
};

/// Native Foundry workflow surface.
pub(crate) struct FoundryDesktopApp {
    state: FoundryAppState,
    tab: FoundryTab,
    drawer: Option<FoundryDrawer>,
    jobs: FoundryJobCoordinator,
    home_thumbnails: HomeThumbnailCoordinator,
    texture_cache: FoundryTextureCache,
    current_preview_orbit: CurrentPreviewOrbitState,
    home_profiles: Vec<ProductHomeProfile>,
    home_search_query: String,
    home_filter: HomeTemplateFilter,
    selected_home_profile_slug: Option<String>,
    recent_projects: Vec<PathBuf>,
    requested_start_window_mode: bool,
    make_trace_started_at: Instant,
    make_preparation_started_at: Option<Instant>,
    make_generation_started_at: Option<Instant>,
    material_looks: MakeMaterialLookState,
    object_plan_review_enabled: bool,
    family_studio_lite_enabled: bool,
    legacy_candidate_ui_enabled: bool,
    family_studio_lite: FamilyStudioLiteState,
    screenshot_scenario: Option<ScreenshotScenario>,
    screenshot_scenario_step: u8,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FoundryTab {
    Home,
    Make,
    History,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FoundryDrawer {
    Pack,
    Export,
    ObjectPlanReview,
    FamilyStudioLite,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ScreenshotScenario {
    ChooseGroupedPrimitives,
    ChooseBoxProvenance,
    ChooseFlatPanelProvenance,
    ChooseSpherePreset,
    ChooseSelectedPreview,
    BoxDirectMakeReady,
    BoxPropertyEdit,
    FlatPanelDirectMakeReady,
    FlatPanelPropertyEdit,
    SphereDirectMakeReady,
    SpherePropertyEdit,
    SphereKnobLikePreset,
    PanelKnobDirectMakeReady,
    OrbitAfterDragOrTool,
    ResetView,
    SphereExportDrawer,
    PackDrawer,
    ExportDrawer,
    ObjectPlanReviewDrawer,
    FamilyStudioLiteHiddenDefault,
    FamilyStudioLiteDrawer,
    FamilyStudioLiteTestResult,
    FamilyStudioLiteSaveDraft,
    FamilyStudioLitePersonalSaved,
}

impl ScreenshotScenario {
    const fn choose_selected_slug(self) -> Option<&'static str> {
        match self {
            Self::ChooseGroupedPrimitives | Self::ChooseBoxProvenance => {
                Some(BOX_PRIMITIVE_PROFILE_ID)
            }
            Self::ChooseFlatPanelProvenance => Some(FLAT_PANEL_PRIMITIVE_PROFILE_ID),
            Self::ChooseSpherePreset => Some(SPHERE_PRIMITIVE_PROFILE_ID),
            Self::ChooseSelectedPreview => Some(PANEL_KNOB_PROFILE_ID),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MakeCanvasMode {
    NoAsset,
    PreparingAsset,
    Ready,
    GeneratingWholeAssetIdeas,
    GeneratingFocusedPartIdeas,
    ReviewingIdeas,
    FocusedPart,
    PackDrawerOpen,
    ExportDrawerOpen,
    Error,
}

#[derive(Debug, Copy, Clone, PartialEq)]
struct MakeCanvasLayout {
    top_height: f32,
    tray_height: f32,
    tray_gap: f32,
    stage_width: f32,
    ideas_width: f32,
    inspector_width: f32,
    content_gap: f32,
    stacked_columns: bool,
    inline_ideas: bool,
    compact_ideas: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MakePreparationPhase {
    PreparingModel,
    RenderingPreview,
    Ready,
}

impl MakePreparationPhase {
    fn label(self) -> &'static str {
        match self {
            Self::PreparingModel => "Preparing model",
            Self::RenderingPreview => "Rendering preview",
            Self::Ready => "Ready",
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MakeCandidateTrayState {
    EmptyReady,
    GeneratingSkeletons,
    HasCandidates,
    NoCandidatesWithRecovery,
    ErrorWithRecovery,
}

fn current_preview_stage_status_message(
    has_preview: bool,
    has_output: bool,
    preview_is_stale: bool,
    rendering_preview: bool,
) -> Option<&'static str> {
    if has_preview {
        return preview_is_stale.then_some(PREVIEW_UPDATING_REASON);
    }
    if !has_output {
        return None;
    }
    Some(if rendering_preview {
        PREVIEW_UPDATING_REASON
    } else {
        PREVIEW_PREPARING_REASON
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MakeCanvasViewState {
    mode: MakeCanvasMode,
    asset_name: String,
    preparation_phase: MakePreparationPhase,
    preparation_timed_out: bool,
    preparation_fallback_visible: bool,
    idea_generation_timed_out: bool,
    idea_generation_fallback_visible: bool,
    preview_updating: bool,
    preview_update_required: bool,
    local_banner_title: String,
    local_banner_message: String,
    local_banner_tone: BannerTone,
    primary_title: String,
    primary_action_label: String,
    primary_action_enabled: bool,
    primary_action_disabled_reason: Option<String>,
    local_busy_label: Option<String>,
    local_busy_visible: bool,
    focused_part_label: Option<String>,
    focused_part_visible: bool,
    focused_part_actions_visible: bool,
    quick_template_preview_visible: bool,
    model_ready: bool,
    preview_ready: bool,
    candidate_tray_visible: bool,
    material_look_tray_visible: bool,
    candidate_tray_state: MakeCandidateTrayState,
    candidate_count: usize,
    focused_no_candidates_recovery_visible: bool,
    rejected_candidate_summary: Option<String>,
    selected_comparison_visible: bool,
    pack_drawer_visible: bool,
    export_drawer_visible: bool,
    object_plan_review_drawer_visible: bool,
    family_studio_lite_drawer_visible: bool,
    local_warning_message: Option<String>,
    local_error_message: Option<String>,
    next_action_hint: String,
    direct_primitive_workflow: bool,
    candidate_ui_enabled: bool,
    property_panel_title: &'static str,
    property_labels: Vec<&'static str>,
    simple_box_make_baseline: bool,
    lidded_box_baseline: bool,
    flat_panel_baseline: bool,
    hinged_panel_baseline: bool,
    handled_panel_baseline: bool,
    panel_knob_baseline: bool,
    adjust_heading_label: &'static str,
}

#[derive(Debug, Clone, Default)]
struct MakeMaterialLookState {
    tray_open: bool,
    evidence: Option<MakeMaterialLookEvidence>,
    selected_candidate_id: Option<String>,
    load_error: Option<String>,
    evidence_report_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObjectPlanReviewUiState {
    entry_visible: bool,
    drawer_visible: bool,
    batch_report_visible: bool,
    contact_sheet_visible: bool,
    review_labels: Vec<&'static str>,
    safety_labels: Vec<&'static str>,
    publish_action_visible: bool,
    catalog_mutation_allowed: bool,
    runtime_llm_action_visible: bool,
}

#[derive(Debug, Clone, Default)]
struct FamilyStudioLiteState {
    selected_capability_ids: BTreeSet<String>,
    test_result: Option<FamilyStudioLiteTestResult>,
    saved_visibility: Option<DirectKitVisibility>,
    save_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FamilyStudioLiteUiState {
    entry_visible: bool,
    drawer_visible: bool,
    starting_point_title: String,
    starting_point_summary: String,
    source_label: &'static str,
    supported: bool,
    disabled_reason: Option<String>,
    stays_same: Vec<String>,
    capability_cards: Vec<FamilyStudioLiteCapabilityCardView>,
    test_result: Option<FamilyStudioLiteTestResult>,
    saved_visibility: Option<DirectKitVisibility>,
    save_error: Option<String>,
    draft_save_enabled: bool,
    personal_save_enabled: bool,
    approved: bool,
    publish_allowed: bool,
    runtime_llm_action_visible: bool,
    generated_variation_copy_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FamilyStudioLiteCapabilityCardView {
    capability_id: String,
    display_name: String,
    description: String,
    selected: bool,
    status_label: &'static str,
    visible_test_required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FamilyStudioLiteTestResult {
    status: FamilyStudioLiteTestStatus,
    message: String,
    tested_capabilities: usize,
    human_review_required: bool,
    approved: bool,
    publish_allowed: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FamilyStudioLiteTestStatus {
    Passed,
    Warnings,
    Failed,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FamilyStudioLiteSource {
    Primitive(PrimitiveKind),
    PanelWithKnob,
    Unsupported,
}

impl FamilyStudioLiteSource {
    const fn supported(self) -> bool {
        matches!(self, Self::Primitive(_) | Self::PanelWithKnob)
    }

    const fn source_label(self) -> &'static str {
        match self {
            Self::Primitive(_) => "Current shape",
            Self::PanelWithKnob => "Current shape group",
            Self::Unsupported => "Not available",
        }
    }

    const fn source_ref(self) -> &'static str {
        match self {
            Self::Primitive(PrimitiveKind::BoxPrimitive) => "box_primitive",
            Self::Primitive(PrimitiveKind::FlatPanelPrimitive) => "flat_panel_primitive",
            Self::Primitive(PrimitiveKind::SpherePrimitive) => "sphere_primitive",
            Self::Primitive(PrimitiveKind::CylinderPrimitive) | Self::Unsupported => "unsupported",
            Self::PanelWithKnob => "panel_with_knob",
        }
    }

    const fn identity_summary(self) -> &'static str {
        match self {
            Self::Primitive(PrimitiveKind::BoxPrimitive) => "This stays a box-like primitive.",
            Self::Primitive(PrimitiveKind::FlatPanelPrimitive) => "This stays a flat panel.",
            Self::Primitive(PrimitiveKind::SpherePrimitive) => "This stays a round primitive.",
            Self::Primitive(PrimitiveKind::CylinderPrimitive) | Self::Unsupported => {
                "This starting point is not ready for reusable kits yet."
            }
            Self::PanelWithKnob => "This stays a panel with a bounded knob-like form.",
        }
    }

    const fn direct_kit_source_kind(self) -> DirectKitSourceKind {
        match self {
            Self::Primitive(_) => DirectKitSourceKind::Primitive,
            Self::PanelWithKnob => DirectKitSourceKind::Composition,
            Self::Unsupported => DirectKitSourceKind::Unsupported,
        }
    }

    const fn created_from(self) -> DirectKitCreatedFrom {
        match self {
            Self::Primitive(_) => DirectKitCreatedFrom::CurrentPrimitive,
            Self::PanelWithKnob => DirectKitCreatedFrom::CompositionDraft,
            Self::Unsupported => DirectKitCreatedFrom::InternalTool,
        }
    }

    fn capability_cards(self) -> Vec<KitCapabilityCard> {
        match self {
            Self::Primitive(primitive_kind) => {
                kit_capability_cards_for_primitive(primitive_kind, false)
            }
            Self::PanelWithKnob => kit_capability_cards_for_panel_with_knob(false),
            Self::Unsupported => Vec::new(),
        }
    }

    fn property_exposures(self) -> Vec<shape_foundry::DirectKitPropertyExposure> {
        match self {
            Self::Primitive(primitive_kind) => {
                direct_kit_property_exposures_for_primitive(primitive_kind)
            }
            Self::PanelWithKnob => {
                direct_kit_property_exposures_for_primitive(PrimitiveKind::FlatPanelPrimitive)
            }
            Self::Unsupported => Vec::new(),
        }
    }

    fn preset_refs(self) -> Vec<DirectKitPresetRef> {
        match self {
            Self::Primitive(PrimitiveKind::BoxPrimitive) => vec![DirectKitPresetRef {
                preset_id: "compact_box".to_owned(),
                display_name: "Compact Box".to_owned(),
                source: PresetSource::BuiltIn,
            }],
            Self::Primitive(PrimitiveKind::FlatPanelPrimitive) => vec![DirectKitPresetRef {
                preset_id: "wide_panel".to_owned(),
                display_name: "Wide Panel".to_owned(),
                source: PresetSource::BuiltIn,
            }],
            Self::Primitive(PrimitiveKind::SpherePrimitive) => vec![DirectKitPresetRef {
                preset_id: "round_sphere".to_owned(),
                display_name: "Round Sphere".to_owned(),
                source: PresetSource::BuiltIn,
            }],
            Self::PanelWithKnob => vec![
                DirectKitPresetRef {
                    preset_id: "wide_panel".to_owned(),
                    display_name: "Wide Panel".to_owned(),
                    source: PresetSource::BuiltIn,
                },
                DirectKitPresetRef {
                    preset_id: "knob_like_form".to_owned(),
                    display_name: "Knob-Like Form".to_owned(),
                    source: PresetSource::BuiltIn,
                },
            ],
            Self::Primitive(PrimitiveKind::CylinderPrimitive) | Self::Unsupported => Vec::new(),
        }
    }
}

impl MakeMaterialLookState {
    fn clear_for_asset(&mut self) {
        self.tray_open = false;
        self.evidence = None;
        self.selected_candidate_id = None;
        self.load_error = None;
    }
}

#[derive(Debug, Clone)]
struct MakeMaterialLookEvidence {
    candidates: Vec<MakeMaterialLookCandidate>,
    full_ready_blocker_codes: Vec<String>,
}

#[derive(Debug, Clone)]
struct MakeMaterialLookCandidate {
    candidate_id: String,
    display_name: String,
    material_override_ref: String,
    textured_preview_ref: String,
    surface_delta_ref: String,
    validation_ref: String,
    changed_material_slots: Vec<String>,
    rgba8: Vec<u8>,
    width: u32,
    height: u32,
    visible_surface_pixel_delta: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateEvidenceReportFile {
    schema_version: u32,
    profile_id: String,
    visual_foundry_surface_mode_enabled: bool,
    candidate_count: usize,
    all_candidates_valid: bool,
    full_ready_status: String,
    full_ready_blocker_codes: Vec<String>,
    candidates: Vec<SurfaceCandidateEvidenceReportRowFile>,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateEvidenceReportRowFile {
    candidate_id: String,
    display_name: String,
    material_override_ref: String,
    textured_preview_ref: String,
    surface_delta_ref: String,
    validation_ref: String,
    result_class: String,
    shape_delta_leak_detected: bool,
    visible_surface_pixel_delta: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateSetFile {
    schema_version: u32,
    profile_id: String,
    candidates: Vec<SurfaceCandidateSetRowFile>,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateSetRowFile {
    candidate_id: String,
    display_name: String,
    changed_material_slots: Vec<String>,
    material_override_ref: String,
    textured_preview_ref: String,
    surface_delta_ref: String,
    validation_ref: String,
    frozen_mesh_fingerprint: String,
    preserves_frozen_geometry: bool,
    full_ready_status: String,
    blocked_full_ready: bool,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateValidationFile {
    valid: bool,
    blocker_codes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct SurfaceCandidateDeltaFile {
    profile_id: String,
    candidate_id: String,
    shape_delta_leak_detected: bool,
    result_class: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum HomeTemplateFilter {
    All,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MakeProfileKind {
    BoxPrimitive,
    LiddedBox,
    FlatPanelPrimitive,
    SpherePrimitive,
    HingedPanel,
    HandledPanel,
    PanelWithKnob,
    Other,
}

impl MakeProfileKind {
    const fn direct_primitive_workflow(self) -> bool {
        matches!(
            self,
            Self::BoxPrimitive
                | Self::LiddedBox
                | Self::FlatPanelPrimitive
                | Self::SpherePrimitive
                | Self::HingedPanel
                | Self::HandledPanel
                | Self::PanelWithKnob
        )
    }

    const fn simple_clay_make_baseline(self) -> bool {
        matches!(
            self,
            Self::BoxPrimitive
                | Self::LiddedBox
                | Self::FlatPanelPrimitive
                | Self::SpherePrimitive
                | Self::HingedPanel
                | Self::HandledPanel
                | Self::PanelWithKnob
        )
    }

    const fn is_lidded_box(self) -> bool {
        matches!(self, Self::LiddedBox)
    }

    const fn is_flat_panel_primitive(self) -> bool {
        matches!(self, Self::FlatPanelPrimitive)
    }

    const fn is_sphere_primitive(self) -> bool {
        matches!(self, Self::SpherePrimitive)
    }

    const fn is_hinged_panel(self) -> bool {
        matches!(self, Self::HingedPanel)
    }

    const fn is_handled_panel(self) -> bool {
        matches!(self, Self::HandledPanel)
    }

    const fn is_panel_with_knob(self) -> bool {
        matches!(self, Self::PanelWithKnob)
    }

    const fn is_panel_like(self) -> bool {
        matches!(
            self,
            Self::FlatPanelPrimitive | Self::HingedPanel | Self::HandledPanel | Self::PanelWithKnob
        )
    }
}

const HOME_TEMPLATE_FILTERS: [HomeTemplateFilter; 0] = [];

const HOME_SUBTITLE: &str = "Choose a simple clay starting point for the Make loop.";
const BOX_PRIMITIVE_HOME_SUBTITLE: &str = "A simple clay box for testing the Make loop.";
const HOME_CONTROL_COPY: &str = "You can edit bounded properties.";
const NEED_PROJECT_REASON: &str = "Choose a starting point or open a project first.";
const NEED_SAVE_LOCATION_REASON: &str =
    "Use Save Project first to choose where this project is saved.";
const NEED_MODEL_REASON: &str = "Prepare the current model first.";
const NEED_HISTORY_REASON: &str = "No earlier project step is available.";
const NEED_DIRECTION_REASON: &str = "This direction is not ready to choose.";
const NEED_RESET_REASON: &str = "This control is already at its starting value.";
const NEED_PACK_MEMBER_REASON: &str = "Add at least one asset before exporting a pack.";
const NEED_LOCKED_CONTROLS_REASON: &str = "No locked controls are active.";
const ASSET_PREPARING_REASON: &str = "The asset is preparing. This usually takes a moment.";
const PREVIEW_UPDATING_REASON: &str = "Preview is updating...";
const PREVIEW_PREPARING_REASON: &str = "Preview is being prepared.";
const ACTIVE_IDEA_JOB_REASON: &str = "Finish the current idea search before changing this.";
const STALE_RESULT_WARNING: &str = "An older result was ignored because you changed the asset.";
const CANCELED_IDEA_SEARCH_WARNING: &str = "Canceled earlier idea search.";
const PREPARATION_TIMEOUT_MESSAGE: &str = "Still preparing. You can keep waiting or retry.";
const PREPARATION_TIMEOUT: Duration = Duration::from_secs(12);
const IDEA_GENERATION_TIMEOUT_MESSAGE: &str = "Still trying ideas.";
const IDEA_GENERATION_TIMEOUT: Duration = Duration::from_secs(10);
const CUSTOMIZE_PRIMARY_CONTROL_LIMIT: usize = 7;
const MAKE_CONTEXT_INITIAL_CONTROL_LIMIT: usize = 4;
const CONTROL_FILMSTRIP_LIMIT: usize = 5;
const CONTROL_HEADER_ACTIONS_WIDTH: f32 = 304.0;
const CONTROL_HEADER_STACK_BREAKPOINT: f32 = 520.0;
const MAX_CURRENT_PREVIEW_PIXELS: u32 = DEFAULT_PREVIEW_PIXELS;
const CURRENT_PREVIEW_MODEL_IMAGE_SCALE: f32 = 0.90;
const CURRENT_PREVIEW_ORBIT_DEGREES_PER_POINT: f32 = 0.35;
const COORDINATE_REFERENCE_GRID_STEPS: i32 = 8;
const COORDINATE_REFERENCE_GRID_EXTENT: f32 = 1.18;
const PREVIEW_CATALOG_ENV_VAR: &str = "SHAPE_LAB_PREVIEW_CATALOG";
const OBJECT_PLAN_REVIEW_ENV_VAR: &str = "SHAPE_LAB_OBJECT_PLAN_REVIEW";
const FAMILY_STUDIO_LITE_ENV_VAR: &str = "SHAPE_LAB_FAMILY_STUDIO_LITE";
const LEGACY_CANDIDATE_UI_ENV_VAR: &str = "SHAPE_LAB_LEGACY_CANDIDATE_UI";
const BOX_PRIMITIVE_PROFILE_ID: &str = "box-primitive";
const LIDDED_BOX_PROFILE_ID: &str = "lidded-box";
const FLAT_PANEL_PRIMITIVE_PROFILE_ID: &str = "flat-panel-primitive";
const SPHERE_PRIMITIVE_PROFILE_ID: &str = "sphere-primitive";
const HINGED_PANEL_PROFILE_ID: &str = "hinged-panel";
const HANDLED_PANEL_PROFILE_ID: &str = "handled-panel";
const PANEL_KNOB_PROFILE_ID: &str = "panel-with-knob";
const SURFACE_CANDIDATE_REPORT_RELATIVE_PATH: &str = "target/surface-candidate-evidence-v0/box-primitive/surface/variants/surface-candidate-report.json";
const SURFACE_CANDIDATE_SET_FILE: &str = "candidates.json";
const MATERIAL_LOOK_MISSING_MESSAGE: &str = "Material looks are not generated yet.";
const MATERIAL_LOOK_SECTION_TITLE: &str = "Material looks";
const MATERIAL_LOOK_SURFACE_ONLY_COPY: &str = "Surface only";
const MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY: &str = "Geometry unchanged";
const MATERIAL_LOOK_PREVIEW_ONLY_COPY: &str =
    "Material looks are preview-only in this build and will not affect export.";
const MATERIAL_LOOK_EXPORT_INCLUDED_COPY: &str =
    "Selected material look included in static surface package.";
const MATERIAL_LOOK_FULL_READY_BLOCKED_COPY: &str =
    "Full game-ready remains blocked until manual review and engine import proof.";
const SURFACE_PACKAGE_COMMAND_COPY: &str = "Run static surface package command";
const SURFACE_PACKAGE_COMMAND: &str =
    "Surface packages are not part of the Box Primitive baseline.";
const BOX_PRIMITIVE_EXPORT_TITLE: &str = "Export Box Primitive";
const BOX_PRIMITIVE_EXPORT_DETAIL: &str = "Exports the current clay box asset.";
const LIDDED_BOX_EXPORT_TITLE: &str = "Export Lidded Box";
const LIDDED_BOX_EXPORT_DETAIL: &str = "Exports the current clay lidded box asset.";
const FLAT_PANEL_EXPORT_TITLE: &str = "Export Flat Panel";
const FLAT_PANEL_EXPORT_DETAIL: &str = "Exports the current clay flat panel asset.";
const SPHERE_PRIMITIVE_EXPORT_TITLE: &str = "Export Sphere Primitive";
const SPHERE_PRIMITIVE_EXPORT_DETAIL: &str = "Exports the current clay sphere primitive asset.";
const HINGED_PANEL_EXPORT_TITLE: &str = "Export Hinged Panel";
const HINGED_PANEL_EXPORT_DETAIL: &str = "Exports the current clay hinged panel asset.";
const HANDLED_PANEL_EXPORT_TITLE: &str = "Export Handled Panel";
const HANDLED_PANEL_EXPORT_DETAIL: &str = "Exports the current clay handled panel asset.";
const PANEL_KNOB_EXPORT_TITLE: &str = "Export Panel with Knob";
const PANEL_KNOB_EXPORT_DETAIL: &str = "Exports the current clay panel with knob asset.";
const BOX_PRIMITIVE_EXPORT_LIMITATION: &str =
    "This is not a textured, rigged, animated, or game-ready package.";
const MATERIAL_LOOK_TITLES: [&str; 6] = [
    "Clean Lab White",
    "Worn Hazard Yellow",
    "Dark Industrial Metal",
    "Field Blue Utility",
    "Graphite Cargo",
    "Orange Warning Trim",
];
const ACTION_EXPORT: &str = "Export";
const ACTION_SAVE: &str = "Save";
const ACTION_UNDO: &str = "Undo";
const ACTION_PROJECT_MENU: &str = "Project";
const ACTION_OPEN_PROJECT: &str = "Open Project";
const ACTION_SAVE_PROJECT: &str = "Save Project";
const ACTION_SAVE_PROJECT_AS: &str = "Save Project As";
const ACTION_START_ANOTHER_ASSET: &str = "Start Another Asset";
const ACTION_PROJECT_HISTORY: &str = "History";
const ACTION_SAVE_AS: &str = "Save As";
const ACTION_LOAD: &str = "Load";
const ACTION_SWITCH_TO_REVISION: &str = "Switch to revision";
const ACTION_BRANCH_FROM_REVISION: &str = "Branch from revision";
const ACTION_START: &str = "Start";
const ACTION_TRY_WHOLE_ASSET_IDEAS: &str = "Try ideas";
const ACTION_TRY_BOX_IDEAS: &str = "Try box ideas";
const ACTION_TRY_LIDDED_BOX_IDEAS: &str = "Try lidded box ideas";
const ACTION_TRY_PANEL_IDEAS: &str = "Try panel ideas";
const ACTION_TRY_HINGED_PANEL_IDEAS: &str = "Try hinged panel ideas";
const ACTION_TRY_HANDLED_PANEL_IDEAS: &str = "Try handled panel ideas";
const ACTION_GENERATING_IDEAS: &str = "Trying ideas...";
const ACTION_TRY_WHOLE_ASSET_RECOVERY: &str = "Try whole-asset ideas";
const ACTION_TRY_MORE_IDEAS: &str = "Try more ideas";
const ACTION_TRY_MATERIAL_LOOKS: &str = "Try material looks";
const ACTION_TRY_AGAIN: &str = "Try again";
const ACTION_CHOOSE_TEMPLATE: &str = "Choose a starting point";
const ACTION_CHOOSE_ANOTHER_TEMPLATE: &str = "Choose another starting point";
const ACTION_CANCEL: &str = "Cancel";
const ACTION_KEEP_WAITING: &str = "Keep waiting";
const ACTION_SWITCH: &str = "Switch";
const ACTION_BRANCH: &str = "Branch";
const ACTION_ADD_TO_PACK: &str = "Add to Pack";
const ACTION_OPEN_PACK: &str = "Open Pack";
const ACTION_OPEN_EXPORT: &str = "Open Export";
const ACTION_EXPORT_CURRENT_ASSET: &str = "Export Current Asset";
const ACTION_EXPORT_CURRENT_PRIMITIVE: &str = "Export current primitive";
const ACTION_ADD_CURRENT_ASSET: &str = "Add Current Asset";
const ACTION_EXPORT_PACK: &str = "Export Pack";
const ACTION_CLOSE_DRAWER: &str = "Close drawer";
const ACTION_REVIEW_OBJECT_PLANS: &str = "Review ObjectPlans";
const ACTION_CREATE_REUSABLE_KIT: &str = "Create reusable kit";
const ACTION_TEST_KIT: &str = "Test kit";
const ACTION_SAVE_DRAFT_KIT: &str = "Save Draft";
const ACTION_USE_PERSONALLY: &str = "Use Personally";
const ACTION_SELECT: &str = "Compare";
const ACTION_CHOOSE_DIRECTION: &str = "Use this idea";
const ACTION_USE_THIS_BOX: &str = "Use this box";
const ACTION_USE_THIS_PANEL: &str = "Use this panel";
const ACTION_REJECT: &str = "Reject";
const ACTION_RESET: &str = "Reset";
const ACTION_UNLOCK: &str = "Unlock";
const ACTION_UNLOCK_CONTROLS: &str = "Unlock controls";
const ACTION_LOCK: &str = "Lock";
const ACTION_FOCUS: &str = "Focus";
const ACTION_APPLY: &str = "Use option";
const ACTION_CLEAR_FOCUS: &str = "Clear focus";
const ACTION_CHOOSE_ANOTHER_PART: &str = "Choose another part";
const ACTION_RETRY_PREPARATION: &str = "Retry preparation";
const ACTION_UPDATE_PREVIEW: &str = "Update preview";
const ACTION_ADJUST_BOX: &str = "Adjust box";
const ACTION_ADJUST_LID_SEAM: &str = "Adjust lid seam";
const ACTION_ADJUST_PANEL: &str = "Adjust panel";
const ACTION_ADJUST_HINGE_EDGE: &str = "Adjust hinge edge";
const ACTION_ADJUST_HANDLE_KNOB: &str = "Adjust handle";
const ACTION_ADJUST_DIMENSIONS: &str = "Adjust dimensions";
const ACTION_EDIT_BOX_PRIMITIVE: &str = "Edit Box Primitive";
const ACTION_EDIT_FLAT_PANEL: &str = "Edit Flat Panel";
const ACTION_EDIT_SPHERE_PRIMITIVE: &str = "Edit Sphere Primitive";
const ACTION_KNOB_LIKE_FORM: &str = "Knob-like form";
const ACTION_EDIT_LIDDED_BOX: &str = "Edit Lidded Box";
const ACTION_EDIT_HINGED_PANEL: &str = "Edit Hinged Panel";
const ACTION_EDIT_HANDLED_PANEL: &str = "Edit Handled Panel";
const ACTION_EDIT_PANEL_KNOB: &str = "Edit Panel with Knob";
const VIEW_ORBIT_LABEL: &str = "Orbit view";
const VIEW_RESET_LABEL: &str = "Reset view";
const VIEW_AXIS_LABEL: &str = "Axis view";
const RENDERED_ACTION_LABELS: [&str; 49] = [
    ACTION_EXPORT,
    ACTION_SAVE,
    ACTION_UNDO,
    ACTION_PROJECT_MENU,
    ACTION_OPEN_PROJECT,
    ACTION_SAVE_PROJECT,
    ACTION_SAVE_PROJECT_AS,
    ACTION_START_ANOTHER_ASSET,
    ACTION_PROJECT_HISTORY,
    ACTION_SAVE_AS,
    ACTION_LOAD,
    ACTION_SWITCH_TO_REVISION,
    ACTION_BRANCH_FROM_REVISION,
    ACTION_START,
    ACTION_TRY_AGAIN,
    ACTION_CHOOSE_TEMPLATE,
    ACTION_CHOOSE_ANOTHER_TEMPLATE,
    ACTION_CANCEL,
    ACTION_KEEP_WAITING,
    ACTION_SWITCH,
    ACTION_BRANCH,
    ACTION_ADD_TO_PACK,
    ACTION_OPEN_PACK,
    ACTION_OPEN_EXPORT,
    ACTION_EXPORT_CURRENT_ASSET,
    ACTION_EXPORT_CURRENT_PRIMITIVE,
    ACTION_ADD_CURRENT_ASSET,
    ACTION_EXPORT_PACK,
    ACTION_CLOSE_DRAWER,
    ACTION_CREATE_REUSABLE_KIT,
    ACTION_TEST_KIT,
    ACTION_SAVE_DRAFT_KIT,
    ACTION_USE_PERSONALLY,
    ACTION_RESET,
    ACTION_UNLOCK,
    ACTION_UNLOCK_CONTROLS,
    ACTION_RETRY_PREPARATION,
    ACTION_UPDATE_PREVIEW,
    ACTION_ADJUST_DIMENSIONS,
    ACTION_EDIT_BOX_PRIMITIVE,
    ACTION_EDIT_FLAT_PANEL,
    ACTION_EDIT_SPHERE_PRIMITIVE,
    ACTION_KNOB_LIKE_FORM,
    ACTION_EDIT_LIDDED_BOX,
    ACTION_EDIT_HINGED_PANEL,
    ACTION_EDIT_PANEL_KNOB,
    VIEW_ORBIT_LABEL,
    VIEW_RESET_LABEL,
    VIEW_AXIS_LABEL,
];
