
fn check_member_document_ids(
    issues: &mut Vec<FoundryPackIssue>,
    members: &[FoundryPackMemberReport],
) {
    let mut owners = BTreeMap::<&str, &str>::new();
    for member in members {
        if let Some(first_member) =
            owners.insert(member.document_id.as_str(), member.member_id.as_str())
        {
            issues.push(FoundryPackIssue {
                subject: format!("members.{}.document_id", member.member_id),
                code: "duplicate_pack_member_document_id".to_owned(),
                message: format!(
                    "Member document ID `{}` is already used by member `{first_member}`.",
                    member.document_id
                ),
            });
        }
    }
}

fn check_style_facets(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ContentFingerprint)> = None;
    for (member_id, output) in outputs {
        let Some(facet) = output
            .catalog
            .style_kit
            .family_facets
            .get(&output.catalog.family.id)
        else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.style_facet"),
                code: "pack_missing_style_facet".to_owned(),
                message: "Member style kit does not expose a facet for its family.".to_owned(),
            });
            continue;
        };
        let Ok(fingerprint) =
            fingerprint_serializable("object-orchard.foundry-pack-style-facet.v1", member_id, facet)
        else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.style_facet"),
                code: "pack_style_facet_fingerprint_failed".to_owned(),
                message: "Member style facet could not be fingerprinted deterministically."
                    .to_owned(),
            });
            continue;
        };
        if let Some((baseline_member_id, baseline_fingerprint)) = baseline {
            if fingerprint != baseline_fingerprint {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.style_facet"),
                    code: "pack_style_facet_mismatch".to_owned(),
                    message: format!(
                        "Member style facet differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), fingerprint));
        }
    }
}

fn check_provider_vocabulary(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ProviderVocabularySignature)> = None;
    for (member_id, output) in outputs {
        let Ok(signature) = provider_vocabulary_signature(output) else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.provider_vocabulary"),
                code: "pack_provider_vocabulary_fingerprint_failed".to_owned(),
                message: "Member provider vocabulary could not be fingerprinted deterministically."
                    .to_owned(),
            });
            continue;
        };
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.provider_vocabulary"),
                    code: "pack_provider_vocabulary_mismatch".to_owned(),
                    message: format!(
                        "Member provider vocabulary differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn provider_vocabulary_signature(
    output: &FoundryCompilationOutput,
) -> Result<ProviderVocabularySignature, FingerprintError> {
    let mut by_role = BTreeMap::<String, Vec<ProviderSemanticSignature>>::new();
    for fragment in output.catalog.family_implementation.fragments.values() {
        by_role
            .entry(fragment.provided_role.clone())
            .or_default()
            .push(provider_semantic_signature(
                ProviderVocabularySource::FamilyFragment,
                fragment,
            )?);
    }
    for fragment in output.catalog.style_implementation.prototypes.values() {
        by_role
            .entry(fragment.provided_role.clone())
            .or_default()
            .push(provider_semantic_signature(
                ProviderVocabularySource::StylePrototype,
                fragment,
            )?);
    }
    for providers in by_role.values_mut() {
        providers.sort();
    }
    Ok(ProviderVocabularySignature { by_role })
}

fn provider_semantic_signature(
    source: ProviderVocabularySource,
    fragment: &RecipeFragment,
) -> Result<ProviderSemanticSignature, FingerprintError> {
    let mut semantic_fragment = fragment.clone();
    semantic_fragment.id.clear();
    let fingerprint = fingerprint_serializable(
        "object-orchard.foundry-pack-provider-vocabulary.v1",
        "provider_fragment",
        &semantic_fragment,
    )?;
    Ok(ProviderSemanticSignature {
        source,
        fingerprint,
    })
}

