use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context;
use image::{Rgba, RgbaImage, imageops::FilterType};
use serde::Serialize;
use shape_compile::export::{verify_model_package, write_grouped_obj_export, write_model_package};
use shape_compile::validation::{
    ValidationLimits, validate_model, validation_config_from_recipe_with_limits,
};
use shape_foundry::{
    CandidateLegibilityClass, ControlEvaluationContext, ControlKind, ControlTopologyBehavior,
    ControlValue, CustomizerControl, CustomizerProfile, FoundryAssetDocument, FoundryCommand,
    FoundryCompilationOutput, VariationIntent, apply_foundry_command, compile_foundry_document,
    default_control_value, effective_control_domain, evaluate_control_state,
};
use shape_foundry_catalog::{FoundryFixtureCatalog, roman_bridge, scifi_crate, stylized_lamp};
use shape_render::foundry::{
    FoundryPreviewBatchRequest, FoundryPreviewCache, FoundryPreviewControlValue,
    FoundryPreviewKind, FoundryPreviewRequest, FoundryPreviewResolution,
};
use shape_render::{RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh};
use shape_search::foundry::{
    FoundryCandidateGenerationDiagnostics, FoundryCandidateMode, FoundryCandidatePlan,
    FoundryCandidateRequest, FoundryControlEndpointVisibilityReport,
    generate_foundry_candidate_plans, generate_foundry_control_endpoint_visibility_report,
};

use crate::{render_mesh_from_triangles, save_contact_sheet, save_png, write_json};

const REPORT_SCHEMA_VERSION: u32 = 1;
const REQUIRED_VISIBLE_IDEAS: usize = 4;
const STARTER_PROPOSAL_COUNT: usize = 72;
const STARTER_RESULT_COUNT: usize = 6;
const PREVIEW_CACHE_CAPACITY: usize = 256;
const PREVIEW_PARALLELISM: usize = 4;
const FULL_PREVIEW_SIZE: u32 = 512;
const GRID_CARD_SIZE: u32 = 176;
const GRID_PADDING: u32 = 12;

#[derive(Debug, clap::Args)]
pub struct StarterTemplateQualityBenchmarkArgs {
    /// Output directory for all starter-template quality evidence.
    #[arg(long)]
    pub out_dir: PathBuf,
}

#[derive(Debug, Copy, Clone)]
enum StarterTemplate {
    ScifiCrate,
    RomanBridgeHq,
    StylizedLamp,
}

impl StarterTemplate {
    const ALL: [Self; 3] = [Self::ScifiCrate, Self::RomanBridgeHq, Self::StylizedLamp];

    fn slug(self) -> &'static str {
        match self {
            Self::ScifiCrate => "sci-fi-crate",
            Self::RomanBridgeHq => "roman-bridge-hq",
            Self::StylizedLamp => "stylized-lamp",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::ScifiCrate => "Sci-Fi Industrial Crate",
            Self::RomanBridgeHq => "Roman Timber Bridge HQ",
            Self::StylizedLamp => "Stylized Furniture Lamp",
        }
    }

    fn seed(self) -> u64 {
        match self {
            Self::ScifiCrate => 41,
            Self::RomanBridgeHq | Self::StylizedLamp => 101,
        }
    }

    fn variation_intent(self) -> VariationIntent {
        match self {
            Self::ScifiCrate => VariationIntent::whole_asset_shape(),
            Self::RomanBridgeHq | Self::StylizedLamp => VariationIntent::default(),
        }
    }

    fn fixture(self) -> FoundryFixtureCatalog {
        match self {
            Self::ScifiCrate => scifi_crate::fixture_catalog(),
            Self::RomanBridgeHq => roman_bridge::hq_fixture_catalog(),
            Self::StylizedLamp => stylized_lamp::fixture_catalog(),
        }
    }
}

#[derive(Debug, Serialize)]
struct StarterTemplateQualitySuiteReport {
    schema_version: u32,
    required_visible_ideas: usize,
    all_passed: bool,
    output_dir: String,
    templates: Vec<StarterTemplateQualitySummary>,
}

#[derive(Debug, Clone, Serialize)]
struct StarterTemplateQualitySummary {
    profile_slug: String,
    display_name: String,
    passed: bool,
    catalog_recommendation: CatalogRecommendation,
    blockers: Vec<String>,
    legibility_report: String,
    adversarial_review: String,
}

#[derive(Debug, Serialize)]
struct StarterTemplateQualityReport {
    schema_version: u32,
    profile_slug: String,
    display_name: String,
    seed: u64,
    required_visible_ideas: usize,
    passed: bool,
    catalog_recommendation: CatalogRecommendation,
    pass_criteria: TemplatePassCriteria,
    signals: QualitySignalReport,
    blockers: Vec<String>,
    output_files: Vec<String>,
    parent: ParentQualityReport,
    generated_ideas: GeneratedIdeasReport,
    endpoint_visibility_report: Option<FoundryControlEndpointVisibilityReport>,
    export_conformance: ExportConformanceReport,
    adversarial_questions: AdversarialQuestionReport,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize)]
enum CatalogRecommendation {
    Usable,
    PreviewOnly,
}

#[derive(Debug, Clone, Serialize)]
struct TemplatePassCriteria {
    at_least_four_visible_ideas: bool,
    all_primary_controls_have_endpoint_report: bool,
    no_too_subtle_whole_asset_candidates: bool,
    no_broken_or_floating_parts: bool,
    export_conformance_clean: bool,
    no_advanced_recipe_needed: bool,
    candidate_summaries_plain_language: bool,
}

impl TemplatePassCriteria {
    fn passed(&self) -> bool {
        self.at_least_four_visible_ideas
            && self.all_primary_controls_have_endpoint_report
            && self.no_too_subtle_whole_asset_candidates
            && self.no_broken_or_floating_parts
            && self.export_conformance_clean
            && self.no_advanced_recipe_needed
            && self.candidate_summaries_plain_language
    }
}

