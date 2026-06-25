//! Headless product UI gate for the Visual Foundry app shell.

use shape_foundry::{ControlKind, compile_foundry_document};
use shape_foundry_catalog::{FoundryFixtureCatalog, roman_bridge, scifi_crate, stylized_lamp};

use crate::foundry::{
    app::rendered_action_labels_for_default_shell,
    app::{
        default_app_launches_on_home, default_product_home_profile_count,
        developer_preview_product_home_profile_count, installed_product_kit_count,
        product_visible_strings_for_default_shell,
    },
    panels::directions::{
        DIRECTION_BOARD_MODES, VISIBLE_DIRECTION_CANDIDATE_CARDS, direction_mode_actions,
    },
    ui::copy::first_forbidden_product_term,
};

/// Computed headless evidence for the default Visual Foundry product UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductUiGateReport {
    /// Product shell shape expected by release-readiness.
    pub app_shell: &'static str,
    /// Whether removed product surfaces are still present in default product copy.
    pub legacy_surfaces_present: bool,
    /// Number of kit-backed templates visible in the default novice home screen.
    pub product_home_profiles: usize,
    /// Number of installed built-in kits available to preview/developer catalogs.
    pub installed_kit_count: usize,
    /// Number of built-in kits visible when preview catalog mode is enabled.
    pub developer_preview_kit_count: usize,
    /// Whether startup would land on a blank/non-actionable surface.
    pub startup_blank: bool,
    /// Whether Advanced Recipe is exposed in the default product path.
    pub default_advanced_recipe_visible: bool,
    /// Whether default product copy exposes raw technical terms.
    pub default_raw_technical_terms_visible: bool,
    /// Whether the directions board reserves six whole-model candidate slots and five modes.
    pub directions_board_gate: bool,
    /// Whether core profiles compile and expose novice controls.
    pub customize_deck_gate: bool,
    /// Whether pack actions and readiness reasons are represented in product copy.
    pub pack_gate: bool,
    /// Whether export readiness states are represented in product copy.
    pub export_gate: bool,
    /// Whether disabled/default states include plain-language reasons.
    pub disabled_states_have_reasons: bool,
    /// Human visual inspection is still required for release readiness.
    pub manual_gate_required: bool,
    /// Count of default product-visible strings audited.
    pub product_visible_string_count: usize,
    /// Count of rendered action labels covered by the product copy audit.
    pub rendered_action_label_count: usize,
    /// Whether all rendered action labels are included in the product copy audit.
    pub rendered_action_labels_audited: bool,
    /// Forbidden term findings in default product-visible copy.
    pub forbidden_terms_found: Vec<ProductUiForbiddenTermFinding>,
    /// Core profile compile/start evidence.
    pub core_profiles: Vec<ProductUiProfileGate>,
    /// Direction action labels exposed by the default board.
    pub direction_modes: Vec<&'static str>,
    /// Reserved whole-model candidate card slots.
    pub direction_candidate_slots: usize,
}

impl ProductUiGateReport {
    /// True when every automated Wave 31.5 product UI gate passes.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.legacy_surfaces_present
            && !self.startup_blank
            && !self.default_advanced_recipe_visible
            && !self.default_raw_technical_terms_visible
            && self.product_home_profiles == 0
            && self.installed_kit_count == 16
            && self.developer_preview_kit_count == 16
            && self.directions_board_gate
            && self.customize_deck_gate
            && self.pack_gate
            && self.export_gate
            && self.disabled_states_have_reasons
            && self.rendered_action_labels_audited
            && self.core_profiles.iter().all(ProductUiProfileGate::passed)
    }
}

/// One forbidden term found in product-visible copy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductUiForbiddenTermFinding {
    /// Forbidden implementation term.
    pub term: &'static str,
    /// Product-visible string that contained it.
    pub visible_string: String,
}

/// Compile/start evidence for one core Visual Foundry profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductUiProfileGate {
    /// Built-in fixture slug.
    pub slug: String,
    /// Human-facing profile label.
    pub label: String,
    /// Whether the profile compiled.
    pub compiled: bool,
    /// Whether compiling produced visible asset geometry for the main shell.
    pub reaches_main_shell: bool,
    /// Number of novice-facing controls in the resolved profile.
    pub primary_control_count: usize,
    /// Number of controls with whole-model option tiles.
    pub option_control_count: usize,
    /// Final triangle count from the compile artifact.
    pub triangle_count: usize,
}

impl ProductUiProfileGate {
    /// True when the profile can start into a useful default product flow.
    #[must_use]
    pub fn passed(&self) -> bool {
        self.compiled
            && self.reaches_main_shell
            && (1..=7).contains(&self.primary_control_count)
            && self.triangle_count > 0
    }
}

