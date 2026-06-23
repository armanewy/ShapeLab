//! Catalog reference, resolution, and locking contracts.
#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use shape_family::{AssetFamilySchema, StyleKit};
use shape_family_compile::{
    FamilyImplementation, StyleImplementation,
    identity::{CatalogContentFingerprint, FingerprintError, fingerprint_serializable},
};

use crate::{CustomizerProfile, FoundryAssetDocument, SHAPE_FOUNDRY_CRATE_VERSION};

/// Stable reference to one catalog content document.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CatalogContentRef {
    /// Stable content ID in the catalog namespace.
    pub stable_id: String,
    /// Content schema version.
    pub schema_version: u32,
    /// Exact 256-bit content fingerprint.
    pub fingerprint: CatalogContentFingerprint,
}

/// Embedded snapshot fallback for reproducible builds when a catalog is absent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbeddedCatalogSnapshot {
    /// Reference this snapshot satisfies.
    pub content_ref: CatalogContentRef,
    /// UTF-8 JSON payload captured at lock time.
    pub canonical_json: String,
}

/// Exact catalog lock used by foundry documents, packs, and projects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryCatalogLock {
    /// Required exact references keyed by semantic role, such as `family` or `style_impl`.
    pub exact_refs: BTreeMap<String, CatalogContentRef>,
    /// Optional embedded snapshots for read-only recovery.
    #[serde(default)]
    pub embedded_snapshots: Vec<EmbeddedCatalogSnapshot>,
    /// Compiler crate version used to create the lock.
    pub compiler_version: String,
    /// Catalog format version.
    pub catalog_version: u32,
}

/// Semantic lock key for the asset family contract.
pub const CATALOG_LOCK_KEY_FAMILY: &str = "family";
/// Semantic lock key for the style-kit contract.
pub const CATALOG_LOCK_KEY_STYLE: &str = "style";
/// Semantic lock key for the executable family implementation.
pub const CATALOG_LOCK_KEY_FAMILY_IMPL: &str = "family_impl";
/// Semantic lock key for the executable style implementation.
pub const CATALOG_LOCK_KEY_STYLE_IMPL: &str = "style_impl";
/// Semantic lock key for the customizer profile.
pub const CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE: &str = "customizer_profile";

/// Source used while resolving a locked catalog entry.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCatalogContentSource {
    /// Content was returned by the active catalog resolver.
    Catalog,
    /// Content was recovered from an embedded lock snapshot.
    EmbeddedSnapshot,
}

/// Resolved catalog content with verified fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundryResolvedCatalogContent {
    /// Semantic lock key that requested this content.
    pub lock_key: String,
    /// Exact content reference that was verified.
    pub content_ref: CatalogContentRef,
    /// UTF-8 JSON payload.
    pub canonical_json: String,
    /// Resolution source.
    pub source: FoundryCatalogContentSource,
}

/// Fully resolved document catalog inputs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryResolvedCatalog {
    /// Exact catalog lock used for this build.
    pub catalog_lock: FoundryCatalogLock,
    /// Verified raw content keyed by semantic lock key.
    pub resolved_content: BTreeMap<String, FoundryResolvedCatalogContent>,
    /// Asset family schema.
    pub family: AssetFamilySchema,
    /// Style kit schema.
    pub style_kit: StyleKit,
    /// Executable family implementation.
    pub family_implementation: FamilyImplementation,
    /// Executable style implementation.
    pub style_implementation: StyleImplementation,
    /// Customizer profile.
    pub customizer_profile: CustomizerProfile,
}

/// Catalog resolution and lock verification failure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FoundryCatalogError {
    /// The resolver could not find a requested exact content reference.
    MissingContent {
        /// Requested content reference.
        content_ref: CatalogContentRef,
    },
    /// A catalog payload did not match the expected exact fingerprint.
    FingerprintMismatch {
        /// Semantic lock key.
        lock_key: String,
        /// Stable content ID.
        stable_id: String,
        /// Expected fingerprint.
        expected: CatalogContentFingerprint,
        /// Actual fingerprint.
        actual: CatalogContentFingerprint,
    },
    /// The document and lock disagree for a required exact reference.
    LockRefMismatch {
        /// Semantic lock key.
        lock_key: String,
        /// Reference carried by the document.
        document_ref: CatalogContentRef,
        /// Reference carried by the lock.
        locked_ref: CatalogContentRef,
    },
    /// A lock is missing a required semantic key.
    MissingLockRef {
        /// Semantic lock key.
        lock_key: String,
    },
    /// A JSON snapshot could not be parsed for fingerprinting or typed decode.
    InvalidJson {
        /// Semantic lock key or content subject.
        subject: String,
        /// JSON error text.
        error: String,
    },
    /// Fingerprint canonicalization failed.
    FingerprintSerialization {
        /// Fingerprinted subject.
        subject: String,
        /// Serialization error text.
        error: String,
    },
    /// A verified catalog payload could not be decoded into the expected contract type.
    DecodeContent {
        /// Semantic lock key.
        lock_key: String,
        /// Expected Rust contract type.
        expected_type: &'static str,
        /// JSON error text.
        error: String,
    },
}

