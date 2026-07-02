//! Direct Make to AuthoringOp bridge.

use std::collections::BTreeMap;

use orchard_asset::{AssetRecipe, ParameterId, get_scalar};
use orchard_authoring::{
    AuthoringOp, AuthoringOpLog, AuthoringOpLogEntry, AuthoringOpSource, ReplayValidationReport,
    apply_authoring_op,
};
use orchard_foundry::{
    ControlValue, FoundryCatalogError, FoundryCatalogResolver, FoundryCommand,
    FoundryCompilationOutput, FoundryResolvedCatalog, apply_foundry_command,
    compile_foundry_document,
};

const DIRECT_AUTHORING_FAMILY_IDS: &[&str] = &[
    "box_primitive",
    "flat_panel_primitive",
    "sphere_primitive",
    "panel_with_knob",
];
const SCALAR_EPSILON: f32 = 0.000_001;

/// One scalar parameter changed by a Direct Make authoring operation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectMakeChangedParameter {
    /// Target recipe parameter.
    pub parameter: ParameterId,
    /// Semantic recipe path used by the authoring operation.
    pub path: String,
    /// Value before the authoring operation.
    pub before_value: f32,
    /// Value after the authoring operation.
    pub after_value: f32,
}

/// Replayable breadcrumb produced by the Direct Make authoring bridge.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirectMakeAuthoringBreadcrumb {
    /// Product control that emitted the authoring operation.
    pub control_id: String,
    /// Stable semantic property ID.
    pub property_id: String,
    /// Target recipe parameter.
    pub parameter: ParameterId,
    /// Semantic recipe path used by the authoring operation.
    pub path: String,
    /// Control value requested by the UI.
    pub requested_control_value: f32,
    /// Scalar value written into the compiled recipe.
    pub authored_recipe_value: f32,
    /// Value before the authoring operation.
    pub before_value: f32,
    /// Value after the authoring operation.
    pub after_value: f32,
    /// All scalar parameters changed by this direct edit.
    pub changed_parameters: Vec<DirectMakeChangedParameter>,
    /// Replayable AuthoringOp log.
    pub log: AuthoringOpLog,
    /// Entry created by applying the operation.
    pub entry: AuthoringOpLogEntry,
    /// Validation report returned by the authoring lane.
    pub validation_report: ReplayValidationReport,
}

/// Produce a Direct Make authoring breadcrumb when a command is covered by v0.
#[must_use]
pub(crate) fn direct_make_authoring_breadcrumb(
    output: &FoundryCompilationOutput,
    command: &FoundryCommand,
) -> Option<DirectMakeAuthoringBreadcrumb> {
    let family_id = output.catalog.customizer_profile.family_id.as_str();
    if !DIRECT_AUTHORING_FAMILY_IDS.contains(&family_id) {
        return None;
    }
    let FoundryCommand::SetControl { control_id, value } = command else {
        return None;
    };
    let ControlValue::Scalar(control_value) = value else {
        return None;
    };
    if !control_value.is_finite() {
        return None;
    }

    if !output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .any(|control| control.id == *control_id && control.visible)
    {
        return None;
    }

    let mut next_document = output.document.clone();
    apply_foundry_command(&mut next_document, command).ok()?;
    let resolver = ResolvedCatalogSnapshotResolver::from(&output.catalog);
    let next_output = compile_foundry_document(&next_document, &resolver).ok()?;
    let changed_parameters = changed_scalar_parameters(&output.recipe, &next_output.recipe);
    let first = changed_parameters.first()?;

    let mut replay_recipe = output.recipe.clone();
    let mut log = AuthoringOpLog::new(format!(
        "direct-make-{}-{}",
        family_id.replace('_', "-"),
        control_id.replace('_', "-")
    ));
    let mut last_report = None;
    for (sequence, changed) in changed_parameters.iter().enumerate() {
        let op = AuthoringOp::SetProperty {
            parameter: changed.parameter,
            path: changed.path.clone(),
            value: changed.after_value,
        };
        let outcome =
            apply_authoring_op(&replay_recipe, op, AuthoringOpSource::UserControl).ok()?;
        replay_recipe = outcome.recipe;
        let mut entry = outcome.entry;
        entry.sequence = sequence as u64;
        log.entries.push(entry);
        last_report = Some(outcome.report);
    }
    let entry = log.entries.first()?.clone();

    Some(DirectMakeAuthoringBreadcrumb {
        control_id: control_id.clone(),
        property_id: direct_property_id(family_id, control_id),
        parameter: first.parameter,
        path: first.path.clone(),
        requested_control_value: *control_value,
        authored_recipe_value: first.after_value,
        before_value: first.before_value,
        after_value: first.after_value,
        changed_parameters,
        log,
        entry,
        validation_report: last_report?,
    })
}

