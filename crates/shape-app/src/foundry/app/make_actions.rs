use super::*;

pub(super) fn make_whole_asset_candidate_request(
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

impl FoundryDesktopApp {
    pub(super) fn make_primary_candidate_command(&self) -> Option<FoundryAppCommand> {
        if self.active_make_profile_kind().direct_primitive_workflow() {
            return None;
        }
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

    pub(super) fn make_whole_asset_candidate_command(&self) -> Option<FoundryAppCommand> {
        self.state.document.as_ref().map(|_| {
            make_whole_asset_candidate_request(&self.state, false, FoundryCandidateMode::Explore)
        })
    }

    pub(super) fn make_focused_recovery_commands(&self) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if self.active_make_part_group().is_some() {
            commands.push(directions::clear_focus_part_group_command());
        }
        if let Some(command) = self.make_whole_asset_candidate_command() {
            commands.push(command);
        }
        commands
    }

    pub(super) fn visible_review_candidate(
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

    pub(super) fn accept_visible_candidate_command(&self) -> Option<FoundryAppCommand> {
        if self.active_make_profile_kind().direct_primitive_workflow() {
            return None;
        }
        let candidate = self.visible_review_candidate()?;
        candidate
            .selectable
            .then(|| directions::accept_candidate_command(candidate.id.clone()))
    }

    pub(super) fn push_make_primary_action_commands(
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

    pub(super) fn show_direction_board_panel(
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
                view_state.simple_box_make_baseline,
                view_state.lidded_box_baseline,
                view_state.flat_panel_baseline,
                view_state.hinged_panel_baseline,
                view_state.handled_panel_baseline,
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
                let (title, message) = empty_candidate_tray_copy(view_state);
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

    pub(super) fn show_no_candidates_recovery_card(
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

    pub(super) fn show_candidate_error_recovery_card(
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
}
