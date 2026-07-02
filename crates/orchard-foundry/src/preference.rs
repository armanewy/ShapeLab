//! Local-only preference signals for Foundry candidate biasing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::FoundryCandidateId;

/// Current schema version for local Foundry preference profiles.
pub const FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION: u32 = 1;

/// Default maximum contribution of preference scores during candidate selection.
pub const DEFAULT_PREFERENCE_SELECTION_STRENGTH: f32 = 0.18;

/// Default minimum descriptor distance preference selection tries to preserve.
pub const DEFAULT_PREFERENCE_NOVELTY_FLOOR: f32 = 0.025;

/// Scope for a local preference profile.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct FoundryPreferenceScope {
    /// Asset family ID.
    pub family_id: String,
    /// Exact customizer profile content ID.
    pub customizer_profile_id: String,
}

impl FoundryPreferenceScope {
    /// Create a new preference scope.
    #[must_use]
    pub fn new(family_id: impl Into<String>, customizer_profile_id: impl Into<String>) -> Self {
        Self {
            family_id: family_id.into(),
            customizer_profile_id: customizer_profile_id.into(),
        }
    }
}

/// Local preference log.
///
/// This is intentionally explicit and local-only. Events contain visible control
/// IDs and candidate IDs, but no geometry, recipe snapshots, file paths, or
/// export directories.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPreferenceLog {
    /// Preference schema version.
    pub schema_version: u32,
    /// Signals are local to this installation/session.
    pub local_only: bool,
    /// Ordered preference events.
    #[serde(default)]
    pub events: Vec<FoundryPreferenceEvent>,
}

impl Default for FoundryPreferenceLog {
    fn default() -> Self {
        Self::new()
    }
}

impl FoundryPreferenceLog {
    /// Create an empty local preference log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            schema_version: FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION,
            local_only: true,
            events: Vec::new(),
        }
    }

    /// Append an explicit preference event.
    pub fn record(&mut self, event: FoundryPreferenceEvent) {
        self.events.push(event);
    }

    /// Build a bounded preference profile for a single catalog scope.
    #[must_use]
    pub fn profile_for_scope(&self, scope: FoundryPreferenceScope) -> FoundryPreferenceProfile {
        let mut profile = FoundryPreferenceProfile::new(scope.clone());
        if self.schema_version != FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION {
            profile.schema_version = self.schema_version;
            profile.local_only = false;
            return profile;
        }
        if !self.local_only {
            profile.local_only = false;
            return profile;
        }
        for event in &self.events {
            if event.scope() == &scope {
                profile.apply_event(event);
            }
        }
        profile
    }
}

/// One explicit local preference event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FoundryPreferenceEvent {
    /// User accepted one candidate over one or more visible alternatives.
    CandidateComparison {
        /// Catalog scope the comparison belongs to.
        scope: FoundryPreferenceScope,
        /// Optional candidate mode label supplied by the host.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
        /// Accepted candidate ID.
        accepted_candidate_id: FoundryCandidateId,
        /// Visible controls changed by the accepted candidate.
        accepted_controls: Vec<String>,
        /// Rejected candidate IDs.
        #[serde(default)]
        rejected_candidate_ids: Vec<FoundryCandidateId>,
        /// Visible controls changed by rejected alternatives.
        #[serde(default)]
        rejected_controls: Vec<String>,
        /// Explicit signal weight.
        weight: f32,
    },
    /// User rejected a proposed candidate.
    CandidateRejected {
        /// Catalog scope the rejection belongs to.
        scope: FoundryPreferenceScope,
        /// Rejected candidate ID.
        candidate_id: FoundryCandidateId,
        /// Visible controls changed by the rejected candidate.
        changed_controls: Vec<String>,
        /// Explicit signal weight.
        weight: f32,
    },
    /// User locked a visible control.
    ControlLocked {
        /// Catalog scope the lock belongs to.
        scope: FoundryPreferenceScope,
        /// Visible control ID.
        control_id: String,
        /// Explicit signal weight.
        weight: f32,
    },
    /// User reset a visible control.
    ControlReset {
        /// Catalog scope the reset belongs to.
        scope: FoundryPreferenceScope,
        /// Visible control ID.
        control_id: String,
        /// Explicit signal weight.
        weight: f32,
    },
    /// User exported a variant after changing these visible controls.
    VariantExported {
        /// Catalog scope the export belongs to.
        scope: FoundryPreferenceScope,
        /// Visible controls changed in the exported variant.
        changed_controls: Vec<String>,
        /// Explicit signal weight.
        weight: f32,
    },
    /// User added a variant to a pack after changing these visible controls.
    PackMemberAdded {
        /// Catalog scope the pack action belongs to.
        scope: FoundryPreferenceScope,
        /// Visible controls changed in the packed variant.
        changed_controls: Vec<String>,
        /// Explicit signal weight.
        weight: f32,
    },
}

