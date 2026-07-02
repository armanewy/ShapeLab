
fn effective_member_document(
    pack: &FoundryPackDocument,
    document: &FoundryAssetDocument,
) -> FoundryAssetDocument {
    let mut member = document.clone();
    let mut catalog_refs_changed = false;
    for (control_id, value) in &pack.shared_controls {
        member
            .control_state
            .insert(control_id.clone(), value.clone());
    }
    if let SharedExact(providers) = &pack.shared_provider_policy {
        for (role, provider_ref) in providers {
            member.provider_overrides.insert(
                role.clone(),
                ProviderOverride {
                    role: role.clone(),
                    provider_ref: provider_ref.clone(),
                },
            );
            catalog_refs_changed = true;
        }
    }
    for shared_lock in &pack.shared_locks {
        if !member
            .foundry_locks
            .iter()
            .any(|lock| lock.target == shared_lock.target)
        {
            member.foundry_locks.push(shared_lock.clone());
        }
    }
    if catalog_refs_changed || pack.catalog_lock.is_some() {
        let mut lock = member
            .catalog_lock
            .clone()
            .unwrap_or_else(|| FoundryCatalogLock::from_document_refs(&member));
        if catalog_refs_changed {
            lock.exact_refs = document_catalog_refs(&member);
        }
        if let Some(pack_lock) = &pack.catalog_lock {
            lock.exact_refs.extend(pack_lock.exact_refs.clone());
            for snapshot in &pack_lock.embedded_snapshots {
                if !lock
                    .embedded_snapshots
                    .iter()
                    .any(|existing| existing.content_ref == snapshot.content_ref)
                {
                    lock.embedded_snapshots.push(snapshot.clone());
                }
            }
            lock.compiler_version = pack_lock.compiler_version.clone();
            lock.catalog_version = pack_lock.catalog_version;
        }
        member.catalog_lock = Some(lock);
    }
    member
}

