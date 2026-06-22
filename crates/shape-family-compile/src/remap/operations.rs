//! Modeling-operation remap boundary.

use super::FragmentRemapError;

/// Validate that operation remapping is intentionally routed through this module.
pub fn unsupported_operation_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "operations".to_owned(),
        reason: reason.to_owned(),
    }
}
