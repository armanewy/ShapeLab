//! Replayable foundry project contracts.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use shape_asset::{ASSET_RECIPE_SCHEMA_VERSION, AssetRecipe, RevisionId};
use shape_family_compile::identity::{RecipeFingerprint, fingerprint_serializable};

use crate::{
    FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION, FoundryAssetDocument, FoundryBuildStamp,
    FoundryCatalogLock, FoundryCommand, FoundryConformanceSummary, FoundryEdit,
    GeneratedRecipeSnapshot,
};

/// Distinct project kind marker used to reject unrelated Shape Lab project JSON.
pub const FOUNDRY_PROJECT_KIND: &str = "shape-lab.foundry-project";

/// Stored command program that produced a foundry revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "program_kind", rename_all = "snake_case")]
pub enum FoundryProjectRevisionProgram {
    /// Existing foundry edit contract.
    Edit {
        /// Foundry edit row.
        edit: FoundryEdit,
    },
    /// Raw command program with an explicit human label.
    Commands {
        /// Human-facing edit label.
        label: String,
        /// Ordered commands applied by this program.
        commands: Vec<FoundryCommand>,
    },
}

impl FoundryProjectRevisionProgram {
    /// Create a revision program from a foundry edit.
    #[must_use]
    pub fn from_edit(edit: FoundryEdit) -> Self {
        Self::Edit { edit }
    }

    /// Create a revision program from raw foundry commands.
    #[must_use]
    pub fn from_commands(
        label: impl Into<String>,
        commands: impl Into<Vec<FoundryCommand>>,
    ) -> Self {
        Self::Commands {
            label: label.into(),
            commands: commands.into(),
        }
    }

    /// Return the human-facing program label.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Edit { edit } => &edit.label,
            Self::Commands { label, .. } => label,
        }
    }

    /// Return the ordered command list.
    #[must_use]
    pub fn commands(&self) -> &[FoundryCommand] {
        match self {
            Self::Edit { edit } => &edit.commands,
            Self::Commands { commands, .. } => commands,
        }
    }
}

impl From<FoundryEdit> for FoundryProjectRevisionProgram {
    fn from(edit: FoundryEdit) -> Self {
        Self::from_edit(edit)
    }
}

/// Stored foundry project revision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryProjectRevision {
    /// Stable revision ID.
    pub id: RevisionId,
    /// Parent revision.
    pub parent: Option<RevisionId>,
    /// Human-facing label.
    pub label: String,
    /// Semantic source snapshot.
    pub document: FoundryAssetDocument,
    /// Edit or command program that produced this revision.
    pub program: Option<FoundryProjectRevisionProgram>,
    /// Catalog lock.
    pub catalog_lock: FoundryCatalogLock,
    /// Build stamp captured when this revision was compiled.
    pub build_stamp: Option<FoundryBuildStamp>,
    /// Exact generated recipe snapshot.
    pub recipe_snapshot: Option<GeneratedRecipeSnapshot>,
    /// Conformance summary.
    pub conformance: FoundryConformanceSummary,
}

/// Replayable foundry project file contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryProjectDocument {
    /// Distinct project kind marker.
    pub project_kind: String,
    /// Foundry project schema version.
    pub schema_version: u32,
    /// Project title.
    pub title: String,
    /// Current revision.
    pub current_revision: RevisionId,
    /// Next revision ID.
    pub next_revision_id: u64,
    /// Revision graph.
    #[serde(with = "revision_map_serde")]
    pub revisions: BTreeMap<RevisionId, FoundryProjectRevision>,
}

impl FoundryProjectDocument {
    /// Create an empty project contract.
    #[must_use]
    pub fn empty(title: impl Into<String>) -> Self {
        Self {
            project_kind: FOUNDRY_PROJECT_KIND.to_owned(),
            schema_version: FOUNDRY_PROJECT_DOCUMENT_SCHEMA_VERSION,
            title: title.into(),
            current_revision: RevisionId(0),
            next_revision_id: 1,
            revisions: BTreeMap::new(),
        }
    }
}

impl GeneratedRecipeSnapshot {
    /// Capture an exact canonical snapshot for a generated recipe.
    pub fn from_recipe(recipe: &AssetRecipe) -> Result<Self, FoundryRecipeSnapshotError> {
        let canonical_json = canonical_json_for_serializable("instantiated_recipe", recipe)?;
        let recipe_fingerprint = RecipeFingerprint(
            fingerprint_serializable("shape-lab.recipe.v1", "instantiated_recipe", recipe)
                .map_err(|error| FoundryRecipeSnapshotError::Serialization {
                    subject: "instantiated_recipe".to_owned(),
                    error: error.to_string(),
                })?,
        );
        Ok(Self {
            schema_version: recipe.schema_version,
            canonical_json,
            recipe_fingerprint,
        })
    }

