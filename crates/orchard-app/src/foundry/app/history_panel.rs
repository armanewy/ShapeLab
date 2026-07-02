use super::*;

impl FoundryDesktopApp {
    pub(super) fn show_history(&self, ui: &mut egui::Ui) -> Vec<FoundryAppCommand> {
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

    pub(super) fn history_dispatch_command(
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
}
