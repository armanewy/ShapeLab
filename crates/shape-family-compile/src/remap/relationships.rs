//! Relationship and attachment-policy remap boundary.

use super::FragmentRemapError;

/// Validate that relationship remapping is intentionally routed through this module.
pub fn unsupported_relationship_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "relationships".to_owned(),
        reason: reason.to_owned(),
    }
}
