use super::*;

pub(super) fn make_canvas_build_dependent_actions_enabled(
    view_state: &MakeCanvasViewState,
) -> bool {
    view_state.model_ready
        && view_state.preview_ready
        && !view_state.preview_updating
        && !view_state.local_busy_visible
        && view_state.mode != MakeCanvasMode::Error
}

pub(super) fn make_canvas_build_dependent_disabled_reason(
    view_state: &MakeCanvasViewState,
) -> &'static str {
    if view_state.mode == MakeCanvasMode::NoAsset {
        NEED_PROJECT_REASON
    } else if matches!(
        view_state.mode,
        MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas
    ) {
        ACTIVE_IDEA_JOB_REASON
    } else if view_state.preview_updating {
        PREVIEW_UPDATING_REASON
    } else {
        ASSET_PREPARING_REASON
    }
}

pub(super) fn make_canvas_controls_enabled(view_state: &MakeCanvasViewState) -> bool {
    view_state.model_ready && view_state.preview_ready && !view_state.local_busy_visible
}

pub(super) fn make_canvas_candidate_actions_enabled(view_state: &MakeCanvasViewState) -> bool {
    view_state.candidate_ui_enabled
        && view_state.model_ready
        && view_state.preview_ready
        && !matches!(
            view_state.mode,
            MakeCanvasMode::PreparingAsset
                | MakeCanvasMode::GeneratingWholeAssetIdeas
                | MakeCanvasMode::GeneratingFocusedPartIdeas
                | MakeCanvasMode::Error
        )
}

pub(super) fn make_canvas_layout(
    available: egui::Vec2,
    view_state: &MakeCanvasViewState,
) -> MakeCanvasLayout {
    let available_width = available.x.max(1.0);
    let available_height = available.y.max(1.0);
    let stacked_columns = available_width < 900.0;
    let inline_ideas = make_canvas_uses_inline_ideas(view_state)
        && !view_state.material_look_tray_visible
        && !stacked_columns
        && available_width >= 1240.0;
    let content_gap = if available_width >= 1280.0 {
        14.0
    } else {
        10.0
    };
    let compact_ideas = make_canvas_uses_compact_ideas(view_state);
    let requested_tray_height = if inline_ideas
        || (!view_state.candidate_tray_visible && !view_state.material_look_tray_visible)
    {
        0.0
    } else if view_state.material_look_tray_visible {
        (available_height * 0.52).clamp(330.0, 440.0)
    } else if compact_ideas {
        (available_height * 0.13).clamp(88.0, 128.0)
    } else {
        (available_height * 0.38).clamp(240.0, 420.0)
    };
    let min_top_height = if view_state.material_look_tray_visible {
        if stacked_columns { 360.0 } else { 320.0 }
    } else if stacked_columns {
        520.0
    } else {
        390.0
    };
    let max_tray_height = (available_height - min_top_height - content_gap).max(0.0);
    let tray_height = requested_tray_height.min(max_tray_height);
    let tray_gap = if tray_height > 0.0 { content_gap } else { 0.0 };
    let top_height = (available_height - tray_height - tray_gap).max(available_height.min(260.0));

    let (stage_width, ideas_width, inspector_width) = if stacked_columns {
        (available_width, available_width, available_width)
    } else if inline_ideas {
        let desired_inspector_width = (available_width * 0.20).clamp(320.0, 380.0);
        let desired_stage_width = (available_width * 0.38).clamp(520.0, 760.0);
        let min_stage_width = 430.0;
        let min_ideas_width = 430.0;
        let min_inspector_width = 300.0;
        let mut inspector_width = desired_inspector_width.min(available_width * 0.27);
        let mut stage_width = desired_stage_width;
        let mut ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);

        if ideas_width < min_ideas_width {
            let deficit = min_ideas_width - ideas_width;
            stage_width = (stage_width - deficit).max(min_stage_width);
            ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);
        }
        if ideas_width < min_ideas_width {
            let deficit = min_ideas_width - ideas_width;
            inspector_width = (inspector_width - deficit).max(min_inspector_width);
            ideas_width = available_width - stage_width - inspector_width - (content_gap * 2.0);
        }

        (stage_width, ideas_width.max(300.0), inspector_width)
    } else {
        let min_stage_width = if available_width >= 1500.0 {
            760.0
        } else {
            520.0
        };
        let max_inspector_width = (available_width - min_stage_width - content_gap).max(300.0);
        let desired_inspector_width = if available_width >= 1500.0 {
            (available_width * 0.30).clamp(420.0, 520.0)
        } else {
            (available_width * 0.34).clamp(340.0, 440.0)
        };
        let inspector_width = desired_inspector_width
            .min(max_inspector_width)
            .min(available_width - content_gap)
            .max(300.0);
        let stage_width = (available_width - inspector_width - content_gap).max(300.0);
        (stage_width, 0.0, inspector_width)
    };

    MakeCanvasLayout {
        top_height,
        tray_height,
        tray_gap,
        stage_width,
        ideas_width,
        inspector_width,
        content_gap,
        stacked_columns,
        inline_ideas,
        compact_ideas,
    }
}

pub(super) fn make_canvas_uses_compact_ideas(view_state: &MakeCanvasViewState) -> bool {
    view_state.candidate_tray_state == MakeCandidateTrayState::EmptyReady
        && view_state.local_warning_message.is_none()
        && view_state.local_error_message.is_none()
}

pub(super) fn make_canvas_uses_inline_ideas(view_state: &MakeCanvasViewState) -> bool {
    view_state.candidate_tray_visible
        && matches!(
            view_state.candidate_tray_state,
            MakeCandidateTrayState::GeneratingSkeletons
                | MakeCandidateTrayState::HasCandidates
                | MakeCandidateTrayState::NoCandidatesWithRecovery
                | MakeCandidateTrayState::ErrorWithRecovery
        )
}

pub(super) fn make_canvas_stacked_stage_height(top_height: f32) -> f32 {
    if top_height <= 420.0 {
        return (top_height * 0.56).max(180.0);
    }
    (top_height * 0.58).clamp(240.0, top_height - 180.0)
}

pub(super) fn make_stage_preview_edge(available_width: f32, available_height: f32) -> f32 {
    let vertical_budget = available_height * 0.68;
    let horizontal_budget = available_width - 18.0;
    vertical_budget.min(horizontal_budget).clamp(180.0, 520.0)
}

pub(super) fn make_canvas_inspector_build_actions_visible(
    view_state: &MakeCanvasViewState,
) -> bool {
    view_state.preview_ready
        || view_state.preview_update_required
        || matches!(
            view_state.mode,
            MakeCanvasMode::ReviewingIdeas
                | MakeCanvasMode::FocusedPart
                | MakeCanvasMode::PackDrawerOpen
                | MakeCanvasMode::ExportDrawerOpen
                | MakeCanvasMode::Error
        )
}
