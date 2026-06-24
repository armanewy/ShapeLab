use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use clap::Subcommand;
use serde_json::Value;
use shape_compile::export::{verify_model_package, write_grouped_obj_export, write_model_package};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_foundry::{
    FoundryKitPackage, FoundryKitQualityTier, KitReviewManifest, compile_foundry_document,
    foundry_kit_visibility_decision, validate_foundry_kit_package,
};
use shape_foundry_catalog::{
    FoundryFixtureCatalog, built_in_fixture_catalogs_with_labels, built_in_foundry_kit_package,
};
use shape_render::{RenderSettings, RenderedImage, fit_camera_to_bounds_from_angles, render_mesh};

use crate::{render_mesh_from_triangles, save_contact_sheet, save_png, validity_label, write_json};

/// Commands for curated Foundry kit packages.
#[derive(Debug, clap::Args)]
pub struct FoundryKitArgs {
    /// Foundry kit operation.
    #[command(subcommand)]
    pub command: FoundryKitCommand,
}

/// Foundry kit subcommands.
#[derive(Debug, Subcommand)]
pub enum FoundryKitCommand {
    /// Validate a kit package JSON file or built-in kit slug.
    Validate {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
    },
    /// Print a human-readable kit summary.
    Inspect {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
    },
    /// Render a clay preview for a built-in-backed kit.
    Preview {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Render a clay contact sheet for a built-in-backed kit.
    ContactSheet {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Write a portable kit metadata package.
    Package {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
        /// Output directory.
        #[arg(long)]
        out_dir: PathBuf,
    },
    /// Convert an HQ quality report into a kit review manifest.
    Review {
        /// Kit package path, package directory, or built-in slug.
        kit_path: String,
        /// Wave 32 HQ quality report JSON.
        #[arg(long)]
        quality_report: PathBuf,
        /// Review manifest output path.
        #[arg(long)]
        out: PathBuf,
    },
}

/// Run a Foundry kit CLI command.
pub fn run_foundry_kit(args: FoundryKitArgs) -> anyhow::Result<()> {
    match args.command {
        FoundryKitCommand::Validate { kit_path } => run_validate(&kit_path),
        FoundryKitCommand::Inspect { kit_path } => run_inspect(&kit_path),
        FoundryKitCommand::Preview { kit_path, out_dir } => run_preview(&kit_path, &out_dir),
        FoundryKitCommand::ContactSheet { kit_path, out_dir } => {
            run_contact_sheet(&kit_path, &out_dir)
        }
        FoundryKitCommand::Package { kit_path, out_dir } => run_package(&kit_path, &out_dir),
        FoundryKitCommand::Review {
            kit_path,
            quality_report,
            out,
        } => run_review(&kit_path, &quality_report, &out),
    }
}

fn run_validate(input: &str) -> anyhow::Result<()> {
    let package = load_kit_package(input)?;
    let report = validate_foundry_kit_package(&package);
    println!("Foundry kit {}", package.kit.display_name);
    println!("  kit id: {}", package.kit.kit_id);
    println!("  status: {}", validity_label(report.is_valid()));
    for issue in &report.issues {
        eprintln!("  [{}] {}: {}", issue.code, issue.subject, issue.message);
    }
    if report.is_valid() {
        Ok(())
    } else {
        bail!(
            "Foundry kit validation failed with {} issue(s)",
            report.issues.len()
        )
    }
}

fn run_inspect(input: &str) -> anyhow::Result<()> {
    let package = load_kit_package(input)?;
    let report = validate_foundry_kit_package(&package);
    let visibility = foundry_kit_visibility_decision(&package.kit, &package.review_manifest, false);
    println!("Foundry kit: {}", package.kit.display_name);
    println!("  kit id: {}", package.kit.kit_id);
    println!("  family: {}", package.family_blueprint.display_name);
    println!("  style: {}", package.style_pack.display_name);
    println!("  quality tier: {}", package.kit.quality_tier.label());
    println!(
        "  default catalog: {}",
        if visibility.visible {
            "visible"
        } else {
            "hidden"
        }
    );
    if let Some(reason) = visibility.reason {
        println!("  hidden reason: {reason}");
    }
    println!(
        "  primary controls: {}",
        package
            .control_profile
            .controls
            .iter()
            .filter(|control| control.visible && control.primary)
            .count()
    );
    println!(
        "  provider options: {}",
        package.provider_pack.provider_options.len()
    );
    println!(
        "  candidate strategies: {}",
        package.candidate_strategy_pack.strategies.len()
    );
    println!("  validation: {}", validity_label(report.is_valid()));
    for issue in &report.issues {
        println!(
            "  issue [{}] {}: {}",
            issue.code, issue.subject, issue.message
        );
    }
    Ok(())
}

fn run_preview(input: &str, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let package = load_kit_package(input)?;
    ensure_valid(&package)?;
    let fixture = fixture_for_package(&package, BuiltInProofPolicy::FullPackage)?;
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("foundry kit preview failed: {error:#?}"))?;
    let preview = render_artifact_view(&output.artifact, 35.0, 20.0, false)?;
    save_png(&preview, out_dir.join("preview.png"))?;
    write_json(out_dir.join("foundry-kit-package.json"), &package)?;
    write_json(out_dir.join("build-stamp.json"), &output.build_stamp)?;
    println!(
        "Rendered Foundry kit {} preview to {}",
        package.kit.display_name,
        out_dir.display()
    );
    Ok(())
}

fn run_contact_sheet(input: &str, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let package = load_kit_package(input)?;
    ensure_valid(&package)?;
    let fixture = fixture_for_package(&package, BuiltInProofPolicy::FullPackage)?;
    let output = compile_foundry_document(&fixture.document, &fixture)
        .map_err(|error| anyhow::anyhow!("foundry kit contact sheet failed: {error:#?}"))?;
    let front = render_artifact_view(&output.artifact, 0.0, 12.0, false)?;
    let three_quarter = render_artifact_view(&output.artifact, 35.0, 20.0, false)?;
    let side = render_artifact_view(&output.artifact, 90.0, 12.0, false)?;
    let back = render_artifact_view(&output.artifact, 180.0, 12.0, false)?;
    let wireframe = render_artifact_view(&output.artifact, 35.0, 20.0, true)?;
    save_png(&front, out_dir.join("front.png"))?;
    save_png(&three_quarter, out_dir.join("three-quarter.png"))?;
    save_png(&side, out_dir.join("side.png"))?;
    save_png(&back, out_dir.join("back.png"))?;
    save_png(&wireframe, out_dir.join("wireframe.png"))?;
    let candidates = [&three_quarter, &side, &back, &wireframe];
    save_contact_sheet(&front, &candidates, out_dir.join("contact-sheet.png"))?;
    write_json(out_dir.join("foundry-kit-package.json"), &package)?;
    println!(
        "Rendered Foundry kit {} contact sheet to {}",
        package.kit.display_name,
        out_dir.display()
    );
    Ok(())
}

fn run_package(input: &str, out_dir: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let package = load_kit_package(input)?;
    ensure_valid(&package)?;
    write_json(out_dir.join("foundry-kit-package.json"), &package)?;
    write_json(out_dir.join("kit-manifest.json"), &package.kit)?;
    write_json(
        out_dir.join("family-blueprint.json"),
        &package.family_blueprint,
    )?;
    write_json(out_dir.join("provider-pack.json"), &package.provider_pack)?;
    write_json(out_dir.join("style-pack.json"), &package.style_pack)?;
    write_json(
        out_dir.join("control-profile.json"),
        &package.control_profile,
    )?;
    write_json(
        out_dir.join("candidate-strategy-pack.json"),
        &package.candidate_strategy_pack,
    )?;
    write_json(
        out_dir.join("quality-gate-profile.json"),
        &package.quality_gate_profile,
    )?;
    write_json(
        out_dir.join("compatibility-matrix.json"),
        &package.compatibility_matrix,
    )?;
    write_json(
        out_dir.join("review-manifest.json"),
        &package.review_manifest,
    )?;
    write_json(
        out_dir.join("kit-catalog-manifest.json"),
        &package.catalog_manifest,
    )?;
    if package.kit.source_profile_slug.is_some() {
        let fixture = fixture_for_package(&package, BuiltInProofPolicy::FullPackage)?;
        let output = compile_foundry_document(&fixture.document, &fixture).map_err(|error| {
            anyhow::anyhow!("foundry kit package build proof failed: {error:#?}")
        })?;
        let proof_dir = out_dir.join("build-proof");
        fs::create_dir_all(&proof_dir)
            .with_context(|| format!("creating {}", proof_dir.display()))?;
        write_json(proof_dir.join("foundry-document.json"), &output.document)?;
        write_json(proof_dir.join("recipe.json"), &output.recipe)?;
        write_json(proof_dir.join("build-stamp.json"), &output.build_stamp)?;
        let model_config = validation_config_from_recipe_with_limits(
            &output.recipe,
            &output.artifact,
            ValidationLimits::default(),
        );
        let model_report = validate_model(&output.artifact, &model_config);
        write_json(proof_dir.join("model-validation.json"), &model_report)?;
        let grouped_obj = write_grouped_obj_export(&output.artifact, Some(&output.recipe))
            .context("writing Foundry kit grouped OBJ")?;
        fs::write(proof_dir.join("asset.obj"), grouped_obj.obj)
            .with_context(|| format!("writing asset.obj to {}", proof_dir.display()))?;
        write_json(
            proof_dir.join("grouped-obj-report.json"),
            &grouped_obj.report,
        )?;
        let package_dir = proof_dir.join("model-package");
        write_model_package(&output.recipe, &output.artifact, &package_dir)
            .with_context(|| format!("writing model package {}", package_dir.display()))?;
        let verification = verify_model_package(&package_dir)
            .with_context(|| format!("verifying model package {}", package_dir.display()))?;
        write_json(proof_dir.join("package-verification.json"), &verification)?;
        let preview = render_artifact_view(&output.artifact, 35.0, 20.0, false)?;
        save_png(&preview, proof_dir.join("preview.png"))?;
    }
    println!(
        "Packaged Foundry kit {} into {}",
        package.kit.display_name,
        out_dir.display()
    );
    Ok(())
}

fn run_review(input: &str, quality_report: &Path, out: &Path) -> anyhow::Result<()> {
    let package = load_kit_package(input)?;
    ensure_valid(&package)?;
    let _fixture = fixture_for_package(&package, BuiltInProofPolicy::BuildMetadata)?;
    let value: Value = serde_json::from_slice(
        &fs::read(quality_report)
            .with_context(|| format!("reading {}", quality_report.display()))?,
    )
    .with_context(|| format!("parsing {}", quality_report.display()))?;
    validate_quality_report_identity(&package, &value)?;
    let mut review = package.review_manifest.clone();
    review.tier_requested =
        parse_tier(value.get("quality_tier_requested")).unwrap_or(package.kit.quality_tier);
    review.tier_achieved =
        parse_tier(value.get("quality_tier_achieved")).unwrap_or(review.tier_achieved);
    review.human_approval_marker =
        value.get("human_approval_status").and_then(Value::as_str) == Some("approved");
    review.adversarial_review_marker = value
        .get("adversarial_review_status")
        .and_then(Value::as_str)
        == Some("approved");
    review.benchmark_refs = vec![quality_report.display().to_string()];
    review.contact_sheet_paths.clear();
    if value
        .get("contact_sheet_available")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let contact_sheet_path = quality_report
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("contact-sheet.png");
        if !contact_sheet_path.is_file() {
            bail!(
                "quality report claims contact_sheet_available but {} does not exist",
                contact_sheet_path.display()
            );
        }
        review.contact_sheet_paths = vec![contact_sheet_path.display().to_string()];
    }
    review.blocked_reasons = value
        .get("quality_tier_blockers")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if review.tier_achieved >= FoundryKitQualityTier::Usable && !review.human_approval_marker {
        review
            .blocked_reasons
            .push("Manual review pending before novice catalog exposure.".to_owned());
    }
    if let Some(parent) = out.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    write_json(out, &review)?;
    println!("Wrote Foundry kit review manifest to {}", out.display());
    Ok(())
}

