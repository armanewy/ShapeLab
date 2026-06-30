//! User intent briefs and kernel-readiness classification.
//!
//! This module is intentionally contract-only. It does not generate meshes, call
//! an LLM, publish catalog profiles, or expose internal kernel/module language to
//! novice users. Its job is to turn a user's plain-language object request into a
//! safe readiness report that says whether Object Orchard can use an existing
//! proven kernel, should create a draft kernel, or must block the request until
//! more capabilities exist.

use serde::{Deserialize, Serialize};

/// User-provided brief for a reusable object kit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectIntentBrief {
    /// Plain-language object description, such as "simple box" or "door kit".
    pub object_description: String,
    /// What the user wants to do with the family in their project.
    #[serde(default)]
    pub intended_game_use: Option<String>,
    /// Things the user says must always remain true.
    #[serde(default)]
    pub must_stay_true: Vec<String>,
    /// Things the user says may vary.
    #[serde(default)]
    pub desired_variations: Vec<String>,
    /// Optional style direction. This is geometry/style bias only in v0.
    #[serde(default)]
    pub style_direction: Option<IntentStyleDirection>,
}

impl ObjectIntentBrief {
    /// Construct a brief from a single description.
    #[must_use]
    pub fn from_description(description: impl Into<String>) -> Self {
        Self {
            object_description: description.into(),
            intended_game_use: None,
            must_stay_true: Vec::new(),
            desired_variations: Vec::new(),
            style_direction: None,
        }
    }
}

/// User-facing style direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentStyleDirection {
    Clean,
    Rugged,
    Industrial,
    Stylized,
    SciFi,
    Fantasy,
}

/// Proven or draftable kernel family category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectKernelKind {
    BoxLike,
    FlatPanel,
    StandingSupport,
    Appliance,
    Vehicle,
    Unsupported,
}

/// Whether Object Orchard can proceed with a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KernelReadinessStatus {
    /// Existing proven kernel can be used today.
    Ready,
    /// A new draft kernel can be proposed, but it is not product-approved.
    Draftable,
    /// The request depends on unsupported capabilities or too much complexity.
    Blocked,
}

/// A suggested user-facing capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IntentCapabilitySuggestion {
    pub display_name: String,
    pub user_description: String,
    pub recommended_now: bool,
    pub plain_reason: String,
}

impl IntentCapabilitySuggestion {
    #[must_use]
    pub fn new(
        display_name: impl Into<String>,
        user_description: impl Into<String>,
        recommended_now: bool,
        plain_reason: impl Into<String>,
    ) -> Self {
        Self {
            display_name: display_name.into(),
            user_description: user_description.into(),
            recommended_now,
            plain_reason: plain_reason.into(),
        }
    }
}

/// Readiness report for an object intent brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelReadinessReport {
    pub status: KernelReadinessStatus,
    pub kernel_kind: ObjectKernelKind,
    pub user_facing_summary: String,
    pub recommended_starting_point: Option<String>,
    #[serde(default)]
    pub what_stays_true_prompts: Vec<String>,
    #[serde(default)]
    pub suggested_capabilities: Vec<IntentCapabilitySuggestion>,
    #[serde(default)]
    pub blocked_reasons: Vec<String>,
    #[serde(default)]
    pub next_actions: Vec<String>,
}

impl KernelReadinessReport {
    /// Validate the report for safe user-facing display.
    pub fn validate(&self) -> Result<(), KernelReadinessError> {
        validate_user_copy(&self.user_facing_summary)?;
        if let Some(starting_point) = &self.recommended_starting_point {
            validate_user_copy(starting_point)?;
        }
        for prompt in &self.what_stays_true_prompts {
            validate_user_copy(prompt)?;
        }
        for capability in &self.suggested_capabilities {
            validate_user_copy(&capability.display_name)?;
            validate_user_copy(&capability.user_description)?;
            validate_user_copy(&capability.plain_reason)?;
        }
        for reason in &self.blocked_reasons {
            validate_user_copy(reason)?;
        }
        for action in &self.next_actions {
            validate_user_copy(action)?;
        }
        Ok(())
    }
}

