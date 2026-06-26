//! Native desktop host for the Foundry workflow state.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
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
    FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels, headless_fixture_catalogs,
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
    FoundryPreviewImage,
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
    jobs: FoundryJobCoordinator,
    home_thumbnails: HomeThumbnailCoordinator,
    texture_cache: FoundryTextureCache,
    home_profiles: Vec<ProductHomeProfile>,
    home_search_query: String,
    home_filter: HomeTemplateFilter,
    selected_home_profile_slug: Option<String>,
    recent_projects: Vec<PathBuf>,
    requested_start_window_mode: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FoundryTab {
    Home,
    Directions,
    Customize,
    Pack,
    History,
    Export,
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
    "Start with an asset template, then generate directions and customize the result.";
const NEED_PROJECT_REASON: &str = "Choose a template or open a project first.";
const NEED_SAVE_LOCATION_REASON: &str =
    "Use Save Project first to choose where this project is saved.";
const NEED_MODEL_REASON: &str = "Build the current model first.";
const NEED_HISTORY_REASON: &str = "No earlier project step is available.";
const NEED_DIRECTION_REASON: &str = "This direction is not ready to choose.";
const NEED_RESET_REASON: &str = "This control is already at its starting value.";
const NEED_PACK_MEMBER_REASON: &str = "Add at least one asset before exporting a pack.";
const CUSTOMIZE_PRIMARY_CONTROL_LIMIT: usize = 7;
const CONTROL_FILMSTRIP_LIMIT: usize = 5;
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
const ACTION_GENERATE_DIRECTIONS: &str = "Generate 6 Directions";
const ACTION_GENERATING_DIRECTIONS: &str = "Generating...";
const ACTION_CHOOSE_TEMPLATE: &str = "Choose Template";
const ACTION_BUILD_ASSET: &str = "Build Asset";
const ACTION_REFRESH_PREVIEW: &str = "Refresh Preview";
const ACTION_SWITCH: &str = "Switch";
const ACTION_BRANCH: &str = "Branch";
const ACTION_EXPORT_CURRENT_ASSET: &str = "Export Current Asset";
const ACTION_ADD_CURRENT_ASSET: &str = "Add Current Asset";
const ACTION_EXPORT_PACK: &str = "Export Pack";
const ACTION_SELECT: &str = "Preview Direction";
const ACTION_CHOOSE_DIRECTION: &str = "Use This Direction";
const ACTION_REJECT: &str = "Reject";
const ACTION_RESET: &str = "Reset";
const ACTION_UNLOCK: &str = "Unlock";
const ACTION_LOCK: &str = "Lock";
const ACTION_FOCUS: &str = "Focus";
const ACTION_TRY: &str = "Preview";
const ACTION_APPLY: &str = "Use Option";
const RENDERED_ACTION_LABELS: [&str; 38] = [
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
    ACTION_GENERATE_DIRECTIONS,
    ACTION_GENERATING_DIRECTIONS,
    ACTION_CHOOSE_TEMPLATE,
    ACTION_BUILD_ASSET,
    ACTION_REFRESH_PREVIEW,
    "Refine",
    "Explore",
    "Silhouette",
    "Structure",
    "Detail",
    ACTION_SWITCH,
    ACTION_BRANCH,
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
    ACTION_TRY,
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
            jobs: FoundryJobCoordinator::default(),
            home_thumbnails: HomeThumbnailCoordinator::default(),
            texture_cache: FoundryTextureCache::default(),
            home_profiles,
            home_search_query: String::new(),
            home_filter: HomeTemplateFilter::All,
            selected_home_profile_slug,
            recent_projects: Vec::new(),
            requested_start_window_mode: false,
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
        let mut commands = Vec::new();
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
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.center_bg)
                    .inner_margin(egui::Margin::symmetric(22, 18)),
            )
            .show_inside(ui, |ui| match self.tab {
                FoundryTab::Home => self.show_home(ui),
                FoundryTab::Directions => commands.extend(self.show_directions(ui)),
                FoundryTab::Customize => commands.extend(self.show_customize(ui)),
                FoundryTab::Pack => commands.extend(self.show_pack(ui)),
                FoundryTab::History => commands.extend(self.show_history(ui)),
                FoundryTab::Export => commands.extend(self.show_export(ui)),
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
            let has_document = self.state.document.is_some();
            let has_output = self.state.current_output.is_some();
            ui.label(
                RichText::new(self.current_project_title())
                    .size(16.0)
                    .strong(),
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
                        has_output,
                        ACTION_EXPORT,
                        ButtonTone::Primary,
                        NEED_MODEL_REASON,
                    ),
                )
                .clicked()
                {
                    self.tab = FoundryTab::Export;
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
                ui.close();
            }
            if ui.button(ACTION_PROJECT_HISTORY).clicked() {
                self.tab = FoundryTab::History;
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
            "Model needs build"
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

    fn show_directions(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        egui::ScrollArea::vertical()
            .id_salt("foundry_directions_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                section_header(
                    ui,
                    SectionHeaderSpec {
                        eyebrow: "Directions",
                        title: "Explore directions",
                        subtitle: Some(
                            "Generate coherent whole-model options from the current asset.",
                        ),
                    },
                );
                if self.state.document.is_none() {
                    commands.extend(self.show_choose_asset_empty_state(
                        ui,
                        "Choose an asset first",
                        "Pick a template or open a project to generate directions.",
                    ));
                } else {
                    ui.add_space(12.0);

                    let generating = self.directions_are_generating();
                    let has_output = self.state.current_output.is_some();
                    commands.extend(self.show_direction_control_panel(ui, generating, has_output));
                    ui.add_space(18.0);
                    commands.extend(self.show_direction_board_panel(ui, generating));
                }
            });
        commands
    }

    fn show_direction_control_panel(
        &mut self,
        ui: &mut egui::Ui,
        generating: bool,
        has_output: bool,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        let panel_height = ui.available_height().clamp(300.0, 460.0);
        product_card(ui, false, |ui| {
            ui.set_min_height(panel_height);
            ui.horizontal_top(|ui| {
                let preview_edge = (ui.available_width() * 0.42).clamp(360.0, 620.0);
                ui.vertical(|ui| {
                    ui.set_width(preview_edge + 24.0);
                    ui.label(
                        RichText::new("Current model")
                            .color(colors.accent_hover)
                            .small()
                            .strong(),
                    );
                    self.show_current_preview_sized(ui, preview_edge);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(self.current_project_title())
                            .color(colors.text)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Use the model as the anchor, then generate visually distinct directions.")
                            .color(colors.text_muted)
                            .small(),
                    );
                });
                ui.add_space(20.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width().clamp(360.0, 680.0));
                    ui.label(
                        RichText::new("Generate directions")
                            .color(colors.text)
                            .size(20.0)
                            .strong(),
                    );
                    ui.label(
                        RichText::new("Choose the kind of variation you want. Surface is listed only as an export capability until textured previews exist.")
                            .color(colors.text_muted)
                            .small(),
                    );
                    ui.add_space(14.0);
                    ui.horizontal_wrapped(|ui| {
                        let variation_actions =
                            direction_variation_mode_actions_for_panel(self.state.document.as_ref());
                        let generate_action = variation_actions
                            .iter()
                            .find(|action| action.label == "Complete Looks" && action.enabled)
                            .or_else(|| variation_actions.iter().find(|action| action.enabled));
                        if let Some(generate_action) = generate_action
                            && action_button(
                                ui,
                                &action_spec(
                                    !generating,
                                    if generating {
                                        ACTION_GENERATING_DIRECTIONS
                                    } else {
                                        ACTION_GENERATE_DIRECTIONS
                                    },
                                    ButtonTone::Primary,
                                    "Directions are being generated.",
                                ),
                            )
                            .clicked()
                            && let Some(command) = generate_action.app_command()
                        {
                            commands.push(command);
                        }
                        if !has_output
                            && action_button(
                                ui,
                                &ActionSpec::enabled(ACTION_BUILD_ASSET, ButtonTone::Secondary),
                            )
                            .clicked()
                        {
                            commands.push(FoundryAppCommand::RequestBuild);
                        }
                        if action_button(
                            ui,
                            &action_spec(
                                has_output,
                                ACTION_REFRESH_PREVIEW,
                                ButtonTone::Secondary,
                                NEED_MODEL_REASON,
                            ),
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
                    ui.add_space(18.0);
                    ui.label(
                        RichText::new("Variation mode")
                            .color(colors.text_muted)
                            .small()
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for variation_action in
                            direction_variation_mode_actions_for_panel(self.state.document.as_ref())
                        {
                            let response = variation_mode_button(
                                ui,
                                variation_action.label,
                                variation_action.selected,
                                variation_action.enabled,
                                variation_action.unavailable_reason.unwrap_or(NEED_DIRECTION_REASON),
                            );
                            if response.clicked()
                                && let Some(command) = variation_action.app_command()
                            {
                                commands.push(command);
                            }
                        }
                    });
                    for reason in direction_variation_mode_actions_for_panel(self.state.document.as_ref())
                        .into_iter()
                        .filter_map(|action| {
                            (!action.enabled)
                                .then_some(action.unavailable_reason)
                                .flatten()
                        })
                    {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(reason)
                                .color(colors.text_muted)
                                .small(),
                        );
                    }
                    ui.add_space(16.0);
                    if let Some(document) = self.state.document.as_ref() {
                        let part_groups = directions::direction_part_groups_for_document(document);
                        if !part_groups.is_empty() {
                            let active_group_id = document
                                .variation_state
                                .intent
                                .scope
                                .semantic_part_group_id()
                                .map(str::to_owned);
                            ui.label(
                                RichText::new("Focus Part")
                                    .color(colors.text_muted)
                                    .small()
                                    .strong(),
                            );
                            ui.add_space(6.0);
                            ui.horizontal_wrapped(|ui| {
                                for group in &part_groups {
                                    let selected = active_group_id.as_deref()
                                        == Some(group.group_id.as_str());
                                    let reason = group
                                        .unavailable_reason
                                        .as_deref()
                                        .unwrap_or("This part has no focused variations yet.");
                                    let response = variation_mode_button(
                                        ui,
                                        &directions::focus_part_chip_label(group),
                                        selected,
                                        group.focusable,
                                        reason,
                                    );
                                    if response.clicked() && group.focusable {
                                        commands
                                            .push(directions::set_focus_part_group_command(group));
                                    }
                                }
                            });
                            if let Some(active_group) =
                                active_group_id.as_deref().and_then(|group_id| {
                                    part_groups.iter().find(|group| group.group_id == group_id)
                                })
                            {
                                ui.add_space(8.0);
                                ui.horizontal_wrapped(|ui| {
                                    let generate_label =
                                        directions::generate_focused_part_label(active_group);
                                    let lock_label =
                                        directions::lock_focused_part_label(active_group);
                                    ui.label(
                                        RichText::new(directions::focus_part_status_label(
                                            active_group,
                                        ))
                                        .color(colors.accent_hover)
                                        .strong(),
                                    );
                                    if action_button(
                                        ui,
                                        &ActionSpec::enabled(&generate_label, ButtonTone::Primary),
                                    )
                                    .clicked()
                                    {
                                        commands.push(FoundryAppCommand::RequestCandidates(
                                            FoundryCandidateRequest {
                                                seed: document.seed,
                                                proposal_count:
                                                    directions::DEFAULT_DIRECTION_PROPOSALS,
                                                result_count:
                                                    directions::VISIBLE_DIRECTION_CANDIDATE_CARDS,
                                                mode: FoundryCandidateMode::Refine,
                                                strategy_id: None,
                                                preference_profile: None,
                                                variation_intent: VariationIntent::focus_part_shape(
                                                    &active_group.group_id,
                                                    &active_group.label,
                                                ),
                                            },
                                        ));
                                    }
                                    if action_button(
                                        ui,
                                        &ActionSpec::enabled(&lock_label, ButtonTone::Secondary),
                                    )
                                    .clicked()
                                    {
                                        commands.push(FoundryAppCommand::run(
                                            FoundryCommand::SetLock {
                                                lock: shape_foundry::FoundryLock {
                                                    target: shape_foundry::FoundryLockTarget::FocusPartGroup(
                                                        active_group.group_id.clone(),
                                                    ),
                                                    mode: shape_foundry::FoundryLockMode::SearchProtected,
                                                    reason: Some(format!(
                                                        "{} kept while generating directions.",
                                                        active_group.label
                                                    )),
                                                },
                                            },
                                        ));
                                    }
                                    if action_button(
                                        ui,
                                        &ActionSpec::enabled("Clear focus", ButtonTone::Quiet),
                                    )
                                    .clicked()
                                    {
                                        commands
                                            .push(directions::clear_focus_part_group_command());
                                    }
                                });
                            }
                            ui.add_space(16.0);
                        }
                    }
                    ui.label(
                        RichText::new("Generation style")
                            .color(colors.text_muted)
                            .small()
                            .strong(),
                    );
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for mode_action in direction_mode_actions_for_panel() {
                            if variation_mode_button(ui, mode_action.label, false, true, "").clicked() {
                                commands.push(mode_action.app_command());
                            }
                        }
                    });
                    if generating {
                        ui.add_space(16.0);
                        status_banner(
                            ui,
                            StatusBannerSpec {
                                title: "Generating 6 directions",
                                message: &format!(
                                    "Generating 6 directions from {}...",
                                    self.current_project_title()
                                ),
                                tone: BannerTone::Info,
                            },
                        );
                    }
                });
            });
        });
        commands
    }

    fn show_direction_board_panel(
        &mut self,
        ui: &mut egui::Ui,
        generating: bool,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let direction_count_label = if generating && self.state.candidates.is_empty() {
            "Preparing six whole-model options.".to_owned()
        } else {
            direction_board_count_label(self.state.candidates.len())
        };
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Explore Directions",
                title: "Direction options",
                subtitle: Some(direction_count_label.as_str()),
            },
        );
        ui.add_space(8.0);
        if generating {
            show_direction_skeleton_grid(ui);
            return commands;
        }
        if self.state.candidates.is_empty() {
            product_empty_state(
                ui,
                "No directions yet",
                "Generate 6 directions to compare coherent whole-model options.",
            );
            return commands;
        }

        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        commands.extend(show_direction_candidate_grid(
            ui,
            texture_cache,
            current_build,
            &self.state.candidates,
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
                "Pick a template or open a project before building a pack.",
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
            let current_preview = self.state.current_preview.clone();
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
                    current_preview.as_ref(),
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

    fn show_current_preview_sized(&mut self, ui: &mut egui::Ui, max_edge: f32) {
        let preview = self.state.current_preview.clone();
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
                    ui.weak("Preview is ready to refresh.");
                }
            } else {
                ui.weak("Build the current asset to render a preview.");
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
        for command in commands {
            match self.state.handle_command(command) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
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
            ctx.request_repaint();
        }
    }

    fn poll_home_thumbnail_jobs(&mut self, ctx: &egui::Context) {
        if self.home_thumbnails.poll() {
            ctx.request_repaint();
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
                    self.state.status = Some(format!("Loaded {}", path.display()));
                    self.tab = FoundryTab::Directions;
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
        match FoundryAppState::new(fixture.document) {
            Ok(mut state) => match state.request_build() {
                Ok(effects) => {
                    state.status = Some(format!("Loaded {} fixture.", fixture.slug));
                    self.state = state;
                    self.tab = FoundryTab::Directions;
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

fn tab_for_workflow_step(index: usize) -> FoundryTab {
    match index {
        1 => FoundryTab::Home,
        2 => FoundryTab::Directions,
        3 => FoundryTab::Customize,
        4 => FoundryTab::Pack,
        5 => FoundryTab::Export,
        _ => FoundryTab::Home,
    }
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
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .filter_map(|(_label, fixture)| {
            let card = cards
                .iter()
                .find(|card| card.source_profile_slug.as_deref() == Some(fixture.slug.as_str()))?;
            if !product_home_profile_visible(card, developer_preview_enabled) {
                return None;
            }
            Some(ProductHomeProfile {
                label: card.display_name.clone(),
                fixture,
                family_id: card.family_id.clone(),
                family_name: card.family_name.clone(),
                quality_badge: card.quality_badge.clone(),
                style_name: card.style_name.clone(),
                category_chips: card.category_chips.clone(),
            })
        })
        .collect()
}

fn product_home_profile_visible(
    card: &crate::foundry::kit_view::FoundryKitCardView,
    developer_preview_enabled: bool,
) -> bool {
    developer_preview_enabled || !card.hidden_by_default || card.quality_badge == "Usable"
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
        "Directions",
        "Customize",
        "Pack",
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
        "Generate 6 Directions",
        "Generated 4 visually distinct directions.",
        "Rejected 2 subtle candidates that looked too similar.",
        "Focus: Handles",
        "Focused: Handles",
        "Generate handle variations",
        "Lock handles",
        "Clear focus",
        "Surface options need textured previews before they can be shown.",
        "This part has no focused variations yet.",
        "Generating...",
        "Choose Template",
        "Build Asset",
        "Refresh Preview",
        "Current Asset",
        "Direction options",
        "No directions yet",
        "Generate 6 directions to compare coherent whole-model options.",
        "Direction focus",
        "Refine",
        "Explore",
        "Silhouette",
        "Structure",
        "Detail",
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
        "Model needs build",
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
        "Explore directions",
        "Generate coherent whole-model options from the current asset.",
        "Current Asset",
        "Direction options",
        "Generating 6 directions",
        "Generate visual directions from this model.",
        "Current model",
        "Use the model as the anchor, then generate visually distinct directions.",
        "Generate directions",
        "Choose the kind of variation you want.",
        "Surface is listed only as an export capability until textured previews exist.",
        "Pick a template or open a project to generate directions.",
        "Preview could not be rendered for this direction.",
        "Preview this direction before choosing it.",
        "Project history",
        "Review previous project steps and branch from a saved point.",
        "saved step(s)",
        "Project step",
        "Adjust controls",
        "Tune the main asset controls and lock the parts you want to keep.",
        "Choose an asset first",
        "Pick a template or open a project before customizing.",
        "Whole-model preview",
        "Select a part or option below; the full asset stays in view for context.",
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
        "Export this asset here, or export the prepared pack from Pack.",
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
        "Build the current asset before exporting.",
        "No family pack workspace is open.",
        "Preview is ready to refresh.",
        "Whole-model options",
        "Primary control",
        "No reviewed templates yet",
        "Open a saved project, or enable the preview catalog for internal kit testing.",
        "Choose a template below to start a new project.",
        "Pick a template or open a project before exporting.",
        "Pick a template or open a project before building a pack.",
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
        "Generate six coherent whole-model options.".to_owned()
    } else {
        format!("{count} coherent whole-model option(s)")
    }
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

fn show_direction_candidate_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
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
                ));
            }
        });
        ui.add_space(8.0);
    }
    commands
}