fn check_edge_language(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, EdgeLanguageSignature)> = None;
    for (member_id, output) in outputs {
        let signature = edge_language_signature(output);
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.edge_language"),
                    code: "pack_edge_language_mismatch".to_owned(),
                    message: format!(
                        "Member edge language differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn edge_language_signature(output: &FoundryCompilationOutput) -> EdgeLanguageSignature {
    let kit = &output.catalog.style_kit;
    let facet_bevel = kit
        .family_facets
        .get(&output.catalog.family.id)
        .and_then(|facet| facet.policy_overrides.bevel_policy.as_ref());
    let bevel = facet_bevel.unwrap_or(&kit.bevel_policy);
    let mut allowed_profiles = kit.profile_language.allowed_profiles.clone();
    allowed_profiles.sort();
    EdgeLanguageSignature {
        curve_family: kit.profile_language.curve_family.clone(),
        allowed_profiles,
        allow_asymmetry: kit.profile_language.allow_asymmetry,
        bevel_segments: bevel.segments,
        bevel_profile_micros: quantize_unit(bevel.profile.normalized),
        bevel_width: describe_length_value(&bevel.width),
    }
}

fn check_detail_density_range(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    const MAX_DETAIL_DENSITY_SPREAD: f32 = 0.35;

    let mut minimum = f32::INFINITY;
    let mut maximum = f32::NEG_INFINITY;
    let mut min_member = "";
    let mut max_member = "";
    for (member_id, output) in outputs {
        let density = detail_density(output);
        if density < minimum {
            minimum = density;
            min_member = member_id;
        }
        if density > maximum {
            maximum = density;
            max_member = member_id;
        }
    }
    if minimum.is_finite() && maximum.is_finite() && maximum - minimum > MAX_DETAIL_DENSITY_SPREAD {
        issues.push(FoundryPackIssue {
            subject: "members.detail_density".to_owned(),
            code: "pack_detail_density_range_mismatch".to_owned(),
            message: format!(
                "Detail density spread from member `{min_member}` to `{max_member}` exceeds the coherent pack range."
            ),
        });
    }
}

fn detail_density(output: &FoundryCompilationOutput) -> f32 {
    let kit = &output.catalog.style_kit;
    let repetition = kit
        .family_facets
        .get(&output.catalog.family.id)
        .and_then(|facet| facet.policy_overrides.repetition.as_ref())
        .unwrap_or(&kit.repetition);
    let detail_modules = kit
        .family_facets
        .get(&output.catalog.family.id)
        .map_or(0.0, |facet| facet.detail_modules.len() as f32 * 0.05);
    (repetition.density * 0.5 + kit.exaggeration.detail * 0.5 + detail_modules).clamp(0.0, 1.0)
}

fn check_scale_family(
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    let mut baseline: Option<(&str, ScaleFamilySignature)> = None;
    for (member_id, output) in outputs {
        let signature = scale_family_signature(output);
        if let Some((baseline_member_id, baseline_signature)) = &baseline {
            if signature != *baseline_signature {
                issues.push(FoundryPackIssue {
                    subject: format!("members.{member_id}.scale_family"),
                    code: "pack_scale_family_mismatch".to_owned(),
                    message: format!(
                        "Member scale family differs from baseline member `{baseline_member_id}`."
                    ),
                });
            }
        } else {
            baseline = Some((member_id.as_str(), signature));
        }
    }
}

fn scale_family_signature(output: &FoundryCompilationOutput) -> ScaleFamilySignature {
    let mut role_scale_units = BTreeMap::new();
    if let Some(facet) = output
        .catalog
        .style_kit
        .family_facets
        .get(&output.catalog.family.id)
    {
        for proportion in &facet.proportions {
            role_scale_units.insert(
                proportion.role.clone(),
                [
                    length_value_unit(&proportion.preferred_scale[0]).to_owned(),
                    length_value_unit(&proportion.preferred_scale[1]).to_owned(),
                    length_value_unit(&proportion.preferred_scale[2]).to_owned(),
                ],
            );
        }
    }
    ScaleFamilySignature {
        family_id: output.catalog.family.id.clone(),
        role_scale_units,
    }
}

fn check_duplicate_geometry(
    pack: &FoundryPackDocument,
    issues: &mut Vec<FoundryPackIssue>,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) {
    if duplicate_geometry_is_intentional(pack) {
        return;
    }
    let mut owners = BTreeMap::<ContentFingerprint, &str>::new();
    for (member_id, output) in outputs {
        let Ok(fingerprint) = geometry_fingerprint(output) else {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.geometry_fingerprint"),
                code: "pack_geometry_fingerprint_failed".to_owned(),
                message: "Member geometry could not be fingerprinted deterministically.".to_owned(),
            });
            continue;
        };
        if let Some(first_member) = owners.insert(fingerprint, member_id.as_str()) {
            issues.push(FoundryPackIssue {
                subject: format!("members.{member_id}.geometry_fingerprint"),
                code: "pack_duplicate_geometry".to_owned(),
                message: format!(
                    "Member geometry duplicates member `{first_member}` without an intentional duplicate policy."
                ),
            });
        }
    }
}

fn geometry_fingerprint(
    output: &FoundryCompilationOutput,
) -> Result<ContentFingerprint, FingerprintError> {
    fingerprint_serializable(
        "object-orchard.foundry-pack-geometry.v1",
        "combined_preview_mesh",
        &output.artifact.combined_preview.mesh,
    )
}