/// Error returned by object-intent validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelReadinessError {
    TechnicalTerm { term: &'static str },
    EmptyUserSummary,
}

const FORBIDDEN_USER_TERMS: [&str; 13] = [
    "kernel",
    "module",
    "provider",
    "slot",
    "placement zone",
    "candidate strategy",
    "quality gate",
    "semantic role",
    "conformance",
    "fingerprint",
    "topology",
    "artifact",
    "uv",
];

fn validate_user_copy(copy: &str) -> Result<(), KernelReadinessError> {
    if copy.trim().is_empty() {
        return Err(KernelReadinessError::EmptyUserSummary);
    }
    let lower = copy.to_ascii_lowercase();
    for term in FORBIDDEN_USER_TERMS {
        if lower.contains(term) {
            return Err(KernelReadinessError::TechnicalTerm { term });
        }
    }
    Ok(())
}

/// Classify a user intent brief into an existing, draftable, or blocked object family path.
#[must_use]
pub fn classify_object_intent(brief: &ObjectIntentBrief) -> KernelReadinessReport {
    let text = combined_brief_text(brief);

    if contains_any(&text, &["box", "cube", "block", "container"]) {
        return box_like_ready_report();
    }

    if contains_any(&text, &["panel", "flat panel", "wall panel", "slab"]) {
        return flat_panel_ready_report();
    }

    if contains_any(&text, &["door", "gate", "hatch"]) {
        return door_draft_report();
    }

    if contains_any(&text, &["stool", "bench", "table", "chair"]) {
        return standing_support_draft_report();
    }

    if contains_any(&text, &["stove", "oven", "range", "appliance"]) {
        return appliance_draft_report();
    }

    if contains_any(&text, &["car", "truck", "vehicle", "wheel"]) {
        return vehicle_blocked_report();
    }

    unknown_draft_report()
}

fn combined_brief_text(brief: &ObjectIntentBrief) -> String {
    let mut text = brief.object_description.to_ascii_lowercase();
    if let Some(use_case) = &brief.intended_game_use {
        text.push(' ');
        text.push_str(&use_case.to_ascii_lowercase());
    }
    for item in &brief.must_stay_true {
        text.push(' ');
        text.push_str(&item.to_ascii_lowercase());
    }
    for item in &brief.desired_variations {
        text.push(' ');
        text.push_str(&item.to_ascii_lowercase());
    }
    text
}

fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}

fn box_like_ready_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Ready,
        kernel_kind: ObjectKernelKind::BoxLike,
        user_facing_summary: "This fits the current Box Primitive path.".to_owned(),
        recommended_starting_point: Some("Box Primitive".to_owned()),
        what_stays_true_prompts: vec![
            "It stays box-like.".to_owned(),
            "It has readable width, depth, and height.".to_owned(),
            "It sits on a support surface.".to_owned(),
        ],
        suggested_capabilities: vec![
            IntentCapabilitySuggestion::new(
                "Change size",
                "Try compact, wide, tall, or flat boxes.",
                true,
                "Size changes are already supported.",
            ),
            IntentCapabilitySuggestion::new(
                "Change edge softness",
                "Try sharper or softer box edges.",
                true,
                "Edge softness is already supported.",
            ),
        ],
        blocked_reasons: Vec::new(),
        next_actions: vec!["Start from Box Primitive.".to_owned()],
    }
}

fn flat_panel_ready_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Ready,
        kernel_kind: ObjectKernelKind::FlatPanel,
        user_facing_summary: "This fits the current Flat Panel Primitive path.".to_owned(),
        recommended_starting_point: Some("Flat Panel Primitive".to_owned()),
        what_stays_true_prompts: vec![
            "It stays panel-like.".to_owned(),
            "It has readable width, height, and thickness.".to_owned(),
            "It stands upright.".to_owned(),
        ],
        suggested_capabilities: vec![
            IntentCapabilitySuggestion::new(
                "Change panel size",
                "Try narrow, wide, tall, or short panels.",
                true,
                "Panel size changes are already supported.",
            ),
            IntentCapabilitySuggestion::new(
                "Add a hinge-side edge",
                "Try a visible hinge-side edge without open or close motion.",
                true,
                "A hinge-side edge has passed as a visible feature.",
            ),
        ],
        blocked_reasons: Vec::new(),
        next_actions: vec!["Start from Flat Panel Primitive.".to_owned()],
    }
}

