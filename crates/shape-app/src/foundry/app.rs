//! Native desktop host for the Foundry workflow state.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, unbounded};
use egui::{ColorImage, RichText, TextureOptions};
use shape_foundry::{
    CatalogContentRef, FoundryBuildStamp, FoundryCatalogError, FoundryCatalogResolver,
    FoundryCommand,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels, headless_fixture_catalogs,
};
use shape_project::foundry::{
    FOUNDRY_PROJECT_FILE_SUFFIX, FoundryProject, FoundryProjectFile, ensure_foundry_project_path,
};
use shape_render::foundry::FoundryPreviewCache;

use crate::foundry::{
    FoundryAppCommand, FoundryAppEffect, FoundryAppState, FoundryJobEvent, FoundryJobRequest,
    panels::{customize, directions, history, pack},
    run_foundry_job,
    ui::{
        copy::WORKFLOW_STEPS,
        theme::apply_visual_foundry_theme,
        tokens::VisualFoundryTokens,
        widgets::{
            ActionSpec, ButtonTone, ProfileCardSpec, SectionHeaderSpec, StatusPillSpec, StatusTone,
            action_button, profile_card, section_header, status_pill,
        },
    },
};

/// Native Foundry workflow surface.
pub(crate) struct FoundryDesktopApp {
    state: FoundryAppState,
    tab: FoundryTab,
    jobs: FoundryJobCoordinator,
    texture_cache: FoundryTextureCache,
    recent_projects: Vec<PathBuf>,
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

impl Default for FoundryDesktopApp {
    fn default() -> Self {
        Self {
            state: FoundryAppState::default(),
            tab: FoundryTab::Home,
            jobs: FoundryJobCoordinator::default(),
            texture_cache: FoundryTextureCache::default(),
            recent_projects: Vec::new(),
        }
    }
}

impl FoundryDesktopApp {
    /// Draw the Foundry workflow surface.
    pub(crate) fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        apply_visual_foundry_theme(&ctx);
        self.poll_jobs(&ctx);

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
        egui::Panel::bottom("foundry_status")
            .default_size(tokens.sizing.status_bar_height)
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .fill(colors.top_bar)
                    .inner_margin(egui::Margin::symmetric(16, 6))
                    .show(ui, |ui| self.show_status_strip(ui));
            });
        egui::Panel::left("foundry_step_rail")
            .resizable(false)
            .default_size(tokens.sizing.left_rail_width)
            .show_inside(ui, |ui| {
                egui::Frame::new()
                    .fill(colors.left_rail)
                    .inner_margin(egui::Margin::symmetric(14, 14))
                    .show(ui, |ui| {
                        commands.extend(self.show_step_rail(ui));
                    });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.center_bg)
                    .inner_margin(egui::Margin::symmetric(16, 14)),
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
        if !self.state.active_jobs.is_empty() {
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
                    &action_spec(has_output, "Export", ButtonTone::Primary, NEED_MODEL_REASON),
                )
                .clicked()
                {
                    self.tab = FoundryTab::Export;
                }
                if action_button(
                    ui,
                    &action_spec(can_save, "Save", ButtonTone::Secondary, save_reason),
                )
                .clicked()
                {
                    commands.push(FoundryAppCommand::Save);
                }
                if action_button(
                    ui,
                    &action_spec(can_undo, "Undo", ButtonTone::Quiet, NEED_HISTORY_REASON),
                )
                .clicked()
                {
                    commands.push(history::undo_command());
                }
            });
        });
        commands
    }

    fn show_step_rail(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        ui.horizontal(|ui| {
            ui.label(RichText::new("Shape Lab").size(16.0).strong());
        });
        ui.add_space(18.0);
        ui.label(
            RichText::new("VISUAL FOUNDRY")
                .color(colors.accent_hover)
                .small(),
        );
        ui.add_space(8.0);
        for step in WORKFLOW_STEPS {
            let tab = tab_for_workflow_step(step.index);
            let selected = self.tab == tab;
            let label = format!("{}  {}", step.index, step.label);
            let response = ui.selectable_label(selected, label);
            if response.clicked() {
                self.tab = tab;
            }
            ui.indent(format!("step-detail-{}", step.index), |ui| {
                ui.label(RichText::new(step.detail).color(colors.text_muted).small());
            });
            ui.add_space(6.0);
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(RichText::new("PROJECT").color(colors.accent_hover).small());
        if action_button(ui, &ActionSpec::enabled("Open Project", ButtonTone::Quiet)).clicked()
            && let Some(path) = open_foundry_project_file()
        {
            commands.push(FoundryAppCommand::Load(path));
        }
        let save_project_spec = action_spec(
            self.state.document.is_some(),
            "Save Project",
            ButtonTone::Quiet,
            NEED_PROJECT_REASON,
        );
        if action_button(ui, &save_project_spec).clicked() {
            if self.state.project_path.is_some() {
                commands.push(FoundryAppCommand::Save);
            } else if let Some(path) = save_foundry_project_file() {
                commands.push(FoundryAppCommand::SaveAs(path));
            }
        }
        if action_button(
            ui,
            &action_spec(
                self.state.document.is_some(),
                "Save Project As",
                ButtonTone::Quiet,
                NEED_PROJECT_REASON,
            ),
        )
        .clicked()
            && let Some(path) = save_foundry_project_file()
        {
            commands.push(FoundryAppCommand::SaveAs(path));
        }
        if action_button(ui, &ActionSpec::enabled("New Project", ButtonTone::Quiet)).clicked() {
            self.tab = FoundryTab::Home;
        }
        if action_button(
            ui,
            &ActionSpec::enabled("Project History", ButtonTone::Quiet),
        )
        .clicked()
        {
            self.tab = FoundryTab::History;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);
        ui.label(
            RichText::new("RECENT PROJECTS")
                .color(colors.accent_hover)
                .small(),
        );
        if self.recent_projects.is_empty() {
            ui.label(RichText::new("No recent projects yet.").color(colors.text_muted));
            ui.label(
                RichText::new("Open a project to keep working here.")
                    .color(colors.text_subtle)
                    .small(),
            );
        } else {
            for path in self.recent_projects.iter().take(4) {
                let title = project_file_title(path);
                let response = action_button(ui, &ActionSpec::enabled(&title, ButtonTone::Quiet));
                if response.clicked() {
                    commands.push(FoundryAppCommand::Load(path.clone()));
                }
                if self.state.project_path.as_ref() == Some(path) {
                    ui.label(
                        RichText::new("Current project")
                            .color(colors.text_muted)
                            .small(),
                    );
                }
            }
        }
        commands
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
            ("No project", StatusTone::Neutral)
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

    fn show_home(&mut self, ui: &mut egui::Ui) {
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Choose",
                title: "Choose what to make",
                subtitle: Some(HOME_SUBTITLE),
            },
        );
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if action_button(
                ui,
                &ActionSpec::enabled("Open Project", ButtonTone::Secondary),
            )
            .clicked()
                && let Some(path) = open_foundry_project_file()
            {
                self.apply_commands(vec![FoundryAppCommand::Load(path)], ui.ctx());
            }
            ui.label(RichText::new("Pick a template to generate whole-model directions.").small());
        });
        ui.add_space(14.0);
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.columns(2, |columns| {
                for (index, (label, fixture)) in built_in_fixture_catalogs_with_labels()
                    .into_iter()
                    .enumerate()
                {
                    let column = &mut columns[index % 2];
                    let response = profile_card(
                        column,
                        ProfileCardSpec {
                            title: label,
                            description: profile_description(&fixture.slug),
                            action: ActionSpec::enabled("Start", ButtonTone::Primary),
                        },
                    );
                    if response
                        .action
                        .as_ref()
                        .is_some_and(egui::Response::clicked)
                    {
                        self.load_fixture(fixture, column.ctx());
                    }
                    column.add_space(8.0);
                }
            });
        });
    }

    fn show_directions(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Directions",
                title: "Explore directions",
                subtitle: Some("Generate coherent whole-model options from the current asset."),
            },
        );
        if self.state.document.is_none() {
            ui.weak("Open a Foundry project to generate whole-model directions.");
            return commands;
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let has_document = self.state.document.is_some();
            let has_output = self.state.current_output.is_some();
            let mode_actions = directions::direction_mode_actions(None, 0, None);
            if let Some(generate_action) = mode_actions.first()
                && action_button(
                    ui,
                    &ActionSpec::enabled("Generate Directions", ButtonTone::Primary),
                )
                .clicked()
            {
                commands.push(generate_action.app_command());
            }
            if action_button(
                ui,
                &action_spec(
                    has_document,
                    "Build Asset",
                    ButtonTone::Secondary,
                    NEED_PROJECT_REASON,
                ),
            )
            .clicked()
            {
                commands.push(FoundryAppCommand::RequestBuild);
            }
            if action_button(
                ui,
                &action_spec(
                    has_output,
                    "Refresh Preview",
                    ButtonTone::Secondary,
                    NEED_MODEL_REASON,
                ),
            )
            .clicked()
            {
                commands.push(FoundryAppCommand::RequestPreview);
            }
            for action in mode_actions {
                if action_button(ui, &ActionSpec::enabled(action.label, ButtonTone::Quiet))
                    .clicked()
                {
                    commands.push(action.app_command());
                }
            }
        });
        ui.add_space(12.0);
        self.show_current_preview(ui);
        ui.label(format!(
            "{} candidate direction(s)",
            self.state.candidates.len()
        ));
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        for candidate in &self.state.candidates {
            ui.horizontal(|ui| {
                let preview_id = candidate_preview_texture_id(candidate);
                show_rgba_preview(
                    ui,
                    texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: current_build,
                        rgba8: &candidate.rgba8,
                        width: candidate.width,
                        height: candidate.height,
                        max_edge: 96.0,
                    },
                );
                ui.vertical(|ui| {
                    ui.label(&candidate.title);
                    ui.weak(&candidate.subtitle);
                    if candidate.width > 0 && candidate.height > 0 {
                        ui.weak(format!("{}x{}", candidate.width, candidate.height));
                    }
                    if let Some(reason) = &candidate.preview_failure {
                        ui.weak(reason);
                    }
                    if candidate.selected {
                        ui.weak("selected");
                    }
                });
                if ui.button("Select").clicked() {
                    commands.push(FoundryAppCommand::SelectCandidate(Some(
                        candidate.id.clone(),
                    )));
                }
                let choose_reason = candidate
                    .preview_failure
                    .as_deref()
                    .unwrap_or(NEED_DIRECTION_REASON);
                if button_with_disabled_reason(ui, candidate.selectable, "Choose", choose_reason)
                    .clicked()
                {
                    commands.push(directions::accept_candidate_command(candidate.id.clone()));
                }
                if ui.button("Reject").clicked() {
                    commands.push(directions::reject_candidate_command(candidate.id.clone()));
                }
            });
        }
        commands
    }

    fn show_customize(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Customize");
        self.show_current_preview(ui);
        if self.state.controls.is_empty() {
            ui.weak("This model has no quick controls yet.");
            return commands;
        }
        let current_build = self.state.current_build.as_ref();
        let texture_cache = &mut self.texture_cache;
        for control in default_customize_controls(&self.state.controls) {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(&control.label);
                    ui.weak(product_control_summary(control));
                    ui.weak(format!("{} option(s)", control.options.len()));
                    if let Some(reason) = &control.locked_reason {
                        ui.weak(reason);
                    }
                    if ui.button("Select").clicked() {
                        commands.push(customize::select_control_command(Some(control.id.clone())));
                    }
                    let lock_label = if control.locked { "Unlock" } else { "Lock" };
                    if ui.button(lock_label).clicked()
                        && let Some(command) =
                            customize::control_lock_command(control, !control.locked)
                    {
                        commands.push(command);
                    }
                    if button_with_disabled_reason(
                        ui,
                        customize::control_can_reset(control),
                        "Reset",
                        NEED_RESET_REASON,
                    )
                    .clicked()
                    {
                        commands.extend(customize::reset_control_intents(control));
                    }
                });
                for option in &control.options {
                    ui.horizontal(|ui| {
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
                                max_edge: 64.0,
                            },
                        );
                        ui.vertical(|ui| {
                            ui.weak(&option.label);
                            if option.selected {
                                ui.weak("current");
                            }
                            if let Some(reason) = &option.unavailable_reason {
                                ui.weak(reason);
                            }
                        });
                        let disabled_reason =
                            customize::option_action_disabled_reason(control, option);
                        let preview_response =
                            ui.add_enabled(disabled_reason.is_none(), egui::Button::new("Preview"));
                        let preview_clicked = if let Some(reason) = &disabled_reason {
                            preview_response.on_disabled_hover_text(reason).clicked()
                        } else {
                            preview_response.clicked()
                        };
                        if preview_clicked {
                            commands.extend(customize::preview_control_value_intents(
                                control,
                                option.value.clone(),
                            ));
                        }
                        let apply_response =
                            ui.add_enabled(disabled_reason.is_none(), egui::Button::new("Apply"));
                        let apply_clicked = if let Some(reason) = &disabled_reason {
                            apply_response.on_disabled_hover_text(reason).clicked()
                        } else {
                            apply_response.clicked()
                        };
                        if apply_clicked {
                            commands.extend(customize::choose_option_intents(control, option));
                        }
                    });
                }
            });
        }
        commands
    }

    fn show_history(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = history::build_history_view(&self.state);
        ui.heading("History");
        ui.horizontal(|ui| {
            for action in &view.actions {
                if button_with_disabled_reason(
                    ui,
                    action.enabled,
                    &action.label,
                    NEED_HISTORY_REASON,
                )
                .clicked()
                    && let Some(command) = self.history_dispatch_command(action.dispatch.as_ref())
                {
                    commands.push(command);
                }
            }
        });
        ui.label(format!("{} revision(s)", view.rows.len()));
        for row in view.rows {
            ui.horizontal(|ui| {
                ui.label(row.label);
                if row.selected {
                    ui.weak("current");
                }
                if let Some(intent) = &row.switch_intent
                    && ui.button("Switch").clicked()
                    && let Some(command) = self.history_dispatch_command(intent.dispatch.as_ref())
                {
                    commands.push(command);
                }
                if let Some(intent) = &row.branch_intent
                    && ui.button("Branch").clicked()
                    && let Some(command) = self.history_dispatch_command(intent.dispatch.as_ref())
                {
                    commands.push(command);
                }
            });
        }
        commands
    }

    fn show_export(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        ui.heading("Export");
        self.show_current_preview(ui);
        let can_export = self.state.current_output.is_some();
        if button_with_disabled_reason(ui, can_export, "Export Current Asset", NEED_MODEL_REASON)
            .clicked()
            && let Some(out_dir) = select_asset_export_dir()
        {
            commands.push(FoundryAppCommand::run(FoundryCommand::Export {
                profile: "default".to_owned(),
                out_dir: Some(out_dir.to_string_lossy().to_string()),
            }));
        }
        if self.state.pack.can_export {
            ui.weak("Pack export is available from the Pack tab.");
        }
        if !can_export {
            ui.weak("Build the current asset before exporting.");
        }
        commands
    }

    fn show_pack(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view = pack::pack_panel_view(&self.state.pack);
        ui.heading("Pack");
        ui.horizontal(|ui| {
            let add_enabled = self.state.document.is_some();
            if button_with_disabled_reason(
                ui,
                add_enabled,
                "Add Current Asset",
                NEED_PROJECT_REASON,
            )
            .clicked()
                && let Some(command) = self.add_current_to_pack_command()
            {
                commands.push(command);
            }
            let batch_export_reason = view
                .export
                .disabled_reason
                .as_deref()
                .unwrap_or(NEED_PACK_MEMBER_REASON);
            if button_with_disabled_reason(
                ui,
                view.export.enabled,
                "Batch Export",
                batch_export_reason,
            )
            .clicked()
                && let Some(out_dir) = select_pack_export_dir()
                && let Some(command) = pack::batch_export_command(&self.state.pack, out_dir)
            {
                commands.push(command);
            }
        });
        if !view.active {
            ui.weak("No family pack workspace is open.");
            return commands;
        }
        ui.label(format!("{} member(s)", view.members.len()));
        if view.export.enabled {
            ui.weak("Batch export ready");
        } else if let Some(reason) = &view.export.disabled_reason {
            ui.weak(reason);
        }
        commands
    }

    fn show_current_preview(&mut self, ui: &mut egui::Ui) {
        let preview = self.state.current_preview.clone();
        let has_output = self.state.current_output.is_some();
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        ui.horizontal(|ui| {
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
                        max_edge: 180.0,
                    },
                );
                ui.weak(format!("Preview {}x{}", preview.width, preview.height));
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
            if self.state.handle_job_event(event) {
                affected = true;
                schedule_preview |= should_preview;
            }
        }

        if schedule_preview {
            match self.state.request_preview(128, 128) {
                Ok(effects) => self.apply_effects(effects, ctx),
                Err(error) => self.state.status = Some(error.to_string()),
            }
        }
        if affected {
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

fn product_visible_strings_for_default_shell() -> Vec<&'static str> {
    let mut strings = vec![
        "Shape Lab",
        "Visual Foundry",
        "Choose",
        "Directions",
        "Customize",
        "Pack",
        "Export",
        "Choose what to make",
        "Open Project",
        "Save Project",
        "Save Project As",
        "New Project",
        "Project History",
        "Recent Projects",
        "No recent projects yet.",
        "Open a project to keep working here.",
        "Start",
        "Generate Directions",
        "Build Asset",
        "Refresh Preview",
        "Save",
        "Undo",
        "No project",
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
        "Pack: 0 assets",
        "Export complete",
        "Pack export complete",
        HOME_SUBTITLE,
        "Pick a template to generate whole-model directions.",
        "Explore directions",
        "Generate coherent whole-model options from the current asset.",
        NEED_PROJECT_REASON,
        NEED_SAVE_LOCATION_REASON,
        NEED_MODEL_REASON,
        NEED_HISTORY_REASON,
        NEED_DIRECTION_REASON,
        NEED_RESET_REASON,
        NEED_PACK_MEMBER_REASON,
        "This model has no quick controls yet.",
        "Build the current asset before exporting.",
        "Batch export ready",
        "No family pack workspace is open.",
        "Preview is ready to refresh.",
        "Whole-model options",
        "Primary control",
    ];
    for step in WORKFLOW_STEPS {
        strings.push(step.label);
        strings.push(step.detail);
    }
    for (label, fixture) in built_in_fixture_catalogs_with_labels() {
        strings.push(label);
        strings.push(profile_description(&fixture.slug));
    }
    strings
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

        let color_image =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba8);
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
        ui.allocate_space(egui::vec2(preview.max_edge, preview.max_edge));
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
    let scale =
        (preview.max_edge / preview.width as f32).min(preview.max_edge / preview.height as f32);
    let size = egui::vec2(preview.width as f32 * scale, preview.height as f32 * scale);
    ui.image((texture.id(), size));
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
    fn product_home_lists_ten_profiles() {
        let labels = built_in_fixture_catalogs_with_labels()
            .into_iter()
            .map(|(label, _)| label)
            .collect::<Vec<_>>();

        assert_eq!(labels.len(), 10);
        assert!(labels.contains(&"Roman Timber Bridge"));
        assert!(labels.contains(&"Sci-Fi Industrial Crate"));
        assert!(labels.contains(&"Stylized Furniture Lamp"));
        assert!(labels.contains(&"Market Stall Kit"));
        assert!(labels.contains(&"Sci-Fi Door Panel"));
        assert!(labels.contains(&"Coopered Storage Barrel"));
        assert!(labels.contains(&"Wayfinding Signpost"));
        assert!(labels.contains(&"Workshop Chair"));
        assert!(labels.contains(&"Market Handcart"));
        assert!(labels.contains(&"Storybook Tree"));
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
            "Open Project",
            "Save Project",
            "Save Project As",
            "Recent Projects",
            "Generate Directions",
            "Start",
        ] {
            assert!(
                strings.contains(&required),
                "missing product string {required}"
            );
        }
    }

    #[test]
    fn launch_save_state_does_not_claim_project_is_saved() {
        let mut app = FoundryDesktopApp::default();
        assert_eq!(app.save_state_pill(), ("No project", StatusTone::Neutral));

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

        let controls = default_customize_controls(&app.state.controls).collect::<Vec<_>>();
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

        for _ in 0..200 {
            app.poll_jobs(&ctx);
            if !app.state.controls.is_empty() {
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
