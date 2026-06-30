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
use shape_core::Aabb;
use shape_foundry::{
    CatalogContentRef, FoundryAssetDocument, FoundryBuildStamp, FoundryCatalogError,
    FoundryCatalogResolver, FoundryCommand, FoundryCompilationOutput, VariationIntent,
    built_in_surface_capability_for_profile, compile_foundry_document,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, curated_fixture_catalogs_with_labels, headless_fixture_catalogs,
};
use shape_mesh::TriangleMesh;
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryProject, FoundryProjectFile, ensure_foundry_project_path,
};
use shape_render::{
    OrbitCamera, clay_readability_render_settings, fit_camera_to_bounds,
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

/// Native Foundry workflow surface.
pub(crate) struct FoundryDesktopApp {
    state: FoundryAppState,
    tab: FoundryTab,
    drawer: Option<FoundryDrawer>,
    jobs: FoundryJobCoordinator,
    home_thumbnails: HomeThumbnailCoordinator,
    texture_cache: FoundryTextureCache,
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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum ScreenshotScenario {
    MakeInitialBox,
    GeneratingBoxIdeas,
    GeneratedBoxIdeas,
    SelectedComparison,
    AdjustedBoxControl,
    PackDrawer,
    ExportDrawer,
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
    candidate_search_finished_empty: bool,
    focused_no_candidates_recovery_visible: bool,
    rejected_candidate_summary: Option<String>,
    selected_candidate_present: bool,
    selected_comparison_visible: bool,
    pack_drawer_visible: bool,
    export_drawer_visible: bool,
    local_warning_message: Option<String>,
    local_error_message: Option<String>,
    next_action_hint: String,
    box_primitive_baseline: bool,
    try_ideas_action_label: &'static str,
    use_candidate_action_label: &'static str,
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

const HOME_TEMPLATE_FILTERS: [HomeTemplateFilter; 0] = [];

const HOME_SUBTITLE: &str = "A simple clay box for testing the Make loop.";
const HOME_CONTROL_COPY: &str = "You can vary proportions and edge softness.";
const NEED_PROJECT_REASON: &str = "Choose Box Primitive or open a project first.";
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
const PREVIEW_CATALOG_ENV_VAR: &str = "SHAPE_LAB_PREVIEW_CATALOG";
const BOX_PRIMITIVE_PROFILE_ID: &str = "box-primitive";
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
const ACTION_GENERATING_IDEAS: &str = "Trying ideas...";
const ACTION_TRY_WHOLE_ASSET_RECOVERY: &str = "Try whole-asset ideas";
const ACTION_TRY_MORE_IDEAS: &str = "Try more ideas";
const ACTION_TRY_MATERIAL_LOOKS: &str = "Try material looks";
const ACTION_TRY_AGAIN: &str = "Try again";
const ACTION_CHOOSE_TEMPLATE: &str = "Choose Box Primitive";
const ACTION_CHOOSE_ANOTHER_TEMPLATE: &str = "Choose another starting point";
const ACTION_CANCEL: &str = "Cancel";
const ACTION_KEEP_WAITING: &str = "Keep waiting";
const ACTION_SWITCH: &str = "Switch";
const ACTION_BRANCH: &str = "Branch";
const ACTION_ADD_TO_PACK: &str = "Add to Pack";
const ACTION_OPEN_PACK: &str = "Open Pack";
const ACTION_OPEN_EXPORT: &str = "Open Export";
const ACTION_EXPORT_CURRENT_ASSET: &str = "Export Current Asset";
const ACTION_ADD_CURRENT_ASSET: &str = "Add Current Asset";
const ACTION_EXPORT_PACK: &str = "Export Pack";
const ACTION_CLOSE_DRAWER: &str = "Close drawer";
const ACTION_SELECT: &str = "Compare";
const ACTION_CHOOSE_DIRECTION: &str = "Use this idea";
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
const RENDERED_ACTION_LABELS: [&str; 39] = [
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
    ACTION_TRY_BOX_IDEAS,
    ACTION_GENERATING_IDEAS,
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
    ACTION_ADD_CURRENT_ASSET,
    ACTION_EXPORT_PACK,
    ACTION_CLOSE_DRAWER,
    ACTION_SELECT,
    ACTION_CHOOSE_DIRECTION,
    ACTION_REJECT,
    ACTION_RESET,
    ACTION_UNLOCK,
    ACTION_UNLOCK_CONTROLS,
    ACTION_RETRY_PREPARATION,
    ACTION_UPDATE_PREVIEW,
    ACTION_ADJUST_BOX,
];

impl Default for FoundryDesktopApp {
    fn default() -> Self {
        let developer_preview_catalog = developer_preview_catalog_enabled();
        let home_profiles = product_home_profiles(developer_preview_catalog);
        let selected_home_profile_slug = default_home_profile_slug(&home_profiles);
        Self {
            state: FoundryAppState::default(),
            tab: FoundryTab::Home,
            drawer: None,
            jobs: FoundryJobCoordinator::default(),
            home_thumbnails: HomeThumbnailCoordinator::default(),
            texture_cache: FoundryTextureCache::default(),
            home_profiles,
            home_search_query: String::new(),
            home_filter: HomeTemplateFilter::All,
            selected_home_profile_slug,
            recent_projects: Vec::new(),
            requested_start_window_mode: false,
            make_trace_started_at: Instant::now(),
            make_preparation_started_at: None,
            make_generation_started_at: None,
            material_looks: MakeMaterialLookState::default(),
            screenshot_scenario: read_screenshot_scenario(),
            screenshot_scenario_step: 0,
        }
    }
}

impl FoundryDesktopApp {
    /// Draw the Foundry workflow surface.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if !self.requested_start_window_mode {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            self.requested_start_window_mode = true;
        }
        apply_visual_foundry_theme(&ctx);
        self.refresh_make_trace_clock();
        self.poll_jobs(&ctx);
        self.poll_home_thumbnail_jobs(&ctx);
        self.refresh_make_preparation_timer();
        self.refresh_make_generation_timer();

        let tokens = VisualFoundryTokens::dark();
        let colors = tokens.colors;
        let mut commands = self.apply_screenshot_scenario(&ctx);
        egui::Panel::top("foundry_app_bar")
            .default_size(tokens.sizing.top_bar_height)
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .fill(colors.top_bar)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        commands.extend(self.show_app_bar(ui));
                    });
            });
        egui::Panel::top("foundry_workflow_tabs")
            .default_size(52.0)
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .fill(colors.center_bg)
                    .inner_margin(egui::Margin::symmetric(16, 8))
                    .show(ui, |ui| {
                        self.show_workflow_tabs(ui);
                    });
            });
        egui::Panel::bottom("foundry_status")
            .default_size(tokens.sizing.status_bar_height)
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .fill(colors.top_bar)
                    .inner_margin(egui::Margin::symmetric(16, 6))
                    .show(ui, |ui| self.show_status_strip(ui));
            });
        if let Some(drawer) = self.drawer {
            egui::Panel::right("foundry_action_drawer")
                .resizable(false)
                .default_size(430.0)
                .show_inside(ui, |ui| {
                    egui::Frame::new()
                        .fill(colors.panel)
                        .inner_margin(egui::Margin::symmetric(16, 14))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let title = match drawer {
                                    FoundryDrawer::Pack => "Pack",
                                    FoundryDrawer::Export => "Export",
                                };
                                ui.label(RichText::new(title).size(18.0).strong());
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if action_button(
                                            ui,
                                            &ActionSpec::enabled(
                                                ACTION_CLOSE_DRAWER,
                                                ButtonTone::Secondary,
                                            ),
                                        )
                                        .clicked()
                                        {
                                            self.drawer = None;
                                        }
                                    },
                                );
                            });
                            ui.add_space(10.0);
                            match drawer {
                                FoundryDrawer::Pack => commands.extend(self.show_pack_drawer(ui)),
                                FoundryDrawer::Export => {
                                    commands.extend(self.show_export_drawer(ui));
                                }
                            }
                        });
                });
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.center_bg)
                    .inner_margin(egui::Margin::symmetric(22, 18)),
            )
            .show_inside(ui, |ui| match self.tab {
                FoundryTab::Home => self.show_home(ui),
                FoundryTab::Make => commands.extend(self.show_make(ui)),
                FoundryTab::History => commands.extend(self.show_history(ui)),
            });

        self.apply_commands(commands, &ctx);
        self.refresh_make_preparation_timer();
        if !self.state.active_jobs.is_empty() || self.home_thumbnails.has_active_jobs() {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        let _ = frame;
    }

    fn show_app_bar(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.horizontal(|ui| {
            let view_state = self.make_canvas_view_state();
            let has_document = self.state.document.is_some();
            let build_dependent_actions_enabled =
                make_canvas_build_dependent_actions_enabled(&view_state);
            ui.label(RichText::new("Shape Lab").size(16.0).strong());
            ui.separator();
            ui.label(
                RichText::new(self.current_project_title())
                    .color(VisualFoundryTokens::dark().colors.text_muted),
            );
            ui.add_space(8.0);
            let (save_label, save_tone) = self.save_state_pill();
            let _ = status_pill(ui, StatusPillSpec::new(save_label, save_tone));
            ui.add_space(8.0);
            commands.extend(self.show_project_menu(ui));

            let can_save = has_document && self.state.project_path.is_some();
            let save_reason = if has_document {
                NEED_SAVE_LOCATION_REASON
            } else {
                NEED_PROJECT_REASON
            };
            let can_undo = self
                .state
                .project_file
                .as_ref()
                .is_some_and(|project| project.project.can_undo());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(
                    ui,
                    &action_spec(
                        build_dependent_actions_enabled,
                        ACTION_EXPORT,
                        ButtonTone::Primary,
                        make_canvas_build_dependent_disabled_reason(&view_state),
                    ),
                )
                .clicked()
                {
                    self.drawer = Some(FoundryDrawer::Export);
                }
                if action_button(
                    ui,
                    &action_spec(
                        build_dependent_actions_enabled,
                        ACTION_ADD_TO_PACK,
                        ButtonTone::Secondary,
                        make_canvas_build_dependent_disabled_reason(&view_state),
                    ),
                )
                .clicked()
                    && let Some(command) = self.add_current_to_pack_command()
                {
                    commands.push(command);
                    self.drawer = Some(FoundryDrawer::Pack);
                }
                if action_button(
                    ui,
                    &action_spec(can_save, ACTION_SAVE, ButtonTone::Secondary, save_reason),
                )
                .clicked()
                {
                    commands.push(FoundryAppCommand::Save);
                }
                if action_button(
                    ui,
                    &action_spec(
                        can_undo,
                        ACTION_UNDO,
                        ButtonTone::Quiet,
                        NEED_HISTORY_REASON,
                    ),
                )
                .clicked()
                {
                    commands.push(history::undo_command());
                }
            });
        });
        commands
    }

    fn show_project_menu(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let has_document = self.state.document.is_some();
        ui.menu_button(ACTION_PROJECT_MENU, |ui| {
            ui.set_min_width(230.0);
            if ui.button(ACTION_OPEN_PROJECT).clicked()
                && let Some(path) = open_foundry_project_file()
            {
                commands.push(FoundryAppCommand::Load(path));
                ui.close();
            }

            let save_response =
                ui.add_enabled(has_document, egui::Button::new(ACTION_SAVE_PROJECT));
            let save_response = if has_document {
                save_response
            } else {
                save_response.on_disabled_hover_text(NEED_PROJECT_REASON)
            };
            if save_response.clicked() {
                if self.state.project_path.is_some() {
                    commands.push(FoundryAppCommand::Save);
                } else if let Some(path) = save_foundry_project_file() {
                    commands.push(FoundryAppCommand::SaveAs(path));
                }
                ui.close();
            }

            let save_as_response =
                ui.add_enabled(has_document, egui::Button::new(ACTION_SAVE_PROJECT_AS));
            let save_as_response = if has_document {
                save_as_response
            } else {
                save_as_response.on_disabled_hover_text(NEED_PROJECT_REASON)
            };
            if save_as_response.clicked()
                && let Some(path) = save_foundry_project_file()
            {
                commands.push(FoundryAppCommand::SaveAs(path));
                ui.close();
            }

            ui.separator();
            if ui.button(ACTION_START_ANOTHER_ASSET).clicked() {
                self.tab = FoundryTab::Home;
                self.drawer = None;
                ui.close();
            }
            if ui.button(ACTION_PROJECT_HISTORY).clicked() {
                self.tab = FoundryTab::History;
                self.drawer = None;
                ui.close();
            }

            if !self.recent_projects.is_empty() {
                ui.separator();
                ui.label(
                    RichText::new("Recent Projects")
                        .color(VisualFoundryTokens::dark().colors.text_muted)
                        .small(),
                );
                for path in self.recent_projects.iter().take(6) {
                    let title = project_file_title(path);
                    if ui.button(title).clicked() {
                        commands.push(FoundryAppCommand::Load(path.clone()));
                        ui.close();
                    }
                }
            }
        });
        commands
    }

    fn show_workflow_tabs(&mut self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Visual Foundry")
                    .color(colors.accent_hover)
                    .small()
                    .strong(),
            );
            ui.add_space(14.0);
            for step in WORKFLOW_STEPS {
                let tab = tab_for_workflow_step(step.index);
                let selected = self.tab == tab;
                if workflow_tab_button(ui, step.index, step.label, step.detail, selected).clicked()
                {
                    self.tab = tab;
                }
            }
        });
    }

    fn show_status_strip(&self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal(|ui| {
            ui.label(RichText::new(self.status_summary()).color(colors.text));
            ui.separator();
            ui.label(RichText::new(self.model_status()).color(colors.text_muted));
            ui.separator();
            ui.label(RichText::new(self.preview_status()).color(colors.text_muted));
            ui.separator();
            ui.label(
                RichText::new(format!("Pack: {} assets", self.state.pack.members.len()))
                    .color(colors.text_muted),
            );
            if self.state.read_only {
                ui.separator();
                ui.label(RichText::new("Read-only recovery").color(colors.warning));
            }
            if let Some(status) = &self.state.status {
                ui.separator();
                ui.label(RichText::new(product_safe_status(status)).color(colors.text_subtle));
            }
        });
    }

    fn current_project_title(&self) -> String {
        if let Some(path) = &self.state.project_path {
            return project_file_title(path);
        }

        self.state
            .document
            .as_ref()
            .map(|document| asset_title_from_id(&document.document_id.0).to_owned())
            .unwrap_or_else(|| "Start with Box Primitive".to_owned())
    }

    fn active_make_part_group(&self) -> Option<directions::DirectionPartGroup> {
        let document = self.state.document.as_ref()?;
        let active_group_id = document
            .variation_state
            .intent
            .scope
            .semantic_part_group_id()?;
        directions::direction_part_groups_for_document(document)
            .into_iter()
            .find(|group| group.group_id == active_group_id)
    }

    fn make_canvas_view_state(&self) -> MakeCanvasViewState {
        let active_group = self.active_make_part_group();
        let selected_candidate = self
            .state
            .selected_candidate
            .as_ref()
            .and_then(|selected| {
                self.state
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == selected)
            })
            .or_else(|| self.state.candidates.first());
        let asset_name = self.current_project_title();
        let box_primitive_baseline = self.active_profile_is_box_primitive();
        let try_ideas_action_label = if box_primitive_baseline {
            ACTION_TRY_BOX_IDEAS
        } else {
            ACTION_TRY_WHOLE_ASSET_IDEAS
        };
        let use_candidate_action_label = ACTION_CHOOSE_DIRECTION;
        let adjust_heading_label = if box_primitive_baseline {
            ACTION_ADJUST_BOX
        } else {
            "Adjust"
        };
        let active_candidate_job = self.state.active_jobs.values().any(|request| {
            matches!(
                request,
                FoundryJobRequest::GenerateCandidates { .. }
                    | FoundryJobRequest::RenderCandidatePreviews { .. }
            )
        });
        let candidate_previews_pending = candidate_previews_are_pending(&self.state.candidates);
        let generating = active_candidate_job || candidate_previews_pending;
        let compiling_or_editing = self.state.active_jobs.values().any(|request| {
            matches!(
                request.slot(),
                FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
            )
        });
        let preview_rendering = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == FoundryJobSlot::RenderPreview);
        let current_asset_job_active = compiling_or_editing || preview_rendering;
        let model_ready = self.state.current_output.is_some() && !compiling_or_editing;
        let quick_template_preview_visible =
            self.state.document.is_some() && self.state.current_output.is_none();
        let preview_image_ready = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| !preview.rgba8.is_empty());
        let preview_matches_current_build = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| preview.build == self.state.current_build);
        let stale_preview = preview_image_ready && !preview_matches_current_build;
        let preview_ready = model_ready
            && preview_image_ready
            && preview_matches_current_build
            && !preview_rendering;
        let preview_updating = model_ready && (preview_rendering || stale_preview);
        let preview_update_required = model_ready && !preview_ready && !preview_rendering;
        let preparation_phase = if !model_ready {
            MakePreparationPhase::PreparingModel
        } else if !preview_ready {
            MakePreparationPhase::RenderingPreview
        } else {
            MakePreparationPhase::Ready
        };
        let focused_part_label = active_group.as_ref().map(|group| group.label.clone());
        let focused_part_visible = focused_part_label.is_some();
        let focused_part_actions_visible = focused_part_visible && model_ready && preview_ready;
        let preparing = self.state.document.is_some()
            && (!model_ready || !preview_ready || current_asset_job_active);
        let preparation_timed_out = preparing
            && self
                .make_preparation_started_at
                .is_some_and(|started| started.elapsed() >= PREPARATION_TIMEOUT);
        let preparation_fallback_visible = preparation_timed_out;
        let idea_generation_timed_out = generating
            && self
                .make_generation_started_at
                .is_some_and(|started| started.elapsed() >= IDEA_GENERATION_TIMEOUT);
        let idea_generation_fallback_visible = idea_generation_timed_out;
        let local_warning_message = self.make_canvas_local_warning();
        let local_error_message = self.make_canvas_local_error();
        let candidate_search_finished_empty = self.state.candidate_output.is_some()
            && self.state.candidates.is_empty()
            && !generating;
        let mode = if self.drawer == Some(FoundryDrawer::Pack) {
            MakeCanvasMode::PackDrawerOpen
        } else if self.drawer == Some(FoundryDrawer::Export) {
            MakeCanvasMode::ExportDrawerOpen
        } else if local_error_message.is_some() {
            MakeCanvasMode::Error
        } else if self.state.document.is_none() {
            MakeCanvasMode::NoAsset
        } else if preparing {
            MakeCanvasMode::PreparingAsset
        } else if !self.state.candidates.is_empty() {
            MakeCanvasMode::ReviewingIdeas
        } else if generating && focused_part_label.is_some() {
            MakeCanvasMode::GeneratingFocusedPartIdeas
        } else if generating {
            MakeCanvasMode::GeneratingWholeAssetIdeas
        } else if focused_part_label.is_some() {
            MakeCanvasMode::FocusedPart
        } else {
            MakeCanvasMode::Ready
        };
        let local_busy_label = match mode {
            MakeCanvasMode::PreparingAsset => Some(format!("Preparing {}...", asset_name)),
            MakeCanvasMode::GeneratingFocusedPartIdeas => focused_part_label.as_ref().map(|part| {
                format!(
                    "Trying {} ideas from the current {}...",
                    singular_part_copy(part).to_ascii_lowercase(),
                    make_busy_asset_noun(&asset_name)
                )
            }),
            MakeCanvasMode::GeneratingWholeAssetIdeas => Some(format!(
                "Trying ideas from the current {}...",
                make_busy_asset_noun(&asset_name)
            )),
            MakeCanvasMode::ReviewingIdeas if candidate_previews_pending => {
                Some("Rendering previews...".to_owned())
            }
            _ => None,
        };
        let local_busy_visible = local_busy_label.is_some();
        let candidate_tray_state = if local_error_message.is_some() {
            MakeCandidateTrayState::ErrorWithRecovery
        } else if !self.state.candidates.is_empty() {
            MakeCandidateTrayState::HasCandidates
        } else if generating {
            MakeCandidateTrayState::GeneratingSkeletons
        } else if candidate_search_finished_empty {
            MakeCandidateTrayState::NoCandidatesWithRecovery
        } else {
            MakeCandidateTrayState::EmptyReady
        };
        let focused_no_candidates_recovery_visible = candidate_tray_state
            == MakeCandidateTrayState::NoCandidatesWithRecovery
            && focused_part_label.is_some();
        let (local_banner_title, local_banner_message, local_banner_tone) =
            make_canvas_local_banner(MakeCanvasBannerContext {
                mode: &mode,
                asset_name: &asset_name,
                preparation_phase,
                preparation_timed_out,
                idea_generation_timed_out,
                preview_updating,
                candidate_previews_pending,
                local_busy_label: &local_busy_label,
                active_group: active_group.as_ref(),
                candidate_output: self.state.candidate_output.as_deref(),
                local_warning_message: local_warning_message.as_deref(),
                local_error_message: local_error_message.as_deref(),
                box_primitive_baseline,
            });
        let primary_title = match (&mode, focused_part_label.as_deref()) {
            (MakeCanvasMode::NoAsset, _) => "Choose an asset".to_owned(),
            (MakeCanvasMode::PreparingAsset, _) => {
                format!("Preparing {}", asset_name)
            }
            (_, Some(label)) => label.to_owned(),
            _ => asset_name.clone(),
        };
        let primary_action_label = match (&mode, focused_part_label.as_ref()) {
            (MakeCanvasMode::GeneratingWholeAssetIdeas, _)
            | (MakeCanvasMode::GeneratingFocusedPartIdeas, _) => ACTION_GENERATING_IDEAS.to_owned(),
            (MakeCanvasMode::NoAsset, _) => ACTION_CHOOSE_TEMPLATE.to_owned(),
            (MakeCanvasMode::ReviewingIdeas, _) => use_candidate_action_label.to_owned(),
            _ if focused_no_candidates_recovery_visible => {
                ACTION_TRY_WHOLE_ASSET_RECOVERY.to_owned()
            }
            (_, Some(label)) => format!(
                "Try {} ideas",
                singular_part_copy(label).to_ascii_lowercase()
            ),
            _ => try_ideas_action_label.to_owned(),
        };
        let primary_action_enabled = match mode {
            MakeCanvasMode::NoAsset => true,
            MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart => {
                model_ready && preview_ready && !generating
            }
            MakeCanvasMode::ReviewingIdeas => {
                model_ready
                    && preview_ready
                    && !generating
                    && selected_candidate.is_some_and(|candidate| candidate.selectable)
            }
            _ => false,
        };
        let primary_action_disabled_reason = (!primary_action_enabled).then(|| {
            if preparing {
                if preview_updating {
                    PREVIEW_UPDATING_REASON.to_owned()
                } else {
                    ASSET_PREPARING_REASON.to_owned()
                }
            } else if generating {
                ACTIVE_IDEA_JOB_REASON.to_owned()
            } else if mode == MakeCanvasMode::ReviewingIdeas {
                NEED_DIRECTION_REASON.to_owned()
            } else if let Some(message) = &local_error_message {
                message.clone()
            } else {
                NEED_PROJECT_REASON.to_owned()
            }
        });
        let candidate_tray_visible = self.state.document.is_some();
        let material_look_tray_visible = self.material_looks.tray_open && !box_primitive_baseline;
        let rejected_candidate_summary = self.make_canvas_rejected_candidate_summary();
        let selected_comparison_visible = selected_candidate.is_some_and(|candidate| {
            preview_ready
                && !candidate.rgba8.is_empty()
                && self
                    .state
                    .current_preview
                    .as_ref()
                    .is_some_and(|preview| !preview.rgba8.is_empty())
        });
        let mut next_action_hint = make_canvas_next_action_hint(
            &mode,
            focused_part_label.as_deref(),
            selected_comparison_visible,
            box_primitive_baseline,
        );
        if preparation_fallback_visible {
            next_action_hint = PREPARATION_TIMEOUT_MESSAGE.to_owned();
        } else if idea_generation_fallback_visible {
            next_action_hint = IDEA_GENERATION_TIMEOUT_MESSAGE.to_owned();
        } else if preview_update_required {
            next_action_hint = "Update preview to keep making changes.".to_owned();
        }

        MakeCanvasViewState {
            asset_name,
            mode,
            preparation_phase,
            preparation_timed_out,
            preparation_fallback_visible,
            idea_generation_timed_out,
            idea_generation_fallback_visible,
            preview_updating,
            preview_update_required,
            local_banner_title,
            local_banner_message,
            local_banner_tone,
            primary_title,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            local_busy_label,
            local_busy_visible,
            focused_part_label,
            focused_part_visible,
            focused_part_actions_visible,
            quick_template_preview_visible,
            model_ready,
            preview_ready,
            candidate_tray_visible,
            material_look_tray_visible,
            candidate_tray_state,
            candidate_count: self.state.candidates.len(),
            candidate_search_finished_empty,
            focused_no_candidates_recovery_visible,
            rejected_candidate_summary,
            selected_candidate_present: self.state.selected_candidate.is_some(),
            selected_comparison_visible,
            pack_drawer_visible: self.drawer == Some(FoundryDrawer::Pack),
            export_drawer_visible: self.drawer == Some(FoundryDrawer::Export),
            local_warning_message,
            local_error_message,
            next_action_hint,
            box_primitive_baseline,
            try_ideas_action_label,
            use_candidate_action_label,
            adjust_heading_label,
        }
    }

    fn make_canvas_local_warning(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.starts_with("Ignored a background result") {
            Some(STALE_RESULT_WARNING.to_owned())
        } else if status == CANCELED_IDEA_SEARCH_WARNING {
            Some(CANCELED_IDEA_SEARCH_WARNING.to_owned())
        } else {
            None
        }
    }

    fn make_canvas_rejected_candidate_summary(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.contains("Rejected") {
            Some(product_panel_message(
                status,
                "Some ideas were rejected because they looked too similar.",
            ))
        } else {
            None
        }
    }

    fn make_canvas_local_error(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.starts_with("Ignored a background result") {
            return None;
        }
        let lower = status.to_ascii_lowercase();
        if lower.contains("failed")
            || lower.contains("could not")
            || lower.contains("missing")
            || lower.contains("disconnected")
        {
            Some(product_panel_message(
                status,
                "The current asset needs attention.",
            ))
        } else {
            None
        }
    }

    fn make_is_preparing_now(&self) -> bool {
        if self.state.document.is_none() {
            return false;
        }

        let compiling_or_editing = self.state.active_jobs.values().any(|request| {
            matches!(
                request.slot(),
                FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
            )
        });
        let preview_rendering = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == FoundryJobSlot::RenderPreview);
        let model_ready = self.state.current_output.is_some() && !compiling_or_editing;
        let preview_ready = model_ready
            && self.state.current_preview.as_ref().is_some_and(|preview| {
                !preview.rgba8.is_empty() && preview.build == self.state.current_build
            })
            && !preview_rendering;

        !model_ready || !preview_ready || compiling_or_editing || preview_rendering
    }

    fn refresh_make_preparation_timer(&mut self) {
        if self.make_is_preparing_now() {
            self.make_preparation_started_at
                .get_or_insert_with(Instant::now);
        } else {
            self.make_preparation_started_at = None;
        }
    }

    fn refresh_make_generation_timer(&mut self) {
        if self.directions_are_generating() {
            self.make_generation_started_at
                .get_or_insert_with(Instant::now);
        } else {
            self.make_generation_started_at = None;
        }
    }

    fn refresh_make_trace_clock(&mut self) {
        let elapsed_ms = self.make_trace_started_at.elapsed().as_millis();
        let elapsed_ms = elapsed_ms.min(u128::from(u64::MAX)) as u64;
        self.state.set_make_trace_elapsed_ms(elapsed_ms);
    }

    fn persist_make_job_trace_outputs(&mut self) {
        if let Err(error) = self
            .state
            .write_make_job_trace_outputs(Path::new(MAKE_JOB_TRACE_DIR))
        {
            self.state.status = Some(format!("Could not write Make job trace: {error}"));
        }
    }

    fn save_state_pill(&self) -> (&'static str, StatusTone) {
        if self.state.document.is_none() {
            ("Choose Box Primitive", StatusTone::Neutral)
        } else if self.state.project_path.is_none() {
            ("Not saved", StatusTone::Warning)
        } else if self.state.dirty {
            ("Unsaved", StatusTone::Warning)
        } else {
            ("Saved", StatusTone::Ready)
        }
    }

    fn status_summary(&self) -> &'static str {
        if !self.state.active_jobs.is_empty() {
            "Working"
        } else if self.state.dirty {
            "Unsaved changes"
        } else {
            "Ready"
        }
    }

    fn model_status(&self) -> &'static str {
        if self.state.current_output.is_some() {
            "Model ready"
        } else if self.state.document.is_some() {
            "Preparing model"
        } else {
            "Choose Box Primitive"
        }
    }

    fn preview_status(&self) -> &'static str {
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        if rendering_preview {
            "Preview building"
        } else if self.state.current_preview.is_some() {
            "Ready"
        } else if self.state.current_output.is_some() {
            "Ready soon"
        } else {
            "Preparing"
        }
    }

    fn directions_are_generating(&self) -> bool {
        self.state.active_jobs.values().any(|request| {
            matches!(
                request,
                FoundryJobRequest::GenerateCandidates { .. }
                    | FoundryJobRequest::RenderCandidatePreviews { .. }
            )
        }) || candidate_previews_are_pending(&self.state.candidates)
    }

    fn active_profile_matches(&self, profile_id: &str) -> bool {
        self.state.document.as_ref().is_some_and(|document| {
            document
                .customizer_profile_ref
                .stable_id
                .contains(profile_id)
                || document.family_content_ref.stable_id.contains(profile_id)
                || document.document_id.0.contains(profile_id)
        })
    }

    fn active_profile_is_box_primitive(&self) -> bool {
        self.active_profile_matches(BOX_PRIMITIVE_PROFILE_ID)
    }

    fn material_look_action_visible(&self, view_state: &MakeCanvasViewState) -> bool {
        let _ = view_state;
        false
    }

    fn material_look_report_path(&self) -> PathBuf {
        self.material_looks
            .evidence_report_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(SURFACE_CANDIDATE_REPORT_RELATIVE_PATH))
    }

    fn current_artifact_fingerprint_hex(&self) -> Option<String> {
        self.state
            .current_build
            .as_ref()
            .map(|build| build.artifact_fingerprint.0.to_hex())
    }

    fn open_material_looks_panel(&mut self) {
        self.material_looks.tray_open = true;
        self.material_looks.load_error = None;

        let Some(current_fingerprint) = self.current_artifact_fingerprint_hex() else {
            self.material_looks.load_error =
                Some("Prepare the Box Primitive before trying material looks.".to_owned());
            self.material_looks.evidence = None;
            self.material_looks.selected_candidate_id = None;
            return;
        };

        let report_path = self.material_look_report_path();
        match load_material_look_evidence(&report_path, Some(current_fingerprint.as_str())) {
            Ok(evidence) => {
                if self
                    .material_looks
                    .selected_candidate_id
                    .as_ref()
                    .is_none_or(|selected| {
                        !evidence
                            .candidates
                            .iter()
                            .any(|candidate| &candidate.candidate_id == selected)
                    })
                {
                    self.material_looks.selected_candidate_id = evidence
                        .candidates
                        .first()
                        .map(|candidate| candidate.candidate_id.clone());
                }
                self.material_looks.evidence = Some(evidence);
            }
            Err(error) => {
                self.material_looks.load_error = Some(error);
                self.material_looks.evidence = None;
                self.material_looks.selected_candidate_id = None;
            }
        }
    }

    fn selected_material_look(&self) -> Option<&MakeMaterialLookCandidate> {
        let evidence = self.material_looks.evidence.as_ref()?;
        let selected_id = self.material_looks.selected_candidate_id.as_deref();
        selected_id
            .and_then(|id| {
                evidence
                    .candidates
                    .iter()
                    .find(|candidate| candidate.candidate_id == id)
            })
            .or_else(|| evidence.candidates.first())
    }

    fn material_look_export_copy(&self) -> Option<(&'static str, &'static str)> {
        if self.active_profile_is_box_primitive() {
            return None;
        }
        self.selected_material_look().map(|_| {
            (
                MATERIAL_LOOK_PREVIEW_ONLY_COPY,
                MATERIAL_LOOK_FULL_READY_BLOCKED_COPY,
            )
        })
    }

    fn show_home(&mut self, ui: &mut egui::Ui) {
        let profiles = self.home_profiles.as_slice();
        if profiles.is_empty() {
            product_empty_state(
                ui,
                "Box Primitive is not available",
                "Open a saved project, or enable the Box Primitive baseline.",
            );
            return;
        }
        let mut selected_fixture = None;

        ui.horizontal_top(|ui| {
            let left_width = home_browser_width(ui.available_width());
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    show_home_browser_panel(
                        ui,
                        profiles,
                        &mut self.home_search_query,
                        &mut self.home_filter,
                        &mut self.selected_home_profile_slug,
                    );
                },
            );
            ui.add_space(18.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let selected =
                        selected_home_profile(profiles, &self.selected_home_profile_slug);
                    if let Some(profile) = selected {
                        if show_home_selected_template_stage(
                            ui,
                            profile,
                            &mut self.home_thumbnails,
                            &mut self.texture_cache,
                        )
                        .clicked()
                        {
                            selected_fixture = Some(profile.fixture.clone());
                        }
                    } else {
                        product_empty_state(
                            ui,
                            "No matching starting point",
                            "Change the search to choose Box Primitive.",
                        );
                    }
                },
            );
        });

        if let Some(fixture) = selected_fixture {
            self.load_fixture(fixture, ui.ctx());
        }
    }

    fn show_make(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view_state = self.make_canvas_view_state();
        if view_state.mode == MakeCanvasMode::NoAsset {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive or open a project before making changes.",
            ));
            return commands;
        }

        let available = ui.available_size();
        let layout = make_canvas_layout(available, &view_state);

        ui.allocate_ui_with_layout(
            egui::vec2(available.x, layout.top_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                if layout.stacked_columns {
                    let stage_height = make_canvas_stacked_stage_height(layout.top_height);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), stage_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            commands.extend(self.show_make_model_stage_panel(ui, &view_state));
                        },
                    );
                    ui.add_space(layout.content_gap);
                    ui.allocate_ui_with_layout(
                        egui::vec2(
                            ui.available_width(),
                            (layout.top_height - stage_height - layout.content_gap).max(180.0),
                        ),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            commands.extend(self.show_make_inspector_panel(ui, &view_state));
                        },
                    );
                } else if layout.inline_ideas {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(layout.stage_width, layout.top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                commands.extend(self.show_make_model_stage_panel(ui, &view_state));
                            },
                        );
                        ui.add_space(layout.content_gap);
                        ui.allocate_ui_with_layout(
                            egui::vec2(layout.ideas_width, layout.top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                commands.extend(self.show_direction_board_panel(ui, &view_state));
                            },
                        );
                        ui.add_space(layout.content_gap);
                        ui.allocate_ui_with_layout(
                            egui::vec2(layout.inspector_width, layout.top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                commands.extend(self.show_make_inspector_panel(ui, &view_state));
                            },
                        );
                    });
                } else {
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            egui::vec2(layout.stage_width, layout.top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                commands.extend(self.show_make_model_stage_panel(ui, &view_state));
                            },
                        );
                        ui.add_space(layout.content_gap);
                        ui.allocate_ui_with_layout(
                            egui::vec2(layout.inspector_width, layout.top_height),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                commands.extend(self.show_make_inspector_panel(ui, &view_state));
                            },
                        );
                    });
                }
            },
        );

        if layout.tray_height > 0.0 {
            ui.add_space(layout.tray_gap);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), layout.tray_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    commands.extend(self.show_direction_board_panel(ui, &view_state));
                },
            );
        }
        commands
    }

    fn show_make_model_stage_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        let part_groups = self
            .state
            .document
            .as_ref()
            .map(directions::direction_part_groups_for_document)
            .unwrap_or_default();
        let active_group_id = self
            .state
            .document
            .as_ref()
            .and_then(|document| {
                document
                    .variation_state
                    .intent
                    .scope
                    .semantic_part_group_id()
            })
            .map(str::to_owned);
        let active_group = active_group_id
            .as_deref()
            .and_then(|group_id| part_groups.iter().find(|group| group.group_id == group_id));
        let interactions_enabled =
            view_state.model_ready && view_state.preview_ready && !view_state.local_busy_visible;

        let response = product_stage(ui, |ui| {
            ui.set_min_height(ui.available_height().max(220.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Current asset")
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                let state_label = match view_state.mode {
                    MakeCanvasMode::PreparingAsset => view_state.preparation_phase.label(),
                    MakeCanvasMode::GeneratingWholeAssetIdeas
                    | MakeCanvasMode::GeneratingFocusedPartIdeas => "Trying ideas",
                    _ if view_state.preview_ready => "Ready",
                    _ => "Preparing",
                };
                let tone = match view_state.mode {
                    MakeCanvasMode::PreparingAsset
                    | MakeCanvasMode::GeneratingWholeAssetIdeas
                    | MakeCanvasMode::GeneratingFocusedPartIdeas => StatusTone::Working,
                    _ if view_state.preview_ready => StatusTone::Ready,
                    _ => StatusTone::Neutral,
                };
                let _ = status_pill(ui, StatusPillSpec::new(state_label, tone));
            });
            ui.add_space(8.0);
            let preview_edge = make_stage_preview_edge(ui.available_width(), ui.available_height());
            self.show_current_preview_sized(ui, preview_edge);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for group in &part_groups {
                    let selected = active_group_id.as_deref() == Some(group.group_id.as_str());
                    let reason = if interactions_enabled {
                        group
                            .unavailable_reason
                            .as_deref()
                            .unwrap_or("This part has no focused ideas yet.")
                    } else if view_state.mode == MakeCanvasMode::PreparingAsset {
                        ASSET_PREPARING_REASON
                    } else {
                        ACTIVE_IDEA_JOB_REASON
                    };
                    let response = variation_mode_button(
                        ui,
                        &group.label,
                        selected,
                        group.focusable && interactions_enabled,
                        reason,
                    );
                    if response.clicked() && group.focusable && interactions_enabled {
                        commands.push(directions::set_focus_part_group_command(group));
                    }
                }
            });
            if let Some(group) = active_group {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("Focused: {}", group.label))
                        .color(colors.accent_hover)
                        .strong(),
                );
            }
            ui.add_space(10.0);
            if let Some(group) = active_group {
                commands.extend(self.show_make_focus_action_tray(ui, view_state, group));
            } else if view_state.mode != MakeCanvasMode::ReviewingIdeas {
                commands.extend(self.show_make_primary_stage_action(ui, view_state));
            }
        });
        if let Some(group) = active_group {
            draw_focus_callout(ui, response.response.rect, &group.label);
        }
        if let Some(label) = &view_state.local_busy_label {
            draw_busy_overlay(ui, response.response.rect, label);
        }
        commands
    }

    fn show_make_primary_stage_action(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.horizontal_wrapped(|ui| {
            let primary = action_spec(
                view_state.primary_action_enabled,
                &view_state.primary_action_label,
                ButtonTone::Primary,
                view_state
                    .primary_action_disabled_reason
                    .as_deref()
                    .unwrap_or(ASSET_PREPARING_REASON),
            );
            if action_button(ui, &primary).clicked() {
                self.push_make_primary_action_commands(&mut commands, view_state);
            }
            if self.material_look_action_visible(view_state)
                && action_button(
                    ui,
                    &ActionSpec::enabled(ACTION_TRY_MATERIAL_LOOKS, ButtonTone::Secondary),
                )
                .clicked()
            {
                self.open_material_looks_panel();
            }
        });
        commands
    }

    fn show_make_focus_action_tray(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
        group: &directions::DirectionPartGroup,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        product_card(ui, true, |ui| {
            let colors = VisualFoundryTokens::dark().colors;
            ui.horizontal_wrapped(|ui| {
                let focus_status = format!("Focused: {}", group.label);
                let _ = status_pill(ui, StatusPillSpec::new(&focus_status, StatusTone::Working));
                let try_label = if view_state.mode == MakeCanvasMode::ReviewingIdeas {
                    ACTION_TRY_MORE_IDEAS.to_owned()
                } else {
                    view_state.primary_action_label.clone()
                };
                let try_tone = if view_state.mode == MakeCanvasMode::ReviewingIdeas {
                    ButtonTone::Secondary
                } else {
                    ButtonTone::Primary
                };
                let try_enabled = if view_state.mode == MakeCanvasMode::ReviewingIdeas {
                    make_canvas_candidate_actions_enabled(view_state)
                } else {
                    view_state.primary_action_enabled
                };
                let disabled_reason = view_state
                    .primary_action_disabled_reason
                    .as_deref()
                    .unwrap_or(ACTIVE_IDEA_JOB_REASON);
                if action_button(
                    ui,
                    &action_spec(try_enabled, &try_label, try_tone, disabled_reason),
                )
                .clicked()
                {
                    if view_state.mode == MakeCanvasMode::ReviewingIdeas {
                        if let Some(command) = self.make_primary_candidate_command() {
                            commands.push(command);
                        }
                    } else {
                        self.push_make_primary_action_commands(&mut commands, view_state);
                    }
                }
                let lock_label = directions::lock_focused_part_label(group);
                if action_button(
                    ui,
                    &action_spec(
                        make_canvas_controls_enabled(view_state),
                        &lock_label,
                        ButtonTone::Secondary,
                        disabled_reason,
                    ),
                )
                .clicked()
                {
                    commands.push(FoundryAppCommand::run(FoundryCommand::SetLock {
                        lock: shape_foundry::FoundryLock {
                            target: shape_foundry::FoundryLockTarget::FocusPartGroup(
                                group.group_id.clone(),
                            ),
                            mode: shape_foundry::FoundryLockMode::SearchProtected,
                            reason: Some(format!("{} kept while trying ideas.", group.label)),
                        },
                    }));
                }
                if action_button(
                    ui,
                    &action_spec(
                        make_canvas_controls_enabled(view_state),
                        ACTION_CLEAR_FOCUS,
                        ButtonTone::Secondary,
                        disabled_reason,
                    ),
                )
                .clicked()
                {
                    commands.push(directions::clear_focus_part_group_command());
                }
            });
            ui.label(
                RichText::new("The highlight shows which part these actions affect.")
                    .color(colors.text_muted)
                    .small(),
            );
        });
        commands
    }

    fn show_make_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        product_card(ui, false, |ui| {
            ui.set_min_height(ui.available_height().max(240.0));
            ui.label(
                RichText::new(&view_state.primary_title)
                    .color(colors.text)
                    .size(18.0)
                    .strong(),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(make_canvas_mode_summary(view_state))
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
            ui.add(
                egui::Label::new(
                    RichText::new(&view_state.next_action_hint)
                        .color(colors.accent_hover)
                        .strong(),
                )
                .wrap(),
            );
            ui.add_space(12.0);
            if make_canvas_inspector_build_actions_visible(view_state) {
                ui.horizontal_wrapped(|ui| {
                    let build_actions_enabled =
                        make_canvas_build_dependent_actions_enabled(view_state);
                    let build_actions_reason =
                        make_canvas_build_dependent_disabled_reason(view_state);
                    if view_state.mode == MakeCanvasMode::ReviewingIdeas
                        && action_button(
                            ui,
                            &action_spec(
                                make_canvas_candidate_actions_enabled(view_state),
                                view_state.try_ideas_action_label,
                                ButtonTone::Secondary,
                                build_actions_reason,
                            ),
                        )
                        .clicked()
                        && let Some(command) = self.make_primary_candidate_command()
                    {
                        commands.push(command);
                    }
                    if self.material_look_action_visible(view_state)
                        && action_button(
                            ui,
                            &ActionSpec::enabled(ACTION_TRY_MATERIAL_LOOKS, ButtonTone::Secondary),
                        )
                        .clicked()
                    {
                        self.open_material_looks_panel();
                    }
                    if action_button(
                        ui,
                        &action_spec(
                            build_actions_enabled,
                            ACTION_ADD_TO_PACK,
                            ButtonTone::Secondary,
                            build_actions_reason,
                        ),
                    )
                    .clicked()
                        && let Some(command) = self.add_current_to_pack_command()
                    {
                        commands.push(command);
                        self.drawer = Some(FoundryDrawer::Pack);
                    }
                    if action_button(
                        ui,
                        &action_spec(
                            build_actions_enabled,
                            ACTION_OPEN_PACK,
                            ButtonTone::Secondary,
                            build_actions_reason,
                        ),
                    )
                    .clicked()
                    {
                        self.drawer = Some(FoundryDrawer::Pack);
                    }
                    if action_button(
                        ui,
                        &action_spec(
                            build_actions_enabled,
                            ACTION_OPEN_EXPORT,
                            ButtonTone::Secondary,
                            build_actions_reason,
                        ),
                    )
                    .clicked()
                    {
                        self.drawer = Some(FoundryDrawer::Export);
                    }
                });
                ui.add_space(10.0);
            }
            if view_state.material_look_tray_visible {
                self.show_material_look_inspector_summary(ui);
                ui.add_space(10.0);
            }
            if view_state.mode != MakeCanvasMode::ReviewingIdeas
                || matches!(
                    view_state.local_banner_tone,
                    BannerTone::Warning | BannerTone::Error
                )
            {
                status_banner(
                    ui,
                    StatusBannerSpec {
                        title: &view_state.local_banner_title,
                        message: &view_state.local_banner_message,
                        tone: view_state.local_banner_tone,
                    },
                );
                ui.add_space(8.0);
            }
            ui.horizontal_wrapped(|ui| {
                if view_state.local_warning_message.is_some()
                    && action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_TRY_AGAIN, ButtonTone::Secondary),
                    )
                    .clicked()
                    && let Some(command) = self.make_primary_candidate_command()
                {
                    commands.push(command);
                }
                if view_state.idea_generation_fallback_visible {
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_CANCEL, ButtonTone::Secondary),
                    )
                    .clicked()
                    {
                        commands.push(FoundryAppCommand::CancelIdeaGeneration);
                    }
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_KEEP_WAITING, ButtonTone::Secondary),
                    )
                    .clicked()
                    {
                        self.make_generation_started_at = Some(Instant::now());
                    }
                }
                if view_state.preparation_fallback_visible {
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_RETRY_PREPARATION, ButtonTone::Primary),
                    )
                    .clicked()
                    {
                        commands.push(FoundryAppCommand::RetryPreparation);
                    }
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_CHOOSE_ANOTHER_TEMPLATE, ButtonTone::Secondary),
                    )
                    .clicked()
                    {
                        self.tab = FoundryTab::Home;
                    }
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_OPEN_PROJECT, ButtonTone::Secondary),
                    )
                    .clicked()
                        && let Some(path) = open_foundry_project_file()
                    {
                        commands.push(FoundryAppCommand::Load(path));
                    }
                }
                if view_state.preview_update_required
                    && action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_UPDATE_PREVIEW, ButtonTone::Primary),
                    )
                    .clicked()
                {
                    let preview_pixels = current_preview_pixels_for_context(ui.ctx());
                    commands.push(FoundryAppCommand::RequestPreview {
                        width: preview_pixels,
                        height: preview_pixels,
                    });
                }
            });
            ui.add_space(12.0);
            ui.label(
                RichText::new(view_state.adjust_heading_label)
                    .color(colors.text)
                    .strong(),
            );
            ui.add_space(6.0);
            let active_group = self.active_make_part_group();
            let control_sections =
                make_context_inspector_controls(&self.state.controls, active_group.as_ref());
            egui::ScrollArea::vertical()
                .id_salt("make_context_inspector_controls")
                .auto_shrink([false, false])
                .max_height((ui.available_height() - 8.0).max(72.0))
                .show(ui, |ui| {
                    if control_sections.visible.is_empty() {
                        product_compact_empty_state(
                            ui,
                            control_sections.empty_title,
                            control_sections.empty_message,
                        );
                    } else {
                        let current_build = self.state.current_build.clone();
                        let texture_cache = &mut self.texture_cache;
                        let actions_enabled = make_canvas_controls_enabled(view_state);
                        let disabled_reason = view_state
                            .primary_action_disabled_reason
                            .as_deref()
                            .unwrap_or(ACTIVE_IDEA_JOB_REASON);
                        for control in &control_sections.visible {
                            commands.extend(show_customize_control_card(
                                ui,
                                texture_cache,
                                current_build.as_ref(),
                                control,
                                actions_enabled,
                                disabled_reason,
                                !view_state.box_primitive_baseline,
                            ));
                            ui.add_space(8.0);
                        }
                        if !control_sections.overflow.is_empty() {
                            egui::CollapsingHeader::new(control_sections.disclosure_label)
                                .id_salt("make_context_more_controls")
                                .show(ui, |ui| {
                                    for control in &control_sections.overflow {
                                        commands.extend(show_customize_control_card(
                                            ui,
                                            texture_cache,
                                            current_build.as_ref(),
                                            control,
                                            actions_enabled,
                                            disabled_reason,
                                            !view_state.box_primitive_baseline,
                                        ));
                                        ui.add_space(8.0);
                                    }
                                });
                        }
                    }
                });
        });
        commands
    }

    fn show_material_look_inspector_summary(&mut self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        compact_section_header(
            ui,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        );
        ui.add_space(6.0);

        if let Some(error) = self.material_looks.load_error.clone() {
            ui.label(
                RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                    MATERIAL_LOOK_MISSING_MESSAGE
                } else {
                    "Material looks unavailable"
                })
                .color(colors.text)
                .strong(),
            );
            ui.label(
                RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                    SURFACE_PACKAGE_COMMAND_COPY
                } else {
                    error.as_str()
                })
                .color(colors.text_muted)
                .small(),
            );
            return;
        }

        let Some(evidence) = self.material_looks.evidence.clone() else {
            ui.label(
                RichText::new(MATERIAL_LOOK_MISSING_MESSAGE)
                    .color(colors.text_muted)
                    .small(),
            );
            return;
        };
        if evidence.candidates.is_empty() {
            ui.label(
                RichText::new("Material looks unavailable")
                    .color(colors.text_muted)
                    .small(),
            );
            return;
        }

        let selected_id = self
            .material_looks
            .selected_candidate_id
            .as_deref()
            .unwrap_or_else(|| evidence.candidates[0].candidate_id.as_str());
        let selected_candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == selected_id)
            .unwrap_or(&evidence.candidates[0])
            .clone();
        let current_preview = self.state.current_preview.clone();
        let current_build = self.state.current_build.clone();
        let texture_cache = &mut self.texture_cache;
        let preview_edge = (ui.available_width() * 0.18).clamp(54.0, 72.0);

        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width((ui.available_width() * 0.40).max(120.0));
                ui.label(
                    RichText::new("Current Material")
                        .color(colors.text)
                        .small()
                        .strong(),
                );
                if let Some(preview) = current_preview.as_ref() {
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: "material-look-inspector-current",
                            build: preview.build.as_ref(),
                            rgba8: &preview.rgba8,
                            width: preview.width,
                            height: preview.height,
                            max_edge: preview_edge,
                        },
                    );
                }
            });
            ui.vertical_centered(|ui| {
                ui.set_width((ui.available_width() * 0.55).max(120.0));
                ui.label(
                    RichText::new("Candidate Material")
                        .color(colors.text)
                        .small()
                        .strong(),
                );
                let preview_id = format!(
                    "material-look-inspector-selected-{}",
                    selected_candidate.candidate_id
                );
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build.as_ref(),
                        rgba8: &selected_candidate.rgba8,
                        width: selected_candidate.width,
                        height: selected_candidate.height,
                        max_edge: preview_edge,
                    },
                );
            });
        });
        ui.label(
            RichText::new(&selected_candidate.display_name)
                .color(colors.text)
                .small()
                .strong(),
        );
        ui.label(
            RichText::new(MATERIAL_LOOK_PREVIEW_ONLY_COPY)
                .color(colors.warning)
                .small(),
        );
        ui.add_space(6.0);

        let columns = evidence.candidates.iter().take(3).collect::<Vec<_>>();
        if !columns.is_empty() {
            let column_width = ((ui.available_width() - 12.0) / columns.len() as f32).max(84.0);
            let mut selected = None;
            ui.horizontal_top(|ui| {
                for candidate in columns {
                    ui.allocate_ui_with_layout(
                        egui::vec2(column_width, 132.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            let preview_id =
                                format!("material-look-inspector-card-{}", candidate.candidate_id);
                            show_rgba_preview(
                                ui,
                                texture_cache,
                                FoundryPreviewDraw {
                                    preview_id: &preview_id,
                                    build: current_build.as_ref(),
                                    rgba8: &candidate.rgba8,
                                    width: candidate.width,
                                    height: candidate.height,
                                    max_edge: 54.0,
                                },
                            );
                            ui.label(
                                RichText::new(&candidate.display_name)
                                    .color(colors.text)
                                    .small(),
                            );
                            if action_button(
                                ui,
                                &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
                            )
                            .clicked()
                            {
                                selected = Some(candidate.candidate_id.clone());
                            }
                        },
                    );
                }
            });
            if let Some(candidate_id) = selected {
                self.material_looks.selected_candidate_id = Some(candidate_id);
            }
        }
    }

    fn show_material_looks_panel(&mut self, ui: &mut egui::Ui) {
        compact_section_header(
            ui,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
        );
        ui.add_space(8.0);

        if let Some(error) = self.material_looks.load_error.clone() {
            let title = if error == MATERIAL_LOOK_MISSING_MESSAGE {
                MATERIAL_LOOK_MISSING_MESSAGE
            } else {
                "Material looks unavailable"
            };
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(RichText::new(title).color(colors.text).strong());
                ui.add(
                    egui::Label::new(
                        RichText::new(if error == MATERIAL_LOOK_MISSING_MESSAGE {
                            "Material looks are not part of the Box Primitive baseline."
                        } else {
                            error.as_str()
                        })
                        .color(colors.text_muted),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(SURFACE_PACKAGE_COMMAND_COPY)
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                ui.monospace(SURFACE_PACKAGE_COMMAND);
            });
            return;
        }

        let Some(evidence) = self.material_looks.evidence.clone() else {
            product_compact_empty_state(
                ui,
                MATERIAL_LOOK_MISSING_MESSAGE,
                "Material looks are not part of the Box Primitive baseline.",
            );
            return;
        };
        if evidence.candidates.is_empty() {
            product_compact_empty_state(
                ui,
                "Material looks unavailable",
                "The surface package did not include valid material candidates.",
            );
            return;
        }

        let selected_id = self
            .material_looks
            .selected_candidate_id
            .as_deref()
            .unwrap_or_else(|| evidence.candidates[0].candidate_id.as_str());
        let selected_candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == selected_id)
            .unwrap_or(&evidence.candidates[0])
            .clone();
        if !evidence.full_ready_blocker_codes.is_empty() {
            ui.label(
                RichText::new(MATERIAL_LOOK_FULL_READY_BLOCKED_COPY)
                    .color(VisualFoundryTokens::dark().colors.text_muted)
                    .small(),
            );
            ui.add_space(6.0);
        }
        let current_preview = self.state.current_preview.as_ref();
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        let panel_width = ui.available_width();
        let selected = if panel_width >= 980.0 {
            let comparison_width = (panel_width * 0.38).clamp(360.0, 580.0);
            let mut selected = None;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(comparison_width, ui.available_height().max(280.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        show_material_look_comparison_card(
                            ui,
                            texture_cache,
                            current_preview,
                            current_build,
                            &selected_candidate,
                        );
                    },
                );
                ui.add_space(10.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ui.available_height().max(280.0)),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        selected = show_material_look_candidate_grid(
                            ui,
                            texture_cache,
                            current_build,
                            &evidence.candidates,
                            selected_candidate.candidate_id.as_str(),
                            true,
                        );
                    },
                );
            });
            selected
        } else {
            show_material_look_comparison_card(
                ui,
                texture_cache,
                current_preview,
                current_build,
                &selected_candidate,
            );
            ui.add_space(8.0);
            show_material_look_candidate_grid(
                ui,
                texture_cache,
                current_build,
                &evidence.candidates,
                selected_candidate.candidate_id.as_str(),
                false,
            )
        };

        if let Some(selected) = selected {
            self.material_looks.selected_candidate_id = Some(selected);
        }
    }

    fn make_primary_candidate_command(&self) -> Option<FoundryAppCommand> {
        let document = self.state.document.as_ref()?;
        let active_group = self.active_make_part_group();
        let variation_intent = active_group
            .as_ref()
            .map_or_else(VariationIntent::complete_look, |group| {
                VariationIntent::focus_part_shape(&group.group_id, &group.label)
            });
        Some(FoundryAppCommand::RequestCandidates(
            FoundryCandidateRequest {
                seed: document.seed,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: if active_group.is_some() {
                    FoundryCandidateMode::Refine
                } else {
                    FoundryCandidateMode::Explore
                },
                strategy_id: None,
                preference_profile: None,
                variation_intent,
            },
        ))
    }

    fn make_whole_asset_candidate_command(&self) -> Option<FoundryAppCommand> {
        self.state.document.as_ref().map(|_| {
            make_whole_asset_candidate_request(&self.state, false, FoundryCandidateMode::Explore)
        })
    }

    fn make_focused_recovery_commands(&self) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if self.active_make_part_group().is_some() {
            commands.push(directions::clear_focus_part_group_command());
        }
        if let Some(command) = self.make_whole_asset_candidate_command() {
            commands.push(command);
        }
        commands
    }

    fn visible_review_candidate(
        &self,
    ) -> Option<&crate::foundry::view_model::FoundryCandidateCard> {
        self.state
            .selected_candidate
            .as_ref()
            .and_then(|selected| {
                self.state
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == selected)
            })
            .or_else(|| self.state.candidates.first())
    }

    fn accept_visible_candidate_command(&self) -> Option<FoundryAppCommand> {
        let candidate = self.visible_review_candidate()?;
        candidate
            .selectable
            .then(|| directions::accept_candidate_command(candidate.id.clone()))
    }

    fn push_make_primary_action_commands(
        &mut self,
        commands: &mut Vec<FoundryAppCommand>,
        view_state: &MakeCanvasViewState,
    ) {
        match view_state.mode {
            MakeCanvasMode::NoAsset => {
                self.tab = FoundryTab::Home;
            }
            MakeCanvasMode::ReviewingIdeas => {
                if let Some(command) = self.accept_visible_candidate_command() {
                    commands.push(command);
                }
            }
            _ if view_state.focused_no_candidates_recovery_visible => {
                commands.extend(self.make_focused_recovery_commands());
            }
            _ => {
                if let Some(command) = self.make_primary_candidate_command() {
                    commands.push(command);
                }
            }
        }
    }

    fn show_direction_board_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if view_state.material_look_tray_visible {
            self.show_material_looks_panel(ui);
            ui.add_space(12.0);
        }
        let generating = matches!(
            view_state.mode,
            MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas
        );
        let active_group = self.active_make_part_group();
        let ideas_title = active_group.as_ref().map_or_else(
            || "Ideas".to_owned(),
            |group| format!("{} ideas", singular_title_case_part_label(&group.label)),
        );
        let direction_count_label = if view_state.mode == MakeCanvasMode::PreparingAsset {
            "Ideas unlock after the model and preview are ready.".to_owned()
        } else if generating {
            view_state
                .local_busy_label
                .as_deref()
                .unwrap_or("Trying ideas from the current asset...")
                .to_owned()
        } else {
            direction_board_count_label(
                view_state.candidate_count,
                view_state.box_primitive_baseline,
            )
        };
        if make_canvas_uses_compact_ideas(view_state) {
            compact_section_header(
                ui,
                "Ideas",
                ideas_title.as_str(),
                direction_count_label.as_str(),
            );
        } else {
            section_header(
                ui,
                SectionHeaderSpec {
                    eyebrow: if view_state.candidate_count > 0 {
                        "Compare"
                    } else {
                        "Ideas"
                    },
                    title: ideas_title.as_str(),
                    subtitle: Some(direction_count_label.as_str()),
                },
            );
        }
        if let Some(message) = &view_state.local_warning_message {
            status_banner(
                ui,
                StatusBannerSpec {
                    title: make_canvas_warning_title(message),
                    message,
                    tone: BannerTone::Warning,
                },
            );
            ui.add_space(6.0);
            if action_button(
                ui,
                &ActionSpec::enabled(ACTION_TRY_AGAIN, ButtonTone::Secondary),
            )
            .clicked()
                && let Some(command) = self.make_primary_candidate_command()
            {
                commands.push(command);
            }
            ui.add_space(8.0);
        } else if let Some(message) = &view_state.rejected_candidate_summary {
            status_banner(
                ui,
                StatusBannerSpec {
                    title: "Idea search result",
                    message,
                    tone: BannerTone::Info,
                },
            );
            ui.add_space(8.0);
        } else if !view_state.candidate_tray_visible {
            ui.label(
                RichText::new("Try ideas when the asset is ready.")
                    .color(VisualFoundryTokens::dark().colors.text_muted)
                    .small(),
            );
            ui.add_space(8.0);
        }
        if let Some(message) = &view_state.local_error_message {
            status_banner(
                ui,
                StatusBannerSpec {
                    title: "Asset needs attention",
                    message,
                    tone: BannerTone::Error,
                },
            );
            ui.add_space(8.0);
        }
        ui.add_space(8.0);
        match view_state.candidate_tray_state {
            MakeCandidateTrayState::GeneratingSkeletons => {
                show_direction_skeleton_grid(ui);
                return commands;
            }
            MakeCandidateTrayState::EmptyReady => {
                let (title, message) = if view_state.mode == MakeCanvasMode::PreparingAsset {
                    (
                        "Ideas unlock when ready",
                        "The asset can be adjusted while the preview prepares.",
                    )
                } else {
                    (
                        "Ready to try ideas",
                        if view_state.box_primitive_baseline {
                            "Try box ideas when the box is ready."
                        } else {
                            "Try ideas or focus a part when the asset is ready."
                        },
                    )
                };
                product_compact_empty_state(ui, title, message);
                return commands;
            }
            MakeCandidateTrayState::NoCandidatesWithRecovery => {
                commands.extend(self.show_no_candidates_recovery_card(ui, active_group.as_ref()));
                return commands;
            }
            MakeCandidateTrayState::ErrorWithRecovery => {
                commands.extend(self.show_candidate_error_recovery_card(ui, view_state));
                return commands;
            }
            MakeCandidateTrayState::HasCandidates => {}
        }
        if self.state.candidates.is_empty() {
            return commands;
        }

        let current_preview = self.state.current_preview.as_ref();
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        commands.extend(show_visible_direction_ideas_board(
            ui,
            texture_cache,
            VisibleDirectionIdeasBoard {
                current_build,
                current_preview,
                candidates: &self.state.candidates,
                actions_enabled: make_canvas_candidate_actions_enabled(view_state),
                disabled_reason: view_state
                    .primary_action_disabled_reason
                    .as_deref()
                    .unwrap_or(ACTIVE_IDEA_JOB_REASON),
                use_candidate_label: view_state.use_candidate_action_label,
            },
        ));
        commands
    }

    fn show_no_candidates_recovery_card(
        &mut self,
        ui: &mut egui::Ui,
        active_group: Option<&directions::DirectionPartGroup>,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let (title, message) =
            no_candidates_recovery_copy(active_group, self.state.candidate_output.as_deref());
        product_card(ui, false, |ui| {
            let colors = VisualFoundryTokens::dark().colors;
            ui.label(RichText::new(title).color(colors.text).size(17.0).strong());
            ui.add(egui::Label::new(RichText::new(message).color(colors.text_muted)).wrap());
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let primary_label = if active_group.is_some() {
                    ACTION_TRY_WHOLE_ASSET_RECOVERY
                } else {
                    ACTION_TRY_AGAIN
                };
                if action_button(ui, &ActionSpec::enabled(primary_label, ButtonTone::Primary))
                    .clicked()
                {
                    if active_group.is_some() {
                        commands.extend(self.make_focused_recovery_commands());
                    } else if let Some(command) = self.make_primary_candidate_command() {
                        commands.push(command);
                    }
                }
                if active_group.is_some()
                    && action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_CLEAR_FOCUS, ButtonTone::Secondary),
                    )
                    .clicked()
                {
                    commands.push(directions::clear_focus_part_group_command());
                }
                let unlock_command = self.clear_all_locks_command();
                let unlock_spec = action_spec(
                    unlock_command.is_some(),
                    ACTION_UNLOCK_CONTROLS,
                    ButtonTone::Secondary,
                    NEED_LOCKED_CONTROLS_REASON,
                );
                if action_button(ui, &unlock_spec).clicked()
                    && let Some(command) = unlock_command
                {
                    commands.push(command);
                }
            });
        });
        commands
    }

    fn show_candidate_error_recovery_card(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let message = view_state
            .local_error_message
            .as_deref()
            .unwrap_or("The current asset needs attention.");
        product_card(ui, false, |ui| {
            let colors = VisualFoundryTokens::dark().colors;
            ui.label(
                RichText::new("Asset needs attention")
                    .color(colors.text)
                    .size(17.0)
                    .strong(),
            );
            ui.add(egui::Label::new(RichText::new(message).color(colors.text_muted)).wrap());
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                let try_enabled = view_state.model_ready && view_state.preview_ready;
                let try_reason = view_state
                    .primary_action_disabled_reason
                    .as_deref()
                    .unwrap_or(ASSET_PREPARING_REASON);
                if action_button(
                    ui,
                    &action_spec(
                        try_enabled,
                        ACTION_TRY_AGAIN,
                        ButtonTone::Primary,
                        try_reason,
                    ),
                )
                .clicked()
                    && let Some(command) = self.make_primary_candidate_command()
                {
                    commands.push(command);
                }
                if action_button(
                    ui,
                    &ActionSpec::enabled(ACTION_CHOOSE_ANOTHER_TEMPLATE, ButtonTone::Secondary),
                )
                .clicked()
                {
                    self.tab = FoundryTab::Home;
                }
                if action_button(
                    ui,
                    &ActionSpec::enabled(ACTION_OPEN_PROJECT, ButtonTone::Secondary),
                )
                .clicked()
                    && let Some(path) = open_foundry_project_file()
                {
                    commands.push(FoundryAppCommand::Load(path));
                }
            });
        });
        commands
    }

    fn clear_all_locks_command(&self) -> Option<FoundryAppCommand> {
        if self.state.locks.is_empty() {
            return None;
        }

        Some(FoundryAppCommand::RunFoundryCommandProgram {
            label: ACTION_UNLOCK_CONTROLS.to_owned(),
            commands: self
                .state
                .locks
                .iter()
                .map(|lock| FoundryCommand::ClearLock {
                    target: lock.target.clone(),
                })
                .collect(),
        })
    }

    fn show_choose_asset_empty_state(
        &mut self,
        ui: &mut egui::Ui,
        title: &str,
        message: &str,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 260.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                product_card(ui, false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.set_min_height(220.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.add_space(32.0);
                        ui.label(RichText::new(title).color(colors.text).size(18.0).strong());
                        ui.add(
                            egui::Label::new(RichText::new(message).color(colors.text_muted))
                                .wrap(),
                        );
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.add_space(((ui.available_width() - 260.0) * 0.5).max(0.0));
                            if action_button(
                                ui,
                                &ActionSpec::enabled(ACTION_CHOOSE_TEMPLATE, ButtonTone::Primary),
                            )
                            .clicked()
                            {
                                self.tab = FoundryTab::Home;
                            }
                            if action_button(
                                ui,
                                &ActionSpec::enabled(ACTION_OPEN_PROJECT, ButtonTone::Secondary),
                            )
                            .clicked()
                                && let Some(path) = open_foundry_project_file()
                            {
                                commands.push(FoundryAppCommand::Load(path));
                            }
                        });
                    });
                });
            },
        );
        commands
    }

    fn show_customize(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        egui::ScrollArea::vertical()
            .id_salt("foundry_customize_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                section_header(
                    ui,
                    SectionHeaderSpec {
                        eyebrow: "Customize",
                        title: "Adjust controls",
                        subtitle: Some(
                            "Tune the main box controls and lock the settings you want to keep.",
                        ),
                    },
                );
                if self.state.document.is_none() {
                    commands.extend(self.show_choose_asset_empty_state(
                        ui,
                        "Choose an asset first",
                        "Choose Box Primitive or open a project before adjusting.",
                    ));
                } else {
                    ui.add_space(10.0);
                    let controls = display_customize_controls(&self.state.controls)
                        .into_iter()
                        .cloned()
                        .collect::<Vec<_>>();
                    if controls.is_empty() {
                        product_empty_state(
                            ui,
                            "No quick controls yet",
                            "This asset has no quick controls yet.",
                        );
                    } else {
                        let preview_edge = if ui.available_width() >= 980.0 {
                            (ui.available_width() * 0.42).clamp(460.0, 680.0)
                        } else {
                            360.0
                        };
                        self.show_customize_preview_panel(ui, preview_edge);
                        ui.add_space(18.0);
                        commands.extend(self.show_customize_control_deck(ui, &controls));
                    }
                }
            });

        commands
    }

    fn show_customize_preview_panel(&mut self, ui: &mut egui::Ui, preview_edge: f32) {
        product_stage(ui, |ui| {
            let colors = VisualFoundryTokens::dark().colors;
            ui.label(
                RichText::new("Whole-model preview")
                    .color(colors.accent_hover)
                    .small()
                    .strong(),
            );
            self.show_current_preview_sized(ui, preview_edge);
            ui.add_space(8.0);
            ui.label(
                RichText::new(self.current_project_title())
                    .color(colors.text)
                    .strong(),
            );
            ui.label(
                RichText::new(
                    "Select a part or option below; the full asset stays in view for context.",
                )
                .color(colors.text_muted)
                .small(),
            );
        });
    }

    fn show_customize_control_deck(
        &mut self,
        ui: &mut egui::Ui,
        controls: &[crate::foundry::view_model::FoundryControlView],
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        ui.label(
            RichText::new("Make it yours")
                .color(colors.text)
                .size(18.0)
                .strong(),
        );
        ui.label(
            RichText::new(format!("{} primary controls", controls.len())).color(colors.text_muted),
        );
        ui.add_space(8.0);
        let current_build = self.state.current_build.clone();
        let texture_cache = &mut self.texture_cache;
        for control in controls {
            commands.extend(show_customize_control_card(
                ui,
                texture_cache,
                current_build.as_ref(),
                control,
                true,
                ACTIVE_IDEA_JOB_REASON,
                true,
            ));
            ui.add_space(8.0);
        }
        commands
    }

    fn show_history(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = history::build_history_view(&self.state);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Project",
                title: "Project history",
                subtitle: Some("Review previous project steps and branch from a saved point."),
            },
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            for action in &view.actions {
                if action_button(
                    ui,
                    &action_spec(
                        action.enabled,
                        &action.label,
                        ButtonTone::Secondary,
                        NEED_HISTORY_REASON,
                    ),
                )
                .clicked()
                    && let Some(command) = self.history_dispatch_command(action.dispatch.as_ref())
                {
                    commands.push(command);
                }
            }
        });
        ui.add_space(12.0);
        ui.label(
            RichText::new(format!("{} saved step(s)", view.rows.len()))
                .color(VisualFoundryTokens::dark().colors.text_muted),
        );
        for row in view.rows {
            product_card(ui, row.selected, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(product_panel_message(&row.summary.label, "Project step"));
                        if let Some(detail) = &row.summary.detail {
                            ui.label(
                                RichText::new(product_panel_message(detail, "Project updated."))
                                    .color(VisualFoundryTokens::dark().colors.text_muted)
                                    .small(),
                            );
                        }
                    });
                    if row.selected {
                        ui.weak("Current");
                    }
                    if let Some(intent) = &row.switch_intent
                        && action_button(ui, &ActionSpec::enabled(ACTION_SWITCH, ButtonTone::Quiet))
                            .clicked()
                        && let Some(command) =
                            self.history_dispatch_command(intent.dispatch.as_ref())
                    {
                        commands.push(command);
                    }
                    if let Some(intent) = &row.branch_intent
                        && action_button(ui, &ActionSpec::enabled(ACTION_BRANCH, ButtonTone::Quiet))
                            .clicked()
                        && let Some(command) =
                            self.history_dispatch_command(intent.dispatch.as_ref())
                    {
                        commands.push(command);
                    }
                });
            });
            ui.add_space(8.0);
        }
        commands
    }

    fn show_export(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Export",
                title: BOX_PRIMITIVE_EXPORT_TITLE,
                subtitle: Some(BOX_PRIMITIVE_EXPORT_DETAIL),
            },
        );
        ui.add_space(10.0);
        if self.state.document.is_none() {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive or open a project before exporting.",
            ));
            return commands;
        }
        if let Some((export_copy, full_ready_copy)) = self.material_look_export_copy() {
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(
                    RichText::new(MATERIAL_LOOK_SECTION_TITLE)
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                ui.add(egui::Label::new(RichText::new(export_copy).color(colors.warning)).wrap());
                ui.add(
                    egui::Label::new(RichText::new(full_ready_copy).color(colors.text_muted))
                        .wrap(),
                );
            });
            ui.add_space(10.0);
        }
        let can_export = self.state.current_output.is_some();
        let pack_view = self.state.pack.clone();
        let pack_panel = pack::pack_panel_view(&pack_view);
        let wide_layout = ui.available_width() >= 980.0;
        if wide_layout {
            ui.columns(2, |columns| {
                product_card(&mut columns[0], false, |ui| {
                    ui.label(
                        RichText::new("CURRENT ASSET")
                            .color(VisualFoundryTokens::dark().colors.accent_hover)
                            .small(),
                    );
                    self.show_current_preview_sized(ui, 420.0);
                });
                commands.extend(show_export_readiness_panel(
                    &mut columns[1],
                    can_export,
                    &pack_view,
                    &pack_panel,
                ));
            });
        } else {
            product_card(ui, false, |ui| {
                ui.label(
                    RichText::new("CURRENT ASSET")
                        .color(VisualFoundryTokens::dark().colors.accent_hover)
                        .small(),
                );
                self.show_current_preview_sized(ui, 320.0);
            });
            ui.add_space(12.0);
            commands.extend(show_export_readiness_panel(
                ui,
                can_export,
                &pack_view,
                &pack_panel,
            ));
        }
        commands
    }

    fn show_pack(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = pack::pack_panel_view(&self.state.pack);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Pack",
                title: "Pack preview",
                subtitle: Some("Collect coherent variants before exporting a set."),
            },
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let add_enabled = self.state.document.is_some();
            if action_button(
                ui,
                &action_spec(
                    add_enabled,
                    ACTION_ADD_CURRENT_ASSET,
                    ButtonTone::Primary,
                    NEED_PROJECT_REASON,
                ),
            )
            .clicked()
                && let Some(command) = self.add_current_to_pack_command()
            {
                commands.push(command);
            }
            let batch_export_reason = view
                .export
                .disabled_reason
                .as_ref()
                .map(|reason| product_panel_message(reason, NEED_PACK_MEMBER_REASON))
                .unwrap_or_else(|| NEED_PACK_MEMBER_REASON.to_owned());
            if action_button(
                ui,
                &action_spec(
                    view.export.enabled,
                    ACTION_EXPORT_PACK,
                    ButtonTone::Secondary,
                    batch_export_reason.as_str(),
                ),
            )
            .clicked()
                && let Some(out_dir) = select_pack_export_dir()
                && let Some(command) = pack::batch_export_command(&self.state.pack, out_dir)
            {
                commands.push(command);
            }
        });
        ui.add_space(12.0);
        if self.state.document.is_none() && view.members.is_empty() {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive or open a project before starting a pack.",
            ));
            return commands;
        }
        if !view.active {
            ui.columns(2, |columns| {
                product_card(&mut columns[0], false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(
                        RichText::new("CURRENT ASSET")
                            .color(colors.accent_hover)
                            .small(),
                    );
                    self.show_current_preview_sized(ui, 280.0);
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(self.current_project_title())
                            .color(colors.text)
                            .strong(),
                    );
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_ADD_CURRENT_ASSET, ButtonTone::Primary),
                    )
                    .clicked()
                        && let Some(command) = self.add_current_to_pack_command()
                    {
                        commands.push(command);
                    }
                });
                product_card(&mut columns[1], false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(RichText::new("Add assets to export").color(colors.text));
                    ui.label(
                        RichText::new("0 assets in pack")
                            .color(colors.text_muted)
                            .small(),
                    );
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(NEED_PACK_MEMBER_REASON)
                            .color(colors.warning)
                            .small(),
                    );
                });
            });
            return commands;
        }
        ui.columns(2, |columns| {
            let current_preview = self.state.current_preview.as_ref();
            let current_build = self.state.current_build.as_ref();
            let texture_cache = &mut self.texture_cache;
            product_card(&mut columns[0], false, |ui| {
                ui.label(
                    RichText::new("Contact sheet")
                        .color(VisualFoundryTokens::dark().colors.accent_hover)
                        .small(),
                );
                ui.label(
                    RichText::new(format!("{} assets in pack", view.contact_sheet.cells.len()))
                        .color(VisualFoundryTokens::dark().colors.text),
                );
                ui.add_space(6.0);
                show_pack_contact_sheet(
                    ui,
                    texture_cache,
                    current_preview,
                    current_build,
                    &view.contact_sheet,
                );
            });

            product_card(&mut columns[1], view.export.enabled, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.label(RichText::new(pack_readiness_label(&view)).color(colors.text));
                ui.label(
                    RichText::new(pack_readiness_detail(&view))
                        .color(colors.text_muted)
                        .small(),
                );
                if !view.coherence_warnings.is_empty() {
                    ui.add_space(8.0);
                    for warning in view.coherence_warnings.iter().take(3) {
                        ui.label(
                            RichText::new(product_panel_message(
                                &warning.message,
                                "Pack needs attention before export.",
                            ))
                            .color(colors.warning),
                        );
                    }
                }
            });
        });
        commands
    }

    fn show_pack_drawer(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        self.show_pack(ui)
    }

    fn show_export_drawer(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        self.show_export(ui)
    }

    fn show_current_preview_sized(&mut self, ui: &mut egui::Ui, max_edge: f32) {
        let asset_name = self.current_project_title();
        let preview = self.state.current_preview.as_ref();
        let has_output = self.state.current_output.is_some();
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        let preview_is_stale =
            preview.is_some_and(|preview| preview.build != self.state.current_build);
        let draw_edge = max_edge.min(ui.available_width().max(1.0));
        ui.vertical_centered(|ui| {
            if let Some(preview) = &preview {
                let preview_id = format!("current-{}", preview.preview_id);
                show_rgba_preview(
                    ui,
                    &mut self.texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: preview.build.as_ref(),
                        rgba8: &preview.rgba8,
                        width: preview.width,
                        height: preview.height,
                        max_edge: draw_edge,
                    },
                );
                if preview_is_stale || rendering_preview {
                    ui.label(
                        RichText::new(PREVIEW_UPDATING_REASON)
                            .color(VisualFoundryTokens::dark().colors.warning)
                            .small(),
                    );
                }
            } else if has_output {
                if rendering_preview {
                    ui.weak(PREVIEW_UPDATING_REASON);
                } else {
                    ui.weak("Preview is being prepared.");
                }
            } else {
                draw_quick_template_preview(ui, draw_edge, &asset_name);
                ui.add_space(8.0);
                ui.weak("Preparing your asset...");
            }
        });
    }

    fn history_dispatch_command(
        &self,
        dispatch: Option<&history::FoundryHistoryActionDispatch>,
    ) -> Option<FoundryAppCommand> {
        match dispatch? {
            history::FoundryHistoryActionDispatch::Command(command) => Some(command.clone()),
            history::FoundryHistoryActionDispatch::RequestSaveAsPath => {
                save_foundry_project_file().map(FoundryAppCommand::SaveAs)
            }
            history::FoundryHistoryActionDispatch::RequestLoadPath => {
                open_foundry_project_file().map(FoundryAppCommand::Load)
            }
        }
    }

    fn add_current_to_pack_command(&self) -> Option<FoundryAppCommand> {
        let document = self.state.document.as_ref()?;
        let pack_id = self
            .state
            .pack
            .pack_id
            .clone()
            .unwrap_or_else(|| "foundry_pack".to_owned());
        let member_id = unique_pack_member_id(&self.state.pack, &document.document_id.0);
        Some(pack::add_current_asset_to_pack_command(pack_id, member_id))
    }

    fn apply_commands(&mut self, commands: Vec<FoundryAppCommand>, ctx: &egui::Context) {
        let mut state_changed = false;
        for command in commands {
            self.refresh_make_trace_clock();
            if matches!(
                command,
                FoundryAppCommand::RetryPreparation
                    | FoundryAppCommand::RequestBuild
                    | FoundryAppCommand::RequestPreview { .. }
            ) {
                self.make_preparation_started_at = Some(Instant::now());
            }
            if matches!(command, FoundryAppCommand::RequestCandidates(_)) {
                self.make_generation_started_at = Some(Instant::now());
            } else if matches!(command, FoundryAppCommand::CancelIdeaGeneration) {
                self.make_generation_started_at = None;
            }
            match self.state.handle_command(command) {
                Ok(effects) => {
                    state_changed = true;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if state_changed {
            self.texture_cache.clear();
        }
    }

    fn apply_effects(&mut self, effects: Vec<FoundryAppEffect>, ctx: &egui::Context) {
        let mut trace_changed = false;
        for effect in effects {
            match effect {
                FoundryAppEffect::StartJob(request) => {
                    trace_changed = true;
                    self.submit_job(*request);
                }
                FoundryAppEffect::SaveProject { path, project } => {
                    self.save_project(path, *project);
                }
                FoundryAppEffect::LoadProject(path) => self.load_project(path, ctx),
            }
        }
        if trace_changed {
            self.persist_make_job_trace_outputs();
        }
    }

    fn submit_job(&mut self, request: FoundryJobRequest) {
        self.jobs.submit(request);
    }

    fn poll_jobs(&mut self, ctx: &egui::Context) {
        if self.screenshot_scenario_holds_active_job_capture() {
            ctx.request_repaint_after(Duration::from_millis(250));
            return;
        }

        let mut affected = false;
        let mut trace_changed = false;
        let mut schedule_preview = false;
        let mut schedule_candidate_previews = None;

        loop {
            let event = match self.jobs.try_recv() {
                Ok(event) => event,
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.state.status = Some("Foundry background worker disconnected.".to_owned());
                    break;
                }
            };
            self.refresh_make_trace_clock();
            let should_preview = matches!(
                event,
                FoundryJobEvent::CompileFinished { .. } | FoundryJobEvent::EditApplied { .. }
            );
            let candidate_preview_request = match &event {
                FoundryJobEvent::CandidatesGenerated {
                    request, output, ..
                } => Some((request.clone(), output.as_ref().clone())),
                _ => None,
            };
            if self.state.handle_job_event(event) {
                affected = true;
                trace_changed = true;
                schedule_preview |= should_preview;
                if let Some(request) = candidate_preview_request {
                    schedule_candidate_previews = Some(request);
                }
            } else {
                trace_changed = true;
            }
        }

        if schedule_preview {
            self.refresh_make_trace_clock();
            let preview_pixels = current_preview_pixels_for_context(ctx);
            match self.state.request_preview(preview_pixels, preview_pixels) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if let Some((request, output)) = schedule_candidate_previews {
            self.refresh_make_trace_clock();
            match self.state.request_candidate_previews(request, output) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if trace_changed {
            self.persist_make_job_trace_outputs();
        }
        if affected {
            self.texture_cache.clear();
            ctx.request_repaint();
        }
    }

    fn screenshot_scenario_holds_active_job_capture(&self) -> bool {
        self.screenshot_scenario_step == u8::MAX
            && !self.state.active_jobs.is_empty()
            && matches!(
                self.screenshot_scenario,
                Some(ScreenshotScenario::GeneratingBoxIdeas)
            )
    }

    fn poll_home_thumbnail_jobs(&mut self, ctx: &egui::Context) {
        if self.home_thumbnails.poll() {
            ctx.request_repaint();
        }
    }

    fn apply_screenshot_scenario(&mut self, ctx: &egui::Context) -> Vec<FoundryAppCommand> {
        let Some(scenario) = self.screenshot_scenario else {
            return Vec::new();
        };
        if self.screenshot_scenario_step == u8::MAX {
            return Vec::new();
        }

        let mut commands = Vec::new();
        if self.screenshot_scenario_step == 0 {
            self.load_fixture(read_screenshot_fixture_catalog(), ctx);
            self.tab = FoundryTab::Make;
            self.screenshot_scenario_step = 1;
            return commands;
        }
        if self.state.current_output.is_none() {
            return commands;
        }
        if !self.state.active_jobs.is_empty() {
            let view_state = self.make_canvas_view_state();
            match scenario {
                ScreenshotScenario::GeneratingBoxIdeas
                    if view_state.mode == MakeCanvasMode::GeneratingWholeAssetIdeas =>
                {
                    self.complete_screenshot_scenario(scenario);
                    return commands;
                }
                _ => {
                    ctx.request_repaint_after(Duration::from_millis(33));
                    return commands;
                }
            }
        }

        match scenario {
            ScreenshotScenario::MakeInitialBox => {
                if self.state.active_jobs.is_empty() {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::GeneratingBoxIdeas => {
                if self.state.candidates.is_empty() && !self.directions_are_generating() {
                    commands.push(make_whole_asset_candidate_request(
                        &self.state,
                        false,
                        FoundryCandidateMode::Explore,
                    ));
                    self.screenshot_scenario_step = 2;
                }
            }
            ScreenshotScenario::GeneratedBoxIdeas | ScreenshotScenario::SelectedComparison => {
                if self.state.candidates.is_empty() && !self.directions_are_generating() {
                    commands.push(make_whole_asset_candidate_request(
                        &self.state,
                        false,
                        FoundryCandidateMode::Explore,
                    ));
                    self.screenshot_scenario_step = 2;
                } else if !self.state.candidates.is_empty() && self.state.active_jobs.is_empty() {
                    let target_candidate = match scenario {
                        ScreenshotScenario::SelectedComparison => self
                            .state
                            .candidates
                            .get(1)
                            .or_else(|| self.state.candidates.first()),
                        _ => self.state.candidates.first(),
                    };
                    if let Some(candidate) = target_candidate
                        && self.state.selected_candidate.as_ref() != Some(&candidate.id)
                    {
                        commands.push(FoundryAppCommand::SelectCandidate(Some(
                            candidate.id.clone(),
                        )));
                    } else {
                        let view_state = self.make_canvas_view_state();
                        if candidate_review_is_ready(&view_state, &self.state.candidates) {
                            self.complete_screenshot_scenario(scenario);
                        } else {
                            ctx.request_repaint_after(Duration::from_millis(33));
                        }
                    }
                }
            }
            ScreenshotScenario::AdjustedBoxControl => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(FoundryAppCommand::run(FoundryCommand::SetControl {
                        control_id: "edge_softness".to_owned(),
                        value: shape_foundry::ControlValue::Scalar(0.55),
                    }));
                    self.screenshot_scenario_step = 2;
                } else if self.make_canvas_view_state().mode == MakeCanvasMode::Ready {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::PackDrawer => {
                self.drawer = Some(FoundryDrawer::Pack);
                if self.state.pack.members.is_empty() {
                    if let Some(command) = self.add_current_to_pack_command() {
                        commands.push(command);
                    }
                    self.screenshot_scenario_step = 1;
                } else if self.make_canvas_view_state().pack_drawer_visible {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::ExportDrawer => {
                self.drawer = Some(FoundryDrawer::Export);
                if self.make_canvas_view_state().export_drawer_visible {
                    self.complete_screenshot_scenario(scenario);
                }
            }
        }
        if self.screenshot_scenario_step != u8::MAX {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        commands
    }

    #[cfg(test)]
    fn ensure_screenshot_focus(&mut self, group_id: &str) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if !self.state.active_jobs.is_empty() {
            return commands;
        }
        let Some(group) = screenshot_part_group(&self.state, group_id) else {
            return commands;
        };
        let active_group_id = self.state.document.as_ref().and_then(|document| {
            document
                .variation_state
                .intent
                .scope
                .semantic_part_group_id()
        });
        if active_group_id == Some(group_id) {
            self.screenshot_scenario_step = self.screenshot_scenario_step.max(2);
            return commands;
        }
        commands.push(directions::set_focus_part_group_command(&group));
        self.screenshot_scenario_step = 1;
        commands
    }

    fn complete_screenshot_scenario(&mut self, scenario: ScreenshotScenario) {
        let view_state = self.make_canvas_view_state();
        let result = screenshot_scenario_assertion(scenario, &view_state);
        record_screenshot_state_assertion(
            scenario,
            &view_state,
            result.as_ref().err().map(String::as_str),
        );
        match result {
            Ok(()) => {
                self.screenshot_scenario_step = u8::MAX;
            }
            Err(message) => {
                self.state.status = Some(format!(
                    "Screenshot state assertion failed: {}",
                    product_panel_message(&message, "Screenshot state assertion failed.")
                ));
                self.screenshot_scenario_step = u8::MAX;
            }
        }
    }

    fn save_project(&mut self, path: PathBuf, project: FoundryProject) {
        if let Err(error) = ensure_foundry_project_path(&path) {
            self.state.status = Some(error.to_string());
            return;
        }
        match project.save_json(&path) {
            Ok(()) => {
                self.state.mark_saved(path.clone());
                self.remember_recent_project(path.clone());
                self.state.status = Some(format!("Saved {}", path.display()));
            }
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn load_project(&mut self, path: PathBuf, ctx: &egui::Context) {
        match FoundryProjectFile::load(&path) {
            Ok(project_file) => match self.state.replace_loaded_project(project_file) {
                Ok(effects) => {
                    self.make_trace_started_at = Instant::now();
                    self.state.set_make_trace_elapsed_ms(0);
                    self.jobs.reset();
                    self.texture_cache.clear();
                    self.material_looks.clear_for_asset();
                    self.state.status = Some(format!("Loaded {}", path.display()));
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
                    self.remember_recent_project(path.clone());
                    self.make_preparation_started_at = Some(Instant::now());
                    self.make_generation_started_at = None;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn load_fixture(&mut self, fixture: FoundryFixtureCatalog, ctx: &egui::Context) {
        self.make_trace_started_at = Instant::now();
        self.make_generation_started_at = None;
        self.jobs.reset();
        self.texture_cache.clear();
        self.material_looks.clear_for_asset();
        match FoundryAppState::new(fixture.document) {
            Ok(mut state) => match state.request_build() {
                Ok(effects) => {
                    state.status = Some(format!("Loaded {} fixture.", fixture.slug));
                    self.state = state;
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
                    self.make_preparation_started_at = Some(Instant::now());
                    self.make_generation_started_at = None;
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn remember_recent_project(&mut self, path: PathBuf) {
        self.recent_projects.retain(|recent| recent != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(5);
    }
}

impl eframe::App for FoundryDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        FoundryDesktopApp::ui(self, ui, frame);
    }
}

#[derive(Default)]
struct FoundryJobCoordinator {
    tx: Option<Sender<FoundryJobEvent>>,
    rx: Option<Receiver<FoundryJobEvent>>,
    preview_cache: Arc<Mutex<FoundryPreviewCache>>,
}

impl FoundryJobCoordinator {
    fn submit(&mut self, request: FoundryJobRequest) {
        let job_id = request.job_id();
        let tx = self.tx().clone();
        let preview_cache = Arc::clone(&self.preview_cache);
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let event = if request_uses_preview_cache(&request) {
                match preview_cache.lock() {
                    Ok(mut preview_cache) => {
                        run_foundry_job(request, &resolver, &mut preview_cache)
                    }
                    Err(_) => FoundryJobEvent::Failed {
                        job_id,
                        message: "Foundry preview cache lock was poisoned.".to_owned(),
                    },
                }
            } else {
                let mut preview_cache = FoundryPreviewCache::default();
                run_foundry_job(request, &resolver, &mut preview_cache)
            };
            let _ = tx.send(event);
        });
    }

    fn try_recv(&mut self) -> Result<FoundryJobEvent, crossbeam_channel::TryRecvError> {
        self.rx().try_recv()
    }

    fn reset(&mut self) {
        let (tx, rx) = unbounded();
        self.tx = Some(tx);
        self.rx = Some(rx);
        self.preview_cache = Arc::new(Mutex::new(FoundryPreviewCache::default()));
    }

    fn tx(&mut self) -> &Sender<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.tx.as_ref().expect("tx initialized")
    }

    fn rx(&mut self) -> &Receiver<FoundryJobEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.rx.as_ref().expect("rx initialized")
    }
}

fn request_uses_preview_cache(request: &FoundryJobRequest) -> bool {
    matches!(
        request,
        FoundryJobRequest::RenderPreview { .. }
            | FoundryJobRequest::PreviewControlValue { .. }
            | FoundryJobRequest::GenerateCandidates { .. }
            | FoundryJobRequest::RenderCandidatePreviews { .. }
    )
}

struct BuiltInFoundryCatalogResolver {
    fixtures: Vec<FoundryFixtureCatalog>,
}

impl Default for BuiltInFoundryCatalogResolver {
    fn default() -> Self {
        Self {
            fixtures: headless_fixture_catalogs(),
        }
    }
}

impl FoundryCatalogResolver for BuiltInFoundryCatalogResolver {
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.fixtures
            .iter()
            .find_map(|fixture| {
                fixture
                    .entries
                    .get(&content_ref.stable_id)
                    .map(|entry| entry.canonical_json.clone())
            })
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

#[derive(Default)]
struct HomeThumbnailCoordinator {
    tx: Option<Sender<HomeThumbnailEvent>>,
    rx: Option<Receiver<HomeThumbnailEvent>>,
    active: BTreeSet<String>,
    failed: BTreeSet<String>,
    pending_frames: BTreeMap<String, i32>,
    thumbnails: BTreeMap<String, HomeTemplateThumbnail>,
}

#[derive(Debug)]
struct HomeThumbnailEvent {
    slug: String,
    frame_index: i32,
    result: Result<HomeThumbnailJobOutput, String>,
}

#[derive(Debug)]
struct HomeThumbnailJobOutput {
    mesh: Option<Arc<TriangleMesh>>,
    base_camera: OrbitCamera,
    frame: HomeThumbnailFrame,
}

#[derive(Debug, Clone)]
struct HomeTemplateThumbnail {
    mesh: Arc<TriangleMesh>,
    base_camera: OrbitCamera,
    selected_yaw_degrees: f32,
    selected_frame_index: i32,
    prewarm_cursor: usize,
    frames: BTreeMap<i32, HomeThumbnailFrame>,
}

#[derive(Debug, Clone)]
struct HomeThumbnailFrame {
    frame_index: i32,
    rgba8: Vec<u8>,
    width: u32,
    height: u32,
}

const HOME_THUMBNAIL_PIXELS: u32 = 512;
const HOME_TURNTABLE_FRAME_COUNT: i32 = 24;
const MAX_HOME_THUMBNAIL_JOBS: usize = 2;
const HOME_THUMBNAIL_YAW_DEGREES_PER_POINT: f32 = 0.45;

impl HomeThumbnailCoordinator {
    fn ensure(&mut self, profile: &ProductHomeProfile) {
        let slug = profile.fixture.slug.clone();
        if self.thumbnails.contains_key(&slug)
            || self.active.contains(&slug)
            || self.failed.contains(&slug)
            || self.active.len() >= MAX_HOME_THUMBNAIL_JOBS
        {
            return;
        }

        self.active.insert(slug.clone());
        let fixture = profile.fixture.clone();
        let tx = self.tx().clone();
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let result = compile_foundry_document(&fixture.document, &resolver)
                .map_err(|error| format!("{error:?}"))
                .and_then(|output| {
                    render_home_thumbnail_from_output(&output, 0)
                        .ok_or_else(|| "Could not render template thumbnail.".to_owned())
                });
            let _ = tx.send(HomeThumbnailEvent {
                slug,
                frame_index: 0,
                result,
            });
        });
    }

    fn orbit_thumbnail(&mut self, slug: &str, delta: egui::Vec2) -> bool {
        let Some(thumbnail) = self.thumbnails.get_mut(slug) else {
            return false;
        };
        if delta == egui::Vec2::ZERO {
            return false;
        }

        let frame_index = {
            thumbnail.selected_yaw_degrees = home_turntable_yaw(
                thumbnail.selected_yaw_degrees + delta.x * HOME_THUMBNAIL_YAW_DEGREES_PER_POINT,
            );
            let frame_index = home_turntable_frame_index(thumbnail.selected_yaw_degrees);
            thumbnail.selected_frame_index = frame_index;
            frame_index
        };
        self.ensure_frame(slug, frame_index);
        true
    }

    fn poll(&mut self) -> bool {
        let mut changed = false;
        let rx = self.rx().clone();
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    self.active.remove(&event.slug);
                    match event.result {
                        Ok(output) => {
                            self.failed.remove(&event.slug);
                            self.store_frame(event.slug.clone(), output);
                            changed = true;
                        }
                        Err(_) => {
                            self.failed.insert(event.slug.clone());
                            changed = true;
                        }
                    }
                    if let Some(frame_index) = self.pending_frames.remove(&event.slug) {
                        self.spawn_frame_render(&event.slug, frame_index);
                    }
                    self.spawn_next_pending_frame();
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.active.clear();
                    break;
                }
            }
        }
        changed
    }

    fn thumbnail(&self, slug: &str) -> Option<&HomeThumbnailFrame> {
        let thumbnail = self.thumbnails.get(slug)?;
        thumbnail
            .frames
            .get(&thumbnail.selected_frame_index)
            .or_else(|| nearest_home_thumbnail_frame(thumbnail))
    }

    fn prewarm_turntable(&mut self, slug: &str) {
        if self.active.len() >= MAX_HOME_THUMBNAIL_JOBS {
            return;
        }
        let Some(frame_index) = ({
            let Some(thumbnail) = self.thumbnails.get_mut(slug) else {
                return;
            };
            let frame_order = home_turntable_prewarm_order(thumbnail.selected_frame_index);
            let mut frame_index = None;
            for offset in 0..frame_order.len() {
                let cursor = (thumbnail.prewarm_cursor + offset) % frame_order.len();
                let candidate = frame_order[cursor];
                if !thumbnail.frames.contains_key(&candidate) {
                    thumbnail.prewarm_cursor = (cursor + 1) % frame_order.len();
                    frame_index = Some(candidate);
                    break;
                }
            }
            frame_index
        }) else {
            return;
        };
        self.spawn_frame_render(slug, frame_index);
    }

    fn spawn_next_pending_frame(&mut self) {
        while self.active.len() < MAX_HOME_THUMBNAIL_JOBS {
            let next = self.pending_frames.iter().find_map(|(slug, frame_index)| {
                let thumbnail = self.thumbnails.get(slug)?;
                (!self.active.contains(slug) && !thumbnail.frames.contains_key(frame_index))
                    .then(|| (slug.clone(), *frame_index))
            });
            let Some((slug, frame_index)) = next else {
                self.pending_frames.retain(|slug, frame_index| {
                    self.thumbnails
                        .get(slug)
                        .is_some_and(|thumbnail| !thumbnail.frames.contains_key(frame_index))
                });
                return;
            };
            self.pending_frames.remove(&slug);
            self.spawn_frame_render(&slug, frame_index);
        }
    }

    fn has_active_jobs(&self) -> bool {
        !self.active.is_empty()
    }

    fn ensure_frame(&mut self, slug: &str, frame_index: i32) {
        let Some(thumbnail) = self.thumbnails.get(slug) else {
            return;
        };
        if thumbnail.frames.contains_key(&frame_index) {
            return;
        }
        if self.active.contains(slug) {
            self.pending_frames.insert(slug.to_owned(), frame_index);
            return;
        }
        self.spawn_frame_render(slug, frame_index);
    }

    fn spawn_frame_render(&mut self, slug: &str, frame_index: i32) {
        if self.active.len() >= MAX_HOME_THUMBNAIL_JOBS {
            self.pending_frames.insert(slug.to_owned(), frame_index);
            return;
        }
        let Some(thumbnail) = self.thumbnails.get(slug) else {
            return;
        };
        self.active.insert(slug.to_owned());
        let mesh = Arc::clone(&thumbnail.mesh);
        let camera = home_turntable_camera(&thumbnail.base_camera, frame_index);
        let base_camera = thumbnail.base_camera.clone();
        let tx = self.tx().clone();
        let slug = slug.to_owned();
        thread::spawn(move || {
            let result = render_home_thumbnail(mesh, base_camera, camera, frame_index, None)
                .ok_or_else(|| "Could not render template thumbnail.".to_owned());
            let _ = tx.send(HomeThumbnailEvent {
                slug,
                frame_index,
                result,
            });
        });
    }

    fn store_frame(&mut self, slug: String, output: HomeThumbnailJobOutput) {
        if let Some(thumbnail) = self.thumbnails.get_mut(&slug) {
            thumbnail
                .frames
                .insert(output.frame.frame_index, output.frame);
            return;
        }
        let Some(mesh) = output.mesh else {
            return;
        };
        let mut frames = BTreeMap::new();
        frames.insert(output.frame.frame_index, output.frame);
        self.thumbnails.insert(
            slug,
            HomeTemplateThumbnail {
                mesh,
                base_camera: output.base_camera,
                selected_yaw_degrees: 0.0,
                selected_frame_index: 0,
                prewarm_cursor: 0,
                frames,
            },
        );
    }

    fn reset(&mut self) {
        let (tx, rx) = unbounded();
        self.tx = Some(tx);
        self.rx = Some(rx);
        self.active.clear();
        self.failed.clear();
        self.pending_frames.clear();
        self.thumbnails.clear();
    }

    fn tx(&mut self) -> &Sender<HomeThumbnailEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.tx.as_ref().expect("home thumbnail tx initialized")
    }

    fn rx(&mut self) -> &Receiver<HomeThumbnailEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.rx.as_ref().expect("home thumbnail rx initialized")
    }
}

fn render_home_thumbnail_from_output(
    output: &FoundryCompilationOutput,
    frame_index: i32,
) -> Option<HomeThumbnailJobOutput> {
    let mesh = &output.artifact.combined_preview.mesh;
    let mesh = Arc::new(TriangleMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: mesh.bounds.min.into(),
            max: mesh.bounds.max.into(),
        },
    });
    let base_camera = fit_camera_to_bounds(mesh.bounds);
    let camera = home_turntable_camera(&base_camera, frame_index);
    render_home_thumbnail(
        Arc::clone(&mesh),
        base_camera,
        camera,
        frame_index,
        Some(Arc::clone(&mesh)),
    )
}

fn render_home_thumbnail(
    mesh: Arc<TriangleMesh>,
    base_camera: OrbitCamera,
    camera: OrbitCamera,
    frame_index: i32,
    include_mesh: Option<Arc<TriangleMesh>>,
) -> Option<HomeThumbnailJobOutput> {
    let settings = clay_readability_render_settings(HOME_THUMBNAIL_PIXELS, HOME_THUMBNAIL_PIXELS);
    let image = render_mesh(mesh.as_ref(), &camera, &settings).ok()?;
    Some(HomeThumbnailJobOutput {
        mesh: include_mesh,
        base_camera,
        frame: HomeThumbnailFrame {
            frame_index,
            rgba8: image.rgba8,
            width: image.width,
            height: image.height,
        },
    })
}

#[cfg(test)]
fn orbit_home_thumbnail_camera(camera: &OrbitCamera, delta: egui::Vec2) -> OrbitCamera {
    let mut camera = camera.clone();
    camera.orbit(delta.x * HOME_THUMBNAIL_YAW_DEGREES_PER_POINT, 0.0);
    camera
}

fn home_turntable_yaw(yaw_degrees: f32) -> f32 {
    yaw_degrees.rem_euclid(360.0)
}

fn home_turntable_frame_index(yaw_degrees: f32) -> i32 {
    let frame_width = 360.0 / HOME_TURNTABLE_FRAME_COUNT as f32;
    ((home_turntable_yaw(yaw_degrees) / frame_width).round() as i32)
        .rem_euclid(HOME_TURNTABLE_FRAME_COUNT)
}

fn home_turntable_camera(base_camera: &OrbitCamera, frame_index: i32) -> OrbitCamera {
    let mut camera = base_camera.clone();
    let frame_width = 360.0 / HOME_TURNTABLE_FRAME_COUNT as f32;
    camera.yaw_degrees = home_turntable_yaw(frame_index as f32 * frame_width);
    camera.clamped()
}

fn home_turntable_prewarm_order(selected_frame_index: i32) -> Vec<i32> {
    let selected = selected_frame_index.rem_euclid(HOME_TURNTABLE_FRAME_COUNT);
    let mut frames = Vec::with_capacity(HOME_TURNTABLE_FRAME_COUNT as usize);
    frames.push(selected);
    for distance in 1..=HOME_TURNTABLE_FRAME_COUNT / 2 {
        frames.push((selected + distance).rem_euclid(HOME_TURNTABLE_FRAME_COUNT));
        let opposite = (selected - distance).rem_euclid(HOME_TURNTABLE_FRAME_COUNT);
        if opposite != *frames.last().expect("distance frame was inserted") {
            frames.push(opposite);
        }
    }
    frames.truncate(HOME_TURNTABLE_FRAME_COUNT as usize);
    frames
}

fn nearest_home_thumbnail_frame(thumbnail: &HomeTemplateThumbnail) -> Option<&HomeThumbnailFrame> {
    thumbnail.frames.values().min_by_key(|frame| {
        home_turntable_frame_distance(frame.frame_index, thumbnail.selected_frame_index)
    })
}

fn home_turntable_frame_distance(left: i32, right: i32) -> i32 {
    let direct = (left - right).abs();
    direct.min(HOME_TURNTABLE_FRAME_COUNT - direct)
}

fn button_with_disabled_reason(
    ui: &mut egui::Ui,
    enabled: bool,
    label: &str,
    disabled_reason: &str,
) -> egui::Response {
    let spec = if enabled {
        ActionSpec::enabled(label, ButtonTone::Secondary)
    } else {
        ActionSpec::disabled(label, ButtonTone::Secondary, disabled_reason)
    };
    action_button(ui, &spec)
}

fn action_spec<'a>(
    enabled: bool,
    label: &'a str,
    tone: ButtonTone,
    disabled_reason: &'a str,
) -> ActionSpec<'a> {
    if enabled {
        ActionSpec::enabled(label, tone)
    } else {
        ActionSpec::disabled(label, tone, disabled_reason)
    }
}

fn make_canvas_build_dependent_actions_enabled(view_state: &MakeCanvasViewState) -> bool {
    view_state.model_ready
        && view_state.preview_ready
        && !view_state.local_busy_visible
        && view_state.mode != MakeCanvasMode::Error
}

fn make_canvas_build_dependent_disabled_reason(view_state: &MakeCanvasViewState) -> &'static str {
    if view_state.mode == MakeCanvasMode::NoAsset {
        NEED_PROJECT_REASON
    } else if matches!(
        view_state.mode,
        MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas
    ) {
        ACTIVE_IDEA_JOB_REASON
    } else {
        ASSET_PREPARING_REASON
    }
}

fn make_canvas_controls_enabled(view_state: &MakeCanvasViewState) -> bool {
    view_state.model_ready && view_state.preview_ready && !view_state.local_busy_visible
}

fn make_canvas_candidate_actions_enabled(view_state: &MakeCanvasViewState) -> bool {
    view_state.model_ready
        && view_state.preview_ready
        && !matches!(
            view_state.mode,
            MakeCanvasMode::PreparingAsset
                | MakeCanvasMode::GeneratingWholeAssetIdeas
                | MakeCanvasMode::GeneratingFocusedPartIdeas
                | MakeCanvasMode::Error
        )
}

fn make_canvas_layout(available: egui::Vec2, view_state: &MakeCanvasViewState) -> MakeCanvasLayout {
    let available_width = available.x.max(1.0);
    let available_height = available.y.max(1.0);
    let stacked_columns = available_width < 900.0;
    let inline_ideas = make_canvas_uses_inline_ideas(view_state)
        && !view_state.material_look_tray_visible
        && !stacked_columns
        && available_width >= 1240.0;
    let content_gap = if available_width >= 1280.0 {
        14.0
    } else {
        10.0
    };
    let compact_ideas = make_canvas_uses_compact_ideas(view_state);
    let requested_tray_height = if inline_ideas
        || (!view_state.candidate_tray_visible && !view_state.material_look_tray_visible)
    {
        0.0
    } else if view_state.material_look_tray_visible {
        (available_height * 0.52).clamp(330.0, 440.0)
    } else if compact_ideas {
        (available_height * 0.13).clamp(88.0, 128.0)
    } else {
        (available_height * 0.38).clamp(240.0, 420.0)
    };
    let min_top_height = if view_state.material_look_tray_visible {
        if stacked_columns { 360.0 } else { 320.0 }
    } else if stacked_columns {
        520.0
    } else {
        390.0
    };
    let max_tray_height = (available_height - min_top_height - content_gap).max(0.0);
    let tray_height = requested_tray_height.min(max_tray_height);
    let tray_gap = if tray_height > 0.0 { content_gap } else { 0.0 };
    let top_height = (available_height - tray_height - tray_gap).max(available_height.min(260.0));

    let (stage_width, ideas_width, inspector_width) = if stacked_columns {
        (available_width, available_width, available_width)
    } else if inline_ideas {
        let desired_inspector_width = (available_width * 0.20).clamp(320.0, 380.0);
        let desired_stage_width = (available_width * 0.38).clamp(520.0, 760.0);
        let min_stage_width = 430.0;
        let min_ideas_width = 430.0;
        let min_inspector_width = 300.0;
        let mut inspector_width = desired_inspector_width.min(available_width * 0.27);
        let mut stage_width = desired_stage_width;
        let mut ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);

        if ideas_width < min_ideas_width {
            let deficit = min_ideas_width - ideas_width;
            stage_width = (stage_width - deficit).max(min_stage_width);
            ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);
        }
        if ideas_width < min_ideas_width {
            let deficit = min_ideas_width - ideas_width;
            inspector_width = (inspector_width - deficit).max(min_inspector_width);
            ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);
        }

        (stage_width, ideas_width.max(300.0), inspector_width)
    } else {
        let min_stage_width = if available_width >= 1500.0 {
            760.0
        } else {
            520.0
        };
        let max_inspector_width = (available_width - min_stage_width - content_gap).max(300.0);
        let desired_inspector_width = if available_width >= 1500.0 {
            (available_width * 0.30).clamp(420.0, 520.0)
        } else {
            (available_width * 0.34).clamp(340.0, 440.0)
        };
        let inspector_width = desired_inspector_width
            .min(max_inspector_width)
            .min(available_width - content_gap)
            .max(300.0);
        let stage_width = (available_width - inspector_width - content_gap).max(300.0);
        (stage_width, 0.0, inspector_width)
    };

    MakeCanvasLayout {
        top_height,
        tray_height,
        tray_gap,
        stage_width,
        ideas_width,
        inspector_width,
        content_gap,
        stacked_columns,
        inline_ideas,
        compact_ideas,
    }
}