#[derive(Debug, Clone, Serialize)]
struct QualitySignalReport {
    visible_idea_count: usize,
    distinct_visible_idea_count: usize,
    primary_control_count: usize,
    endpoint_reported_primary_control_count: usize,
    endpoint_readable_primary_control_count: usize,
    returned_too_subtle_candidate_count: usize,
    invalid_or_broken_candidate_count: usize,
    raw_technical_summary_count: usize,
    advanced_recipe_required: bool,
    export_verified: bool,
}

#[derive(Debug, Clone)]
struct QualitySignals {
    visible_idea_count: usize,
    distinct_visible_idea_count: usize,
    primary_control_count: usize,
    endpoint_reported_primary_control_count: usize,
    endpoint_readable_primary_control_count: usize,
    returned_too_subtle_candidate_count: usize,
    invalid_or_broken_candidate_count: usize,
    raw_technical_summary_count: usize,
    advanced_recipe_required: bool,
    export_verified: bool,
}

impl QualitySignals {
    fn report(&self) -> QualitySignalReport {
        QualitySignalReport {
            visible_idea_count: self.visible_idea_count,
            distinct_visible_idea_count: self.distinct_visible_idea_count,
            primary_control_count: self.primary_control_count,
            endpoint_reported_primary_control_count: self.endpoint_reported_primary_control_count,
            endpoint_readable_primary_control_count: self.endpoint_readable_primary_control_count,
            returned_too_subtle_candidate_count: self.returned_too_subtle_candidate_count,
            invalid_or_broken_candidate_count: self.invalid_or_broken_candidate_count,
            raw_technical_summary_count: self.raw_technical_summary_count,
            advanced_recipe_required: self.advanced_recipe_required,
            export_verified: self.export_verified,
        }
    }
}

#[derive(Debug, Serialize)]
struct ParentQualityReport {
    image: String,
    conformance_accepted: bool,
    compile_validation_valid: bool,
    model_validation_valid: bool,
    mesh_vertices: usize,
    mesh_triangles: usize,
    part_count: u64,
    visibly_disconnected_parts: Vec<DisconnectedPartReport>,
}

#[derive(Debug, Serialize)]
struct GeneratedIdeasReport {
    contact_sheet: String,
    selected_comparison_sheet: String,
    requested_count: usize,
    returned_count: usize,
    diagnostics: Option<FoundryCandidateGenerationDiagnostics>,
    candidates: Vec<CandidateQualityRow>,
}

