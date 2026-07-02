use super::*;

impl FoundryDesktopApp {
    pub(super) fn family_studio_lite_ui_state(&self) -> FamilyStudioLiteUiState {
        let source = self.family_studio_lite_source();
        let supported = source.supported();
        let drawer_visible =
            self.family_studio_lite_enabled && self.drawer == Some(FoundryDrawer::FamilyStudioLite);
        let draft = supported.then(|| self.family_studio_lite_draft(DirectKitVisibility::Draft));
        let summary = draft.as_ref().map(direct_kit_user_summary);
        FamilyStudioLiteUiState {
            entry_visible: self.family_studio_lite_enabled,
            drawer_visible,
            starting_point_title: self.current_project_title(),
            starting_point_summary: source.identity_summary().to_owned(),
            source_label: source.source_label(),
            supported,
            disabled_reason: (!supported)
                .then(|| "This starting point cannot be saved as a reusable kit yet.".to_owned()),
            stays_same: summary
                .as_ref()
                .map(|summary| summary.stays_fixed.clone())
                .unwrap_or_else(|| vec![source.identity_summary().to_owned()]),
            capability_cards: self.family_studio_lite_capability_card_views(source),
            test_result: self.family_studio_lite.test_result.clone(),
            saved_visibility: self.family_studio_lite.saved_visibility,
            save_error: self.family_studio_lite.save_error.clone(),
            draft_save_enabled: supported,
            personal_save_enabled: supported,
            approved: false,
            publish_allowed: false,
            runtime_llm_action_visible: false,
            generated_variation_copy_visible: false,
        }
    }

    pub(super) fn family_studio_lite_source(&self) -> FamilyStudioLiteSource {
        match self.active_make_profile_kind() {
            MakeProfileKind::BoxPrimitive => {
                FamilyStudioLiteSource::Primitive(PrimitiveKind::BoxPrimitive)
            }
            MakeProfileKind::FlatPanelPrimitive => {
                FamilyStudioLiteSource::Primitive(PrimitiveKind::FlatPanelPrimitive)
            }
            MakeProfileKind::SpherePrimitive => {
                FamilyStudioLiteSource::Primitive(PrimitiveKind::SpherePrimitive)
            }
            MakeProfileKind::PanelWithKnob => FamilyStudioLiteSource::PanelWithKnob,
            MakeProfileKind::LiddedBox
            | MakeProfileKind::HingedPanel
            | MakeProfileKind::HandledPanel
            | MakeProfileKind::Other => FamilyStudioLiteSource::Unsupported,
        }
    }

    pub(super) fn family_studio_lite_capability_card_views(
        &self,
        source: FamilyStudioLiteSource,
    ) -> Vec<FamilyStudioLiteCapabilityCardView> {
        let cards = source.capability_cards();
        let selected = self.family_studio_lite_selected_capability_ids(source);
        cards
            .into_iter()
            .filter(|card| card.source_kind != KitCapabilitySourceKind::SurfaceLook)
            .map(|card| FamilyStudioLiteCapabilityCardView {
                selected: selected.contains(&card.capability_id),
                status_label: match card.availability {
                    KitCapabilityAvailability::Included => "Included",
                    KitCapabilityAvailability::Available => "Available",
                    KitCapabilityAvailability::Recommended => "Recommended",
                    KitCapabilityAvailability::Later => "Later",
                    KitCapabilityAvailability::Blocked => "Blocked",
                },
                capability_id: card.capability_id,
                display_name: card.display_name,
                description: card.description,
                visible_test_required: card.visible_test_required,
            })
            .collect()
    }

    pub(super) fn family_studio_lite_selected_capability_ids(
        &self,
        source: FamilyStudioLiteSource,
    ) -> BTreeSet<String> {
        let visible_ids = source
            .capability_cards()
            .into_iter()
            .filter(|card| card.source_kind != KitCapabilitySourceKind::SurfaceLook)
            .map(|card| card.capability_id)
            .collect::<BTreeSet<_>>();
        let selected = self
            .family_studio_lite
            .selected_capability_ids
            .intersection(&visible_ids)
            .cloned()
            .collect::<BTreeSet<_>>();
        if !selected.is_empty() {
            return selected;
        }

        source
            .capability_cards()
            .into_iter()
            .filter(|card| {
                card.source_kind == KitCapabilitySourceKind::PrimitiveProperty
                    && matches!(
                        card.availability,
                        KitCapabilityAvailability::Available
                            | KitCapabilityAvailability::Included
                            | KitCapabilityAvailability::Recommended
                    )
            })
            .take(1)
            .map(|card| card.capability_id)
            .collect()
    }

