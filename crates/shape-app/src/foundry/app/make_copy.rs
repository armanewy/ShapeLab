use super::*;

pub(super) struct MakeCanvasBannerContext<'a> {
    pub(super) mode: &'a MakeCanvasMode,
    pub(super) asset_name: &'a str,
    pub(super) preparation_phase: MakePreparationPhase,
    pub(super) preparation_timed_out: bool,
    pub(super) idea_generation_timed_out: bool,
    pub(super) preview_updating: bool,
    pub(super) candidate_previews_pending: bool,
    pub(super) local_busy_label: &'a Option<String>,
    pub(super) active_group: Option<&'a directions::DirectionPartGroup>,
    pub(super) candidate_output: Option<&'a FoundryCandidateOutput>,
    pub(super) local_warning_message: Option<&'a str>,
    pub(super) local_error_message: Option<&'a str>,
    pub(super) direct_primitive_workflow: bool,
    pub(super) simple_box_make_baseline: bool,
    pub(super) lidded_box_baseline: bool,
    pub(super) flat_panel_baseline: bool,
    pub(super) hinged_panel_baseline: bool,
    pub(super) handled_panel_baseline: bool,
    pub(super) panel_knob_baseline: bool,
}

pub(super) fn make_canvas_local_banner(
    context: MakeCanvasBannerContext<'_>,
) -> (String, String, BannerTone) {
    let MakeCanvasBannerContext {
        mode,
        asset_name,
        preparation_phase,
        preparation_timed_out,
        idea_generation_timed_out,
        preview_updating,
        candidate_previews_pending,
        local_busy_label,
        active_group,
        candidate_output,
        local_warning_message,
        local_error_message,
        direct_primitive_workflow,
        simple_box_make_baseline,
        lidded_box_baseline,
        flat_panel_baseline,
        hinged_panel_baseline,
        handled_panel_baseline,
        panel_knob_baseline,
    } = context;
    if let Some(message) = local_warning_message {
        return (
            make_canvas_warning_title(message).to_owned(),
            format!("{message} Try again when you are ready."),
            BannerTone::Warning,
        );
    }
    if let Some(message) = local_error_message {
        return (
            "Asset needs attention".to_owned(),
            message.to_owned(),
            BannerTone::Error,
        );
    }
    if !direct_primitive_workflow
        && matches!(mode, MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart)
        && candidate_output.is_some_and(|output| output.candidates.is_empty())
    {
        let (title, message) = no_candidates_recovery_copy(active_group, candidate_output);
        return (title, message, BannerTone::Warning);
    }

    match mode {
        MakeCanvasMode::NoAsset => (
            "Choose an asset".to_owned(),
            concat!(
                "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, ",
                "Hinged Panel, Panel with Knob, or open a project before ",
                "making changes."
            )
            .to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::PreparingAsset => {
            let message = if preparation_timed_out {
                PREPARATION_TIMEOUT_MESSAGE.to_owned()
            } else if preview_updating {
                PREVIEW_UPDATING_REASON.to_owned()
            } else {
                preparation_phase.label().to_owned()
            };
            (format!("Preparing {asset_name}"), message, BannerTone::Info)
        }
        MakeCanvasMode::GeneratingWholeAssetIdeas | MakeCanvasMode::GeneratingFocusedPartIdeas => {
            let message = if idea_generation_timed_out {
                IDEA_GENERATION_TIMEOUT_MESSAGE.to_owned()
            } else {
                local_busy_label
                    .clone()
                    .unwrap_or_else(|| "Trying ideas from the current asset...".to_owned())
            };
            ("Trying ideas".to_owned(), message, BannerTone::Info)
        }
        MakeCanvasMode::ReviewingIdeas if candidate_previews_pending => (
            "Rendering previews".to_owned(),
            "Candidate shells are ready. Use an idea after its preview renders.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::ReviewingIdeas => (
            if lidded_box_baseline {
                "Lidded box ideas ready".to_owned()
            } else if handled_panel_baseline {
                "Handled panel ideas ready".to_owned()
            } else if hinged_panel_baseline {
                "Hinged panel ideas ready".to_owned()
            } else if flat_panel_baseline {
                "Panel ideas ready".to_owned()
            } else if simple_box_make_baseline {
                "Box ideas ready".to_owned()
            } else {
                "Ideas ready".to_owned()
            },
            if lidded_box_baseline {
                "Use this box, or reject it.".to_owned()
            } else if handled_panel_baseline || hinged_panel_baseline || flat_panel_baseline {
                "Use this panel, or reject it.".to_owned()
            } else if simple_box_make_baseline {
                "Use this idea, or reject it.".to_owned()
            } else {
                "Compare the selected idea, then use it or reject it.".to_owned()
            },
            BannerTone::Success,
        ),
        MakeCanvasMode::PackDrawerOpen => (
            "Pack drawer open".to_owned(),
            "Review pack members or export the pack.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::ExportDrawerOpen => (
            "Export drawer open".to_owned(),
            "Choose an export option when readiness is clear.".to_owned(),
            BannerTone::Info,
        ),
        MakeCanvasMode::Error => (
            "Asset needs attention".to_owned(),
            "The current asset needs attention.".to_owned(),
            BannerTone::Error,
        ),
        MakeCanvasMode::Ready | MakeCanvasMode::FocusedPart => {
            if direct_primitive_workflow {
                let message = if panel_knob_baseline {
                    "Adjust properties, Add to Pack, or Export current asset."
                } else {
                    "Adjust dimensions, Add to Pack, or Export current primitive."
                };
                ("Ready".to_owned(), message.to_owned(), BannerTone::Success)
            } else if lidded_box_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try lidded box ideas, adjust lid seam or proportions, Add to Pack, or Export."
                        .to_owned(),
                    BannerTone::Success,
                )
            } else if hinged_panel_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try hinged panel ideas, adjust hinge edge or proportions, Add to Pack, or Export."
                        .to_owned(),
                    BannerTone::Success,
                )
            } else if handled_panel_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try handled panel ideas, adjust handle or proportions, Add to Pack, or Export."
                        .to_owned(),
                    BannerTone::Success,
                )
            } else if flat_panel_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try panel ideas, adjust Proportions or Edge Softness, Add to Pack, or Export."
                        .to_owned(),
                    BannerTone::Success,
                )
            } else if simple_box_make_baseline && matches!(mode, MakeCanvasMode::Ready) {
                (
                    "Ready".to_owned(),
                    "Try box ideas, adjust box, Add to Pack, or Export.".to_owned(),
                    BannerTone::Success,
                )
            } else {
                (
                    "Ready to try ideas".to_owned(),
                    "Try ideas, focus a part, or tune controls.".to_owned(),
                    BannerTone::Success,
                )
            }
        }
    }
}

