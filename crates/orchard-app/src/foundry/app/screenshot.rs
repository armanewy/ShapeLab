use super::*;

pub(super) fn read_screenshot_scenario() -> Option<ScreenshotScenario> {
    let path = env::temp_dir().join("shape-lab-screenshot-scenario.txt");
    let value = fs::read_to_string(path).ok()?;
    match value.trim() {
        "choose_grouped_primitives" => Some(ScreenshotScenario::ChooseGroupedPrimitives),
        "choose_box_provenance" => Some(ScreenshotScenario::ChooseBoxProvenance),
        "choose_flat_panel_provenance" => Some(ScreenshotScenario::ChooseFlatPanelProvenance),
        "choose_sphere_preset" => Some(ScreenshotScenario::ChooseSpherePreset),
        "choose_selected_preview" => Some(ScreenshotScenario::ChooseSelectedPreview),
        "box_direct_make_ready"
        | "make_initial_box"
        | "box_stage_no_grid"
        | "box_exact_values_compact"
        | "box_ready_actions" => Some(ScreenshotScenario::BoxDirectMakeReady),
        "box_property_edit" | "adjusted_box_control" => Some(ScreenshotScenario::BoxPropertyEdit),
        "flat_panel_direct_make_ready"
        | "flat_panel_stage_no_grid"
        | "flat_panel_exact_values_compact" => Some(ScreenshotScenario::FlatPanelDirectMakeReady),
        "flat_panel_property_edit" => Some(ScreenshotScenario::FlatPanelPropertyEdit),
        "sphere_direct_make_ready" | "sphere_stage_no_grid" | "sphere_exact_values_compact" => {
            Some(ScreenshotScenario::SphereDirectMakeReady)
        }
        "sphere_property_edit" => Some(ScreenshotScenario::SpherePropertyEdit),
        "sphere_knob_like_preset" => Some(ScreenshotScenario::SphereKnobLikePreset),
        "panel_knob_stage_no_grid" | "panel_knob_exact_values_compact" => {
            Some(ScreenshotScenario::PanelKnobDirectMakeReady)
        }
        "orbit_after_drag_or_tool" => Some(ScreenshotScenario::OrbitAfterDragOrTool),
        "reset_view" => Some(ScreenshotScenario::ResetView),
        "sphere_export_drawer" => Some(ScreenshotScenario::SphereExportDrawer),
        "pack_drawer" => Some(ScreenshotScenario::PackDrawer),
        "export_drawer" => Some(ScreenshotScenario::ExportDrawer),
        "object_plan_review_drawer" => Some(ScreenshotScenario::ObjectPlanReviewDrawer),
        "family_studio_lite_hidden_default" => {
            Some(ScreenshotScenario::FamilyStudioLiteHiddenDefault)
        }
        "family_studio_lite_drawer"
        | "family_studio_lite_starting_point"
        | "family_studio_lite_stays_same"
        | "family_studio_lite_can_change" => Some(ScreenshotScenario::FamilyStudioLiteDrawer),
        "family_studio_lite_test_result" => Some(ScreenshotScenario::FamilyStudioLiteTestResult),
        "family_studio_lite_save_draft" => Some(ScreenshotScenario::FamilyStudioLiteSaveDraft),
        "family_studio_lite_personal_saved" => {
            Some(ScreenshotScenario::FamilyStudioLitePersonalSaved)
        }
        _ => None,
    }
}

pub(super) fn read_screenshot_fixture_catalog(
    scenario: ScreenshotScenario,
) -> orchard_foundry_catalog::FoundryFixtureCatalog {
    match scenario {
        ScreenshotScenario::ChooseGroupedPrimitives
        | ScreenshotScenario::ChooseBoxProvenance
        | ScreenshotScenario::ChooseFlatPanelProvenance
        | ScreenshotScenario::ChooseSpherePreset
        | ScreenshotScenario::ChooseSelectedPreview
        | ScreenshotScenario::BoxDirectMakeReady
        | ScreenshotScenario::BoxPropertyEdit
        | ScreenshotScenario::OrbitAfterDragOrTool
        | ScreenshotScenario::ResetView
        | ScreenshotScenario::PackDrawer
        | ScreenshotScenario::ExportDrawer
        | ScreenshotScenario::ObjectPlanReviewDrawer
        | ScreenshotScenario::FamilyStudioLiteHiddenDefault
        | ScreenshotScenario::FamilyStudioLiteDrawer
        | ScreenshotScenario::FamilyStudioLiteTestResult
        | ScreenshotScenario::FamilyStudioLiteSaveDraft
        | ScreenshotScenario::FamilyStudioLitePersonalSaved => {
            orchard_foundry_catalog::box_primitive::fixture_catalog()
        }
        ScreenshotScenario::FlatPanelDirectMakeReady
        | ScreenshotScenario::FlatPanelPropertyEdit => {
            orchard_foundry_catalog::flat_panel::fixture_catalog()
        }
        ScreenshotScenario::SphereDirectMakeReady
        | ScreenshotScenario::SpherePropertyEdit
        | ScreenshotScenario::SphereKnobLikePreset
        | ScreenshotScenario::SphereExportDrawer => {
            orchard_foundry_catalog::sphere_primitive::fixture_catalog()
        }
        ScreenshotScenario::PanelKnobDirectMakeReady => {
            orchard_foundry_catalog::panel_knob::fixture_catalog()
        }
    }
}