fn changed_scalar_parameters(
    before: &AssetRecipe,
    after: &AssetRecipe,
) -> Vec<DirectMakeChangedParameter> {
    before
        .parameters
        .iter()
        .filter_map(|(parameter, descriptor)| {
            let before_value = get_scalar(before, &descriptor.path).ok()?;
            let after_value = after
                .parameters
                .get(parameter)
                .and_then(|after_descriptor| get_scalar(after, &after_descriptor.path).ok())?;
            ((before_value - after_value).abs() > SCALAR_EPSILON).then(|| {
                DirectMakeChangedParameter {
                    parameter: *parameter,
                    path: descriptor.path.clone(),
                    before_value,
                    after_value,
                }
            })
        })
        .collect()
}

fn direct_property_id(family_id: &str, control_id: &str) -> String {
    match family_id {
        "box_primitive" => format!("box.{control_id}"),
        "flat_panel_primitive" => format!("flat_panel.{control_id}"),
        "sphere_primitive" => format!("sphere.{control_id}"),
        "panel_with_knob" => format!("panel_with_knob.{control_id}"),
        _ => format!("{family_id}.{control_id}"),
    }
}

struct ResolvedCatalogSnapshotResolver {
    content_by_stable_id: BTreeMap<String, String>,
}

impl From<&FoundryResolvedCatalog> for ResolvedCatalogSnapshotResolver {
    fn from(catalog: &FoundryResolvedCatalog) -> Self {
        Self {
            content_by_stable_id: catalog
                .resolved_content
                .values()
                .map(|content| {
                    (
                        content.content_ref.stable_id.clone(),
                        content.canonical_json.clone(),
                    )
                })
                .collect(),
        }
    }
}