pub(super) fn empty_candidate_tray_copy(
    view_state: &MakeCanvasViewState,
) -> (&'static str, &'static str) {
    if view_state.direct_primitive_workflow {
        if view_state.panel_knob_baseline {
            return (
                "Direct properties ready",
                "Adjust properties, Add to Pack, or Export current asset.",
            );
        }
        return (
            "Direct properties ready",
            "Adjust dimensions, Add to Pack, or Export current primitive.",
        );
    }
    if view_state.mode == MakeCanvasMode::PreparingAsset {
        (
            "Ideas unlock when ready",
            "The asset can be adjusted while the preview prepares.",
        )
    } else {
        (
            "Ready to try ideas",
            if view_state.lidded_box_baseline {
                "Try lidded box ideas when the box is ready."
            } else if view_state.handled_panel_baseline {
                "Try handled panel ideas when the panel is ready."
            } else if view_state.hinged_panel_baseline {
                "Try hinged panel ideas when the panel is ready."
            } else if view_state.flat_panel_baseline {
                "Try panel ideas when the panel is ready."
            } else if view_state.simple_box_make_baseline {
                "Try box ideas when the box is ready."
            } else {
                "Try ideas or focus a part when the asset is ready."
            },
        )
    }
}

pub(super) fn no_candidates_recovery_copy(
    active_group: Option<&directions::DirectionPartGroup>,
    candidate_output: Option<&FoundryCandidateOutput>,
) -> (String, String) {
    let reason = no_candidates_reason_copy(candidate_output);
    if let Some(group) = active_group {
        let part = singular_part_copy(&group.label).to_ascii_lowercase();
        let message =
            format!("No clear {part} ideas survived. {reason} Try again or unlock controls.");
        return ("No clear focused ideas survived".to_owned(), message);
    }

    (
        "No clear ideas survived".to_owned(),
        format!("{reason} Try again or adjust the current asset."),
    )
}