fn make_canvas_uses_compact_ideas(view_state: &MakeCanvasViewState) -> bool {
    view_state.candidate_tray_state == MakeCandidateTrayState::EmptyReady
        && view_state.local_warning_message.is_none()
        && view_state.local_error_message.is_none()
}

fn make_canvas_uses_inline_ideas(view_state: &MakeCanvasViewState) -> bool {
    view_state.candidate_tray_visible
        && matches!(
            view_state.candidate_tray_state,
            MakeCandidateTrayState::GeneratingSkeletons
                | MakeCandidateTrayState::HasCandidates
                | MakeCandidateTrayState::NoCandidatesWithRecovery
                | MakeCandidateTrayState::ErrorWithRecovery
        )
}

fn make_canvas_stacked_stage_height(top_height: f32) -> f32 {
    if top_height <= 420.0 {
        return (top_height * 0.56).max(180.0);
    }
    (top_height * 0.58).clamp(240.0, top_height - 180.0)
}

fn make_stage_preview_edge(available_width: f32, available_height: f32) -> f32 {
    let vertical_budget = available_height * 0.68;
    let horizontal_budget = available_width - 18.0;
    vertical_budget.min(horizontal_budget).clamp(180.0, 520.0)
}

fn make_canvas_inspector_build_actions_visible(view_state: &MakeCanvasViewState) -> bool {
    view_state.preview_ready
        || view_state.preview_update_required
        || matches!(
            view_state.mode,
            MakeCanvasMode::ReviewingIdeas
                | MakeCanvasMode::FocusedPart
                | MakeCanvasMode::PackDrawerOpen
                | MakeCanvasMode::ExportDrawerOpen
                | MakeCanvasMode::Error
        )
}