/// Catalog content lookup used by foundry document compilation.
pub trait FoundryCatalogResolver {
    /// Resolve a catalog content reference to a JSON payload.
    fn resolve_catalog_content(
        &self,
        content_ref: &CatalogContentRef,
    ) -> Result<String, FoundryCatalogError>;
}

impl FoundryCatalogLock {
    /// Build an exact lock directly from the references carried by a document.
    #[must_use]
    pub fn from_document_refs(document: &FoundryAssetDocument) -> Self {
        Self {
            exact_refs: document_catalog_refs(document),
            embedded_snapshots: Vec::new(),
            compiler_version: SHAPE_FOUNDRY_CRATE_VERSION.to_owned(),
            catalog_version: 0,
        }
    }
}

/// Return the required exact catalog references for a document.
#[must_use]
pub fn document_catalog_refs(
    document: &FoundryAssetDocument,
) -> BTreeMap<String, CatalogContentRef> {
    let mut refs = BTreeMap::from([
        (
            CATALOG_LOCK_KEY_FAMILY.to_owned(),
            document.family_content_ref.clone(),
        ),
        (
            CATALOG_LOCK_KEY_STYLE.to_owned(),
            document.style_content_ref.clone(),
        ),
        (
            CATALOG_LOCK_KEY_FAMILY_IMPL.to_owned(),
            document.family_implementation_ref.clone(),
        ),
        (
            CATALOG_LOCK_KEY_STYLE_IMPL.to_owned(),
            document.style_implementation_ref.clone(),
        ),
        (
            CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE.to_owned(),
            document.customizer_profile_ref.clone(),
        ),
    ]);
    for (role, provider) in &document.provider_overrides {
        refs.insert(format!("provider.{role}"), provider.provider_ref.clone());
    }
    refs
}