pub(super) fn direct_property_panel_title(profile_kind: MakeProfileKind) -> &'static str {
    match profile_kind {
        MakeProfileKind::BoxPrimitive => ACTION_EDIT_BOX_PRIMITIVE,
        MakeProfileKind::FlatPanelPrimitive => ACTION_EDIT_FLAT_PANEL,
        MakeProfileKind::SpherePrimitive => ACTION_EDIT_SPHERE_PRIMITIVE,
        MakeProfileKind::LiddedBox => ACTION_EDIT_LIDDED_BOX,
        MakeProfileKind::HingedPanel => ACTION_EDIT_HINGED_PANEL,
        MakeProfileKind::HandledPanel => ACTION_EDIT_HANDLED_PANEL,
        MakeProfileKind::PanelWithKnob => ACTION_EDIT_PANEL_KNOB,
        MakeProfileKind::Other => "Adjust controls",
    }
}

pub(super) fn direct_property_labels(profile_kind: MakeProfileKind) -> &'static [&'static str] {
    match profile_kind {
        MakeProfileKind::BoxPrimitive => &["Width", "Depth", "Height", "Edge Softness"],
        MakeProfileKind::FlatPanelPrimitive => &["Width", "Height", "Thickness", "Edge Softness"],
        MakeProfileKind::SpherePrimitive => {
            &["Width", "Height", "Depth", "Front Flatten", "Back Flatten"]
        }
        MakeProfileKind::LiddedBox => &["Width", "Depth", "Height", "Edge Softness", "Lid Seam"],
        MakeProfileKind::HingedPanel => &[
            "Width",
            "Height",
            "Thickness",
            "Edge Softness",
            "Hinge Edge",
        ],
        MakeProfileKind::HandledPanel => &[
            "Width",
            "Height",
            "Thickness",
            "Edge Softness",
            "Hinge Edge",
            "Handle",
        ],
        MakeProfileKind::PanelWithKnob => &[
            "Panel Width",
            "Panel Height",
            "Panel Thickness",
            "Panel Edge Softness",
            "Knob Width",
            "Knob Height",
            "Knob Depth",
            "Knob Front Flatten",
            "Knob Back Flatten",
            "Knob Horizontal Position",
            "Knob Vertical Position",
        ],
        MakeProfileKind::Other => &[],
    }
}

pub(super) fn sphere_knob_like_form_preset_command() -> FoundryAppCommand {
    FoundryAppCommand::RunFoundryCommandProgram {
        label: ACTION_KNOB_LIKE_FORM.to_owned(),
        commands: shape_foundry_catalog::sphere_primitive::knob_like_form_preset_values()
            .into_iter()
            .map(|(control_id, value)| FoundryCommand::SetControl { control_id, value })
            .collect(),
    }
}

pub(super) fn make_canvas_warning_title(message: &str) -> &'static str {
    if message == CANCELED_IDEA_SEARCH_WARNING {
        "Idea search canceled"
    } else {
        "Older result ignored"
    }
}

pub(super) fn no_candidates_reason_copy(
    candidate_output: Option<&FoundryCandidateOutput>,
) -> &'static str {
    let Some(output) = candidate_output else {
        return "The search did not find a clear visible change.";
    };
    if output.diagnostics.wrong_scope_rejections > 0 {
        "The clearest changes affected something outside the focused part."
    } else if output.diagnostics.hidden_internal_rejections > 0 {
        "The search found changes that were hidden or too subtle."
    } else if output.diagnostics.duplicate_looking_rejections > 0 {
        "The search found ideas that looked too similar."
    } else {
        "The search did not find a clear visible change."
    }
}