impl FoundryCatalogResolver for ResolvedCatalogSnapshotResolver {
    fn resolve_catalog_content(
        &self,
        content_ref: &orchard_foundry::CatalogContentRef,
    ) -> Result<String, FoundryCatalogError> {
        self.content_by_stable_id
            .get(&content_ref.stable_id)
            .cloned()
            .ok_or_else(|| FoundryCatalogError::MissingContent {
                content_ref: content_ref.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchard_authoring::{AuthoringEffect, coalesce_set_property_drag, replay_authoring_log};

    #[test]
    fn direct_make_authoring_bridge_ignores_non_scalar_commands() {
        let output = test_output();
        let command = FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: ControlValue::Choice("wide".to_owned()),
        };

        assert!(direct_make_authoring_breadcrumb(&output, &command).is_none());
    }

    #[test]
    fn direct_make_authoring_bridge_replays_box_width_log() {
        let output = test_output();
        let command = FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: ControlValue::Scalar(2.2),
        };
        let breadcrumb = direct_make_authoring_breadcrumb(&output, &command).expect("breadcrumb");

        assert_eq!(breadcrumb.property_id, "box.width");
        assert_eq!(breadcrumb.entry.effect, AuthoringEffect::SetProperty);
        assert_eq!(breadcrumb.requested_control_value, 2.2);
        assert_eq!(breadcrumb.authored_recipe_value, 1.1);
        assert_eq!(breadcrumb.after_value, 1.1);
        assert_eq!(breadcrumb.changed_parameters.len(), 1);
        assert!(breadcrumb.validation_report.accepted);
        assert_eq!(breadcrumb.log.entries.len(), 1);

        let replayed = replay_authoring_log(&output.recipe, &breadcrumb.log).expect("replays");
        assert_eq!(get_scalar(&replayed.recipe, &breadcrumb.path).unwrap(), 1.1);
    }

    #[test]
    fn direct_make_authoring_bridge_can_coalesce_drag_samples() {
        let output = test_output();
        let command = FoundryCommand::SetControl {
            control_id: "width".to_owned(),
            value: ControlValue::Scalar(2.2),
        };
        let breadcrumb = direct_make_authoring_breadcrumb(&output, &command).expect("breadcrumb");
        let coalesced = coalesce_set_property_drag(
            breadcrumb.parameter,
            breadcrumb.path.clone(),
            &[
                orchard_authoring::DragSample {
                    t_millis: 0,
                    value: 1.0,
                },
                orchard_authoring::DragSample {
                    t_millis: 16,
                    value: 1.1,
                },
            ],
        )
        .expect("coalesces");

        assert_eq!(coalesced.first.value, 1.0);
        assert_eq!(coalesced.last.value, 1.1);
    }

    #[test]
    fn direct_make_authoring_bridge_covers_active_direct_controls() {
        let cases = [
            (
                orchard_foundry_catalog::box_primitive::fixture_catalog(),
                vec![
                    ("width", 2.2),
                    ("depth", 1.6),
                    ("height", 1.2),
                    ("edge_softness", 0.12),
                ],
            ),
            (
                orchard_foundry_catalog::flat_panel::fixture_catalog(),
                vec![
                    ("width", 2.0),
                    ("height", 2.8),
                    ("thickness", 0.22),
                    ("edge_softness", 0.08),
                ],
            ),
            (
                orchard_foundry_catalog::sphere_primitive::fixture_catalog(),
                vec![
                    ("width", 1.2),
                    ("height", 1.3),
                    ("depth", 0.8),
                    ("front_flatten", 0.20),
                    ("back_flatten", 0.22),
                ],
            ),
            (
                orchard_foundry_catalog::panel_knob::fixture_catalog(),
                vec![
                    ("panel_width", 2.0),
                    ("panel_height", 2.5),
                    ("panel_thickness", 0.20),
                    ("panel_edge_softness", 0.10),
                    ("knob_width", 0.45),
                    ("knob_height", 0.48),
                    ("knob_depth", 0.30),
                    ("knob_front_flatten", 0.22),
                    ("knob_back_flatten", 0.24),
                    ("knob_x_offset", 0.62),
                    ("knob_y_offset", 0.58),
                ],
            ),
        ];

        for (fixture, controls) in cases {
            let output = orchard_foundry::compile_foundry_document(&fixture.document, &fixture)
                .expect("fixture compiles");
            for (control_id, value) in controls {
                let command = FoundryCommand::SetControl {
                    control_id: control_id.to_owned(),
                    value: ControlValue::Scalar(value),
                };
                let breadcrumb = direct_make_authoring_breadcrumb(&output, &command)
                    .unwrap_or_else(|| panic!("{control_id} should emit AuthoringOp breadcrumb"));
                assert_eq!(breadcrumb.control_id, control_id);
                assert_eq!(breadcrumb.entry.effect, AuthoringEffect::SetProperty);
                assert!(
                    !breadcrumb.changed_parameters.is_empty(),
                    "{control_id} should change at least one scalar parameter"
                );
                assert_eq!(
                    breadcrumb.log.entries.len(),
                    breadcrumb.changed_parameters.len(),
                    "{control_id} should log every changed scalar parameter"
                );
                let replayed =
                    replay_authoring_log(&output.recipe, &breadcrumb.log).expect("replays");
                for changed in &breadcrumb.changed_parameters {
                    assert_eq!(
                        get_scalar(&replayed.recipe, &changed.path).unwrap(),
                        changed.after_value,
                        "{control_id} replay should reproduce {}",
                        changed.path
                    );
                }
            }
        }
    }

    fn test_output() -> FoundryCompilationOutput {
        let fixture = orchard_foundry_catalog::box_primitive::fixture_catalog();
        orchard_foundry::compile_foundry_document(&fixture.document, &fixture).expect("compiles")
    }
}