    pub(super) fn family_studio_lite_draft(
        &self,
        visibility: DirectKitVisibility,
    ) -> DirectKitDraft {
        let source = self.family_studio_lite_source();
        let selected = self.family_studio_lite_selected_capability_ids(source);
        let mut exposures = source.property_exposures();
        let mut changeable_properties = Vec::new();
        let mut locked_properties = Vec::new();
        for exposure in exposures.drain(..) {
            if selected
                .iter()
                .any(|id| id.ends_with(&exposure.property_id))
            {
                changeable_properties.push(exposure);
            } else {
                locked_properties.push(exposure);
            }
        }
        if changeable_properties.is_empty()
            && let Some(first) = locked_properties.first().cloned()
        {
            changeable_properties.push(first.clone());
            locked_properties.retain(|property| property.property_id != first.property_id);
        }

        DirectKitDraft {
            kit_id: format!("{}_reusable_kit", source.source_ref()),
            display_name: format!("{} Reusable Kit", self.current_project_title()),
            source_kind: source.direct_kit_source_kind(),
            source_ref: source.source_ref().to_owned(),
            identity_summary: source.identity_summary().to_owned(),
            changeable_properties,
            locked_properties,
            included_presets: source.preset_refs(),
            evidence_refs: Vec::new(),
            review_tier: if visibility == DirectKitVisibility::PersonalOnly {
                ObjectPlanReviewTier::Personal
            } else {
                ObjectPlanReviewTier::Draft
            },
            visibility,
            created_from: source.created_from(),
        }
    }

    pub(super) fn run_family_studio_lite_test(&mut self) {
        let draft = self.family_studio_lite_draft(DirectKitVisibility::Draft);
        let validation = validate_direct_kit_draft(&draft);
        let status = if !validation.errors.is_empty() {
            FamilyStudioLiteTestStatus::Failed
        } else if !validation.warnings.is_empty() {
            FamilyStudioLiteTestStatus::Warnings
        } else {
            FamilyStudioLiteTestStatus::Passed
        };
        let message = match status {
            FamilyStudioLiteTestStatus::Passed => "Value controls checked.".to_owned(),
            FamilyStudioLiteTestStatus::Warnings => {
                "Value controls checked. Add review images before sharing.".to_owned()
            }
            FamilyStudioLiteTestStatus::Failed => {
                "Fix the highlighted choices before saving.".to_owned()
            }
        };
        self.family_studio_lite.test_result = Some(FamilyStudioLiteTestResult {
            status,
            message,
            tested_capabilities: draft.changeable_properties.len(),
            human_review_required: true,
            approved: false,
            publish_allowed: false,
        });
    }

    pub(super) fn save_family_studio_lite_kit(&mut self, visibility: DirectKitVisibility) {
        self.save_family_studio_lite_kit_to(visibility, &family_studio_lite_store_base_dir());
    }

    pub(super) fn save_family_studio_lite_kit_to(
        &mut self,
        visibility: DirectKitVisibility,
        base_dir: &Path,
    ) {
        let draft = self.family_studio_lite_draft(visibility);
        match save_direct_kit(base_dir, &draft) {
            Ok(_) => {
                self.family_studio_lite.saved_visibility = Some(visibility);
                self.family_studio_lite.save_error = None;
            }
            Err(error) => {
                self.family_studio_lite.save_error = Some(format!("Could not save kit: {error}"));
            }
        }
    }

