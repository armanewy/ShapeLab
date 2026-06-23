#![forbid(unsafe_code)]

//! Foundry catalog manifest contracts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shape_foundry::{CatalogContentRef, FoundryCatalogLock};

/// Current schema version for catalog manifests.
pub const FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION: u32 = 1;
/// Package version for catalog contracts.
pub const SHAPE_FOUNDRY_CATALOG_CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// One named catalog entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogEntry {
    /// Content reference.
    pub content_ref: CatalogContentRef,
    /// Human-facing label.
    pub label: String,
    /// Catalog tags.
    pub tags: Vec<String>,
}

/// Catalog manifest that can produce exact locks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogManifest {
    /// Manifest schema version.
    pub schema_version: u32,
    /// Stable catalog ID.
    pub catalog_id: String,
    /// Catalog version.
    pub catalog_version: u32,
    /// Entries keyed by stable content ID.
    pub entries: BTreeMap<String, FoundryCatalogEntry>,
}

/// Error while constructing a catalog lock from a manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCatalogManifestError {
    /// A requested content ID was not present in the manifest.
    UnknownContentId {
        /// Semantic lock key, such as `family`.
        lock_key: String,
        /// Missing content ID.
        content_id: String,
    },
}

impl FoundryCatalogManifest {
    /// Build an exact lock for selected `(lock_key, content_id)` pairs.
    ///
    /// Lock keys are semantic roles such as `family`, `style`, `family_impl`,
    /// `style_impl`, or `customizer_profile`; they are not content IDs.
    pub fn lock_selected(
        &self,
        selected_refs: impl IntoIterator<Item = (String, String)>,
        compiler_version: impl Into<String>,
    ) -> Result<FoundryCatalogLock, FoundryCatalogManifestError> {
        let mut exact_refs = BTreeMap::new();
        for (lock_key, content_id) in selected_refs {
            let Some(entry) = self.entries.get(&content_id) else {
                return Err(FoundryCatalogManifestError::UnknownContentId {
                    lock_key,
                    content_id,
                });
            };
            exact_refs.insert(lock_key, entry.content_ref.clone());
        }
        Ok(FoundryCatalogLock {
            exact_refs,
            embedded_snapshots: Vec::new(),
            compiler_version: compiler_version.into(),
            catalog_version: self.catalog_version,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shape_family_compile::identity::{CatalogContentFingerprint, ContentFingerprint};
    use shape_foundry::CatalogContentRef;

    #[test]
    fn lock_selected_uses_semantic_lock_keys() {
        let manifest = FoundryCatalogManifest {
            schema_version: FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id: "test-catalog".to_owned(),
            catalog_version: 4,
            entries: BTreeMap::from([(
                "bridge-family-content".to_owned(),
                FoundryCatalogEntry {
                    content_ref: content_ref("bridge-family-content", 1),
                    label: "Bridge Family".to_owned(),
                    tags: Vec::new(),
                },
            )]),
        };

        let lock = manifest
            .lock_selected(
                [("family".to_owned(), "bridge-family-content".to_owned())],
                "0.1.0",
            )
            .expect("lock should resolve");

        assert!(lock.exact_refs.contains_key("family"));
        assert!(!lock.exact_refs.contains_key("bridge-family-content"));
    }

    #[test]
    fn lock_selected_reports_unknown_content_ids() {
        let manifest = FoundryCatalogManifest {
            schema_version: FOUNDRY_CATALOG_MANIFEST_SCHEMA_VERSION,
            catalog_id: "test-catalog".to_owned(),
            catalog_version: 4,
            entries: BTreeMap::new(),
        };

        assert_eq!(
            manifest.lock_selected([("family".to_owned(), "missing".to_owned())], "0.1.0"),
            Err(FoundryCatalogManifestError::UnknownContentId {
                lock_key: "family".to_owned(),
                content_id: "missing".to_owned(),
            })
        );
    }

    fn content_ref(stable_id: &str, byte: u8) -> CatalogContentRef {
        CatalogContentRef {
            stable_id: stable_id.to_owned(),
            schema_version: 1,
            fingerprint: CatalogContentFingerprint(ContentFingerprint([byte; 32])),
        }
    }
}
