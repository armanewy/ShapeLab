use super::*;

pub(super) fn show_customize_control_card(
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
        let action_state = CustomizeActionState {
            enabled: actions_enabled,
            disabled_reason,
        };
        if let Some(range) = control.numeric_range {
            commands.extend(show_direct_numeric_control(
                ui,
                control,
                range,
                action_state,
            ));
        } else {
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
        }
    });
    commands
}

pub(super) fn direct_exact_value_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> Vec<crate::foundry::view_model::FoundryControlView> {
    display_customize_controls(controls)
        .into_iter()
        .cloned()
        .collect()
}

pub(super) fn show_direct_exact_value_control_row(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    current_build: Option<&FoundryBuildStamp>,
    control: &crate::foundry::view_model::FoundryControlView,
    actions_enabled: bool,
    disabled_reason: &str,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
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
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                show_customize_control_header_actions(
                    ui,
                    control,
                    actions_enabled,
                    disabled_reason,
                    false,
                    &mut commands,
                );
            });
        });
    } else {
        ui.horizontal(|ui| {
            let actions_width = CONTROL_HEADER_ACTIONS_WIDTH.min(header_width * 0.5);
            let text_width = (header_width - actions_width - 12.0).max(140.0);
            ui.allocate_ui_with_layout(
                egui::vec2(text_width, 34.0),
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
                egui::vec2(actions_width, 34.0),
                egui::Layout::right_to_left(egui::Align::Center),
                |ui| {
                    show_customize_control_header_actions(
                        ui,
                        control,
                        actions_enabled,
                        disabled_reason,
                        false,
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
    ui.add_space(4.0);
    let action_state = CustomizeActionState {
        enabled: actions_enabled,
        disabled_reason,
    };
    if let Some(range) = control.numeric_range {
        commands.extend(show_compact_direct_numeric_control(
            ui,
            control,
            range,
            action_state,
        ));
    } else {
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
            action_state,
        ));
    }
    commands
}

pub(super) fn show_compact_direct_numeric_control(
    ui: &mut egui::Ui,
    control: &crate::foundry::view_model::FoundryControlView,
    range: crate::foundry::view_model::FoundryNumericRange,
    action_state: CustomizeActionState<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let colors = VisualFoundryTokens::dark().colors;
    let current = direct_numeric_value(control, range);
    let step = range.step.max(0.01);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Value").color(colors.text_muted).small());
        ui.monospace(format!("{current:.2}"));
    });
    let mut adjusted = current;
    let response = ui
        .add_enabled_ui(action_state.enabled, |ui| {
            ui.add_sized(
                [ui.available_width(), 18.0],
                egui::Slider::new(&mut adjusted, range.minimum..=range.maximum)
                    .step_by(f64::from(step))
                    .show_value(false),
            )
        })
        .inner;
    if !action_state.enabled {
        response.on_disabled_hover_text(action_state.disabled_reason);
    } else if response.changed() {
        commands.extend(customize::release_control_value_intents(
            control,
            shape_foundry::ControlValue::Scalar(snap_direct_numeric_value(adjusted, range, step)),
        ));
    }
    if !action_state.enabled {
        ui.label(
            RichText::new(product_panel_message(
                action_state.disabled_reason,
                PREVIEW_UPDATING_REASON,
            ))
            .color(colors.text_muted)
            .small(),
        );
    }
    commands
}

pub(super) fn show_direct_numeric_control(
    ui: &mut egui::Ui,
    control: &crate::foundry::view_model::FoundryControlView,
    range: crate::foundry::view_model::FoundryNumericRange,
    action_state: CustomizeActionState<'_>,
) -> Vec<FoundryAppCommand> {
    let mut commands = Vec::new();
    let colors = VisualFoundryTokens::dark().colors;
    let current = direct_numeric_value(control, range);
    let step = range.step.max(0.01);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Value").color(colors.text_muted).small());
        ui.monospace(format!("{current:.2}"));
    });
    ui.add_space(4.0);
    let mut adjusted = current;
    let response = ui
        .add_enabled_ui(action_state.enabled, |ui| {
            ui.add_sized(
                [ui.available_width(), 22.0],
                egui::Slider::new(&mut adjusted, range.minimum..=range.maximum)
                    .step_by(f64::from(step))
                    .show_value(false),
            )
        })
        .inner;
    if !action_state.enabled {
        response.on_disabled_hover_text(action_state.disabled_reason);
    } else if response.changed() {
        commands.extend(customize::release_control_value_intents(
            control,
            shape_foundry::ControlValue::Scalar(snap_direct_numeric_value(adjusted, range, step)),
        ));
    }
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{:.2}", range.minimum))
                .color(colors.text_muted)
                .small(),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(format!("{:.2}", range.maximum))
                    .color(colors.text_muted)
                    .small(),
            );
        });
    });
    if !action_state.enabled {
        ui.label(
            RichText::new(product_panel_message(
                action_state.disabled_reason,
                PREVIEW_UPDATING_REASON,
            ))
            .color(colors.text_muted)
            .small(),
        );
    }
    commands
}