fn build_pack_report(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Result<FoundryPackReport, FoundryPackCompilationError> {
    let members = member_reports(outputs);
    let shared_controls = shared_controls(outputs);
    let differences = difference_reports(outputs);
    let triangle_totals = triangle_totals(&members);
    let visual_descriptor_spread = visual_descriptor_spread(&members);
    let conformance_status = conformance_status(pack, outputs, &members);
    let payload = PackReportFingerprintPayload {
        pack_id: &pack.pack_id,
        members: &members,
        shared_controls: &shared_controls,
        differences: &differences,
        triangle_totals: &triangle_totals,
        visual_descriptor_spread: &visual_descriptor_spread,
        conformance_status: &conformance_status,
    };
    let report_fingerprint = fingerprint_serializable(
        "object-orchard.foundry-pack-report.v1",
        "foundry_pack_report",
        &payload,
    )
    .map_err(pack_fingerprint_error)?;

    Ok(FoundryPackReport {
        pack_id: pack.pack_id.clone(),
        members,
        shared_controls,
        differences,
        triangle_totals,
        visual_descriptor_spread,
        conformance_status,
        report_fingerprint,
    })
}

fn member_reports(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackMemberReport> {
    outputs
        .iter()
        .map(|(member_id, output)| FoundryPackMemberReport {
            member_id: member_id.clone(),
            document_id: output.document.document_id.0.clone(),
            family_ref: output.document.family_content_ref.clone(),
            style_ref: output.document.style_content_ref.clone(),
            provider_choices: provider_choices(output),
            controls: output.document.control_state.clone(),
            triangle_count: output.artifact.statistics.triangle_count,
            visual_descriptor: visual_descriptor(output),
            conformance: output.conformance_summary.clone(),
        })
        .collect()
}

fn provider_choices(output: &FoundryCompilationOutput) -> BTreeMap<String, String> {
    let mut choices = output
        .catalog
        .family_implementation
        .default_role_providers
        .clone();
    choices.extend(
        output
            .catalog
            .style_implementation
            .default_role_providers
            .clone(),
    );
    for report in &output.provider_override_reports {
        choices.insert(report.role.clone(), report.provider_id.clone());
    }
    choices
}

fn shared_controls(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackSharedControlReport> {
    let Some((_, first)) = outputs.iter().next() else {
        return Vec::new();
    };
    first
        .document
        .control_state
        .iter()
        .filter(|(control_id, value)| {
            outputs
                .values()
                .all(|output| output.document.control_state.get(*control_id) == Some(*value))
        })
        .map(|(control_id, value)| FoundryPackSharedControlReport {
            control_id: control_id.clone(),
            value: value.clone(),
        })
        .collect()
}

fn difference_reports(
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Vec<FoundryPackDifferenceReport> {
    let Some((baseline_member_id, baseline)) = outputs.iter().next() else {
        return Vec::new();
    };
    let baseline_providers = provider_choices(baseline);
    let control_ids = outputs
        .values()
        .flat_map(|output| output.document.control_state.keys().cloned())
        .collect::<BTreeSet<_>>();
    let provider_roles = outputs
        .values()
        .flat_map(provider_choices)
        .map(|(role, _)| role)
        .collect::<BTreeSet<_>>();

    let mut differences = Vec::new();
    for (member_id, output) in outputs {
        if member_id == baseline_member_id {
            continue;
        }
        for control_id in &control_ids {
            let baseline_value = baseline.document.control_state.get(control_id);
            let member_value = output.document.control_state.get(control_id);
            if baseline_value != member_value {
                differences.push(FoundryPackDifferenceReport {
                    member_id: member_id.clone(),
                    subject: format!("control_state.{control_id}"),
                    baseline: baseline_value.map(describe_control_value),
                    value: member_value.map(describe_control_value),
                });
            }
        }
        let member_providers = provider_choices(output);
        for role in &provider_roles {
            let baseline_value = baseline_providers.get(role);
            let member_value = member_providers.get(role);
            if baseline_value != member_value {
                differences.push(FoundryPackDifferenceReport {
                    member_id: member_id.clone(),
                    subject: format!("provider_overrides.{role}"),
                    baseline: baseline_value.cloned(),
                    value: member_value.cloned(),
                });
            }
        }
    }
    differences
}

fn triangle_totals(members: &[FoundryPackMemberReport]) -> FoundryPackTriangleTotals {
    let by_member = members
        .iter()
        .map(|member| (member.member_id.clone(), member.triangle_count))
        .collect::<BTreeMap<_, _>>();
    FoundryPackTriangleTotals {
        total: members.iter().map(|member| member.triangle_count).sum(),
        minimum_member: members
            .iter()
            .map(|member| member.triangle_count)
            .min()
            .unwrap_or(0),
        maximum_member: members
            .iter()
            .map(|member| member.triangle_count)
            .max()
            .unwrap_or(0),
        by_member,
    }
}

fn visual_descriptor(output: &FoundryCompilationOutput) -> FoundryPackVisualDescriptor {
    let bounds = output.artifact.combined_preview.mesh.bounds;
    let bounds_extent = if bounds.is_empty() {
        [0.0, 0.0, 0.0]
    } else {
        [
            (bounds.max[0] - bounds.min[0]).max(0.0),
            (bounds.max[1] - bounds.min[1]).max(0.0),
            (bounds.max[2] - bounds.min[2]).max(0.0),
        ]
    };
    let bounds_volume = bounds_extent[0] * bounds_extent[1] * bounds_extent[2];
    let triangle_density = if bounds_volume > 0.0 {
        output.artifact.statistics.triangle_count as f32 / bounds_volume
    } else {
        0.0
    };
    FoundryPackVisualDescriptor {
        bounds_extent,
        bounds_volume,
        part_count: output.artifact.statistics.part_count,
        polygon_face_count: output.artifact.statistics.polygon_face_count,
        triangle_count: output.artifact.statistics.triangle_count,
        triangle_density,
    }
}

fn visual_descriptor_spread(
    members: &[FoundryPackMemberReport],
) -> FoundryPackVisualDescriptorSpread {
    let by_member = members
        .iter()
        .map(|member| (member.member_id.clone(), member.visual_descriptor.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut maximum_pairwise_distance = 0.0_f32;
    for (left_index, left) in members.iter().enumerate() {
        for right in members.iter().skip(left_index + 1) {
            maximum_pairwise_distance = maximum_pairwise_distance.max(descriptor_distance(
                &left.visual_descriptor,
                &right.visual_descriptor,
            ));
        }
    }
    FoundryPackVisualDescriptorSpread {
        by_member,
        maximum_pairwise_distance,
    }
}

fn descriptor_distance(
    left: &FoundryPackVisualDescriptor,
    right: &FoundryPackVisualDescriptor,
) -> f32 {
    let mut total = 0.0_f32;
    for axis in 0..3 {
        total += normalized_delta(left.bounds_extent[axis], right.bounds_extent[axis]);
    }
    total += normalized_delta(left.bounds_volume, right.bounds_volume);
    total += normalized_delta(left.part_count as f32, right.part_count as f32);
    total += normalized_delta(
        left.polygon_face_count as f32,
        right.polygon_face_count as f32,
    );
    total += normalized_delta(left.triangle_count as f32, right.triangle_count as f32);
    total += normalized_delta(left.triangle_density, right.triangle_density);
    total / 8.0
}

fn normalized_delta(left: f32, right: f32) -> f32 {
    let denominator = left.abs().max(right.abs()).max(1.0);
    ((left - right).abs() / denominator).min(1.0)
}

fn conformance_status(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
    members: &[FoundryPackMemberReport],
) -> FoundryPackConformanceStatus {
    let mut issues = Vec::new();
    check_member_document_ids(&mut issues, members);
    check_style_facets(&mut issues, outputs);
    check_provider_vocabulary(&mut issues, outputs);
    check_edge_language(&mut issues, outputs);
    check_detail_density_range(&mut issues, outputs);
    check_scale_family(&mut issues, outputs);
    check_duplicate_geometry(pack, &mut issues, outputs);
    for member in members {
        if !member.conformance.accepted {
            issues.push(FoundryPackIssue {
                subject: format!("members.{}.conformance", member.member_id),
                code: "pack_member_conformance_rejected".to_owned(),
                message: "Pack member final conformance was not accepted.".to_owned(),
            });
        }
    }
    FoundryPackConformanceStatus {
        accepted: issues.is_empty(),
        issues,
    }
}