fn candidate_previews_are_pending(
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
) -> bool {
    !candidates.is_empty()
        && candidates.iter().any(|candidate| {
            candidate.validation_label == "Preview pending"
                || candidate
                    .preview_failure
                    .as_deref()
                    .is_some_and(|reason| reason.contains("Preview rendering"))
        })
}

fn candidate_review_is_ready(
    view_state: &MakeCanvasViewState,
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
) -> bool {
    view_state.mode == MakeCanvasMode::ReviewingIdeas
        && view_state.preview_ready
        && view_state.selected_comparison_visible
        && candidates.iter().any(|candidate| {
            candidate.selected
                && candidate.selectable
                && candidate.preview_failure.is_none()
                && !candidate.rgba8.is_empty()
        })
}

fn tab_for_workflow_step(index: usize) -> FoundryTab {
    match index {
        1 => FoundryTab::Home,
        2 => FoundryTab::Make,
        _ => FoundryTab::Home,
    }
}

fn read_screenshot_scenario() -> Option<ScreenshotScenario> {
    let path = env::temp_dir().join("shape-lab-screenshot-scenario.txt");
    let value = fs::read_to_string(path).ok()?;
    match value.trim() {
        "make_initial_box" => Some(ScreenshotScenario::MakeInitialBox),
        "generating_box_ideas" | "generating_whole_asset_ideas" => {
            Some(ScreenshotScenario::GeneratingBoxIdeas)
        }
        "generated_box_ideas" | "generated_whole_asset_ideas" => {
            Some(ScreenshotScenario::GeneratedBoxIdeas)
        }
        "selected_comparison" => Some(ScreenshotScenario::SelectedComparison),
        "adjusted_box_control" => Some(ScreenshotScenario::AdjustedBoxControl),
        "pack_drawer" => Some(ScreenshotScenario::PackDrawer),
        "export_drawer" => Some(ScreenshotScenario::ExportDrawer),
        _ => None,
    }
}

