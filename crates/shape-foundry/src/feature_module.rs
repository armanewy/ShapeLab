use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Current schema version for internal feature module contracts.
pub const FEATURE_MODULE_CONTRACT_SCHEMA_VERSION: u32 = 1;

/// Internal contract for one visible feature module.
///
/// These contracts are intentionally internal authoring records. Novice users
/// should see reusable-kit language and object ideas, not module terminology.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureModuleContract {
    /// Contract schema version.
    pub schema_version: u32,
    /// Stable module ID.
    pub module_id: String,
    /// Internal display name.
    pub display_name: String,
    /// Preconditions that must already be present before this module can run.
    pub requires: Vec<FeatureModuleRequirement>,
    /// Roles, controls, and hooks this module contributes.
    pub provides: Vec<FeatureModuleProvision>,
    /// Control IDs owned by this module.
    pub owns_controls: Vec<String>,
    /// Candidate hooks owned by this module.
    pub candidate_hooks: Vec<String>,
    /// Quality gates required before this module can be surfaced.
    pub quality_gates: Vec<FeatureModuleQualityGate>,
    /// Non-data behavior hooks that prove the module is not just nullable fields.
    pub behavior_hooks: Vec<String>,
}

/// One module precondition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureModuleRequirement {
    /// Stable requirement ID.
    pub id: String,
    /// Human-readable internal summary.
    pub summary: String,
}

/// One capability contribution from a feature module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FeatureModuleProvision {
    /// A visible semantic role.
    VisibleRole {
        /// Role ID.
        role: String,
        /// Internal role label.
        label: String,
    },
    /// A visible control owned by the module.
    Control {
        /// Control ID.
        control_id: String,
        /// Control label.
        label: String,
    },
    /// Candidate-generation hook.
    CandidateHook {
        /// Hook ID.
        hook_id: String,
        /// Hook label.
        label: String,
    },
    /// Data-only field. A valid visible module cannot consist only of these.
    NullableField {
        /// Field ID.
        field_id: String,
    },
}

/// One feature-module quality gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureModuleQualityGate {
    /// Stable gate ID.
    pub id: String,
    /// Internal gate summary.
    pub summary: String,
}

/// Validation report for a feature module contract.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FeatureModuleValidationReport {
    /// Validation issues.
    pub issues: Vec<FeatureModuleValidationIssue>,
}

impl FeatureModuleValidationReport {
    /// Return true when no issues were recorded.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.issues.is_empty()
    }

    fn push(&mut self, subject: impl Into<String>, code: &'static str, message: &'static str) {
        self.issues.push(FeatureModuleValidationIssue {
            subject: subject.into(),
            code: code.to_owned(),
            message: message.to_owned(),
        });
    }
}

/// One feature-module validation issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureModuleValidationIssue {
    /// Field or object that failed validation.
    pub subject: String,
    /// Stable issue code.
    pub code: String,
    /// Developer-facing message.
    pub message: String,
}