impl FoundryPreferenceEvent {
    /// Return the event scope.
    #[must_use]
    pub fn scope(&self) -> &FoundryPreferenceScope {
        match self {
            Self::CandidateComparison { scope, .. }
            | Self::CandidateRejected { scope, .. }
            | Self::ControlLocked { scope, .. }
            | Self::ControlReset { scope, .. }
            | Self::VariantExported { scope, .. }
            | Self::PackMemberAdded { scope, .. } => scope,
        }
    }
}

/// Derived bounded preference profile consumed by candidate generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FoundryPreferenceProfile {
    /// Preference schema version.
    pub schema_version: u32,
    /// Signals are local to this installation/session.
    pub local_only: bool,
    /// Catalog scope this profile applies to.
    pub scope: FoundryPreferenceScope,
    /// Control-level preference rows keyed by visible control ID.
    pub control_preferences: BTreeMap<String, FoundryControlPreference>,
    /// Maximum preference contribution during candidate selection.
    pub selection_strength: f32,
    /// Minimum descriptor distance to try to preserve while applying bias.
    pub novelty_floor: f32,
    /// Number of local events that contributed to this profile.
    pub source_event_count: u64,
}

impl FoundryPreferenceProfile {
    /// Create an empty profile for a scope.
    #[must_use]
    pub fn new(scope: FoundryPreferenceScope) -> Self {
        Self {
            schema_version: FOUNDRY_PREFERENCE_PROFILE_SCHEMA_VERSION,
            local_only: true,
            scope,
            control_preferences: BTreeMap::new(),
            selection_strength: DEFAULT_PREFERENCE_SELECTION_STRENGTH,
            novelty_floor: DEFAULT_PREFERENCE_NOVELTY_FLOOR,
            source_event_count: 0,
        }
    }

