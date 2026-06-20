//! Blender adapter contract for schema-3 packages.
//!
//! The schema-3 Blender adapter must reconstruct exact shape-key stages from
//! cumulative baked positions files. Native Blender deformers may be useful for
//! user editing later, but they are not the replay authority for package
//! verification.

use serde::{Deserialize, Serialize};

/// Options for generating a schema-3 Blender reconstruction script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlenderAdapterOptions {
    /// Package-relative manifest path used by the generated script.
    pub manifest_file: String,
    /// Report path for adapter verification output.
    pub report_file: String,
    /// Whether the script should verify an already-saved reconstruction.
    pub verify_existing: bool,
}

impl Default for BlenderAdapterOptions {
    fn default() -> Self {
        Self {
            manifest_file: "manifest.json".to_owned(),
            report_file: "blender-verification.json".to_owned(),
            verify_existing: false,
        }
    }
}

/// Builds a schema-3 Blender reconstruction script.
///
/// This is a contract stub. The future implementation must create one exact
/// cumulative shape key per baked positions file and must not rely on native
/// Blender bend or affine deformers for replay verification.
pub fn blender_reconstruction_script_v3(options: &BlenderAdapterOptions) -> String {
    format!(
        r#"# Shape Lab schema-3 Blender reconstruction stub.
# Exact replay must read {manifest_file:?}, load each cumulative baked
# positions file, and create shape-key stages from those baked positions.
# Native Blender deformers are not the replay authority for schema-3 packages.

SCHEMA_VERSION = 3
MANIFEST_FILE = {manifest_file:?}
REPORT_FILE = {report_file:?}
VERIFY_EXISTING = {verify_existing}

def main():
    raise NotImplementedError(
        "schema-3 Blender adapter must reconstruct exact baked shape-key stages"
    )

if __name__ == "__main__":
    main()
"#,
        manifest_file = options.manifest_file,
        report_file = options.report_file,
        verify_existing = options.verify_existing,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blender_stub_documents_baked_shape_key_replay() {
        let script = blender_reconstruction_script_v3(&BlenderAdapterOptions::default());

        assert!(script.contains("SCHEMA_VERSION = 3"));
        assert!(script.contains("shape-key stages from those baked positions"));
        assert!(script.contains("Native Blender deformers are not the replay authority"));
    }
}