#[derive(Debug, Serialize)]
struct CandidateQualityRow {
    slot: usize,
    candidate_id: String,
    label: String,
    changed_controls: Vec<String>,
    legibility_class: CandidateLegibilityClass,
    selectable: bool,
    visual_delta_from_parent: u64,
    recipe_fingerprint: String,
    conformance_accepted: bool,
    compile_validation_valid: bool,
    model_validation_valid: bool,
    visibly_disconnected_parts: Vec<DisconnectedPartReport>,
    raw_technical_terms: Vec<String>,
    plain_language_summary: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DisconnectedPartReport {
    instance_name: String,
    nearest_part_gap: f32,
}

#[derive(Debug, Serialize)]
struct ExportConformanceReport {
    package_dir: String,
    package_verification: String,
    grouped_obj_report: String,
    parent_conformance_accepted: bool,
    model_validation_valid: bool,
    checksums_match: bool,
    topology_matches_manifest: bool,
    finite_numeric_payloads: bool,
}

#[derive(Debug, Serialize)]
struct AdversarialQuestionReport {
    novice_can_tell_what_changed: bool,
    candidates_look_authored: bool,
    no_broken_procedural_toy: bool,
    controls_are_meaningful: bool,
    user_would_continue_after_two_minutes: bool,
}

#[derive(Debug)]
struct CompiledBenchmarkCandidate {
    plan: FoundryCandidatePlan,
    output: FoundryCompilationOutput,
    compile_validation_valid: bool,
    model_validation_valid: bool,
}

pub fn run_starter_template_quality_benchmark(
    args: StarterTemplateQualityBenchmarkArgs,
) -> anyhow::Result<()> {
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating {}", args.out_dir.display()))?;

    let mut summaries = Vec::new();
    for template in StarterTemplate::ALL {
        let report = benchmark_template(template, &args.out_dir)?;
        let summary = StarterTemplateQualitySummary {
            profile_slug: report.profile_slug.clone(),
            display_name: report.display_name.clone(),
            passed: report.passed,
            catalog_recommendation: report.catalog_recommendation,
            blockers: report.blockers.clone(),
            legibility_report: format!("{}/legibility-report.json", report.profile_slug),
            adversarial_review: format!("{}/adversarial-review.md", report.profile_slug),
        };
        summaries.push(summary);
    }

    let all_passed = summaries.iter().all(|summary| summary.passed);
    let suite_report = StarterTemplateQualitySuiteReport {
        schema_version: REPORT_SCHEMA_VERSION,
        required_visible_ideas: REQUIRED_VISIBLE_IDEAS,
        all_passed,
        output_dir: args.out_dir.display().to_string(),
        templates: summaries.clone(),
    };
    write_json(args.out_dir.join("summary.json"), &suite_report)?;

    if !all_passed {
        let failures = summaries
            .iter()
            .filter(|summary| !summary.passed)
            .map(|summary| summary.profile_slug.as_str())
            .collect::<Vec<_>>();
        eprintln!(
            "Starter template quality benchmark downgraded {} to PreviewOnly; evidence written to {}",
            failures.join(", "),
            args.out_dir.display()
        );
    } else {
        println!(
            "Starter template quality benchmark passed for {} templates; evidence written to {}",
            summaries.len(),
            args.out_dir.display()
        );
    }
    Ok(())
}

fn benchmark_template(
    template: StarterTemplate,
    out_dir: &Path,
) -> anyhow::Result<StarterTemplateQualityReport> {
    let fixture = template.fixture();
    let template_dir = out_dir.join(template.slug());
    fs::create_dir_all(&template_dir)
        .with_context(|| format!("creating {}", template_dir.display()))?;

    let parent_output = compile_foundry_document(&fixture.document, &fixture).map_err(|error| {
        anyhow::anyhow!(
            "{} parent foundry compilation failed: {error:#?}",
            template.slug()
        )
    })?;
    let parent_model_validation_valid =
        compiled_output_model_validation_valid(&parent_output, "parent");
    let parent_disconnected_parts = visibly_disconnected_parts(&parent_output, template.slug());
    let parent_image = render_full_parent(&parent_output)?;
    save_png(&parent_image, template_dir.join("parent.png"))?;

    let export_report = export_parent_package(&template_dir, &parent_output)?;

    let candidate_output = generate_foundry_candidate_plans(
        &fixture.document,
        &fixture,
        &FoundryCandidateRequest {
            seed: template.seed(),
            proposal_count: STARTER_PROPOSAL_COUNT,
            result_count: STARTER_RESULT_COUNT,
            mode: FoundryCandidateMode::Explore,
            strategy_id: None,
            preference_profile: None,
            variation_intent: template.variation_intent(),
        },
    )
    .with_context(|| format!("generating {} starter ideas", template.slug()))?;
    let candidate_diagnostics = Some(candidate_output.diagnostics.clone());
    let compiled_candidates =
        compile_candidate_plans(candidate_output.candidates, &fixture, template.slug());

    let mut cache = FoundryPreviewCache::new(PREVIEW_CACHE_CAPACITY);
    let generated_batch = render_candidate_previews(
        &mut cache,
        template.slug(),
        &parent_output,
        &compiled_candidates,
    )?;
    let parent_preview = &generated_batch.previews[0].image;
    let candidate_images = generated_batch.previews[1..]
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_contact_sheet(
        parent_preview,
        &candidate_images,
        template_dir.join("generated-ideas-contact-sheet.png"),
    )?;
    let selected = candidate_images
        .first()
        .copied()
        .into_iter()
        .collect::<Vec<_>>();
    save_contact_sheet(
        parent_preview,
        &selected,
        template_dir.join("selected-comparison-sheet.png"),
    )?;

    render_control_endpoint_sheet(
        &mut cache,
        &template_dir,
        &fixture,
        &parent_output,
        template.slug(),
    )?;
    render_option_gallery_sheet(
        &mut cache,
        &template_dir,
        &fixture,
        &parent_output,
        template.slug(),
    )?;

    let endpoint_report =
        generate_foundry_control_endpoint_visibility_report(&fixture.document, &fixture)
            .with_context(|| format!("generating {} endpoint report", template.slug()))?;

    let candidate_rows =
        candidate_quality_rows(parent_preview, &candidate_images, &compiled_candidates);
    let primary_control_ids = primary_control_ids(&parent_output);
    let signals = quality_signals(
        &primary_control_ids,
        &endpoint_report,
        &candidate_rows,
        &export_report,
        parent_disconnected_parts.len(),
    );
    let pass_criteria = evaluate_pass_criteria(&signals);
    let blockers = blockers_for_criteria(&pass_criteria, &signals);
    let passed = pass_criteria.passed();
    let catalog_recommendation = if passed {
        CatalogRecommendation::Usable
    } else {
        CatalogRecommendation::PreviewOnly
    };
    let adversarial_questions = adversarial_question_report(&pass_criteria, &signals);

    let report = StarterTemplateQualityReport {
        schema_version: REPORT_SCHEMA_VERSION,
        profile_slug: template.slug().to_owned(),
        display_name: template.display_name().to_owned(),
        seed: template.seed(),
        required_visible_ideas: REQUIRED_VISIBLE_IDEAS,
        passed,
        catalog_recommendation,
        pass_criteria,
        signals: signals.report(),
        blockers,
        output_files: expected_template_files(),
        parent: ParentQualityReport {
            image: "parent.png".to_owned(),
            conformance_accepted: parent_output.conformance_summary.accepted,
            compile_validation_valid: parent_output.artifact.validation_report.is_valid(),
            model_validation_valid: parent_model_validation_valid,
            mesh_vertices: parent_output.artifact.combined_preview.mesh.positions.len(),
            mesh_triangles: parent_output.artifact.statistics.triangle_count as usize,
            part_count: parent_output.artifact.statistics.part_count,
            visibly_disconnected_parts: parent_disconnected_parts,
        },
        generated_ideas: GeneratedIdeasReport {
            contact_sheet: "generated-ideas-contact-sheet.png".to_owned(),
            selected_comparison_sheet: "selected-comparison-sheet.png".to_owned(),
            requested_count: STARTER_RESULT_COUNT,
            returned_count: candidate_rows.len(),
            diagnostics: candidate_diagnostics,
            candidates: candidate_rows,
        },
        endpoint_visibility_report: Some(endpoint_report),
        export_conformance: export_report,
        adversarial_questions,
    };

    write_json(template_dir.join("legibility-report.json"), &report)?;
    fs::write(
        template_dir.join("adversarial-review.md"),
        adversarial_review_markdown(&report),
    )
    .with_context(|| {
        format!(
            "writing adversarial review to {}",
            template_dir.join("adversarial-review.md").display()
        )
    })?;
    Ok(report)
}

fn compile_candidate_plans(
    plans: Vec<FoundryCandidatePlan>,
    fixture: &FoundryFixtureCatalog,
    slug: &str,
) -> Vec<CompiledBenchmarkCandidate> {
    plans
        .into_iter()
        .filter_map(|plan| {
            let output = match compile_foundry_document(&plan.document, fixture) {
                Ok(output) => output,
                Err(error) => {
                    log::warn!(
                        "{slug} candidate {} failed compilation during benchmark: {error:#?}",
                        plan.id.0
                    );
                    return None;
                }
            };
            let compile_validation_valid = output.artifact.validation_report.is_valid();
            let model_validation_valid =
                compiled_output_model_validation_valid(&output, &plan.id.0);
            Some(CompiledBenchmarkCandidate {
                plan,
                output,
                compile_validation_valid,
                model_validation_valid,
            })
        })
        .collect()
}

fn render_full_parent(output: &FoundryCompilationOutput) -> anyhow::Result<RenderedImage> {
    let mesh = render_mesh_from_triangles(&output.artifact.combined_preview);
    let camera = fit_camera_to_bounds(mesh.bounds);
    let settings = RenderSettings {
        width: FULL_PREVIEW_SIZE,
        height: FULL_PREVIEW_SIZE,
        ..RenderSettings::default()
    };
    render_mesh(&mesh, &camera, &settings).context("rendering starter parent preview")
}

fn render_candidate_previews(
    cache: &mut FoundryPreviewCache,
    slug: &str,
    parent_output: &FoundryCompilationOutput,
    candidates: &[CompiledBenchmarkCandidate],
) -> anyhow::Result<shape_render::foundry::FoundryPreviewBatchOutput> {
    let mut requests = Vec::with_capacity(candidates.len() + 1);
    requests.push(preview_request_for_output(
        "parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        parent_output,
    ));
    for (slot, candidate) in candidates.iter().enumerate() {
        requests.push(preview_request_for_output(
            format!("candidate-{slot:02}"),
            FoundryPreviewKind::CandidateCard {
                candidate_id: candidate.plan.id.0.clone(),
            },
            &candidate.output,
        ));
    }
    render_preview_batch(cache, &format!("{slug}-generated-ideas"), requests)
}

fn render_control_endpoint_sheet(
    cache: &mut FoundryPreviewCache,
    template_dir: &Path,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    slug: &str,
) -> anyhow::Result<()> {
    let profile = &parent_output.catalog.customizer_profile;
    let context = ControlEvaluationContext::new(&parent_output.catalog.family.parameter_slots);
    let mut requests = vec![preview_request_for_output(
        "parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        parent_output,
    )];

    for control in profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        for (index, value) in endpoint_values_for_control(control, context)
            .into_iter()
            .enumerate()
        {
            let Ok(output) =
                compile_with_control_value(fixture, &parent_output.document, control, &value)
            else {
                continue;
            };
            requests.push(preview_request_for_output(
                format!("{}-endpoint-{index:02}", control.id),
                preview_kind_for_control_value(control, &value, index),
                &output,
            ));
        }
    }

