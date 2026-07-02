
fn apply_provider_overrides(
    family: &AssetFamilySchema,
    family_implementation: &mut FamilyImplementation,
    style_implementation: &mut StyleImplementation,
    requests: &[ProviderOverrideRequest],
) -> Result<Vec<ProviderOverrideApplicationReport>, FoundryCompilationError> {
    let roles = family
        .part_roles
        .iter()
        .map(|role| (role.id.as_str(), role))
        .collect::<BTreeMap<_, _>>();
    let mut reports = Vec::with_capacity(requests.len());
    for request in requests {
        let Some(role) = roles.get(request.role.as_str()) else {
            return Err(FoundryCompilationError::ProviderOverrideUnavailable {
                role: request.role.clone(),
                provider_id: request.provider_id.clone(),
            });
        };
        let family_match = family_implementation
            .fragments
            .get(&request.provider_id)
            .is_some_and(|fragment| fragment.provided_role == request.role);
        let style_match = style_implementation
            .prototypes
            .get(&request.provider_id)
            .is_some_and(|fragment| fragment.provided_role == request.role);
        match (family_match, style_match) {
            (false, false) => {
                return Err(FoundryCompilationError::ProviderOverrideUnavailable {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                });
            }
            (true, true) => {
                return Err(FoundryCompilationError::ProviderOverrideAmbiguous {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                });
            }
            (true, false) => {
                if matches!(
                    role.provision,
                    RoleProvision::StyleRequired | RoleProvision::Derived
                ) {
                    return Err(FoundryCompilationError::ProviderOverrideInvalidProvision {
                        role: request.role.clone(),
                        provider_id: request.provider_id.clone(),
                    });
                }
                family_implementation
                    .default_role_providers
                    .insert(request.role.clone(), request.provider_id.clone());
                if role.provision == RoleProvision::FamilyOrStyle {
                    style_implementation
                        .default_role_providers
                        .remove(&request.role);
                }
                reports.push(ProviderOverrideApplicationReport {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                    source: ProviderOverrideSource::FamilyFragment,
                });
            }
            (false, true) => {
                if matches!(
                    role.provision,
                    RoleProvision::FamilyDefault | RoleProvision::Derived
                ) {
                    return Err(FoundryCompilationError::ProviderOverrideInvalidProvision {
                        role: request.role.clone(),
                        provider_id: request.provider_id.clone(),
                    });
                }
                style_implementation
                    .default_role_providers
                    .insert(request.role.clone(), request.provider_id.clone());
                reports.push(ProviderOverrideApplicationReport {
                    role: request.role.clone(),
                    provider_id: request.provider_id.clone(),
                    source: ProviderOverrideSource::StylePrototype,
                });
            }
        }
    }
    Ok(reports)
}

