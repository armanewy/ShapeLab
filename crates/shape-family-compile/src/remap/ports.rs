//! Fragment port remap and attachment binding boundary.

use super::FragmentRemapError;

/// Validate that port remapping is intentionally routed through this module.
pub fn unsupported_port_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "ports".to_owned(),
        reason: reason.to_owned(),
    }
}
