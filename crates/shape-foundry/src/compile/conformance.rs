
fn evaluate_foundry_conformance(
    family: &AssetFamilySchema,
    style_kit: &shape_family::StyleKit,
    family_implementation: &FamilyImplementation,
    style_implementation: &StyleImplementation,
    instantiation_report: &FamilyInstantiationReport,
    recipe: &AssetRecipe,
    artifact: &AssetArtifact,
) -> FamilyConformanceReport {
    let selected_fragments = selected_fragment_ports(
        family_implementation,
        style_implementation,
        instantiation_report,
    );
    let mut report = FamilyConformanceReport {
        family_id: family.id.clone(),
        style_kit_id: style_kit.id.clone(),
        ..FamilyConformanceReport::default()
    };
    report.roles = evaluate_role_conformance(family, recipe, &selected_fragments);
    report.attachments =
        evaluate_attachment_conformance(family, family_implementation, recipe, &selected_fragments);
    report.constraints = evaluate_geometric_constraints(
        &family.constraints,
        &ConstraintBindingMap::new(),
        recipe,
        Some(artifact),
    );
    report.operations = evaluate_operation_conformance(family, recipe);
    report.exports =
        evaluate_export_requirements(&family.export_requirements, recipe, Some(artifact));
    for issue in artifact.validation_report.issues.iter() {
        report.issues.push(ConformanceIssue {
            subject: issue
                .subject
                .clone()
                .unwrap_or_else(|| "artifact".to_owned()),
            code: issue.code.clone(),
            message: issue.message.clone(),
            policy: FamilyRuleExecutionPolicy::Required,
            status: ConformanceStatus::Failed,
        });
    }
    append_rejecting_conformance_issues(&mut report);
    report
}

fn selected_fragment_ports<'a>(
    family_implementation: &'a FamilyImplementation,
    style_implementation: &'a StyleImplementation,
    instantiation_report: &'a FamilyInstantiationReport,
) -> Vec<SelectedFragmentPorts<'a>> {
    let remaps = instantiation_report
        .fragment_remaps
        .iter()
        .map(|report| (report.fragment_id.as_str(), &report.remap))
        .collect::<BTreeMap<_, _>>();
    instantiation_report
        .selected_providers
        .iter()
        .filter_map(|(role, fragment_id)| {
            let fragment = family_implementation
                .fragments
                .get(fragment_id)
                .or_else(|| style_implementation.prototypes.get(fragment_id))?;
            let remap = remaps.get(fragment_id.as_str()).copied()?;
            Some(SelectedFragmentPorts {
                role: role.as_str(),
                fragment,
                remap,
            })
        })
        .collect()
}

fn append_rejecting_conformance_issues(report: &mut FamilyConformanceReport) {
    for role in &report.roles {
        if role.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("roles.{}", role.role),
                code: "role_conformance_failed".to_owned(),
                message: "Final recipe does not satisfy required family role cardinality."
                    .to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: role.status,
            });
        }
    }
    for attachment in &report.attachments {
        if attachment.policy == FamilyRuleExecutionPolicy::Required
            && attachment.status.rejects_required()
        {
            report.issues.push(ConformanceIssue {
                subject: format!("attachments.{}", attachment.rule_id),
                code: "attachment_conformance_failed".to_owned(),
                message: "Final recipe does not satisfy required family attachment rules."
                    .to_owned(),
                policy: attachment.policy,
                status: attachment.status,
            });
        }
    }
    for constraint in &report.constraints {
        if constraint.policy == FamilyRuleExecutionPolicy::Required
            && constraint.status.rejects_required()
        {
            report.issues.push(ConformanceIssue {
                subject: format!("constraints.{}", constraint.constraint_id),
                code: "constraint_conformance_failed".to_owned(),
                message: "Final artifact does not satisfy required geometric constraints."
                    .to_owned(),
                policy: constraint.policy,
                status: constraint.status,
            });
        }
    }
    for operation in &report.operations {
        if operation.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("operations.{:?}", operation.operation),
                code: "operation_conformance_failed".to_owned(),
                message: "Final recipe contains a forbidden or invalid operation class.".to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: operation.status,
            });
        }
    }
    for export in &report.exports {
        if export.status.rejects_required() {
            report.issues.push(ConformanceIssue {
                subject: format!("exports.{}", export.profile),
                code: "export_conformance_failed".to_owned(),
                message: "Final artifact does not satisfy export requirements.".to_owned(),
                policy: FamilyRuleExecutionPolicy::Required,
                status: export.status,
            });
        }
    }
}

fn summarize_conformance(report: &FamilyConformanceReport) -> FoundryConformanceSummary {
    let required_issue_failures = report
        .issues
        .iter()
        .filter(|issue| {
            issue.policy == FamilyRuleExecutionPolicy::Required && issue.status.rejects_required()
        })
        .count();
    let role_failures = report
        .roles
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let attachment_failures = report
        .attachments
        .iter()
        .filter(|row| {
            row.policy == FamilyRuleExecutionPolicy::Required && row.status.rejects_required()
        })
        .count();
    let constraint_failures = report
        .constraints
        .iter()
        .filter(|row| {
            row.policy == FamilyRuleExecutionPolicy::Required && row.status.rejects_required()
        })
        .count();
    let operation_failures = report
        .operations
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let export_failures = report
        .exports
        .iter()
        .filter(|row| row.status.rejects_required())
        .count();
    let runtime_deferred_count = report
        .issues
        .iter()
        .filter(|issue| issue.status == ConformanceStatus::Deferred)
        .count();
    let advisory_issue_count = report
        .issues
        .iter()
        .filter(|issue| issue.policy != FamilyRuleExecutionPolicy::Required)
        .count();
    FoundryConformanceSummary {
        accepted: report.is_accepted(),
        required_failure_count: required_issue_failures
            + role_failures
            + attachment_failures
            + constraint_failures
            + operation_failures
            + export_failures,
        advisory_issue_count,
        runtime_deferred_count,
    }
}
