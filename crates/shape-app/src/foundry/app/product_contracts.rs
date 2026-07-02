use super::*;

pub(crate) fn default_product_home_profile_count() -> usize {
    product_home_profiles(false).len()
}

pub(crate) fn developer_preview_product_home_profile_count() -> usize {
    product_home_profiles(true).len()
}

pub(crate) fn installed_product_kit_count() -> usize {
    built_in_kit_card_views().len()
}

pub(crate) fn product_visible_strings_for_default_shell() -> Vec<&'static str> {
    let mut strings = vec![
        "Shape Lab",
        "Visual Foundry",
        "Choose",
        "Make",
        "Export",
        "Start with Box Primitive",
        "Lidded Box",
        "Flat Panel Primitive",
        "Sphere Primitive",
        "Hinged Panel",
        "Panel with Knob",
        "Project",
        "Open Project",
        "Save Project",
        "Save Project As",
        "Start Another Asset",
        "History",
        "Recent Projects",
        "Start",
        ACTION_ADJUST_DIMENSIONS,
        ACTION_EDIT_BOX_PRIMITIVE,
        ACTION_EDIT_FLAT_PANEL,
        ACTION_EDIT_SPHERE_PRIMITIVE,
        ACTION_EDIT_LIDDED_BOX,
        ACTION_EDIT_HINGED_PANEL,
        ACTION_EDIT_PANEL_KNOB,
        "Add to Pack",
        "Open Pack",
        "Open Export",
        ACTION_EXPORT_CURRENT_ASSET,
        ACTION_EXPORT_CURRENT_PRIMITIVE,
        ACTION_CHOOSE_TEMPLATE,
        "Preparing model",
        "Rendering preview",
        "Ready for adjustments",
        PREVIEW_UPDATING_REASON,
        PREPARATION_TIMEOUT_MESSAGE,
        ASSET_PREPARING_REASON,
        STALE_RESULT_WARNING,
        "Try again when you are ready.",
        "Current Asset",
        "Save",
        "Undo",
        "Choose starting point",
        "Not saved",
        "Saved",
        "Unsaved",
        "Unsaved changes",
        "Ready",
        "Working",
        "Model ready",
        "Preparing model",
        "Choose starting point",
        "Ready",
        "Preview available",
        "Preparing",
        "Preview",
        "Pack: 0 assets",
        "Export complete",
        "Pack export complete",
        "Current asset ready",
        "Needs a model first",
        HOME_SUBTITLE,
        BOX_PRIMITIVE_HOME_SUBTITLE,
        HOME_CONTROL_COPY,
        "Primitives",
        "Derived from Box Primitive",
        "Derived from Flat Panel Primitive",
        "Preset",
        "Pick a primitive, or choose a derived starting point under its source.",
        "Starting points are grouped by provenance so the source stays clear.",
        "Derived from Box Primitive + Lid Seam.",
        "Derived from Flat Panel Primitive + Hinge Edge.",
        "Derived from Flat Panel Primitive + Sphere attachment.",
        "Knob-like Form",
        "Preset from Sphere Primitive properties.",
        "A simple box with a visible lid seam.",
        "You can vary proportions, edge softness, and lid seam.",
        "One upright clay panel with readable width, height, and thickness.",
        "One closed round clay volume with readable dimensions and flattening.",
        "One upright clay panel with a visible hinge edge.",
        "You can vary proportions, edge softness, and hinge edge.",
        "One upright clay panel with a bounded knob-like sphere form attached through a safe anchor.",
        "You can adjust panel size, knob form, and bounded knob position.",
        "Start with Box Primitive.",
        "Choose a starting point",
        "Preview building",
        "No matching starting point",
        "Make asset",
        "Adjust dimensions, Add to Pack, or Export current primitive.",
        "Adjust properties, Add to Pack, or Export current asset.",
        "Direct properties ready",
        "Width",
        "Depth",
        "Height",
        "Thickness",
        "Edge Softness",
        "Front Flatten",
        "Back Flatten",
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
        "Lid Seam",
        "Hinge Edge",
        VIEW_ORBIT_LABEL,
        VIEW_RESET_LABEL,
        VIEW_AXIS_LABEL,
        "Current Asset",
        "Current asset",
        "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before making changes.",
        "Project history",
        "Review previous project steps and branch from a saved point.",
        "saved step(s)",
        "Project step",
        "Tune the main box controls and lock the settings you want to keep.",
        "Choose an asset first",
        "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before adjusting.",
        "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, or Panel with Knob first.",
        "Make it yours",
        "No quick controls yet",
        "This asset has no quick controls yet.",
        "Preview",
        "Controls width.",
        "Controls depth.",
        "Controls height.",
        "Controls thickness.",
        "Controls corner softness.",
        "Controls front flattening.",
        "Controls back flattening.",
        "Keeps knob position within the safe anchor area.",
        "Keep within authored safe range.",
        "More options",
        "This option is not available right now.",
        "This control is locked.",
        "Export ready",
        BOX_PRIMITIVE_EXPORT_TITLE,
        BOX_PRIMITIVE_EXPORT_DETAIL,
        LIDDED_BOX_EXPORT_TITLE,
        LIDDED_BOX_EXPORT_DETAIL,
        FLAT_PANEL_EXPORT_TITLE,
        FLAT_PANEL_EXPORT_DETAIL,
        HINGED_PANEL_EXPORT_TITLE,
        HINGED_PANEL_EXPORT_DETAIL,
        SPHERE_PRIMITIVE_EXPORT_TITLE,
        SPHERE_PRIMITIVE_EXPORT_DETAIL,
        PANEL_KNOB_EXPORT_TITLE,
        PANEL_KNOB_EXPORT_DETAIL,
        BOX_PRIMITIVE_EXPORT_LIMITATION,
        "Current Asset",
        "Export options",
        "Pack members",
        "Export this asset here, or export the prepared pack from the Pack drawer.",
        "Export the current asset as an individual result.",
        "Pack preview",
        "Collect coherent variants before exporting a set.",
        "Add Current Asset",
        "Export Pack",
        "Pack is empty",
        "Add the current asset to start a pack.",
        "assets in pack",
        "Pack asset",
        "Add assets to export",
        "Needs attention",
        "Contact sheet",
        "Pack needs attention before export.",
        "Resolve pack warnings before export.",
        "All assets are ready for pack export.",
        NEED_PROJECT_REASON,
        NEED_SAVE_LOCATION_REASON,
        NEED_MODEL_REASON,
        NEED_HISTORY_REASON,
        NEED_DIRECTION_REASON,
        NEED_RESET_REASON,
        NEED_PACK_MEMBER_REASON,
        "This model has no quick controls yet.",
        "Prepare the current asset before exporting.",
        "No pack workspace is open.",
        PREVIEW_PREPARING_REASON,
        "Primary control",
        "Starting point is not available",
        "Open a saved project, or enable the clay starting points.",
        "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before exporting.",
        "Choose Box Primitive, Lidded Box, Flat Panel Primitive, Sphere Primitive, Hinged Panel, Panel with Knob, or open a project before starting a pack.",
    ];
    strings.extend(RENDERED_ACTION_LABELS);
    for step in WORKFLOW_STEPS {
        strings.push(step.label);
        strings.push(step.detail);
    }
    strings
}

