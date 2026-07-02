use super::*;

pub(super) struct VisibleDirectionIdeasBoard<'a> {
    pub(super) current_build: Option<&'a FoundryBuildStamp>,
    pub(super) current_preview: Option<&'a FoundryPreviewImage>,
    pub(super) candidates: &'a [crate::foundry::view_model::FoundryCandidateCard],
    pub(super) actions_enabled: bool,
    pub(super) disabled_reason: &'a str,
    pub(super) use_candidate_label: &'a str,
}

pub(super) fn show_visible_direction_ideas_board(
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

pub(super) fn show_selected_candidate_comparison_compact(
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

pub(super) fn show_selected_candidate_comparison(
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

pub(super) fn show_direction_candidate_grid(
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

pub(super) fn show_direction_candidate_grid_compact(
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

pub(super) fn direction_grid_columns(width: f32) -> usize {
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

pub(super) fn compact_direction_grid_columns(width: f32) -> usize {
    if width >= 560.0 {
        3
    } else if width >= 380.0 {
        2
    } else {
        1
    }
}

pub(super) fn compact_comparison_preview_edge(width: f32) -> f32 {
    (width * 0.16).clamp(84.0, 128.0)
}

pub(super) fn compact_direction_card_preview_edge(width: f32) -> f32 {
    (width * 0.22).clamp(46.0, 64.0)
}

pub(super) fn show_direction_skeleton_grid(ui: &mut egui::Ui) {
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

pub(super) fn show_direction_candidate_card_compact(
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

pub(super) fn show_direction_candidate_card(
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

pub(super) fn show_direction_candidate_details(
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

pub(super) fn candidate_display_title(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> String {
    let title = candidate.title.trim();
    if !candidate_title_looks_raw(title) && !candidate_title_looks_trait_derived(title) {
        let safe_title = product_panel_message(title, "");
        if !safe_title.trim().is_empty() {
            return safe_title;
        }
    }

    DIRECTION_INTENT_TITLES[candidate.slot % DIRECTION_INTENT_TITLES.len()].to_owned()
}

pub(super) fn candidate_display_subtitle(
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

pub(super) fn candidate_display_detail(
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

pub(super) const DIRECTION_INTENT_TITLES: [&str; 6] = [
    "Compact Box",
    "Wide Box",
    "Tall Box",
    "Flat Box",
    "Soft-Edged Box",
    "Sharp Box",
];

pub(super) fn candidate_title_looks_raw(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    title.contains('#')
        || title.contains('(')
        || title.contains(')')
        || lower.contains("candidate")
        || lower.contains("preview_id")
}

pub(super) fn candidate_title_looks_trait_derived(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    lower.ends_with(" direction")
        || lower.contains("edge softness")
        || lower.contains("proportions")
}

pub(super) fn candidate_change_phrase(raw: &str) -> Option<String> {
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

pub(super) fn product_title_fragment(raw: &str) -> Option<String> {
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

pub(super) fn looks_like_generated_token(word: &str) -> bool {
    word.len() >= 8 && word.chars().all(|character| character.is_ascii_hexdigit())
}

pub(super) fn title_case_word(word: &str) -> String {
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