pub(super) fn snap_direct_numeric_value(
    value: f32,
    range: crate::foundry::view_model::FoundryNumericRange,
    step: f32,
) -> f32 {
    let step = step.max(0.01);
    let steps = ((value - range.minimum) / step).round();
    (range.minimum + steps * step).clamp(range.minimum, range.maximum)
}

pub(super) fn direct_numeric_value(
    control: &crate::foundry::view_model::FoundryControlView,
    range: crate::foundry::view_model::FoundryNumericRange,
) -> f32 {
    let value = match control.value.as_ref().or(control.default_value.as_ref()) {
        Some(shape_foundry::ControlValue::Scalar(value)) if value.is_finite() => *value,
        _ => range.minimum,
    };
    value.clamp(range.minimum, range.maximum)
}

pub(super) fn show_customize_control_header_actions(
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
pub(super) struct CustomizeActionState<'a> {
    enabled: bool,
    disabled_reason: &'a str,
}

pub(super) fn show_customize_option_grid(
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

pub(super) fn customize_option_grid_columns(width: f32) -> usize {
    if width >= 1160.0 {
        3
    } else if width >= 720.0 {
        2
    } else {
        1
    }
}

pub(super) fn show_customize_option_tile(
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

pub(super) fn display_customize_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> Vec<&crate::foundry::view_model::FoundryControlView> {
    let mut visible = default_customize_controls(controls)
        .take(CUSTOMIZE_PRIMARY_CONTROL_LIMIT)
        .collect::<Vec<_>>();
    visible.extend(
        controls
            .iter()
            .filter(|control| control.visible && !control.primary),
    );
    visible
}

#[derive(Debug, Clone)]
pub(super) struct MakeInspectorControlSections {
    pub(super) visible: Vec<crate::foundry::view_model::FoundryControlView>,
    pub(super) overflow: Vec<crate::foundry::view_model::FoundryControlView>,
    pub(super) disclosure_label: &'static str,
    pub(super) empty_title: &'static str,
    pub(super) empty_message: &'static str,
}

pub(super) fn make_context_inspector_controls(
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

pub(super) fn make_control_matches_focus(
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

pub(super) fn default_customize_controls(
    controls: &[crate::foundry::view_model::FoundryControlView],
) -> impl Iterator<Item = &crate::foundry::view_model::FoundryControlView> {
    controls
        .iter()
        .filter(|control| control.primary && control.visible)
}

impl FoundryDesktopApp {
    pub(super) fn clear_all_locks_command(&self) -> Option<FoundryAppCommand> {
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

    pub(super) fn show_customize(&mut self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
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
                "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before adjusting.",
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

    pub(super) fn show_customize_preview_panel(&mut self, ui: &mut egui::Ui, preview_edge: f32) {
        product_stage(ui, |ui| {
            let colors = VisualFoundryTokens::dark().colors;
            ui.label(
                RichText::new("Whole-model preview")
                    .color(colors.accent_hover)
                    .small()
                    .strong(),
            );
            let _ = self.show_current_preview_sized(ui, preview_edge);
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

    pub(super) fn show_customize_control_deck(
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
}
