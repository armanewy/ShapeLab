//! Direct Make to AuthoringOp bridge.

use shape_asset::{AssetRecipe, ParameterId, get_scalar};
use shape_authoring::{
    AuthoringOp, AuthoringOpLog, AuthoringOpLogEntry, AuthoringOpSource, ReplayValidationReport,
    apply_authoring_op,
};
use shape_foundry::{ControlValue, FoundryCommand, FoundryCompilationOutput};

const BOX_PRIMITIVE_FAMILY_ID: &str = "box_primitive";
const BOX_WIDTH_CONTROL_ID: &str = "width";
const BOX_WIDTH_PARAMETER_SUFFIX: &str = "geometry.rounded_box.half_extents.x";

/// Replayable breadcrumb produced by the first Direct Make authoring bridge.
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
    /// One-entry AuthoringOp log.
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
    if output.catalog.customizer_profile.family_id != BOX_PRIMITIVE_FAMILY_ID {
        return None;
    }
    let FoundryCommand::SetControl { control_id, value } = command else {
        return None;
    };
    if control_id != BOX_WIDTH_CONTROL_ID {
        return None;
    }
    let ControlValue::Scalar(control_value) = value else {
        return None;
    };
    if !control_value.is_finite() {
        return None;
    }

    box_width_authoring_breadcrumb(&output.recipe, *control_value)
}

fn box_width_authoring_breadcrumb(
    recipe: &AssetRecipe,
    control_value: f32,
) -> Option<DirectMakeAuthoringBreadcrumb> {
    let authored_recipe_value = control_value * 0.5;
    let (parameter, path) = box_width_parameter(recipe)?;
    let before_value = get_scalar(recipe, &path).ok()?;
    let op = AuthoringOp::SetProperty {
        parameter,
        path: path.clone(),
        value: authored_recipe_value,
    };
    let outcome = apply_authoring_op(recipe, op, AuthoringOpSource::UserControl).ok()?;
    let after_value = get_scalar(&outcome.recipe, &path).ok()?;
    let mut log = AuthoringOpLog::new("direct-make-box-width");
    log.entries.push(outcome.entry.clone());

    Some(DirectMakeAuthoringBreadcrumb {
        control_id: BOX_WIDTH_CONTROL_ID.to_owned(),
        property_id: "box.width".to_owned(),
        parameter,
        path,
        requested_control_value: control_value,
        authored_recipe_value,
        before_value,
        after_value,
        log,
        entry: outcome.entry,
        validation_report: outcome.report,
    })
}

fn box_width_parameter(recipe: &AssetRecipe) -> Option<(ParameterId, String)> {
    recipe
        .parameters
        .iter()
        .find(|(_, descriptor)| descriptor.path.ends_with(BOX_WIDTH_PARAMETER_SUFFIX))
        .map(|(parameter, descriptor)| (*parameter, descriptor.path.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_authoring::{AuthoringEffect, coalesce_set_property_drag, replay_authoring_log};

    #[test]
    fn direct_make_authoring_bridge_ignores_non_box_commands() {
        let output = test_output();
        let command = FoundryCommand::SetControl {
            control_id: "depth".to_owned(),
            value: ControlValue::Scalar(2.2),
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
                shape_authoring::DragSample {
                    t_millis: 0,
                    value: 1.0,
                },
                shape_authoring::DragSample {
                    t_millis: 16,
                    value: 1.1,
                },
            ],
        )
        .expect("coalesces");

        assert_eq!(coalesced.first.value, 1.0);
        assert_eq!(coalesced.last.value, 1.1);
    }

    fn test_output() -> FoundryCompilationOutput {
        let fixture = shape_foundry_catalog::box_primitive::fixture_catalog();
        shape_foundry::compile_foundry_document(&fixture.document, &fixture).expect("compiles")
    }
}