/// Run the deterministic product UI integration gate.
pub fn visual_foundry_product_ui_gate_report() -> Result<ProductUiGateReport, String> {
    let visible_strings = product_visible_strings_for_default_shell();
    let forbidden_terms_found = visible_strings
        .iter()
        .filter_map(|visible| {
            first_forbidden_product_term(visible).map(|term| ProductUiForbiddenTermFinding {
                term,
                visible_string: (*visible).to_owned(),
            })
        })
        .collect::<Vec<_>>();
    let joined_lower = visible_strings.join("\n").to_ascii_lowercase();
    let rendered_action_labels = rendered_action_labels_for_default_shell();
    let rendered_action_labels_audited = rendered_action_labels.iter().all(|label| {
        visible_strings.contains(label) && first_forbidden_product_term(label).is_none()
    });
    let legacy_surfaces_present = [
        "legacy",
        "implicit",
        "asset modeling lab",
        "modeling workspace",
        "from existing recipe",
    ]
    .into_iter()
    .any(|term| joined_lower.contains(term));
    let default_advanced_recipe_visible = joined_lower.contains("advanced recipe");
    let direction_mode_actions = direction_mode_actions(None, 0, None);
    let direction_modes = direction_mode_actions
        .iter()
        .map(|action| action.label)
        .collect::<Vec<_>>();
    let directions_board_gate = VISIBLE_DIRECTION_CANDIDATE_CARDS == 6
        && DIRECTION_BOARD_MODES.len() == 5
        && direction_mode_actions.len() == 5
        && direction_mode_actions.iter().all(|action| {
            action.request.result_count == VISIBLE_DIRECTION_CANDIDATE_CARDS
                && action.request.proposal_count >= 24
        });
    let product_home_profiles = default_product_home_profile_count();
    let installed_kit_count = installed_product_kit_count();
    let developer_preview_kit_count = developer_preview_product_home_profile_count();
    let core_profiles = core_profile_fixtures()
        .into_iter()
        .map(|(label, fixture)| profile_gate(label, fixture))
        .collect::<Result<Vec<_>, _>>()?;
    let customize_deck_gate = core_profiles.iter().all(ProductUiProfileGate::passed);
    let pack_gate = visible_strings.contains(&"Add Current Asset")
        && visible_strings.contains(&"Export Pack")
        && visible_strings.contains(&"Add at least one asset before exporting a pack.");
    let export_gate = visible_strings.contains(&"Export ready")
        && visible_strings.contains(&"Current asset ready")
        && visible_strings.contains(&"Build the current asset before exporting.");
    let disabled_states_have_reasons = [
        "Choose a template or open a project first.",
        "Build the current model first.",
        "Add at least one asset before exporting a pack.",
        "This option is not available right now.",
    ]
    .iter()
    .all(|reason| visible_strings.contains(reason));

    Ok(ProductUiGateReport {
        app_shell: "direct_visual_foundry",
        legacy_surfaces_present,
        product_home_profiles,
        installed_kit_count,
        developer_preview_kit_count,
        startup_blank: !default_app_launches_on_home(),
        default_advanced_recipe_visible,
        default_raw_technical_terms_visible: !forbidden_terms_found.is_empty(),
        directions_board_gate,
        customize_deck_gate,
        pack_gate,
        export_gate,
        disabled_states_have_reasons,
        manual_gate_required: true,
        product_visible_string_count: visible_strings.len(),
        rendered_action_label_count: rendered_action_labels.len(),
        rendered_action_labels_audited,
        forbidden_terms_found,
        core_profiles,
        direction_modes,
        direction_candidate_slots: VISIBLE_DIRECTION_CANDIDATE_CARDS,
    })
}

fn core_profile_fixtures() -> Vec<(&'static str, FoundryFixtureCatalog)> {
    vec![
        ("Roman Timber Bridge", roman_bridge::fixture_catalog()),
        ("Sci-Fi Industrial Crate", scifi_crate::fixture_catalog()),
        ("Stylized Furniture Lamp", stylized_lamp::fixture_catalog()),
    ]
}

fn profile_gate(
    label: &'static str,
    fixture: FoundryFixtureCatalog,
) -> Result<ProductUiProfileGate, String> {
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| format!("{label} did not compile: {error:?}"))?;
    let primary_control_count = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
        .count();
    let option_control_count = output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| {
            matches!(
                control.kind,
                ControlKind::ChoiceGallery { .. } | ControlKind::ProviderGallery { .. }
            )
        })
        .count();

    let triangle_count = output.artifact.statistics.triangle_count as usize;

    Ok(ProductUiProfileGate {
        slug: fixture.slug,
        label: label.to_owned(),
        compiled: true,
        reaches_main_shell: triangle_count > 0,
        primary_control_count,
        option_control_count,
        triangle_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_ui_gate_passes_for_default_visual_foundry_shell() {
        let report = visual_foundry_product_ui_gate_report().expect("product UI gate report");

        assert!(report.passed(), "{report:#?}");
        assert_eq!(report.app_shell, "direct_visual_foundry");
        assert_eq!(report.product_home_profiles, 0);
        assert_eq!(report.installed_kit_count, 16);
        assert_eq!(report.developer_preview_kit_count, 16);
        assert_eq!(report.direction_candidate_slots, 6);
        assert_eq!(
            report.direction_modes,
            vec!["Refine", "Explore", "Silhouette", "Structure", "Detail"]
        );
        assert!(report.manual_gate_required);
    }
}