/// Resolve, lock-check, fingerprint-check, and decode every catalog input for a document.
pub fn resolve_foundry_catalog(
    document: &FoundryAssetDocument,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryResolvedCatalog, FoundryCatalogError> {
    let catalog_lock = document
        .catalog_lock
        .clone()
        .unwrap_or_else(|| FoundryCatalogLock::from_document_refs(document));
    verify_document_catalog_lock(document, &catalog_lock)?;

    let family_content = resolve_locked_content(&catalog_lock, CATALOG_LOCK_KEY_FAMILY, resolver)?;
    let style_content = resolve_locked_content(&catalog_lock, CATALOG_LOCK_KEY_STYLE, resolver)?;
    let family_impl_content =
        resolve_locked_content(&catalog_lock, CATALOG_LOCK_KEY_FAMILY_IMPL, resolver)?;
    let style_impl_content =
        resolve_locked_content(&catalog_lock, CATALOG_LOCK_KEY_STYLE_IMPL, resolver)?;
    let profile_content =
        resolve_locked_content(&catalog_lock, CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE, resolver)?;

    let family = decode_locked_content::<AssetFamilySchema>(
        &family_content,
        "shape_family::AssetFamilySchema",
    )?;
    let style_kit = decode_locked_content::<StyleKit>(&style_content, "shape_family::StyleKit")?;
    let family_implementation = decode_locked_content::<FamilyImplementation>(
        &family_impl_content,
        "shape_family_compile::FamilyImplementation",
    )?;
    let style_implementation = decode_locked_content::<StyleImplementation>(
        &style_impl_content,
        "shape_family_compile::StyleImplementation",
    )?;
    let customizer_profile = decode_locked_content::<CustomizerProfile>(
        &profile_content,
        "shape_foundry::CustomizerProfile",
    )?;

    let resolved_content = BTreeMap::from([
        (CATALOG_LOCK_KEY_FAMILY.to_owned(), family_content),
        (CATALOG_LOCK_KEY_STYLE.to_owned(), style_content),
        (CATALOG_LOCK_KEY_FAMILY_IMPL.to_owned(), family_impl_content),
        (CATALOG_LOCK_KEY_STYLE_IMPL.to_owned(), style_impl_content),
        (
            CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE.to_owned(),
            profile_content,
        ),
    ]);

    Ok(FoundryResolvedCatalog {
        catalog_lock,
        resolved_content,
        family,
        style_kit,
        family_implementation,
        style_implementation,
        customizer_profile,
    })
}

/// Verify that a document's catalog lock pins exactly the document references.
pub fn verify_document_catalog_lock(
    document: &FoundryAssetDocument,
    lock: &FoundryCatalogLock,
) -> Result<(), FoundryCatalogError> {
    let expected_refs = document_catalog_refs(document);
    for (lock_key, document_ref) in &expected_refs {
        let Some(locked_ref) = lock.exact_refs.get(lock_key) else {
            return Err(FoundryCatalogError::MissingLockRef {
                lock_key: lock_key.clone(),
            });
        };
        if locked_ref != document_ref {
            return Err(FoundryCatalogError::LockRefMismatch {
                lock_key: lock_key.clone(),
                document_ref: document_ref.clone(),
                locked_ref: locked_ref.clone(),
            });
        }
    }
    for (lock_key, locked_ref) in &lock.exact_refs {
        if !expected_refs.contains_key(lock_key) {
            return Err(FoundryCatalogError::LockRefMismatch {
                lock_key: lock_key.clone(),
                document_ref: locked_ref.clone(),
                locked_ref: locked_ref.clone(),
            });
        }
    }
    Ok(())
}

/// Compute the catalog content fingerprint for a JSON payload.
pub fn catalog_content_fingerprint_from_json(
    subject: &str,
    json: &str,
) -> Result<CatalogContentFingerprint, FoundryCatalogError> {
    let value = serde_json::from_str::<serde_json::Value>(json).map_err(|error| {
        FoundryCatalogError::InvalidJson {
            subject: subject.to_owned(),
            error: error.to_string(),
        }
    })?;
    let fingerprint = fingerprint_serializable("shape-lab.catalog-content.v1", subject, &value)
        .map_err(catalog_fingerprint_error)?;
    Ok(CatalogContentFingerprint(fingerprint))
}

/// Verify a JSON payload against an exact catalog reference.
pub fn verify_catalog_content_fingerprint(
    lock_key: &str,
    content_ref: &CatalogContentRef,
    json: &str,
) -> Result<(), FoundryCatalogError> {
    let actual = catalog_content_fingerprint_from_json(&content_ref.stable_id, json)?;
    if actual != content_ref.fingerprint {
        return Err(FoundryCatalogError::FingerprintMismatch {
            lock_key: lock_key.to_owned(),
            stable_id: content_ref.stable_id.clone(),
            expected: content_ref.fingerprint,
            actual,
        });
    }
    Ok(())
}

fn resolve_locked_content(
    lock: &FoundryCatalogLock,
    lock_key: &str,
    resolver: &impl FoundryCatalogResolver,
) -> Result<FoundryResolvedCatalogContent, FoundryCatalogError> {
    let content_ref =
        lock.exact_refs
            .get(lock_key)
            .ok_or_else(|| FoundryCatalogError::MissingLockRef {
                lock_key: lock_key.to_owned(),
            })?;
    match resolver.resolve_catalog_content(content_ref) {
        Ok(canonical_json) => {
            verify_catalog_content_fingerprint(lock_key, content_ref, &canonical_json)?;
            Ok(FoundryResolvedCatalogContent {
                lock_key: lock_key.to_owned(),
                content_ref: content_ref.clone(),
                canonical_json,
                source: FoundryCatalogContentSource::Catalog,
            })
        }
        Err(FoundryCatalogError::MissingContent { .. }) => {
            resolve_embedded_snapshot(lock, lock_key, content_ref)
        }
        Err(error) => Err(error),
    }
}

fn resolve_embedded_snapshot(
    lock: &FoundryCatalogLock,
    lock_key: &str,
    content_ref: &CatalogContentRef,
) -> Result<FoundryResolvedCatalogContent, FoundryCatalogError> {
    let Some(snapshot) = lock
        .embedded_snapshots
        .iter()
        .find(|snapshot| snapshot.content_ref == *content_ref)
    else {
        return Err(FoundryCatalogError::MissingContent {
            content_ref: content_ref.clone(),
        });
    };
    verify_catalog_content_fingerprint(lock_key, content_ref, &snapshot.canonical_json)?;
    Ok(FoundryResolvedCatalogContent {
        lock_key: lock_key.to_owned(),
        content_ref: content_ref.clone(),
        canonical_json: snapshot.canonical_json.clone(),
        source: FoundryCatalogContentSource::EmbeddedSnapshot,
    })
}

fn decode_locked_content<T: DeserializeOwned>(
    content: &FoundryResolvedCatalogContent,
    expected_type: &'static str,
) -> Result<T, FoundryCatalogError> {
    serde_json::from_str(&content.canonical_json).map_err(|error| {
        FoundryCatalogError::DecodeContent {
            lock_key: content.lock_key.clone(),
            expected_type,
            error: error.to_string(),
        }
    })
}

fn catalog_fingerprint_error(error: FingerprintError) -> FoundryCatalogError {
    match error {
        FingerprintError::Serialization { subject, error } => {
            FoundryCatalogError::FingerprintSerialization { subject, error }
        }
        FingerprintError::NonFiniteNumber { subject } => {
            FoundryCatalogError::FingerprintSerialization {
                subject,
                error: "canonical fingerprint input contained a non-finite number".to_owned(),
            }
        }
    }
}