fn read_screenshot_fixture_catalog() -> shape_foundry_catalog::FoundryFixtureCatalog {
    shape_foundry_catalog::box_primitive::fixture_catalog()
}

fn screenshot_scenario_assertion(
    scenario: ScreenshotScenario,
    view_state: &MakeCanvasViewState,
) -> Result<(), String> {
    match scenario {
        ScreenshotScenario::MakeInitialBox => {
            require_screenshot_state(view_state.mode == MakeCanvasMode::Ready, scenario, "Ready")?;
            require_screenshot_state(view_state.model_ready, scenario, "model_ready")?;
            require_screenshot_state(view_state.preview_ready, scenario, "preview_ready")
        }
        ScreenshotScenario::GeneratingBoxIdeas => require_screenshot_state(
            view_state.local_busy_visible
                && view_state.mode == MakeCanvasMode::GeneratingWholeAssetIdeas,
            scenario,
            "local_busy_visible whole-asset generation",
        ),
        ScreenshotScenario::GeneratedBoxIdeas => require_screenshot_state(
            view_state.candidate_tray_visible && view_state.selected_comparison_visible,
            scenario,
            "candidate_tray_visible with rendered selected comparison",
        ),
        ScreenshotScenario::SelectedComparison => require_screenshot_state(
            view_state.selected_comparison_visible,
            scenario,
            "selected_comparison_visible",
        ),
        ScreenshotScenario::AdjustedBoxControl => require_screenshot_state(
            view_state.mode == MakeCanvasMode::Ready
                && view_state.model_ready
                && view_state.preview_ready,
            scenario,
            "ready adjusted box control",
        ),
        ScreenshotScenario::PackDrawer => require_screenshot_state(
            view_state.pack_drawer_visible,
            scenario,
            "pack_drawer_visible",
        ),
        ScreenshotScenario::ExportDrawer => require_screenshot_state(
            view_state.export_drawer_visible,
            scenario,
            "export_drawer_visible",
        ),
    }
}

fn require_screenshot_state(
    passed: bool,
    scenario: ScreenshotScenario,
    requirement: &str,
) -> Result<(), String> {
    if passed {
        Ok(())
    } else {
        Err(format!("{scenario:?} missing {requirement}"))
    }
}

fn record_screenshot_state_assertion(
    scenario: ScreenshotScenario,
    view_state: &MakeCanvasViewState,
    failure: Option<&str>,
) {
    let path = env::temp_dir().join("shape-lab-screenshot-state-assertions.txt");
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let result = failure.unwrap_or("PASS");
    let _ = writeln!(
        file,
        "{scenario:?}: {result}; mode={:?}; asset={}; busy={}; tray={}; comparison={}; focus={}; pack={}; export={}",
        view_state.mode,
        view_state.asset_name,
        view_state.local_busy_visible,
        view_state.candidate_tray_visible,
        view_state.selected_comparison_visible,
        view_state.focused_part_label.as_deref().unwrap_or("None"),
        view_state.pack_drawer_visible,
        view_state.export_drawer_visible
    );
}

#[cfg(test)]
fn screenshot_part_group(
    state: &FoundryAppState,
    group_id: &str,
) -> Option<directions::DirectionPartGroup> {
    state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)?
        .into_iter()
        .find(|group| group.group_id == group_id && group.focusable)
}

fn make_whole_asset_candidate_request(
    state: &FoundryAppState,
    shape_only: bool,
    mode: FoundryCandidateMode,
) -> FoundryAppCommand {
    let seed = state.document.as_ref().map_or(0, |document| document.seed);
    FoundryAppCommand::RequestCandidates(FoundryCandidateRequest {
        seed,
        proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
        result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode,
        strategy_id: None,
        preference_profile: None,
        variation_intent: if shape_only {
            VariationIntent::whole_asset_shape()
        } else {
            VariationIntent::complete_look()
        },
    })
}

fn project_file_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|stem| {
            stem.trim_end_matches(".shapelab-foundry")
                .replace(['-', '_'], " ")
        })
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| "Shape Lab Project".to_owned())
}

fn asset_title_from_id(document_id: &str) -> &'static str {
    match document_id {
        id if id.contains("box-primitive") => "Box Primitive",
        _ => "Shape Lab Project",
    }
}

fn product_safe_status(status: &str) -> String {
    if status.starts_with("Saved ") {
        "Project saved".to_owned()
    } else if status.starts_with("Loaded ") {
        "Project loaded".to_owned()
    } else if status.starts_with("Exported ") && status.contains(" pack member") {
        "Pack export complete".to_owned()
    } else if status.starts_with("Exported ") {
        "Export complete".to_owned()
    } else if status.contains('\\') || status.contains('/') {
        "Project path needs attention".to_owned()
    } else if crate::foundry::ui::copy::first_forbidden_product_term(status).is_some() {
        "Project needs attention".to_owned()
    } else {
        status.to_owned()
    }
}

fn open_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab Foundry", &["json"])
        .pick_file()
}

fn save_foundry_project_file() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .add_filter("Shape Lab Foundry", &["json"])
        .set_file_name("foundry-project.shapelab-foundry.json")
        .save_file()
        .map(normalize_foundry_project_path)
}

fn select_pack_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Foundry Pack")
        .pick_folder()
}

fn select_asset_export_dir() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Export Current Foundry Asset")
        .pick_folder()
}

fn normalize_foundry_project_path(path: PathBuf) -> PathBuf {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return path;
    };
    if file_name.ends_with(FOUNDRY_PROJECT_FILE_SUFFIX) {
        return path;
    }

    let base_name = file_name.strip_suffix(".json").unwrap_or(file_name);
    path.with_file_name(format!("{base_name}{FOUNDRY_PROJECT_FILE_SUFFIX}"))
}

fn profile_description(slug: &str) -> &'static str {
    match slug {
        "box-primitive" => HOME_SUBTITLE,
        _ => "A Box Primitive starting point ready for box idea generation.",
    }
}

#[derive(Clone)]
struct ProductHomeProfile {
    label: String,
    fixture: FoundryFixtureCatalog,
    family_id: String,
    family_name: String,
    style_name: String,
    category_chips: Vec<String>,
}

#[cfg(test)]
struct ProductHomeProfileGroup {
    family_id: String,
    family_name: String,
    profiles: Vec<ProductHomeProfile>,
}

fn product_home_profiles(developer_preview_enabled: bool) -> Vec<ProductHomeProfile> {
    let cards = built_in_kit_card_views();
    curated_fixture_catalogs_with_labels(developer_preview_enabled)
        .into_iter()
        .filter_map(|(_label, fixture)| {
            let card = cards
                .iter()
                .find(|card| card.source_profile_slug.as_deref() == Some(fixture.slug.as_str()))?;
            Some(ProductHomeProfile {
                label: card.display_name.clone(),
                fixture,
                family_id: card.family_id.clone(),
                family_name: card.family_name.clone(),
                style_name: card.style_name.clone(),
                category_chips: card.category_chips.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
fn product_home_profile_groups(profiles: Vec<ProductHomeProfile>) -> Vec<ProductHomeProfileGroup> {
    let mut groups = BTreeMap::<String, (String, Vec<ProductHomeProfile>)>::new();
    for profile in profiles {
        groups
            .entry(profile.family_id.clone())
            .or_insert_with(|| (profile.family_name.clone(), Vec::new()))
            .1
            .push(profile);
    }
    let mut groups = groups
        .into_iter()
        .map(
            |(family_id, (family_name, profiles))| ProductHomeProfileGroup {
                family_id,
                family_name,
                profiles,
            },
        )
        .collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        left.family_name
            .cmp(&right.family_name)
            .then_with(|| left.family_id.cmp(&right.family_id))
    });
    groups
}

fn current_preview_pixels_for_context(ctx: &egui::Context) -> u32 {
    current_preview_pixels_for_scale(ctx.pixels_per_point())
}

fn current_preview_pixels_for_scale(pixels_per_point: f32) -> u32 {
    let scale = pixels_per_point.max(1.0);
    ((DEFAULT_PREVIEW_PIXELS as f32 * scale).ceil() as u32)
        .clamp(DEFAULT_PREVIEW_PIXELS, MAX_CURRENT_PREVIEW_PIXELS)
}

pub(crate) fn default_product_home_profile_count() -> usize {
    product_home_profiles(false).len()
}

pub(crate) fn developer_preview_product_home_profile_count() -> usize {
    product_home_profiles(true).len()
}

pub(crate) fn installed_product_kit_count() -> usize {
    built_in_kit_card_views().len()
}

fn developer_preview_catalog_enabled() -> bool {
    env::var(PREVIEW_CATALOG_ENV_VAR).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub(crate) fn product_visible_strings_for_default_shell() -> Vec<&'static str> {
    let mut strings = vec![
        "Shape Lab",
        "Visual Foundry",
        "Choose",
        "Make",
        "Export",
        "Start with Box Primitive",
        "Project",
        "Open Project",
        "Save Project",
        "Save Project As",
        "Start Another Asset",
        "History",
        "Recent Projects",
        "Start",
        ACTION_TRY_BOX_IDEAS,
        "Found 4 clear ideas",
        "Rejected 2 that looked too similar",
        ACTION_CHOOSE_DIRECTION,
        "Add to Pack",
        "Open Pack",
        "Open Export",
        "Trying ideas...",
        ACTION_CHOOSE_TEMPLATE,
        "Preparing model",
        "Rendering preview",
        "Ready to try ideas",
        PREVIEW_UPDATING_REASON,
        PREPARATION_TIMEOUT_MESSAGE,
        ASSET_PREPARING_REASON,
        STALE_RESULT_WARNING,
        "Try again when you are ready.",
        "Current Asset",
        "Compare",
        "What changed",
        "No clear ideas yet",
        "No clear ideas survived",
        "The search found changes that were hidden or too subtle.",
        "No ideas yet",
        "Ideas",
        "Save",
        "Undo",
        "Choose Box Primitive",
        "Not saved",
        "Saved",
        "Unsaved",
        "Unsaved changes",
        "Ready",
        "Working",
        "Model ready",
        "Preparing model",
        "Choose Box Primitive",
        "Ready",
        "Preview available",
        "Preparing",
        "Preview",
        "Pack: 0 assets",
        "Export complete",
        "Pack export complete",
        "Current asset ready",
        "Needs a model first",
        HOME_SUBTITLE,
        HOME_CONTROL_COPY,
        "Start with Box Primitive.",
        "Preview building",
        "No matching starting point",
        "Make asset",
        "Use the model as the workspace: try box ideas, tune controls, and compare.",
        "Try box ideas, adjust box, Add to Pack, or Export.",
        "Current Asset",
        "Ideas",
        "Trying ideas",
        "Current asset",
        "Choose Box Primitive or open a project before making changes.",
        "Preview could not be rendered for this idea.",
        "Preview this idea before using it.",
        "Project history",
        "Review previous project steps and branch from a saved point.",
        "saved step(s)",
        "Project step",
        "Tune the main box controls and lock the settings you want to keep.",
        "Choose an asset first",
        "Choose Box Primitive or open a project before adjusting.",
        "Make it yours",
        "No quick controls yet",
        "This asset has no quick controls yet.",
        "Preview",
        "More options",
        "This option is not available right now.",
        "This control is locked.",
        "Export ready",
        BOX_PRIMITIVE_EXPORT_TITLE,
        BOX_PRIMITIVE_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        "Current Asset",
        "Export options",
        "Pack members",
        "Export this asset here, or export the prepared pack from the Pack drawer.",
        "Export the current asset as an individual result.",
        "Pack preview",
        "Collect coherent variants before exporting a set.",
        "Add Current Asset",
        "Export Pack",
        "Pack is empty",
        "Add the current asset to start a pack.",
        "assets in pack",
        "Pack asset",
        "Add assets to export",
        "Needs attention",
        "Contact sheet",
        "Pack needs attention before export.",
        "Resolve pack warnings before export.",
        "All assets are ready for pack export.",
        NEED_PROJECT_REASON,
        NEED_SAVE_LOCATION_REASON,
        NEED_MODEL_REASON,
        NEED_HISTORY_REASON,
        NEED_DIRECTION_REASON,
        NEED_RESET_REASON,
        NEED_PACK_MEMBER_REASON,
        "This model has no quick controls yet.",
        "Prepare the current asset before exporting.",
        "No pack workspace is open.",
        "Preview is being prepared.",
        "Primary control",
        "Box Primitive is not available",
        "Open a saved project, or enable the Box Primitive baseline.",
        "Choose Box Primitive or open a project before exporting.",
        "Choose Box Primitive or open a project before starting a pack.",
    ];
    strings.extend(RENDERED_ACTION_LABELS);
    for step in WORKFLOW_STEPS {
        strings.push(step.label);
        strings.push(step.detail);
    }
    strings
}

pub(crate) fn rendered_action_labels_for_default_shell() -> &'static [&'static str] {
    &RENDERED_ACTION_LABELS
}

pub(crate) fn core_make_action_specs_for_default_shell() -> Vec<ActionSpec<'static>> {
    vec![
        ActionSpec::enabled(ACTION_START, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_TRY_BOX_IDEAS, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_GENERATING_IDEAS, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_TRY_AGAIN, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_RETRY_PREPARATION, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_UPDATE_PREVIEW, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_CHOOSE_DIRECTION, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_UNLOCK_CONTROLS, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_RESET, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_CHOOSE_ANOTHER_TEMPLATE, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_ADD_TO_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_OPEN_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_EXPORT, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_OPEN_EXPORT, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_CLOSE_DRAWER, ButtonTone::Secondary),
    ]
}

fn product_control_summary(
    control: &crate::foundry::view_model::FoundryControlView,
) -> &'static str {
    if control.options.len() > 1 {
        "Whole-model options"
    } else {
        "Primary control"
    }
}

fn product_card<R>(
    ui: &mut egui::Ui,
    selected: bool,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let tokens = VisualFoundryTokens::dark();
    let stroke_color = if selected {
        tokens.colors.accent_hover
    } else {
        tokens.colors.stroke
    };
    egui::Frame::new()
        .fill(if selected {
            tokens.colors.accent_soft
        } else {
            tokens.colors.panel
        })
        .stroke(egui::Stroke::new(1.0, stroke_color))
        .corner_radius(egui::CornerRadius::same(tokens.radius.lg as u8))
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, add_contents)
}

fn product_stage<R>(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let tokens = VisualFoundryTokens::dark();
    egui::Frame::new()
        .fill(tokens.colors.center_bg)
        .stroke(egui::Stroke::new(1.0, tokens.colors.stroke))
        .corner_radius(egui::CornerRadius::same(tokens.radius.lg as u8))
        .inner_margin(egui::Margin::symmetric(18, 16))
        .show(ui, |ui| ui.vertical_centered(|ui| add_contents(ui)).inner)
}

struct MakeCanvasBannerContext<'a> {
    mode: &'a MakeCanvasMode,
    asset_name: &'a str,
    preparation_phase: MakePreparationPhase,
    preparation_timed_out: bool,
    idea_generation_timed_out: bool,
    preview_updating: bool,
    candidate_previews_pending: bool,
    local_busy_label: &'a Option<String>,
    active_group: Option<&'a directions::DirectionPartGroup>,
    candidate_output: Option<&'a FoundryCandidateOutput>,
    local_warning_message: Option<&'a str>,
    local_error_message: Option<&'a str>,
    box_primitive_baseline: bool,
}

fn make_canvas_local_banner(context: MakeCanvasBannerContext<'_>) -> (String, String, BannerTone) {
    let MakeCanvasBannerContext {
        mode,
        asset_name,
        preparation_phase,
        preparation_timed_out,
        idea_generation_timed_out,
        preview_updating,
        candidate_previews_pending,
        local_busy_label,
        active_group,
        candidate_output,
        local_warning_message,
        local_error_message,
        box_primitive_baseline,
    } = context;
    if let Some(message) = local_warning_message {
        return (
            make_canvas_warning_title(message).to_owned(),
            format!("{message} Try again when you are ready."),
            BannerTone::Warning,
        );
    }
    if let Some(message) = local_error_message {
        return (
            "Asset needs attention".to_owned(),
            message.to_owned(),
            BannerTone::Error,
        );
    }
    if matches!(mode, MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart)
        && candidate_output.is_some_and(|output| output.candidates.is_empty())
    {
        let (title, message) = no_candidates_recovery_copy(active_group, candidate_output);
        return (title, message, BannerTone::Warning);
    }

    match mode {
        MakeCanvasMode::NoAsset => (
            "Choose an asset".to_owned(),
            "Choose Box Primitive or open a project before making changes.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::PreparingAsset => {
            let message = if preparation_timed_out {
                PREPARATION_TIMEOUT_MESSAGE.to_owned()
            } else if preview_updating {
                PREVIEW_UPDATING_REASON.to_owned()
            } else {
                preparation_phase.label().to_owned()
            };
            (format!("Preparing {asset_name}"), message, BannerTone::Info)
        }
        MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas => {
            let message = if idea_generation_timed_out {
                IDEA_GENERATION_TIMEOUT_MESSAGE.to_owned()
            } else {
                local_busy_label
                    .clone()
                    .unwrap_or_else(|| "Trying ideas from the current asset...".to_owned())
            };
            ("Trying ideas".to_owned(), message, BannerTone::Info)
        }
        MakeCanvasMode::ReviewingIdeas if candidate_previews_pending => (
            "Rendering previews".to_owned(),
            "Candidate shells are ready. Use an idea after its preview renders.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::ReviewingIdeas => (
            if box_primitive_baseline {
                "Box ideas ready".to_owned()
            } else {
                "Ideas ready".to_owned()
            },
            if box_primitive_baseline {
                "Use this idea, or reject it.".to_owned()
            } else {
                "Compare the selected idea, then use it or reject it.".to_owned()
            },
            BannerTone::Success,
        ),
        MakeCanvasMode::PackDrawerOpen => (
            "Pack drawer open".to_owned(),
            "Review pack members or export the pack.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::ExportDrawerOpen => (
            "Export drawer open".to_owned(),
            "Choose an export option when readiness is clear.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::Error => (
            "Asset needs attention".to_owned(),
            "The current asset needs attention.".to_owned(),
            BannerTone::Error,
        ),
        MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart => {
            if box_primitive_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try box ideas, adjust box, Add to Pack, or Export.".to_owned(),
                    BannerTone::Success,
                )
            } else {
                (
                    "Ready to try ideas".to_owned(),
                    "Try ideas, focus a part, or tune controls.".to_owned(),
                    BannerTone::Success,
                )
            }
        }
    }
}

fn no_candidates_recovery_copy(
    active_group: Option<&directions::DirectionPartGroup>,
    candidate_output: Option<&FoundryCandidateOutput>,
) -> (String, String) {
    let reason = no_candidates_reason_copy(candidate_output);
    if let Some(group) = active_group {
        let part = singular_part_copy(&group.label).to_ascii_lowercase();
        let message =
            format!("No clear {part} ideas survived. {reason} Try again or unlock controls.");
        return ("No clear focused ideas survived".to_owned(), message);
    }

    (
        "No clear ideas survived".to_owned(),
        format!("{reason} Try again or adjust the current asset."),
    )
}

fn make_canvas_warning_title(message: &str) -> &'static str {
    if message == CANCELED_IDEA_SEARCH_WARNING {
        "Idea search canceled"
    } else {
        "Older result ignored"
    }
}

fn no_candidates_reason_copy(candidate_output: Option<&FoundryCandidateOutput>) -> &'static str {
    let Some(output) = candidate_output else {
        return "The search did not find a clear visible change.";
    };
    if output.diagnostics.wrong_scope_rejections > 0 {
        "The clearest changes affected something outside the focused part."
    } else if output.diagnostics.hidden_internal_rejections > 0 {
        "The search found changes that were hidden or too subtle."
    } else if output.diagnostics.duplicate_looking_rejections > 0 {
        "The search found ideas that looked too similar."
    } else {
        "The search did not find a clear visible change."
    }
}

fn make_canvas_mode_summary(view_state: &MakeCanvasViewState) -> &'static str {
    match view_state.mode {
        MakeCanvasMode::NoAsset => "Choose Box Primitive first.",
        MakeCanvasMode::PreparingAsset if view_state.preparation_timed_out => {
            PREPARATION_TIMEOUT_MESSAGE
        }
        MakeCanvasMode::PreparingAsset if view_state.preview_updating => PREVIEW_UPDATING_REASON,
        MakeCanvasMode::PreparingAsset => ASSET_PREPARING_REASON,
        MakeCanvasMode::GeneratingWholeAssetIdeas => "Trying ideas from the current asset.",
        MakeCanvasMode::GeneratingFocusedPartIdeas => "Trying ideas for the focused part.",
        MakeCanvasMode::ReviewingIdeas if view_state.box_primitive_baseline => {
            "Use this idea, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas => "Compare the selected idea against the current asset.",
        MakeCanvasMode::FocusedPart => "This part is focused. Try ideas, lock it, or clear focus.",
        MakeCanvasMode::PackDrawerOpen => "The pack drawer is open.",
        MakeCanvasMode::ExportDrawerOpen => "The export drawer is open.",
        MakeCanvasMode::Ready if view_state.box_primitive_baseline => {
            "Try box ideas or adjust box."
        }
        MakeCanvasMode::Ready => "Try ideas, focus a part, or tune controls.",
        MakeCanvasMode::Error => "The current asset needs attention.",
    }
}

fn make_canvas_next_action_hint(
    mode: &MakeCanvasMode,
    focused_part_label: Option<&str>,
    selected_comparison_visible: bool,
    box_primitive_baseline: bool,
) -> String {
    match (mode, focused_part_label, selected_comparison_visible) {
        (MakeCanvasMode::NoAsset, _, _) => "Start with Box Primitive from Choose.".to_owned(),
        (MakeCanvasMode::PreparingAsset, _, _) => {
            "Wait for the model and preview to finish preparing.".to_owned()
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, Some(part), _) => {
            format!(
                "Watch this area for new {} ideas.",
                singular_part_copy(part).to_ascii_lowercase()
            )
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, None, _) => {
            "Watch this area for new focused ideas.".to_owned()
        }
        (MakeCanvasMode::GeneratingWholeAssetIdeas, _, _) => {
            "Watch this area for new ideas.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if box_primitive_baseline => {
            "Use this idea, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) => {
            "Compare the selected idea, then use it or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, false) => {
            "Select an idea to compare it against the current asset.".to_owned()
        }
        (MakeCanvasMode::FocusedPart, Some(part), _) => {
            format!(
                "Try {} ideas, lock this part, or clear focus.",
                singular_part_copy(part).to_ascii_lowercase()
            )
        }
        (MakeCanvasMode::FocusedPart, None, _) => {
            "Try focused ideas, lock this part, or clear focus.".to_owned()
        }
        (MakeCanvasMode::PackDrawerOpen, _, _) => {
            "Review pack members or export the pack.".to_owned()
        }
        (MakeCanvasMode::ExportDrawerOpen, _, _) => {
            "Choose an export option when readiness is clear.".to_owned()
        }
        (MakeCanvasMode::Error, _, _) => "Resolve the local issue before continuing.".to_owned(),
        (MakeCanvasMode::Ready, _, _) if box_primitive_baseline => {
            "Try box ideas, adjust box, Add to Pack, or Export.".to_owned()
        }
        (MakeCanvasMode::Ready, _, _) => {
            "Try ideas, focus a part, add to pack, or export.".to_owned()
        }
    }
}

fn draw_busy_overlay(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    let overlay = rect.shrink2(egui::vec2(18.0, 48.0));
    ui.painter().rect_filled(
        overlay,
        egui::CornerRadius::same(10),
        egui::Color32::from_rgba_premultiplied(8, 13, 18, 190),
    );
    ui.painter().rect_stroke(
        overlay,
        egui::CornerRadius::same(10),
        egui::Stroke::new(1.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        overlay.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(18.0),
        colors.text,
    );
}

fn draw_focus_callout(ui: &egui::Ui, rect: egui::Rect, label: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    let focus_rect = focus_callout_rect(rect, label);
    ui.painter().rect_stroke(
        focus_rect,
        egui::CornerRadius::same(10),
        egui::Stroke::new(2.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    let tag = egui::Rect::from_min_size(
        egui::pos2(focus_rect.left(), focus_rect.top() - 30.0),
        egui::vec2((label.len() as f32 * 8.0).clamp(74.0, 140.0), 24.0),
    );
    ui.painter()
        .rect_filled(tag, egui::CornerRadius::same(8), colors.accent_soft);
    ui.painter().rect_stroke(
        tag,
        egui::CornerRadius::same(8),
        egui::Stroke::new(1.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        tag.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        colors.text,
    );
}

fn focus_callout_rect(rect: egui::Rect, label: &str) -> egui::Rect {
    let center = rect.center();
    let width = (rect.width() * 0.24).clamp(96.0, 190.0);
    let height = (rect.height() * 0.18).clamp(68.0, 130.0);
    let lower = label.to_ascii_lowercase();
    let offset = if lower.contains("handle") {
        egui::vec2(rect.width() * 0.20, rect.height() * 0.06)
    } else if lower.contains("vent") {
        egui::vec2(rect.width() * 0.04, -rect.height() * 0.15)
    } else if lower.contains("body") {
        egui::vec2(0.0, 0.0)
    } else {
        egui::vec2(-rect.width() * 0.10, -rect.height() * 0.02)
    };
    egui::Rect::from_center_size(center + offset, egui::vec2(width, height))
}

fn workflow_tab_button(
    ui: &mut egui::Ui,
    index: usize,
    label: &str,
    detail: &str,
    selected: bool,
) -> egui::Response {
    let tokens = VisualFoundryTokens::dark();
    let colors = tokens.colors;
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if selected {
        colors.text
    } else {
        colors.text_muted
    };
    let title = format!("{index}. {label}");
    let response = egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.set_min_width(124.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).color(text).strong());
                ui.label(RichText::new(detail).color(colors.text_subtle).small());
            });
        })
        .response
        .interact(egui::Sense::click());
    response.on_hover_text(detail)
}

fn variation_mode_button(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    enabled: bool,
    disabled_reason: &str,
) -> egui::Response {
    let tokens = VisualFoundryTokens::dark();
    let colors = tokens.colors;
    let fill = if selected {
        colors.accent_soft
    } else if enabled {
        colors.panel_elevated
    } else {
        colors.panel_subtle
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if enabled {
        colors.text
    } else {
        colors.text_subtle
    };
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).color(text).strong())
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(8))
            .min_size(egui::vec2(118.0, 36.0)),
    );
    if enabled {
        response
    } else {
        response.on_disabled_hover_text(disabled_reason)
    }
}

fn product_empty_state(ui: &mut egui::Ui, title: &str, message: &str) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 190.0),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            product_card(ui, false, |ui| {
                let colors = VisualFoundryTokens::dark().colors;
                ui.set_min_height(150.0);
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(28.0);
                    ui.label(RichText::new(title).color(colors.text).size(17.0).strong());
                    ui.add(
                        egui::Label::new(RichText::new(message).color(colors.text_muted)).wrap(),
                    );
                });
            });
        },
    );
}

fn product_compact_empty_state(ui: &mut egui::Ui, title: &str, message: &str) {
    product_card(ui, false, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.set_min_height(64.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(title).color(colors.text).strong());
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(RichText::new(message).color(colors.text_muted).small()).wrap(),
            );
        });
    });
}

fn compact_section_header(ui: &mut egui::Ui, eyebrow: &str, title: &str, subtitle: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(eyebrow.to_ascii_uppercase())
                .color(colors.accent_hover)
                .small(),
        );
        ui.label(RichText::new(title).color(colors.text).strong());
        ui.label(RichText::new(subtitle).color(colors.text_muted).small());
    });
}

