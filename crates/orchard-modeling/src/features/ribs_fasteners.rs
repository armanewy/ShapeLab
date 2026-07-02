
/// Build repeated ribs.
pub fn build_rib_feature(
    feature: &RibFeature,
    context: &GeneratorContext,
) -> FeatureResult<SemanticFeatureBuild> {
    let placements = span_frames(
        feature.start,
        feature.end,
        feature.count,
        feature.spacing,
        feature.up_hint,
        "rib",
    )?;
    let prototype = build_rib_prototype(feature, context)?;
    let definition = literal_definition(
        context.part_definition,
        "Rib Feature",
        &prototype,
        &["semantic-feature", "rib"],
    );
    let mut definitions = BTreeMap::new();
    definitions.insert(definition.id, definition);
    let mut generated_parts = BTreeMap::new();
    generated_parts.insert(context.part_definition, prototype.clone());
    let instances = feature_instances(
        "Rib",
        context.part_definition,
        context.part_instance,
        feature.operation,
        &placements,
        &["semantic-feature", "rib"],
    );
    let combined_part = if feature.mode == RibBuildMode::CombinedPart {
        Some(combine_feature_instances(
            &prototype,
            &placements,
            feature.operation,
            context.part_definition,
            context.part_instance,
            "ribs_combined",
        )?)
    } else {
        None
    };
    Ok(SemanticFeatureBuild {
        definitions,
        generated_parts,
        instances,
        combined_part,
        provenance: SemanticFeatureProvenance {
            feature: "rib".to_owned(),
            operation: feature.operation,
            definition_ids: vec![context.part_definition],
            instance_ids: placements
                .iter()
                .enumerate()
                .map(|(index, _)| PartInstanceId(context.part_instance.0 + index as u64))
                .collect(),
        },
    })
}

/// Fastener prototype geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FastenerPrototype {
    /// Cylinder prototype.
    Cylinder {
        /// Radius.
        radius: f32,
        /// Height along local Y.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
    /// Frustum prototype.
    Frustum {
        /// Bottom radius.
        bottom_radius: f32,
        /// Top radius.
        top_radius: f32,
        /// Height along local Y.
        height: f32,
        /// Radial segment count.
        radial_segments: u32,
    },
}

/// Fastener placement pattern.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FastenerPlacement {
    /// Linear placement between two points.
    Linear {
        /// Start of the span.
        start: [f32; 3],
        /// End of the span.
        end: [f32; 3],
        /// Number of instances.
        count: u32,
        /// Center-to-center spacing behavior.
        spacing: PatternSpacing,
        /// Up hint for instance orientation.
        up_hint: [f32; 3],
    },
    /// Radial placement around an axis.
    Radial {
        /// Pattern center.
        center: [f32; 3],
        /// Radius from center to each instance.
        radius: f32,
        /// Axis of the radial pattern.
        axis: [f32; 3],
        /// Number of instances.
        count: u32,
        /// Starting angle in degrees.
        start_angle_degrees: f32,
        /// Angular spacing in degrees.
        angular_spacing_degrees: f32,
    },
    /// Perimeter placement along a polyline.
    Perimeter {
        /// Ordered perimeter points.
        points: Vec<[f32; 3]>,
        /// Whether the last point connects to the first.
        closed: bool,
        /// Number of instances.
        count: u32,
        /// Center-to-center spacing behavior.
        spacing: PatternSpacing,
        /// Up hint for instance orientation.
        up_hint: [f32; 3],
    },
}

/// Repeated fastener feature with a shared prototype definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FastenerPattern {
    /// Stable operation ID used as face provenance.
    pub operation: OperationId,
    /// Shared prototype geometry.
    pub prototype: FastenerPrototype,
    /// Placement pattern.
    pub placement: FastenerPlacement,
}

/// Build a repeated fastener pattern.
pub fn build_fastener_pattern(
    feature: &FastenerPattern,
    context: &GeneratorContext,
) -> FeatureResult<SemanticFeatureBuild> {
    let prototype = build_fastener_prototype(feature, context)?;
    let placements = fastener_frames(&feature.placement)?;
    let definition = literal_definition(
        context.part_definition,
        "Fastener Pattern Prototype",
        &prototype,
        &["semantic-feature", "fastener", "shared-prototype"],
    );
    let mut definitions = BTreeMap::new();
    definitions.insert(definition.id, definition);
    let mut generated_parts = BTreeMap::new();
    generated_parts.insert(context.part_definition, prototype);
    let instances = feature_instances(
        "Fastener",
        context.part_definition,
        context.part_instance,
        feature.operation,
        &placements,
        &["semantic-feature", "fastener"],
    );
    Ok(SemanticFeatureBuild {
        definitions,
        generated_parts,
        instances,
        combined_part: None,
        provenance: SemanticFeatureProvenance {
            feature: "fastener".to_owned(),
            operation: feature.operation,
            definition_ids: vec![context.part_definition],
            instance_ids: placements
                .iter()
                .enumerate()
                .map(|(index, _)| PartInstanceId(context.part_instance.0 + index as u64))
                .collect(),
        },
    })
}

