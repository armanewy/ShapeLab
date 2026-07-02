
/// Validate preview camera policy descriptors.
#[must_use]
pub fn validate_preview_camera_policy(
    policy: &PreviewCameraPolicyDescriptor,
) -> AuthorStudioValidationReport {
    let mut report = AuthorStudioValidationReport::default();
    let camera_ids = std::iter::once(&policy.default_camera)
        .chain(std::iter::once(&policy.direction_board_camera))
        .chain(std::iter::once(&policy.option_gallery_camera))
        .chain(policy.contact_sheet_cameras.iter())
        .map(|camera| camera.camera_id.as_str())
        .collect::<BTreeSet<_>>();

    let mut seen_camera_ids = BTreeSet::new();
    for camera in std::iter::once(&policy.default_camera)
        .chain(std::iter::once(&policy.direction_board_camera))
        .chain(std::iter::once(&policy.option_gallery_camera))
        .chain(policy.contact_sheet_cameras.iter())
    {
        if camera.camera_id.trim().is_empty()
            || camera.label.trim().is_empty()
            || camera.view.trim().is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "missing_camera_identity",
                "Camera specs require stable ID, label, and view.",
            );
        }
        if !camera.camera_id.trim().is_empty() && !seen_camera_ids.insert(camera.camera_id.as_str())
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "duplicate_camera_id",
                "Camera IDs must be unique within one preview policy.",
            );
        }
        if !camera.supported
            && camera
                .unsupported_reason
                .as_deref()
                .unwrap_or("")
                .is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "unsupported_camera_missing_reason",
                "Unsupported camera output must report an honest reason.",
            );
        }
        if camera.fitted_scale_policy.trim().is_empty() || camera.lighting_policy.trim().is_empty()
        {
            report.push(
                format!("preview_cameras.{}", camera.camera_id),
                "missing_camera_policy",
                "Camera specs require fitted-scale and lighting policies.",
            );
        }
    }
    for gallery in &policy.option_gallery_policies {
        if gallery.option_camera_ids.is_empty() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "missing_option_gallery_camera",
                "Option galleries require a camera policy.",
            );
        }
        if gallery.option_fitted_scale_policies.is_empty() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "missing_option_gallery_fitted_scale",
                "Option galleries require a fitted-scale policy.",
            );
        }
        if gallery
            .option_fitted_scale_policies
            .iter()
            .any(|policy| policy.trim().is_empty())
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "blank_option_gallery_fitted_scale",
                "Option-gallery fitted-scale policies cannot be blank.",
            );
        }
        if gallery.option_camera_ids.len() != gallery.option_fitted_scale_policies.len() {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "option_gallery_policy_length_mismatch",
                "Option-gallery camera and fitted-scale policy lists must align.",
            );
        }
        if !gallery
            .option_camera_ids
            .iter()
            .all(|camera_id| camera_ids.contains(camera_id.as_str()))
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "unknown_option_gallery_camera",
                "Option-gallery cameras must reference declared camera specs.",
            );
        }
        if gallery
            .option_camera_ids
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            > 1
            || gallery
                .option_fitted_scale_policies
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                > 1
        {
            report.push(
                format!("option_gallery_policies.{}", gallery.control_id),
                "option_gallery_camera_not_consistent",
                "All options in one control gallery must use the same camera and fitted scale.",
            );
        }
    }
    let required_views = ["front", "side", "back", "three-quarter"];
    let declared_views = policy
        .contact_sheet_cameras
        .iter()
        .map(|camera| camera.view.as_str())
        .collect::<BTreeSet<_>>();
    for required_view in required_views {
        if !declared_views.contains(required_view) {
            report.push(
                "contact_sheet_cameras",
                "missing_contact_sheet_view",
                format!("Contact sheets must declare a {required_view} view."),
            );
        }
    }
    report
}

