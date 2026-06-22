//! Variation metadata remap boundary.

use super::FragmentRemapError;

/// Validate that variation remapping is intentionally routed through this module.
pub fn unsupported_variation_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "variation".to_owned(),
        reason: reason.to_owned(),
    }
}