fn apply_local_recipe_overrides(
    recipe: &mut AssetRecipe,
    overrides: &[LocalRecipeOverride],
    current_base_geometry_fingerprint: GeometryInputFingerprint,
) -> Result<Vec<LocalOverrideApplicationReport>, FoundryCompilationError> {
    let mut reports = Vec::with_capacity(overrides.len());
    for override_row in overrides {
        let base_matches =
            override_row.base_geometry_fingerprint == current_base_geometry_fingerprint;
        match (base_matches, override_row.survival_policy) {
            (true, _) => {
                apply_local_edit_program(recipe, &override_row.id, &override_row.edit_program)?;
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Applied,
                    reason: "base_geometry_unchanged".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
            (false, OverrideSurvivalPolicy::Pinned) => {
                return Err(FoundryCompilationError::LocalOverrideRejected {
                    override_id: override_row.id.clone(),
                    reason: "pinned_base_geometry_changed".to_owned(),
                });
            }
            (false, OverrideSurvivalPolicy::DropOnStyleChange) => {
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Dropped,
                    reason: "base_geometry_changed".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
            (false, OverrideSurvivalPolicy::Revalidate) => {
                apply_local_edit_program(recipe, &override_row.id, &override_row.edit_program)?;
                reports.push(LocalOverrideApplicationReport {
                    id: override_row.id.clone(),
                    survival_policy: override_row.survival_policy,
                    status: LocalOverrideApplicationStatus::Revalidated,
                    reason: "base_geometry_changed_revalidated".to_owned(),
                    authored_base_geometry_fingerprint: override_row.base_geometry_fingerprint,
                    current_base_geometry_fingerprint,
                });
            }
        }
    }
    Ok(reports)
}

fn annotate_panel_knob_relationship(
    recipe: &mut AssetRecipe,
    family_id: &str,
    document: &crate::FoundryAssetDocument,
) {
    if family_id != "panel_with_knob" {
        return;
    }
    if recipe
        .semantic
        .relationships
        .values()
        .any(|relationship| relationship.label == "Panel with Knob surface mount")
    {
        return;
    }

    let Some(parent) = single_role_instance(recipe, "panel_body") else {
        return;
    };
    let Some(child) = single_role_instance(recipe, "knob_form") else {
        return;
    };
    let u = scalar_control_value(document, "knob_x_offset").unwrap_or(0.5);
    let v = scalar_control_value(document, "knob_y_offset").unwrap_or(0.5);
    let relationship_id = recipe.allocate_relationship_id();
    recipe.semantic.relationships.insert(
        relationship_id,
        RelationshipContract {
            id: relationship_id,
            relationship_type: RelationshipType::SurfaceMounted,
            parent: Some(parent),
            child: Some(child),
            parent_node_ref: Some("panel".to_owned()),
            child_node_ref: Some("knob".to_owned()),
            parent_anchor_id: Some("front_handle_zone".to_owned()),
            child_anchor_id: Some("back_mount_point".to_owned()),
            label: "Panel with Knob surface mount".to_owned(),
            export_profile: None,
            placement_policy: PlacementPolicy {
                position_rule: PositionRule::ProportionalUv {
                    u: u.clamp(0.0, 1.0),
                    v: v.clamp(0.0, 1.0),
                },
            },
            orientation_policy: OrientationPolicy::AlignToSurfaceNormal {
                max_angle_degrees: 0.0,
            },
            scale_policy: ScalePolicy::PreserveChild,
            contact_policy: ContactPolicy::SurfaceContact { clearance: 0.0 },
            edit_policy: Default::default(),
            selection_policy: Default::default(),
            reset_policy: Default::default(),
            export_realization: ExportRealizationPolicy::PreserveSemanticSidecar,
        },
    );
}

fn single_role_instance(recipe: &AssetRecipe, role: &str) -> Option<PartInstanceId> {
    let tag = format!("role:{role}");
    let mut instances = recipe
        .instances
        .iter()
        .filter(|(_, instance)| instance.tags.contains(&tag))
        .map(|(id, _)| *id);
    let first = instances.next()?;
    if instances.next().is_some() {
        None
    } else {
        Some(first)
    }
}

fn scalar_control_value(document: &crate::FoundryAssetDocument, control_id: &str) -> Option<f32> {
    match document.control_state.get(control_id) {
        Some(ControlValue::Scalar(value)) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn compute_local_override_divergence_reports(
    profile: &crate::CustomizerProfile,
    document: &crate::FoundryAssetDocument,
    instantiation_report: &FamilyInstantiationReport,
    recipe: &AssetRecipe,
    application_reports: &[LocalOverrideApplicationReport],
) -> (
    BTreeMap<String, ControlDivergence>,
    Vec<LocalOverrideDivergenceReport>,
) {
    let mut control_divergence = control_divergence_state(profile, document);
    let controls_by_slot = controls_by_slot(profile);
    let control_labels = profile
        .controls
        .iter()
        .map(|control| (control.id.as_str(), control.label.as_str()))
        .collect::<BTreeMap<_, _>>();
    let slots_by_parameter = parameter_slots_by_id(instantiation_report, recipe);
    let reports_by_id = application_reports
        .iter()
        .map(|report| (&report.id, report))
        .collect::<BTreeMap<_, _>>();

    let mut reports = Vec::new();
    for override_row in &document.local_recipe_overrides {
        let Some(application_report) = reports_by_id.get(&override_row.id) else {
            continue;
        };
        if application_report.status == LocalOverrideApplicationStatus::Dropped {
            continue;
        }

        let mut slots = BTreeSet::new();
        for target in &override_row.touched_targets {
            match target {
                TouchedSemanticTarget::FamilySlot(slot) => {
                    slots.insert(slot.clone());
                }
                TouchedSemanticTarget::Parameter(parameter) => {
                    if let Some(mapped_slots) = slots_by_parameter.get(parameter) {
                        slots.extend(mapped_slots.iter().cloned());
                    }
                }
                TouchedSemanticTarget::PartDefinition(_)
                | TouchedSemanticTarget::PartInstance(_)
                | TouchedSemanticTarget::Operation(_)
                | TouchedSemanticTarget::Region(_)
                | TouchedSemanticTarget::BoundaryLoop(_)
                | TouchedSemanticTarget::Socket(_)
                | TouchedSemanticTarget::Custom(_) => {}
            }
        }

        let mut slots_by_control = BTreeMap::<String, BTreeSet<String>>::new();
        for slot in slots {
            if let Some(control_ids) = controls_by_slot.get(&slot) {
                for control_id in control_ids {
                    slots_by_control
                        .entry(control_id.clone())
                        .or_default()
                        .insert(slot.clone());
                    control_divergence
                        .insert(control_id.clone(), ControlDivergence::DivergedByOverride);
                }
            }
        }

        reports.push(LocalOverrideDivergenceReport {
            id: override_row.id.clone(),
            status: application_report.status,
            touched_targets: override_row.touched_targets.clone(),
            diverged_controls: slots_by_control
                .into_iter()
                .map(|(control_id, slots)| DivergedControlReport {
                    label: control_labels
                        .get(control_id.as_str())
                        .copied()
                        .unwrap_or(control_id.as_str())
                        .to_owned(),
                    control_id,
                    slots: slots.into_iter().collect(),
                })
                .collect(),
        });
    }

    (control_divergence, reports)
}

fn controls_by_slot(profile: &crate::CustomizerProfile) -> BTreeMap<String, BTreeSet<String>> {
    let mut controls_by_slot = BTreeMap::<String, BTreeSet<String>>::new();
    for control in &profile.controls {
        for binding in &control.bindings {
            controls_by_slot
                .entry(binding.slot.clone())
                .or_default()
                .insert(control.id.clone());
        }
    }
    controls_by_slot
}

fn parameter_slots_by_id(
    instantiation_report: &FamilyInstantiationReport,
    recipe: &AssetRecipe,
) -> BTreeMap<ParameterId, BTreeSet<String>> {
    let mut parameters_by_path = BTreeMap::<&str, Vec<ParameterId>>::new();
    for parameter in recipe.parameters.values() {
        parameters_by_path
            .entry(parameter.path.as_str())
            .or_default()
            .push(parameter.id);
    }

    let mut slots_by_parameter = BTreeMap::<ParameterId, BTreeSet<String>>::new();
    for application in &instantiation_report.parameter_applications {
        if let Some(parameter_ids) = parameters_by_path.get(application.target.as_str()) {
            for parameter_id in parameter_ids {
                slots_by_parameter
                    .entry(*parameter_id)
                    .or_default()
                    .insert(application.slot.clone());
            }
        }
    }
    slots_by_parameter
}

fn apply_local_edit_program(
    recipe: &mut AssetRecipe,
    override_id: &LocalRecipeOverrideId,
    program: &AssetEditProgram,
) -> Result<(), FoundryCompilationError> {
    match shape_asset::apply_edit_program(recipe, program) {
        Ok(edited) => {
            *recipe = edited;
            Ok(())
        }
        Err(error) => Err(FoundryCompilationError::LocalOverrideRejected {
            override_id: override_id.clone(),
            reason: error.to_string(),
        }),
    }
}