fn validate_quality_report_identity(
    package: &FoundryKitPackage,
    value: &Value,
) -> anyhow::Result<()> {
    let expected_profile = package
        .kit
        .source_profile_slug
        .as_deref()
        .with_context(|| "kit review requires source_profile_slug metadata")?;
    let profile_id = value
        .get("profile_id")
        .and_then(Value::as_str)
        .with_context(|| "quality report is missing profile_id")?;
    if profile_id != expected_profile {
        bail!(
            "quality report profile_id '{profile_id}' does not match kit source profile '{expected_profile}'"
        );
    }
    if let Some(kit_id) = value.get("kit_id").and_then(Value::as_str)
        && kit_id != package.kit.kit_id
        && kit_id != package.style_pack.style_id
    {
        bail!(
            "quality report kit_id '{kit_id}' does not match kit '{}' or style '{}'",
            package.kit.kit_id,
            package.style_pack.style_id
        );
    }
    if let Some(style_id) = value.get("style_id").and_then(Value::as_str)
        && style_id != package.style_pack.style_id
    {
        bail!(
            "quality report style_id '{style_id}' does not match kit style '{}'",
            package.style_pack.style_id
        );
    }
    Ok(())
}

fn load_kit_package(input: &str) -> anyhow::Result<FoundryKitPackage> {
    let path = Path::new(input);
    if path.exists() {
        let file_path = if path.is_dir() {
            path.join("foundry-kit-package.json")
        } else {
            path.to_path_buf()
        };
        let bytes =
            fs::read(&file_path).with_context(|| format!("reading {}", file_path.display()))?;
        return serde_json::from_slice(&bytes)
            .with_context(|| format!("parsing {}", file_path.display()));
    }
    built_in_foundry_kit_package(input)
        .with_context(|| format!("unknown Foundry kit path or built-in slug '{input}'"))
}

