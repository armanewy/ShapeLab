use super::*;

impl FoundryDesktopApp {
    pub(super) fn show_make(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let view_state = self.make_canvas_view_state();
        if view_state.mode == MakeCanvasMode::NoAsset {
            commands.extend(self.show_choose_asset_empty_state(
                ui,
                "Choose an asset first",
                "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before making changes.",
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

    pub(super) fn show_make_model_stage_panel(
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
            commands.extend(self.show_current_preview_sized(ui, preview_edge));
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
            } else if view_state.direct_primitive_workflow {
                commands.extend(show_make_view_controls(
                    ui,
                    self.state.current_preview.as_ref(),
                    &mut self.current_preview_orbit,
                ));
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

    pub(super) fn show_make_primary_stage_action(
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

    pub(super) fn show_make_focus_action_tray(
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

    pub(super) fn show_make_inspector_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;
        product_card(ui, false, |ui| {
            ui.set_min_height(ui.available_height().max(240.0));
            if view_state.direct_primitive_workflow {
                commands.extend(self.show_direct_make_exact_value_panel(ui, view_state));
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("make_inspector_panel_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
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
                                if view_state.candidate_ui_enabled
                                    && view_state.mode == MakeCanvasMode::ReviewingIdeas
                                    && action_button(
                                        ui,
                                        &action_spec(
                                            make_canvas_candidate_actions_enabled(view_state),
                                            try_ideas_action_label(self.active_make_profile_kind()),
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
                                        &ActionSpec::enabled(
                                            ACTION_TRY_MATERIAL_LOOKS,
                                            ButtonTone::Secondary,
                                        ),
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
                                        if view_state.panel_knob_baseline {
                                            ACTION_EXPORT_CURRENT_ASSET
                                        } else if view_state.direct_primitive_workflow {
                                            ACTION_EXPORT_CURRENT_PRIMITIVE
                                        } else {
                                            ACTION_OPEN_EXPORT
                                        },
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
                                    &ActionSpec::enabled(
                                        ACTION_KEEP_WAITING,
                                        ButtonTone::Secondary,
                                    ),
                                )
                                .clicked()
                                {
                                    self.make_generation_started_at = Some(Instant::now());
                                }
                            }
                            if view_state.preparation_fallback_visible {
                                if action_button(
                                    ui,
                                    &ActionSpec::enabled(
                                        ACTION_RETRY_PREPARATION,
                                        ButtonTone::Primary,
                                    ),
                                )
                                .clicked()
                                {
                                    commands.push(FoundryAppCommand::RetryPreparation);
                                }
                                if action_button(
                                    ui,
                                    &ActionSpec::enabled(
                                        ACTION_CHOOSE_ANOTHER_TEMPLATE,
                                        ButtonTone::Secondary,
                                    ),
                                )
                                .clicked()
                                {
                                    self.tab = FoundryTab::Home;
                                }
                                if action_button(
                                    ui,
                                    &ActionSpec::enabled(
                                        ACTION_OPEN_PROJECT,
                                        ButtonTone::Secondary,
                                    ),
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
                                    &ActionSpec::enabled(
                                        ACTION_UPDATE_PREVIEW,
                                        ButtonTone::Primary,
                                    ),
                                )
                                .clicked()
                            {
                                let preview_pixels = current_preview_pixels_for_context(ui.ctx());
                                commands.push(FoundryAppCommand::RequestPreview {
                                    width: preview_pixels,
                                    height: preview_pixels,
                                    camera: None,
                                });
                            }
                        });
                        ui.add_space(12.0);
                        ui.label(
                            RichText::new(view_state.property_panel_title)
                                .color(colors.text)
                                .strong(),
                        );
                        if view_state.direct_primitive_workflow
                            && self.active_make_profile_kind().is_sphere_primitive()
                        {
                            ui.add_space(6.0);
                            let preset_enabled = make_canvas_controls_enabled(view_state);
                            let disabled_reason = view_state
                                .primary_action_disabled_reason
                                .as_deref()
                                .unwrap_or(ACTIVE_IDEA_JOB_REASON);
                            if action_button(
                                ui,
                                &action_spec(
                                    preset_enabled,
                                    ACTION_KNOB_LIKE_FORM,
                                    ButtonTone::Secondary,
                                    disabled_reason,
                                ),
                            )
                            .clicked()
                            {
                                commands.push(sphere_knob_like_form_preset_command());
                            }
                        }
                        ui.add_space(6.0);
                        let active_group = self.active_make_part_group();
                        let control_sections = make_context_inspector_controls(
                            &self.state.controls,
                            active_group.as_ref(),
                        );
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
                                    !view_state.simple_box_make_baseline,
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
                                                !view_state.simple_box_make_baseline,
                                            ));
                                            ui.add_space(8.0);
                                        }
                                    });
                            }
                        }
                    });
            }
        });
        commands
    }

    pub(super) fn show_direct_make_exact_value_panel(
        &mut self,
        ui: &mut egui::Ui,
        view_state: &MakeCanvasViewState,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let colors = VisualFoundryTokens::dark().colors;

        ui.set_width(ui.available_width());
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
        ui.add_space(10.0);

        if make_canvas_inspector_build_actions_visible(view_state) {
            let build_actions_enabled = make_canvas_build_dependent_actions_enabled(view_state);
            let build_actions_reason = make_canvas_build_dependent_disabled_reason(view_state);
            ui.horizontal_wrapped(|ui| {
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
                        if view_state.panel_knob_baseline {
                            ACTION_EXPORT_CURRENT_ASSET
                        } else {
                            ACTION_EXPORT_CURRENT_PRIMITIVE
                        },
                        ButtonTone::Primary,
                        build_actions_reason,
                    ),
                )
                .clicked()
                {
                    self.drawer = Some(FoundryDrawer::Export);
                }
            });
            if !build_actions_enabled {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(product_panel_message(
                        build_actions_reason,
                        PREVIEW_UPDATING_REASON,
                    ))
                    .color(colors.text_muted)
                    .small(),
                );
            }
            ui.add_space(10.0);
        }

        if matches!(
            view_state.local_banner_tone,
            BannerTone::Warning | BannerTone::Error
        ) {
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
                    camera: None,
                });
            }
        });
        ui.add_space(8.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(view_state.property_panel_title)
                    .color(colors.text)
                    .strong(),
            );
            if self.active_make_profile_kind().is_sphere_primitive() {
                let preset_enabled = make_canvas_controls_enabled(view_state);
                let disabled_reason = view_state
                    .primary_action_disabled_reason
                    .as_deref()
                    .unwrap_or(ACTIVE_IDEA_JOB_REASON);
                if action_button(
                    ui,
                    &action_spec(
                        preset_enabled,
                        ACTION_KNOB_LIKE_FORM,
                        ButtonTone::Secondary,
                        disabled_reason,
                    ),
                )
                .clicked()
                {
                    commands.push(sphere_knob_like_form_preset_command());
                }
            }
        });
        ui.add(
            egui::Label::new(
                RichText::new("Use exact values while Orchard handles are being built.")
                    .color(colors.text_muted)
                    .small(),
            )
            .wrap(),
        );
        ui.add_space(6.0);

        let controls = direct_exact_value_controls(&self.state.controls);
        if controls.is_empty() {
            product_compact_empty_state(
                ui,
                "No direct controls",
                "This starting point has no editable exact values yet.",
            );
        } else {
            let current_build = self.state.current_build.clone();
            let texture_cache = &mut self.texture_cache;
            let actions_enabled = make_canvas_controls_enabled(view_state);
            let disabled_reason = view_state
                .primary_action_disabled_reason
                .as_deref()
                .unwrap_or(ACTIVE_IDEA_JOB_REASON);
            for row in controls.chunks(2) {
                ui.columns(2, |columns| {
                    for (column, control) in columns.iter_mut().zip(row.iter()) {
                        commands.extend(show_direct_exact_value_control_row(
                            column,
                            texture_cache,
                            current_build.as_ref(),
                            control,
                            actions_enabled,
                            disabled_reason,
                        ));
                    }
                });
                ui.add_space(8.0);
            }
        }

        commands
    }

    pub(super) fn show_choose_asset_empty_state(
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
}
