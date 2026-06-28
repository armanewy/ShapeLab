//! Make canvas panel copy and small view helpers.

use super::directions::DirectionPartGroup;

/// Primary Make canvas workflow tabs.
pub(crate) const MAKE_WORKFLOW_STEPS: [&str; 2] = ["Choose", "Make"];

/// Default whole-asset generation action.
pub(crate) const TRY_WHOLE_ASSET_IDEAS: &str = "Try ideas";

/// Product-visible selected candidate actions.
pub(crate) const SELECTED_COMPARISON_ACTIONS: [&str; 2] = ["Use this idea", "Reject"];

/// Return the active Make scope label.
#[must_use]
pub(crate) fn make_scope_label(active_group: Option<&DirectionPartGroup>) -> &str {
    active_group.map_or("Current asset", |group| group.label.as_str())
}