fn door_draft_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Draftable,
        kernel_kind: ObjectKernelKind::FlatPanel,
        user_facing_summary: "This can start from the panel path, but it is not a door yet."
            .to_owned(),
        recommended_starting_point: Some("Flat Panel Primitive".to_owned()),
        what_stays_true_prompts: vec![
            "It stays upright and panel-like.".to_owned(),
            "It has a front and back.".to_owned(),
            "It may later need a visible handle side and hinge side.".to_owned(),
        ],
        suggested_capabilities: vec![
            IntentCapabilitySuggestion::new(
                "Add a hinge-side edge",
                "Add a visible edge that suggests which side could hinge later.",
                true,
                "This has already passed as a panel feature.",
            ),
            IntentCapabilitySuggestion::new(
                "Add a handle or knob",
                "Add one visible user-facing door cue.",
                true,
                "This is the next single feature to prove before using door naming.",
            ),
        ],
        blocked_reasons: vec!["Open and close behavior is not supported yet.".to_owned()],
        next_actions: vec![
            "Start from Flat Panel Primitive and add one visible door cue.".to_owned(),
        ],
    }
}

fn standing_support_draft_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Draftable,
        kernel_kind: ObjectKernelKind::StandingSupport,
        user_facing_summary: "This needs a new standing-object draft.".to_owned(),
        recommended_starting_point: None,
        what_stays_true_prompts: vec![
            "It has a top or seat surface.".to_owned(),
            "It has supports that reach the ground.".to_owned(),
            "It stays stable.".to_owned(),
        ],
        suggested_capabilities: vec![
            IntentCapabilitySuggestion::new(
                "Change top shape",
                "Try different seat or top proportions.",
                true,
                "This would be the first useful visible control.",
            ),
            IntentCapabilitySuggestion::new(
                "Change supports",
                "Try different support thickness or count.",
                true,
                "Support placement would need a new proof.",
            ),
        ],
        blocked_reasons: Vec::new(),
        next_actions: vec!["Create a standing-object draft before adding it to Make.".to_owned()],
    }
}

fn appliance_draft_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Draftable,
        kernel_kind: ObjectKernelKind::Appliance,
        user_facing_summary: "This needs a new appliance draft before it can be made.".to_owned(),
        recommended_starting_point: None,
        what_stays_true_prompts: vec![
            "It has a main body.".to_owned(),
            "It has a clear front and top.".to_owned(),
            "It sits on a support surface.".to_owned(),
        ],
        suggested_capabilities: vec![
            IntentCapabilitySuggestion::new(
                "Add a top surface",
                "Reserve the top for cooking or work-surface details later.",
                true,
                "The top surface would be part of the first draft.",
            ),
            IntentCapabilitySuggestion::new(
                "Add front controls",
                "Reserve a front area for simple control details later.",
                false,
                "This should wait until the body reads clearly.",
            ),
        ],
        blocked_reasons: Vec::new(),
        next_actions: vec!["Create an appliance draft before adding it to Make.".to_owned()],
    }
}

fn vehicle_blocked_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Blocked,
        kernel_kind: ObjectKernelKind::Vehicle,
        user_facing_summary: "Vehicle families are too complex for the current baseline.".to_owned(),
        recommended_starting_point: None,
        what_stays_true_prompts: Vec::new(),
        suggested_capabilities: Vec::new(),
        blocked_reasons: vec![
            "Vehicle families need wheel placement, stronger proportion rules, and more review before they can be reliable.".to_owned(),
        ],
        next_actions: vec!["Start with a simpler panel, box, or standing-object proof first.".to_owned()],
    }
}