pub(super) fn screenshot_scenario_assertion(
    scenario: ScreenshotScenario,
    view_state: &MakeCanvasViewState,
) -> Result<(), String> {
    match scenario {
        ScreenshotScenario::ChooseGroupedPrimitives
        | ScreenshotScenario::ChooseBoxProvenance
        | ScreenshotScenario::ChooseFlatPanelProvenance
        | ScreenshotScenario::ChooseSpherePreset
        | ScreenshotScenario::ChooseSelectedPreview => {
            require_screenshot_state(view_state.mode == MakeCanvasMode::NoAsset, scenario, "Home")
        }
        ScreenshotScenario::BoxDirectMakeReady
        | ScreenshotScenario::BoxPropertyEdit
        | ScreenshotScenario::FlatPanelDirectMakeReady
        | ScreenshotScenario::FlatPanelPropertyEdit
        | ScreenshotScenario::SphereDirectMakeReady
        | ScreenshotScenario::SpherePropertyEdit
        | ScreenshotScenario::SphereKnobLikePreset
        | ScreenshotScenario::PanelKnobDirectMakeReady
        | ScreenshotScenario::OrbitAfterDragOrTool
        | ScreenshotScenario::ResetView => {
            require_screenshot_state(view_state.mode == MakeCanvasMode::Ready, scenario, "Ready")?;
            require_screenshot_state(view_state.model_ready, scenario, "model_ready")?;
            require_screenshot_state(view_state.preview_ready, scenario, "preview_ready")?;
            require_screenshot_state(
                view_state.direct_primitive_workflow,
                scenario,
                "direct primitive workflow",
            )?;
            require_screenshot_state(
                !view_state.candidate_tray_visible && !view_state.selected_comparison_visible,
                scenario,
                "no active variation UI",
            )
        }
        ScreenshotScenario::PackDrawer => require_screenshot_state(
            view_state.pack_drawer_visible,
            scenario,
            "pack_drawer_visible",
        ),
        ScreenshotScenario::ExportDrawer => require_screenshot_state(
            view_state.export_drawer_visible,
            scenario,
            "export_drawer_visible",
        ),
        ScreenshotScenario::SphereExportDrawer => require_screenshot_state(
            view_state.export_drawer_visible && view_state.direct_primitive_workflow,
            scenario,
            "sphere_export_drawer_visible",
        ),
        ScreenshotScenario::ObjectPlanReviewDrawer => require_screenshot_state(
            view_state.object_plan_review_drawer_visible,
            scenario,
            "object_plan_review_drawer_visible",
        ),
        ScreenshotScenario::FamilyStudioLiteHiddenDefault => require_screenshot_state(
            !view_state.family_studio_lite_drawer_visible,
            scenario,
            "family studio entry hidden",
        ),
        ScreenshotScenario::FamilyStudioLiteDrawer
        | ScreenshotScenario::FamilyStudioLiteTestResult
        | ScreenshotScenario::FamilyStudioLiteSaveDraft
        | ScreenshotScenario::FamilyStudioLitePersonalSaved => require_screenshot_state(
            view_state.family_studio_lite_drawer_visible,
            scenario,
            "family_studio_lite_drawer_visible",
        ),
    }
}

pub(super) fn require_screenshot_state(
    passed: bool,
    scenario: ScreenshotScenario,
    requirement: &str,
) -> Result<(), String> {
    if passed {
        Ok(())
    } else {
        Err(format!("{scenario:?} missing {requirement}"))
    }
}