fn product_panel_message(message: &str, fallback: &str) -> String {
    let trimmed = message.trim();
    let lowercase = trimmed.to_ascii_lowercase();
    let raw_markers = [
        "\\",
        "/",
        "::",
        "_",
        "members.",
        "document",
        "catalog",
        "schema",
        "validation",
        "diagnostic",
        "recipe",
    ];
    if trimmed.is_empty()
        || crate::foundry::ui::copy::first_forbidden_product_term(trimmed).is_some()
        || raw_markers.iter().any(|marker| lowercase.contains(marker))
    {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn direction_board_count_label(count: usize, box_primitive_baseline: bool) -> String {
    if count == 0 {
        if box_primitive_baseline {
            "Try box ideas.".to_owned()
        } else {
            "Try ideas from the current asset.".to_owned()
        }
    } else {
        format!("Found {count} clear ideas")
    }
}

fn make_busy_asset_noun(asset_name: &str) -> &'static str {
    let lower = asset_name.to_ascii_lowercase();
    if lower.contains("box") {
        "box"
    } else {
        "asset"
    }
}

fn singular_part_copy(label: &str) -> &str {
    match label {
        "Body" => "Body",
        other => other.trim_end_matches('s'),
    }
}

fn singular_title_case_part_label(label: &str) -> String {
    singular_part_copy(label).to_owned()
}

pub(crate) fn direction_mode_actions_for_panel() -> Vec<directions::DirectionModeAction> {
    directions::direction_mode_actions(None, 0, None)
}

pub(crate) fn direction_variation_mode_actions_for_panel(
    document: Option<&FoundryAssetDocument>,
) -> Vec<directions::DirectionVariationModeAction> {
    let Some(document) = document else {
        return directions::direction_variation_mode_actions(
            &VariationIntent::default(),
            0,
            None,
            None,
            &[],
        );
    };
    let part_groups = directions::direction_part_groups_for_document(document);
    let surface_capability =
        built_in_surface_capability_for_profile(&document.customizer_profile_ref.stable_id);
    directions::direction_variation_mode_actions(
        &document.variation_state.intent,
        document.seed,
        None,
        Some(&surface_capability),
        &part_groups,
    )
}

pub(crate) fn default_app_launches_on_home() -> bool {
    let app = FoundryDesktopApp::default();
    app.tab == FoundryTab::Home && app.state.document.is_none()
}

fn home_browser_width(available_width: f32) -> f32 {
    available_width.clamp(320.0, 420.0)
}

fn show_home_browser_panel(
    ui: &mut egui::Ui,
    profiles: &[ProductHomeProfile],
    search_query: &mut String,
    filter: &mut HomeTemplateFilter,
    selected_slug: &mut Option<String>,
) {
    let colors = VisualFoundryTokens::dark().colors;
    let single_profile_mode = profiles.len() == 1 && HOME_TEMPLATE_FILTERS.is_empty();
    product_card(ui, false, |ui| {
        ui.set_min_height(ui.available_height().max(420.0));
        if single_profile_mode {
            normalize_home_selection(profiles, "", HomeTemplateFilter::All, selected_slug);
            section_header(
                ui,
                SectionHeaderSpec {
                    eyebrow: "Choose",
                    title: "Start with Box Primitive.",
                    subtitle: Some(HOME_SUBTITLE),
                },
            );
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(HOME_CONTROL_COPY)
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
            if let Some(profile) = profiles.first() {
                ui.add_space(18.0);
                ui.label(
                    RichText::new(profile.label.as_str())
                        .color(colors.text)
                        .size(18.0)
                        .strong(),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(profile_description(&profile.fixture.slug))
                            .color(colors.text_muted),
                    )
                    .wrap(),
                );
            }
        } else {
            section_header(
                ui,
                SectionHeaderSpec {
                    eyebrow: "Choose",
                    title: "Choose what to make",
                    subtitle: Some(HOME_SUBTITLE),
                },
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new("Choose the Box Primitive starting point below.")
                    .color(colors.text_muted)
                    .small(),
            );
            ui.add_space(12.0);
            let response = ui.add_sized(
                [ui.available_width(), 32.0],
                egui::TextEdit::singleline(search_query)
                    .hint_text("Search starting point...")
                    .desired_width(f32::INFINITY),
            );
            if response.changed() {
                *selected_slug =
                    default_filtered_home_profile_slug(profiles, search_query, *filter);
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for option in HOME_TEMPLATE_FILTERS {
                    if home_filter_button(ui, option, *filter).clicked() {
                        *filter = option;
                        *selected_slug =
                            default_filtered_home_profile_slug(profiles, search_query, *filter);
                    }
                }
            });
            normalize_home_selection(profiles, search_query, *filter, selected_slug);
            ui.add_space(12.0);

            let filtered_indices = filtered_home_profile_indices(profiles, search_query, *filter);
            let count_label = home_profile_count_label(filtered_indices.len());
            ui.label(
                RichText::new(count_label)
                    .color(colors.text_subtle)
                    .small()
                    .strong(),
            );
            ui.add_space(6.0);
            egui::ScrollArea::vertical()
                .id_salt("foundry_home_template_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if filtered_indices.is_empty() {
                        ui.label(
                            RichText::new("No matching starting point")
                                .color(colors.text_muted)
                                .small(),
                        );
                    }
                    for index in filtered_indices {
                        let profile = &profiles[index];
                        let selected =
                            selected_slug.as_deref() == Some(profile.fixture.slug.as_str());
                        if show_home_template_row(ui, profile, selected).clicked() {
                            *selected_slug = Some(profile.fixture.slug.clone());
                        }
                        ui.add_space(6.0);
                    }
                });
        }
    });
}

fn home_filter_button(
    ui: &mut egui::Ui,
    option: HomeTemplateFilter,
    selected: HomeTemplateFilter,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let is_selected = option == selected;
    let fill = if is_selected {
        colors.accent_soft
    } else {
        colors.panel_elevated
    };
    let stroke = if is_selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if is_selected {
        colors.text
    } else {
        colors.text_muted
    };
    ui.add(
        egui::Button::new(RichText::new(option.label()).color(text))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(egui::vec2(58.0, 28.0)),
    )
}

fn show_home_template_row(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    selected: bool,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel_subtle
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(profile.label.as_str())
                    .color(colors.text)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(profile_description(&profile.fixture.slug))
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
        })
        .response
        .interact(egui::Sense::click())
}

fn show_home_selected_template_stage(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    home_thumbnails: &mut HomeThumbnailCoordinator,
    texture_cache: &mut FoundryTextureCache,
) -> egui::Response {
    let mut action = None;
    product_stage(ui, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.set_min_height(ui.available_height().max(480.0));
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(profile.label.as_str())
                    .color(colors.text)
                    .size(18.0)
                    .strong(),
            );
        });
        ui.add_space(8.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{} {}",
                    profile_description(&profile.fixture.slug),
                    HOME_CONTROL_COPY
                ))
                .color(colors.text_muted),
            )
            .wrap(),
        );
        ui.add_space(14.0);
        let preview_height = (ui.available_height() - 98.0).clamp(320.0, 620.0);
        show_home_selected_model_preview(
            ui,
            profile,
            home_thumbnails,
            texture_cache,
            preview_height,
        );
        ui.add_space(14.0);
        action = Some(start_template_button(ui));
    });
    action.expect("selected template stage always renders start action")
}

fn show_home_selected_model_preview(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    home_thumbnails: &mut HomeThumbnailCoordinator,
    texture_cache: &mut FoundryTextureCache,
    height: f32,
) {
    let colors = VisualFoundryTokens::dark().colors;
    let width = ui.available_width().max(260.0);
    home_thumbnails.ensure(profile);
    home_thumbnails.prewarm_turntable(&profile.fixture.slug);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
    if response.dragged_by(egui::PointerButton::Secondary)
        && home_thumbnails.orbit_thumbnail(&profile.fixture.slug, response.drag_delta())
    {
        ui.ctx().request_repaint();
    }
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), colors.panel_subtle);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, colors.stroke_strong),
        egui::StrokeKind::Inside,
    );

    if let Some(frame) = home_thumbnails.thumbnail(&profile.fixture.slug)
        && let Some(size) =
            scaled_preview_size(frame.width, frame.height, (height - 24.0).min(width - 24.0))
    {
        let preview_id = format!(
            "home-template-{}-frame-{}",
            profile.fixture.slug, frame.frame_index
        );
        let texture = texture_cache.texture(
            ui.ctx(),
            &preview_id,
            None,
            &frame.rgba8,
            frame.width,
            frame.height,
        );
        let image_rect = egui::Rect::from_center_size(rect.center(), size);
        ui.painter().image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Preview building",
            egui::FontId::proportional(13.0),
            colors.text_muted,
        );
    }
}

fn home_profile_count_label(count: usize) -> String {
    match count {
        1 => "1 starting point".to_owned(),
        count => format!("{count} starting points"),
    }
}

fn selected_home_profile<'a>(
    profiles: &'a [ProductHomeProfile],
    selected_slug: &Option<String>,
) -> Option<&'a ProductHomeProfile> {
    selected_slug
        .as_deref()
        .and_then(|slug| profiles.iter().find(|profile| profile.fixture.slug == slug))
}

fn normalize_home_selection(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
    selected_slug: &mut Option<String>,
) {
    if selected_slug
        .as_deref()
        .is_some_and(|slug| home_profile_is_visible(profiles, slug, search_query, filter))
    {
        return;
    }
    *selected_slug = default_filtered_home_profile_slug(profiles, search_query, filter);
}

fn default_home_profile_slug(profiles: &[ProductHomeProfile]) -> Option<String> {
    profiles.first().map(|profile| profile.fixture.slug.clone())
}

fn default_filtered_home_profile_slug(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
) -> Option<String> {
    filtered_home_profile_indices(profiles, search_query, filter)
        .first()
        .map(|index| profiles[*index].fixture.slug.clone())
}

fn home_profile_is_visible(
    profiles: &[ProductHomeProfile],
    slug: &str,
    search_query: &str,
    filter: HomeTemplateFilter,
) -> bool {
    filtered_home_profile_indices(profiles, search_query, filter)
        .into_iter()
        .any(|index| profiles[index].fixture.slug == slug)
}

fn filtered_home_profile_indices(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
) -> Vec<usize> {
    profiles
        .iter()
        .enumerate()
        .filter_map(|(index, profile)| {
            (filter.matches(profile) && home_profile_matches_search(profile, search_query))
                .then_some(index)
        })
        .collect()
}

fn home_profile_matches_search(profile: &ProductHomeProfile, search_query: &str) -> bool {
    let terms = search_query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return true;
    }
    let haystack = home_profile_search_haystack(profile);
    terms.iter().all(|term| haystack.contains(term))
}

fn home_profile_search_haystack(profile: &ProductHomeProfile) -> String {
    format!(
        "{} {} {} {} {} {}",
        profile.label,
        profile.style_name,
        profile.family_id,
        profile.family_name,
        profile.category_chips.join(" "),
        profile_description(&profile.fixture.slug)
    )
    .to_ascii_lowercase()
}

impl HomeTemplateFilter {
    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
        }
    }

    fn matches(self, _profile: &ProductHomeProfile) -> bool {
        match self {
            Self::All => true,
        }
    }
}

fn start_template_button(ui: &mut egui::Ui) -> egui::Response {
    ui.horizontal(|ui| action_button(ui, &ActionSpec::enabled(ACTION_START, ButtonTone::Primary)))
        .inner
}

fn load_material_look_evidence(
    report_path: &Path,
    current_artifact_fingerprint: Option<&str>,
) -> Result<MakeMaterialLookEvidence, String> {
    if !report_path.is_file() {
        return Err(MATERIAL_LOOK_MISSING_MESSAGE.to_owned());
    }
    let variants_dir = report_path
        .parent()
        .ok_or_else(|| "Material look report path is invalid.".to_owned())?;
    let package_root = package_root_for_surface_candidate_report(report_path)
        .ok_or_else(|| "Material look package path is invalid.".to_owned())?;
    let report: SurfaceCandidateEvidenceReportFile = read_json_file(report_path)?;
    if report.schema_version != 1 {
        return Err("Material look report schema is not supported.".to_owned());
    }
    if report.profile_id != BOX_PRIMITIVE_PROFILE_ID {
        return Err("Material looks are not part of the Box Primitive baseline.".to_owned());
    }
    if report.visual_foundry_surface_mode_enabled {
        return Err("Material look report overclaims Surface mode readiness.".to_owned());
    }
    if !report.all_candidates_valid || report.candidate_count != MATERIAL_LOOK_TITLES.len() {
        return Err("Material look report does not contain six valid candidates.".to_owned());
    }
    if report.full_ready_status != "blocked" {
        return Err("Material look package must keep full game-ready status blocked.".to_owned());
    }
    if !full_ready_blockers_are_honest(&report.full_ready_blocker_codes) {
        return Err("Material look package is missing required game-ready blockers.".to_owned());
    }

    let candidate_set: SurfaceCandidateSetFile =
        read_json_file(&variants_dir.join(SURFACE_CANDIDATE_SET_FILE))?;
    if candidate_set.schema_version == 0 || candidate_set.profile_id != BOX_PRIMITIVE_PROFILE_ID {
        return Err("Material look candidate set is not for Box Primitive.".to_owned());
    }
    let candidate_rows = candidate_set
        .candidates
        .into_iter()
        .map(|candidate| (candidate.candidate_id.clone(), candidate))
        .collect::<BTreeMap<_, _>>();

    let mut candidates = Vec::with_capacity(report.candidates.len());
    for (index, row) in report.candidates.iter().enumerate() {
        let Some(expected_title) = MATERIAL_LOOK_TITLES.get(index) else {
            return Err("Material look report contains unexpected candidates.".to_owned());
        };
        if row.display_name != *expected_title {
            return Err("Material look candidate titles do not match the approved set.".to_owned());
        }
        if row.shape_delta_leak_detected {
            return Err("Material look evidence detected a shape change.".to_owned());
        }
        if row.result_class == "unsupported" || row.result_class == "duplicate_looking" {
            return Err("Material look evidence is not visually usable.".to_owned());
        }
        let set_row = candidate_rows.get(&row.candidate_id).ok_or_else(|| {
            format!(
                "Material look candidate {} is missing from the candidate set.",
                row.display_name
            )
        })?;
        validate_matching_candidate_refs(row, set_row)?;
        if !package_root.join(&row.material_override_ref).is_file() {
            return Err("Material look material override evidence is missing.".to_owned());
        }
        if !set_row.preserves_frozen_geometry {
            return Err("Material look candidate does not preserve frozen geometry.".to_owned());
        }
        if set_row.full_ready_status != "blocked" || !set_row.blocked_full_ready {
            return Err(
                "Material look candidate must remain blocked from full game-ready.".to_owned(),
            );
        }
        if let Some(fingerprint) = current_artifact_fingerprint
            && set_row.frozen_mesh_fingerprint != fingerprint
        {
            return Err("Material looks do not match this box build.".to_owned());
        }

        let validation_path = package_root.join(&row.validation_ref);
        let validation: SurfaceCandidateValidationFile = read_json_file(&validation_path)?;
        if !validation.valid || !validation.blocker_codes.is_empty() {
            return Err("Material look candidate validation did not pass.".to_owned());
        }
        let delta_path = package_root.join(&row.surface_delta_ref);
        let delta: SurfaceCandidateDeltaFile = read_json_file(&delta_path)?;
        if delta.profile_id != BOX_PRIMITIVE_PROFILE_ID
            || delta.candidate_id != row.candidate_id
            || delta.shape_delta_leak_detected
            || delta.result_class == "unsupported"
        {
            return Err("Material look surface delta is not material-only.".to_owned());
        }

        let preview_path = package_root.join(&row.textured_preview_ref);
        let preview_bytes = fs::read(&preview_path).map_err(|error| {
            format!(
                "Material look textured preview is missing: {} ({error})",
                preview_path.display()
            )
        })?;
        let preview = image::load_from_memory(&preview_bytes)
            .map_err(|error| format!("Material look textured preview could not load: {error}"))?
            .to_rgba8();
        let width = preview.width();
        let height = preview.height();
        if width == 0 || height == 0 {
            return Err("Material look textured preview is empty.".to_owned());
        }

        candidates.push(MakeMaterialLookCandidate {
            candidate_id: row.candidate_id.clone(),
            display_name: row.display_name.clone(),
            material_override_ref: row.material_override_ref.clone(),
            textured_preview_ref: row.textured_preview_ref.clone(),
            surface_delta_ref: row.surface_delta_ref.clone(),
            validation_ref: row.validation_ref.clone(),
            changed_material_slots: set_row.changed_material_slots.clone(),
            rgba8: preview.into_raw(),
            width,
            height,
            visible_surface_pixel_delta: row.visible_surface_pixel_delta,
        });
    }

    Ok(MakeMaterialLookEvidence {
        candidates,
        full_ready_blocker_codes: report.full_ready_blocker_codes,
    })
}

fn package_root_for_surface_candidate_report(report_path: &Path) -> Option<PathBuf> {
    let variants_dir = report_path.parent()?;
    let surface_dir = variants_dir.parent()?;
    surface_dir.parent().map(Path::to_path_buf)
}

fn read_json_file<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))
}

fn full_ready_blockers_are_honest(blockers: &[String]) -> bool {
    [
        "manual_review_pending",
        "engine_import_proof_missing",
        "engine_native_package_not_implemented",
        "surface_manual_review_required",
    ]
    .into_iter()
    .all(|required| blockers.iter().any(|blocker| blocker == required))
}

fn validate_matching_candidate_refs(
    report: &SurfaceCandidateEvidenceReportRowFile,
    candidate: &SurfaceCandidateSetRowFile,
) -> Result<(), String> {
    if report.display_name != candidate.display_name
        || report.material_override_ref != candidate.material_override_ref
        || report.textured_preview_ref != candidate.textured_preview_ref
        || report.surface_delta_ref != candidate.surface_delta_ref
        || report.validation_ref != candidate.validation_ref
    {
        Err("Material look report and candidate set disagree.".to_owned())
    } else if report.material_override_ref.trim().is_empty()
        || report.textured_preview_ref.trim().is_empty()
        || report.surface_delta_ref.trim().is_empty()
        || report.validation_ref.trim().is_empty()
    {
        Err("Material look evidence has missing file references.".to_owned())
    } else {
        Ok(())
    }
}

fn show_material_look_comparison_card(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    selected_candidate: &MakeMaterialLookCandidate,
) {
    product_card(ui, true, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal_wrapped(|ui| {
            let _ = status_pill(
                ui,
                StatusPillSpec::new(MATERIAL_LOOK_SURFACE_ONLY_COPY, StatusTone::Ready),
            );
            let _ = status_pill(
                ui,
                StatusPillSpec::new(MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY, StatusTone::Ready),
            );
        });
        ui.add_space(6.0);
        let preview_edge = (ui.available_width() * 0.22).clamp(88.0, 132.0);
        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 24.0);
                ui.label(
                    RichText::new("Current Material")
                        .color(colors.text)
                        .strong(),
                );
                if let Some(preview) = current_preview {
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: "material-look-current-comparison",
                            build: preview.build.as_ref(),
                            rgba8: &preview.rgba8,
                            width: preview.width,
                            height: preview.height,
                            max_edge: preview_edge,
                        },
                    );
                } else {
                    ui.label(RichText::new("Preview pending").color(colors.text_muted));
                }
            });
            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 24.0);
                ui.label(
                    RichText::new("Candidate Material")
                        .color(colors.text)
                        .strong(),
                );
                let preview_id = format!(
                    "material-look-selected-comparison-{}",
                    selected_candidate.candidate_id
                );
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build,
                        rgba8: &selected_candidate.rgba8,
                        width: selected_candidate.width,
                        height: selected_candidate.height,
                        max_edge: preview_edge,
                    },
                );
            });
        });
        ui.add_space(8.0);
        ui.label(
            RichText::new(&selected_candidate.display_name)
                .color(colors.text)
                .strong(),
        );
        ui.add(
            egui::Label::new(
                RichText::new(material_look_changed_summary(selected_candidate))
                    .color(colors.text_muted)
                    .small(),
            )
            .wrap(),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(MATERIAL_LOOK_PREVIEW_ONLY_COPY)
                .color(colors.warning)
                .small(),
        );
        ui.label(
            RichText::new(MATERIAL_LOOK_FULL_READY_BLOCKED_COPY)
                .color(colors.text_muted)
                .small(),
        );
    });
}

fn show_material_look_candidate_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidates: &[MakeMaterialLookCandidate],
    selected_candidate_id: &str,
    compact: bool,
) -> Option<String> {
    let mut selected = None;
    let columns = if compact {
        if ui.available_width() >= 760.0 { 3 } else { 2 }
    } else if ui.available_width() >= 760.0 {
        3
    } else {
        2
    };
    let preview_edge = if compact { 96.0 } else { 156.0 };
    for row in candidates.chunks(columns) {
        ui.columns(row.len(), |uis| {
            for (column, candidate) in uis.iter_mut().zip(row) {
                let is_selected = candidate.candidate_id == selected_candidate_id;
                product_card(column, is_selected, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.label(
                        RichText::new(&candidate.display_name)
                            .color(colors.text)
                            .strong(),
                    );
                    ui.add_space(6.0);
                    let preview_id = format!("material-look-card-{}", candidate.candidate_id);
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: &preview_id,
                            build: current_build,
                            rgba8: &candidate.rgba8,
                            width: candidate.width,
                            height: candidate.height,
                            max_edge: preview_edge,
                        },
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        let _ = status_pill(
                            ui,
                            StatusPillSpec::new(MATERIAL_LOOK_SURFACE_ONLY_COPY, StatusTone::Ready),
                        );
                        let _ = status_pill(
                            ui,
                            StatusPillSpec::new(
                                MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
                                StatusTone::Ready,
                            ),
                        );
                    });
                    if !compact {
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                RichText::new(material_look_changed_summary(candidate))
                                    .color(colors.text_muted)
                                    .small(),
                            )
                            .wrap(),
                        );
                        if let Some(delta) = candidate.visible_surface_pixel_delta {
                            ui.label(
                                RichText::new(format!(
                                    "Visible material difference {:.1}%",
                                    delta * 100.0
                                ))
                                .color(colors.text_subtle)
                                .small(),
                            );
                        }
                    }
                    ui.add_space(8.0);
                    if action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
                    )
                    .clicked()
                    {
                        selected = Some(candidate.candidate_id.clone());
                    }
                });
            }
        });
        ui.add_space(8.0);
    }
    selected
}

fn material_look_changed_summary(candidate: &MakeMaterialLookCandidate) -> String {
    let labels = candidate
        .changed_material_slots
        .iter()
        .filter_map(|slot| material_look_slot_summary_label(slot))
        .collect::<BTreeSet<_>>();
    if labels.is_empty() {
        "Changes the visible finish while keeping the box shape fixed.".to_owned()
    } else {
        format!(
            "Changes {} while keeping the box shape fixed.",
            human_join(labels.into_iter().collect::<Vec<_>>().as_slice())
        )
    }
}

fn material_look_slot_summary_label(slot: &str) -> Option<&'static str> {
    match slot {
        "painted_metal_body" => Some("body finish"),
        "shadowed_body_edges" => Some("edge contrast"),
        "exposed_edge_detail" => Some("edge detail"),
        "soft_edge_highlights" => Some("edge highlights"),
        "fallback_hard_surface" => Some("secondary surfaces"),
        _ => None,
    }
}

fn human_join(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [first, second] => format!("{first} and {second}"),
        _ => {
            let mut joined = items[..items.len() - 1].join(", ");
            joined.push_str(", and ");
            joined.push_str(items[items.len() - 1]);
            joined
        }
    }
}

struct VisibleDirectionIdeasBoard<'a> {
    current_build: Option<&'a FoundryBuildStamp>,
    current_preview: Option<&'a FoundryPreviewImage>,
    candidates: &'a [crate::foundry::view_model::FoundryCandidateCard],
    actions_enabled: bool,
    disabled_reason: &'a str,
    use_candidate_label: &'a str,
}

fn show_visible_direction_ideas_board(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    board: VisibleDirectionIdeasBoard<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    commands.extend(show_selected_candidate_comparison_compact(
        ui,
        texture_cache,
        &board,
    ));
    ui.add_space(8.0);
    commands.extend(show_direction_candidate_grid_compact(
        ui,
        texture_cache,
        board.current_build,
        board.candidates,
        board.actions_enabled,
        board.disabled_reason,
    ));
    commands
}

fn show_selected_candidate_comparison_compact(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    board: &VisibleDirectionIdeasBoard<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let Some(candidate) = board
        .candidates
        .iter()
        .find(|candidate| candidate.selected)
        .or_else(|| board.candidates.first())
    else {
        return commands;
    };
    let Some(current_preview) = board.current_preview else {
        return commands;
    };
    if candidate.rgba8.is_empty() || current_preview.rgba8.is_empty() {
        return commands;
    }

    product_card(ui, true, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal_top(|ui| {
            let preview_edge = compact_comparison_preview_edge(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.set_width((preview_edge * 2.0) + 18.0);
                ui.horizontal_top(|ui| {
                    ui.vertical_centered(|ui| {
                        ui.set_width(preview_edge);
                        ui.label(RichText::new("Current").color(colors.text).small().strong());
                        show_rgba_preview(
                            ui,
                            texture_cache,
                            FoundryPreviewDraw {
                                preview_id: "direction-current-comparison-compact",
                                build: current_preview.build.as_ref(),
                                rgba8: &current_preview.rgba8,
                                width: current_preview.width,
                                height: current_preview.height,
                                max_edge: preview_edge,
                            },
                        );
                    });
                    ui.add_space(8.0);
                    ui.vertical_centered(|ui| {
                        ui.set_width(preview_edge);
                        ui.label(RichText::new("Idea").color(colors.text).small().strong());
                        let preview_id =
                            format!("direction-selected-comparison-compact-{}", candidate.id.0);
                        show_rgba_preview(
                            ui,
                            texture_cache,
                            FoundryPreviewDraw {
                                preview_id: &preview_id,
                                build: board.current_build,
                                rgba8: &candidate.rgba8,
                                width: candidate.width,
                                height: candidate.height,
                                max_edge: preview_edge,
                            },
                        );
                    });
                });
            });
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(candidate_display_title(candidate))
                        .color(colors.text)
                        .strong(),
                );
                if let Some(detail) = candidate_display_detail(candidate) {
                    ui.add(
                        egui::Label::new(RichText::new(detail).color(colors.text_muted).small())
                            .wrap(),
                    );
                }
                ui.label(
                    RichText::new(candidate_display_subtitle(candidate))
                        .color(colors.text_muted)
                        .small(),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    let choose_reason = candidate
                        .preview_failure
                        .as_ref()
                        .map(|reason| {
                            product_panel_message(reason, "Preview this idea before using it.")
                        })
                        .unwrap_or_else(|| NEED_DIRECTION_REASON.to_owned());
                    if action_button(
                        ui,
                        &action_spec(
                            board.actions_enabled && candidate.selectable,
                            board.use_candidate_label,
                            ButtonTone::Primary,
                            if board.actions_enabled {
                                choose_reason.as_str()
                            } else {
                                board.disabled_reason
                            },
                        ),
                    )
                    .clicked()
                    {
                        commands.push(directions::accept_candidate_command(candidate.id.clone()));
                    }
                    if action_button(
                        ui,
                        &action_spec(
                            board.actions_enabled,
                            ACTION_REJECT,
                            ButtonTone::Secondary,
                            board.disabled_reason,
                        ),
                    )
                    .clicked()
                    {
                        commands.push(directions::reject_candidate_command(candidate.id.clone()));
                    }
                });
            });
        });
    });

    commands
}

fn show_selected_candidate_comparison(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    current_preview: Option<&FoundryPreviewImage>,
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let Some(candidate) = candidates
        .iter()
        .find(|candidate| candidate.selected)
        .or_else(|| candidates.first())
    else {
        return commands;
    };
    let Some(current_preview) = current_preview else {
        return commands;
    };
    if candidate.rgba8.is_empty() || current_preview.rgba8.is_empty() {
        return commands;
    }

    product_card(ui, true, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.label(
            RichText::new("Compare")
                .color(colors.accent_hover)
                .small()
                .strong(),
        );
        ui.add_space(8.0);
        let preview_edge = (ui.available_width() * 0.24).clamp(180.0, 320.0);
        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 28.0);
                ui.label(RichText::new("Current").color(colors.text).strong());
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: "direction-current-comparison",
                        build: current_preview.build.as_ref(),
                        rgba8: &current_preview.rgba8,
                        width: current_preview.width,
                        height: current_preview.height,
                        max_edge: preview_edge,
                    },
                );
            });
            ui.add_space(18.0);
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 28.0);
                ui.label(RichText::new("Candidate").color(colors.text).strong());
                ui.label(
                    RichText::new(candidate_display_title(candidate))
                        .color(colors.text)
                        .strong(),
                );
                let preview_id = format!("direction-selected-comparison-{}", candidate.id.0);
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build,
                        rgba8: &candidate.rgba8,
                        width: candidate.width,
                        height: candidate.height,
                        max_edge: preview_edge,
                    },
                );
            });
        });
        ui.add_space(12.0);
        ui.label(RichText::new("What changed").color(colors.text).strong());
        if let Some(detail) = candidate_display_detail(candidate) {
            ui.add(egui::Label::new(RichText::new(detail).color(colors.text_muted)).wrap());
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Affected parts").color(colors.text).strong());
        ui.label(
            RichText::new(candidate_display_subtitle(candidate))
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(12.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
            )
            .clicked()
            {
                commands.push(FoundryAppCommand::SelectCandidate(Some(
                    candidate.id.clone(),
                )));
            }
            let choose_reason = candidate
                .preview_failure
                .as_ref()
                .map(|reason| product_panel_message(reason, "Preview this idea before using it."))
                .unwrap_or_else(|| NEED_DIRECTION_REASON.to_owned());
            if action_button(
                ui,
                &action_spec(
                    actions_enabled && candidate.selectable,
                    ACTION_CHOOSE_DIRECTION,
                    ButtonTone::Primary,
                    if actions_enabled {
                        choose_reason.as_str()
                    } else {
                        disabled_reason
                    },
                ),
            )
            .clicked()
            {
                commands.push(directions::accept_candidate_command(candidate.id.clone()));
            }
            if action_button(
                ui,
                &action_spec(
                    actions_enabled,
                    ACTION_REJECT,
                    ButtonTone::Secondary,
                    disabled_reason,
                ),
            )
            .clicked()
            {
                commands.push(directions::reject_candidate_command(candidate.id.clone()));
            }
        });
    });

    commands
}

fn show_direction_candidate_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let columns = direction_grid_columns(ui.available_width());
    for row in candidates.chunks(columns) {
        ui.columns(columns, |column_uis| {
            for (column, candidate) in column_uis.iter_mut().zip(row) {
                commands.extend(show_direction_candidate_card(
                    column,
                    texture_cache,
                    current_build,
                    candidate,
                    actions_enabled,
                    disabled_reason,
                ));
            }
        });
        ui.add_space(8.0);
    }
    commands
}

fn show_direction_candidate_grid_compact(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let columns = if candidates.len() >= 6 {
        3
    } else {
        compact_direction_grid_columns(ui.available_width())
    };
    for row in candidates.chunks(columns) {
        ui.columns(columns, |column_uis| {
            for (column, candidate) in column_uis.iter_mut().zip(row) {
                commands.extend(show_direction_candidate_card_compact(
                    column,
                    texture_cache,
                    current_build,
                    candidate,
                    actions_enabled,
                    disabled_reason,
                ));
            }
        });
        ui.add_space(6.0);
    }
    commands
}

fn direction_grid_columns(width: f32) -> usize {
    if width >= 1500.0 {
        4
    } else if width >= 1080.0 {
        3
    } else if width >= 720.0 {
        2
    } else {
        1
    }
}

fn compact_direction_grid_columns(width: f32) -> usize {
    if width >= 560.0 {
        3
    } else if width >= 380.0 {
        2
    } else {
        1
    }
}

fn compact_comparison_preview_edge(width: f32) -> f32 {
    (width * 0.16).clamp(84.0, 128.0)
}

fn compact_direction_card_preview_edge(width: f32) -> f32 {
    (width * 0.22).clamp(46.0, 64.0)
}

fn show_direction_skeleton_grid(ui: &mut egui::Ui) {
    let columns = direction_grid_columns(ui.available_width());
    let slots = (1..=6).collect::<Vec<_>>();
    for row in slots.chunks(columns) {
        ui.columns(columns, |columns| {
            for (slot, column) in row.iter().zip(columns.iter_mut()) {
                product_card(column, false, |ui| {
                    let colors = VisualFoundryTokens::dark().colors;
                    ui.set_min_height(190.0);
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        ui.spinner();
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(format!("Idea {}", slot))
                                .color(colors.text)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Preparing idea.")
                                .color(colors.text_muted)
                                .small(),
                        );
                    });
                });
            }
        });
        ui.add_space(8.0);
    }
}

fn show_direction_candidate_card_compact(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
    _actions_enabled: bool,
    _disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, candidate.selected, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.set_min_height(92.0);
        let available_width = ui.available_width();
        let preview_id = candidate_preview_texture_id(candidate);
        let preview_edge = compact_direction_card_preview_edge(available_width);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(preview_edge + 4.0, preview_edge + 4.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    show_rgba_preview(
                        ui,
                        texture_cache,
                        FoundryPreviewDraw {
                            preview_id: &preview_id,
                            build: current_build,
                            rgba8: &candidate.rgba8,
                            width: candidate.width,
                            height: candidate.height,
                            max_edge: preview_edge,
                        },
                    );
                },
            );
            ui.add_space(8.0);
            let text_width = (available_width - preview_edge - 18.0).max(92.0);
            ui.allocate_ui_with_layout(
                egui::vec2(text_width, 52.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(text_width);
                    ui.label(RichText::new(candidate_display_title(candidate)).strong());
                    ui.label(
                        RichText::new(candidate_display_subtitle(candidate))
                            .color(colors.text_muted)
                            .small(),
                    );
                    if candidate.selected {
                        ui.label(RichText::new("Selected").color(colors.accent_hover).small());
                    }
                },
            );
        });
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
            )
            .clicked()
            {
                commands.push(FoundryAppCommand::SelectCandidate(Some(
                    candidate.id.clone(),
                )));
            }
        });
    });
    commands
}

fn show_direction_candidate_card(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, candidate.selected, |ui| {
        let preview_id = candidate_preview_texture_id(candidate);
        let available_width = ui.available_width().max(1.0);
        let preview_edge = available_width.clamp(160.0, 260.0);
        ui.set_min_height(preview_edge + 132.0);

        ui.vertical_centered(|ui| {
            show_rgba_preview(
                ui,
                texture_cache,
                FoundryPreviewDraw {
                    preview_id: &preview_id,
                    build: current_build,
                    rgba8: &candidate.rgba8,
                    width: candidate.width,
                    height: candidate.height,
                    max_edge: preview_edge,
                },
            );
        });

        ui.add_space(10.0);
        commands.extend(show_direction_candidate_details(
            ui,
            candidate,
            actions_enabled,
            disabled_reason,
        ));
    });
    commands
}

