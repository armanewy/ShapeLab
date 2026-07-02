use orchard_family::FamilyRuleExecutionPolicy;
use orchard_family_compile::conformance::{
    AttachmentConformance, AttachmentCoverageConformance, ConformanceIssue, ConformanceStatus,
    FamilyConformanceReport, RoleConformance, RoleMultiplicityExpectation,
};

#[test]
fn required_failures_reject_but_advisory_failures_do_not() {
    let mut report = FamilyConformanceReport {
        family_id: "box_primitive".to_owned(),
        style_kit_id: "plain_clay".to_owned(),
        ..FamilyConformanceReport::default()
    };
    report.issues.push(ConformanceIssue {
        subject: "constraints.deck_clearance".to_owned(),
        code: "clearance_warning".to_owned(),
        message: "Advisory clearance was below the preferred target.".to_owned(),
        policy: FamilyRuleExecutionPolicy::Advisory,
        status: ConformanceStatus::Failed,
    });
    assert!(report.is_accepted());

    report.issues.push(ConformanceIssue {
        subject: "attachments.support_span".to_owned(),
        code: "missing_required_attachment".to_owned(),
        message: "Required support-span attachment was not found.".to_owned(),
        policy: FamilyRuleExecutionPolicy::Required,
        status: ConformanceStatus::Missing,
    });
    assert!(!report.is_accepted());
}

#[test]
fn required_row_status_rejects_without_flattened_issue() {
    let mut report = FamilyConformanceReport {
        family_id: "box_primitive".to_owned(),
        style_kit_id: "plain_clay".to_owned(),
        ..FamilyConformanceReport::default()
    };
    report.roles.push(RoleConformance {
        role: "support".to_owned(),
        expected: RoleMultiplicityExpectation { min: 1, max: None },
        actual_occurrences: 0,
        effective_enabled: true,
        status: ConformanceStatus::Missing,
        issue_codes: Vec::new(),
    });

    assert!(!report.is_accepted());
}

#[test]
fn runtime_deferred_rows_are_non_blocking_but_required_deferred_rows_reject() {
    let mut report = FamilyConformanceReport {
        family_id: "box_primitive".to_owned(),
        style_kit_id: "plain_clay".to_owned(),
        ..FamilyConformanceReport::default()
    };
    report.attachments.push(AttachmentConformance {
        rule_id: "walkable_runtime".to_owned(),
        from_role: "deck".to_owned(),
        to_role: "runtime_profile".to_owned(),
        policy: FamilyRuleExecutionPolicy::RuntimeOnly,
        pairs: Vec::new(),
        coverage: AttachmentCoverageConformance::default(),
        status: ConformanceStatus::Deferred,
        issue_codes: Vec::new(),
    });
    assert!(report.is_accepted());

    report.attachments.push(AttachmentConformance {
        rule_id: "support_span".to_owned(),
        from_role: "support".to_owned(),
        to_role: "span".to_owned(),
        policy: FamilyRuleExecutionPolicy::Required,
        pairs: Vec::new(),
        coverage: AttachmentCoverageConformance::default(),
        status: ConformanceStatus::Deferred,
        issue_codes: Vec::new(),
    });
    assert!(!report.is_accepted());
}