fn duplicate_geometry_is_intentional(pack: &FoundryPackDocument) -> bool {
    matches!(
        pack.coherence_policy,
        PackCoherencePolicy::Custom(ref key) if key == "allow_duplicate_geometry"
    ) || pack.shared_locks.iter().any(|lock| {
        matches!(
            &lock.target,
            FoundryLockTarget::Custom(key) if key == "allow_duplicate_geometry"
        )
    })
}

fn shared_catalog_lock(
    pack: &FoundryPackDocument,
    outputs: &BTreeMap<String, FoundryCompilationOutput>,
) -> Option<FoundryCatalogLock> {
    let first = outputs.values().next()?;
    let mut exact_refs = BTreeMap::new();
    exact_refs.insert(
        CATALOG_LOCK_KEY_FAMILY.to_owned(),
        shared_catalog_ref(outputs, CATALOG_LOCK_KEY_FAMILY)?.clone(),
    );
    match &pack.coherence_policy {
        PackCoherencePolicy::ExactFamilyAndStyle => {
            exact_refs.insert(
                CATALOG_LOCK_KEY_STYLE.to_owned(),
                shared_catalog_ref(outputs, CATALOG_LOCK_KEY_STYLE)?.clone(),
            );
        }
        PackCoherencePolicy::SharedFamilyOnly | PackCoherencePolicy::Custom(_) => {
            if let Some(style_ref) = shared_catalog_ref(outputs, CATALOG_LOCK_KEY_STYLE) {
                exact_refs.insert(CATALOG_LOCK_KEY_STYLE.to_owned(), style_ref.clone());
            }
        }
    }
    for key in [
        CATALOG_LOCK_KEY_FAMILY_IMPL,
        CATALOG_LOCK_KEY_STYLE_IMPL,
        CATALOG_LOCK_KEY_CUSTOMIZER_PROFILE,
    ] {
        if let Some(content_ref) = shared_catalog_ref(outputs, key) {
            exact_refs.insert(key.to_owned(), content_ref.clone());
        }
    }
    Some(FoundryCatalogLock {
        exact_refs,
        embedded_snapshots: Vec::new(),
        compiler_version: first.catalog.catalog_lock.compiler_version.clone(),
        catalog_version: first.catalog.catalog_lock.catalog_version,
    })
}

fn shared_catalog_ref<'a>(
    outputs: &'a BTreeMap<String, FoundryCompilationOutput>,
    key: &str,
) -> Option<&'a CatalogContentRef> {
    let first_ref = outputs
        .values()
        .next()?
        .catalog
        .catalog_lock
        .exact_refs
        .get(key)?;
    outputs
        .values()
        .all(|output| output.catalog.catalog_lock.exact_refs.get(key) == Some(first_ref))
        .then_some(first_ref)
}

fn describe_control_value(value: &ControlValue) -> String {
    match value {
        ControlValue::Scalar(value) => value.to_string(),
        ControlValue::Integer(value) => value.to_string(),
        ControlValue::Toggle(value) => value.to_string(),
        ControlValue::Choice(value) | ControlValue::Provider(value) => value.clone(),
    }
}

fn quantize_unit(value: f32) -> i64 {
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as i64
}

fn describe_length_value(value: &LengthValue) -> String {
    match value {
        LengthValue::Meters(value) => format!("meters:{}", quantize_length(*value)),
        LengthValue::FamilyUnits(value) => format!("family_units:{}", quantize_length(*value)),
        LengthValue::RelativeToRole { role, ratio } => {
            format!("relative_to_role:{role}:{}", quantize_length(*ratio))
        }
    }
}

fn length_value_unit(value: &LengthValue) -> &'static str {
    match value {
        LengthValue::Meters(_) => "meters",
        LengthValue::FamilyUnits(_) => "family_units",
        LengthValue::RelativeToRole { .. } => "relative_to_role",
    }
}

fn quantize_length(value: f32) -> i64 {
    (value * 1_000_000.0).round() as i64
}

fn pack_fingerprint_error(error: FingerprintError) -> FoundryPackCompilationError {
    match error {
        FingerprintError::Serialization { subject, error } => {
            FoundryPackCompilationError::Fingerprint { subject, error }
        }
        FingerprintError::NonFiniteNumber { subject } => FoundryPackCompilationError::Fingerprint {
            subject,
            error: "canonical pack report contained a non-finite number".to_owned(),
        },
    }
}
