//! Gated Foundry Author Studio app-side view contracts.
//!
//! This is intentionally separate from the default Visual Foundry product
//! shell. The pro/internal authoring surface may show technical kit language;
//! the novice shell must not.

use orchard_foundry::{
    AuthorStudioStep, FoundryAuthorStudioGate, foundation_draft_fixtures,
    foundry_author_studio_shell,
};

/// Author Studio view-model row for one workflow step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryAuthorStudioStepView {
    /// 1-based step index.
    pub index: u8,
    /// Stable step ID.
    pub step_id: String,
    /// Author-facing label.
    pub label: String,
}

impl From<AuthorStudioStep> for FoundryAuthorStudioStepView {
    fn from(step: AuthorStudioStep) -> Self {
        Self {
            index: step.index,
            step_id: step.step_id,
            label: step.label,
        }
    }
}

/// Gated Author Studio view model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryAuthorStudioView {
    /// Whether the surface is reachable.
    pub available: bool,
    /// Hidden reason when unavailable.
    pub unavailable_reason: Option<String>,
    /// Workflow steps.
    pub steps: Vec<FoundryAuthorStudioStepView>,
    /// Technical authoring labels exposed only in gated mode.
    pub authoring_terms: Vec<&'static str>,
    /// Descriptor-only note for unsupported mesh/component import.
    pub descriptor_only_notice: Option<&'static str>,
    /// Gated foundation-draft import panel.
    pub foundation_draft_panel: Option<FoundryFoundationDraftPanelView>,
}

/// Gated Foundation Draft panel view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundryFoundationDraftPanelView {
    /// Internal fixture draft IDs.
    pub internal_draft_ids: Vec<String>,
    /// Supported author actions.
    pub actions: Vec<&'static str>,
    /// Catalog visibility promise.
    pub catalog_visibility_notice: &'static str,
}

/// Build a gated Author Studio view model.
#[must_use]
pub(crate) fn foundry_author_studio_view(gate: FoundryAuthorStudioGate) -> FoundryAuthorStudioView {
    let shell = foundry_author_studio_shell(gate);
    FoundryAuthorStudioView {
        available: shell.available,
        unavailable_reason: shell.unavailable_reason,
        steps: shell.steps.into_iter().map(Into::into).collect(),
        authoring_terms: if gate.developer_ui_enabled {
            vec![
                "Family Blueprint",
                "Provider Packs",
                "Style Compatibility",
                "Sockets",
                "Ports",
                "Controls",
                "Candidate Strategies",
                "Quality Gates",
                "Review & Package",
            ]
        } else {
            Vec::new()
        },
        descriptor_only_notice: gate.developer_ui_enabled.then_some(
            "Component import is descriptor only until mesh import support is reviewed.",
        ),
        foundation_draft_panel: gate.developer_ui_enabled.then(|| FoundryFoundationDraftPanelView {
            internal_draft_ids: foundation_draft_fixtures()
                .into_iter()
                .map(|draft| draft.draft_id)
                .collect(),
            actions: vec![
                "Inspect Draft",
                "Validate Draft",
                "Show Adversarial Report",
                "Materialize Internal Kit Draft",
            ],
            catalog_visibility_notice: "Foundation drafts stay internal until human review approves authored geometry.",
        }),
    }
}

/// True when the default Visual Foundry product shell exposes Author Studio.
#[must_use]
pub(crate) fn default_visual_foundry_exposes_author_studio() -> bool {
    foundry_author_studio_view(FoundryAuthorStudioGate::default_release()).available
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundry::app::product_visible_strings_for_default_shell;

    #[test]
    fn foundry_author_default_visual_foundry_does_not_expose_author_studio() {
        assert!(!default_visual_foundry_exposes_author_studio());
        let visible = product_visible_strings_for_default_shell().join("\n");
        assert!(!visible.contains("Foundry Author Studio"));
        assert!(!visible.contains("Sockets"));
        assert!(!visible.contains("Provider Packs"));
        let view = foundry_author_studio_view(FoundryAuthorStudioGate::default_release());
        assert!(view.authoring_terms.is_empty());
        assert!(view.descriptor_only_notice.is_none());
        assert!(view.foundation_draft_panel.is_none());
        assert!(!visible.contains("box_primitive_core_draft"));
    }

    #[test]
    fn foundry_author_gated_view_exposes_authoring_surfaces() {
        let view = foundry_author_studio_view(FoundryAuthorStudioGate::developer_enabled());
        assert!(view.available);
        assert_eq!(view.steps.len(), 9);
        assert!(view.steps.iter().any(|step| step.label == "Provider Packs"));
        assert!(view.steps.iter().any(|step| step.label == "Quality Gates"));
        assert!(view.authoring_terms.contains(&"Sockets"));
        assert!(
            view.descriptor_only_notice
                .is_some_and(|notice| notice.contains("descriptor only"))
        );
        let panel = view
            .foundation_draft_panel
            .expect("foundation draft panel should be gated");
        assert!(
            panel
                .internal_draft_ids
                .contains(&"box_primitive_core_draft".to_owned())
        );
        assert!(panel.actions.contains(&"Materialize Internal Kit Draft"));
        assert!(panel.catalog_visibility_notice.contains("stay internal"));
    }
}