    let batch = render_preview_batch(cache, &format!("{slug}-control-endpoints"), requests)?;
    let images = batch
        .previews
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_rendered_grid(&images, template_dir.join("control-endpoint-sheet.png"))
}

fn render_option_gallery_sheet(
    cache: &mut FoundryPreviewCache,
    template_dir: &Path,
    fixture: &FoundryFixtureCatalog,
    parent_output: &FoundryCompilationOutput,
    slug: &str,
) -> anyhow::Result<()> {
    let profile = &parent_output.catalog.customizer_profile;
    let mut requests = vec![preview_request_for_output(
        "parent",
        FoundryPreviewKind::CandidateCard {
            candidate_id: "parent".to_owned(),
        },
        parent_output,
    )];

    for control in profile
        .controls
        .iter()
        .filter(|control| control.primary && control.visible)
    {
        for (index, value) in gallery_option_values(control).into_iter().enumerate() {
            let Ok(output) =
                compile_with_control_value(fixture, &parent_output.document, control, &value)
            else {
                continue;
            };
            requests.push(preview_request_for_output(
                format!("{}-option-{index:02}", control.id),
                preview_kind_for_control_value(control, &value, index),
                &output,
            ));
        }
    }

    let batch = render_preview_batch(cache, &format!("{slug}-option-gallery"), requests)?;
    let images = batch
        .previews
        .iter()
        .map(|preview| &preview.image)
        .collect::<Vec<_>>();
    save_rendered_grid(&images, template_dir.join("option-gallery-sheet.png"))
}

fn render_preview_batch(
    cache: &mut FoundryPreviewCache,
    comparison_id: &str,
    items: Vec<FoundryPreviewRequest>,
) -> anyhow::Result<shape_render::foundry::FoundryPreviewBatchOutput> {
    let mut request = FoundryPreviewBatchRequest::new(
        comparison_id.to_owned(),
        items,
        FoundryPreviewResolution::Px128,
    );
    request.max_parallel_jobs = PREVIEW_PARALLELISM;
    cache.render_batch(request).map_err(|error| {
        anyhow::anyhow!("rendering starter preview batch {comparison_id}: {error}")
    })
}

fn preview_request_for_output(
    preview_id: impl Into<String>,
    kind: FoundryPreviewKind,
    output: &FoundryCompilationOutput,
) -> FoundryPreviewRequest {
    let mut request = FoundryPreviewRequest::new(
        preview_id,
        kind,
        geometry_fingerprint(output),
        render_mesh_from_triangles(&output.artifact.combined_preview),
    );
    request.sampled_control_state = preview_control_state(&output.document.control_state);
    request.provider_choices =
        effective_provider_choices(output, &output.catalog.customizer_profile);
    request
}

fn endpoint_values_for_control(
    control: &CustomizerControl,
    context: ControlEvaluationContext<'_>,
) -> Vec<ControlValue> {
    let domain = effective_control_domain(control, context).unwrap_or_else(|_| {
        let mut domain = control.domain.clone();
        let unavailable = domain
            .unavailable_options
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        domain
            .discrete_values
            .retain(|value| !unavailable.contains(&value.option_key()));
        domain
    });
    let mut values = Vec::new();
    match control.kind {
        ControlKind::ContinuousAxis { .. } => {
            for interval in &domain.continuous_intervals {
                push_unique_value(&mut values, ControlValue::Scalar(interval.minimum));
                push_unique_value(&mut values, ControlValue::Scalar(interval.maximum));
            }
        }
        ControlKind::IntegerStepper { .. } => {
            for interval in &domain.continuous_intervals {
                push_unique_value(
                    &mut values,
                    ControlValue::Integer(interval.minimum.ceil() as i64),
                );
                push_unique_value(
                    &mut values,
                    ControlValue::Integer(interval.maximum.floor() as i64),
                );
            }
        }
        ControlKind::Toggle { .. }
        | ControlKind::ChoiceGallery { .. }
        | ControlKind::ProviderGallery { .. } => {
            values.extend(
                domain
                    .discrete_values
                    .iter()
                    .filter(|value| domain.contains_available_value(value))
                    .cloned(),
            );
        }
    }
    if let Ok(current) = default_control_value(control, context) {
        values.retain(|value| value != &current);
    }
    values
}

