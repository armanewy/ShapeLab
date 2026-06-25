//! Product copy inventory for the Visual Foundry surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkflowStepCopy {
    pub index: usize,
    pub label: &'static str,
    pub detail: &'static str,
}

pub(crate) const WORKFLOW_STEPS: [WorkflowStepCopy; 5] = [
    WorkflowStepCopy {
        index: 1,
        label: "Choose",
        detail: "Template",
    },
    WorkflowStepCopy {
        index: 2,
        label: "Directions",
        detail: "Explore options",
    },
    WorkflowStepCopy {
        index: 3,
        label: "Customize",
        detail: "Adjust controls",
    },
    WorkflowStepCopy {
        index: 4,
        label: "Pack",
        detail: "Build your set",
    },
    WorkflowStepCopy {
        index: 5,
        label: "Export",
        detail: "Output your assets",
    },
];

pub(crate) const PRIMARY_ACTION_LABELS: [&str; 13] = [
    "Generate Directions",
    "Refine",
    "Explore",
    "Choose Direction",
    "Customize",
    "Add Current",
    "Export Pack",
    "Export",
    "Save",
    "Open Project",
    "New Asset",
    "Lock All",
    "Reset All",
];

pub(crate) const STATUS_LABELS: [&str; 8] = [
    "Ready",
    "Saved",
    "Unsaved",
    "Build complete",
    "Preview building",
    "Export ready",
    "Needs a model first",
    "Ready for adjustments",
];

pub(crate) const DEFAULT_SECTION_LABELS: [&str; 8] = [
    "Visual Foundry",
    "Choose what to make",
    "Directions",
    "Explore Directions",
    "Customize",
    "Pack Preview",
    "Export Ready",
    "Recent Projects",
];

pub(crate) const FORBIDDEN_PRODUCT_TERMS: &[&str] = &[
    "Legacy Implicit Mode",
    "Asset Modeling Lab",
    "Modeling Workspace",
    "Advanced Recipe",
    "ProviderPack",
    "provider pack",
    "socket",
    "port ID",
    "port id",
    "family facet",
    "raw recipe",
    "recipe",
    "raw scalar path",
    "scalar path",
    "scalar",
    "provider ID",
    "provider id",
    "provider",
    "semantic ID",
    "semantic id",
    "semantic",
    "operation ID",
    "operation id",
    "operation",
    "compiler",
    "decompiler",
    "SDF",
    "fragment remap",
    "fragment",
    "remap",
    "role binding",
    "role provider",
    "conformance binding",
    "conformance",
];

pub(crate) const FORBIDDEN_PRODUCT_TOKEN_TERMS: &[&str] = &["port", "ports", "facet", "facets"];

#[must_use]
pub(crate) fn default_product_labels() -> Vec<&'static str> {
    let mut labels = Vec::new();
    labels.extend(
        WORKFLOW_STEPS
            .iter()
            .flat_map(|step| [step.label, step.detail]),
    );
    labels.extend(PRIMARY_ACTION_LABELS);
    labels.extend(STATUS_LABELS);
    labels.extend(DEFAULT_SECTION_LABELS);
    labels
}

#[must_use]
pub(crate) fn first_forbidden_product_term(text: &str) -> Option<&'static str> {
    let lowercase = text.to_ascii_lowercase();
    let phrase = FORBIDDEN_PRODUCT_TERMS
        .iter()
        .copied()
        .find(|term| lowercase.contains(&term.to_ascii_lowercase()));
    phrase.or_else(|| {
        FORBIDDEN_PRODUCT_TOKEN_TERMS
            .iter()
            .copied()
            .find(|term| contains_forbidden_token(&lowercase, term))
    })
}

fn contains_forbidden_token(lowercase: &str, token: &str) -> bool {
    lowercase
        .split(|character: char| !character.is_ascii_alphanumeric())
        .any(|part| part == token)
}

#[must_use]
pub(crate) fn labels_are_product_safe(labels: &[&str]) -> bool {
    labels
        .iter()
        .all(|label| first_forbidden_product_term(label).is_none())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_steps_match_reference_sequence() {
        let labels = WORKFLOW_STEPS
            .iter()
            .map(|step| step.label)
            .collect::<Vec<_>>();
        assert_eq!(
            labels,
            vec!["Choose", "Directions", "Customize", "Pack", "Export"]
        );
    }

    #[test]
    fn default_product_labels_exclude_forbidden_terms() {
        let labels = default_product_labels();
        assert!(
            labels_are_product_safe(&labels),
            "product labels contain forbidden implementation copy: {labels:?}"
        );
    }

    #[test]
    fn forbidden_copy_detection_is_case_insensitive() {
        assert_eq!(
            first_forbidden_product_term("Show PROVIDER id details"),
            Some("provider ID")
        );
        assert_eq!(first_forbidden_product_term("Export ready"), None);
        assert_eq!(first_forbidden_product_term("Export pack"), None);
        assert_eq!(
            first_forbidden_product_term("socket port details"),
            Some("socket")
        );
        assert_eq!(first_forbidden_product_term("style facets"), Some("facets"));
    }
}