    /// Return true when no signal can affect selection.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.control_preferences.is_empty()
            || self
                .control_preferences
                .values()
                .all(|preference| preference.score == 0.0)
    }

    /// Return true when the profile applies to the requested scope.
    #[must_use]
    pub fn matches_scope(&self, scope: &FoundryPreferenceScope) -> bool {
        &self.scope == scope
    }

    /// Score a candidate by its visible changed controls in `[-1, 1]`.
    #[must_use]
    pub fn score_changed_controls(&self, changed_controls: &[String]) -> f32 {
        let controls = normalized_control_ids(changed_controls);
        if controls.is_empty() {
            return 0.0;
        }
        let score = controls
            .iter()
            .filter_map(|control| {
                self.control_preferences
                    .get(control.as_str())
                    .map(|preference| preference.score)
            })
            .sum::<f32>()
            / controls.len() as f32;
        bounded_score(score)
    }

    /// Return a sanitized selection strength.
    #[must_use]
    pub fn bounded_selection_strength(&self) -> f32 {
        finite_non_negative_or_default(
            self.selection_strength,
            DEFAULT_PREFERENCE_SELECTION_STRENGTH,
        )
        .min(0.35)
    }

    /// Return a sanitized novelty floor.
    #[must_use]
    pub fn bounded_novelty_floor(&self) -> f32 {
        finite_non_negative_or_default(self.novelty_floor, DEFAULT_PREFERENCE_NOVELTY_FLOOR)
            .min(0.25)
    }

    fn apply_event(&mut self, event: &FoundryPreferenceEvent) {
        match event {
            FoundryPreferenceEvent::CandidateComparison {
                accepted_controls,
                rejected_controls,
                weight,
                ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(accepted_controls, PreferenceSignal::Accepted, *weight);
                self.apply_controls(rejected_controls, PreferenceSignal::Rejected, *weight);
            }
            FoundryPreferenceEvent::CandidateRejected {
                changed_controls,
                weight,
                ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(changed_controls, PreferenceSignal::Rejected, *weight);
            }
            FoundryPreferenceEvent::ControlLocked {
                control_id, weight, ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(
                    std::slice::from_ref(control_id),
                    PreferenceSignal::Locked,
                    *weight,
                );
            }
            FoundryPreferenceEvent::ControlReset {
                control_id, weight, ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(
                    std::slice::from_ref(control_id),
                    PreferenceSignal::Reset,
                    *weight,
                );
            }
            FoundryPreferenceEvent::VariantExported {
                changed_controls,
                weight,
                ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(changed_controls, PreferenceSignal::Exported, *weight);
            }
            FoundryPreferenceEvent::PackMemberAdded {
                changed_controls,
                weight,
                ..
            } => {
                self.source_event_count += 1;
                self.apply_controls(changed_controls, PreferenceSignal::Packed, *weight);
            }
        }
    }

    fn apply_controls(&mut self, controls: &[String], signal: PreferenceSignal, weight: f32) {
        let weight = sanitized_weight(weight);
        if weight == 0.0 {
            return;
        }
        for control in normalized_control_ids(controls) {
            self.control_preferences
                .entry(control.clone())
                .or_default()
                .apply(signal, weight);
        }
    }
}

/// Derived preference state for one visible control.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FoundryControlPreference {
    /// Weighted accepted count.
    pub accepted_weight: f32,
    /// Weighted rejected count.
    pub rejected_weight: f32,
    /// Weighted lock count.
    pub locked_weight: f32,
    /// Weighted reset count.
    pub reset_weight: f32,
    /// Weighted export count.
    pub exported_weight: f32,
    /// Weighted pack count.
    pub packed_weight: f32,
    /// Bounded final score in `[-1, 1]`.
    pub score: f32,
}

impl FoundryControlPreference {
    fn apply(&mut self, signal: PreferenceSignal, weight: f32) {
        match signal {
            PreferenceSignal::Accepted => self.accepted_weight += weight,
            PreferenceSignal::Rejected => self.rejected_weight += weight,
            PreferenceSignal::Locked => self.locked_weight += weight,
            PreferenceSignal::Reset => self.reset_weight += weight,
            PreferenceSignal::Exported => self.exported_weight += weight,
            PreferenceSignal::Packed => self.packed_weight += weight,
        }
        self.recompute_score();
    }

    fn recompute_score(&mut self) {
        let positive = self.accepted_weight + self.exported_weight * 0.8 + self.packed_weight * 0.7;
        let negative = self.rejected_weight + self.reset_weight * 0.7 + self.locked_weight * 0.35;
        let total = positive + negative;
        self.score = if total <= f32::EPSILON {
            0.0
        } else {
            bounded_score((positive - negative) / total)
        };
    }
}

#[derive(Debug, Copy, Clone)]
enum PreferenceSignal {
    Accepted,
    Rejected,
    Locked,
    Reset,
    Exported,
    Packed,
}

fn normalized_control_ids(controls: &[String]) -> Vec<&String> {
    let mut controls = controls
        .iter()
        .filter(|control| !control.trim().is_empty())
        .collect::<Vec<_>>();
    controls.sort();
    controls.dedup();
    controls
}

fn sanitized_weight(weight: f32) -> f32 {
    finite_non_negative_or_default(weight, 1.0).min(4.0)
}

fn finite_non_negative_or_default(value: f32, default: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        default
    }
}

fn bounded_score(score: f32) -> f32 {
    if score.is_finite() {
        score.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}