/// Validate one internal feature module contract.
#[must_use]
pub fn validate_feature_module_contract(
    contract: &FeatureModuleContract,
) -> FeatureModuleValidationReport {
    let mut report = FeatureModuleValidationReport::default();

    if contract.schema_version != FEATURE_MODULE_CONTRACT_SCHEMA_VERSION {
        report.push(
            "schema_version",
            "unsupported_feature_module_contract_schema",
            "Feature module contract schema version is not supported.",
        );
    }
    validate_id(&mut report, "module_id", &contract.module_id);
    validate_label(&mut report, "display_name", &contract.display_name);
    validate_non_empty(
        &mut report,
        "requires",
        contract.requires.is_empty(),
        "missing_feature_module_requirements",
        "Feature modules must declare what they require.",
    );
    validate_non_empty(
        &mut report,
        "provides",
        contract.provides.is_empty(),
        "missing_feature_module_provisions",
        "Feature modules must declare what they provide.",
    );
    validate_non_empty(
        &mut report,
        "quality_gates",
        contract.quality_gates.is_empty(),
        "missing_feature_module_quality_gates",
        "Feature modules must declare quality gates.",
    );
    validate_non_empty(
        &mut report,
        "behavior_hooks",
        contract.behavior_hooks.is_empty(),
        "missing_feature_module_behavior",
        "Feature modules must declare behavior hooks, not only nullable fields.",
    );

    let mut requirement_ids = BTreeSet::new();
    for (index, requirement) in contract.requires.iter().enumerate() {
        let subject = format!("requires.{index}");
        validate_id(&mut report, format!("{subject}.id"), &requirement.id);
        validate_label(
            &mut report,
            format!("{subject}.summary"),
            &requirement.summary,
        );
        if !requirement_ids.insert(requirement.id.as_str()) {
            report.push(
                format!("{subject}.id"),
                "duplicate_feature_module_requirement",
                "Feature module requirement IDs must be unique.",
            );
        }
    }

    let mut provision_ids = BTreeSet::new();
    let mut control_provisions = BTreeSet::new();
    let mut hook_provisions = BTreeSet::new();
    let mut behavioral_provision_count = 0usize;
    for (index, provision) in contract.provides.iter().enumerate() {
        let subject = format!("provides.{index}");
        match provision {
            FeatureModuleProvision::VisibleRole { role, label } => {
                validate_id(&mut report, format!("{subject}.role"), role);
                validate_label(&mut report, format!("{subject}.label"), label);
                behavioral_provision_count += 1;
                track_unique(&mut report, &mut provision_ids, subject, role);
            }
            FeatureModuleProvision::Control { control_id, label } => {
                validate_id(&mut report, format!("{subject}.control_id"), control_id);
                validate_label(&mut report, format!("{subject}.label"), label);
                control_provisions.insert(control_id.as_str());
                behavioral_provision_count += 1;
                track_unique(&mut report, &mut provision_ids, subject, control_id);
            }
            FeatureModuleProvision::CandidateHook { hook_id, label } => {
                validate_id(&mut report, format!("{subject}.hook_id"), hook_id);
                validate_label(&mut report, format!("{subject}.label"), label);
                hook_provisions.insert(hook_id.as_str());
                behavioral_provision_count += 1;
                track_unique(&mut report, &mut provision_ids, subject, hook_id);
            }
            FeatureModuleProvision::NullableField { field_id } => {
                validate_id(&mut report, format!("{subject}.field_id"), field_id);
                track_unique(&mut report, &mut provision_ids, subject, field_id);
            }
        }
    }
    if behavioral_provision_count == 0 {
        report.push(
            "provides",
            "feature_module_nullable_only",
            "Feature modules cannot be only nullable fields without visible behavior.",
        );
    }

    let mut owned_controls = BTreeSet::new();
    for (index, control_id) in contract.owns_controls.iter().enumerate() {
        validate_id(
            &mut report,
            format!("owns_controls.{index}"),
            control_id.as_str(),
        );
        if !control_provisions.contains(control_id.as_str()) {
            report.push(
                format!("owns_controls.{index}"),
                "feature_module_owned_control_not_provided",
                "Owned controls must be declared as provided controls.",
            );
        }
        if !owned_controls.insert(control_id.as_str()) {
            report.push(
                format!("owns_controls.{index}"),
                "duplicate_feature_module_owned_control",
                "Owned control IDs must be unique.",
            );
        }
    }

    let mut candidate_hooks = BTreeSet::new();
    for (index, hook_id) in contract.candidate_hooks.iter().enumerate() {
        validate_id(
            &mut report,
            format!("candidate_hooks.{index}"),
            hook_id.as_str(),
        );
        if !hook_provisions.contains(hook_id.as_str()) {
            report.push(
                format!("candidate_hooks.{index}"),
                "feature_module_candidate_hook_not_provided",
                "Candidate hooks must be declared as provided hooks.",
            );
        }
        if !candidate_hooks.insert(hook_id.as_str()) {
            report.push(
                format!("candidate_hooks.{index}"),
                "duplicate_feature_module_candidate_hook",
                "Candidate hook IDs must be unique.",
            );
        }
    }

    let mut gate_ids = BTreeSet::new();
    for (index, gate) in contract.quality_gates.iter().enumerate() {
        let subject = format!("quality_gates.{index}");
        validate_id(&mut report, format!("{subject}.id"), &gate.id);
        validate_label(&mut report, format!("{subject}.summary"), &gate.summary);
        if !gate_ids.insert(gate.id.as_str()) {
            report.push(
                format!("{subject}.id"),
                "duplicate_feature_module_quality_gate",
                "Feature module quality gate IDs must be unique.",
            );
        }
    }

    for (index, hook_id) in contract.behavior_hooks.iter().enumerate() {
        validate_id(
            &mut report,
            format!("behavior_hooks.{index}"),
            hook_id.as_str(),
        );
    }

    report
}

