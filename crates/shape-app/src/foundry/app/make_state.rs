use super::*;

pub(super) fn candidate_previews_are_pending(
    candidates: &[crate::foundry::view_model::FoundryCandidateCard],
) -> bool {
    !candidates.is_empty()
        && candidates.iter().any(|candidate| {
            candidate.validation_label == "Preview pending"
                || candidate
                    .preview_failure
                    .as_deref()
                    .is_some_and(|reason| reason.contains("Preview rendering"))
        })
}

pub(super) fn tab_for_workflow_step(index: usize) -> FoundryTab {
    match index {
        1 => FoundryTab::Home,
        2 => FoundryTab::Make,
        _ => FoundryTab::Home,
    }
}

impl FoundryDesktopApp {
    pub(super) fn active_make_part_group(&self) -> Option<directions::DirectionPartGroup> {
        let document = self.state.document.as_ref()?;
        let active_group_id = document
            .variation_state
            .intent
            .scope
            .semantic_part_group_id()?;
        directions::direction_part_groups_for_document(document)
            .into_iter()
            .find(|group| group.group_id == active_group_id)
    }

    pub(super) fn make_canvas_view_state(&self) -> MakeCanvasViewState {
        let active_group = self.active_make_part_group();
        let selected_candidate = self
            .state
            .selected_candidate
            .as_ref()
            .and_then(|selected| {
                self.state
                    .candidates
                    .iter()
                    .find(|candidate| &candidate.id == selected)
            })
            .or_else(|| self.state.candidates.first());
        let asset_name = self.current_project_title();
        let active_profile_kind = self.active_make_profile_kind();
        let direct_primitive_workflow = active_profile_kind.direct_primitive_workflow();
        let simple_box_make_baseline = active_profile_kind.simple_clay_make_baseline();
        let lidded_box_baseline = active_profile_kind.is_lidded_box();
        let flat_panel_baseline = active_profile_kind.is_flat_panel_primitive();
        let hinged_panel_baseline = active_profile_kind.is_hinged_panel();
        let handled_panel_baseline = active_profile_kind.is_handled_panel();
        let panel_knob_baseline = active_profile_kind.is_panel_with_knob();
        let try_ideas_action_label = match active_profile_kind {
            MakeProfileKind::BoxPrimitive => ACTION_TRY_BOX_IDEAS,
            MakeProfileKind::LiddedBox => ACTION_TRY_LIDDED_BOX_IDEAS,
            MakeProfileKind::FlatPanelPrimitive => ACTION_TRY_PANEL_IDEAS,
            MakeProfileKind::SpherePrimitive => ACTION_TRY_WHOLE_ASSET_IDEAS,
            MakeProfileKind::HingedPanel => ACTION_TRY_HINGED_PANEL_IDEAS,
            MakeProfileKind::HandledPanel => ACTION_TRY_HANDLED_PANEL_IDEAS,
            MakeProfileKind::PanelWithKnob => ACTION_TRY_HANDLED_PANEL_IDEAS,
            MakeProfileKind::Other => ACTION_TRY_WHOLE_ASSET_IDEAS,
        };
        let use_candidate_action_label = if lidded_box_baseline {
            ACTION_USE_THIS_BOX
        } else if active_profile_kind.is_panel_like() {
            ACTION_USE_THIS_PANEL
        } else {
            ACTION_CHOOSE_DIRECTION
        };
        let adjust_heading_label = match active_profile_kind {
            MakeProfileKind::BoxPrimitive => ACTION_EDIT_BOX_PRIMITIVE,
            MakeProfileKind::LiddedBox => ACTION_EDIT_LIDDED_BOX,
            MakeProfileKind::FlatPanelPrimitive => ACTION_EDIT_FLAT_PANEL,
            MakeProfileKind::SpherePrimitive => ACTION_EDIT_SPHERE_PRIMITIVE,
            MakeProfileKind::HingedPanel => ACTION_EDIT_HINGED_PANEL,
            MakeProfileKind::HandledPanel => ACTION_EDIT_HANDLED_PANEL,
            MakeProfileKind::PanelWithKnob => ACTION_EDIT_PANEL_KNOB,
            MakeProfileKind::Other => "Adjust",
        };
        let active_candidate_job = self.state.active_jobs.values().any(|request| {
            matches!(
                request,
                FoundryJobRequest::GenerateCandidates { .. }
                    | FoundryJobRequest::RenderCandidatePreviews { .. }
            )
        });
        let candidate_previews_pending = candidate_previews_are_pending(&self.state.candidates);
        let generating =
            !direct_primitive_workflow && (active_candidate_job || candidate_previews_pending);
        let compiling_or_editing = self.state.active_jobs.values().any(|request| {
            matches!(
                request.slot(),
                FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
            )
        });
        let preview_rendering = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == FoundryJobSlot::RenderPreview);
        let model_ready = self.state.current_output.is_some();
        let quick_template_preview_visible =
            self.state.document.is_some() && self.state.current_output.is_none();
        let preview_image_ready = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| !preview.rgba8.is_empty());
        let preview_matches_current_build = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| preview.build == self.state.current_build);
        let stale_preview = preview_image_ready && !preview_matches_current_build;
        let current_asset_job_active =
            !preview_image_ready && (compiling_or_editing || preview_rendering);
        let preview_ready = model_ready && preview_image_ready;
        let preview_updating = model_ready
            && preview_image_ready
            && (stale_preview
                || compiling_or_editing
                || (preview_rendering && !preview_matches_current_build));
        let preview_update_required = model_ready
            && (!preview_image_ready || stale_preview)
            && !preview_rendering
            && !compiling_or_editing;
        let preparation_phase = if !model_ready {
            MakePreparationPhase::PreparingModel
        } else if !preview_image_ready {
            MakePreparationPhase::RenderingPreview
        } else {
            MakePreparationPhase::Ready
        };
        let focused_part_label = active_group.as_ref().map(|group| group.label.clone());
        let focused_part_visible = focused_part_label.is_some();
        let focused_part_actions_visible = focused_part_visible && model_ready && preview_ready;
        let preparing = self.state.document.is_some()
            && (!model_ready || !preview_image_ready || current_asset_job_active);
        let preparation_timed_out = preparing
            && self
                .make_preparation_started_at
                .is_some_and(|started| started.elapsed() >= PREPARATION_TIMEOUT);
        let preparation_fallback_visible = preparation_timed_out;
        let idea_generation_timed_out = !direct_primitive_workflow
            && generating
            && self
                .make_generation_started_at
                .is_some_and(|started| started.elapsed() >= IDEA_GENERATION_TIMEOUT);
        let idea_generation_fallback_visible = idea_generation_timed_out;
        let local_warning_message = self.make_canvas_local_warning().filter(|message| {
            !direct_primitive_workflow || message.as_str() != CANCELED_IDEA_SEARCH_WARNING
        });
        let local_error_message = self.make_canvas_local_error();
        let candidate_search_finished_empty = !direct_primitive_workflow
            && self.state.candidate_output.is_some()
            && self.state.candidates.is_empty()
            && !generating;
        let mode = if self.drawer == Some(FoundryDrawer::Pack) {
            MakeCanvasMode::PackDrawerOpen
        } else if self.drawer == Some(FoundryDrawer::Export) {
            MakeCanvasMode::ExportDrawerOpen
        } else if local_error_message.is_some() {
            MakeCanvasMode::Error
        } else if self.state.document.is_none() {
            MakeCanvasMode::NoAsset
        } else if preparing {
            MakeCanvasMode::PreparingAsset
        } else if !direct_primitive_workflow && !self.state.candidates.is_empty() {
            MakeCanvasMode::ReviewingIdeas
        } else if generating && focused_part_label.is_some() {
            MakeCanvasMode::GeneratingFocusedPartIdeas
        } else if generating {
            MakeCanvasMode::GeneratingWholeAssetIdeas
        } else if focused_part_label.is_some() {
            MakeCanvasMode::FocusedPart
        } else {
            MakeCanvasMode::Ready
        };
        let local_busy_label = match mode {
            MakeCanvasMode::PreparingAsset => Some(format!("Preparing {}...", asset_name)),
            MakeCanvasMode::GeneratingFocusedPartIdeas => focused_part_label.as_ref().map(|part| {
                format!(
                    "Trying {} ideas from the current {}...",
                    singular_part_copy(part).to_ascii_lowercase(),
                    make_busy_asset_noun(&asset_name)
                )
            }),
            MakeCanvasMode::GeneratingWholeAssetIdeas => Some(format!(
                "Trying ideas from the current {}...",
                make_busy_asset_noun(&asset_name)
            )),
            MakeCanvasMode::ReviewingIdeas if candidate_previews_pending => {
                Some("Rendering previews...".to_owned())
            }
            _ => None,
        };
        let local_busy_visible = local_busy_label.is_some();
        let candidate_tray_state = if direct_primitive_workflow {
            MakeCandidateTrayState::EmptyReady
        } else if local_error_message.is_some() {
            MakeCandidateTrayState::ErrorWithRecovery
        } else if !self.state.candidates.is_empty() {
            MakeCandidateTrayState::HasCandidates
        } else if generating {
            MakeCandidateTrayState::GeneratingSkeletons
        } else if candidate_search_finished_empty {
            MakeCandidateTrayState::NoCandidatesWithRecovery
        } else {
            MakeCandidateTrayState::EmptyReady
        };
        let focused_no_candidates_recovery_visible = candidate_tray_state
            == MakeCandidateTrayState::NoCandidatesWithRecovery
            && focused_part_label.is_some();
        let (local_banner_title, local_banner_message, local_banner_tone) =
            make_canvas_local_banner(MakeCanvasBannerContext {
                mode: &mode,
                asset_name: &asset_name,
                preparation_phase,
                preparation_timed_out,
                idea_generation_timed_out,
                preview_updating,
                candidate_previews_pending,
                local_busy_label: &local_busy_label,
                active_group: active_group.as_ref(),
                candidate_output: self.state.candidate_output.as_deref(),
                local_warning_message: local_warning_message.as_deref(),
                local_error_message: local_error_message.as_deref(),
                direct_primitive_workflow,
                simple_box_make_baseline,
                lidded_box_baseline,
                flat_panel_baseline,
                hinged_panel_baseline,
                handled_panel_baseline,
                panel_knob_baseline,
            });
        let primary_title = match (&mode, focused_part_label.as_deref()) {
            (MakeCanvasMode::NoAsset, _) => "Choose an asset".to_owned(),
            (MakeCanvasMode::PreparingAsset, _) => {
                format!("Preparing {}", asset_name)
            }
            (_, Some(label)) => label.to_owned(),
            _ => asset_name.clone(),
        };
        let primary_action_label = match (&mode, focused_part_label.as_ref()) {
            (MakeCanvasMode::GeneratingWholeAssetIdeas, _)
            | (MakeCanvasMode::GeneratingFocusedPartIdeas, _) => ACTION_GENERATING_IDEAS.to_owned(),
            (MakeCanvasMode::NoAsset, _) => ACTION_CHOOSE_TEMPLATE.to_owned(),
            (MakeCanvasMode::ReviewingIdeas, _) => use_candidate_action_label.to_owned(),
            _ if focused_no_candidates_recovery_visible => {
                ACTION_TRY_WHOLE_ASSET_RECOVERY.to_owned()
            }
            (_, Some(label)) => format!(
                "Try {} ideas",
                singular_part_copy(label).to_ascii_lowercase()
            ),
            _ if direct_primitive_workflow => ACTION_ADJUST_DIMENSIONS.to_owned(),
            _ => try_ideas_action_label.to_owned(),
        };
        let primary_action_enabled = match mode {
            MakeCanvasMode::NoAsset => true,
            MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart => {
                direct_primitive_workflow || (model_ready && preview_ready && !generating)
            }
            MakeCanvasMode::ReviewingIdeas => {
                model_ready
                    && preview_ready
                    && !generating
                    && selected_candidate.is_some_and(|candidate| candidate.selectable)
            }
            _ => false,
        };
        let primary_action_disabled_reason = (!primary_action_enabled).then(|| {
            if preparing {
                if preview_updating {
                    PREVIEW_UPDATING_REASON.to_owned()
                } else {
                    ASSET_PREPARING_REASON.to_owned()
                }
            } else if generating {
                ACTIVE_IDEA_JOB_REASON.to_owned()
            } else if mode == MakeCanvasMode::ReviewingIdeas {
                NEED_DIRECTION_REASON.to_owned()
            } else if let Some(message) = &local_error_message {
                message.clone()
            } else {
                NEED_PROJECT_REASON.to_owned()
            }
        });
        let candidate_tray_visible = self.state.document.is_some() && !direct_primitive_workflow;
        let material_look_tray_visible = self.material_looks.tray_open && !simple_box_make_baseline;
        let rejected_candidate_summary = (!direct_primitive_workflow)
            .then(|| self.make_canvas_rejected_candidate_summary())
            .flatten();
        let selected_comparison_visible = !direct_primitive_workflow
            && selected_candidate.is_some_and(|candidate| {
                preview_ready
                    && !candidate.rgba8.is_empty()
                    && self
                        .state
                        .current_preview
                        .as_ref()
                        .is_some_and(|preview| !preview.rgba8.is_empty())
            });
        let mut next_action_hint = make_canvas_next_action_hint(
            &mode,
            focused_part_label.as_deref(),
            selected_comparison_visible,
            active_profile_kind,
        );
        if preparation_fallback_visible {
            next_action_hint = PREPARATION_TIMEOUT_MESSAGE.to_owned();
        } else if idea_generation_fallback_visible {
            next_action_hint = IDEA_GENERATION_TIMEOUT_MESSAGE.to_owned();
        } else if preview_update_required {
            next_action_hint = "Update preview to keep making changes.".to_owned();
        }

        MakeCanvasViewState {
            asset_name,
            mode,
            preparation_phase,
            preparation_timed_out,
            preparation_fallback_visible,
            idea_generation_timed_out,
            idea_generation_fallback_visible,
            preview_updating,
            preview_update_required,
            local_banner_title,
            local_banner_message,
            local_banner_tone,
            primary_title,
            primary_action_label,
            primary_action_enabled,
            primary_action_disabled_reason,
            local_busy_label,
            local_busy_visible,
            focused_part_label,
            focused_part_visible,
            focused_part_actions_visible,
            quick_template_preview_visible,
            model_ready,
            preview_ready,
            candidate_tray_visible,
            material_look_tray_visible,
            candidate_tray_state,
            candidate_count: if direct_primitive_workflow {
                0
            } else {
                self.state.candidates.len()
            },
            candidate_search_finished_empty,
            focused_no_candidates_recovery_visible,
            rejected_candidate_summary,
            selected_candidate_present: !direct_primitive_workflow
                && self.state.selected_candidate.is_some(),
            selected_comparison_visible,
            pack_drawer_visible: self.drawer == Some(FoundryDrawer::Pack),
            export_drawer_visible: self.drawer == Some(FoundryDrawer::Export),
            object_plan_review_drawer_visible: self.object_plan_review_ui_state().drawer_visible,
            family_studio_lite_drawer_visible: self.family_studio_lite_ui_state().drawer_visible,
            local_warning_message,
            local_error_message,
            next_action_hint,
            direct_primitive_workflow,
            property_panel_title: direct_property_panel_title(active_profile_kind),
            property_labels: direct_property_labels(active_profile_kind).to_vec(),
            simple_box_make_baseline,
            lidded_box_baseline,
            flat_panel_baseline,
            hinged_panel_baseline,
            handled_panel_baseline,
            panel_knob_baseline,
            try_ideas_action_label,
            use_candidate_action_label,
            adjust_heading_label,
        }
    }

    pub(super) fn make_canvas_local_warning(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if self.suppresses_background_result_status(status) {
            return None;
        }
        if status.starts_with("Ignored a background result") {
            Some(STALE_RESULT_WARNING.to_owned())
        } else if status == CANCELED_IDEA_SEARCH_WARNING {
            Some(CANCELED_IDEA_SEARCH_WARNING.to_owned())
        } else {
            None
        }
    }

    pub(super) fn suppresses_background_result_status(&self, status: &str) -> bool {
        if !status.starts_with("Ignored a background result") {
            return false;
        }

        self.active_make_profile_kind().direct_primitive_workflow()
            || (self.drawer == Some(FoundryDrawer::FamilyStudioLite)
                && self.family_studio_lite_enabled)
    }

    pub(super) fn make_canvas_rejected_candidate_summary(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.contains("Rejected") {
            Some(product_panel_message(
                status,
                "Some ideas were rejected because they looked too similar.",
            ))
        } else {
            None
        }
    }

    pub(super) fn make_canvas_local_error(&self) -> Option<String> {
        let status = self.state.status.as_deref()?;
        if status.starts_with("Ignored a background result") {
            return None;
        }
        let lower = status.to_ascii_lowercase();
        if lower.contains("failed")
            || lower.contains("could not")
            || lower.contains("missing")
            || lower.contains("disconnected")
        {
            Some(product_panel_message(
                status,
                "The current asset needs attention.",
            ))
        } else {
            None
        }
    }

    pub(super) fn make_is_preparing_now(&self) -> bool {
        if self.state.document.is_none() {
            return false;
        }

        let compiling_or_editing = self.state.active_jobs.values().any(|request| {
            matches!(
                request.slot(),
                FoundryJobSlot::CompileCurrent | FoundryJobSlot::ApplyEdit
            )
        });
        let preview_rendering = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == FoundryJobSlot::RenderPreview);
        let model_ready = self.state.current_output.is_some();
        let preview_image_ready = self
            .state
            .current_preview
            .as_ref()
            .is_some_and(|preview| !preview.rgba8.is_empty());
        let preview_ready = model_ready && preview_image_ready;

        !model_ready
            || !preview_ready
            || (!preview_image_ready && (compiling_or_editing || preview_rendering))
    }

    pub(super) fn refresh_make_preparation_timer(&mut self) {
        if self.make_is_preparing_now() {
            self.make_preparation_started_at
                .get_or_insert_with(Instant::now);
        } else {
            self.make_preparation_started_at = None;
        }
    }

    pub(super) fn refresh_make_generation_timer(&mut self) {
        if self.directions_are_generating() {
            self.make_generation_started_at
                .get_or_insert_with(Instant::now);
        } else {
            self.make_generation_started_at = None;
        }
    }

    pub(super) fn refresh_make_trace_clock(&mut self) {
        let elapsed_ms = self.make_trace_started_at.elapsed().as_millis();
        let elapsed_ms = elapsed_ms.min(u128::from(u64::MAX)) as u64;
        self.state.set_make_trace_elapsed_ms(elapsed_ms);
    }

    pub(super) fn persist_make_job_trace_outputs(&mut self) {
        if let Err(error) = self
            .state
            .write_make_job_trace_outputs(Path::new(MAKE_JOB_TRACE_DIR))
        {
            eprintln!("Could not write Make job trace: {error}");
        }
    }

    pub(super) fn directions_are_generating(&self) -> bool {
        self.state.active_jobs.values().any(|request| {
            matches!(
                request,
                FoundryJobRequest::GenerateCandidates { .. }
                    | FoundryJobRequest::RenderCandidatePreviews { .. }
            )
        }) || candidate_previews_are_pending(&self.state.candidates)
    }

    pub(super) fn active_profile_matches(&self, profile_id: &str) -> bool {
        self.state.document.as_ref().is_some_and(|document| {
            document
                .customizer_profile_ref
                .stable_id
                .contains(profile_id)
                || document.family_content_ref.stable_id.contains(profile_id)
                || document.document_id.0.contains(profile_id)
        })
    }

    pub(super) fn active_make_profile_kind(&self) -> MakeProfileKind {
        if self.active_profile_matches(BOX_PRIMITIVE_PROFILE_ID) {
            MakeProfileKind::BoxPrimitive
        } else if self.active_profile_matches(LIDDED_BOX_PROFILE_ID) {
            MakeProfileKind::LiddedBox
        } else if self.active_profile_matches(FLAT_PANEL_PRIMITIVE_PROFILE_ID) {
            MakeProfileKind::FlatPanelPrimitive
        } else if self.active_profile_matches(SPHERE_PRIMITIVE_PROFILE_ID) {
            MakeProfileKind::SpherePrimitive
        } else if self.active_profile_matches(HINGED_PANEL_PROFILE_ID) {
            MakeProfileKind::HingedPanel
        } else if self.active_profile_matches(HANDLED_PANEL_PROFILE_ID) {
            MakeProfileKind::HandledPanel
        } else if self.active_profile_matches(PANEL_KNOB_PROFILE_ID) {
            MakeProfileKind::PanelWithKnob
        } else {
            MakeProfileKind::Other
        }
    }
}
