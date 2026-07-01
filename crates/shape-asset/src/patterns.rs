//! Pattern contract shells for deterministic repetition.

use serde::{Deserialize, Serialize};

/// Count policy for a pattern.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternCountPolicy {
    /// Count is not yet authored.
    #[default]
    Unspecified,
    /// Exact finite count.
    Exact(u32),
    /// Bounded finite count range.
    Range {
        /// Minimum count.
        minimum: u32,
        /// Maximum count.
        maximum: u32,
    },
}

/// Density policy for future pattern tools.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternDensityPolicy {
    /// Exact density.
    Exact(f32),
    /// Bounded density range.
    Range {
        /// Minimum density.
        minimum: f32,
        /// Maximum density.
        maximum: f32,
    },
}

/// Export instancing policy shell.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternExportInstancingPolicy {
    /// Export instancing has not been decided.
    #[default]
    Pending,
    /// Do not claim export instancing.
    Disabled,
    /// Requested for later proof, not active yet.
    PreserveInstancesWhenSupported,
}