fn direction_grid_columns(width: f32) -> usize {
    if width >= 1320.0 { 2 } else { 1 }
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
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, candidate.selected, |ui| {
        let preview_id = candidate_preview_texture_id(candidate);
        let wide_layout = ui.available_width() >= 720.0;
        ui.set_min_height(if wide_layout { 300.0 } else { 420.0 });
        let preview_edge = if wide_layout {
            (ui.available_width() * 0.42).clamp(360.0, 520.0)
        } else {
            ui.available_width().clamp(260.0, 420.0)
        };
        if wide_layout {
            ui.horizontal_top(|ui| {
                ui.vertical_centered(|ui| {
                    ui.set_width(preview_edge + 12.0);
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
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.set_width(ui.available_width());
                    commands.extend(show_direction_candidate_details(ui, candidate));
                });
            });
        } else {
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
            ui.add_space(8.0);
            commands.extend(show_direction_candidate_details(ui, candidate));
        }
    });
    commands
}

fn show_direction_candidate_details(
    ui: &mut egui::Ui,
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
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
        if action_button(ui, &ActionSpec::enabled(ACTION_SELECT, ButtonTone::Quiet)).clicked() {
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
                candidate.selectable,
                ACTION_CHOOSE_DIRECTION,
                ButtonTone::Primary,
                choose_reason.as_str(),
            ),
        )
        .clicked()
        {
            commands.push(directions::accept_candidate_command(candidate.id.clone()));
        }
        if action_button(ui, &ActionSpec::enabled(ACTION_REJECT, ButtonTone::Quiet)).clicked() {
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
    let intent = product_panel_message(&candidate.variation_intent_label, "Direction");
    let delta = product_panel_message(&candidate.visible_delta_label, "Visible change");
    if let Some(focus_part) = &candidate.focus_part_label {
        return format!("{intent} · {focus_part} · {delta}");
    }
    let channels = candidate
        .variation_channel_labels
        .iter()
        .map(|label| product_panel_message(label, "Variation"))
        .collect::<Vec<_>>();
    if channels.is_empty() {
        format!("{intent} · {delta}")
    } else {
        format!("{intent} · {} · {delta}", channels.join(", "))
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
    if !candidate.what_changed_summary.trim().is_empty() {
        let summary =
            product_panel_message(&candidate.what_changed_summary, "Visible shape adjusted.");
        if !summary.trim().is_empty() {
            return Some(summary);
        }
    }
    let labels = candidate
        .changed_controls
        .iter()
        .chain(candidate.changed_roles.iter())
        .filter_map(|label| candidate_change_phrase(label))
        .take(3)
        .collect::<Vec<_>>();
    if labels.is_empty() {
        return None;
    }
    Some(labels.join(" · "))
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
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, control.locked, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(&control.label).strong());
                ui.label(
                    RichText::new(product_control_summary(control))
                        .color(colors.text_muted)
                        .small(),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if action_button(
                    ui,
                    &action_spec(
                        customize::control_can_reset(control),
                        ACTION_RESET,
                        ButtonTone::Quiet,
                        NEED_RESET_REASON,
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
                if action_button(ui, &ActionSpec::enabled(lock_label, ButtonTone::Quiet)).clicked()
                    && let Some(command) = customize::control_lock_command(control, !control.locked)
                {
                    commands.push(command);
                }
                if action_button(ui, &ActionSpec::enabled(ACTION_FOCUS, ButtonTone::Quiet))
                    .clicked()
                {
                    commands.push(customize::select_control_command(Some(control.id.clone())));
                }
            });
        });
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
        commands.extend(show_customize_option_grid(
            ui,
            texture_cache,
            current_build,
            control,
            &visible_options,
            true,
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
            ));
        }
    });
    commands
}