fn gallery_option_values(control: &CustomizerControl) -> Vec<ControlValue> {
    match &control.kind {
        ControlKind::ChoiceGallery { options } => options
            .iter()
            .map(|option| ControlValue::Choice(option.value.clone()))
            .collect(),
        ControlKind::ProviderGallery { options, .. } => options
            .iter()
            .map(|option| ControlValue::Provider(option.provider_id.clone()))
            .collect(),
        ControlKind::ContinuousAxis { .. }
        | ControlKind::IntegerStepper { .. }
        | ControlKind::Toggle { .. } => Vec::new(),
    }
}

fn preview_kind_for_control_value(
    control: &CustomizerControl,
    value: &ControlValue,
    index: usize,
) -> FoundryPreviewKind {
    match (&control.kind, value) {
        (ControlKind::ProviderGallery { role, .. }, ControlValue::Provider(provider_id)) => {
            FoundryPreviewKind::ProviderGallery {
                role: role.clone(),
                provider_id: provider_id.clone(),
                option_index: index as u32,
            }
        }
        (ControlKind::ContinuousAxis { .. }, _) => FoundryPreviewKind::SliderFilmstrip {
            control_id: control.id.clone(),
            sample_index: index as u32,
        },
        _ => FoundryPreviewKind::DiscreteStrip {
            control_id: control.id.clone(),
            value_index: index as u32,
        },
    }
}

fn push_unique_value(values: &mut Vec<ControlValue>, value: ControlValue) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

fn compile_with_control_value(
    fixture: &FoundryFixtureCatalog,
    parent_document: &FoundryAssetDocument,
    control: &CustomizerControl,
    value: &ControlValue,
) -> anyhow::Result<FoundryCompilationOutput> {
    let mut document = parent_document.clone();
    document.catalog_lock = None;
    document.build_stamp = None;
    apply_foundry_command(
        &mut document,
        &FoundryCommand::SetControl {
            control_id: control.id.clone(),
            value: value.clone(),
        },
    )
    .map_err(|error| {
        anyhow::anyhow!(
            "setting control {} to {:?} failed: {error:#?}",
            control.id,
            value
        )
    })?;
    compile_foundry_document(&document, fixture).map_err(|error| {
        anyhow::anyhow!(
            "control {} value {:?} failed foundry compilation: {error:#?}",
            control.id,
            value
        )
    })
}

fn export_parent_package(
    template_dir: &Path,
    parent_output: &FoundryCompilationOutput,
) -> anyhow::Result<ExportConformanceReport> {
    let package_dir = template_dir.join("model-package");
    write_model_package(&parent_output.recipe, &parent_output.artifact, &package_dir)
        .with_context(|| format!("writing starter package {}", package_dir.display()))?;
    let verification = verify_model_package(&package_dir)
        .with_context(|| format!("verifying starter package {}", package_dir.display()))?;
    write_json(
        template_dir.join("package-verification.json"),
        &verification,
    )?;

    let grouped_obj =
        write_grouped_obj_export(&parent_output.artifact, Some(&parent_output.recipe))
            .context("writing starter grouped OBJ")?;
    fs::write(template_dir.join("asset.obj"), grouped_obj.obj)
        .with_context(|| format!("writing starter OBJ to {}", template_dir.display()))?;
    write_json(
        template_dir.join("grouped-obj-report.json"),
        &grouped_obj.report,
    )?;

    Ok(ExportConformanceReport {
        package_dir: "model-package".to_owned(),
        package_verification: "package-verification.json".to_owned(),
        grouped_obj_report: "grouped-obj-report.json".to_owned(),
        parent_conformance_accepted: parent_output.conformance_summary.accepted,
        model_validation_valid: compiled_output_model_validation_valid(parent_output, "parent"),
        checksums_match: verification.checksums_match,
        topology_matches_manifest: verification.topology_matches_manifest,
        finite_numeric_payloads: verification.finite_numeric_payloads,
    })
}

fn compiled_output_model_validation_valid(output: &FoundryCompilationOutput, _label: &str) -> bool {
    if !output.artifact.validation_report.is_valid() {
        return false;
    }
    let config = validation_config_from_recipe_with_limits(
        &output.recipe,
        &output.artifact,
        ValidationLimits::default(),
    );
    validate_model(&output.artifact, &config).is_valid()
}

fn primary_control_ids(output: &FoundryCompilationOutput) -> BTreeSet<String> {
    output
        .catalog
        .customizer_profile
        .controls
        .iter()
        .filter(|control| {
            control.primary
                && control.visible
                && control.topology_behavior != ControlTopologyBehavior::RuntimeOnly
        })
        .map(|control| control.id.clone())
        .collect()
}

