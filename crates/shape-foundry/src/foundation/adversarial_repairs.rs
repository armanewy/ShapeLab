
/// Build a deterministic adversarial report for a draft.
#[must_use]
pub fn foundation_adversarial_report(draft: &FoundryFoundationDraft) -> DraftAdversarialReport {
    let primary_count = draft
        .control_profile
        .controls
        .iter()
        .filter(|control| control.visible && control.primary)
        .count();
    let provider_slot_count = draft.provider_taxonomy.provider_slots.len();
    let contact_sheet_required = draft
        .quality_gate_profile
        .as_ref()
        .is_some_and(|gate| gate.contact_sheet_required);
    let missing_geometry = vec![
        "Reviewed authored geometry or procedural art ingredients.".to_owned(),
        "Clay contact sheets once geometry exists.".to_owned(),
        "Human taste review notes.".to_owned(),
    ];
    let questions = vec![
        question(
            "over_abstracted",
            "Is this over-abstracted?",
            if draft.family_blueprint.roles.len() > 6 {
                "Role count is high; consolidate before authoring geometry."
            } else {
                "Role count is compact enough for a foundation draft."
            },
        ),
        question(
            "fewer_controls",
            "Does the kit need fewer controls?",
            if primary_count > 5 {
                "Primary controls are approaching novice complexity; reduce before promotion."
            } else {
                "Primary control count is restrained for a draft."
            },
        ),
        question(
            "provider_reuse",
            "Are provider slots reusable?",
            if provider_slot_count == 0 {
                "No provider slots are declared."
            } else {
                "Provider slots are explicit and can be reviewed for reuse."
            },
        ),
        question(
            "style_salad",
            "Could this become style salad?",
            if draft.style_pack.allowed_provider_tags.len() > 4 {
                "Allowed tags are broad; style compatibility needs pruning."
            } else {
                "Style tag surface is narrow enough for a draft."
            },
        ),
        question(
            "clear_labels",
            "Are noob-facing labels clear?",
            if validate_foundation_draft(draft)
                .issues
                .iter()
                .any(|issue| issue.code == "technical_term_in_novice_label")
            {
                "Some labels expose technical language."
            } else {
                "No technical label leakage detected."
            },
        ),
        question(
            "too_many_choices",
            "Are there too many choices for a novice?",
            if primary_count > DEFAULT_MAX_PRIMARY_NOVICE_CONTROLS as usize {
                "Primary choices exceed the novice control limit."
            } else {
                "Primary choices stay within the novice limit."
            },
        ),
        question(
            "mechanical_gates",
            "Are quality gates only mechanical?",
            if draft.review_checklist.items.is_empty() {
                "Review checklist is empty; add human taste gates."
            } else {
                "Review checklist includes human-review evidence."
            },
        ),
        question(
            "contact_sheets",
            "What visual contact sheets are required?",
            if contact_sheet_required {
                "Contact sheet gate is required for the target."
            } else {
                "Draft target does not require a contact sheet yet; require one before Usable."
            },
        ),
        question(
            "human_review",
            "What human review is required?",
            "Human review must approve geometry, controls, labels, quality evidence, and catalog visibility.",
        ),
        question(
            "missing_geometry",
            "What geometry/art ingredients are missing?",
            "Taste-bearing geometry, visual variants, and contact-sheet renders are not supplied by the foundation draft.",
        ),
        question(
            "procedural_filler",
            "What prevents this from becoming procedural filler?",
            "Internal-only visibility, validation gates, contact sheets, and human review block automatic promotion.",
        ),
    ];
    DraftAdversarialReport {
        schema_version: FOUNDRY_FOUNDATION_ADVERSARIAL_SCHEMA_VERSION,
        draft_id: draft.draft_id.clone(),
        questions,
        missing_geometry_art_ingredients: missing_geometry,
        human_review_required: vec![
            "Approve authored geometry.".to_owned(),
            "Approve contact sheets before Usable or Showcase claims.".to_owned(),
            "Approve catalog visibility explicitly.".to_owned(),
        ],
        summary: format!(
            "{} remains an internal foundation draft until authored geometry and review evidence pass.",
            draft.draft_id
        ),
    }
}

/// Suggest deterministic repairs from a validation report.
#[must_use]
pub fn suggest_foundation_repairs(
    draft: &FoundryFoundationDraft,
    validation_report_ref: impl Into<String>,
    report: &FoundationDraftValidationReport,
) -> DraftRepairSuggestion {
    DraftRepairSuggestion {
        schema_version: FOUNDRY_FOUNDATION_DRAFT_SCHEMA_VERSION,
        draft_id: draft.draft_id.clone(),
        validation_report_ref: validation_report_ref.into(),
        suggestions: report
            .issues
            .iter()
            .map(|issue| repair_for_issue(&issue.code))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(str::to_owned)
            .collect(),
    }
}

fn repair_for_issue(code: &str) -> &'static str {
    match code {
        "missing_required_roles" => "Add at least one required family role.",
        "too_many_primary_controls" => "Reduce visible primary controls to seven or fewer.",
        "technical_term_in_novice_label" => "Replace technical labels with product-facing words.",
        "duplicate_slot_ownership" => "Assign each visible slot to one control.",
        "missing_provider_slots" | "missing_provider_slot_for_required_role" => {
            "Add provider slots for every required role."
        }
        "incoherent_style_provider_compatibility" => {
            "Remove contradictory style/provider compatibility rules."
        }
        "empty_candidate_strategy" | "candidate_strategy_not_control_space" => {
            "Make candidate strategies operate on visible controls."
        }
        "missing_quality_gate" => "Add a quality gate profile.",
        "usable_or_showcase_requires_contact_sheet" => {
            "Require contact sheets for Usable or Showcase targets."
        }
        "publish_requires_human_review" | "draft_or_prototype_cannot_be_novice_visible" => {
            "Keep the draft internal until explicit human review approves promotion."
        }
        "forbidden_command_attempt" | "direct_geometry_payload_attempt" => {
            "Remove forbidden commands and direct geometry payloads."
        }
        _ => "Review the validation issue and update the structured draft.",
    }
}

fn question(
    question_id: impl Into<String>,
    question: impl Into<String>,
    finding: impl Into<String>,
) -> DraftAdversarialQuestion {
    DraftAdversarialQuestion {
        question_id: question_id.into(),
        question: question.into(),
        finding: finding.into(),
    }
}

fn contains_raw_authoring_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "::",
        "scalar",
        "recipe",
        "semantic id",
        "semantic_id",
        "operation id",
        "operation_id",
        "provider id",
        "provider_id",
        "compiler",
        "decompiler",
        "raw vertex",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

fn primary_role_for_family(family_id: &str) -> String {
    let _ = family_id;
    "body".to_owned()
}

fn secondary_role_for_family(family_id: &str) -> String {
    let _ = family_id;
    "detail".to_owned()
}

fn normalize_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn title_from_id(value: &str) -> String {
    value
        .split(['_', '-'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