    /// Return true when this snapshot exactly matches `recipe`.
    pub fn matches_recipe(&self, recipe: &AssetRecipe) -> Result<bool, FoundryRecipeSnapshotError> {
        Ok(self == &Self::from_recipe(recipe)?)
    }

    /// Return this snapshot's recipe fingerprint as lowercase hex.
    #[must_use]
    pub fn recipe_fingerprint_hex(&self) -> String {
        self.recipe_fingerprint.0.to_hex()
    }

    /// Return true when the snapshot claims the current asset recipe schema.
    #[must_use]
    pub fn uses_current_recipe_schema(&self) -> bool {
        self.schema_version == ASSET_RECIPE_SCHEMA_VERSION
    }
}

/// Error while building or verifying an exact recipe snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoundryRecipeSnapshotError {
    /// Serialization into canonical JSON failed.
    Serialization {
        /// Serialized subject.
        subject: String,
        /// Source error string.
        error: String,
    },
    /// Canonical JSON contained a non-finite number.
    NonFiniteNumber {
        /// Serialized subject.
        subject: String,
    },
}

impl fmt::Display for FoundryRecipeSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Serialization { subject, error } => {
                write!(
                    formatter,
                    "failed to serialize `{subject}` for recipe snapshot: {error}"
                )
            }
            Self::NonFiniteNumber { subject } => {
                write!(
                    formatter,
                    "non-finite number in `{subject}` cannot be snapshotted"
                )
            }
        }
    }
}

impl Error for FoundryRecipeSnapshotError {}

fn canonical_json_for_serializable<T: Serialize>(
    subject: &str,
    value: &T,
) -> Result<String, FoundryRecipeSnapshotError> {
    let value =
        serde_json::to_value(value).map_err(|error| FoundryRecipeSnapshotError::Serialization {
            subject: subject.to_owned(),
            error: error.to_string(),
        })?;
    let mut out = String::new();
    write_canonical_value(subject, &value, &mut out)?;
    Ok(out)
}

fn write_canonical_value(
    subject: &str,
    value: &Value,
    out: &mut String,
) -> Result<(), FoundryRecipeSnapshotError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => {
            let Some(value) = number.as_f64() else {
                return Err(FoundryRecipeSnapshotError::NonFiniteNumber {
                    subject: subject.to_owned(),
                });
            };
            if !value.is_finite() {
                return Err(FoundryRecipeSnapshotError::NonFiniteNumber {
                    subject: subject.to_owned(),
                });
            }
            out.push_str(&number.to_string());
        }
        Value::String(value) => {
            out.push_str(&serde_json::to_string(value).map_err(|error| {
                FoundryRecipeSnapshotError::Serialization {
                    subject: subject.to_owned(),
                    error: error.to_string(),
                }
            })?);
        }
        Value::Array(values) => {
            out.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical_value(subject, value, out)?;
            }
            out.push(']');
        }
        Value::Object(values) => {
            out.push('{');
            let mut first = true;
            let mut entries = values.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(key, _)| *key);
            for (key, value) in entries {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&serde_json::to_string(key).map_err(|error| {
                    FoundryRecipeSnapshotError::Serialization {
                        subject: subject.to_owned(),
                        error: error.to_string(),
                    }
                })?);
                out.push(':');
                write_canonical_value(subject, value, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

mod revision_map_serde {
    use std::collections::BTreeMap;

    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
    use shape_asset::RevisionId;

    use super::FoundryProjectRevision;

    pub fn serialize<S>(
        revisions: &BTreeMap<RevisionId, FoundryProjectRevision>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let string_keyed = revisions
            .iter()
            .map(|(id, revision)| (id.0.to_string(), revision))
            .collect::<BTreeMap<_, _>>();
        string_keyed.serialize(serializer)
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<BTreeMap<RevisionId, FoundryProjectRevision>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let string_keyed = BTreeMap::<String, FoundryProjectRevision>::deserialize(deserializer)?;
        let mut revisions = BTreeMap::new();
        for (raw, revision) in string_keyed {
            let id = raw.parse::<u64>().map(RevisionId).map_err(|error| {
                de::Error::custom(format!("invalid foundry revision id `{raw}`: {error}"))
            })?;
            revisions.insert(id, revision);
        }
        Ok(revisions)
    }
}
