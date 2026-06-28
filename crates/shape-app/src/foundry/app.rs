//! Native desktop host for the Foundry workflow state.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, RichText, TextureOptions};
use shape_core::Aabb;
use shape_foundry::{
    CatalogContentRef, FoundryAssetDocument, FoundryBuildStamp, FoundryCatalogError,
    FoundryCatalogResolver, FoundryCommand, FoundryCompilationOutput,
    STATIC_PROP_FULL_READY_BLOCKED_NOTE, STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL,
    STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION, VariationIntent,
    built_in_surface_capability_for_profile, compile_foundry_document,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, catalog_curation_metadata_for_slug,
    curated_fixture_catalogs_with_labels, headless_fixture_catalogs,
};
use shape_mesh::TriangleMesh;
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryProject, FoundryProjectFile, ensure_foundry_project_path,
};
use shape_render::{
    OrbitCamera, RenderSettings, fit_camera_to_bounds, foundry::FoundryPreviewCache, render_mesh,
};
use shape_search::foundry::{FoundryCandidateMode, FoundryCandidateRequest};

use crate::foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
    FoundryJobSlot, FoundryPreviewImage,
    kit_view::built_in_kit_card_views,
    panels::{customize, directions, history, pack},
    run_foundry_job,
    state::DEFAULT_PREVIEW_PIXELS,
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
    MakeInitialCrate,
    GeneratingWholeAssetIdeas,
    GeneratedWholeAssetIdeas,
    SelectedComparison,
    FocusHandles,
    GeneratingHandleIdeas,
    HandleIdeas,
    FocusVents,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct MakeCanvasViewState {
    mode: MakeCanvasMode,
    asset_name: String,
    primary_title: String,
    primary_action_label: String,
    primary_action_enabled: bool,
    primary_action_disabled_reason: Option<String>,
    local_busy_label: Option<String>,
    local_busy_visible: bool,
    focused_part_label: Option<String>,
    focused_part_visible: bool,
    focused_part_actions_visible: bool,
    model_ready: bool,
    preview_ready: bool,
    candidate_tray_visible: bool,
    candidate_count: usize,
    rejected_candidate_summary: Option<String>,
    selected_candidate_present: bool,
    selected_comparison_visible: bool,
    pack_drawer_visible: bool,
    export_drawer_visible: bool,
    local_warning_message: Option<String>,
    local_error_message: Option<String>,
    next_action_hint: String,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum HomeTemplateFilter {
    All,
    Props,
    Architecture,
    Gear,
    Furniture,
    Environment,
}

const HOME_TEMPLATE_FILTERS: [HomeTemplateFilter; 6] = [
    HomeTemplateFilter::All,
    HomeTemplateFilter::Props,
    HomeTemplateFilter::Architecture,
    HomeTemplateFilter::Gear,
    HomeTemplateFilter::Furniture,
    HomeTemplateFilter::Environment,
];

const HOME_SUBTITLE: &str =
    "Start with an asset template, then make whole-asset and part-focused ideas.";
const NEED_PROJECT_REASON: &str = "Choose a template or open a project first.";
const NEED_SAVE_LOCATION_REASON: &str =
    "Use Save Project first to choose where this project is saved.";
const NEED_MODEL_REASON: &str = "Prepare the current model first.";
const NEED_HISTORY_REASON: &str = "No earlier project step is available.";
const NEED_DIRECTION_REASON: &str = "This direction is not ready to choose.";
const NEED_RESET_REASON: &str = "This control is already at its starting value.";
const NEED_PACK_MEMBER_REASON: &str = "Add at least one asset before exporting a pack.";
const ASSET_PREPARING_REASON: &str = "The asset is preparing. This usually takes a moment.";
const ACTIVE_IDEA_JOB_REASON: &str = "Finish the current idea search before changing this.";
const STALE_RESULT_WARNING: &str = "An older result was ignored because you changed the asset.";
const CUSTOMIZE_PRIMARY_CONTROL_LIMIT: usize = 7;
const CONTROL_FILMSTRIP_LIMIT: usize = 5;
const CONTROL_HEADER_ACTIONS_WIDTH: f32 = 304.0;
const CONTROL_HEADER_STACK_BREAKPOINT: f32 = 520.0;
const MAX_CURRENT_PREVIEW_PIXELS: u32 = 1024;
const PREVIEW_CATALOG_ENV_VAR: &str = "SHAPE_LAB_PREVIEW_CATALOG";
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
const ACTION_GENERATING_IDEAS: &str = "Trying ideas...";
const ACTION_CHOOSE_TEMPLATE: &str = "Choose Template";
const ACTION_SWITCH: &str = "Switch";
const ACTION_BRANCH: &str = "Branch";
const ACTION_ADD_TO_PACK: &str = "Add to Pack";
const ACTION_OPEN_PACK: &str = "Open Pack";
const ACTION_OPEN_EXPORT: &str = "Open Export";
const ACTION_EXPORT_CURRENT_ASSET: &str = "Export Current Asset";
const ACTION_ADD_CURRENT_ASSET: &str = "Add Current Asset";
const ACTION_EXPORT_PACK: &str = "Export Pack";
const ACTION_SELECT: &str = "Compare";
const ACTION_CHOOSE_DIRECTION: &str = "Use this idea";
const ACTION_REJECT: &str = "Reject";
const ACTION_RESET: &str = "Reset";
const ACTION_UNLOCK: &str = "Unlock";
const ACTION_LOCK: &str = "Lock";
const ACTION_FOCUS: &str = "Focus";
const ACTION_APPLY: &str = "Use option";
const RENDERED_ACTION_LABELS: [&str; 33] = [
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
    ACTION_TRY_WHOLE_ASSET_IDEAS,
    ACTION_GENERATING_IDEAS,
    ACTION_CHOOSE_TEMPLATE,
    ACTION_SWITCH,
    ACTION_BRANCH,
    ACTION_ADD_TO_PACK,
    ACTION_OPEN_PACK,
    ACTION_OPEN_EXPORT,
    ACTION_EXPORT_CURRENT_ASSET,
    ACTION_ADD_CURRENT_ASSET,
    ACTION_EXPORT_PACK,
    ACTION_SELECT,
    ACTION_CHOOSE_DIRECTION,
    ACTION_REJECT,
    ACTION_RESET,
    ACTION_UNLOCK,
    ACTION_LOCK,
    ACTION_FOCUS,
    ACTION_APPLY,
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
        self.poll_jobs(&ctx);
        self.poll_home_thumbnail_jobs(&ctx);

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
                                            &ActionSpec::enabled("Close", ButtonTone::Quiet),
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
            .unwrap_or_else(|| "Choose what to make".to_owned())
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
        let selected_candidate = self.state.selected_candidate.as_ref().and_then(|selected| {
            self.state
                .candidates
                .iter()
                .find(|candidate| &candidate.id == selected)
        });
        let asset_name = self.current_project_title();
        let generating = self.directions_are_generating();
        let current_asset_job_active = self.current_asset_job_active();
        let model_ready = self.state.current_output.is_some()
            && !self.state.active_jobs.values().any(|request| {
                matches!(
                    request.slot(),
                    FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
                )
            });
        let preview_ready = model_ready
            && self
                .state
                .current_preview
                .as_ref()
                .is_some_and(|preview| !preview.rgba8.is_empty())
            && !self
                .state
                .active_jobs
                .values()
                .any(|request| request.slot() == FoundryJobSlot::RenderPreview);
        let focused_part_label = active_group.as_ref().map(|group| group.label.clone());
        let focused_part_visible = focused_part_label.is_some();
        let focused_part_actions_visible = focused_part_visible && model_ready && preview_ready;
        let preparing = self.state.document.is_some()
            && (!model_ready || !preview_ready || current_asset_job_active);
        let local_warning_message = self.make_canvas_local_warning();
        let local_error_message = self.make_canvas_local_error();
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
        } else if generating && focused_part_label.is_some() {
            MakeCanvasMode::GeneratingFocusedPartIdeas
        } else if generating {
            MakeCanvasMode::GeneratingWholeAssetIdeas
        } else if !self.state.candidates.is_empty() {
            MakeCanvasMode::ReviewingIdeas
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
            _ => None,
        };
        let local_busy_visible = local_busy_label.is_some();
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
            (_, Some(label)) => format!(
                "Try {} ideas",
                singular_part_copy(label).to_ascii_lowercase()
            ),
            _ => ACTION_TRY_WHOLE_ASSET_IDEAS.to_owned(),
        };
        let primary_action_enabled = matches!(
            mode,
            MakeCanvasMode::NoAsset
                | MakeCanvasMode::Ready
                | MakeCanvasMode::FocusedPart
                | MakeCanvasMode::ReviewingIdeas
        ) && (self.state.document.is_none()
            || (model_ready && preview_ready && !generating));
        let primary_action_disabled_reason = (!primary_action_enabled).then(|| {
            if preparing {
                ASSET_PREPARING_REASON.to_owned()
            } else if generating {
                ACTIVE_IDEA_JOB_REASON.to_owned()
            } else if let Some(message) = &local_error_message {
                message.clone()
            } else {
                NEED_PROJECT_REASON.to_owned()
            }
        });
        let candidate_tray_visible = generating || !self.state.candidates.is_empty();
        let rejected_candidate_summary = self.make_canvas_rejected_candidate_summary();
        let selected_candidate_present = selected_candidate.is_some();
        let selected_comparison_visible = selected_candidate.is_some_and(|candidate| {
            preview_ready
                && !candidate.rgba8.is_empty()
                && self
                    .state
                    .current_preview
                    .as_ref()
                    .is_some_and(|preview| !preview.rgba8.is_empty())
        });
        let next_action_hint = make_canvas_next_action_hint(
            &mode,
            focused_part_label.as_deref(),
            selected_comparison_visible,
        );

        MakeCanvasViewState {
            asset_name,
            mode,
            primary_title,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            local_busy_label,
            local_busy_visible,
            focused_part_label,
            focused_part_visible,
            focused_part_actions_visible,
            model_ready,
            preview_ready,
            candidate_tray_visible,
            candidate_count: self.state.candidates.len(),
            rejected_candidate_summary,
            selected_candidate_present,
            selected_comparison_visible,
            pack_drawer_visible: self.drawer == Some(FoundryDrawer::Pack),
            export_drawer_visible: self.drawer == Some(FoundryDrawer::Export),
            local_warning_message,
            local_error_message,
            next_action_hint,
        }
    }

    fn make_canvas_local_warning(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.starts_with("Ignored a background result") {
            Some(STALE_RESULT_WARNING.to_owned())
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

    fn current_asset_job_active(&self) -> bool {
        self.state.active_jobs.values().any(|request| {
            matches!(
                request.slot(),
                FoundryJobSlot::CompileCurrent
                    | FoundryJobSlot::RenderPreview
                    | FoundryJobSlot::ApplyEdit
            )
        })
    }

    fn save_state_pill(&self) -> (&'static str, StatusTone) {
        if self.state.document.is_none() {
            ("Choose a template to start", StatusTone::Neutral)
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
            "Choose a template"
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
            "Preview ready"
        } else if self.state.current_output.is_some() {
            "Preview available"
        } else {
            "Preview waiting"
        }
    }

    fn directions_are_generating(&self) -> bool {
        self.state
            .active_jobs
            .values()
            .any(|request| matches!(request, FoundryJobRequest::GenerateCandidates { .. }))
    }

    fn show_home(&mut self, ui: &mut egui::Ui) {
        let profiles = self.home_profiles.as_slice();
        if profiles.is_empty() {
            product_empty_state(
                ui,
                "No reviewed templates yet",
                "Open a saved project, or enable the preview catalog for internal kit testing.",
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
                            "No matching template",
                            "Change the search or filter to choose a template.",
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
                "Pick a template or open a project before making changes.",
            ));
            return commands;
        }

        let available = ui.available_size();
        let tray_height = (available.y * 0.42).clamp(260.0, 430.0);
        let top_height = (available.y - tray_height - 14.0).max(320.0);

        ui.allocate_ui_with_layout(
            egui::vec2(available.x, top_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                let total_width = ui.available_width();
                let inspector_width = if total_width >= 1120.0 {
                    430.0_f32
                } else {
                    360.0_f32
                }
                .min(total_width * 0.42);
                let stage_width = (total_width - inspector_width - 18.0).max(420.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(stage_width, top_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            commands.extend(self.show_make_model_stage_panel(ui, &view_state));
                        },
                    );
                    ui.add_space(18.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), top_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            commands.extend(self.show_make_inspector_panel(ui, &view_state));
                        },
                    );
                });
            },
        );

        ui.add_space(14.0);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), tray_height),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                commands.extend(self.show_direction_board_panel(ui, &view_state));
            },
        );
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
            ui.set_min_height(ui.available_height().max(260.0));
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Model workspace")
                        .color(colors.accent_hover)
                        .small()
                        .strong(),
                );
                let state_label = match view_state.mode {
                    MakeCanvasMode::PreparingAsset => "Preparing model",
                    MakeCanvasMode::GeneratingWholeAssetIdeas
                    | MakeCanvasMode::GeneratingFocusedPartIdeas => "Trying ideas",
                    _ if view_state.preview_ready => "Preview ready",
                    _ => "Preview waiting",
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
            let preview_edge = (ui.available_height() - 116.0)
                .min(ui.available_width() - 18.0)
                .clamp(220.0, 620.0);
            self.show_current_preview_sized(ui, preview_edge);
            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                for group in &part_groups {
                    let selected = active_group_id.as_deref() == Some(group.group_id.as_str());
                    let reason = if interactions_enabled {
                        group
                            .unavailable_reason
                            .as_deref()
                            .unwrap_or("This part has no focused variations yet.")
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
                    RichText::new(format!("{} is focused", group.label))
                        .color(colors.accent_hover)
                        .strong(),
                );
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

    fn show_make_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        product_card(ui, false, |ui| {
            ui.set_min_height(ui.available_height().max(320.0));
            ui.label(
                RichText::new(&view_state.primary_title)
                    .color(colors.text)
                    .size(20.0)
                    .strong(),
            );
            ui.label(
                RichText::new(make_canvas_mode_summary(view_state))
                    .color(colors.text_muted)
                    .small(),
            );
            ui.label(
                RichText::new(&view_state.next_action_hint)
                    .color(colors.accent_hover)
                    .strong(),
            );
            ui.add_space(12.0);
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
                if view_state.mode == MakeCanvasMode::NoAsset {
                    self.tab = FoundryTab::Home;
                } else if let Some(command) = self.make_primary_candidate_command() {
                    commands.push(command);
                }
            }
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let build_actions_enabled = make_canvas_build_dependent_actions_enabled(view_state);
                let build_actions_reason = make_canvas_build_dependent_disabled_reason(view_state);
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
            if let Some(message) = &view_state.local_warning_message {
                ui.add_space(10.0);
                status_banner(
                    ui,
                    StatusBannerSpec {
                        title: "Older result ignored",
                        message,
                        tone: BannerTone::Warning,
                    },
                );
            }
            if let Some(message) = &view_state.local_error_message {
                ui.add_space(10.0);
                status_banner(
                    ui,
                    StatusBannerSpec {
                        title: "Asset needs attention",
                        message,
                        tone: BannerTone::Error,
                    },
                );
            }
            if let Some(group) = self.active_make_part_group() {
                ui.add_space(12.0);
                product_card(ui, true, |ui| {
                    ui.label(RichText::new(&group.label).color(colors.text).strong());
                    ui.label(
                        RichText::new("Try changes for this part, lock it when it works, or clear focus to return to the whole asset.")
                            .color(colors.text_muted)
                            .small(),
                    );
                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        let try_label = directions::try_focused_part_label(&group);
                        if action_button(
                            ui,
                            &action_spec(
                                view_state.primary_action_enabled,
                                &try_label,
                                ButtonTone::Primary,
                                view_state
                                    .primary_action_disabled_reason
                                    .as_deref()
                                    .unwrap_or(ACTIVE_IDEA_JOB_REASON),
                            ),
                        )
                        .clicked()
                            && let Some(command) = self.make_primary_candidate_command()
                        {
                            commands.push(command);
                        }
                        let lock_label = directions::lock_focused_part_label(&group);
                        if action_button(
                            ui,
                            &action_spec(
                                view_state.primary_action_enabled,
                                &lock_label,
                                ButtonTone::Secondary,
                                view_state
                                    .primary_action_disabled_reason
                                    .as_deref()
                                    .unwrap_or(ACTIVE_IDEA_JOB_REASON),
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
                                    reason: Some(format!(
                                        "{} kept while trying ideas.",
                                        group.label
                                    )),
                                },
                            }));
                        }
                        if action_button(
                            ui,
                            &action_spec(
                                view_state.primary_action_enabled,
                                "Clear focus",
                                ButtonTone::Secondary,
                                view_state
                                    .primary_action_disabled_reason
                                    .as_deref()
                                    .unwrap_or(ACTIVE_IDEA_JOB_REASON),
                            ),
                        )
                        .clicked()
                        {
                            commands.push(directions::clear_focus_part_group_command());
                        }
                    });
                });
            }
            ui.add_space(12.0);
            ui.label(RichText::new("Controls").color(colors.text).strong());
            ui.add_space(6.0);
            let active_group = self.active_make_part_group();
            let controls = display_customize_controls(&self.state.controls)
                .into_iter()
                .filter(|control| make_control_matches_focus(control, active_group.as_ref()))
                .cloned()
                .collect::<Vec<_>>();
            egui::ScrollArea::vertical()
                .id_salt("make_context_inspector_controls")
                .auto_shrink([false, false])
                .max_height((ui.available_height() - 18.0).max(160.0))
                .show(ui, |ui| {
                    if controls.is_empty() {
                        product_empty_state(
                            ui,
                            "No quick controls yet",
                            "This asset has no quick controls yet.",
                        );
                    } else {
                        let current_build = self.state.current_build.clone();
                        let texture_cache = &mut self.texture_cache;
                        let actions_enabled = make_canvas_controls_enabled(view_state);
                        let disabled_reason = view_state
                            .primary_action_disabled_reason
                            .as_deref()
                            .unwrap_or(ACTIVE_IDEA_JOB_REASON);
                        for control in &controls {
                            commands.extend(show_customize_control_card(
                                ui,
                                texture_cache,
                                current_build.as_ref(),
                                control,
                                actions_enabled,
                                disabled_reason,
                            ));
                            ui.add_space(8.0);
                        }
                    }
                });
        });
        commands
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

    fn show_direction_board_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let generating = matches!(
            view_state.mode,
            MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas
        );
        let active_group = self.active_make_part_group();
        let ideas_title = active_group.as_ref().map_or_else(
            || "Candidate tray".to_owned(),
            |group| format!("{} ideas", singular_title_case_part_label(&group.label)),
        );
        let direction_count_label = if generating {
            view_state
                .local_busy_label
                .as_deref()
                .unwrap_or("Trying ideas from the current asset...")
                .to_owned()
        } else {
            direction_board_count_label(view_state.candidate_count)
        };
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Ideas",
                title: ideas_title.as_str(),
                subtitle: Some(direction_count_label.as_str()),
            },
        );
        if let Some(message) = &view_state.local_warning_message {
            status_banner(
                ui,
                StatusBannerSpec {
                    title: "Older result ignored",
                    message,
                    tone: BannerTone::Warning,
                },
            );
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
        if generating {
            show_direction_skeleton_grid(ui);
            return commands;
        }
        if self.state.candidates.is_empty() {
            if let Some(group) = active_group.as_ref() {
                let message = format!(
                    "No clear {} ideas survived. Try unlocking more controls.",
                    singular_part_copy(&group.label).to_ascii_lowercase()
                );
                product_empty_state(ui, "No clear ideas yet", &message);
            } else {
                product_empty_state(
                    ui,
                    "No ideas yet",
                    "Try whole-asset ideas to compare readable candidates.",
                );
            }
            return commands;
        }

        let current_preview = self.state.current_preview.as_ref();
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        commands.extend(show_selected_candidate_comparison(
            ui,
            texture_cache,
            current_build,
            current_preview,
            &self.state.candidates,
            make_canvas_candidate_actions_enabled(view_state),
            view_state
                .primary_action_disabled_reason
                .as_deref()
                .unwrap_or(ACTIVE_IDEA_JOB_REASON),
        ));
        ui.add_space(10.0);
        commands.extend(show_direction_candidate_grid(
            ui,
            texture_cache,
            current_build,
            &self.state.candidates,
            make_canvas_candidate_actions_enabled(view_state),
            view_state
                .primary_action_disabled_reason
                .as_deref()
                .unwrap_or(ACTIVE_IDEA_JOB_REASON),
        ));
        commands
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
                            "Tune the main asset controls and lock the parts you want to keep.",
                        ),
                    },
                );
                if self.state.document.is_none() {
                    commands.extend(self.show_choose_asset_empty_state(
                        ui,
                        "Choose an asset first",
                        "Pick a template or open a project before customizing.",
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
                title: "Export ready",
                subtitle: Some("Send the current asset or prepared pack to disk."),
            },
        );
        ui.add_space(10.0);
        if self.state.document.is_none() {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Pick a template or open a project before exporting.",
            ));
            return commands;
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
                "Pick a template or open a project before starting a pack.",
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
        let preview = self.state.current_preview.as_ref();
        let has_output = self.state.current_output.is_some();
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
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
            } else if has_output {
                if rendering_preview {
                    ui.weak("Rendering preview...");
                } else {
                    ui.weak("Preview is being prepared.");
                }
            } else {
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
        for effect in effects {
            match effect {
                FoundryAppEffect::StartJob(request) => self.submit_job(*request),
                FoundryAppEffect::SaveProject { path, project } => {
                    self.save_project(path, *project);
                }
                FoundryAppEffect::LoadProject(path) => self.load_project(path, ctx),
            }
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
                schedule_preview |= should_preview;
                if let Some(request) = candidate_preview_request {
                    schedule_candidate_previews = Some(request);
                }
            }
        }

        if schedule_preview {
            let preview_pixels = current_preview_pixels_for_context(ctx);
            match self.state.request_preview(preview_pixels, preview_pixels) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if let Some((request, output)) = schedule_candidate_previews {
            match self.state.request_candidate_previews(request, output) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
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
                Some(
                    ScreenshotScenario::GeneratingWholeAssetIdeas
                        | ScreenshotScenario::GeneratingHandleIdeas
                )
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
                ScreenshotScenario::GeneratingWholeAssetIdeas
                    if view_state.mode == MakeCanvasMode::GeneratingWholeAssetIdeas =>
                {
                    self.complete_screenshot_scenario(scenario);
                    return commands;
                }
                ScreenshotScenario::GeneratingHandleIdeas
                    if view_state.mode == MakeCanvasMode::GeneratingFocusedPartIdeas
                        && view_state.focused_part_label.as_deref() == Some("Handles") =>
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
            ScreenshotScenario::MakeInitialCrate => {
                if self.state.active_jobs.is_empty() {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::GeneratingWholeAssetIdeas => {
                if self.state.candidates.is_empty() && !self.directions_are_generating() {
                    commands.push(make_whole_asset_candidate_request(&self.state));
                    self.screenshot_scenario_step = 2;
                }
            }
            ScreenshotScenario::GeneratedWholeAssetIdeas
            | ScreenshotScenario::SelectedComparison => {
                if self.state.candidates.is_empty() && !self.directions_are_generating() {
                    commands.push(make_whole_asset_candidate_request(&self.state));
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
                        if view_state.mode == MakeCanvasMode::ReviewingIdeas
                            && view_state.preview_ready
                            && view_state.selected_candidate_present
                        {
                            self.complete_screenshot_scenario(scenario);
                        }
                    }
                }
            }
            ScreenshotScenario::FocusHandles => {
                commands.extend(self.ensure_screenshot_focus("handles"));
                if self.make_canvas_view_state().focused_part_label.as_deref() == Some("Handles") {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::GeneratingHandleIdeas => {
                commands.extend(self.ensure_screenshot_focus("handles"));
                if self.screenshot_scenario_step >= 2
                    && self.state.candidates.is_empty()
                    && !self.directions_are_generating()
                    && let Some(group) = screenshot_part_group(&self.state, "handles")
                {
                    commands.push(make_focused_candidate_request(&self.state, &group));
                    self.screenshot_scenario_step = 3;
                }
            }
            ScreenshotScenario::HandleIdeas => {
                commands.extend(self.ensure_screenshot_focus("handles"));
                if self.screenshot_scenario_step >= 2
                    && self.state.candidates.is_empty()
                    && !self.directions_are_generating()
                    && let Some(group) = screenshot_part_group(&self.state, "handles")
                {
                    commands.push(make_focused_candidate_request(&self.state, &group));
                    self.screenshot_scenario_step = 3;
                } else if !self.state.candidates.is_empty() && self.state.active_jobs.is_empty() {
                    if self
                        .state
                        .selected_candidate
                        .as_ref()
                        .is_none_or(|selected| {
                            !self
                                .state
                                .candidates
                                .iter()
                                .any(|card| &card.id == selected)
                        })
                        && let Some(candidate) = self.state.candidates.first()
                    {
                        commands.push(FoundryAppCommand::SelectCandidate(Some(
                            candidate.id.clone(),
                        )));
                    } else if self.make_canvas_view_state().focused_part_label.as_deref()
                        == Some("Handles")
                    {
                        self.complete_screenshot_scenario(scenario);
                    }
                }
            }
            ScreenshotScenario::FocusVents => {
                commands.extend(self.ensure_screenshot_focus("vents"));
                if self.make_canvas_view_state().focused_part_label.as_deref() == Some("Vents") {
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

    fn ensure_screenshot_focus(&mut self, group_id: &str) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if !self.state.active_jobs.is_empty() {
            return commands;
        }
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
        if let Some(group) = screenshot_part_group(&self.state, group_id) {
            commands.push(directions::set_focus_part_group_command(&group));
            self.screenshot_scenario_step = 1;
        }
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
                    self.jobs.reset();
                    self.texture_cache.clear();
                    self.state.status = Some(format!("Loaded {}", path.display()));
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
                    self.remember_recent_project(path.clone());
                    self.apply_effects(effects, ctx);
                }
                Err(error) => self.state.status = Some(error.to_string()),
            },
            Err(error) => self.state.status = Some(error.to_string()),
        }
    }

    fn load_fixture(&mut self, fixture: FoundryFixtureCatalog, ctx: &egui::Context) {
        self.jobs.reset();
        self.texture_cache.clear();
        match FoundryAppState::new(fixture.document) {
            Ok(mut state) => match state.request_build() {
                Ok(effects) => {
                    state.status = Some(format!("Loaded {} fixture.", fixture.slug));
                    self.state = state;
                    self.tab = FoundryTab::Make;
                    self.drawer = None;
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
    let settings = RenderSettings {
        width: HOME_THUMBNAIL_PIXELS,
        height: HOME_THUMBNAIL_PIXELS,
        ..RenderSettings::default()
    };
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
        "make_initial_crate" => Some(ScreenshotScenario::MakeInitialCrate),
        "generating_whole_asset_ideas" => Some(ScreenshotScenario::GeneratingWholeAssetIdeas),
        "generated_whole_asset_ideas" => Some(ScreenshotScenario::GeneratedWholeAssetIdeas),
        "selected_comparison" => Some(ScreenshotScenario::SelectedComparison),
        "focus_handles" => Some(ScreenshotScenario::FocusHandles),
        "generating_handle_ideas" => Some(ScreenshotScenario::GeneratingHandleIdeas),
        "handle_ideas" => Some(ScreenshotScenario::HandleIdeas),
        "focus_vents" => Some(ScreenshotScenario::FocusVents),
        "pack_drawer" => Some(ScreenshotScenario::PackDrawer),
        "export_drawer" => Some(ScreenshotScenario::ExportDrawer),
        _ => None,
    }
}

fn read_screenshot_fixture_catalog() -> shape_foundry_catalog::FoundryFixtureCatalog {
    let path = env::temp_dir().join("shape-lab-screenshot-template.txt");
    let value = fs::read_to_string(path).unwrap_or_default();
    match value.trim() {
        "roman_bridge_hq" => shape_foundry_catalog::roman_bridge::hq_fixture_catalog(),
        "stylized_lamp" => shape_foundry_catalog::stylized_lamp::fixture_catalog(),
        _ => shape_foundry_catalog::scifi_crate::fixture_catalog(),
    }
}

fn screenshot_scenario_assertion(
    scenario: ScreenshotScenario,
    view_state: &MakeCanvasViewState,
) -> Result<(), String> {
    match scenario {
        ScreenshotScenario::MakeInitialCrate => {
            require_screenshot_state(view_state.mode == MakeCanvasMode::Ready, scenario, "Ready")?;
            require_screenshot_state(view_state.model_ready, scenario, "model_ready")?;
            require_screenshot_state(view_state.preview_ready, scenario, "preview_ready")
        }
        ScreenshotScenario::GeneratingWholeAssetIdeas => require_screenshot_state(
            view_state.local_busy_visible
                && view_state.mode == MakeCanvasMode::GeneratingWholeAssetIdeas,
            scenario,
            "local_busy_visible whole-asset generation",
        ),
        ScreenshotScenario::GeneratedWholeAssetIdeas => require_screenshot_state(
            view_state.candidate_tray_visible,
            scenario,
            "candidate_tray_visible",
        ),
        ScreenshotScenario::SelectedComparison => require_screenshot_state(
            view_state.selected_comparison_visible,
            scenario,
            "selected_comparison_visible",
        ),
        ScreenshotScenario::FocusHandles => require_screenshot_state(
            view_state.focused_part_label.as_deref() == Some("Handles"),
            scenario,
            "focused_part_label Handles",
        ),
        ScreenshotScenario::GeneratingHandleIdeas => require_screenshot_state(
            view_state.local_busy_visible
                && view_state.focused_part_label.as_deref() == Some("Handles")
                && view_state.mode == MakeCanvasMode::GeneratingFocusedPartIdeas,
            scenario,
            "local_busy_visible Handles generation",
        ),
        ScreenshotScenario::HandleIdeas => require_screenshot_state(
            view_state.candidate_tray_visible
                && view_state.focused_part_label.as_deref() == Some("Handles"),
            scenario,
            "candidate_tray_visible with Handles focus",
        ),
        ScreenshotScenario::FocusVents => require_screenshot_state(
            view_state.focused_part_label.as_deref() == Some("Vents"),
            scenario,
            "focused_part_label Vents",
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

fn make_whole_asset_candidate_request(state: &FoundryAppState) -> FoundryAppCommand {
    let seed = state.document.as_ref().map_or(0, |document| document.seed);
    FoundryAppCommand::RequestCandidates(FoundryCandidateRequest {
        seed,
        proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
        result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode: FoundryCandidateMode::Explore,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::complete_look(),
    })
}

fn make_focused_candidate_request(
    state: &FoundryAppState,
    group: &directions::DirectionPartGroup,
) -> FoundryAppCommand {
    let seed = state.document.as_ref().map_or(0, |document| document.seed);
    FoundryAppCommand::RequestCandidates(FoundryCandidateRequest {
        seed,
        proposal_count: directions::DEFAULT_DIRECTION_PROPOSALS,
        result_count: directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
        mode: FoundryCandidateMode::Refine,
        strategy_id: None,
        preference_profile: None,
        variation_intent: VariationIntent::focus_part_shape(&group.group_id, &group.label),
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
        id if id.contains("roman-bridge") => "Roman Timber Bridge",
        id if id.contains("sci-fi-crate") => "Sci-Fi Industrial Crate",
        id if id.contains("stylized-lamp") => "Stylized Furniture Lamp",
        id if id.contains("market-stall") => "Market Stall Kit",
        id if id.contains("sci-fi-door") => "Sci-Fi Door Panel",
        id if id.contains("storage-barrel") => "Coopered Storage Barrel",
        id if id.contains("signpost") => "Wayfinding Signpost",
        id if id.contains("workshop-chair") => "Workshop Chair",
        id if id.contains("handcart") => "Market Handcart",
        id if id.contains("stylized-tree") => "Storybook Tree",
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
        "roman-bridge" => {
            "A reinforced timber bridge with supports, bracing, railings, and span controls."
        }
        "sci-fi-crate" => {
            "A compact hard-surface prop with vents, handles, trims, and panel detail."
        }
        "stylized-lamp" => {
            "A furniture-scale lamp with height, stem, base, shade, and softness controls."
        }
        "market-stall" => {
            "A modular market stall with canopy, counter, display, and signage choices."
        }
        "sci-fi-door" => {
            "A panelized industrial door with frame, vents, locks, and surface detail."
        }
        "storage-barrel" => {
            "A coopered storage barrel with bands, proportions, lid, and wearable silhouette."
        }
        "signpost" => "A wayfinding post with arrows, boards, base, and directional variation.",
        "workshop-chair" => {
            "A practical workshop chair with seat, legs, back, braces, and style controls."
        }
        "handcart" => {
            "A market handcart with bed, wheels, handles, rails, and cargo-ready proportions."
        }
        "stylized-tree" => {
            "A storybook tree with trunk, canopy, branch, root, and stylized silhouette controls."
        }
        _ => "A built-in asset template ready for visual direction generation.",
    }
}

#[derive(Clone)]
struct ProductHomeProfile {
    label: String,
    fixture: FoundryFixtureCatalog,
    family_id: String,
    family_name: String,
    quality_badge: String,
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
            let curation = catalog_curation_metadata_for_slug(&fixture.slug)?;
            Some(ProductHomeProfile {
                label: card.display_name.clone(),
                fixture,
                family_id: card.family_id.clone(),
                family_name: card.family_name.clone(),
                quality_badge: curation.state.label().to_owned(),
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
        "Choose what to make",
        "Project",
        "Open Project",
        "Save Project",
        "Save Project As",
        "Start Another Asset",
        "History",
        "Recent Projects",
        "Start",
        "Try ideas",
        "Found 4 clear ideas",
        "Rejected 2 that looked too similar",
        "Handles",
        "Handles is focused",
        "Try handle ideas",
        "Try vent ideas",
        "Lock handles",
        "Clear focus",
        "Add to Pack",
        "Open Pack",
        "Open Export",
        "Material looks are not previewable yet.",
        "This part has no focused variations yet.",
        "Trying ideas...",
        "Choose Template",
        "Preparing model",
        ASSET_PREPARING_REASON,
        STALE_RESULT_WARNING,
        "Current Asset",
        "Candidate tray",
        "Compare current vs. candidate",
        "What changed",
        "Affected parts",
        "Focused: Handles",
        "Focused: Vents",
        "Handle ideas",
        "Vent ideas",
        "No clear ideas yet",
        "No clear handle ideas survived. Try unlocking more controls.",
        "No clear vent ideas survived. Try unlocking more controls.",
        "No ideas yet",
        "Try whole-asset ideas to compare readable candidates.",
        "Ideas",
        "Save",
        "Undo",
        "Choose a template to start",
        "Not saved",
        "Saved",
        "Unsaved",
        "Unsaved changes",
        "Ready",
        "Working",
        "Model ready",
        "Preparing model",
        "Choose a template",
        "Preview ready",
        "Preview available",
        "Preview waiting",
        "Preview",
        "Pack: 0 assets",
        "Export complete",
        "Pack export complete",
        "Current asset ready",
        "Needs a model first",
        HOME_SUBTITLE,
        "Choose a template below to start a new project.",
        "Search assets...",
        "All",
        "Props",
        "Architecture",
        "Gear",
        "Furniture",
        "Environment",
        "Preview building",
        "No matching templates",
        "Make asset",
        "Use the model as the workspace: try ideas, focus parts, tune controls, and compare.",
        "Current Asset",
        "Direction options",
        "Trying ideas",
        "Model workspace",
        "Whole asset",
        "Try broader ideas, then use controls below to tune the current asset.",
        "Pick a template or open a project before making changes.",
        "Preview could not be rendered for this direction.",
        "Preview this direction before choosing it.",
        "Project history",
        "Review previous project steps and branch from a saved point.",
        "saved step(s)",
        "Project step",
        "Tune the main asset controls and lock the parts you want to keep.",
        "Choose an asset first",
        "Pick a template or open a project before customizing.",
        "Make it yours",
        "No quick controls yet",
        "This asset has no quick controls yet.",
        "Preview",
        "More options",
        "This option is not available right now.",
        "This control is locked.",
        "Export ready",
        "Send the current asset or prepared pack to disk.",
        "Current Asset",
        "Export options",
        "Pack members",
        STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL,
        STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION,
        STATIC_PROP_FULL_READY_BLOCKED_NOTE,
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
        "No family pack workspace is open.",
        "Preview is being prepared.",
        "Whole-asset ideas",
        "Primary control",
        "No reviewed templates yet",
        "Open a saved project, or enable the preview catalog for internal kit testing.",
        "Choose a template below to start a new project.",
        "Pick a template or open a project before exporting.",
        "Pick a template or open a project before starting a pack.",
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
        ActionSpec::enabled(ACTION_TRY_WHOLE_ASSET_IDEAS, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_GENERATING_IDEAS, ButtonTone::Primary),
        ActionSpec::enabled("Try handle ideas", ButtonTone::Primary),
        ActionSpec::enabled("Try vent ideas", ButtonTone::Primary),
        ActionSpec::enabled(ACTION_CHOOSE_DIRECTION, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_APPLY, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_FOCUS, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_LOCK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_RESET, ButtonTone::Secondary),
        ActionSpec::enabled("Clear focus", ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_ADD_TO_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_OPEN_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_EXPORT, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_OPEN_EXPORT, ButtonTone::Secondary),
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

fn make_canvas_mode_summary(view_state: &MakeCanvasViewState) -> &'static str {
    match view_state.mode {
        MakeCanvasMode::NoAsset => "Choose a template first.",
        MakeCanvasMode::PreparingAsset => ASSET_PREPARING_REASON,
        MakeCanvasMode::GeneratingWholeAssetIdeas => "Trying 6 ideas from the current asset.",
        MakeCanvasMode::GeneratingFocusedPartIdeas => "Trying ideas for the focused part.",
        MakeCanvasMode::ReviewingIdeas => "Compare the selected idea against the current asset.",
        MakeCanvasMode::FocusedPart => "This part is focused. Try ideas, lock it, or clear focus.",
        MakeCanvasMode::PackDrawerOpen => "The pack drawer is open.",
        MakeCanvasMode::ExportDrawerOpen => "The export drawer is open.",
        MakeCanvasMode::Ready => "Try ideas, focus a part, or tune controls.",
        MakeCanvasMode::Error => "The current asset needs attention.",
    }
}

fn make_canvas_next_action_hint(
    mode: &MakeCanvasMode,
    focused_part_label: Option<&str>,
    selected_comparison_visible: bool,
) -> String {
    match (mode, focused_part_label, selected_comparison_visible) {
        (MakeCanvasMode::NoAsset, _, _) => "Start with a template from Choose.".to_owned(),
        (MakeCanvasMode::PreparingAsset, _, _) => {
            "Wait for the model and preview to finish preparing.".to_owned()
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, Some(part), _) => {
            format!(
                "Watch the tray for new {} ideas.",
                singular_part_copy(part).to_ascii_lowercase()
            )
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, None, _) => {
            "Watch the tray for new focused ideas.".to_owned()
        }
        (MakeCanvasMode::GeneratingWholeAssetIdeas, _, _) => {
            "Watch the tray for new whole-asset ideas.".to_owned()
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

fn direction_board_count_label(count: usize) -> String {
    if count == 0 {
        "Try ideas from the current asset.".to_owned()
    } else {
        format!("Found {count} clear ideas")
    }
}

fn make_busy_asset_noun(asset_name: &str) -> &'static str {
    let lower = asset_name.to_ascii_lowercase();
    if lower.contains("crate") {
        "crate"
    } else if lower.contains("bridge") {
        "bridge"
    } else if lower.contains("lamp") {
        "lamp"
    } else {
        "asset"
    }
}

fn singular_part_copy(label: &str) -> &str {
    match label {
        "Handles" => "Handle",
        "Panels" => "Panel",
        "Vents" => "Vent",
        "Fasteners" => "Fastener",
        "Supports" => "Support",
        "Joints" => "Joint",
        "Ramps" => "Ramp",
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
    product_card(ui, false, |ui| {
        ui.set_min_height(ui.available_height().max(420.0));
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
            RichText::new("Choose a template below to start a new project.")
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(12.0);
        let response = ui.add_sized(
            [ui.available_width(), 32.0],
            egui::TextEdit::singleline(search_query)
                .hint_text("Search assets...")
                .desired_width(f32::INFINITY),
        );
        if response.changed() {
            *selected_slug = default_filtered_home_profile_slug(profiles, search_query, *filter);
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
                        RichText::new("No matching templates")
                            .color(colors.text_muted)
                            .small(),
                    );
                }
                for index in filtered_indices {
                    let profile = &profiles[index];
                    let selected = selected_slug.as_deref() == Some(profile.fixture.slug.as_str());
                    if show_home_template_row(ui, profile, selected).clicked() {
                        *selected_slug = Some(profile.fixture.slug.clone());
                    }
                    ui.add_space(6.0);
                }
            });
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
                    RichText::new(format!(
                        "{}. {}",
                        profile.style_name,
                        profile_description(&profile.fixture.slug)
                    ))
                    .color(colors.text_muted)
                    .small(),
                )
                .wrap(),
            );
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                let _ = status_pill(
                    ui,
                    StatusPillSpec::new(&profile.quality_badge, StatusTone::Working),
                );
                for chip in &profile.category_chips {
                    let _ = status_pill(ui, StatusPillSpec::new(chip, StatusTone::Neutral));
                }
            });
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
            ui.add_space(6.0);
            let _ = status_pill(
                ui,
                StatusPillSpec::new(&profile.quality_badge, StatusTone::Working),
            );
            for chip in &profile.category_chips {
                let _ = status_pill(ui, StatusPillSpec::new(chip, StatusTone::Neutral));
            }
        });
        ui.add_space(8.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{}. {}",
                    profile.style_name,
                    profile_description(&profile.fixture.slug)
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
        1 => "1 template".to_owned(),
        count => format!("{count} templates"),
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
            Self::Props => "Props",
            Self::Architecture => "Architecture",
            Self::Gear => "Gear",
            Self::Furniture => "Furniture",
            Self::Environment => "Environment",
        }
    }

    fn matches(self, profile: &ProductHomeProfile) -> bool {
        if self == Self::All {
            return true;
        }
        let chips = profile
            .category_chips
            .iter()
            .map(|chip| chip.to_ascii_lowercase())
            .collect::<Vec<_>>();
        match self {
            Self::All => true,
            Self::Props => chips.iter().any(|chip| chip == "prop" || chip == "weapon"),
            Self::Architecture => chips
                .iter()
                .any(|chip| chip == "architecture" || chip == "structure"),
            Self::Gear => chips
                .iter()
                .any(|chip| chip == "armor" || chip == "weapon" || chip == "heroic"),
            Self::Furniture => chips
                .iter()
                .any(|chip| chip == "furniture" || chip == "lighting"),
            Self::Environment => chips
                .iter()
                .any(|chip| chip == "environment" || chip == "market" || chip == "wayfinding"),
        }
    }
}

fn start_template_button(ui: &mut egui::Ui) -> egui::Response {
    ui.horizontal(|ui| action_button(ui, &ActionSpec::enabled(ACTION_START, ButtonTone::Primary)))
        .inner
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
            RichText::new("Compare current vs. candidate")
                .color(colors.accent_hover)
                .small()
                .strong(),
        );
        ui.add_space(8.0);
        let preview_edge = (ui.available_width() * 0.08).clamp(72.0, 96.0);
        ui.horizontal_top(|ui| {
            ui.vertical_centered(|ui| {
                ui.set_width(preview_edge + 18.0);
                ui.label(RichText::new("Current").color(colors.text_muted).small());
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
                ui.set_width(preview_edge + 18.0);
                ui.label(RichText::new("Candidate").color(colors.text_muted).small());
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
            ui.add_space(18.0);
            ui.vertical(|ui| {
                ui.set_width(ui.available_width().max(260.0));
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
                        .map(|reason| {
                            product_panel_message(
                                reason,
                                "Preview this direction before choosing it.",
                            )
                        })
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
                            RichText::new(format!("Direction {}", slot))
                                .color(colors.text)
                                .strong(),
                        );
                        ui.label(
                            RichText::new("Preparing whole-model option.")
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
        ui.weak("Current direction");
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
            .map(|reason| {
                product_panel_message(reason, "Preview this direction before choosing it.")
            })
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

    let intent = product_panel_message(&candidate.variation_intent_label, "Direction");
    let channels = candidate
        .variation_channel_labels
        .iter()
        .map(|label| product_panel_message(label, "Variation"))
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
            "This direction is unavailable for the current kit.",
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

    Some("Direction changes are visible in the comparison above.".to_owned())
}

const DIRECTION_INTENT_TITLES: [&str; 6] = [
    "Compact Vented",
    "Reinforced Cargo",
    "Minimal Industrial",
    "Side-Rail Utility",
    "Deep Panel Build",
    "Clean Service Variant",
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
        || lower.contains("detail density")
        || lower.contains("panel depth")
        || lower.contains("structural heft")
        || lower.contains("handle style")
}

fn candidate_change_phrase(raw: &str) -> Option<String> {
    let lower = raw.to_ascii_lowercase();
    let phrase = if lower.contains("edge") && lower.contains("soft") {
        "softer edges"
    } else if lower.contains("structural") || lower.contains("heft") {
        "heavier frame"
    } else if lower.contains("handle") {
        "handle variation"
    } else if lower.contains("detail") {
        "more surface detail"
    } else if lower.contains("panel") || lower.contains("depth") {
        "deeper panels"
    } else if lower.contains("vent") {
        "more vents"
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
    if action_button(
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

fn make_control_matches_focus(
    control: &crate::foundry::view_model::FoundryControlView,
    active_group: Option<&directions::DirectionPartGroup>,
) -> bool {
    let Some(group) = active_group else {
        return true;
    };
    let control_text = format!("{} {}", control.id, control.label).to_ascii_lowercase();
    let group_label = group.label.to_ascii_lowercase();
    if group_label.contains("handle") {
        control_text.contains("handle") || control_text.contains("heft")
    } else if group_label.contains("vent") {
        control_text.contains("vent") || control_text.contains("panel")
    } else if group_label.contains("body") {
        control_text.contains("body")
            || control_text.contains("proportion")
            || control_text.contains("heft")
            || control_text.contains("edge")
    } else if group_label.contains("panel") {
        control_text.contains("panel") || control_text.contains("detail")
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
        ui.add_space(10.0);
        export_checklist_row(ui, "Model prepared", can_export_current, NEED_MODEL_REASON);
        export_checklist_row(
            ui,
            "Preview ready",
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
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(
            RichText::new("Material looks are not previewable yet.")
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL)
                .color(colors.text)
                .strong(),
        );
        ui.label(
            RichText::new(STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION)
                .color(colors.text_muted)
                .small(),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(STATIC_PROP_FULL_READY_BLOCKED_NOTE)
                .color(colors.text_muted)
                .small(),
        );
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
        app.load_fixture(shape_foundry_catalog::roman_bridge::fixture_catalog(), &ctx);

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
        let normalized = normalize_foundry_project_path(PathBuf::from("bridge.json"));
        assert_eq!(normalized, PathBuf::from("bridge.shapelab-foundry.json"));
        ensure_foundry_project_path(&normalized).expect("normalized path is loadable");
    }

    #[test]
    fn desktop_foundry_pack_action_dispatches_through_reducer() {
        let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
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
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc"
        );

        pack.members.insert(
            "roman-bridge-doc".to_owned(),
            shape_foundry::FoundryDocumentId("roman-bridge-doc".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc-2"
        );

        pack.members.insert(
            "roman-bridge-doc-2".to_owned(),
            shape_foundry::FoundryDocumentId("roman-bridge-doc-2".to_owned()),
        );
        assert_eq!(
            unique_pack_member_id(&pack, "roman-bridge-doc"),
            "roman-bridge-doc-3"
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
        assert_eq!(installed_product_kit_count(), 17);
        assert_eq!(default_product_home_profile_count(), 3);

        let default_profiles = product_home_profiles(false);
        let default_labels = default_profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();
        assert!(default_labels.contains(&"Roman Timber Bridge HQ"));
        assert!(default_labels.contains(&"Sci-Fi Industrial Crate"));
        assert!(default_labels.contains(&"Stylized Furniture Lamp"));
        assert!(!default_labels.contains(&"Roman Timber Bridge"));
        assert!(!default_labels.contains(&"Sci-Fi Door Panel"));

        let profiles = product_home_profiles(true);
        let labels = profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(profiles.len(), 16);
        assert!(labels.contains(&"Roman Timber Bridge"));
        assert!(labels.contains(&"Roman Timber Bridge HQ"));
        assert!(labels.contains(&"Sci-Fi Industrial Crate"));
        assert!(labels.contains(&"Stylized Furniture Lamp"));
        assert!(labels.contains(&"Market Stall Kit"));
        assert!(labels.contains(&"Sci-Fi Door Panel"));
        assert!(labels.contains(&"Coopered Storage Barrel"));
        assert!(labels.contains(&"Wayfinding Signpost"));
        assert!(labels.contains(&"Workshop Chair"));
        assert!(labels.contains(&"Market Handcart"));
        assert!(labels.contains(&"Storybook Tree"));
        assert!(labels.contains(&"Fantasy Sword"));
        assert!(labels.contains(&"Round Shield"));
        assert!(labels.contains(&"Hero Helmet"));
        assert!(labels.contains(&"Pauldron Pair"));
        assert!(labels.contains(&"Chest Armor"));
        assert!(!labels.contains(&"Hero Character"));
        assert!(!labels.iter().any(|label| label.contains("MVP")));
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
        assert!(family_names.contains(&"Bridge"));
        assert!(family_names.contains(&"Crate"));
        assert!(family_names.contains(&"Lamp"));
        assert!(!family_names.contains(&"Hero Character"));
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

        let bridge = groups
            .iter()
            .find(|group| group.family_name == "Bridge")
            .expect("bridge family group");
        let bridge_labels = bridge
            .profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();
        assert_eq!(bridge.profiles.len(), 2);
        assert!(bridge_labels.contains(&"Roman Timber Bridge"));
        assert!(bridge_labels.contains(&"Roman Timber Bridge HQ"));
    }

    #[test]
    fn home_template_search_defaults_to_first_matching_profile() {
        let profiles = product_home_profiles(false);
        let selected_slug =
            default_filtered_home_profile_slug(&profiles, "lamp", HomeTemplateFilter::All);

        assert_eq!(selected_slug.as_deref(), Some("stylized-lamp"));
    }

    #[test]
    fn home_template_selection_tracks_filter_visibility() {
        let profiles = product_home_profiles(false);
        let mut selected_slug = Some("sci-fi-crate".to_owned());

        normalize_home_selection(
            &profiles,
            "",
            HomeTemplateFilter::Architecture,
            &mut selected_slug,
        );

        assert_eq!(selected_slug.as_deref(), Some("roman-bridge-hq"));
        assert!(
            filtered_home_profile_indices(&profiles, "", HomeTemplateFilter::Props)
                .iter()
                .any(|index| profiles[*index].fixture.slug == "sci-fi-crate")
        );
    }

    #[test]
    fn product_home_grouping_uses_stable_family_ids() {
        let profiles = product_home_profiles(true);
        let bridge = profiles
            .iter()
            .find(|profile| profile.label == "Roman Timber Bridge")
            .expect("bridge profile")
            .clone();
        let mut crate_profile = profiles
            .iter()
            .find(|profile| profile.label == "Sci-Fi Industrial Crate")
            .expect("crate profile")
            .clone();
        crate_profile.family_name = bridge.family_name.clone();

        let groups = product_home_profile_groups(vec![bridge.clone(), crate_profile.clone()]);
        assert_eq!(groups.len(), 2);
        assert!(
            groups
                .iter()
                .any(|group| group.family_id == bridge.family_id)
        );
        assert!(
            groups
                .iter()
                .any(|group| group.family_id == crate_profile.family_id)
        );
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
        assert_eq!(current_preview_pixels_for_scale(1.5), 768);
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
    fn foundry_recovery_docs_do_not_claim_current_make_dogfood_success() {
        let readme = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../README.md"));
        assert!(readme.contains("Choose -> Make"));
        assert!(!readme.contains("Open Directions"));
        assert!(!readme.contains("Customize controls"));

        let screenshot_results = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/MAKE_CANVAS_SCREENSHOT_GATE_RESULTS.md"
        ));
        assert!(screenshot_results.contains("HUMAN DOGFOOD NOT PASSED"));
        assert!(screenshot_results.contains("NO-GO"));

        let integration_report = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/PRODUCT_QUALITY_RECOVERY_INTEGRATION_REPORT.md"
        ));
        assert!(integration_report.contains("HUMAN DOGFOOD NO-GO"));
        assert!(integration_report.contains("unstable product-recovery baseline"));

        let manual_gate = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/RELEASE_CANDIDATE_MANUAL_GATE.md"
        ));
        assert!(manual_gate.contains("human dogfood video audit is"));
        assert!(manual_gate.contains("NO-GO"));
    }

    #[test]
    fn export_surface_copy_states_package_without_full_game_ready_claim() {
        let strings = product_visible_strings_for_default_shell();

        assert!(strings.contains(&STATIC_PROP_SURFACE_PACKAGE_AVAILABLE_LABEL));
        assert!(strings.contains(&STATIC_PROP_SURFACE_PACKAGE_DESCRIPTION));
        assert!(strings.contains(&STATIC_PROP_FULL_READY_BLOCKED_NOTE));

        let joined = strings.join("\n").to_ascii_lowercase();
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
            "rigged",
            "animated",
            "retarget",
        ] {
            assert!(
                !joined.contains(overclaim),
                "export copy should not overclaim {overclaim}: {joined}"
            );
        }
    }

    #[test]
    fn product_shell_steps_are_novice_facing() {
        let strings = product_visible_strings_for_default_shell();

        for required in [
            "Choose what to make",
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
            "Try ideas",
            "Choose Template",
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
            "Focus Part",
            "Generate 6 Directions",
        ] {
            assert!(
                !strings.contains(&forbidden),
                "Make canvas product copy should not expose {forbidden}"
            );
        }

        for required in [
            "Try ideas",
            "Try handle ideas",
            "Use this idea",
            "Candidate tray",
            "Material looks are not previewable yet.",
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
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
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
        assert_eq!(visible.primary_action_label, ACTION_TRY_WHOLE_ASSET_IDEAS);
        assert_eq!(
            visible.next_action_hint,
            "Compare the selected idea, then use it or reject it."
        );
    }

    #[test]
    fn make_canvas_view_state_requires_selected_candidate_for_comparison() {
        let mut app = visible_state_test_app();
        app.state.current_preview = Some(test_preview_image("current"));
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.candidates = vec![test_candidate_card("candidate-a", false, None)];

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::ReviewingIdeas);
        assert_eq!(visible.candidate_count, 1);
        assert!(visible.candidate_tray_visible);
        assert!(!visible.selected_candidate_present);
        assert!(!visible.selected_comparison_visible);
        assert_eq!(
            visible.next_action_hint,
            "Select an idea to compare it against the current asset."
        );
    }

    #[test]
    fn focus_part_changes_visible_scope_and_local_tray() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        set_test_focus_scope(&mut app, "handles", "Handles");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::FocusedPart);
        assert_eq!(visible.primary_title, "Handles");
        assert_eq!(visible.focused_part_label.as_deref(), Some("Handles"));
        assert!(visible.focused_part_visible);
        assert!(visible.focused_part_actions_visible);
        assert_eq!(visible.primary_action_label, "Try handle ideas");
        assert_eq!(
            visible.next_action_hint,
            "Try handle ideas, lock this part, or clear focus."
        );
    }

    #[test]
    fn focus_vents_changes_visible_scope_and_action() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        set_test_focus_scope(&mut app, "vents", "Vents");

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.mode, MakeCanvasMode::FocusedPart);
        assert_eq!(visible.primary_title, "Vents");
        assert_eq!(visible.focused_part_label.as_deref(), Some("Vents"));
        assert!(visible.focused_part_visible);
        assert_eq!(visible.primary_action_label, "Try vent ideas");
    }

    #[test]
    fn focused_generation_state_does_not_render_whole_asset_heading() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        set_test_focus_scope(&mut app, "handles", "Handles");
        let selected = shape_foundry::FoundryCandidateId("handle-candidate".to_owned());
        app.state.current_preview = Some(test_preview_image("current"));
        app.state.selected_candidate = Some(selected.clone());
        app.state.candidates = vec![test_candidate_card(
            &selected.0,
            true,
            Some("Handles".to_owned()),
        )];

        let visible = app.make_canvas_view_state();

        assert_eq!(visible.primary_title, "Handles");
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
    fn screenshot_focus_scenario_helper_waits_for_visible_focus() {
        let mut app = visible_state_test_app();

        let commands = app.ensure_screenshot_focus("handles");
        assert_eq!(app.screenshot_scenario_step, 1);
        assert_eq!(commands.len(), 1);
        assert_eq!(
            app.make_canvas_view_state().focused_part_label.as_deref(),
            None
        );

        set_test_focus_scope(&mut app, "handles", "Handles");
        let commands = app.ensure_screenshot_focus("handles");
        assert!(commands.is_empty());
        assert_eq!(app.screenshot_scenario_step, 2);
        assert_eq!(
            app.make_canvas_view_state().focused_part_label.as_deref(),
            Some("Handles")
        );
    }

    #[test]
    fn screenshot_state_assertions_cover_required_make_scenarios() {
        let mut app = ready_visible_state_test_app();
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::MakeInitialCrate,
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
                ScreenshotScenario::GeneratingWholeAssetIdeas,
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
            screenshot_scenario_assertion(ScreenshotScenario::GeneratedWholeAssetIdeas, &view)
                .is_ok()
        );
        assert!(
            screenshot_scenario_assertion(ScreenshotScenario::SelectedComparison, &view).is_ok()
        );

        set_test_focus_scope(&mut app, "handles", "Handles");
        app.state.candidates = vec![test_candidate_card(
            &selected.0,
            true,
            Some("Handles".to_owned()),
        )];
        let view = app.make_canvas_view_state();
        assert!(screenshot_scenario_assertion(ScreenshotScenario::FocusHandles, &view).is_ok());
        assert!(screenshot_scenario_assertion(ScreenshotScenario::HandleIdeas, &view).is_ok());

        set_test_focus_scope(&mut app, "vents", "Vents");
        assert!(
            screenshot_scenario_assertion(
                ScreenshotScenario::FocusVents,
                &app.make_canvas_view_state(),
            )
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
        assert_eq!(
            visible.primary_action_disabled_reason.as_deref(),
            Some(ASSET_PREPARING_REASON)
        );
        assert_eq!(
            visible.local_busy_label.as_deref(),
            Some("Preparing Sci-Fi Industrial Crate...")
        );
        assert!(visible.local_busy_visible);
    }

    #[test]
    fn starting_template_queues_model_and_preview_automatically() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();

        app.load_fixture(shape_foundry_catalog::scifi_crate::fixture_catalog(), &ctx);

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
    fn active_candidate_job_disables_conflicting_actions() {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
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
        assert_eq!(
            visible.next_action_hint,
            "Watch the tray for new whole-asset ideas."
        );
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
        for forbidden_phrase in ["fingerprint", "gltf primitive", "rigged", "animated"] {
            assert!(
                !joined.contains(forbidden_phrase),
                "default product strings contain forbidden phrase {forbidden_phrase}: {joined}"
            );
        }
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
            ("Choose a template to start", StatusTone::Neutral)
        );

        app.state =
            FoundryAppState::new(shape_foundry_catalog::roman_bridge::fixture_catalog().document)
                .expect("fixture state");
        app.state.project_path = None;
        app.state.dirty = false;
        assert_eq!(app.save_state_pill(), ("Not saved", StatusTone::Warning));

        app.state.project_path = Some(PathBuf::from("roman.shapelab-foundry.json"));
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
        app.load_fixture(shape_foundry_catalog::roman_bridge::fixture_catalog(), &ctx);

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
        assert!(
            controls
                .iter()
                .any(|control| control.kind.to_ascii_lowercase().contains("provider")),
            "fixture should cover internal provider-style control kinds"
        );
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
    fn compact_filmstrip_does_not_hide_remaining_options() {
        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(shape_foundry_catalog::scifi_crate::fixture_catalog(), &ctx);

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if app
                .state
                .controls
                .iter()
                .filter(|control| control.primary && control.visible)
                .any(|control| control.options.len() > CONTROL_FILMSTRIP_LIMIT)
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let control = display_customize_controls(&app.state.controls)
            .into_iter()
            .find(|control| control.options.len() > CONTROL_FILMSTRIP_LIMIT)
            .expect("fixture exposes a control with more options than the filmstrip");
        let filmstrip_count = control.options.iter().take(CONTROL_FILMSTRIP_LIMIT).count();
        let more_count = control.options.iter().skip(CONTROL_FILMSTRIP_LIMIT).count();

        assert_eq!(filmstrip_count, CONTROL_FILMSTRIP_LIMIT);
        assert!(more_count > 0);
        assert_eq!(filmstrip_count + more_count, control.options.len());
    }

    #[test]
    fn pack_member_labels_hide_source_document_ids() {
        let member = pack::PackMemberRow {
            member_id: "roman-bridge-doc".to_owned(),
            name: "roman-bridge-doc".to_owned(),
            document_id: "roman-bridge-doc".to_owned(),
            selected: false,
            override_count: 0,
        };

        let label = pack_member_display_name(&member);
        assert_eq!(label, "Roman Timber Bridge");
        assert!(!label.contains("-doc"));

        let cell = pack::PackContactSheetCell {
            row: 0,
            column: 0,
            member_id: "roman-bridge-doc".to_owned(),
            name: "roman-bridge-doc".to_owned(),
            document_id: "roman-bridge-doc".to_owned(),
            status: pack::PackMemberStatus::Ready,
            override_count: 0,
            selected: false,
        };
        let cell_label = pack_cell_display_name(&cell);
        assert_eq!(cell_label, "Roman Timber Bridge");
        assert!(!cell_label.contains("-doc"));
    }

    #[test]
    fn pack_contact_sheet_uses_product_safe_thumbnail_markers() {
        let current_cell = pack::PackContactSheetCell {
            row: 0,
            column: 0,
            member_id: "roman-bridge-doc".to_owned(),
            name: "roman-bridge-doc".to_owned(),
            document_id: "roman-bridge-doc".to_owned(),
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
                "members.roman-bridge-doc.document_id failed validation",
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
            product_safe_status("Saved C:\\work\\roman.shapelab-foundry.json"),
            "Project saved"
        );
        assert_eq!(
            product_safe_status("Loaded C:\\work\\roman.shapelab-foundry.json"),
            "Project loaded"
        );
        assert_eq!(
            product_safe_status("Could not use C:\\work\\broken.json"),
            "Project path needs attention"
        );
        assert_eq!(
            product_safe_status("Exported default to C:\\exports\\bridge"),
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
        let bridge = shape_foundry_catalog::roman_bridge::fixture_catalog();
        let crate_fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
        let build_a = compile_foundry_document(&bridge.document, &bridge)
            .expect("bridge fixture compiles")
            .build_stamp;
        let build_b = compile_foundry_document(&crate_fixture.document, &crate_fixture)
            .expect("crate fixture compiles")
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
        assert_ne!(
            identity,
            FoundryTextureIdentity::new("option-a", Some(&build_b), 2, 1)
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new("option-a", Some(&build_a), 1, 2)
        );
    }

    #[test]
    fn desktop_foundry_exposes_product_steps_and_lamp_profile() {
        let tabs = [FoundryTab::Home, FoundryTab::Make, FoundryTab::History];
        assert_eq!(tabs.len(), 3);

        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::stylized_lamp::fixture_catalog(),
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
            Some("stylized-lamp-family")
        );
        assert!(app.state.current_output.is_some());
    }

    #[test]
    fn loading_project_enters_workflow_step() {
        let fixture = shape_foundry_catalog::roman_bridge::fixture_catalog();
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
        app.load_fixture(shape_foundry_catalog::scifi_crate::fixture_catalog(), &ctx);

        for _ in 0..3000 {
            app.poll_jobs(&ctx);
            if default_customize_controls(&app.state.controls)
                .any(|control| control.id == "body_proportions")
            {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let default_ids = default_customize_controls(&app.state.controls)
            .map(|control| control.id.as_str())
            .collect::<Vec<_>>();
        assert!(default_ids.contains(&"body_proportions"));
        assert!(!default_ids.contains(&"has_trim"));
        assert!(!default_ids.contains(&"runtime_wear"));
        assert!(!default_ids.contains(&"advisory_weathering"));
        assert!(
            default_customize_controls(&app.state.controls)
                .all(|control| control.primary && control.visible)
        );
    }

    fn visible_state_test_app() -> FoundryDesktopApp {
        FoundryDesktopApp {
            tab: FoundryTab::Make,
            state: FoundryAppState::new(
                shape_foundry_catalog::scifi_crate::fixture_catalog().document,
            )
            .expect("fixture state"),
            ..FoundryDesktopApp::default()
        }
    }

    fn ready_visible_state_test_app() -> FoundryDesktopApp {
        let mut app = visible_state_test_app();
        app.state.current_output = Some(Box::new(
            compile_foundry_document(
                app.state.document.as_ref().expect("document"),
                &shape_foundry_catalog::scifi_crate::fixture_catalog(),
            )
            .expect("fixture compiles"),
        ));
        app.state.current_preview = Some(test_preview_image("current"));
        app
    }

    fn test_preview_image(preview_id: &str) -> FoundryPreviewImage {
        FoundryPreviewImage {
            preview_id: preview_id.to_owned(),
            rgba8: vec![24, 32, 40, 255],
            width: 1,
            height: 1,
            camera: OrbitCamera::default(),
            build: None,
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
            title: "Reinforced cargo idea".to_owned(),
            subtitle: "Clear model change".to_owned(),
            preview_id: Some(format!("{candidate_id}-preview")),
            rgba8: vec![220, 225, 214, 255],
            width: 1,
            height: 1,
            camera: Some(OrbitCamera::default()),
            preview_failure: None,
            changed_controls: vec!["Handle Style".to_owned()],
            changed_roles: vec!["Handles".to_owned()],
            explanations: Vec::new(),
            rejections: std::collections::BTreeMap::new(),
            validation_label: "Ready".to_owned(),
            validation_detail: None,
            selectable: true,
            selected,
            variation_intent_label: "Handle idea".to_owned(),
            variation_scope_label: "Focused: Handles".to_owned(),
            variation_channel_labels: vec!["Shape".to_owned()],
            visible_delta_label: "Clear change".to_owned(),
            what_changed_summary: "Handles change with visible attached mounts.".to_owned(),
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