fn unknown_draft_report() -> KernelReadinessReport {
    KernelReadinessReport {
        status: KernelReadinessStatus::Draftable,
        kernel_kind: ObjectKernelKind::Unsupported,
        user_facing_summary: "This needs a new draft before Object Orchard can make it.".to_owned(),
        recommended_starting_point: None,
        what_stays_true_prompts: vec![
            "Describe what must always stay recognizable.".to_owned(),
            "Describe what can change safely.".to_owned(),
        ],
        suggested_capabilities: Vec::new(),
        blocked_reasons: Vec::new(),
        next_actions: vec![
            "Create a draft and test simple variations before adding it to Make.".to_owned(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_intent_is_ready() {
        let brief = ObjectIntentBrief::from_description("simple box family");
        let report = classify_object_intent(&brief);
        assert_eq!(report.status, KernelReadinessStatus::Ready);
        assert_eq!(report.kernel_kind, ObjectKernelKind::BoxLike);
        assert_eq!(
            report.recommended_starting_point.as_deref(),
            Some("Box Primitive")
        );
        report.validate().expect("box report should use safe copy");
    }

    #[test]
    fn panel_intent_is_ready() {
        let brief = ObjectIntentBrief::from_description("upright flat panel prop");
        let report = classify_object_intent(&brief);
        assert_eq!(report.status, KernelReadinessStatus::Ready);
        assert_eq!(report.kernel_kind, ObjectKernelKind::FlatPanel);
        assert_eq!(
            report.recommended_starting_point.as_deref(),
            Some("Flat Panel Primitive")
        );
        report
            .validate()
            .expect("panel report should use safe copy");
    }

    #[test]
    fn door_intent_is_draftable_without_door_claim() {
        let brief = ObjectIntentBrief::from_description("simple door kit");
        let report = classify_object_intent(&brief);
        assert_eq!(report.status, KernelReadinessStatus::Draftable);
        assert_eq!(report.kernel_kind, ObjectKernelKind::FlatPanel);
        assert_eq!(
            report.recommended_starting_point.as_deref(),
            Some("Flat Panel Primitive")
        );
        assert!(report.user_facing_summary.contains("not a door yet"));
        assert!(
            report
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("Open and close"))
        );
        report
            .validate()
            .expect("door draft report should use safe copy");
    }

    #[test]
    fn vehicle_intent_is_blocked() {
        let brief = ObjectIntentBrief::from_description("simple car family");
        let report = classify_object_intent(&brief);
        assert_eq!(report.status, KernelReadinessStatus::Blocked);
        assert_eq!(report.kernel_kind, ObjectKernelKind::Vehicle);
        assert!(
            report
                .blocked_reasons
                .iter()
                .any(|reason| reason.contains("wheel placement"))
        );
        report
            .validate()
            .expect("vehicle report should use safe copy");
    }

    #[test]
    fn technical_terms_are_rejected_from_user_copy() {
        for term in FORBIDDEN_USER_TERMS {
            let report = KernelReadinessReport {
                status: KernelReadinessStatus::Ready,
                kernel_kind: ObjectKernelKind::BoxLike,
                user_facing_summary: format!("This exposes {term} details to the user."),
                recommended_starting_point: None,
                what_stays_true_prompts: Vec::new(),
                suggested_capabilities: Vec::new(),
                blocked_reasons: Vec::new(),
                next_actions: Vec::new(),
            };
            assert_eq!(
                report.validate(),
                Err(KernelReadinessError::TechnicalTerm { term })
            );
        }
    }

    #[test]
    fn readiness_report_roundtrips() {
        let brief = ObjectIntentBrief::from_description("stool family");
        let report = classify_object_intent(&brief);
        let json = serde_json::to_string_pretty(&report).expect("serialize report");
        let reparsed: KernelReadinessReport = serde_json::from_str(&json).expect("parse report");
        assert_eq!(report, reparsed);
        let reparsed_json = serde_json::to_string_pretty(&reparsed).expect("reserialize report");
        assert_eq!(json, reparsed_json);
    }
}
