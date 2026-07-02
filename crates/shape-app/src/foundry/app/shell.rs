use super::*;

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
        let visible_drawer = self.drawer.filter(|drawer| {
            (*drawer != FoundryDrawer::ObjectPlanReview || self.object_plan_review_enabled)
                && (*drawer != FoundryDrawer::FamilyStudioLite || self.family_studio_lite_enabled)
        });
        if let Some(drawer) = visible_drawer {
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
                                    FoundryDrawer::ObjectPlanReview => "ObjectPlan Review",
                                    FoundryDrawer::FamilyStudioLite => "Reusable Kit",
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
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| match drawer {
                                    FoundryDrawer::Pack => {
                                        commands.extend(self.show_pack_drawer(ui));
                                    }
                                    FoundryDrawer::Export => {
                                        commands.extend(self.show_export_drawer(ui));
                                    }
                                    FoundryDrawer::ObjectPlanReview => {
                                        self.show_object_plan_review_drawer(ui);
                                    }
                                    FoundryDrawer::FamilyStudioLite => {
                                        self.show_family_studio_lite_drawer(ui);
                                    }
                                });
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

    pub(super) fn show_app_bar(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
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
                if self.object_plan_review_enabled
                    && action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_REVIEW_OBJECT_PLANS, ButtonTone::Secondary),
                    )
                    .clicked()
                {
                    self.drawer = Some(FoundryDrawer::ObjectPlanReview);
                }
                if self.family_studio_lite_enabled
                    && action_button(
                        ui,
                        &ActionSpec::enabled(ACTION_CREATE_REUSABLE_KIT, ButtonTone::Secondary),
                    )
                    .clicked()
                {
                    self.drawer = Some(FoundryDrawer::FamilyStudioLite);
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

    pub(super) fn show_project_menu(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
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

    pub(super) fn show_workflow_tabs(&mut self, ui: &mut egui::Ui) {
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

    pub(super) fn show_status_strip(&self, ui: &mut egui::Ui) {
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
            if let Some(status) = &self.state.status
                && !self.suppresses_background_result_status(status)
            {
                ui.separator();
                ui.label(RichText::new(product_safe_status(status)).color(colors.text_subtle));
            }
        });
    }

    pub(super) fn current_project_title(&self) -> String {
        if let Some(path) = &self.state.project_path {
            return project_file_title(path);
        }

        if self.state.document.is_none()
            && self.tab == FoundryTab::Home
            && let Some(profile) =
                selected_home_profile(&self.home_profiles, &self.selected_home_profile_slug)
        {
            return format!("Start with {}", profile.label);
        }

        self.state
            .document
            .as_ref()
            .map(|document| asset_title_from_id(&document.document_id.0).to_owned())
            .unwrap_or_else(|| "Start with Box Primitive".to_owned())
    }

    pub(super) fn save_state_pill(&self) -> (&'static str, StatusTone) {
        if self.state.document.is_none() {
            ("Choose starting point", StatusTone::Neutral)
        } else if self.state.project_path.is_none() {
            ("Not saved", StatusTone::Warning)
        } else if self.state.dirty {
            ("Unsaved", StatusTone::Warning)
        } else {
            ("Saved", StatusTone::Ready)
        }
    }

    pub(super) fn status_summary(&self) -> &'static str {
        if self.drawer == Some(FoundryDrawer::FamilyStudioLite)
            && self.family_studio_lite_enabled
            && self.state.current_output.is_some()
            && self.state.current_preview.is_some()
        {
            "Ready"
        } else if !self.state.active_jobs.is_empty() {
            "Working"
        } else if self.state.dirty {
            "Unsaved changes"
        } else {
            "Ready"
        }
    }

    pub(super) fn model_status(&self) -> &'static str {
        if self.state.current_output.is_some() {
            "Model ready"
        } else if self.state.document.is_some() {
            "Preparing model"
        } else {
            "Choose starting point"
        }
    }

    pub(super) fn preview_status(&self) -> &'static str {
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        let preview_image_ready = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| !preview.rgba8.is_empty());
        if rendering_preview && !preview_image_ready {
            "Preview building"
        } else if self.state.current_preview.is_some() {
            "Ready"
        } else if self.state.current_output.is_some() {
            "Ready soon"
        } else {
            "Preparing"
        }
    }
}

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
            current_preview_orbit: CurrentPreviewOrbitState::default(),
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
            object_plan_review_enabled: object_plan_review_enabled(),
            family_studio_lite_enabled: family_studio_lite_enabled(),
            legacy_candidate_ui_enabled: legacy_candidate_ui_enabled(),
            family_studio_lite: FamilyStudioLiteState::default(),
            screenshot_scenario: read_screenshot_scenario(),
            screenshot_scenario_step: 0,
        }
    }
}

impl eframe::App for FoundryDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        FoundryDesktopApp::ui(self, ui, frame);
    }
}