fn show_direction_candidate_details(
    ui: &mut egui::Ui,
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    ui.label(RichText::new(candidate_display_title(candidate)).strong());
    ui.label(
        RichText::new(candidate_display_subtitle(candidate))
            .color(VisualFoundryTokens::dark().colors.text_muted)
            .small(),
    );
    if let Some(detail) = candidate_display_detail(candidate) {
        ui.add(
            egui::Label::new(
                RichText::new(detail)
                    .color(VisualFoundryTokens::dark().colors.text_muted)
                    .small(),
            )
            .wrap(),
        );
    }
    if candidate.selected {
        ui.weak("Selected idea");
    }
    if let Some(reason) = &candidate.preview_failure {
        ui.label(
            RichText::new(product_panel_message(
                reason,
                "Preview could not be rendered for this direction.",
            ))
            .color(VisualFoundryTokens::dark().colors.warning),
        );
    }
    ui.add_space(8.0);
    ui.horizontal_wrapped(|ui| {
        if action_button(
            ui,
            &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Secondary),
        )
        .clicked()
        {
            commands.push(FoundryAppCommand::SelectCandidate(Some(
                candidate.id.clone(),
            )));
        }
        let choose_reason = candidate
            .preview_failure
            .as_ref()
            .map(|reason| product_panel_message(reason, "Preview this idea before using it."))
            .unwrap_or_else(|| NEED_DIRECTION_REASON.to_owned());
        if action_button(
            ui,
            &action_spec(
                actions_enabled && candidate.selectable,
                ACTION_CHOOSE_DIRECTION,
                ButtonTone::Secondary,
                if actions_enabled {
                    choose_reason.as_str()
                } else {
                    disabled_reason
                },
            ),
        )
        .clicked()
        {
            commands.push(directions::accept_candidate_command(candidate.id.clone()));
        }
        if action_button(
            ui,
            &action_spec(
                actions_enabled,
                ACTION_REJECT,
                ButtonTone::Secondary,
                disabled_reason,
            ),
        )
        .clicked()
        {
            commands.push(directions::reject_candidate_command(candidate.id.clone()));
        }
    });
    commands
}

fn candidate_display_title(candidate: &crate::foundry::view_model::FoundryCandidateCard) -> String {
    let title = candidate.title.trim();
    if !candidate_title_looks_raw(title) && !candidate_title_looks_trait_derived(title) {
        let safe_title = product_panel_message(title, "");
        if !safe_title.trim().is_empty() {
            return safe_title;
        }
    }

    DIRECTION_INTENT_TITLES[candidate.slot % DIRECTION_INTENT_TITLES.len()].to_owned()
}

fn candidate_display_subtitle(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> String {
    let delta = product_panel_message(&candidate.visible_delta_label, "Visible change");
    if let Some(focus_part) = &candidate.focus_part_label {
        return format!(
            "{} · {}",
            product_panel_message(focus_part, "Focused part"),
            delta
        );
    }

    let intent = product_panel_message(&candidate.variation_intent_label, "Idea");
    let channels = candidate
        .variation_channel_labels
        .iter()
        .map(|label| product_panel_message(label, "Change"))
        .filter(|label| label != &intent && label != "Complete Look" && label != "Complete Looks")
        .collect::<Vec<_>>();

    if channels.is_empty() {
        delta
    } else {
        format!("{} · {}", channels.join(", "), delta)
    }
}

fn candidate_display_detail(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> Option<String> {
    if let Some(reason) = &candidate.surface_unavailable_reason {
        return Some(product_panel_message(
            reason,
            "This idea is unavailable for the current kit.",
        ));
    }

    let labels = candidate
        .changed_controls
        .iter()
        .chain(candidate.changed_roles.iter())
        .filter_map(|label| candidate_change_phrase(label))
        .take(4)
        .collect::<Vec<_>>();

    if !labels.is_empty() {
        return Some(labels.join(" · "));
    }

    let raw_summary = candidate.what_changed_summary.trim();
    if !raw_summary.is_empty()
        && !raw_summary.contains(':')
        && !raw_summary.to_ascii_lowercase().contains("option changed")
        && !raw_summary
            .to_ascii_lowercase()
            .contains("proportion adjusted")
    {
        let summary = product_panel_message(raw_summary, "Visible shape adjusted.");
        if !summary.trim().is_empty() {
            return Some(summary);
        }
    }

    Some("Idea changes are visible in the comparison above.".to_owned())
}

const DIRECTION_INTENT_TITLES: [&str; 6] = [
    "Compact Box",
    "Wide Box",
    "Tall Box",
    "Flat Box",
    "Soft-Edged Box",
    "Sharp Box",
];

fn candidate_title_looks_raw(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    title.contains('#')
        || title.contains('(')
        || title.contains(')')
        || lower.contains("candidate")
        || lower.contains("preview_id")
}

fn candidate_title_looks_trait_derived(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    lower.ends_with(" direction")
        || lower.contains("edge softness")
        || lower.contains("proportions")
}

fn candidate_change_phrase(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let phrase = if lower.contains("edge") && lower.contains("soft") {
        "softer edges"
    } else if lower.contains("proportion") {
        "new proportions"
    } else if lower.contains("detail") {
        "more surface detail"
    } else if lower.contains("silhouette") {
        "cleaner silhouette"
    } else {
        return product_title_fragment(raw).map(|label| {
            let mut words = label.to_ascii_lowercase();
            words.push_str(" adjusted");
            words
        });
    };
    Some(phrase.to_owned())
}

fn product_title_fragment(raw: &str) -> Option<String> {
    let words = raw
        .split(|character: char| {
            character == '_'
                || character == '-'
                || character == '.'
                || character == '/'
                || character == ':'
                || character == '#'
                || character == '('
                || character == ')'
                || character.is_whitespace()
        })
        .filter(|word| !word.trim().is_empty())
        .filter(|word| !looks_like_generated_token(word))
        .take(3)
        .map(title_case_word)
        .collect::<Vec<_>>();
    (!words.is_empty()).then(|| words.join(" "))
}

fn looks_like_generated_token(word: &str) -> bool {
    word.len() >= 8 && word.chars().all(|character| character.is_ascii_hexdigit())
}

fn title_case_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => {
            let mut output = first.to_uppercase().collect::<String>();
            output.push_str(&chars.as_str().to_ascii_lowercase());
            output
        }
        None => String::new(),
    }
}

fn show_customize_control_card(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    control: &crate::foundry::view_model::FoundryControlView,
    actions_enabled: bool,
    disabled_reason: &str,
    show_focus_action: bool,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, control.locked, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        let header_width = ui.available_width();
        if header_width < CONTROL_HEADER_STACK_BREAKPOINT {
            ui.vertical(|ui| {
                ui.add(egui::Label::new(RichText::new(&control.label).strong()).wrap());
                ui.add(
                    egui::Label::new(
                        RichText::new(product_control_summary(control))
                            .color(colors.text_muted)
                            .small(),
                    )
                    .wrap(),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    show_customize_control_header_actions(
                        ui,
                        control,
                        actions_enabled,
                        disabled_reason,
                        show_focus_action,
                        &mut commands,
                    );
                });
            });
        } else {
            ui.horizontal(|ui| {
                let actions_width = CONTROL_HEADER_ACTIONS_WIDTH.min(header_width * 0.58);
                let text_width = (header_width - actions_width - 12.0).max(140.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(text_width, 44.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.add(egui::Label::new(RichText::new(&control.label).strong()).wrap());
                        ui.add(
                            egui::Label::new(
                                RichText::new(product_control_summary(control))
                                    .color(colors.text_muted)
                                    .small(),
                            )
                            .wrap(),
                        );
                    },
                );
                ui.add_space(8.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(actions_width, 44.0),
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        show_customize_control_header_actions(
                            ui,
                            control,
                            actions_enabled,
                            disabled_reason,
                            show_focus_action,
                            &mut commands,
                        );
                    },
                );
            });
        }
        if let Some(reason) = &control.locked_reason {
            ui.label(
                RichText::new(product_panel_message(reason, "This control is locked."))
                    .color(colors.text_muted)
                    .small(),
            );
        }
        ui.add_space(8.0);
        let visible_options = control
            .options
            .iter()
            .take(CONTROL_FILMSTRIP_LIMIT)
            .collect::<Vec<_>>();
        let action_state = CustomizeActionState {
            enabled: actions_enabled,
            disabled_reason,
        };
        commands.extend(show_customize_option_grid(
            ui,
            texture_cache,
            current_build,
            control,
            &visible_options,
            true,
            action_state,
        ));
        let remaining_options = control
            .options
            .iter()
            .skip(CONTROL_FILMSTRIP_LIMIT)
            .collect::<Vec<_>>();
        if !remaining_options.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("More options")
                    .color(colors.text_muted)
                    .small(),
            );
            commands.extend(show_customize_option_grid(
                ui,
                texture_cache,
                current_build,
                control,
                &remaining_options,
                false,
                action_state,
            ));
        }
    });
    commands
}

fn show_customize_control_header_actions(
    ui: &mut egui::Ui,
    control: &crate::foundry::view_model::FoundryControlView,
    actions_enabled: bool,
    disabled_reason: &str,
    show_focus_action: bool,
    commands: &mut Vec<FoundryAppCommand>,
) {
    if action_button(
        ui,
        &action_spec(
            actions_enabled && customize::control_can_reset(control),
            ACTION_RESET,
            ButtonTone::Secondary,
            if actions_enabled {
                NEED_RESET_REASON
            } else {
                disabled_reason
            },
        ),
    )
    .clicked()
    {
        commands.extend(customize::reset_control_intents(control));
    }
    let lock_label = if control.locked {
        ACTION_UNLOCK
    } else {
        ACTION_LOCK
    };
    if action_button(
        ui,
        &action_spec(
            actions_enabled,
            lock_label,
            ButtonTone::Secondary,
            disabled_reason,
        ),
    )
    .clicked()
        && let Some(command) = customize::control_lock_command(control, !control.locked)
    {
        commands.push(command);
    }
    if show_focus_action
        && action_button(
            ui,
            &action_spec(
                actions_enabled,
                ACTION_FOCUS,
                ButtonTone::Secondary,
                disabled_reason,
            ),
        )
        .clicked()
    {
        commands.push(customize::select_control_command(Some(control.id.clone())));
    }
}

#[derive(Debug, Clone, Copy)]
struct CustomizeActionState<'a> {
    enabled: bool,
    disabled_reason: &'a str,
}

fn show_customize_option_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    control: &crate::foundry::view_model::FoundryControlView,
    options: &[&crate::foundry::view_model::FoundryOptionCard],
    show_preview: bool,
    action_state: CustomizeActionState<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    if options.is_empty() {
        return commands;
    }

    let columns = customize_option_grid_columns(ui.available_width());
    for row in options.chunks(columns) {
        ui.columns(columns, |column_uis| {
            for (column, option) in column_uis.iter_mut().zip(row) {
                commands.extend(show_customize_option_tile(
                    column,
                    texture_cache,
                    current_build,
                    control,
                    option,
                    show_preview,
                    action_state,
                ));
            }
        });
        ui.add_space(6.0);
    }
    commands
}

fn customize_option_grid_columns(width: f32) -> usize {
    if width >= 1160.0 {
        3
    } else if width >= 720.0 {
        2
    } else {
        1
    }
}

fn show_customize_option_tile(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    control: &crate::foundry::view_model::FoundryControlView,
    option: &crate::foundry::view_model::FoundryOptionCard,
    show_preview: bool,
    action_state: CustomizeActionState<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, option.selected, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        let tile_width = ui.available_width().clamp(220.0, 360.0);
        ui.set_width(tile_width);
        ui.set_min_height(if show_preview { 250.0 } else { 150.0 });
        if show_preview {
            let preview_id = option_preview_texture_id(option);
            show_rgba_preview(
                ui,
                texture_cache,
                FoundryPreviewDraw {
                    preview_id: &preview_id,
                    build: current_build,
                    rgba8: &option.rgba8,
                    width: option.width,
                    height: option.height,
                    max_edge: 170.0,
                },
            );
        }
        ui.add(egui::Label::new(RichText::new(&option.label).color(colors.text).strong()).wrap());
        if option.selected {
            ui.weak("Current");
        }
        let option_disabled_reason = customize::option_action_disabled_reason(control, option);
        let disabled_message = option_disabled_reason
            .as_ref()
            .map(|reason| product_panel_message(reason, "This option is not available right now."))
            .unwrap_or_else(|| "This option is not available right now.".to_owned());
        if let Some(reason) = &option.unavailable_reason {
            ui.label(
                RichText::new(product_panel_message(
                    reason,
                    "This option is not available right now.",
                ))
                .color(colors.warning)
                .small(),
            );
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                &action_spec(
                    action_state.enabled && option_disabled_reason.is_none(),
                    ACTION_APPLY,
                    ButtonTone::Secondary,
                    if action_state.enabled {
                        disabled_message.as_str()
                    } else {
                        action_state.disabled_reason
                    },
                ),
            )
            .clicked()
            {
                commands.extend(customize::choose_option_intents(control, option));
            }
        });
    });
    commands
}

fn display_customize_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> Vec<&crate::foundry::view_model::FoundryControlView> {
    default_customize_controls(controls)
        .take(CUSTOMIZE_PRIMARY_CONTROL_LIMIT)
        .collect()
}

#[derive(Debug, Clone)]
struct MakeInspectorControlSections {
    visible: Vec<crate::foundry::view_model::FoundryControlView>,
    overflow: Vec<crate::foundry::view_model::FoundryControlView>,
    disclosure_label: &'static str,
    empty_title: &'static str,
    empty_message: &'static str,
}

fn make_context_inspector_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
    active_group: Option<&directions::DirectionPartGroup>,
) -> MakeInspectorControlSections {
    let default_controls = display_customize_controls(controls);
    if let Some(group) = active_group {
        let mut visible = Vec::new();
        let mut overflow = Vec::new();
        for control in default_controls {
            if make_control_matches_focus(control, Some(group))
                && visible.len() < MAKE_CONTEXT_INITIAL_CONTROL_LIMIT
            {
                visible.push(control.clone());
            } else {
                overflow.push(control.clone());
            }
        }
        return MakeInspectorControlSections {
            visible,
            overflow,
            disclosure_label: "Show all controls",
            empty_title: "No focused controls yet",
            empty_message: "Show all controls, or clear focus and try broader ideas.",
        };
    }

    let mut visible = Vec::new();
    let mut overflow = Vec::new();
    for control in default_controls {
        if visible.len() < MAKE_CONTEXT_INITIAL_CONTROL_LIMIT {
            visible.push(control.clone());
        } else {
            overflow.push(control.clone());
        }
    }
    MakeInspectorControlSections {
        visible,
        overflow,
        disclosure_label: "More controls",
        empty_title: "No quick controls yet",
        empty_message: "This asset has no quick controls yet.",
    }
}

fn make_control_matches_focus(
    control: &crate::foundry::view_model::FoundryControlView,
    active_group: Option<&directions::DirectionPartGroup>,
) -> bool {
    let Some(group) = active_group else {
        return true;
    };
    let control_text = format!("{} {}", control.id, control.label).to_ascii_lowercase();
    let group_label = group.label.to_ascii_lowercase();
    if group_label.contains("body") || group_label.contains("box") {
        control_text.contains("body")
            || control_text.contains("proportion")
            || control_text.contains("box")
            || control_text.contains("edge")
    } else {
        control_text.contains(group_label.trim_end_matches('s'))
    }
}

fn pack_member_display_name(member: &pack::PackMemberRow) -> String {
    let title = asset_title_from_id(&member.document_id);
    if title == "Shape Lab Project" {
        "Pack asset".to_owned()
    } else {
        title.to_owned()
    }
}

fn pack_cell_display_name(cell: &pack::PackContactSheetCell) -> String {
    let title = asset_title_from_id(&cell.document_id);
    if title == "Shape Lab Project" {
        "Pack asset".to_owned()
    } else {
        title.to_owned()
    }
}

fn show_pack_contact_sheet(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    sheet: &pack::PackContactSheet,
) {
    if sheet.cells.is_empty() {
        product_empty_state(
            ui,
            "Pack is empty",
            "Add the current asset to start a pack.",
        );
        return;
    }

    let columns = sheet.columns.max(1);
    for row in 0..sheet.rows {
        ui.columns(columns, |column_uis| {
            for cell in sheet.cells.iter().filter(|cell| cell.row == row) {
                let column_index = cell.column.min(column_uis.len().saturating_sub(1));
                product_card(&mut column_uis[column_index], cell.selected, |ui| {
                    ui.set_min_height(144.0);
                    show_pack_cell_thumbnail(
                        ui,
                        texture_cache,
                        current_preview,
                        current_build,
                        cell,
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new(pack_cell_display_name(cell)).strong());
                    let status = match cell.status {
                        pack::PackMemberStatus::Ready => "Ready",
                        pack::PackMemberStatus::NeedsAttention => "Needs attention",
                    };
                    ui.label(RichText::new(status).small());
                    if cell.selected {
                        ui.weak("Current");
                    }
                    if cell.override_count > 0 {
                        ui.weak(format!("{} adjustment(s)", cell.override_count));
                    }
                });
            }
        });
        ui.add_space(8.0);
    }
}

fn show_pack_cell_thumbnail(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_preview: Option<&FoundryPreviewImage>,
    current_build: Option<&FoundryBuildStamp>,
    cell: &pack::PackContactSheetCell,
) {
    if cell.selected
        && let Some(preview) = current_preview
    {
        let preview_id = format!("pack-current-{}", preview.preview_id);
        show_rgba_preview(
            ui,
            texture_cache,
            FoundryPreviewDraw {
                preview_id: &preview_id,
                build: preview.build.as_ref().or(current_build),
                rgba8: &preview.rgba8,
                width: preview.width,
                height: preview.height,
                max_edge: 88.0,
            },
        );
        return;
    }

    pack_thumbnail_placeholder(ui, cell);
}

fn pack_thumbnail_placeholder(ui: &mut egui::Ui, cell: &pack::PackContactSheetCell) {
    let colors = VisualFoundryTokens::dark().colors;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(88.0, 62.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), colors.panel_subtle);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, colors.stroke_strong),
        egui::StrokeKind::Inside,
    );
    let band_height = rect.height() * 0.34;
    let band = egui::Rect::from_min_max(
        egui::pos2(rect.left(), rect.bottom() - band_height),
        rect.right_bottom(),
    );
    ui.painter()
        .rect_filled(band, egui::CornerRadius::same(4), colors.accent_soft);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        pack_thumbnail_marker(cell),
        egui::FontId::proportional(11.0),
        colors.text_muted,
    );
}

fn pack_thumbnail_marker(cell: &pack::PackContactSheetCell) -> &'static str {
    if cell.selected { "Current" } else { "Preview" }
}

fn show_export_readiness_panel(
    ui: &mut egui::Ui,
    can_export_current: bool,
    pack_view: &crate::foundry::view_model::FoundryPackView,
    pack_panel: &pack::PackPanelView,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, can_export_current || pack_panel.export.enabled, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.label(RichText::new(export_readiness_label(can_export_current)).color(colors.text));
        ui.label(
            RichText::new(export_readiness_detail(
                can_export_current,
                pack_panel.export.enabled,
            ))
            .color(colors.text_muted)
            .small(),
        );
        ui.label(
            RichText::new(BOX_PRIMITIVE_EXPORT_LIMITATION)
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(10.0);
        export_checklist_row(ui, "Model prepared", can_export_current, NEED_MODEL_REASON);
        export_checklist_row(
            ui,
            "Asset ready",
            can_export_current,
            "Prepare the current model first.",
        );
        export_checklist_row(
            ui,
            "Pack members",
            !pack_panel.members.is_empty(),
            NEED_PACK_MEMBER_REASON,
        );
        ui.add_space(12.0);
        ui.label(RichText::new("Export options").color(colors.text).strong());
        ui.add_space(6.0);
        if action_button(
            ui,
            &action_spec(
                can_export_current,
                ACTION_EXPORT_CURRENT_ASSET,
                ButtonTone::Primary,
                NEED_MODEL_REASON,
            ),
        )
        .clicked()
            && let Some(out_dir) = select_asset_export_dir()
        {
            commands.push(FoundryAppCommand::run(FoundryCommand::Export {
                profile: "default".to_owned(),
                out_dir: Some(out_dir.to_string_lossy().to_string()),
            }));
        }
        if !can_export_current {
            ui.label(
                RichText::new(NEED_MODEL_REASON)
                    .color(colors.warning)
                    .small(),
            );
        }
        ui.add_space(6.0);
        if action_button(
            ui,
            &action_spec(
                pack_panel.export.enabled,
                ACTION_EXPORT_PACK,
                ButtonTone::Secondary,
                NEED_PACK_MEMBER_REASON,
            ),
        )
        .clicked()
            && let Some(out_dir) = select_pack_export_dir()
            && let Some(command) = pack::batch_export_command(pack_view, out_dir)
        {
            commands.push(command);
        }
        if !pack_panel.export.enabled {
            let reason = if pack_panel.members.is_empty() {
                NEED_PACK_MEMBER_REASON.to_owned()
            } else if let Some(reason) = &pack_panel.export.disabled_reason {
                product_panel_message(reason, "Resolve pack warnings before export.")
            } else {
                "Resolve pack warnings before export.".to_owned()
            };
            ui.label(RichText::new(reason).color(colors.warning).small());
        }
    });
    commands
}

fn export_checklist_row(ui: &mut egui::Ui, label: &str, ready: bool, reason: &str) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.horizontal_wrapped(|ui| {
        let status = if ready { "Ready" } else { "Blocked" };
        ui.label(RichText::new(status).color(if ready {
            colors.success
        } else {
            colors.warning
        }));
        ui.label(RichText::new(label).color(colors.text));
        if !ready {
            ui.label(RichText::new(reason).color(colors.text_muted).small());
        }
    });
}

fn pack_readiness_label(view: &pack::PackPanelView) -> &'static str {
    if view.export.enabled {
        "Export ready"
    } else if view.members.is_empty() {
        "Add assets to export"
    } else {
        "Needs attention"
    }
}

fn pack_readiness_detail(view: &pack::PackPanelView) -> String {
    if view.export.enabled {
        "All assets are ready for pack export.".to_owned()
    } else if let Some(reason) = &view.export.disabled_reason {
        product_panel_message(reason, "Resolve pack warnings before export.")
    } else {
        "Add at least one asset before exporting a pack.".to_owned()
    }
}

fn export_readiness_label(can_export: bool) -> &'static str {
    if can_export {
        "Current asset ready"
    } else {
        "Needs a model first"
    }
}

fn export_readiness_detail(can_export: bool, pack_ready: bool) -> &'static str {
    if can_export && pack_ready {
        "Export this asset here, or export the prepared pack from the Pack drawer."
    } else if can_export {
        "Export the current asset as an individual result."
    } else {
        "Prepare the current asset before exporting."
    }
}

fn default_customize_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> impl Iterator<Item = &crate::foundry::view_model::FoundryControlView> {
    controls
        .iter()
        .filter(|control| control.primary && control.visible)
}

fn candidate_preview_texture_id(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> String {
    candidate
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("candidate-{}", candidate.id.0))
}

fn option_preview_texture_id(option: &crate::foundry::view_model::FoundryOptionCard) -> String {
    option
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("option-{}-{}", option.control_id, option.label))
}

fn unique_pack_member_id(pack: &crate::foundry::view_model::FoundryPackView, base: &str) -> String {
    let base = if base.trim().is_empty() {
        "member"
    } else {
        base.trim()
    };
    if !pack.members.contains_key(base) {
        return base.to_owned();
    }

    for index in 2.. {
        let candidate = format!("{base}-{index}");
        if !pack.members.contains_key(&candidate) {
            return candidate;
        }
    }

    unreachable!("unbounded member suffix search should always find an unused id")
}

#[derive(Default)]
struct FoundryTextureCache {
    textures: BTreeMap<String, CachedFoundryTexture>,
}

struct CachedFoundryTexture {
    identity: FoundryTextureIdentity,
    texture: egui::TextureHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FoundryTextureIdentity {
    preview_id: String,
    build_fingerprint: Option<String>,
    width: u32,
    height: u32,
}

impl FoundryTextureIdentity {
    fn new(preview_id: &str, build: Option<&FoundryBuildStamp>, width: u32, height: u32) -> Self {
        Self {
            preview_id: preview_id.to_owned(),
            build_fingerprint: build.map(|build| build.build_fingerprint.0.to_hex()),
            width,
            height,
        }
    }

    fn texture_name(&self) -> String {
        format!(
            "foundry-preview-{}-{}x{}-{}",
            self.preview_id,
            self.width,
            self.height,
            self.build_fingerprint.as_deref().unwrap_or("no-build")
        )
    }
}

impl FoundryTextureCache {
    fn clear(&mut self) {
        self.textures.clear();
    }

    fn texture(
        &mut self,
        ctx: &egui::Context,
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        rgba8: &[u8],
        width: u32,
        height: u32,
    ) -> egui::TextureHandle {
        let identity = FoundryTextureIdentity::new(preview_id, build, width, height);
        if let Some(cached) = self
            .textures
            .get(preview_id)
            .filter(|cached| cached.identity == identity)
        {
            return cached.texture.clone();
        }

        let preview_pixels = preview_pixels_with_transparent_matte(rgba8, width, height);
        let color_image =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &preview_pixels);
        let texture =
            ctx.load_texture(identity.texture_name(), color_image, TextureOptions::LINEAR);
        self.textures.insert(
            preview_id.to_owned(),
            CachedFoundryTexture {
                identity,
                texture: texture.clone(),
            },
        );
        texture
    }
}

fn preview_pixels_with_transparent_matte(rgba8: &[u8], width: u32, height: u32) -> Vec<u8> {
    let expected_len = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if width == 0 || height == 0 || rgba8.len() != expected_len {
        return rgba8.to_vec();
    }
    let matte = [rgba8[0], rgba8[1], rgba8[2]];
    let is_dark_matte = matte.iter().all(|value| *value <= 48);
    if !is_dark_matte {
        return rgba8.to_vec();
    }
    let mut pixels = rgba8.to_vec();
    for chunk in pixels.chunks_exact_mut(4) {
        let dr = i32::from(chunk[0]) - i32::from(matte[0]);
        let dg = i32::from(chunk[1]) - i32::from(matte[1]);
        let db = i32::from(chunk[2]) - i32::from(matte[2]);
        let distance = dr * dr + dg * dg + db * db;
        if distance <= 20 * 20 {
            chunk[3] = 0;
        }
    }
    pixels
}

struct FoundryPreviewDraw<'a> {
    preview_id: &'a str,
    build: Option<&'a FoundryBuildStamp>,
    rgba8: &'a [u8],
    width: u32,
    height: u32,
    max_edge: f32,
}

