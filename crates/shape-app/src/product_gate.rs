//! Headless product UI gate for the Visual Foundry app shell.

use shape_foundry::{ControlKind, compile_foundry_document};
use shape_foundry_catalog::{FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels};

use crate::foundry::{
    app::rendered_action_labels_for_default_shell,
    app::{
        default_app_launches_on_home, default_product_home_profile_count,
        developer_preview_product_home_profile_count, installed_product_kit_count,
        product_visible_strings_for_default_shell,
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
    /// Number of starting points visible in the default novice home screen.
    ///
    /// This can be lower than the installed kit count because historical
    /// evidence and preview-only profiles stay hidden from the default Choose
    /// page.
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
    /// Whether active primitive Make copy has retired product-visible variation UI.
    pub active_variation_ui_retired: bool,
    /// Forbidden active primitive terms found after allowed negative caveats are removed.
    pub active_primitive_forbidden_terms_found: Vec<&'static str>,
    /// Whether default product copy exposes direct primitive property controls.
    pub direct_property_gate: bool,
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
    /// Direct primitive property labels exposed by the default product shell.
    pub direct_property_labels: Vec<&'static str>,
}

impl ProductUiGateReport {
    /// True when every automated Wave 31.5 product UI gate passes.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.legacy_surfaces_present
            && !self.startup_blank
            && !self.default_advanced_recipe_visible
            && !self.default_raw_technical_terms_visible
            && self.product_home_profiles > 0
            && self.product_home_profiles <= self.installed_kit_count
            && self.installed_kit_count == self.core_profiles.len()
            && self.developer_preview_kit_count == self.installed_kit_count
            && self.active_variation_ui_retired
            && self.active_primitive_forbidden_terms_found.is_empty()
            && self.direct_property_gate
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
    let retired_terms = [
        "try ideas",
        "try box ideas",
        "try lidded box ideas",
        "try panel ideas",
        "try hinged panel ideas",
        "generated ideas",
        "variation mode",
        "candidate",
        "survivor",
        "use this idea",
    ];
    let active_variation_ui_retired = retired_terms
        .iter()
        .all(|term| !joined_lower.contains(term));
    let active_primitive_forbidden_terms_found =
        active_primitive_forbidden_terms_in_default_copy(&visible_strings);
    let direct_property_labels = vec![
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
    ];
    let direct_property_gate = direct_property_labels
        .iter()
        .all(|label| visible_strings.contains(label))
        && visible_strings.contains(&"Adjust dimensions")
        && visible_strings.contains(&"Export current primitive");
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
        && visible_strings.contains(&"Prepare the current asset before exporting.");
    let disabled_states_have_reasons = [
        "Choose a starting point or open a project first.",
        "Prepare the current model first.",
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
        active_variation_ui_retired,
        active_primitive_forbidden_terms_found,
        direct_property_gate,
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
        direct_property_labels,
    })
}

fn active_primitive_forbidden_terms_in_default_copy(visible_strings: &[&str]) -> Vec<&'static str> {
    let mut joined = visible_strings.join("\n").to_ascii_lowercase();
    for allowed in [
        "not a textured, rigged, animated, or game-ready package",
        "not textured",
        "not rigged",
        "not animated",
        "not game-ready",
    ] {
        joined = joined.replace(allowed, "");
    }
    [
        "generated ideas",
        "candidate",
        "survivor",
        "rejected candidate",
        "variation mode",
        "uv",
        "texture",
        "rigging",
        "animation",
        "game-ready",
        "vertex",
        "face",
        "topology",
        "raw transform",
        "boolean",
        "mesh edit",
    ]
    .into_iter()
    .filter(|term| joined.contains(term))
    .collect()
}

fn core_profile_fixtures() -> Vec<(&'static str, FoundryFixtureCatalog)> {
    built_in_fixture_catalogs_with_labels()
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
        assert!(report.product_home_profiles > 0);
        assert_eq!(report.product_home_profiles, 6);
        assert_eq!(report.installed_kit_count, 7);
        assert_eq!(report.developer_preview_kit_count, 7);
        assert!(report.active_variation_ui_retired);
        assert!(
            report.active_primitive_forbidden_terms_found.is_empty(),
            "{:?}",
            report.active_primitive_forbidden_terms_found
        );
        assert!(report.direct_property_gate);
        assert_eq!(
            report.direct_property_labels,
            vec![
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
                "Knob Vertical Position"
            ]
        );
        assert!(report.manual_gate_required);
    }
}