fn ensure_valid(package: &FoundryKitPackage) -> anyhow::Result<()> {
    let report = validate_foundry_kit_package(package);
    if report.is_valid() {
        Ok(())
    } else {
        bail!(
            "Foundry kit validation failed with {} issue(s): {:?}",
            report.issues.len(),
            report.issues
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum BuiltInProofPolicy {
    BuildMetadata,
    FullPackage,
}

fn fixture_for_package(
    package: &FoundryKitPackage,
    proof_policy: BuiltInProofPolicy,
) -> anyhow::Result<FoundryFixtureCatalog> {
    let slug = package
        .kit
        .source_profile_slug
        .as_deref()
        .with_context(|| "kit preview requires built-in source_profile_slug metadata")?;
    let canonical = built_in_foundry_kit_package(slug)
        .with_context(|| format!("unknown built-in source profile '{slug}'"))?;
    let matches_canonical = match proof_policy {
        BuiltInProofPolicy::BuildMetadata => same_built_in_build_backing(package, &canonical),
        BuiltInProofPolicy::FullPackage => package == &canonical,
    };
    if !matches_canonical {
        bail!(
            "built-in build proof requires the canonical built-in kit package for source profile '{slug}'"
        );
    }
    built_in_fixture_catalogs_with_labels()
        .into_iter()
        .find(|(_, fixture)| fixture.slug == slug)
        .map(|(_, fixture)| fixture)
        .with_context(|| format!("unknown built-in source profile '{slug}'"))
}

fn same_built_in_build_backing(a: &FoundryKitPackage, b: &FoundryKitPackage) -> bool {
    a.schema_version == b.schema_version
        && a.kit == b.kit
        && a.family_blueprint == b.family_blueprint
        && a.provider_pack == b.provider_pack
        && a.style_pack == b.style_pack
        && a.control_profile == b.control_profile
        && a.candidate_strategy_pack == b.candidate_strategy_pack
        && a.quality_gate_profile == b.quality_gate_profile
        && a.compatibility_matrix == b.compatibility_matrix
        && a.catalog_manifest == b.catalog_manifest
}

fn render_artifact_view(
    artifact: &shape_compile::AssetArtifact,
    yaw_degrees: f32,
    pitch_degrees: f32,
    wireframe: bool,
) -> anyhow::Result<RenderedImage> {
    let preview_mesh = render_mesh_from_triangles(&artifact.combined_preview);
    let camera =
        fit_camera_to_bounds_from_angles(preview_mesh.bounds, yaw_degrees, pitch_degrees, 1.0);
    let settings = RenderSettings {
        width: 512,
        height: 512,
        wireframe,
        ..RenderSettings::default()
    };
    render_mesh(&preview_mesh, &camera, &settings).context("rendering Foundry kit preview")
}

fn parse_tier(value: Option<&Value>) -> Option<FoundryKitQualityTier> {
    match value.and_then(Value::as_str)? {
        "draft" => Some(FoundryKitQualityTier::Draft),
        "prototype" => Some(FoundryKitQualityTier::Prototype),
        "usable" => Some(FoundryKitQualityTier::Usable),
        "showcase" => Some(FoundryKitQualityTier::Showcase),
        _ => None,
    }
}

#[allow(dead_code)]
fn _review_type_anchor(review: &KitReviewManifest) -> &KitReviewManifest {
    review
}