pub(super) fn make_canvas_mode_summary(view_state: &MakeCanvasViewState) -> &'static str {
    match view_state.mode {
        MakeCanvasMode::NoAsset => "Choose a starting point first.",
        MakeCanvasMode::PreparingAsset if view_state.preparation_timed_out => {
            PREPARATION_TIMEOUT_MESSAGE
        }
        MakeCanvasMode::PreparingAsset if view_state.preview_updating => PREVIEW_UPDATING_REASON,
        MakeCanvasMode::PreparingAsset => ASSET_PREPARING_REASON,
        MakeCanvasMode::GeneratingWholeAssetIdeas => "Trying ideas from the current asset.",
        MakeCanvasMode::GeneratingFocusedPartIdeas => "Trying ideas for the focused part.",
        MakeCanvasMode::ReviewingIdeas if view_state.lidded_box_baseline => {
            "Use this box, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas if view_state.hinged_panel_baseline => {
            "Use this panel, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas if view_state.handled_panel_baseline => {
            "Use this panel, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas if view_state.flat_panel_baseline => {
            "Use this panel, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas if view_state.simple_box_make_baseline => {
            "Use this idea, or try another idea."
        }
        MakeCanvasMode::ReviewingIdeas => "Compare the selected idea against the current asset.",
        MakeCanvasMode::FocusedPart if view_state.direct_primitive_workflow => {
            "Adjust bounded properties for this primitive."
        }
        MakeCanvasMode::FocusedPart => "This part is focused. Try ideas, lock it, or clear focus.",
        MakeCanvasMode::PackDrawerOpen => "The pack drawer is open.",
        MakeCanvasMode::ExportDrawerOpen => "The export drawer is open.",
        MakeCanvasMode::Ready if view_state.direct_primitive_workflow => {
            if view_state.panel_knob_baseline {
                "Adjust panel and knob properties directly."
            } else {
                "Adjust dimensions directly."
            }
        }
        MakeCanvasMode::Ready if view_state.lidded_box_baseline => {
            "Try lidded box ideas or adjust lid seam."
        }
        MakeCanvasMode::Ready if view_state.hinged_panel_baseline => {
            "Try hinged panel ideas or adjust hinge edge."
        }
        MakeCanvasMode::Ready if view_state.handled_panel_baseline => {
            "Try handled panel ideas or adjust handle."
        }
        MakeCanvasMode::Ready if view_state.flat_panel_baseline => {
            "Try panel ideas or adjust Proportions."
        }
        MakeCanvasMode::Ready if view_state.simple_box_make_baseline => {
            "Try box ideas or adjust box."
        }
        MakeCanvasMode::Ready => "Try ideas, focus a part, or tune controls.",
        MakeCanvasMode::Error => "The current asset needs attention.",
    }
}

