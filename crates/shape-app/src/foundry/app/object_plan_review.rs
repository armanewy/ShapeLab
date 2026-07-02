use super::*;

impl FoundryDesktopApp {
    pub(super) fn object_plan_review_ui_state(&self) -> ObjectPlanReviewUiState {
        let drawer_visible =
            self.object_plan_review_enabled && self.drawer == Some(FoundryDrawer::ObjectPlanReview);
        ObjectPlanReviewUiState {
            entry_visible: self.object_plan_review_enabled,
            drawer_visible,
            batch_report_visible: drawer_visible,
            contact_sheet_visible: drawer_visible,
            review_labels: if drawer_visible {
                vec!["Keep", "Regenerate", "Simplify", "Blocked"]
            } else {
                Vec::new()
            },
            safety_labels: if drawer_visible {
                vec![
                    "Draft only",
                    "Not catalog published",
                    "Human review required",
                ]
            } else {
                Vec::new()
            },
            publish_action_visible: false,
            catalog_mutation_allowed: false,
            runtime_llm_action_visible: false,
        }
    }

    pub(super) fn show_object_plan_review_drawer(&self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        let state = self.object_plan_review_ui_state();
        product_card(ui, true, |ui| {
            ui.label(RichText::new("Draft only").color(colors.warning).strong());
            ui.label(RichText::new("Not catalog published").color(colors.text));
            ui.label(RichText::new("Human review required").color(colors.text));
        });
        ui.add_space(12.0);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Internal review",
                title: "Batch report",
                subtitle: Some("Fixed internal review target."),
            },
        );
        product_card(ui, true, |ui| {
            ui.label(RichText::new("target/object-plan-batches/basic-batch").monospace());
            ui.label(RichText::new("Batch report loaded").color(colors.success));
            ui.label(RichText::new("Contact sheet").strong());
            ui.label(RichText::new(
                "Rendered evidence appears here when available.",
            ));
        });
        ui.add_space(12.0);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Review",
                title: "Review labels",
                subtitle: Some("Labels do not publish."),
            },
        );
        ui.horizontal_wrapped(|ui| {
            for label in &state.review_labels {
                let _ = status_pill(ui, StatusPillSpec::new(label, StatusTone::Neutral));
            }
        });
        ui.add_space(10.0);
        ui.label(RichText::new("No catalog action in this gate.").color(colors.text_muted));
    }
}
