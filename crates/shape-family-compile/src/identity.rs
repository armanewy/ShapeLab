use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;
use thiserror::Error;

/// Standard 256-bit content fingerprint.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentFingerprint(pub [u8; 32]);

macro_rules! fingerprint_scope {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        pub struct $name(pub ContentFingerprint);
    };
}

fingerprint_scope!(
    CatalogContentFingerprint,
    "Fingerprint for catalog content documents."
);
fingerprint_scope!(
    FoundryIntentFingerprint,
    "Fingerprint for all semantic foundry intent values."
);
fingerprint_scope!(
    GeometryInputFingerprint,
    "Fingerprint for values that can affect generated geometry."
);
fingerprint_scope!(
    RecipeFingerprint,
    "Fingerprint for an AssetRecipe snapshot."
);
fingerprint_scope!(ArtifactFingerprint, "Fingerprint for a compiled artifact.");

impl ContentFingerprint {
    /// Return this fingerprint as lowercase hexadecimal text.
    #[must_use]
    pub fn to_hex(self) -> String {
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "{byte:02x}");
        }
        out
    }

    /// Derive a compact non-zero `u64` from the first eight bytes.
    #[must_use]
    pub fn to_nonzero_u64(self) -> u64 {
        let mut bytes = [0_u8; 8];
        bytes.copy_from_slice(&self.0[..8]);
        let value = u64::from_le_bytes(bytes);
        if value == 0 { 1 } else { value }
    }
}

impl fmt::Display for ContentFingerprint {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl Serialize for ContentFingerprint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for ContentFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_hex_fingerprint(&raw).map_err(de::Error::custom)
    }
}

/// Errors produced while building content fingerprints.
#[derive(Debug, Error)]
pub enum FingerprintError {
    /// Serialization into canonical JSON failed.
    #[error("failed to serialize `{subject}` for content fingerprint: {error}")]
    Serialization {
        /// Fingerprinted subject.
        subject: String,
        /// Serialization error.
        error: String,
    },
    /// Canonical JSON contained a non-finite number.
    #[error("non-finite number in `{subject}` cannot be fingerprinted")]
    NonFiniteNumber {
        /// Fingerprinted subject.
        subject: String,
    },
}

/// Hash a serializable value using a domain-separated BLAKE3-256 content hash.
pub fn fingerprint_serializable<T: Serialize>(
    domain: &str,
    subject: &str,
    value: &T,
) -> Result<ContentFingerprint, FingerprintError> {
    let value = serde_json::to_value(value).map_err(|error| FingerprintError::Serialization {
        subject: subject.to_owned(),
        error: error.to_string(),
    })?;
    let canonical = canonical_json(subject, &value)?;
    Ok(fingerprint_bytes(domain, canonical.as_bytes()))
}

/// Hash already canonical bytes with a domain-separated BLAKE3-256 content hash.
#[must_use]
pub fn fingerprint_bytes(domain: &str, bytes: &[u8]) -> ContentFingerprint {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"shape-lab.content-fingerprint.v1\0");
    hasher.update(domain.as_bytes());
    hasher.update(b"\0");
    hasher.update(bytes);
    ContentFingerprint(*hasher.finalize().as_bytes())
}

fn canonical_json(subject: &str, value: &Value) -> Result<String, FingerprintError> {
    let mut out = String::new();
    write_canonical_value(subject, value, &mut out)?;
    Ok(out)
}

fn write_canonical_value(
    subject: &str,
    value: &Value,
    out: &mut String,
) -> Result<(), FingerprintError> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => {
            let Some(value) = number.as_f64() else {
                return Err(FingerprintError::NonFiniteNumber {
                    subject: subject.to_owned(),
                });
            };
            if !value.is_finite() {
                return Err(FingerprintError::NonFiniteNumber {
                    subject: subject.to_owned(),
                });
            }
            out.push_str(&number.to_string());
        }
        Value::String(value) => {
            out.push_str(&serde_json::to_string(value).map_err(|error| {
                FingerprintError::Serialization {
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
                    FingerprintError::Serialization {
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

fn parse_hex_fingerprint(raw: &str) -> Result<ContentFingerprint, String> {
    if raw.len() != 64 {
        return Err("content fingerprints must be 64 lowercase hex characters".to_owned());
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        let start = index * 2;
        let pair = &raw[start..start + 2];
        if !pair
            .chars()
            .all(|character| character.is_ascii_digit() || matches!(character, 'a'..='f'))
        {
            return Err("content fingerprints must use lowercase hexadecimal".to_owned());
        }
        *byte = u8::from_str_radix(pair, 16)
            .map_err(|_| "content fingerprints must use hexadecimal bytes".to_owned())?;
    }
    Ok(ContentFingerprint(bytes))
}