pub(crate) fn rendered_action_labels_for_default_shell() -> &'static [&'static str] {
    &RENDERED_ACTION_LABELS
}

pub(crate) fn core_make_action_specs_for_default_shell() -> Vec<ActionSpec<'static>> {
    vec![
        ActionSpec::enabled(ACTION_START, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_ADJUST_DIMENSIONS, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_EDIT_BOX_PRIMITIVE, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_EDIT_FLAT_PANEL, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_EDIT_SPHERE_PRIMITIVE, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_KNOB_LIKE_FORM, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_EDIT_LIDDED_BOX, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_EDIT_HINGED_PANEL, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_TRY_AGAIN, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_RETRY_PREPARATION, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_UPDATE_PREVIEW, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_UNLOCK_CONTROLS, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_RESET, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_CHOOSE_ANOTHER_TEMPLATE, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_ADD_TO_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_OPEN_PACK, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_EXPORT, ButtonTone::Primary),
        ActionSpec::enabled(ACTION_EXPORT_CURRENT_PRIMITIVE, ButtonTone::Secondary),
        ActionSpec::enabled(ACTION_CLOSE_DRAWER, ButtonTone::Secondary),
    ]
}

pub(crate) fn direction_mode_actions_for_panel() -> Vec<directions::DirectionModeAction> {
    directions::direction_mode_actions(None, 0, None)
}

pub(crate) fn direction_variation_mode_actions_for_panel(
    document: Option<&FoundryAssetDocument>,
) -> Vec<directions::DirectionVariationModeAction> {
    let Some(document) = document else {
        return directions::direction_variation_mode_actions(
            &VariationIntent::default(),
            0,
            None,
            None,
            &[],
        );
    };
    let part_groups = directions::direction_part_groups_for_document(document);
    let surface_capability =
        built_in_surface_capability_for_profile(&document.customizer_profile_ref.stable_id);
    directions::direction_variation_mode_actions(
        &document.variation_state.intent,
        document.seed,
        None,
        Some(&surface_capability),
        &part_groups,
    )
}

pub(crate) fn default_app_launches_on_home() -> bool {
    let app = FoundryDesktopApp::default();
    app.tab == FoundryTab::Home && app.state.document.is_none()
}