/// Internal Lid Seam module contract used by the lidded-box preview fixture.
#[must_use]
pub fn lid_seam_feature_module_contract() -> FeatureModuleContract {
    FeatureModuleContract {
        schema_version: FEATURE_MODULE_CONTRACT_SCHEMA_VERSION,
        module_id: "lid-seam".to_owned(),
        display_name: "Lid Seam".to_owned(),
        requires: vec![
            FeatureModuleRequirement {
                id: "closed-box-body".to_owned(),
                summary: "Requires an export-safe closed box body.".to_owned(),
            },
            FeatureModuleRequirement {
                id: "top-lid-candidate-zone".to_owned(),
                summary: "Requires a readable top/lid candidate zone.".to_owned(),
            },
        ],
        provides: vec![
            FeatureModuleProvision::VisibleRole {
                role: "lid_seam".to_owned(),
                label: "Visible lid seam".to_owned(),
            },
            FeatureModuleProvision::Control {
                control_id: "lid_height".to_owned(),
                label: "Lid Seam".to_owned(),
            },
            FeatureModuleProvision::CandidateHook {
                hook_id: "lidded-box-ideas".to_owned(),
                label: "Lidded box ideas".to_owned(),
            },
        ],
        owns_controls: vec!["lid_height".to_owned()],
        candidate_hooks: vec!["lidded-box-ideas".to_owned()],
        quality_gates: vec![
            FeatureModuleQualityGate {
                id: "seam-visible-in-pure-clay".to_owned(),
                summary: "The seam is visible without texture or material color.".to_owned(),
            },
            FeatureModuleQualityGate {
                id: "not-material-stripe".to_owned(),
                summary: "The seam reads as geometry, not a material stripe.".to_owned(),
            },
            FeatureModuleQualityGate {
                id: "closed-box-silhouette-preserved".to_owned(),
                summary: "The seam does not break the closed box silhouette.".to_owned(),
            },
            FeatureModuleQualityGate {
                id: "seam-endpoint-visible".to_owned(),
                summary: "The Lid Height endpoint creates a visible change.".to_owned(),
            },
        ],
        behavior_hooks: vec![
            "apply-lid-seam-geometry".to_owned(),
            "generate-lidded-box-candidates".to_owned(),
            "validate-lid-seam-readability".to_owned(),
        ],
    }
}

fn validate_non_empty(
    report: &mut FeatureModuleValidationReport,
    subject: &'static str,
    empty: bool,
    code: &'static str,
    message: &'static str,
) {
    if empty {
        report.push(subject, code, message);
    }
}

fn validate_id(
    report: &mut FeatureModuleValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    let subject = subject.into();
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' || ch == '_')
    {
        report.push(
            subject,
            "invalid_feature_module_identifier",
            "Feature module identifiers must be lowercase ASCII identifiers.",
        );
    }
}

fn validate_label(
    report: &mut FeatureModuleValidationReport,
    subject: impl Into<String>,
    value: &str,
) {
    if value.trim().is_empty() {
        report.push(
            subject,
            "missing_feature_module_label",
            "Feature module labels and summaries must be non-empty.",
        );
    }
}

fn track_unique<'a>(
    report: &mut FeatureModuleValidationReport,
    seen: &mut BTreeSet<&'a str>,
    subject: String,
    id: &'a str,
) {
    if !seen.insert(id) {
        report.push(
            subject,
            "duplicate_feature_module_provision",
            "Feature module provision IDs must be unique.",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lid_seam_feature_module_contract_validates() {
        let contract = lid_seam_feature_module_contract();
        let report = validate_feature_module_contract(&contract);
        assert!(
            report.is_valid(),
            "Lid Seam feature module should validate: {report:#?}"
        );
        assert_eq!(contract.module_id, "lid-seam");
        assert_eq!(contract.owns_controls, vec!["lid_height"]);
        assert!(
            contract
                .requires
                .iter()
                .any(|requirement| requirement.id == "closed-box-body")
        );
        assert!(
            contract
                .requires
                .iter()
                .any(|requirement| requirement.id == "top-lid-candidate-zone")
        );
    }

    #[test]
    fn feature_module_rejects_nullable_fields_without_behavior() {
        let contract = FeatureModuleContract {
            schema_version: FEATURE_MODULE_CONTRACT_SCHEMA_VERSION,
            module_id: "nullable-only".to_owned(),
            display_name: "Nullable Only".to_owned(),
            requires: vec![FeatureModuleRequirement {
                id: "box-body".to_owned(),
                summary: "Requires a body.".to_owned(),
            }],
            provides: vec![FeatureModuleProvision::NullableField {
                field_id: "maybe-seam".to_owned(),
            }],
            owns_controls: Vec::new(),
            candidate_hooks: Vec::new(),
            quality_gates: vec![FeatureModuleQualityGate {
                id: "field-exists".to_owned(),
                summary: "Only checks the field exists.".to_owned(),
            }],
            behavior_hooks: Vec::new(),
        };

        let report = validate_feature_module_contract(&contract);
        let codes = report
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<BTreeSet<_>>();
        assert!(codes.contains("missing_feature_module_behavior"));
        assert!(codes.contains("feature_module_nullable_only"));
    }
}
