//! Instance hierarchy and assembly remap boundary.

use super::FragmentRemapError;

/// Validate that assembly remapping is intentionally routed through this module.
pub fn unsupported_assembly_remap(fragment: &str, reason: &str) -> FragmentRemapError {
    FragmentRemapError::Unsupported {
        fragment: fragment.to_owned(),
        stage: "assembly".to_owned(),
        reason: reason.to_owned(),
    }
}