pub(super) fn record_screenshot_state_assertion(
    scenario: ScreenshotScenario,
    view_state: &MakeCanvasViewState,
    failure: Option<&str>,
) {
    let path = env::temp_dir().join("shape-lab-screenshot-state-assertions.txt");
    let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let result = failure.unwrap_or("PASS");
    let _ = writeln!(
        file,
        "{scenario:?}: {result}; mode={:?}; asset={}; busy={}; tray={}; comparison={}; focus={}; pack={}; export={}",
        view_state.mode,
        view_state.asset_name,
        view_state.local_busy_visible,
        view_state.candidate_tray_visible,
        view_state.selected_comparison_visible,
        view_state.focused_part_label.as_deref().unwrap_or("None"),
        view_state.pack_drawer_visible,
        view_state.export_drawer_visible
    );
}

#[cfg(test)]
pub(super) fn screenshot_part_group(
    state: &FoundryAppState,
    group_id: &str,
) -> Option<directions::DirectionPartGroup> {
    state
        .document
        .as_ref()
        .map(directions::direction_part_groups_for_document)?
        .into_iter()
        .find(|group| group.group_id == group_id && group.focusable)
}

impl FoundryDesktopApp {
    pub(super) fn screenshot_scenario_holds_active_job_capture(&self) -> bool {
        false
    }