fn draw_quick_template_preview(ui: &mut egui::Ui, max_edge: f32, _asset_name: &str) {
    let edge = max_edge.clamp(180.0, 360.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(edge, edge), egui::Sense::hover());
    let colors = VisualFoundryTokens::dark().colors;
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, colors.panel_elevated);
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0, colors.stroke),
        egui::StrokeKind::Inside,
    );

    let center = rect.center();
    let body = egui::Rect::from_center_size(center, egui::vec2(edge * 0.58, edge * 0.44));
    painter.rect_filled(body, 6.0, colors.accent_soft);
    painter.rect_stroke(
        body,
        6.0,
        egui::Stroke::new(2.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    for inset in [0.08, 0.18] {
        let outline = body.shrink(edge * inset);
        painter.rect_stroke(
            outline,
            4.0,
            egui::Stroke::new(1.25, colors.stroke),
            egui::StrokeKind::Inside,
        );
    }
}

fn show_rgba_preview(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    preview: FoundryPreviewDraw<'_>,
) {
    let width_usize = preview.width as usize;
    let height_usize = preview.height as usize;
    let expected_len = width_usize.saturating_mul(height_usize).saturating_mul(4);
    if preview.width == 0 || preview.height == 0 || preview.rgba8.len() != expected_len {
        let size = egui::vec2(preview.max_edge, preview.max_edge);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let colors = VisualFoundryTokens::dark().colors;
        ui.painter().rect_filled(rect, 6.0, colors.panel_elevated);
        ui.painter().rect_stroke(
            rect,
            6.0,
            egui::Stroke::new(1.0, colors.stroke),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Preview pending",
            egui::FontId::proportional(12.0),
            colors.text_muted,
        );
        return;
    }

    let texture = texture_cache.texture(
        ui.ctx(),
        preview.preview_id,
        preview.build,
        preview.rgba8,
        preview.width,
        preview.height,
    );
    if let Some(size) = scaled_preview_size(preview.width, preview.height, preview.max_edge) {
        ui.image((texture.id(), size));
    }
}

fn scaled_preview_size(width: u32, height: u32, max_edge: f32) -> Option<egui::Vec2> {
    if width == 0 || height == 0 || max_edge <= 0.0 {
        return None;
    }
    let scale = (max_edge / width as f32).min(max_edge / height as f32);
    Some(egui::vec2(width as f32 * scale, height as f32 * scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_foundry::compile_foundry_document;

    #[test]
    fn desktop_foundry_effects_execute_background_jobs() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(app.state.current_output.is_some());
        assert!(app.state.current_preview.is_some() || !app.state.active_jobs.is_empty());
    }

    #[test]
    fn save_as_paths_use_loadable_foundry_suffix() {
        let normalized = normalize_foundry_project_path(PathBuf::from("box-primitive.json"));
        assert_eq!(
            normalized,
            PathBuf::from("box-primitive.shapelab-foundry.json")
        );
        ensure_foundry_project_path(&normalized).expect("normalized path is loadable");
    }

    #[test]
    fn desktop_foundry_pack_action_dispatches_through_reducer() {
        let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
        let app = FoundryDesktopApp {
            state: FoundryAppState::new(fixture.document).expect("fixture state"),
            ..FoundryDesktopApp::default()
        };

        assert!(matches!(
            app.add_current_to_pack_command()
                .and_then(|command| command.single_foundry_command().cloned()),
            Some(shape_foundry::FoundryCommand::AddCurrentToPack { .. })
        ));
    }

    #[test]
    fn pack_member_ids_increment_for_repeated_ui_adds() {
        let mut pack = crate::foundry::view_model::FoundryPackView::default();
        assert_eq!(
            unique_pack_member_id(&pack, "box-primitive-doc"),
            "box-primitive-doc"
        );

        pack.members.insert(
            "box-primitive-doc".to_owned(),
            shape_foundry::FoundryDocumentId("box-primitive-doc".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "box-primitive-doc"),
            "box-primitive-doc-2"
        );

        pack.members.insert(
            "box-primitive-doc-2".to_owned(),
            shape_foundry::FoundryDocumentId("box-primitive-doc-2".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "box-primitive-doc"),
            "box-primitive-doc-3"
        );
    }

    #[test]
    fn product_app_launches_on_choose_home() {
        let app = FoundryDesktopApp::default();

        assert_eq!(app.tab, FoundryTab::Home);
        assert!(app.state.document.is_none());
    }

    #[test]
    fn product_home_shows_curated_usable_kits_by_default_and_preview_mode_hides_drafts() {
        assert_eq!(installed_product_kit_count(), 1);
        assert_eq!(default_product_home_profile_count(), 1);

        let default_profiles = product_home_profiles(false);
        let default_labels = default_profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(default_profiles[0].fixture.slug, "box-primitive");
        assert_eq!(default_labels, vec!["Box Primitive"]);

        let profiles = product_home_profiles(true);
        let labels = profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(profiles.len(), 1);
        assert_eq!(labels, vec!["Box Primitive"]);
        assert!(!labels.iter().any(|label| label.contains("MVP")));
    }

    #[test]
    fn choose_screen_single_profile_mode_has_no_category_filters_or_catalog_count() {
        let profiles = product_home_profiles(false);
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n");

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].fixture.slug, BOX_PRIMITIVE_PROFILE_ID);
        assert!(HOME_TEMPLATE_FILTERS.is_empty());
        assert!(strings.contains(&"Start with Box Primitive."));
        assert!(strings.contains(&HOME_SUBTITLE));
        assert!(strings.contains(&HOME_CONTROL_COPY));

        for hidden in [
            "Props",
            "Architecture",
            "Gear",
            "Furniture",
            "Environment",
            "18 templates",
            "1 starting point",
            "Search starting point...",
            "Choose what to make",
        ] {
            assert!(
                !joined.contains(hidden),
                "single-profile Choose copy should not expose {hidden}: {joined}"
            );
        }
    }

    #[test]
    fn product_home_profiles_group_by_asset_family() {
        let profiles = product_home_profiles(true);
        let groups = product_home_profile_groups(profiles.clone());
        let family_names = groups
            .iter()
            .map(|group| group.family_name.as_str())
            .collect::<Vec<_>>();
        let mut sorted_family_names = family_names.clone();
        sorted_family_names.sort_unstable();

        assert_eq!(family_names, sorted_family_names);
        assert_eq!(family_names, vec!["Box Primitive"]);
        assert!(!family_names.iter().any(|name| name.contains("MVP")));
        assert!(
            groups
                .iter()
                .all(|group| !group.family_id.is_empty() && !group.family_name.is_empty())
        );

        let expected_labels = profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let actual_labels = groups
            .iter()
            .flat_map(|group| group.profiles.iter().map(|profile| profile.label.as_str()))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual_labels, expected_labels);
        assert_eq!(
            groups
                .iter()
                .map(|group| group.profiles.len())
                .sum::<usize>(),
            profiles.len()
        );
    }

    #[test]
    fn home_template_search_defaults_to_first_matching_profile() {
        let profiles = product_home_profiles(false);
        let selected_slug =
            default_filtered_home_profile_slug(&profiles, "box", HomeTemplateFilter::All);

        assert_eq!(selected_slug.as_deref(), Some("box-primitive"));
    }

    #[test]
    fn home_template_selection_tracks_filter_visibility() {
        let profiles = product_home_profiles(false);
        let mut selected_slug = Some("box-primitive".to_owned());

        normalize_home_selection(&profiles, "", HomeTemplateFilter::All, &mut selected_slug);

        assert_eq!(selected_slug.as_deref(), Some("box-primitive"));
        assert!(
            filtered_home_profile_indices(&profiles, "", HomeTemplateFilter::All)
                .iter()
                .all(|index| profiles[*index].fixture.slug == "box-primitive")
        );
    }

    #[test]
    fn product_home_grouping_uses_stable_family_ids() {
        let profiles = product_home_profiles(true);
        let groups = product_home_profile_groups(profiles);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].family_id, "box_primitive");
        assert_eq!(groups[0].profiles.len(), 1);
    }

    #[test]
    fn preview_draw_size_scales_to_model_centric_stage() {
        assert_eq!(
            scaled_preview_size(128, 128, 420.0).expect("valid preview"),
            egui::vec2(420.0, 420.0)
        );
        assert_eq!(
            scaled_preview_size(512, 256, 256.0).expect("valid preview"),
            egui::vec2(256.0, 128.0)
        );
        assert_eq!(
            scaled_preview_size(128, 64, 320.0).expect("valid preview"),
            egui::vec2(320.0, 160.0)
        );
        assert!(scaled_preview_size(0, 128, 256.0).is_none());
    }

    #[test]
    fn home_thumbnail_drag_delta_orbits_camera() {
        let camera = OrbitCamera::default();
        let rotated = orbit_home_thumbnail_camera(&camera, egui::vec2(20.0, -10.0));

        assert!((rotated.yaw_degrees - 44.0).abs() < f32::EPSILON);
        assert_eq!(rotated.pitch_degrees, camera.pitch_degrees);
        assert_eq!(rotated.target, camera.target);
        assert_eq!(rotated.distance, camera.distance);
    }

    #[test]
    fn home_turntable_frame_index_wraps_yaw() {
        assert_eq!(home_turntable_frame_index(0.0), 0);
        assert_eq!(home_turntable_frame_index(359.0), 0);
        assert_eq!(home_turntable_frame_index(15.0), 1);
        assert_eq!(home_turntable_frame_index(-15.0), 23);
        assert_eq!(home_turntable_frame_distance(0, 23), 1);
    }

    #[test]
    fn current_preview_pixels_are_dpi_aware_and_capped() {
        assert_eq!(
            current_preview_pixels_for_scale(1.0),
            DEFAULT_PREVIEW_PIXELS
        );
        assert_eq!(
            current_preview_pixels_for_scale(1.5),
            DEFAULT_PREVIEW_PIXELS
        );
        assert_eq!(
            current_preview_pixels_for_scale(0.5),
            DEFAULT_PREVIEW_PIXELS
        );
        assert_eq!(
            current_preview_pixels_for_scale(4.0),
            MAX_CURRENT_PREVIEW_PIXELS
        );
    }

    #[test]
    fn default_product_strings_hide_legacy_and_technical_surfaces() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n");
        let joined_lower = joined.to_ascii_lowercase();

        for forbidden in [
            "Legacy",
            "Implicit",
            "Asset Modeling Lab",
            "Modeling Workspace",
            "Advanced Recipe",
            "From Existing Recipe",
            "scalar",
            "provider",
            "semantic",
            "operation",
            "compiler",
            "decompiler",
            "Build model",
            "Preview model",
            "toolbar",
        ] {
            assert!(
                !joined_lower.contains(&forbidden.to_ascii_lowercase()),
                "default product strings unexpectedly contain {forbidden}: {joined}"
            );
        }
    }

    #[test]
    fn semantic_clay_docs_keep_preview_display_separate_from_material_support() {
        let docs = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/SEMANTIC_CLAY_PREVIEW_MODE.md"
        ));
        let lower = docs.to_ascii_lowercase();

        assert!(docs.contains("untextured display shading only"));
        assert!(docs.contains("Pure Clay remains the strict mesh gate"));
        assert!(docs.contains("Quality reports must record Pure Clay pass/fail separately"));
        assert!(docs.contains("DiagnosticPartColor: developer/author diagnostic mode only"));
        assert!(lower.contains("does not imply uv/texturing support"));
        assert!(lower.contains("affect export payloads"));

        for forbidden in [
            "uv/texturing support is approved",
            "texture files are supported",
            "material editor is supported",
            "broad surface mode is approved",
        ] {
            assert!(
                !lower.contains(forbidden),
                "Semantic Clay docs must not overclaim material or texturing support: {forbidden}"
            );
        }

        assert_eq!(
            shape_foundry::FoundryPreviewDisplayMode::novice_default(&[]),
            shape_foundry::FoundryPreviewDisplayMode::PureClay
        );
        let assignments = vec![shape_foundry::SemanticClayRoleAssignment::new(
            "body",
            "Primary Mass",
            0.72,
            10,
            true,
        )];
        assert_eq!(
            shape_foundry::FoundryPreviewDisplayMode::novice_default(&assignments),
            shape_foundry::FoundryPreviewDisplayMode::SemanticClay
        );
        assert!(
            !shape_foundry::FoundryPreviewDisplayMode::DiagnosticPartColor.default_novice_safe()
        );
        let strings = product_visible_strings_for_default_shell().join("\n");
        assert!(!strings.contains("DiagnosticPartColor"));
        assert!(!strings.contains("Surface mode"));
        assert!(!strings.contains("Texturing"));
    }

    #[test]
    fn source_and_markdown_hygiene_targets_are_audit_friendly() {
        let targets = [
            (
                "README.md",
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")),
            ),
            (
                "docs/CURRENT_PRODUCT_STATUS.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/CURRENT_PRODUCT_STATUS.md"
                )),
            ),
            (
                "docs/SURFACE_CANDIDATE_V0_INTEGRATION_REPORT.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/SURFACE_CANDIDATE_V0_INTEGRATION_REPORT.md"
                )),
            ),
            (
                "docs/SCIFI_CRATE_VISUAL_SURFACE_CANDIDATES_V0.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/SCIFI_CRATE_VISUAL_SURFACE_CANDIDATES_V0.md"
                )),
            ),
            (
                "docs/SURFACE_MODE_DOGFOOD_V0_RESULTS.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/SURFACE_MODE_DOGFOOD_V0_RESULTS.md"
                )),
            ),
            (
                "docs/NEXT_PRODUCT_STEP_AFTER_DOGFOOD_V4.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/NEXT_PRODUCT_STEP_AFTER_DOGFOOD_V4.md"
                )),
            ),
            (
                "docs/FAMILY_FOUNDATION_PIVOT.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/FAMILY_FOUNDATION_PIVOT.md"
                )),
            ),
            (
                "docs/FAMILY_MATURITY_LADDER.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/FAMILY_MATURITY_LADDER.md"
                )),
            ),
            (
                "docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/NEXT_WORK_AFTER_FAMILY_PIVOT.md"
                )),
            ),
            (
                "docs/SOURCE_FORMAT_HYGIENE_REPORT.md",
                include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../docs/SOURCE_FORMAT_HYGIENE_REPORT.md"
                )),
            ),
        ];

        for (path, contents) in targets {
            let line_count = contents.lines().count();
            let max_line_len = contents.lines().map(str::len).max().unwrap_or_default();
            assert!(line_count >= 20, "{path} is too short to be audit-friendly");
            assert!(
                max_line_len <= 180,
                "{path} has a line longer than 180 characters: {max_line_len}"
            );
        }

        let app_source = include_str!("app.rs");
        let app_line_count = app_source.lines().count();
        let app_max_line_len = app_source.lines().map(str::len).max().unwrap_or_default();
        assert!(
            app_line_count > 1_000,
            "foundry app.rs appears physically collapsed: {app_line_count} lines"
        );
        assert!(
            app_max_line_len <= 180,
            "foundry app.rs has a line longer than 180 characters: {app_max_line_len}"
        );
    }

    #[test]
    fn product_docs_keep_surface_rig_motion_and_game_ready_claims_caveated() {
        let docs = [
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md")),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/CURRENT_PRODUCT_STATUS.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/PRODUCT_RECOVERY_INTEGRATION_V2_REPORT.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/RELEASE_CANDIDATE_MANUAL_GATE.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/FOUNDRY_UI_MANUAL_GATE.md"
            )),
        ];
        let joined = docs.join("\n").to_ascii_lowercase();

        for forbidden in [
            "broad uv/texturing support is ready",
            "broad texturing support is ready",
            "rigging integration is ready",
            "animation integration is ready",
            "full game-ready support",
            "showcase claim approved",
        ] {
            assert!(
                !joined.contains(forbidden),
                "product docs must not overclaim unsupported work: {forbidden}"
            );
        }

        for claim in ["game-ready", "rigging", "animation", "texturing"] {
            if joined.contains(claim) {
                assert!(
                    joined.contains("do not claim")
                        || joined.contains("outside")
                        || joined.contains("not product-supported")
                        || joined.contains("blocked")
                        || joined.contains("manual"),
                    "mentions of {claim} must remain caveated"
                );
            }
        }
    }

    #[test]
    fn make_pipeline_reliability_docs_cover_recovery_contract() {
        let docs = [
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/MAKE_PIPELINE_RELIABILITY.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/MAKE_NO_DEAD_END_STATES.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/MAKE_CANVAS_STATE_MACHINE.md"
            )),
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../docs/FOUNDRY_UI_MANUAL_GATE.md"
            )),
        ];
        let joined = docs.join("\n");

        for required in [
            "Preparing model",
            "Rendering preview",
            "Ready",
            PREPARATION_TIMEOUT_MESSAGE,
            ACTION_RETRY_PREPARATION,
            ACTION_CHOOSE_ANOTHER_TEMPLATE,
            ACTION_OPEN_PROJECT,
            PREVIEW_UPDATING_REASON,
            ACTION_UPDATE_PREVIEW,
            "Ready to try ideas",
            "Trying ideas",
            "No clear ideas survived",
            "EmptyReady",
            "GeneratingSkeletons",
            "HasCandidates",
            "NoCandidatesWithRecovery",
            "ErrorWithRecovery",
            STALE_RESULT_WARNING,
            ACTION_TRY_AGAIN,
        ] {
            assert!(
                joined.contains(required),
                "Make reliability docs missing {required}"
            );
        }
    }

    #[test]
    fn box_primitive_default_shell_hides_surface_export_copy() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n").to_ascii_lowercase();
        for hidden in [
            shape_foundry::STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL,
            shape_foundry::STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION,
            shape_foundry::STATIC_PROP_FULL_READY_BLOCKED_NOTE,
            ACTION_TRY_MATERIAL_LOOKS,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
            MATERIAL_LOOK_PREVIEW_ONLY_COPY,
            MATERIAL_LOOK_EXPORT_INCLUDED_COPY,
            MATERIAL_LOOK_FULL_READY_BLOCKED_COPY,
            SURFACE_PACKAGE_COMMAND_COPY,
            SURFACE_PACKAGE_COMMAND,
            "static surface package",
        ] {
            assert!(
                !strings.contains(&hidden),
                "Box Primitive default shell should not expose {hidden}"
            );
        }
        for overclaim in [
            "game-ready textured asset",
            "visual foundry previews are textured",
            "unity package",
            "unreal package",
            "godot package",
            "surface mode ready",
            "material editor",
            "rigartifact",
            "motionartifact",
            "skeleton template",
            "joint id",
            "skinning",
            "retarget",
        ] {
            assert!(
                !joined.contains(overclaim),
                "export copy should not overclaim {overclaim}: {joined}"
            );
        }
    }

    #[test]
    fn material_look_evidence_loader_requires_valid_box_package() {
        let root = temp_material_look_package_root("valid-material-looks");
        let report = write_test_material_look_evidence(
            &root,
            TestMaterialLookEvidenceOptions {
                frozen_mesh_fingerprint: "box-fingerprint",
                ..TestMaterialLookEvidenceOptions::default()
            },
        );

        let evidence = load_material_look_evidence(&report, Some("box-fingerprint"))
            .expect("valid evidence loads");

        assert_eq!(evidence.candidates.len(), MATERIAL_LOOK_TITLES.len());
        assert!(full_ready_blockers_are_honest(
            &evidence.full_ready_blocker_codes
        ));
        for (candidate, title) in evidence.candidates.iter().zip(MATERIAL_LOOK_TITLES) {
            assert_eq!(candidate.display_name, title);
            assert!(
                candidate
                    .textured_preview_ref
                    .ends_with("textured-preview.png")
            );
            assert!(
                candidate
                    .material_override_ref
                    .ends_with("material-override.json")
            );
            assert!(candidate.surface_delta_ref.ends_with("surface-delta.json"));
            assert!(candidate.validation_ref.ends_with("validation.json"));
            assert_eq!(candidate.width, 2);
            assert_eq!(candidate.height, 2);
            assert_eq!(candidate.rgba8.len(), 16);
        }

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn material_look_evidence_loader_rejects_missing_mismatch_and_shape_delta() {
        let missing_root = temp_material_look_package_root("missing-material-looks");
        let missing_report = missing_root.join("surface/variants/surface-candidate-report.json");
        assert_eq!(
            load_material_look_evidence(&missing_report, None).expect_err("missing rejects"),
            MATERIAL_LOOK_MISSING_MESSAGE
        );

        let mismatch_root = temp_material_look_package_root("mismatch-material-looks");
        let mismatch_report = write_test_material_look_evidence(
            &mismatch_root,
            TestMaterialLookEvidenceOptions {
                frozen_mesh_fingerprint: "expected-fingerprint",
                ..TestMaterialLookEvidenceOptions::default()
            },
        );
        assert!(
            load_material_look_evidence(&mismatch_report, Some("different-fingerprint"))
                .expect_err("mismatch rejects")
                .contains("do not match")
        );

        let leak_root = temp_material_look_package_root("shape-delta-material-looks");
        let leak_report = write_test_material_look_evidence(
            &leak_root,
            TestMaterialLookEvidenceOptions {
                frozen_mesh_fingerprint: "box-fingerprint",
                shape_delta_leak: true,
                ..TestMaterialLookEvidenceOptions::default()
            },
        );
        assert!(
            load_material_look_evidence(&leak_report, Some("box-fingerprint"))
                .expect_err("shape delta rejects")
                .contains("shape change")
        );

        let _ = std::fs::remove_dir_all(missing_root);
        let _ = std::fs::remove_dir_all(mismatch_root);
        let _ = std::fs::remove_dir_all(leak_root);
    }

    #[test]
    fn material_look_action_disabled_for_box_and_open_uses_recovery_copy() {
        let mut box_app = ready_visible_state_test_app();
        let visible = box_app.make_canvas_view_state();
        assert!(!box_app.material_look_action_visible(&visible));

        let missing_root = temp_material_look_package_root("missing-action-material-looks");
        box_app.material_looks.evidence_report_path =
            Some(missing_root.join("surface/variants/surface-candidate-report.json"));
        box_app.open_material_looks_panel();
        assert!(box_app.material_looks.tray_open);
        assert!(!box_app.make_canvas_view_state().material_look_tray_visible);
        assert_eq!(
            box_app.material_looks.load_error.as_deref(),
            Some(MATERIAL_LOOK_MISSING_MESSAGE)
        );

        let _ = std::fs::remove_dir_all(missing_root);
    }

    #[test]
    fn selecting_material_look_is_preview_only_and_preserves_geometry_state() {
        let mut app = ready_visible_state_test_app();
        let fingerprint = app
            .current_artifact_fingerprint_hex()
            .expect("build fingerprint");
        let root = temp_material_look_package_root("select-material-looks");
        let report = write_test_material_look_evidence(
            &root,
            TestMaterialLookEvidenceOptions {
                frozen_mesh_fingerprint: fingerprint.as_str(),
                ..TestMaterialLookEvidenceOptions::default()
            },
        );
        let before_build = app.state.current_build.clone();
        let before_controls = app.state.controls.clone();

        app.material_looks.evidence_report_path = Some(report);
        app.open_material_looks_panel();
        let second_id = app
            .material_looks
            .evidence
            .as_ref()
            .expect("evidence")
            .candidates[1]
            .candidate_id
            .clone();
        app.material_looks.selected_candidate_id = Some(second_id);

        assert_eq!(app.state.current_build, before_build);
        assert_eq!(app.state.controls, before_controls);
        assert_eq!(app.material_look_export_copy(), None);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn box_primitive_default_copy_hides_material_look_terms() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n").to_ascii_lowercase();

        for hidden in [
            ACTION_TRY_MATERIAL_LOOKS,
            MATERIAL_LOOK_SECTION_TITLE,
            MATERIAL_LOOK_SURFACE_ONLY_COPY,
            MATERIAL_LOOK_GEOMETRY_UNCHANGED_COPY,
            "Current Material",
            "Candidate Material",
            MATERIAL_LOOK_MISSING_MESSAGE,
        ] {
            assert!(
                !strings.contains(&hidden),
                "Box Primitive default copy should not expose {hidden}"
            );
        }

        for forbidden in [
            "surface",
            "surfaceartifact",
            "material looks",
            "uv set",
            "material slot id",
            "texture file path",
            "gltf primitive",
            "rigging",
            "game-ready surface",
        ] {
            assert!(
                !joined.contains(forbidden),
                "material look copy leaked {forbidden}: {joined}"
            );
        }
        assert!(strings.contains(&BOX_PRIMITIVE_EXPORT_LIMITATION));
    }

    #[test]
    fn box_primitive_default_copy_has_no_crate_case_or_part_focus_language() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n").to_ascii_lowercase();

        for forbidden in [
            "crate",
            "case",
            "cargo",
            "sci-fi",
            "body",
            "body chip",
            "parts",
            "focus",
            "focused part",
            "focus part",
            "part chip",
            "family studio",
        ] {
            assert!(
                !joined.contains(forbidden),
                "Box Primitive default UI copy should not expose {forbidden}: {joined}"
            );
        }
    }

    #[test]
    fn box_primitive_default_copy_has_no_unsupported_pipeline_overclaim() {
        let strings = product_visible_strings_for_default_shell();
        let joined = strings.join("\n").to_ascii_lowercase();

        for forbidden in ["uv", "texturing", "rigging", "animation"] {
            assert!(
                !joined.contains(forbidden),
                "Box Primitive UI copy should not expose unsupported pipeline term {forbidden}: {joined}"
            );
        }
        assert!(strings.contains(&BOX_PRIMITIVE_EXPORT_LIMITATION));
        assert!(
            joined.contains("not a textured, rigged, animated, or game-ready package"),
            "export copy must explicitly block game-ready overclaim: {joined}"
        );
    }

    #[test]
    fn product_shell_steps_are_novice_facing() {
        let strings = product_visible_strings_for_default_shell();

        for required in [
            "Start with Box Primitive",
            "Choose",
            "Make",
            "Export",
            "Visual Foundry",
            "Project",
            "Open Project",
            "Save Project",
            "Save Project As",
            "Start Another Asset",
            "History",
            ACTION_CHOOSE_TEMPLATE,
            "Start",
        ] {
            assert!(
                strings.contains(&required),
                "missing product string {required}"
            );
        }
    }

    #[test]
    fn make_canvas_product_copy_replaces_old_primary_modes() {
        let strings = product_visible_strings_for_default_shell();
        let steps = WORKFLOW_STEPS
            .iter()
            .map(|step| step.label)
            .collect::<Vec<_>>();
        assert_eq!(steps, vec!["Choose", "Make"]);
        assert!(!steps.contains(&"Directions"));
        assert!(!steps.contains(&"Customize"));
        assert!(!steps.contains(&"Pack"));
        assert!(!steps.contains(&"Export"));

        for forbidden in [
            "Variation Mode",
            "Variation mode",
            "Complete Looks",
            "Candidate tray",
            "Model workspace",
            "Focus Part",
            ACTION_FOCUS,
            "Generate 6 Directions",
            "Try body ideas",
            "Material looks are not previewable yet.",
        ] {
            assert!(
                !strings.contains(&forbidden),
                "Make canvas product copy should not expose {forbidden}"
            );
        }

        for required in [
            "Use this idea",
            ACTION_TRY_BOX_IDEAS,
            ACTION_CHOOSE_DIRECTION,
            "Ideas",
        ] {
            assert!(
                strings.contains(&required),
                "missing Make canvas product copy {required}"
            );
        }
    }

    #[test]
    fn make_canvas_view_state_tracks_generated_candidates_and_comparison() {
        let mut app = visible_state_test_app();
        app.state.current_preview = Some(test_preview_image("current"));
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        let selected = shape_foundry::FoundryCandidateId("candidate-a".to_owned());
        app.state.selected_candidate = Some(selected.clone());
        app.state.candidates = vec![test_candidate_card(&selected.0, true, None)];

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(visible.candidate_count, 1);
        assert!(visible.candidate_tray_visible);
        assert!(visible.selected_candidate_present);
        assert!(visible.selected_comparison_visible);
        assert_eq!(visible.primary_action_label, ACTION_CHOOSE_DIRECTION);
        assert_eq!(visible.next_action_hint, "Use this idea, or reject it.");
    }

    #[test]
    fn make_canvas_view_state_requires_selected_candidate_for_comparison() {
        let mut app = visible_state_test_app();
        app.state.current_preview = Some(test_preview_image("current"));
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.candidates = vec![test_candidate_card("candidate-a", false, None)];

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(visible.candidate_count, 1);
        assert!(visible.candidate_tray_visible);
        assert!(!visible.selected_candidate_present);
        assert!(visible.selected_comparison_visible);
        assert_eq!(visible.primary_action_label, ACTION_CHOOSE_DIRECTION);
        assert_eq!(visible.next_action_hint, "Use this idea, or reject it.");
    }

    #[test]
    fn box_primitive_ignores_stale_focus_part_scope() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        set_test_focus_scope(&mut app, "body", "Body");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::Ready);
        assert_eq!(visible.primary_title, "Box Primitive");
        assert!(visible.focused_part_label.is_none());
        assert!(!visible.focused_part_visible);
        assert!(!visible.focused_part_actions_visible);
        assert_eq!(visible.primary_action_label, ACTION_TRY_BOX_IDEAS);
        assert_eq!(
            visible.next_action_hint,
            "Try box ideas, adjust box, Add to Pack, or Export."
        );
    }

    #[test]
    fn box_primitive_focus_body_does_not_change_visible_scope_or_action() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        set_test_focus_scope(&mut app, "body", "Body");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::Ready);
        assert_eq!(visible.primary_title, "Box Primitive");
        assert!(visible.focused_part_label.is_none());
        assert!(!visible.focused_part_visible);
        assert_eq!(visible.primary_action_label, ACTION_TRY_BOX_IDEAS);
    }

    #[test]
    fn make_canvas_primary_action_changes_by_state() {
        let ready = ready_visible_state_test_app().make_canvas_view_state();
        assert_eq!(ready.mode, MakeCanvasMode::Ready);
        assert_eq!(ready.primary_action_label, ACTION_TRY_BOX_IDEAS);
        assert!(ready.primary_action_enabled);

        let mut focused = ready_visible_state_test_app();
        set_test_focus_scope(&mut focused, "body", "Body");
        let focused = focused.make_canvas_view_state();
        assert_eq!(focused.mode, MakeCanvasMode::Ready);
        assert_eq!(focused.primary_action_label, ACTION_TRY_BOX_IDEAS);

        let mut generating = ready_visible_state_test_app();
        generating
            .state
            .request_candidates(FoundryCandidateRequest {
                seed: 1,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::complete_look(),
            })
            .expect("candidate job schedules");
        let generating = generating.make_canvas_view_state();
        assert_eq!(generating.primary_action_label, ACTION_GENERATING_IDEAS);
        assert!(!generating.primary_action_enabled);

        let mut reviewing = ready_visible_state_test_app();
        reviewing.state.candidates = vec![test_candidate_card("candidate-a", false, None)];
        let reviewing = reviewing.make_canvas_view_state();
        assert_eq!(reviewing.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(reviewing.primary_action_label, ACTION_CHOOSE_DIRECTION);
        assert!(reviewing.primary_action_enabled);
    }

    #[test]
    fn focused_inspector_filters_controls_and_keeps_show_all_available() {
        let controls = vec![
            test_control_view("proportions", "Proportions"),
            test_control_view("edge_softness", "Edge Softness"),
            test_control_view("box_profile", "Box Profile"),
            test_control_view("draft_note", "Draft Note"),
        ];
        let body = directions::DirectionPartGroup {
            group_id: "body".to_owned(),
            label: "Body".to_owned(),
            focusable: true,
            unavailable_reason: None,
        };

        let sections = make_context_inspector_controls(&controls, Some(&body));
        let visible_ids = sections
            .visible
            .iter()
            .map(|control| control.id.as_str())
            .collect::<Vec<_>>();
        let overflow_ids = sections
            .overflow
            .iter()
            .map(|control| control.id.as_str())
            .collect::<Vec<_>>();

        assert!(visible_ids.contains(&"proportions"));
        assert!(visible_ids.contains(&"edge_softness"));
        assert!(visible_ids.contains(&"box_profile"));
        assert!(!visible_ids.contains(&"draft_note"));
        assert!(overflow_ids.contains(&"draft_note"));
        assert_eq!(sections.disclosure_label, "Show all controls");
    }

    #[test]
    fn whole_asset_inspector_starts_with_short_control_list() {
        let controls = vec![
            test_control_view("proportions", "Proportions"),
            test_control_view("edge_softness", "Edge Softness"),
            test_control_view("box_profile", "Box Profile"),
            test_control_view("draft_note", "Draft Note"),
        ];

        let sections = make_context_inspector_controls(&controls, None);

        assert_eq!(sections.visible.len(), MAKE_CONTEXT_INITIAL_CONTROL_LIMIT);
        assert_eq!(
            sections.overflow.len(),
            controls.len() - MAKE_CONTEXT_INITIAL_CONTROL_LIMIT
        );
        assert_eq!(sections.disclosure_label, "More controls");
    }

    #[test]
    fn focused_generation_state_does_not_render_whole_asset_heading() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        set_test_focus_scope(&mut app, "body", "Body");
        let selected = shape_foundry::FoundryCandidateId("body-candidate".to_owned());
        app.state.current_preview = Some(test_preview_image("current"));
        app.state.selected_candidate = Some(selected.clone());
        app.state.candidates = vec![test_candidate_card(
            &selected.0,
            true,
            Some("Body".to_owned()),
        )];

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.primary_title, "Box Primitive");
        assert_eq!(visible.candidate_count, 1);
        assert!(visible.candidate_tray_visible);
        assert!(visible.selected_candidate_present);
        assert!(visible.selected_comparison_visible);
        assert_ne!(visible.primary_title, "Whole asset");
    }

    #[test]
    fn pack_and_export_actions_open_visible_drawer_state() {
        let mut app = visible_state_test_app();
        app.drawer = Some(FoundryDrawer::Pack);
        let pack_visible = app.make_canvas_view_state();
        assert_eq!(pack_visible.mode, MakeCanvasMode::PackDrawerOpen);
        assert!(pack_visible.pack_drawer_visible);
        assert!(!pack_visible.export_drawer_visible);

        app.drawer = Some(FoundryDrawer::Export);
        let export_visible = app.make_canvas_view_state();
        assert_eq!(export_visible.mode, MakeCanvasMode::ExportDrawerOpen);
        assert!(export_visible.export_drawer_visible);
        assert!(!export_visible.pack_drawer_visible);
    }

    #[test]
    fn screenshot_focus_scenario_helper_does_not_focus_box_body() {
        let mut app = visible_state_test_app();

        let commands = app.ensure_screenshot_focus("body");
        assert_eq!(app.screenshot_scenario_step, 0);
        assert!(commands.is_empty());
        assert_eq!(
            app.make_canvas_view_state().focused_part_label.as_deref(),
            None
        );

        set_test_focus_scope(&mut app, "body", "Body");
        let commands = app.ensure_screenshot_focus("body");
        assert!(commands.is_empty());
        assert_eq!(app.screenshot_scenario_step, 0);
        assert_eq!(
            app.make_canvas_view_state().focused_part_label.as_deref(),
            None
        );
    }

    #[test]
    fn screenshot_state_assertions_cover_required_make_scenarios() {
        let mut app = ready_visible_state_test_app();
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::MakeInitialBox,
                &app.make_canvas_view_state(),
            )
            .is_ok()
        );

        app.state
            .request_candidates(FoundryCandidateRequest {
                seed: 1,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::complete_look(),
            })
            .expect("candidate job schedules");
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::GeneratingBoxIdeas,
                &app.make_canvas_view_state(),
            )
            .is_ok()
        );

        app.state.active_jobs.clear();
        let selected = shape_foundry::FoundryCandidateId("candidate-a".to_owned());
        app.state.selected_candidate = Some(selected.clone());
        app.state.candidates = vec![test_candidate_card(&selected.0, true, None)];
        let view = app.make_canvas_view_state();
        assert!(
            screenshot_scenario_assertion(ScreenshotScenario::GeneratedBoxIdeas, &view).is_ok()
        );
        assert!(
            screenshot_scenario_assertion(ScreenshotScenario::SelectedComparison, &view).is_ok()
        );
        let adjusted = ready_visible_state_test_app().make_canvas_view_state();
        assert!(
            screenshot_scenario_assertion(ScreenshotScenario::AdjustedBoxControl, &adjusted)
                .is_ok()
        );

        app.drawer = Some(FoundryDrawer::Pack);
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::PackDrawer,
                &app.make_canvas_view_state()
            )
            .is_ok()
        );
        app.drawer = Some(FoundryDrawer::Export);
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::ExportDrawer,
                &app.make_canvas_view_state(),
            )
            .is_ok()
        );
    }

    #[test]
    fn preparing_asset_disables_idea_generation_with_local_reason() {
        let app = visible_state_test_app();

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
        assert!(!visible.primary_action_enabled);
        assert!(visible.quick_template_preview_visible);
        assert_eq!(
            visible.primary_action_disabled_reason.as_deref(),
            Some(ASSET_PREPARING_REASON)
        );
        assert_eq!(
            visible.local_busy_label.as_deref(),
            Some("Preparing Box Primitive...")
        );
        assert!(visible.local_busy_visible);
    }

    #[test]
    fn preparation_phases_timeout_and_recovery_actions_are_visible() {
        let mut app = visible_state_test_app();

        let visible = app.make_canvas_view_state();
        assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
        assert_eq!(
            visible.preparation_phase,
            MakePreparationPhase::PreparingModel
        );
        assert_eq!(visible.local_banner_title, "Preparing Box Primitive");
        assert_eq!(visible.local_banner_message, "Preparing model");
        assert!(!visible.preparation_fallback_visible);

        let output = compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &shape_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles");
        app.state.current_build = Some(output.build_stamp.clone());
        app.state.current_output = Some(Box::new(output));
        let visible = app.make_canvas_view_state();
        assert_eq!(
            visible.preparation_phase,
            MakePreparationPhase::RenderingPreview
        );
        assert!(!visible.quick_template_preview_visible);
        assert!(visible.preview_update_required);
        assert_eq!(visible.local_banner_message, "Rendering preview");

        app.state.current_preview = Some(test_preview_image_for_build(
            "current",
            app.state.current_build.clone(),
        ));
        let visible = app.make_canvas_view_state();
        assert_eq!(visible.preparation_phase, MakePreparationPhase::Ready);
        assert_eq!(visible.mode, MakeCanvasMode::Ready);
        assert_eq!(visible.local_banner_title, "Ready");

        let mut timed_out = visible_state_test_app();
        timed_out.make_preparation_started_at =
            Some(Instant::now() - PREPARATION_TIMEOUT - Duration::from_secs(1));
        let visible = timed_out.make_canvas_view_state();
        assert!(visible.preparation_timed_out);
        assert!(visible.preparation_fallback_visible);
        assert_eq!(visible.local_banner_message, PREPARATION_TIMEOUT_MESSAGE);

        for label in [
            ACTION_RETRY_PREPARATION,
            ACTION_CHOOSE_ANOTHER_TEMPLATE,
            ACTION_OPEN_PROJECT,
        ] {
            assert!(rendered_action_labels_for_default_shell().contains(&label));
        }
    }

    #[test]
    fn stale_preview_uses_update_preview_copy_and_no_legacy_make_actions() {
        let mut app = visible_state_test_app();
        let output = compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &shape_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles");
        app.state.current_build = Some(output.build_stamp.clone());
        app.state.current_output = Some(Box::new(output));
        app.state.current_preview = Some(test_preview_image("old-current"));

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
        assert!(visible.preview_updating);
        assert!(visible.preview_update_required);
        assert_eq!(
            visible.primary_action_disabled_reason.as_deref(),
            Some(PREVIEW_UPDATING_REASON)
        );
        assert_eq!(visible.local_banner_message, PREVIEW_UPDATING_REASON);
        assert_eq!(
            visible.next_action_hint,
            "Update preview to keep making changes."
        );

        let strings = product_visible_strings_for_default_shell();
        assert!(strings.contains(&ACTION_UPDATE_PREVIEW));
        assert!(!strings.contains(&"Build Asset"));
        assert!(!strings.contains(&"Refresh Preview"));
    }

    #[test]
    fn make_canvas_responsive_layout_keeps_adjust_visible_on_wide_short_viewports() {
        let app = visible_state_test_app();
        let visible = app.make_canvas_view_state();

        let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

        assert!(layout.compact_ideas);
        assert!(!layout.stacked_columns);
        assert!(layout.inspector_width >= 500.0);
        assert!(layout.tray_height <= 128.0);
        assert!(layout.top_height >= 700.0);
        assert!(layout.stage_width > layout.inspector_width * 2.0);
        assert!(!make_canvas_inspector_build_actions_visible(&visible));
    }

    #[test]
    fn make_canvas_responsive_layout_expands_ideas_when_candidates_exist() {
        let mut app = ready_visible_state_test_app();
        app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
        let visible = app.make_canvas_view_state();

        let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

        assert!(!layout.compact_ideas);
        assert!(layout.inline_ideas);
        assert_eq!(layout.tray_height, 0.0);
        assert_eq!(layout.top_height, 860.0);
        assert!(layout.ideas_width >= 650.0);
        assert!(make_canvas_inspector_build_actions_visible(&visible));
    }

    #[test]
    fn make_canvas_responsive_layout_hides_material_looks_tray_for_box() {
        let mut app = ready_visible_state_test_app();
        app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
        app.material_looks.tray_open = true;
        let visible = app.make_canvas_view_state();

        let layout = make_canvas_layout(egui::vec2(1900.0, 860.0), &visible);

        assert!(!visible.material_look_tray_visible);
        assert!(layout.inline_ideas);
        assert_eq!(layout.tray_height, 0.0);
        assert_eq!(layout.top_height, 860.0);
    }

    #[test]
    fn make_canvas_responsive_layout_preview_edge_uses_available_viewport_without_overgrowing() {
        assert_eq!(make_stage_preview_edge(1400.0, 900.0), 520.0);
        assert_eq!(make_stage_preview_edge(260.0, 220.0), 180.0);
        assert!((make_canvas_stacked_stage_height(360.0) - 201.6).abs() < 0.01);
        assert_eq!(compact_direction_grid_columns(734.0), 3);
        assert_eq!(compact_direction_card_preview_edge(330.0), 64.0);
    }

    #[test]
    fn candidate_tray_state_enum_covers_every_rendering_state() {
        let mut app = ready_visible_state_test_app();
        assert_eq!(
            app.make_canvas_view_state().candidate_tray_state,
            MakeCandidateTrayState::EmptyReady
        );

        app.state
            .request_candidates(FoundryCandidateRequest {
                seed: 1,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::complete_look(),
            })
            .expect("candidate job schedules");
        assert_eq!(
            app.make_canvas_view_state().candidate_tray_state,
            MakeCandidateTrayState::GeneratingSkeletons
        );

        app.state.active_jobs.clear();
        let mut pending_card = test_candidate_card("candidate-pending", false, None);
        pending_card.validation_label = "Preview pending".to_owned();
        pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
        pending_card.rgba8.clear();
        pending_card.width = 0;
        pending_card.height = 0;
        pending_card.camera = None;
        pending_card.selectable = false;
        app.state.candidates = vec![pending_card];
        let visible = app.make_canvas_view_state();
        assert_eq!(visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(
            visible.candidate_tray_state,
            MakeCandidateTrayState::HasCandidates
        );
        assert_eq!(visible.local_banner_title, "Rendering previews");

        app.state.candidates = vec![test_candidate_card("candidate-a", true, None)];
        assert_eq!(
            app.make_canvas_view_state().candidate_tray_state,
            MakeCandidateTrayState::HasCandidates
        );

        app.state.candidates.clear();
        app.state.candidate_output = Some(Box::new(empty_test_candidate_output(3, 1, 0)));
        assert_eq!(
            app.make_canvas_view_state().candidate_tray_state,
            MakeCandidateTrayState::NoCandidatesWithRecovery
        );

        app.state.status = Some("Candidate search failed locally.".to_owned());
        assert_eq!(
            app.make_canvas_view_state().candidate_tray_state,
            MakeCandidateTrayState::ErrorWithRecovery
        );
    }

    #[test]
    fn box_primitive_zero_candidates_show_whole_box_recovery_copy() {
        let mut app = ready_visible_state_test_app();
        set_test_focus_scope(&mut app, "body", "Body");
        app.state.candidates.clear();
        app.state.candidate_output = Some(Box::new(empty_test_candidate_output(4, 0, 0)));

        let visible = app.make_canvas_view_state();

        assert_eq!(
            visible.candidate_tray_state,
            MakeCandidateTrayState::NoCandidatesWithRecovery
        );
        assert!(visible.candidate_search_finished_empty);
        assert_eq!(visible.local_banner_title, "No clear ideas survived");
        assert!(!visible.local_banner_message.contains("body"));
        assert!(
            visible
                .local_banner_message
                .contains("hidden or too subtle")
        );
        assert!(!visible.focused_no_candidates_recovery_visible);
        assert_eq!(visible.primary_action_label, ACTION_TRY_BOX_IDEAS);
    }

    #[test]
    fn compiled_candidate_shells_show_before_all_previews_render() {
        let mut app = ready_visible_state_test_app();
        let mut pending_card = test_candidate_card("candidate-pending", true, None);
        pending_card.validation_label = "Preview pending".to_owned();
        pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
        pending_card.rgba8.clear();
        pending_card.width = 0;
        pending_card.height = 0;
        pending_card.camera = None;
        pending_card.selectable = false;
        app.state.selected_candidate = Some(pending_card.id.clone());
        app.state.candidates = vec![pending_card];

        let pending_visible = app.make_canvas_view_state();

        assert_eq!(pending_visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(
            pending_visible.candidate_tray_state,
            MakeCandidateTrayState::HasCandidates
        );
        assert_eq!(pending_visible.local_banner_title, "Rendering previews");
        assert_eq!(
            pending_visible.local_banner_message,
            "Candidate shells are ready. Use an idea after its preview renders."
        );
        assert!(!pending_visible.selected_comparison_visible);
        assert!(!pending_visible.primary_action_enabled);
        assert!(app.accept_visible_candidate_command().is_none());

        let mut pending_card = test_candidate_card("candidate-pending", false, None);
        pending_card.validation_label = "Preview pending".to_owned();
        pending_card.preview_failure = Some("Preview rendering for this direction.".to_owned());
        pending_card.rgba8.clear();
        pending_card.width = 0;
        pending_card.height = 0;
        pending_card.camera = None;
        pending_card.selectable = false;
        let ready_card = test_candidate_card("candidate-ready", true, None);
        app.state.selected_candidate = Some(ready_card.id.clone());
        app.state.candidates = vec![pending_card, ready_card];

        let mixed_visible = app.make_canvas_view_state();

        assert_eq!(mixed_visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(mixed_visible.local_banner_title, "Rendering previews");
        assert!(mixed_visible.selected_comparison_visible);
        assert!(!mixed_visible.primary_action_enabled);
        assert!(make_canvas_candidate_actions_enabled(&mixed_visible));
        assert!(app.accept_visible_candidate_command().is_some());
    }

    #[test]
    fn stale_result_warning_is_local_and_recoverable_with_try_again() {
        let mut app = ready_visible_state_test_app();
        app.state.status =
            Some("Ignored a background result because newer work is active.".to_owned());

        let visible = app.make_canvas_view_state();

        assert_eq!(
            visible.local_warning_message.as_deref(),
            Some(STALE_RESULT_WARNING)
        );
        assert_eq!(visible.local_banner_title, "Older result ignored");
        assert!(visible.local_banner_message.contains(ACTION_TRY_AGAIN));
        assert!(rendered_action_labels_for_default_shell().contains(&ACTION_TRY_AGAIN));
    }

    #[test]
    fn busy_candidate_request_does_not_accept_duplicate_clicks() {
        let mut app = ready_visible_state_test_app();
        let request = FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        };

        let first = app
            .state
            .request_candidates(request.clone())
            .expect("first request schedules");
        let second = app
            .state
            .request_candidates(request)
            .expect("duplicate request is ignored");

        assert_eq!(first.len(), 1);
        assert!(second.is_empty());
        assert_eq!(
            app.state
                .active_jobs
                .values()
                .filter(|request| request.slot() == FoundryJobSlot::GenerateCandidates)
                .count(),
            1
        );
        assert!(!app.make_canvas_view_state().primary_action_enabled);
    }

    #[test]
    fn idea_generation_timeout_recovery_actions_are_visible() {
        let mut app = ready_visible_state_test_app();
        app.state
            .request_candidates(FoundryCandidateRequest {
                seed: 1,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::complete_look(),
            })
            .expect("candidate job schedules");
        app.make_generation_started_at =
            Some(Instant::now() - IDEA_GENERATION_TIMEOUT - Duration::from_secs(1));

        let visible = app.make_canvas_view_state();

        assert!(visible.idea_generation_timed_out);
        assert!(visible.idea_generation_fallback_visible);
        assert_eq!(
            visible.local_banner_message,
            IDEA_GENERATION_TIMEOUT_MESSAGE
        );
        assert_eq!(visible.next_action_hint, IDEA_GENERATION_TIMEOUT_MESSAGE);
        let labels = rendered_action_labels_for_default_shell();
        assert!(labels.contains(&ACTION_CANCEL));
        assert!(labels.contains(&ACTION_KEEP_WAITING));
    }

    #[test]
    fn cancel_idea_generation_cancels_active_job_with_local_warning() {
        let mut app = ready_visible_state_test_app();
        let request = FoundryCandidateRequest {
            seed: 1,
            proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
            result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: VariationIntent::complete_look(),
        };
        let effects = app
            .state
            .request_candidates(request)
            .expect("candidate job schedules");
        let job_id = effects
            .iter()
            .find_map(|effect| match effect {
                FoundryAppEffect::StartJob(job) => Some(job.job_id()),
                _ => None,
            })
            .expect("candidate job id");

        let cancel_effects = app
            .state
            .handle_command(FoundryAppCommand::CancelIdeaGeneration)
            .expect("cancel succeeds");
        let visible = app.make_canvas_view_state();

        assert!(cancel_effects.is_empty());
        assert!(!app.state.active_jobs.contains_key(&job_id));
        assert!(app.state.stale_jobs.contains(&job_id));
        assert_eq!(app.state.make_job_trace.summary().total_jobs_canceled, 1);
        assert_eq!(
            visible.local_warning_message.as_deref(),
            Some(CANCELED_IDEA_SEARCH_WARNING)
        );
        assert_eq!(visible.local_banner_title, "Idea search canceled");
        assert!(visible.local_banner_message.contains(ACTION_TRY_AGAIN));
    }

    #[test]
    fn starting_template_queues_model_and_preview_automatically() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();

        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        assert_eq!(app.tab, FoundryTab::Make);
        assert!(
            app.state
                .active_jobs
                .values()
                .any(|request| matches!(request, FoundryJobRequest::CompileCurrent { .. }))
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() && app.state.current_preview.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert!(app.state.current_output.is_some());
        assert!(app.state.current_preview.is_some());
    }

    #[test]
    fn box_primitive_make_baseline_flow_is_plain_and_complete() {
        let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
        let mut app = box_primitive_ready_state_test_app();
        let ready = app.make_canvas_view_state();

        assert_eq!(ready.mode, MakeCanvasMode::Ready);
        assert_eq!(ready.primary_action_label, ACTION_TRY_BOX_IDEAS);
        assert_eq!(ready.adjust_heading_label, ACTION_ADJUST_BOX);
        assert!(ready.next_action_hint.contains("adjust box"));
        assert!(!app.material_look_action_visible(&ready));
        assert!(!ready.material_look_tray_visible);
        let groups = app
            .state
            .document
            .as_ref()
            .map(directions::direction_part_groups_for_document)
            .expect("box document has direction groups");
        assert!(
            groups.is_empty(),
            "Box Primitive should not expose part focus chips"
        );

        let request_command = app
            .make_primary_candidate_command()
            .expect("request command");
        let FoundryAppCommand::RequestCandidates(request) = request_command.clone() else {
            panic!("expected candidate request command");
        };
        assert_eq!(
            request.variation_intent.channels,
            vec![shape_foundry::VariationChannel::CompleteLook]
        );
        for forbidden_channel in [
            shape_foundry::VariationChannel::Surface,
            shape_foundry::VariationChannel::Rig,
            shape_foundry::VariationChannel::Motion,
        ] {
            assert!(
                !request
                    .variation_intent
                    .channels
                    .contains(&forbidden_channel)
            );
        }

        let candidate_effects = app
            .state
            .handle_command(request_command)
            .expect("Try box ideas schedules");
        let candidate_event = run_fixture_effect(candidate_effects, &fixture);
        let (preview_request, candidate_output) = match &candidate_event {
            FoundryJobEvent::CandidatesGenerated {
                request, output, ..
            } => {
                assert!(
                    !output.candidates.is_empty(),
                    "Try box ideas should yield candidates: {:?} {:?}",
                    output.diagnostics,
                    output.reliability_report
                );
                (request.clone(), output.as_ref().clone())
            }
            other => panic!("expected generated candidates, got {other:?}"),
        };
        assert!(app.state.handle_job_event(candidate_event));

        let preview_effects = app
            .state
            .request_candidate_previews(preview_request, candidate_output)
            .expect("candidate previews schedule");
        let preview_event = run_fixture_effect(preview_effects, &fixture);
        assert!(app.state.handle_job_event(preview_event));

        let reviewing = app.make_canvas_view_state();
        assert_eq!(reviewing.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(reviewing.primary_action_label, ACTION_CHOOSE_DIRECTION);
        assert_eq!(
            reviewing.use_candidate_action_label,
            ACTION_CHOOSE_DIRECTION
        );
        assert!(reviewing.selected_comparison_visible);
        assert!(
            app.state
                .candidates
                .iter()
                .any(|candidate| !candidate.rgba8.is_empty())
        );

        let build_before_accept = app.state.current_build.clone();
        let accept_effects = app
            .state
            .handle_command(
                app.accept_visible_candidate_command()
                    .expect("selectable box idea"),
            )
            .expect("Use this idea schedules");
        let accept_event = run_fixture_effect(accept_effects, &fixture);
        assert!(app.state.handle_job_event(accept_event));
        assert_ne!(app.state.current_build, build_before_accept);
        assert!(app.state.candidates.is_empty());

        let adjust_effects = app
            .state
            .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
                control_id: "edge_softness".to_owned(),
                value: shape_foundry::ControlValue::Scalar(0.55),
            }))
            .expect("adjust box schedules");
        let adjust_event = run_fixture_effect(adjust_effects, &fixture);
        assert!(app.state.handle_job_event(adjust_event));
        assert_eq!(
            app.state
                .document
                .as_ref()
                .and_then(|document| document.control_state.get("edge_softness")),
            Some(&shape_foundry::ControlValue::Scalar(0.55))
        );

        let pack_effects = app
            .state
            .handle_command(app.add_current_to_pack_command().expect("pack command"))
            .expect("Add to Pack schedules");
        let pack_event = run_fixture_effect(pack_effects, &fixture);
        assert!(app.state.handle_job_event(pack_event));
        assert_eq!(app.state.pack.members.len(), 1);

        app.drawer = Some(FoundryDrawer::Pack);
        assert!(app.make_canvas_view_state().pack_drawer_visible);
        app.drawer = Some(FoundryDrawer::Export);
        assert!(app.make_canvas_view_state().export_drawer_visible);

        let simple_make_copy = [
            ready.primary_action_label.as_str(),
            reviewing.primary_action_label.as_str(),
            ready.adjust_heading_label,
            ACTION_ADD_TO_PACK,
            ACTION_EXPORT,
        ]
        .join("\n")
        .to_ascii_lowercase();
        for forbidden in ["surface", "material", "rig", "motion", "focus part"] {
            assert!(
                !simple_make_copy.contains(forbidden),
                "Box Primitive baseline copy must not expose {forbidden}: {simple_make_copy}"
            );
        }
    }

    #[test]
    fn active_candidate_job_disables_conflicting_actions() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::box_primitive::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        app.state
            .request_candidates(FoundryCandidateRequest {
                seed: 1,
                proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
                result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                mode: FoundryCandidateMode::Explore,
                strategy_id: None,
                preference_profile: None,
                variation_intent: VariationIntent::complete_look(),
            })
            .expect("candidate job schedules");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::GeneratingWholeAssetIdeas);
        assert_eq!(visible.primary_action_label, ACTION_GENERATING_IDEAS);
        assert!(!visible.primary_action_enabled);
        assert!(!make_canvas_candidate_actions_enabled(&visible));
        assert!(visible.local_busy_label.is_some());
        assert!(visible.local_busy_visible);
        assert!(visible.candidate_tray_visible);
        assert_eq!(visible.next_action_hint, "Watch this area for new ideas.");
    }

    #[test]
    fn rejected_candidate_summary_is_local_make_state() {
        let mut app = ready_visible_state_test_app();
        app.state.status =
            Some("Found 4 clear ideas. Rejected 2 that looked too similar.".to_owned());

        let visible = app.make_canvas_view_state();

        assert_eq!(
            visible.rejected_candidate_summary.as_deref(),
            Some("Found 4 clear ideas. Rejected 2 that looked too similar.")
        );
    }

    #[test]
    fn active_edit_job_marks_current_build_stale_for_pack_and_export() {
        let mut app = ready_visible_state_test_app();

        app.state
            .handle_command(FoundryAppCommand::run(FoundryCommand::SetControl {
                control_id: "edge_softness".to_owned(),
                value: shape_foundry::ControlValue::Scalar(0.4),
            }))
            .expect("edit schedules rebuild");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::PreparingAsset);
        assert!(!visible.model_ready);
        assert!(!visible.preview_ready);
        assert!(visible.local_busy_visible);
        assert!(!make_canvas_build_dependent_actions_enabled(&visible));
    }

    #[test]
    fn stale_result_status_becomes_local_make_warning() {
        let mut app = visible_state_test_app();
        app.state.status =
            Some("Ignored a background result because newer work is active.".to_owned());

        let visible = app.make_canvas_view_state();

        assert_eq!(
            visible.local_warning_message.as_deref(),
            Some(STALE_RESULT_WARNING)
        );
        assert!(visible.local_error_message.is_none());
    }

    #[test]
    fn make_canvas_forbidden_product_terms_are_absent_from_default_strings() {
        let strings = product_visible_strings_for_default_shell();

        for label in &strings {
            assert!(
                crate::foundry::ui::copy::first_forbidden_product_term(label).is_none(),
                "default product string contains forbidden implementation copy: {label}"
            );
        }

        let joined = strings.join("\n").to_ascii_lowercase();
        for forbidden_phrase in ["fingerprint", "gltf primitive"] {
            assert!(
                !joined.contains(forbidden_phrase),
                "default product strings contain forbidden phrase {forbidden_phrase}: {joined}"
            );
        }
        assert!(joined.contains("not a textured, rigged, animated, or game-ready package"));
    }

    #[test]
    fn rendered_action_labels_are_in_product_visible_inventory() {
        let strings = product_visible_strings_for_default_shell();
        for label in rendered_action_labels_for_default_shell() {
            assert!(
                strings.contains(label),
                "missing rendered action label {label}"
            );
            assert!(
                crate::foundry::ui::copy::first_forbidden_product_term(label).is_none(),
                "rendered action label contains forbidden product copy: {label}"
            );
        }
    }

    #[test]
    fn core_make_actions_are_not_quiet_text_buttons() {
        for spec in core_make_action_specs_for_default_shell() {
            assert_ne!(
                spec.tone,
                ButtonTone::Quiet,
                "core Make action should render as a visible button: {}",
                spec.label
            );
            assert!(spec.validate().is_ok());
        }
    }

    #[test]
    fn directions_panel_exposes_all_generation_modes() {
        let labels = direction_mode_actions_for_panel()
            .into_iter()
            .map(|action| action.label)
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec!["Refine", "Explore", "Silhouette", "Structure", "Detail"]
        );
        for label in labels {
            assert!(crate::foundry::ui::copy::first_forbidden_product_term(label).is_none());
        }
    }

    #[test]
    fn launch_save_state_does_not_claim_project_is_saved() {
        let mut app = FoundryDesktopApp::default();
        assert_eq!(
            app.save_state_pill(),
            ("Choose Box Primitive", StatusTone::Neutral)
        );

        app.state =
            FoundryAppState::new(shape_foundry_catalog::box_primitive::fixture_catalog().document)
                .expect("fixture state");
        app.state.project_path = None;
        app.state.dirty = false;
        assert_eq!(app.save_state_pill(), ("Not saved", StatusTone::Warning));

        app.state.project_path = Some(PathBuf::from("box_primitive.shapelab-foundry.json"));
        app.state.dirty = true;
        assert_eq!(app.save_state_pill(), ("Unsaved", StatusTone::Warning));

        app.state.dirty = false;
        assert_eq!(app.save_state_pill(), ("Saved", StatusTone::Ready));
    }

    #[test]
    fn recent_projects_are_real_load_targets_and_keep_newest_first() {
        let mut app = FoundryDesktopApp::default();
        let first = PathBuf::from("first.shapelab-foundry.json");
        let second = PathBuf::from("second.shapelab-foundry.json");
        let third = PathBuf::from("third.shapelab-foundry.json");

        app.remember_recent_project(first.clone());
        app.remember_recent_project(second.clone());
        app.remember_recent_project(third.clone());
        app.remember_recent_project(first.clone());

        assert_eq!(app.recent_projects, vec![first, third, second]);
    }

    #[test]
    fn default_customize_summaries_hide_internal_control_kinds() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if !app.state.controls.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let controls = display_customize_controls(&app.state.controls);
        assert!(controls.len() <= CUSTOMIZE_PRIMARY_CONTROL_LIMIT);
        assert!(!controls.is_empty());
        assert!(controls.iter().any(|control| control.id == "proportions"));
        assert!(controls.iter().any(|control| control.id == "edge_softness"));
        let visible = controls
            .iter()
            .map(|control| product_control_summary(control))
            .collect::<Vec<_>>();
        assert!(
            crate::foundry::ui::copy::labels_are_product_safe(&visible),
            "visible customize summaries contain implementation copy: {visible:?}"
        );
    }

    #[test]
    fn box_primitive_filmstrip_shows_all_options_without_overflow() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if !app.state.controls.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let controls = display_customize_controls(&app.state.controls);
        assert!(!controls.is_empty());
        assert!(
            controls
                .iter()
                .all(|control| control.options.len() <= CONTROL_FILMSTRIP_LIMIT)
        );
    }

    #[test]
    fn pack_member_labels_hide_source_document_ids() {
        let member = pack::PackMemberRow {
            member_id: "box-primitive-doc".to_owned(),
            name: "box-primitive-doc".to_owned(),
            document_id: "box-primitive-doc".to_owned(),
            selected: false,
            override_count: 0,
        };

        let label = pack_member_display_name(&member);
        assert_eq!(label, "Box Primitive");
        assert!(!label.contains("-doc"));

        let cell = pack::PackContactSheetCell {
            row: 0,
            column: 0,
            member_id: "box-primitive-doc".to_owned(),
            name: "box-primitive-doc".to_owned(),
            document_id: "box-primitive-doc".to_owned(),
            status: pack::PackMemberStatus::Ready,
            override_count: 0,
            selected: false,
        };
        let cell_label = pack_cell_display_name(&cell);
        assert_eq!(cell_label, "Box Primitive");
        assert!(!cell_label.contains("-doc"));
    }

    #[test]
    fn pack_contact_sheet_uses_product_safe_thumbnail_markers() {
        let current_cell = pack::PackContactSheetCell {
            row: 0,
            column: 0,
            member_id: "box-primitive-doc".to_owned(),
            name: "box-primitive-doc".to_owned(),
            document_id: "box-primitive-doc".to_owned(),
            status: pack::PackMemberStatus::Ready,
            override_count: 0,
            selected: true,
        };
        let other_cell = pack::PackContactSheetCell {
            selected: false,
            ..current_cell.clone()
        };

        assert_eq!(pack_thumbnail_marker(&current_cell), "Current");
        assert_eq!(pack_thumbnail_marker(&other_cell), "Preview");
        assert!(crate::foundry::ui::copy::labels_are_product_safe(&[
            pack_thumbnail_marker(&current_cell),
            pack_thumbnail_marker(&other_cell)
        ]));
    }

    #[test]
    fn product_panel_messages_replace_raw_backend_details() {
        assert_eq!(
            product_panel_message(
                "members.box-primitive-doc.document_id failed validation",
                "Pack needs attention before export."
            ),
            "Pack needs attention before export."
        );
        assert_eq!(
            product_panel_message(
                "Could not render C:\\tmp\\preview.png",
                "Preview could not be rendered for this direction."
            ),
            "Preview could not be rendered for this direction."
        );
        assert_eq!(
            product_panel_message("This option is locked.", "Fallback"),
            "This option is locked."
        );
    }

    #[test]
    fn workflow_copy_maps_to_foundry_tabs_without_history_as_primary_step() {
        let tabs = WORKFLOW_STEPS
            .iter()
            .map(|step| tab_for_workflow_step(step.index))
            .collect::<Vec<_>>();

        assert_eq!(tabs, vec![FoundryTab::Home, FoundryTab::Make]);
        assert!(!tabs.contains(&FoundryTab::History));
    }

    #[test]
    fn product_status_hides_raw_paths_from_status_strip() {
        assert_eq!(
            product_safe_status("Saved C:\\work\\box.shapelab-foundry.json"),
            "Project saved"
        );
        assert_eq!(
            product_safe_status("Loaded C:\\work\\box.shapelab-foundry.json"),
            "Project loaded"
        );
        assert_eq!(
            product_safe_status("Could not use C:\\work\\broken.json"),
            "Project path needs attention"
        );
        assert_eq!(
            product_safe_status("Exported default to C:\\exports\\box"),
            "Export complete"
        );
        assert_eq!(
            product_safe_status("Exported 3 pack member(s) with default to C:\\exports\\pack"),
            "Pack export complete"
        );
        assert_eq!(
            product_safe_status("provider socket failed conformance check"),
            "Project needs attention"
        );
        assert_eq!(
            product_safe_status("recipe fragment remap failed"),
            "Project needs attention"
        );
    }

    #[test]
    fn preview_texture_identity_tracks_render_metadata_without_pixel_scan() {
        let box_a = shape_foundry_catalog::box_primitive::fixture_catalog();
        let build_a = compile_foundry_document(&box_a.document, &box_a)
            .expect("box fixture compiles")
            .build_stamp;

        let identity = FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1);

        assert_eq!(
            identity,
            FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1)
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new("option-b", Some(&build_a), 2, 1)
        );
        assert_eq!(
            identity,
            FoundryTextureIdentity::new("option-a", Some(&build_a), 2, 1)
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new("option-a", Some(&build_a), 1, 2)
        );
    }

    #[test]
    fn desktop_foundry_exposes_product_steps_and_box_profile() {
        let tabs = [FoundryTab::Home, FoundryTab::Make, FoundryTab::History];
        assert_eq!(tabs.len(), 3);

        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(app.tab, FoundryTab::Make);
        assert_eq!(
            app.state
                .document
                .as_ref()
                .map(|document| document.family_content_ref.stable_id.as_str()),
            Some("box-primitive-family")
        );
        assert!(app.state.current_output.is_some());
    }

    #[test]
    fn loading_project_enters_workflow_step() {
        let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
        let state = FoundryAppState::new(fixture.document).expect("fixture state");
        let mut project_file = state.project_file.expect("project file");
        let path = temp_foundry_project_path("load-enters-workflow");
        project_file.save_as(&path).expect("project saves");
        let mut app = FoundryDesktopApp::default();

        app.load_project(path.clone(), &egui::Context::default());

        assert_eq!(app.tab, FoundryTab::Make);
        assert!(app.state.document.is_some());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn default_customize_surface_hides_non_primary_and_hidden_controls() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::box_primitive::fixture_catalog(),
            &ctx,
        );

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if default_customize_controls(&app.state.controls)
                .any(|control| control.id == "proportions")
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let default_ids = default_customize_controls(&app.state.controls)
            .map(|control| control.id.as_str())
            .collect::<Vec<_>>();
        assert!(default_ids.contains(&"proportions"));
        assert!(default_ids.contains(&"edge_softness"));
        assert!(
            default_customize_controls(&app.state.controls)
                .all(|control| control.primary && control.visible)
        );
    }

    fn visible_state_test_app() -> FoundryDesktopApp {
        FoundryDesktopApp {
            tab: FoundryTab::Make,
            state: FoundryAppState::new(
                shape_foundry_catalog::box_primitive::fixture_catalog().document,
            )
            .expect("fixture state"),
            ..FoundryDesktopApp::default()
        }
    }

    fn ready_visible_state_test_app() -> FoundryDesktopApp {
        let mut app = visible_state_test_app();
        let output = compile_foundry_document(
            app.state.document.as_ref().expect("document"),
            &shape_foundry_catalog::box_primitive::fixture_catalog(),
        )
        .expect("fixture compiles");
        let build = output.build_stamp.clone();
        app.state.current_build = Some(build.clone());
        app.state.current_output = Some(Box::new(output));
        app.state.current_preview = Some(test_preview_image_for_build("current", Some(build)));
        app
    }

    fn box_primitive_ready_state_test_app() -> FoundryDesktopApp {
        let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
        let mut app = FoundryDesktopApp {
            tab: FoundryTab::Make,
            state: FoundryAppState::new(fixture.document.clone()).expect("fixture state"),
            ..FoundryDesktopApp::default()
        };
        let output =
            compile_foundry_document(app.state.document.as_ref().expect("document"), &fixture)
                .expect("fixture compiles");
        let build = output.build_stamp.clone();
        app.state.current_build = Some(build.clone());
        app.state.current_output = Some(Box::new(output));
        app.state.current_preview = Some(test_preview_image_for_build("current", Some(build)));
        app
    }

    fn run_fixture_effect(
        effects: Vec<FoundryAppEffect>,
        fixture: &FoundryFixtureCatalog,
    ) -> FoundryJobEvent {
        let [FoundryAppEffect::StartJob(job)] = effects.as_slice() else {
            panic!("expected exactly one start job effect, got {effects:?}");
        };
        run_foundry_job(
            job.as_ref().clone(),
            fixture,
            &mut FoundryPreviewCache::default(),
        )
    }

    fn test_preview_image(preview_id: &str) -> FoundryPreviewImage {
        test_preview_image_for_build(preview_id, None)
    }

    fn test_preview_image_for_build(
        preview_id: &str,
        build: Option<FoundryBuildStamp>,
    ) -> FoundryPreviewImage {
        FoundryPreviewImage {
            preview_id: preview_id.to_owned(),
            rgba8: vec![24, 32, 40, 255],
            width: 1,
            height: 1,
            camera: OrbitCamera::default(),
            build,
        }
    }

    fn empty_test_candidate_output(
        hidden_internal_rejections: usize,
        duplicate_looking_rejections: usize,
        wrong_scope_rejections: usize,
    ) -> FoundryCandidateOutput {
        FoundryCandidateOutput {
            candidates: Vec::new(),
            diagnostics: shape_search::foundry::FoundryCandidateGenerationDiagnostics {
                requested_proposals: directions::DEFAULT_DIRECTION_PROPOSALS,
                requested_candidates: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                attempted_proposals: directions::DEFAULT_DIRECTION_PROPOSALS,
                scored_candidates: 0,
                accepted_candidates: 0,
                returned_candidates: 0,
                available_control_count: 0,
                locked_targets_skipped: 0,
                rejections: std::collections::BTreeMap::new(),
                duplicate_looking_rejections,
                hidden_internal_rejections,
                wrong_scope_rejections,
                human_summary: "Generated 0 clear ideas.".to_owned(),
            },
            reliability_report: shape_search::foundry::FoundryCandidateReliabilityReport::default(),
            scoring_report: shape_search::asset::scoring::AssetScoringReport {
                rejected_candidates: Vec::new(),
                scored_candidates: Vec::new(),
                unique_candidates: Vec::new(),
                duplicate_groups: Vec::new(),
                representatives: Vec::new(),
            },
            preference_report: shape_search::foundry::FoundryCandidatePreferenceReport {
                requested: false,
                applied: false,
                scope_matched: false,
                scope: shape_foundry::FoundryPreferenceScope::new("test-family", "test-profile"),
                ignored_reason: None,
                selected_scores: Vec::new(),
            },
        }
    }

    fn test_candidate_card(
        candidate_id: &str,
        selected: bool,
        focus_part_label: Option<String>,
    ) -> crate::foundry::view_model::FoundryCandidateCard {
        crate::foundry::view_model::FoundryCandidateCard {
            id: shape_foundry::FoundryCandidateId(candidate_id.to_owned()),
            slot: 0,
            mode: Some(FoundryCandidateMode::Explore),
            parent: false,
            title: "Readable box idea".to_owned(),
            subtitle: "Clear model change".to_owned(),
            preview_id: Some(format!("{candidate_id}-preview")),
            rgba8: vec![220, 225, 214, 255],
            width: 1,
            height: 1,
            camera: Some(OrbitCamera::default()),
            preview_failure: None,
            changed_controls: vec!["Proportions".to_owned()],
            changed_roles: vec!["Body".to_owned()],
            explanations: Vec::new(),
            rejections: std::collections::BTreeMap::new(),
            validation_label: "Ready".to_owned(),
            validation_detail: None,
            selectable: true,
            selected,
            variation_intent_label: "Body idea".to_owned(),
            variation_scope_label: "Focused: Body".to_owned(),
            variation_channel_labels: vec!["Shape".to_owned()],
            visible_delta_label: "Clear change".to_owned(),
            what_changed_summary: "Body proportions change visibly.".to_owned(),
            legibility_class: shape_foundry::CandidateLegibilityClass::Clear,
            focus_part_label,
            surface_unavailable_reason: None,
        }
    }

    fn set_test_focus_scope(app: &mut FoundryDesktopApp, group_id: &str, display_name: &str) {
        let document = app.state.document.as_mut().expect("fixture document");
        document.variation_state.intent = VariationIntent {
            scope: shape_foundry::VariationScope::SemanticPartGroup {
                group_id: group_id.to_owned(),
                display_name: display_name.to_owned(),
            },
            channels: vec![shape_foundry::VariationChannel::Shape],
            human_label: format!("Focused: {display_name}"),
            human_summary: format!("Try {display_name} ideas."),
        };
    }

    fn test_control_view(id: &str, label: &str) -> crate::foundry::view_model::FoundryControlView {
        crate::foundry::view_model::FoundryControlView {
            id: id.to_owned(),
            label: label.to_owned(),
            section: None,
            kind: "Control".to_owned(),
            presentation:
                crate::foundry::view_model::FoundryControlPresentation::ContinuousMacroAxis,
            value: None,
            default_value: None,
            primary: true,
            visible: true,
            locked: false,
            locked_reason: None,
            topology_behavior: shape_foundry::ControlTopologyBehavior::TopologyPreserving,
            divergence: shape_foundry::ControlDivergence::Synced,
            options: Vec::new(),
            advanced_path: None,
            help: None,
        }
    }

    #[derive(Clone, Copy)]
    struct TestMaterialLookEvidenceOptions<'a> {
        frozen_mesh_fingerprint: &'a str,
        profile_id: &'a str,
        shape_delta_leak: bool,
        missing_preview: bool,
        full_ready_status: &'a str,
    }

    impl Default for TestMaterialLookEvidenceOptions<'_> {
        fn default() -> Self {
            Self {
                frozen_mesh_fingerprint: "box-fingerprint",
                profile_id: BOX_PRIMITIVE_PROFILE_ID,
                shape_delta_leak: false,
                missing_preview: false,
                full_ready_status: "blocked",
            }
        }
    }

    fn write_test_material_look_evidence(
        root: &Path,
        options: TestMaterialLookEvidenceOptions<'_>,
    ) -> PathBuf {
        let variants_dir = root.join("surface/variants");
        std::fs::create_dir_all(&variants_dir).expect("variants dir");
        let candidate_ids = [
            "clean-lab-white",
            "worn-hazard-yellow",
            "dark-industrial-metal",
            "field-blue-utility",
            "graphite-box",
            "orange-warning-edge-detail",
        ];
        let blockers = [
            "manual_review_pending",
            "engine_import_proof_missing",
            "engine_native_package_not_implemented",
            "surface_manual_review_required",
        ];
        let mut report_rows = Vec::new();
        let mut candidate_rows = Vec::new();
        for (index, (candidate_id, display_name)) in
            candidate_ids.iter().zip(MATERIAL_LOOK_TITLES).enumerate()
        {
            let variant_dir = variants_dir.join(candidate_id);
            std::fs::create_dir_all(&variant_dir).expect("variant dir");
            let rel_dir = format!("surface/variants/{candidate_id}");
            let material_override_ref = format!("{rel_dir}/material-override.json");
            let textured_preview_ref = format!("{rel_dir}/textured-preview.png");
            let surface_delta_ref = format!("{rel_dir}/surface-delta.json");
            let validation_ref = format!("{rel_dir}/validation.json");
            let shape_delta = options.shape_delta_leak && index == 0;

            std::fs::write(
                root.join(&material_override_ref),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema_version": 1,
                    "profile_id": options.profile_id,
                    "candidate_id": candidate_id,
                    "display_name": display_name
                }))
                .expect("material override json"),
            )
            .expect("material override writes");
            if !(options.missing_preview && index == 0) {
                let pixel = image::Rgba([
                    32_u8.saturating_add((index as u8).saturating_mul(24)),
                    96,
                    160,
                    255,
                ]);
                let preview = image::RgbaImage::from_pixel(2, 2, pixel);
                preview
                    .save(root.join(&textured_preview_ref))
                    .expect("preview writes");
            }
            std::fs::write(
                root.join(&surface_delta_ref),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "schema_version": 1,
                    "profile_id": options.profile_id,
                    "candidate_id": candidate_id,
                    "shape_delta_leak_detected": shape_delta,
                    "result_class": "clear"
                }))
                .expect("delta json"),
            )
            .expect("delta writes");
            std::fs::write(
                root.join(&validation_ref),
                serde_json::to_vec_pretty(&serde_json::json!({
                    "valid": !(shape_delta || options.missing_preview && index == 0),
                    "blocker_codes": []
                }))
                .expect("validation json"),
            )
            .expect("validation writes");

            report_rows.push(serde_json::json!({
                "candidate_id": candidate_id,
                "display_name": display_name,
                "material_override_ref": material_override_ref,
                "textured_preview_ref": textured_preview_ref,
                "surface_delta_ref": surface_delta_ref,
                "validation_ref": validation_ref,
                "result_class": "clear",
                "shape_delta_leak_detected": shape_delta,
                "visible_surface_pixel_delta": 0.05
            }));
            candidate_rows.push(serde_json::json!({
                "candidate_id": candidate_id,
                "display_name": display_name,
                "changed_material_slots": [
                    "painted_metal_body",
                    "shadowed_body_edges",
                    "exposed_edge_detail",
                    "soft_edge_highlights",
                    "fallback_hard_surface"
                ],
                "material_override_ref": material_override_ref,
                "textured_preview_ref": textured_preview_ref,
                "surface_delta_ref": surface_delta_ref,
                "validation_ref": validation_ref,
                "frozen_mesh_fingerprint": options.frozen_mesh_fingerprint,
                "preserves_frozen_geometry": !shape_delta,
                "full_ready_status": options.full_ready_status,
                "blocked_full_ready": options.full_ready_status == "blocked"
            }));
        }

        std::fs::write(
            variants_dir.join(SURFACE_CANDIDATE_SET_FILE),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "profile_id": options.profile_id,
                "candidates": candidate_rows
            }))
            .expect("candidate set json"),
        )
        .expect("candidate set writes");
        let report_path = variants_dir.join("surface-candidate-report.json");
        std::fs::write(
            &report_path,
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_version": 1,
                "profile_id": options.profile_id,
                "visual_foundry_surface_mode_enabled": false,
                "candidate_count": MATERIAL_LOOK_TITLES.len(),
                "all_candidates_valid": !options.missing_preview,
                "full_ready_status": options.full_ready_status,
                "full_ready_blocker_codes": blockers,
                "candidates": report_rows
            }))
            .expect("report json"),
        )
        .expect("report writes");
        report_path
    }

    fn temp_material_look_package_root(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("shape-lab-{name}-{}-{nanos}", std::process::id()))
    }

    fn temp_foundry_project_path(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "shape-lab-{name}-{}-{nanos}{FOUNDRY_PROJECT_FILE_SUFFIX}",
            std::process::id()
        ))
    }
}