pub(super) fn make_canvas_next_action_hint(
    mode: &MakeCanvasMode,
    focused_part_label: Option<&str>,
    selected_comparison_visible: bool,
    profile_kind: MakeProfileKind,
) -> String {
    match (mode, focused_part_label, selected_comparison_visible) {
        (MakeCanvasMode::NoAsset, _, _) => {
            "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, or Panel with Knob first."
                .to_owned()
        }
        (MakeCanvasMode::PreparingAsset, _, _) => {
            "Wait for the model and preview to finish preparing.".to_owned()
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, Some(part), _) => {
            format!(
                "Watch this area for new {} ideas.",
                singular_part_copy(part).to_ascii_lowercase()
            )
        }
        (MakeCanvasMode::GeneratingFocusedPartIdeas, None, _) => {
            "Watch this area for new focused ideas.".to_owned()
        }
        (MakeCanvasMode::GeneratingWholeAssetIdeas, _, _) => {
            "Watch this area for new ideas.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if profile_kind.is_lidded_box() => {
            "Use this box, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if profile_kind.is_hinged_panel() => {
            "Use this panel, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if profile_kind.is_handled_panel() => {
            "Use this panel, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if profile_kind.is_flat_panel_primitive() => {
            "Use this panel, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) if profile_kind.simple_clay_make_baseline() => {
            "Use this idea, or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, true) => {
            "Compare the selected idea, then use it or reject it.".to_owned()
        }
        (MakeCanvasMode::ReviewingIdeas, _, false) => {
            "Select an idea to compare it against the current asset.".to_owned()
        }
        (MakeCanvasMode::FocusedPart, _, _) if profile_kind.direct_primitive_workflow() => {
            if profile_kind.is_panel_with_knob() {
                "Adjust properties, Add to Pack, or Export current asset.".to_owned()
            } else {
                "Adjust dimensions, Add to Pack, or Export current primitive.".to_owned()
            }
        }
        (MakeCanvasMode::FocusedPart, Some(part), _) => {
            format!(
                "Try {} ideas, lock this part, or clear focus.",
                singular_part_copy(part).to_ascii_lowercase()
            )
        }
        (MakeCanvasMode::FocusedPart, None, _) => {
            "Try focused ideas, lock this part, or clear focus.".to_owned()
        }
        (MakeCanvasMode::PackDrawerOpen, _, _) => {
            "Review pack members or export the pack.".to_owned()
        }
        (MakeCanvasMode::ExportDrawerOpen, _, _) => {
            "Choose an export option when readiness is clear.".to_owned()
        }
        (MakeCanvasMode::Error, _, _) => "Resolve the local issue before continuing.".to_owned(),
        (MakeCanvasMode::Ready, _, _) if profile_kind.direct_primitive_workflow() => {
            if profile_kind.is_panel_with_knob() {
                "Adjust properties, Add to Pack, or Export current asset.".to_owned()
            } else {
                "Adjust dimensions, Add to Pack, or Export current primitive.".to_owned()
            }
        }
        (MakeCanvasMode::Ready, _, _) if profile_kind.is_lidded_box() => {
            "Try lidded box ideas, adjust lid seam or proportions, Add to Pack, or Export."
                .to_owned()
        }
        (MakeCanvasMode::Ready, _, _) if profile_kind.is_hinged_panel() => {
            "Try hinged panel ideas, adjust hinge edge or proportions, Add to Pack, or Export."
                .to_owned()
        }
        (MakeCanvasMode::Ready, _, _) if profile_kind.is_handled_panel() => {
            "Try handled panel ideas, adjust handle or proportions, Add to Pack, or Export."
                .to_owned()
        }
        (MakeCanvasMode::Ready, _, _) if profile_kind.is_flat_panel_primitive() => {
            "Try panel ideas, adjust Proportions or Edge Softness, Add to Pack, or Export."
                .to_owned()
        }
        (MakeCanvasMode::Ready, _, _) if profile_kind.simple_clay_make_baseline() => {
            "Try box ideas, adjust box, Add to Pack, or Export.".to_owned()
        }
        (MakeCanvasMode::Ready, _, _) => {
            "Try ideas, focus a part, add to pack, or export.".to_owned()
        }
    }
}

pub(super) fn product_panel_message(message: &str, fallback: &str) -> String {
    let trimmed = message.trim();
    let lowercase = trimmed.to_ascii_lowercase();
    let raw_markers = [
        "\\",
        "/",
        "::",
        "_",
        "members.",
        "document",
        "catalog",
        "schema",
        "validation",
        "diagnostic",
        "recipe",
    ];
    if trimmed.is_empty()
        || crate::foundry::ui::copy::first_forbidden_product_term(trimmed).is_some()
        || raw_markers.iter().any(|marker| lowercase.contains(marker))
    {
        fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

pub(super) fn direction_board_count_label(
    count: usize,
    simple_box_make_baseline: bool,
    lidded_box_baseline: bool,
    flat_panel_baseline: bool,
    hinged_panel_baseline: bool,
    handled_panel_baseline: bool,
) -> String {
    if count == 0 {
        if lidded_box_baseline {
            "Try lidded box ideas.".to_owned()
        } else if handled_panel_baseline {
            "Try handled panel ideas.".to_owned()
        } else if hinged_panel_baseline {
            "Try hinged panel ideas.".to_owned()
        } else if flat_panel_baseline {
            "Try panel ideas.".to_owned()
        } else if simple_box_make_baseline {
            "Try box ideas.".to_owned()
        } else {
            "Try ideas from the current asset.".to_owned()
        }
    } else {
        format!("Found {count} clear ideas")
    }
}

pub(super) fn make_busy_asset_noun(asset_name: &str) -> &'static str {
    let lower = asset_name.to_ascii_lowercase();
    if lower.contains("box") {
        "box"
    } else if lower.contains("panel") {
        "panel"
    } else {
        "asset"
    }
}

pub(super) fn singular_part_copy(label: &str) -> &str {
    match label {
        "Body" => "Body",
        other => other.trim_end_matches('s'),
    }
}

pub(super) fn singular_title_case_part_label(label: &str) -> String {
    singular_part_copy(label).to_owned()
}