fn build_rib_prototype(
    feature: &RibFeature,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    match &feature.profile {
        RibProfile::Plate {
            width,
            height,
            thickness,
        } => {
            let width = positive(*width, "rib", "width")?;
            let height = positive(*height, "rib", "height")?;
            let thickness = positive(*thickness, "rib", "thickness")?;
            build_extruded_profile_part(
                &[
                    [width * 0.5, height * 0.5],
                    [-width * 0.5, height * 0.5],
                    [-width * 0.5, -height * 0.5],
                    [width * 0.5, -height * 0.5],
                ],
                thickness,
                feature.operation,
                context,
                "rib_plate",
            )
        }
        RibProfile::ExtrudedProfile { profile, depth } => {
            let profile = normalize_profile(profile, "rib")?;
            let depth = positive(*depth, "rib", "depth")?;
            build_extruded_profile_part(&profile, depth, feature.operation, context, "rib_profile")
        }
    }
}

fn build_extruded_profile_part(
    profile: &[[f32; 2]],
    depth: f32,
    operation: OperationId,
    context: &GeneratorContext,
    label: &'static str,
) -> FeatureResult<GeneratedPart> {
    let half_depth = depth * 0.5;
    let mut builder = MeshBuilder::new();
    let back = profile
        .iter()
        .map(|point| [point[0], -half_depth, point[1]])
        .collect::<Vec<_>>();
    let front = profile
        .iter()
        .map(|point| [point[0], half_depth, point[1]])
        .collect::<Vec<_>>();
    let back = builder.add_positions(&back)?;
    let front = builder.add_positions(&front)?;
    add_loop_band(
        &mut builder,
        &back,
        &front,
        FaceStyle {
            operation,
            region: RIB_EDGE_REGION,
            role: SurfaceRole::Side,
            smoothing_group: Some(1),
        },
        true,
    );
    add_cap_face(
        &mut builder,
        &front,
        operation,
        RIB_FRONT_REGION,
        SurfaceRole::PrimarySurface,
        true,
    );
    add_cap_face(
        &mut builder,
        &back,
        operation,
        RIB_BACK_REGION,
        SurfaceRole::PrimarySurface,
        false,
    );
    let mut mesh = builder.finish(operation)?;
    assign_part_context(&mut mesh, context);
    Ok(generated_part(
        mesh,
        feature_regions([
            (
                RIB_FRONT_REGION,
                "rib_front",
                SurfaceRole::PrimarySurface,
                &["rib", "front"][..],
            ),
            (
                RIB_BACK_REGION,
                "rib_back",
                SurfaceRole::PrimarySurface,
                &["rib", "back"][..],
            ),
            (
                RIB_EDGE_REGION,
                "rib_edge",
                SurfaceRole::Side,
                &["rib", "edge"][..],
            ),
        ]),
        BTreeMap::new(),
        format!("{label}:v1:profile={}:depth={depth:.6}", profile.len()),
    ))
}

fn build_fastener_prototype(
    feature: &FastenerPattern,
    context: &GeneratorContext,
) -> FeatureResult<GeneratedPart> {
    let mut part = match feature.prototype {
        FastenerPrototype::Cylinder {
            radius,
            height,
            radial_segments,
        } => {
            let params = CylinderParams {
                radius,
                half_height: height * 0.5,
                radial_segments,
                height_segments: 1,
                cap_mode: BasicCapMode::Both,
                top_bevel_radius: 0.0,
                bottom_bevel_radius: 0.0,
                bevel_segments: 0,
            };
            build_cylinder(&params, context)?
        }
        FastenerPrototype::Frustum {
            bottom_radius,
            top_radius,
            height,
            radial_segments,
        } => {
            let params = FrustumParams {
                bottom_radius,
                top_radius,
                half_height: height * 0.5,
                radial_segments,
                height_segments: 1,
                cap_mode: BasicCapMode::Both,
                top_bevel_radius: 0.0,
                bottom_bevel_radius: 0.0,
                bevel_segments: 0,
            };
            build_frustum(&params, context)?
        }
    };
    assign_operation(&mut part.mesh, feature.operation)?;
    part.generator_signature = format!(
        "fastener_pattern:v1:prototype={:?}:operation={}",
        feature.prototype, feature.operation.0
    );
    assign_part_context(&mut part.mesh, context);
    Ok(part)
}