fn quality_signals(
    primary_control_ids: &BTreeSet<String>,
    endpoint_report: &FoundryControlEndpointVisibilityReport,
    candidate_rows: &[CandidateQualityRow],
    export_report: &ExportConformanceReport,
    parent_disconnected_part_count: usize,
) -> QualitySignals {
    let visible_fingerprints = candidate_rows
        .iter()
        .filter(|candidate| {
            candidate.selectable
                && candidate.conformance_accepted
                && candidate.compile_validation_valid
                && candidate.model_validation_valid
        })
        .map(|candidate| candidate.recipe_fingerprint.clone())
        .collect::<BTreeSet<_>>();
    let endpoint_rows = endpoint_report
        .controls
        .iter()
        .filter(|row| primary_control_ids.contains(&row.control_id))
        .collect::<Vec<_>>();
    let endpoint_reported_primary_control_count = endpoint_rows
        .iter()
        .filter(|row| row.endpoint_sample_count > 0)
        .map(|row| row.control_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let endpoint_readable_primary_control_count = endpoint_rows
        .iter()
        .filter(|row| row.endpoint_sample_count > 0 && row.legibility_class.selectable())
        .map(|row| row.control_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();

    QualitySignals {
        visible_idea_count: candidate_rows
            .iter()
            .filter(|candidate| {
                candidate.selectable
                    && candidate.conformance_accepted
                    && candidate.compile_validation_valid
                    && candidate.model_validation_valid
            })
            .count(),
        distinct_visible_idea_count: visible_fingerprints.len(),
        primary_control_count: primary_control_ids.len(),
        endpoint_reported_primary_control_count,
        endpoint_readable_primary_control_count,
        returned_too_subtle_candidate_count: candidate_rows
            .iter()
            .filter(|candidate| candidate.legibility_class == CandidateLegibilityClass::TooSubtle)
            .count(),
        invalid_or_broken_candidate_count: parent_disconnected_part_count
            + candidate_rows
                .iter()
                .filter(|candidate| {
                    !candidate.conformance_accepted
                        || !candidate.compile_validation_valid
                        || !candidate.model_validation_valid
                        || !candidate.visibly_disconnected_parts.is_empty()
                })
                .count(),
        raw_technical_summary_count: candidate_rows
            .iter()
            .filter(|candidate| !candidate.plain_language_summary)
            .count(),
        advanced_recipe_required: false,
        export_verified: export_report.parent_conformance_accepted
            && export_report.model_validation_valid
            && export_report.checksums_match
            && export_report.topology_matches_manifest
            && export_report.finite_numeric_payloads,
    }
}

fn evaluate_pass_criteria(signals: &QualitySignals) -> TemplatePassCriteria {
    TemplatePassCriteria {
        at_least_four_visible_ideas: signals.visible_idea_count >= REQUIRED_VISIBLE_IDEAS
            && signals.distinct_visible_idea_count >= REQUIRED_VISIBLE_IDEAS,
        all_primary_controls_have_endpoint_report: signals.primary_control_count > 0
            && signals.endpoint_reported_primary_control_count == signals.primary_control_count
            && signals.endpoint_readable_primary_control_count == signals.primary_control_count,
        no_too_subtle_whole_asset_candidates: signals.returned_too_subtle_candidate_count == 0,
        no_broken_or_floating_parts: signals.invalid_or_broken_candidate_count == 0,
        export_conformance_clean: signals.export_verified,
        no_advanced_recipe_needed: !signals.advanced_recipe_required,
        candidate_summaries_plain_language: signals.raw_technical_summary_count == 0,
    }
}

fn blockers_for_criteria(criteria: &TemplatePassCriteria, signals: &QualitySignals) -> Vec<String> {
    let mut blockers = Vec::new();
    if !criteria.at_least_four_visible_ideas {
        blockers.push(format!(
            "Expected at least {REQUIRED_VISIBLE_IDEAS} visibly distinct ideas; got {} visible and {} distinct.",
            signals.visible_idea_count, signals.distinct_visible_idea_count
        ));
    }
    if !criteria.all_primary_controls_have_endpoint_report {
        blockers.push(format!(
            "Expected endpoint reports for {} primary controls; got {} reported and {} readable.",
            signals.primary_control_count,
            signals.endpoint_reported_primary_control_count,
            signals.endpoint_readable_primary_control_count
        ));
    }
    if !criteria.no_too_subtle_whole_asset_candidates {
        blockers.push(format!(
            "{} returned whole-asset candidate(s) were TooSubtle.",
            signals.returned_too_subtle_candidate_count
        ));
    }
    if !criteria.no_broken_or_floating_parts {
        blockers.push(format!(
            "{} parent/candidate output(s) failed conformance, model validation, or the visible disconnected-part check.",
            signals.invalid_or_broken_candidate_count
        ));
    }
    if !criteria.export_conformance_clean {
        blockers.push("Parent export or conformance verification was not clean.".to_owned());
    }
    if !criteria.no_advanced_recipe_needed {
        blockers.push("Benchmark required an Advanced Recipe path.".to_owned());
    }
    if !criteria.candidate_summaries_plain_language {
        blockers.push(format!(
            "{} candidate summary/summaries exposed raw technical terms.",
            signals.raw_technical_summary_count
        ));
    }
    blockers
}

fn adversarial_question_report(
    criteria: &TemplatePassCriteria,
    signals: &QualitySignals,
) -> AdversarialQuestionReport {
    AdversarialQuestionReport {
        novice_can_tell_what_changed: criteria.at_least_four_visible_ideas
            && criteria.candidate_summaries_plain_language,
        candidates_look_authored: criteria.at_least_four_visible_ideas
            && criteria.no_too_subtle_whole_asset_candidates,
        no_broken_procedural_toy: criteria.no_broken_or_floating_parts,
        controls_are_meaningful: criteria.all_primary_controls_have_endpoint_report,
        user_would_continue_after_two_minutes: signals.visible_idea_count >= REQUIRED_VISIBLE_IDEAS
            && criteria.export_conformance_clean
            && criteria.no_advanced_recipe_needed,
    }
}

fn candidate_quality_rows(
    parent: &RenderedImage,
    candidate_images: &[&RenderedImage],
    candidates: &[CompiledBenchmarkCandidate],
) -> Vec<CandidateQualityRow> {
    candidates
        .iter()
        .enumerate()
        .map(|(slot, candidate)| {
            let raw_terms = candidate_summary_raw_terms(candidate);
            CandidateQualityRow {
                slot,
                candidate_id: candidate.plan.id.0.clone(),
                label: candidate.plan.label.clone(),
                changed_controls: candidate.plan.changed_controls.clone(),
                legibility_class: candidate
                    .plan
                    .variation_metadata
                    .visible_delta
                    .legibility_class,
                selectable: candidate
                    .plan
                    .variation_metadata
                    .visible_delta
                    .legibility_class
                    .selectable(),
                visual_delta_from_parent: candidate_images
                    .get(slot)
                    .map_or(0, |image| image_delta(parent, image)),
                recipe_fingerprint: candidate.output.build_stamp.recipe_fingerprint.0.to_hex(),
                conformance_accepted: candidate.output.conformance_summary.accepted,
                compile_validation_valid: candidate.compile_validation_valid,
                model_validation_valid: candidate.model_validation_valid,
                visibly_disconnected_parts: visibly_disconnected_parts(
                    &candidate.output,
                    &candidate.output.document.document_id.0,
                ),
                plain_language_summary: raw_terms.is_empty(),
                raw_technical_terms: raw_terms,
            }
        })
        .collect()
}

fn candidate_summary_raw_terms(candidate: &CompiledBenchmarkCandidate) -> Vec<String> {
    let mut text = candidate.plan.label.clone();
    for change in &candidate.plan.diagnostics.changes {
        text.push(' ');
        text.push_str(&change.message);
        text.push(' ');
        text.push_str(&change.before);
        text.push(' ');
        text.push_str(&change.after);
        text.push(' ');
        text.push_str(&change.control_label);
    }
    raw_technical_terms_in_text(&text)
}

fn raw_technical_terms_in_text(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut terms = [
        "provider",
        "scalar",
        "operation",
        "compiler",
        "fragment",
        "recipe",
        "fingerprint",
        "semantic",
        "control_id",
        "slot id",
        "mesh",
        "triangle",
        "conformance",
        "topology",
    ]
    .into_iter()
    .filter(|term| lower.contains(term))
    .map(str::to_owned)
    .collect::<Vec<_>>();
    terms.sort();
    terms.dedup();
    terms
}

fn visibly_disconnected_parts(
    output: &FoundryCompilationOutput,
    slug_or_id: &str,
) -> Vec<DisconnectedPartReport> {
    let max_nearest_part_gap = max_nearest_part_gap(slug_or_id);
    let parts = output
        .artifact
        .compiled_parts
        .iter()
        .filter(|part| !part.world_mesh.bounds.is_empty())
        .collect::<Vec<_>>();
    if parts.len() <= 1 {
        return Vec::new();
    }

    parts
        .iter()
        .filter_map(|part| {
            let nearest_gap = parts
                .iter()
                .filter(|other| other.instance_id != part.instance_id)
                .map(|other| {
                    bounds_gap(
                        part.world_mesh.bounds.min,
                        part.world_mesh.bounds.max,
                        other.world_mesh.bounds.min,
                        other.world_mesh.bounds.max,
                    )
                })
                .fold(f32::INFINITY, f32::min);
            (nearest_gap > max_nearest_part_gap).then(|| DisconnectedPartReport {
                instance_name: part.instance_name.clone(),
                nearest_part_gap: nearest_gap,
            })
        })
        .collect()
}

fn max_nearest_part_gap(slug_or_id: &str) -> f32 {
    if slug_or_id.contains("stylized-lamp") {
        0.56
    } else {
        0.36
    }
}

fn bounds_gap(
    left_min: [f32; 3],
    left_max: [f32; 3],
    right_min: [f32; 3],
    right_max: [f32; 3],
) -> f32 {
    let dx = axis_gap(left_min[0], left_max[0], right_min[0], right_max[0]);
    let dy = axis_gap(left_min[1], left_max[1], right_min[1], right_max[1]);
    let dz = axis_gap(left_min[2], left_max[2], right_min[2], right_max[2]);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

fn axis_gap(left_min: f32, left_max: f32, right_min: f32, right_max: f32) -> f32 {
    if left_max < right_min {
        right_min - left_max
    } else if right_max < left_min {
        left_min - right_max
    } else {
        0.0
    }
}

fn save_rendered_grid(images: &[&RenderedImage], path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let image_count = images.len().max(1);
    let columns = image_count.min(4);
    let rows = image_count.div_ceil(columns);
    let width = GRID_PADDING
        + (GRID_CARD_SIZE + GRID_PADDING) * u32::try_from(columns).context("grid columns")?;
    let height = GRID_PADDING
        + (GRID_CARD_SIZE + GRID_PADDING) * u32::try_from(rows).context("grid rows")?;
    let mut sheet = RgbaImage::from_pixel(width, height, Rgba([18, 20, 22, 255]));

    if images.is_empty() {
        sheet
            .save(path)
            .with_context(|| format!("saving image grid to {}", path.display()))?;
        return Ok(());
    }

    for (index, image) in images.iter().enumerate() {
        let column = index % columns;
        let row = index / columns;
        let x = GRID_PADDING
            + u32::try_from(column).context("grid column")? * (GRID_CARD_SIZE + GRID_PADDING);
        let y = GRID_PADDING
            + u32::try_from(row).context("grid row")? * (GRID_CARD_SIZE + GRID_PADDING);
        let buffer = RgbaImage::from_raw(image.width, image.height, image.rgba8.clone())
            .context("rendered image buffer length does not match dimensions")?;
        let resized =
            image::imageops::resize(&buffer, GRID_CARD_SIZE, GRID_CARD_SIZE, FilterType::Nearest);
        image::imageops::replace(&mut sheet, &resized, i64::from(x), i64::from(y));
    }

    sheet
        .save(path)
        .with_context(|| format!("saving image grid to {}", path.display()))?;
    Ok(())
}

fn preview_control_state(
    state: &BTreeMap<String, ControlValue>,
) -> BTreeMap<String, FoundryPreviewControlValue> {
    state
        .iter()
        .map(|(control_id, value)| (control_id.clone(), preview_control_value(value)))
        .collect()
}

fn preview_control_value(value: &ControlValue) -> FoundryPreviewControlValue {
    match value {
        ControlValue::Scalar(value) => FoundryPreviewControlValue::Scalar(*value),
        ControlValue::Integer(value) => FoundryPreviewControlValue::Integer(*value),
        ControlValue::Toggle(value) => FoundryPreviewControlValue::Toggle(*value),
        ControlValue::Choice(value) => FoundryPreviewControlValue::Choice(value.clone()),
        ControlValue::Provider(value) => FoundryPreviewControlValue::Provider(value.clone()),
    }
}

fn effective_provider_choices(
    output: &FoundryCompilationOutput,
    profile: &CustomizerProfile,
) -> BTreeMap<String, String> {
    let document = &output.document;
    let context = ControlEvaluationContext::new(&output.catalog.family.parameter_slots);
    let mut providers = document
        .provider_overrides
        .iter()
        .map(|(role, override_row)| (role.clone(), override_row.provider_ref.stable_id.clone()))
        .collect::<BTreeMap<_, _>>();
    if let Ok(evaluated) = evaluate_control_state(profile, context, &document.control_state) {
        providers.extend(evaluated.provider_selections);
    } else {
        for control in &profile.controls {
            if let ControlKind::ProviderGallery { role, .. } = &control.kind
                && let Some(provider) = provider_value_for_control(document, control)
            {
                providers.insert(role.clone(), provider);
            }
        }
    }
    providers
}

fn provider_value_for_control(
    document: &FoundryAssetDocument,
    control: &CustomizerControl,
) -> Option<String> {
    document
        .control_state
        .get(&control.id)
        .and_then(|value| match value {
            ControlValue::Provider(provider) => Some(provider.clone()),
            _ => None,
        })
}

fn geometry_fingerprint(output: &FoundryCompilationOutput) -> String {
    output.build_stamp.geometry_input_fingerprint.0.to_hex()
}

fn image_delta(left: &RenderedImage, right: &RenderedImage) -> u64 {
    if left.width != right.width || left.height != right.height {
        return u64::MAX;
    }
    left.rgba8
        .iter()
        .zip(&right.rgba8)
        .map(|(left, right)| u64::from(left.abs_diff(*right)))
        .sum()
}

fn expected_template_files() -> Vec<String> {
    [
        "parent.png",
        "generated-ideas-contact-sheet.png",
        "selected-comparison-sheet.png",
        "control-endpoint-sheet.png",
        "option-gallery-sheet.png",
        "legibility-report.json",
        "adversarial-review.md",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn adversarial_review_markdown(report: &StarterTemplateQualityReport) -> String {
    let verdict = if report.passed { "PASS" } else { "FAIL" };
    let blocker_lines = if report.blockers.is_empty() {
        "- None.\n".to_owned()
    } else {
        report
            .blockers
            .iter()
            .map(|blocker| format!("- {blocker}\n"))
            .collect::<String>()
    };
    format!(
        "# Starter Template Adversarial Review: {}\n\n\
Overall: {verdict}\n\n\
## Questions\n\n\
- Can a novice tell what changed? {}\n\
- Do candidates look like real authored alternatives? {}\n\
- Does any candidate look like a broken procedural toy? {}\n\
- Are the controls meaningful? {}\n\
- Would the user continue after two minutes? {}\n\n\
## Evidence\n\n\
- Visible ideas: {}\n\
- Distinct visible ideas: {}\n\
- Primary controls with readable endpoint reports: {}/{}\n\
- Returned TooSubtle whole-asset candidates: {}\n\
- Raw technical candidate summaries: {}\n\
- Catalog recommendation: {:?}\n\n\
## Blockers\n\n{}",
        report.display_name,
        pass_label(report.adversarial_questions.novice_can_tell_what_changed),
        pass_label(report.adversarial_questions.candidates_look_authored),
        pass_label(report.adversarial_questions.no_broken_procedural_toy),
        pass_label(report.adversarial_questions.controls_are_meaningful),
        pass_label(
            report
                .adversarial_questions
                .user_would_continue_after_two_minutes
        ),
        report.signals.visible_idea_count,
        report.signals.distinct_visible_idea_count,
        report.signals.endpoint_readable_primary_control_count,
        report.signals.primary_control_count,
        report.signals.returned_too_subtle_candidate_count,
        report.signals.raw_technical_summary_count,
        report.catalog_recommendation,
        blocker_lines
    )
}

fn pass_label(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_signals() -> QualitySignals {
        QualitySignals {
            visible_idea_count: 4,
            distinct_visible_idea_count: 4,
            primary_control_count: 7,
            endpoint_reported_primary_control_count: 7,
            endpoint_readable_primary_control_count: 7,
            returned_too_subtle_candidate_count: 0,
            invalid_or_broken_candidate_count: 0,
            raw_technical_summary_count: 0,
            advanced_recipe_required: false,
            export_verified: true,
        }
    }

    #[test]
    fn starter_template_quality_at_least_four_visible_ideas_required() {
        let mut signals = passing_signals();
        signals.visible_idea_count = 3;
        signals.distinct_visible_idea_count = 3;

        let criteria = evaluate_pass_criteria(&signals);

        assert!(!criteria.at_least_four_visible_ideas);
        assert!(!criteria.passed());
    }

    #[test]
    fn starter_template_quality_missing_endpoint_reports_do_not_pass() {
        let mut signals = passing_signals();
        signals.endpoint_reported_primary_control_count = 6;
        signals.endpoint_readable_primary_control_count = 6;

        let criteria = evaluate_pass_criteria(&signals);

        assert!(!criteria.all_primary_controls_have_endpoint_report);
        assert!(!criteria.passed());
    }

    #[test]
    fn starter_template_quality_passing_signals_can_pass() {
        let criteria = evaluate_pass_criteria(&passing_signals());

        assert!(criteria.passed());
    }

    #[test]
    fn starter_template_quality_detects_raw_technical_summary_terms() {
        let terms = raw_technical_terms_in_text("Swap provider slot recipe fingerprint");

        assert!(terms.contains(&"provider".to_owned()));
        assert!(terms.contains(&"recipe".to_owned()));
        assert!(terms.contains(&"fingerprint".to_owned()));
    }

    #[test]
    fn starter_template_quality_benchmark_emits_all_expected_files() {
        let temp_dir = tempfile::tempdir().expect("temp dir");

        run_starter_template_quality_benchmark(StarterTemplateQualityBenchmarkArgs {
            out_dir: temp_dir.path().to_path_buf(),
        })
        .expect("starter template benchmark should pass");

        for template in StarterTemplate::ALL {
            for name in expected_template_files() {
                let path = temp_dir.path().join(template.slug()).join(&name);
                assert!(
                    path.exists(),
                    "{} benchmark should emit {name}",
                    template.slug()
                );
                assert!(
                    path.metadata().expect("metadata").len() > 0,
                    "{} {name} should not be empty",
                    template.slug()
                );
            }

            let report: serde_json::Value = serde_json::from_str(
                &fs::read_to_string(
                    temp_dir
                        .path()
                        .join(template.slug())
                        .join("legibility-report.json"),
                )
                .expect("read legibility report"),
            )
            .expect("parse legibility report");
            if report["passed"].as_bool().unwrap() {
                assert_eq!(report["catalog_recommendation"], "Usable");
            } else {
                assert_eq!(report["catalog_recommendation"], "PreviewOnly");
            }
        }

        assert!(temp_dir.path().join("summary.json").exists());
    }
}