    pub(super) fn apply_screenshot_scenario(
        &mut self,
        ctx: &egui::Context,
    ) -> Vec<FoundryAppCommand> {
        let Some(scenario) = self.screenshot_scenario else {
            return Vec::new();
        };
        if self.screenshot_scenario_step == u8::MAX {
            return Vec::new();
        }

        let mut commands = Vec::new();
        if let Some(selected_slug) = scenario.choose_selected_slug() {
            self.tab = FoundryTab::Home;
            self.state = FoundryAppState::default();
            self.selected_home_profile_slug = Some(selected_slug.to_owned());
            self.complete_screenshot_scenario(scenario);
            return commands;
        }

        if self.screenshot_scenario_step == 0 {
            self.load_fixture(read_screenshot_fixture_catalog(scenario), ctx);
            self.tab = FoundryTab::Make;
            self.screenshot_scenario_step = 1;
            return commands;
        }
        if self.state.current_output.is_none() {
            return commands;
        }
        if !self.state.active_jobs.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(33));
            return commands;
        }

        match scenario {
            ScreenshotScenario::ChooseGroupedPrimitives
            | ScreenshotScenario::ChooseBoxProvenance
            | ScreenshotScenario::ChooseFlatPanelProvenance
            | ScreenshotScenario::ChooseSpherePreset
            | ScreenshotScenario::ChooseSelectedPreview => {}
            ScreenshotScenario::BoxDirectMakeReady
            | ScreenshotScenario::FlatPanelDirectMakeReady
            | ScreenshotScenario::SphereDirectMakeReady
            | ScreenshotScenario::PanelKnobDirectMakeReady
            | ScreenshotScenario::OrbitAfterDragOrTool
            | ScreenshotScenario::ResetView => {
                if self.state.active_jobs.is_empty() {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::BoxPropertyEdit => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(FoundryAppCommand::run(FoundryCommand::SetControl {
                        control_id: "width".to_owned(),
                        value: orchard_foundry::ControlValue::Scalar(2.4),
                    }));
                    self.screenshot_scenario_step = 2;
                } else if self.make_canvas_view_state().mode == MakeCanvasMode::Ready {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::FlatPanelPropertyEdit => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(FoundryAppCommand::run(FoundryCommand::SetControl {
                        control_id: "thickness".to_owned(),
                        value: orchard_foundry::ControlValue::Scalar(0.28),
                    }));
                    self.screenshot_scenario_step = 2;
                } else if self.make_canvas_view_state().mode == MakeCanvasMode::Ready {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::SpherePropertyEdit => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(FoundryAppCommand::RunFoundryCommandProgram {
                        label: "Edit sphere properties".to_owned(),
                        commands: vec![
                            FoundryCommand::SetControl {
                                control_id: "width".to_owned(),
                                value: orchard_foundry::ControlValue::Scalar(1.08),
                            },
                            FoundryCommand::SetControl {
                                control_id: "height".to_owned(),
                                value: orchard_foundry::ControlValue::Scalar(1.0),
                            },
                            FoundryCommand::SetControl {
                                control_id: "depth".to_owned(),
                                value: orchard_foundry::ControlValue::Scalar(1.0),
                            },
                        ],
                    });
                    self.screenshot_scenario_step = 2;
                } else if self.make_canvas_view_state().mode == MakeCanvasMode::Ready {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::SphereKnobLikePreset => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(sphere_knob_like_form_preset_command());
                    self.screenshot_scenario_step = 2;
                } else if self.make_canvas_view_state().mode == MakeCanvasMode::Ready {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::SphereExportDrawer => {
                if self.screenshot_scenario_step < 2 {
                    commands.push(sphere_knob_like_form_preset_command());
                    self.screenshot_scenario_step = 2;
                } else {
                    self.drawer = Some(FoundryDrawer::Export);
                    if self.make_canvas_view_state().export_drawer_visible {
                        self.complete_screenshot_scenario(scenario);
                    }
                }
            }
            ScreenshotScenario::PackDrawer => {
                self.drawer = Some(FoundryDrawer::Pack);
                if self.state.pack.members.is_empty() {
                    if let Some(command) = self.add_current_to_pack_command() {
                        commands.push(command);
                    }
                    self.screenshot_scenario_step = 1;
                } else if self.make_canvas_view_state().pack_drawer_visible {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::ExportDrawer => {
                self.drawer = Some(FoundryDrawer::Export);
                if self.make_canvas_view_state().export_drawer_visible {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::ObjectPlanReviewDrawer => {
                self.object_plan_review_enabled = true;
                self.drawer = Some(FoundryDrawer::ObjectPlanReview);
                if self
                    .make_canvas_view_state()
                    .object_plan_review_drawer_visible
                {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::FamilyStudioLiteHiddenDefault => {
                self.family_studio_lite_enabled = false;
                self.drawer = None;
                self.complete_screenshot_scenario(scenario);
            }
            ScreenshotScenario::FamilyStudioLiteDrawer => {
                self.family_studio_lite_enabled = true;
                self.drawer = Some(FoundryDrawer::FamilyStudioLite);
                if self
                    .make_canvas_view_state()
                    .family_studio_lite_drawer_visible
                {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::FamilyStudioLiteTestResult => {
                self.family_studio_lite_enabled = true;
                self.drawer = Some(FoundryDrawer::FamilyStudioLite);
                self.run_family_studio_lite_test();
                if self
                    .make_canvas_view_state()
                    .family_studio_lite_drawer_visible
                {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::FamilyStudioLiteSaveDraft => {
                self.family_studio_lite_enabled = true;
                self.drawer = Some(FoundryDrawer::FamilyStudioLite);
                self.family_studio_lite.saved_visibility = Some(DirectKitVisibility::Draft);
                if self
                    .make_canvas_view_state()
                    .family_studio_lite_drawer_visible
                {
                    self.complete_screenshot_scenario(scenario);
                }
            }
            ScreenshotScenario::FamilyStudioLitePersonalSaved => {
                self.family_studio_lite_enabled = true;
                self.drawer = Some(FoundryDrawer::FamilyStudioLite);
                self.family_studio_lite.saved_visibility = Some(DirectKitVisibility::PersonalOnly);
                if self
                    .make_canvas_view_state()
                    .family_studio_lite_drawer_visible
                {
                    self.complete_screenshot_scenario(scenario);
                }
            }
        }
        if self.screenshot_scenario_step != u8::MAX {
            ctx.request_repaint_after(Duration::from_millis(33));
        }
        commands
    }

    #[cfg(test)]
    pub(super) fn ensure_screenshot_focus(&mut self, group_id: &str) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        if !self.state.active_jobs.is_empty() {
            return commands;
        }
        let Some(group) = screenshot_part_group(&self.state, group_id) else {
            return commands;
        };
        let active_group_id = self.state.document.as_ref().and_then(|document| {
            document
                .variation_state
                .intent
                .scope
                .semantic_part_group_id()
        });
        if active_group_id == Some(group_id) {
            self.screenshot_scenario_step = self.screenshot_scenario_step.max(2);
            return commands;
        }
        commands.push(directions::set_focus_part_group_command(&group));
        self.screenshot_scenario_step = 1;
        commands
    }

    pub(super) fn complete_screenshot_scenario(&mut self, scenario: ScreenshotScenario) {
        let view_state = self.make_canvas_view_state();
        let result = screenshot_scenario_assertion(scenario, &view_state);
        record_screenshot_state_assertion(
            scenario,
            &view_state,
            result.as_ref().err().map(String::as_str),
        );
        match result {
            Ok(()) => {
                self.screenshot_scenario_step = u8::MAX;
            }
            Err(message) => {
                self.state.status = Some(format!(
                    "Screenshot state assertion failed: {}",
                    product_panel_message(&message, "Screenshot state assertion failed.")
                ));
                self.screenshot_scenario_step = u8::MAX;
            }
        }
    }
}