    pub(super) fn show_family_studio_lite_drawer(&mut self, ui: &mut egui::Ui) {
        let colors = VisualFoundryTokens::dark().colors;
        let state = self.family_studio_lite_ui_state();
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Reusable kit",
                title: "Create reusable kit",
                subtitle: Some("Choose what stays adjustable for your own reuse."),
            },
        );
        product_card(ui, true, |ui| {
            ui.label(RichText::new(&state.starting_point_title).strong());
            ui.label(RichText::new("Start from current primitive").color(colors.text));
            ui.label(RichText::new(state.source_label).color(colors.text_muted));
            ui.label(RichText::new(&state.starting_point_summary).color(colors.text));
            if let Some(reason) = &state.disabled_reason {
                ui.label(RichText::new(reason).color(colors.warning));
            }
        });

        ui.add_space(12.0);
        if let Some(result) = &state.test_result {
            product_card(ui, true, |ui| {
                let tone = match result.status {
                    FamilyStudioLiteTestStatus::Passed => colors.success,
                    FamilyStudioLiteTestStatus::Warnings => colors.warning,
                    FamilyStudioLiteTestStatus::Failed => colors.danger,
                };
                ui.label(RichText::new("Test result").strong().color(tone));
                ui.label(RichText::new(&result.message).color(colors.text));
                ui.label(
                    RichText::new(format!(
                        "{} control(s) checked.",
                        result.tested_capabilities
                    ))
                    .color(colors.text_muted),
                );
                ui.label(RichText::new("Review required.").color(colors.text_muted));
            });
            ui.add_space(12.0);
        }
        if let Some(visibility) = state.saved_visibility {
            product_card(ui, true, |ui| {
                let label = match visibility {
                    DirectKitVisibility::Draft => "Draft saved.",
                    DirectKitVisibility::PersonalOnly => "Saved for personal use.",
                    DirectKitVisibility::Reviewed
                    | DirectKitVisibility::Showcase
                    | DirectKitVisibility::PublicCatalog => "Save needs review.",
                };
                ui.label(RichText::new(label).strong().color(colors.success));
                ui.label(RichText::new("Only visible to you.").color(colors.text));
                ui.label(RichText::new("Needs review before sharing.").color(colors.text_muted));
            });
            ui.add_space(12.0);
        } else if let Some(error) = &state.save_error {
            ui.label(RichText::new(error).color(colors.warning));
            ui.add_space(12.0);
        }

        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Shape identity",
                title: "What stays the same",
                subtitle: None,
            },
        );
        product_card(ui, true, |ui| {
            for line in &state.stays_same {
                ui.label(RichText::new(line).color(colors.text));
            }
            ui.label(RichText::new("Needs review before sharing.").color(colors.text_muted));
        });

        ui.add_space(12.0);
        section_header(
            ui,
            SectionHeaderSpec {
                eyebrow: "Controls",
                title: "What can change",
                subtitle: Some("Toggle the controls this kit should keep adjustable."),
            },
        );
        product_card(ui, true, |ui| {
            for card in &state.capability_cards {
                let mut selected = card.selected;
                ui.horizontal_wrapped(|ui| {
                    if ui.checkbox(&mut selected, "").changed() {
                        if selected {
                            self.family_studio_lite
                                .selected_capability_ids
                                .insert(card.capability_id.clone());
                        } else {
                            self.family_studio_lite
                                .selected_capability_ids
                                .remove(&card.capability_id);
                        }
                        self.family_studio_lite.test_result = None;
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&card.display_name).strong());
                        ui.label(RichText::new(&card.description).color(colors.text_muted));
                        ui.label(RichText::new(card.status_label).color(colors.text_subtle));
                        if card.visible_test_required {
                            ui.label(RichText::new("Test required").color(colors.text_subtle));
                        }
                    });
                });
                ui.add_space(8.0);
            }
        });

        ui.add_space(12.0);
        let disabled_reason = state
            .disabled_reason
            .as_deref()
            .unwrap_or("Choose a supported shape first.");
        ui.horizontal_wrapped(|ui| {
            if action_button(
                ui,
                &action_spec(
                    state.supported,
                    ACTION_TEST_KIT,
                    ButtonTone::Secondary,
                    disabled_reason,
                ),
            )
            .clicked()
            {
                self.run_family_studio_lite_test();
            }
            if action_button(
                ui,
                &action_spec(
                    state.draft_save_enabled,
                    ACTION_SAVE_DRAFT_KIT,
                    ButtonTone::Secondary,
                    disabled_reason,
                ),
            )
            .clicked()
            {
                self.save_family_studio_lite_kit(DirectKitVisibility::Draft);
            }
            if action_button(
                ui,
                &action_spec(
                    state.personal_save_enabled,
                    ACTION_USE_PERSONALLY,
                    ButtonTone::Primary,
                    disabled_reason,
                ),
            )
            .clicked()
            {
                self.save_family_studio_lite_kit(DirectKitVisibility::PersonalOnly);
            }
        });
    }
}