fn show_customize_option_grid(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    control: &crate::foundry::view_model::FoundryControlView,
    options: &[&crate::foundry::view_model::FoundryOptionCard],
    show_preview: bool,
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
                ));
            }
        });
        ui.add_space(6.0);
    }
    commands
}

fn customize_option_grid_columns(width: f32) -> usize {
    if width >= 960.0 {
        4
    } else if width >= 700.0 {
        3
    } else if width >= 460.0 {
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
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    product_card(ui, option.selected, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        let tile_width = ui.available_width().clamp(156.0, 260.0);
        ui.set_width(tile_width);
        ui.set_min_height(if show_preview { 190.0 } else { 124.0 });
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
                    max_edge: 112.0,
                },
            );
        }
        ui.add(egui::Label::new(RichText::new(&option.label).color(colors.text).strong()).wrap());
        if option.selected {
            ui.weak("Current");
        }
        let disabled_reason = customize::option_action_disabled_reason(control, option);
        let disabled_message = disabled_reason
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
                    disabled_reason.is_none(),
                    ACTION_TRY,
                    ButtonTone::Quiet,
                    disabled_message.as_str(),
                ),
            )
            .clicked()
            {
                commands.extend(customize::preview_control_value_intents(
                    control,
                    option.value.clone(),
                ));
            }
            if action_button(
                ui,
                &action_spec(
                    disabled_reason.is_none(),
                    ACTION_APPLY,
                    ButtonTone::Secondary,
                    disabled_message.as_str(),
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
        export_checklist_row(ui, "Model built", can_export_current, NEED_MODEL_REASON);
        export_checklist_row(
            ui,
            "Preview ready",
            can_export_current,
            "Build the current model first.",
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
        "Export this asset here, or export the prepared pack from Pack."
    } else if can_export {
        "Export the current asset as an individual result."
    } else {
        "Build the current asset before exporting."
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
    rgba_fingerprint: u64,
}

impl FoundryTextureIdentity {
    fn new(
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Self {
        Self {
            preview_id: preview_id.to_owned(),
            build_fingerprint: build.map(|build| build.build_fingerprint.0.to_hex()),
            width,
            height,
            rgba_fingerprint: preview_image_fingerprint(rgba8),
        }
    }

    fn texture_name(&self) -> String {
        format!(
            "foundry-preview-{}-{}x{}-{}-{:016x}",
            self.preview_id,
            self.width,
            self.height,
            self.build_fingerprint.as_deref().unwrap_or("no-build"),
            self.rgba_fingerprint
        )
    }
}

impl FoundryTextureCache {
    fn texture(
        &mut self,
        ctx: &egui::Context,
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        rgba8: &[u8],
        width: u32,
        height: u32,
    ) -> egui::TextureHandle {
        let identity = FoundryTextureIdentity::new(preview_id, build, width, height, rgba8);
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

fn preview_image_fingerprint(rgba8: &[u8]) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    rgba8.iter().fold(OFFSET, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(PRIME)
    })
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

        for _ in 0..200 {
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
    fn product_home_shows_usable_kits_by_default_and_preview_mode_lists_seventeen() {
        assert_eq!(installed_product_kit_count(), 17);
        assert!(default_product_home_profile_count() > 0);
        assert!(default_product_home_profile_count() < 17);

        let default_profiles = product_home_profiles(false);
        let default_labels = default_profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();
        assert!(default_labels.contains(&"Sci-Fi Industrial Crate"));
        assert!(default_labels.contains(&"Roman Timber Bridge HQ"));
        assert!(!default_labels.contains(&"Roman Timber Bridge"));

        let profiles = product_home_profiles(true);
        let labels = profiles
            .iter()
            .map(|profile| profile.label.as_str())
            .collect::<Vec<_>>();

        assert_eq!(profiles.len(), 17);
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
        assert!(labels.contains(&"Hero Character"));
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
        assert!(family_names.contains(&"Hero Character"));
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
            default_filtered_home_profile_slug(&profiles, "door", HomeTemplateFilter::All);

        assert_eq!(selected_slug.as_deref(), Some("sci-fi-door"));
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
            "Directions",
            "Customize",
            "Pack",
            "Export",
            "Visual Foundry",
            "Project",
            "Open Project",
            "Save Project",
            "Save Project As",
            "Start Another Asset",
            "History",
            "Generate 6 Directions",
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

        for _ in 0..200 {
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

        assert_eq!(
            tabs,
            vec![
                FoundryTab::Home,
                FoundryTab::Directions,
                FoundryTab::Customize,
                FoundryTab::Pack,
                FoundryTab::Export,
            ]
        );
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
    fn preview_texture_identity_tracks_preview_id_build_and_pixels() {
        let bridge = shape_foundry_catalog::roman_bridge::fixture_catalog();
        let crate_fixture = shape_foundry_catalog::scifi_crate::fixture_catalog();
        let build_a = compile_foundry_document(&bridge.document, &bridge)
            .expect("bridge fixture compiles")
            .build_stamp;
        let build_b = compile_foundry_document(&crate_fixture.document, &crate_fixture)
            .expect("crate fixture compiles")
            .build_stamp;

        let identity = FoundryTextureIdentity::new(
            "option-a",
            Some(&build_a),
            2,
            1,
            &[0, 0, 0, 255, 255, 255, 255, 255],
        );

        assert_eq!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-b",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_b),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 255, 255],
            )
        );
        assert_ne!(
            identity,
            FoundryTextureIdentity::new(
                "option-a",
                Some(&build_a),
                2,
                1,
                &[0, 0, 0, 255, 255, 255, 254, 255],
            )
        );
    }

    #[test]
    fn desktop_foundry_exposes_product_steps_and_lamp_profile() {
        let tabs = [
            FoundryTab::Home,
            FoundryTab::Directions,
            FoundryTab::Customize,
            FoundryTab::Pack,
            FoundryTab::Export,
            FoundryTab::History,
        ];
        assert_eq!(tabs.len(), 6);

        let ctx = egui::Context::default();
        let mut app = FoundryDesktopApp::default();
        app.load_fixture(
            shape_foundry_catalog::stylized_lamp::fixture_catalog(),
            &ctx,
        );

        for _ in 0..200 {
            app.poll_jobs(&ctx);
            if app.state.current_output.is_some() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        assert_eq!(app.tab, FoundryTab::Directions);
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

        assert_eq!(app.tab, FoundryTab::Directions);
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