/// Build honest launch rows for existing CLI-backed quality gates.
#[must_use]
pub fn author_quality_gate_launches(
    package: &FoundryKitPackage,
    artifacts: &AuthorQualityArtifactRefs,
) -> Vec<AuthorQualityGateLaunch> {
    let slug = package.kit.source_profile_slug.as_deref();
    let package_ref = artifacts
        .package_manifest_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let verified_builtin_arg = artifacts
        .verified_built_in_backing
        .then_some(slug)
        .flatten();
    let current_package_arg = package_ref.or(verified_builtin_arg);
    let full_package_arg = if slug.is_some() && artifacts.verified_built_in_backing {
        package_ref.or(slug)
    } else {
        None
    };
    let out_dir = artifacts.out_dir.trim();
    let has_out_dir = !out_dir.is_empty();

    let unsupported = |task, reason: &str| AuthorQualityGateLaunch {
        task,
        supported: false,
        invocation: None,
        unsupported_reason: Some(reason.to_owned()),
    };
    let supported = |task, invocation: String| AuthorQualityGateLaunch {
        task,
        supported: true,
        invocation: Some(invocation),
        unsupported_reason: None,
    };

    vec![
        if let Some(kit_arg) = current_package_arg {
            supported(
                AuthorQualityGateTask::ValidateKit,
                format!("shape-cli foundry-kit validate {kit_arg}"),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::ValidateKit,
                "Validation requires a package manifest reference or verified built-in backing.",
            )
        },
        if let (Some(kit_arg), true) = (full_package_arg, has_out_dir) {
            supported(
                AuthorQualityGateTask::RenderPreview,
                format!("shape-cli foundry-kit preview {kit_arg} --out-dir {out_dir}/preview"),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::RenderPreview,
                "Preview rendering requires verified canonical built-in backing and an output directory.",
            )
        },
        if let (Some(kit_arg), true) = (full_package_arg, has_out_dir) {
            supported(
                AuthorQualityGateTask::RenderContactSheet,
                format!(
                    "shape-cli foundry-kit contact-sheet {kit_arg} --out-dir {out_dir}/contact-sheet"
                ),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::RenderContactSheet,
                "Contact sheets require verified canonical built-in backing and an output directory.",
            )
        },
        if let (Some(_kit_arg), true, Some(slug)) = (
            full_package_arg,
            has_out_dir,
            package.kit.source_profile_slug.as_deref(),
        ) {
            supported(
                AuthorQualityGateTask::BoxPrimitiveGate,
                format!(
                    "cargo test -p shape-foundry-catalog --test box_primitive --jobs 1 # {slug} -> {out_dir}/box-primitive-gate"
                ),
            )
        } else {
            unsupported(
                AuthorQualityGateTask::BoxPrimitiveGate,
                "Box Primitive gate requires verified canonical built-in backing and an output directory.",
            )
        },
        match (&artifacts.quality_report_ref, full_package_arg, has_out_dir) {
            (Some(report_ref), Some(kit_arg), true) => supported(
                AuthorQualityGateTask::ProduceReviewManifest,
                format!(
                    "shape-cli foundry-kit review {kit_arg} --quality-report {report_ref} --out {out_dir}/review-manifest.json"
                ),
            ),
            _ => unsupported(
                AuthorQualityGateTask::ProduceReviewManifest,
                "Review manifest generation requires a quality report reference and output directory.",
            ),
        },
        if !has_out_dir {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Package export requires an output directory.",
            )
        } else if let Some(kit_arg) = if slug.is_some() {
            full_package_arg
        } else {
            package_ref
        } {
            supported(
                AuthorQualityGateTask::PackageKit,
                format!("shape-cli foundry-kit package {kit_arg} --out-dir {out_dir}/package"),
            )
        } else if slug.is_some() {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Source-backed package export requires verified canonical built-in backing.",
            )
        } else {
            unsupported(
                AuthorQualityGateTask::PackageKit,
                "Package export requires a package manifest reference.",
            )
        },
    ]
}

/// Build package/review refs for Author Studio export.
#[must_use]
pub fn author_package_export_manifest(
    _package: &FoundryKitPackage,
    artifacts: &AuthorQualityArtifactRefs,
) -> AuthorPackageExportManifest {
    AuthorPackageExportManifest {
        kit_manifest_ref: "kit-manifest.json".to_owned(),
        provider_pack_refs: vec!["provider-pack.json".to_owned()],
        style_pack_refs: vec!["style-pack.json".to_owned()],
        control_profile_ref: "control-profile.json".to_owned(),
        candidate_strategy_pack_ref: "candidate-strategy-pack.json".to_owned(),
        quality_gate_profile_ref: "quality-gate-profile.json".to_owned(),
        review_manifest_ref: artifacts
            .review_manifest_ref
            .clone()
            .unwrap_or_else(|| "review-manifest.json".to_owned()),
        quality_report_refs: artifacts.quality_report_ref.iter().cloned().collect(),
        contact_sheet_refs: artifacts.contact_sheet_refs.clone(),
    }
}
