use super::*;

pub(super) fn profile_description(slug: &str) -> &'static str {
    match slug {
        "box-primitive" => BOX_PRIMITIVE_HOME_SUBTITLE,
        "lidded-box" => "A simple box with a visible lid seam.",
        "flat-panel-primitive" => {
            "One upright clay panel with readable width, height, and thickness."
        }
        "sphere-primitive" => {
            "One closed round clay volume with readable dimensions and flattening."
        }
        "hinged-panel" => "One upright clay panel with a visible hinge edge.",
        "handled-panel" => "One upright clay panel with a visible hinge edge and handle.",
        "panel-with-knob" => {
            "One upright clay panel with a bounded knob-like sphere form attached through a safe anchor."
        }
        _ => "A simple clay starting point ready for idea generation.",
    }
}

pub(super) fn profile_control_copy(slug: &str) -> &'static str {
    match slug {
        "lidded-box" => "You can vary proportions, edge softness, and lid seam.",
        "hinged-panel" => "You can vary proportions, edge softness, and hinge edge.",
        "handled-panel" => "You can vary proportions, edge softness, hinge edge, and handle.",
        "panel-with-knob" => "You can adjust panel size, knob form, and bounded knob position.",
        _ => HOME_CONTROL_COPY,
    }
}

pub(super) fn is_visible_starter_profile(slug: &str) -> bool {
    matches!(
        slug,
        BOX_PRIMITIVE_PROFILE_ID
            | LIDDED_BOX_PROFILE_ID
            | FLAT_PANEL_PRIMITIVE_PROFILE_ID
            | SPHERE_PRIMITIVE_PROFILE_ID
            | HINGED_PANEL_PROFILE_ID
            | HANDLED_PANEL_PROFILE_ID
            | PANEL_KNOB_PROFILE_ID
    )
}

#[derive(Clone)]
pub(super) struct ProductHomeProfile {
    pub(super) label: String,
    pub(super) fixture: FoundryFixtureCatalog,
    pub(super) family_id: String,
    pub(super) family_name: String,
    pub(super) style_name: String,
    pub(super) category_chips: Vec<String>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(super) enum StartingPointStatus {
    Active,
    Preview,
    InternalEvidence,
    HistoricalProof,
}

impl StartingPointStatus {
    const fn label(self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Preview => "Preview",
            Self::InternalEvidence => "Internal evidence",
            Self::HistoricalProof => "Historical proof",
        }
    }
}

#[derive(Clone)]
pub(super) struct DerivedStartingPoint {
    pub(super) display_name: &'static str,
    pub(super) profile: Option<ProductHomeProfile>,
    pub(super) derived_from_label: &'static str,
    pub(super) derivation_summary: &'static str,
    pub(super) status: StartingPointStatus,
    pub(super) preset: bool,
}

#[derive(Clone)]
pub(super) struct StartingPointGroup {
    pub(super) source_primitive_slug: &'static str,
    pub(super) display_name: &'static str,
    pub(super) description: &'static str,
    pub(super) status: StartingPointStatus,
    pub(super) primitive_profile: Option<ProductHomeProfile>,
    pub(super) derived_items: Vec<DerivedStartingPoint>,
}

pub(super) fn product_home_profiles(developer_preview_enabled: bool) -> Vec<ProductHomeProfile> {
    let cards = built_in_kit_card_views();
    curated_fixture_catalogs_with_labels(developer_preview_enabled)
        .into_iter()
        .filter(|(_label, fixture)| {
            developer_preview_enabled || fixture.slug.as_str() != HANDLED_PANEL_PROFILE_ID
        })
        .filter_map(|(_label, fixture)| {
            let card = cards
                .iter()
                .find(|card| card.source_profile_slug.as_deref() == Some(fixture.slug.as_str()))?;
            Some(ProductHomeProfile {
                label: card.display_name.clone(),
                fixture,
                family_id: card.family_id.clone(),
                family_name: card.family_name.clone(),
                style_name: card.style_name.clone(),
                category_chips: card.category_chips.clone(),
            })
        })
        .collect()
}

pub(super) fn product_home_starting_point_groups(
    profiles: &[ProductHomeProfile],
    developer_preview_enabled: bool,
) -> Vec<StartingPointGroup> {
    let profile_for = |slug: &str| profiles.iter().find(|profile| profile.fixture.slug == slug);

    let mut groups = Vec::new();
    if let Some(box_profile) = profile_for(BOX_PRIMITIVE_PROFILE_ID) {
        let mut derived_items = Vec::new();
        if let Some(lidded_box) = profile_for(LIDDED_BOX_PROFILE_ID) {
            derived_items.push(DerivedStartingPoint {
                display_name: "Lidded Box",
                profile: Some(lidded_box.clone()),
                derived_from_label: "Box Primitive",
                derivation_summary: "Derived from Box Primitive + Lid Seam.",
                status: StartingPointStatus::Active,
                preset: false,
            });
        }
        groups.push(StartingPointGroup {
            source_primitive_slug: BOX_PRIMITIVE_PROFILE_ID,
            display_name: "Box Primitive",
            description: profile_description(BOX_PRIMITIVE_PROFILE_ID),
            status: StartingPointStatus::Active,
            primitive_profile: Some(box_profile.clone()),
            derived_items,
        });
    }

    if let Some(flat_panel) = profile_for(FLAT_PANEL_PRIMITIVE_PROFILE_ID) {
        let mut derived_items = Vec::new();
        if let Some(hinged_panel) = profile_for(HINGED_PANEL_PROFILE_ID) {
            derived_items.push(DerivedStartingPoint {
                display_name: "Hinged Panel",
                profile: Some(hinged_panel.clone()),
                derived_from_label: "Flat Panel Primitive",
                derivation_summary: "Derived from Flat Panel Primitive + Hinge Edge.",
                status: StartingPointStatus::Active,
                preset: false,
            });
        }
        if let Some(panel_knob) = profile_for(PANEL_KNOB_PROFILE_ID) {
            derived_items.push(DerivedStartingPoint {
                display_name: "Panel with Knob",
                profile: Some(panel_knob.clone()),
                derived_from_label: "Flat Panel Primitive",
                derivation_summary: "Derived from Flat Panel Primitive + Sphere attachment.",
                status: StartingPointStatus::Active,
                preset: false,
            });
        }
        if developer_preview_enabled
            && let Some(handled_panel) = profile_for(HANDLED_PANEL_PROFILE_ID)
        {
            derived_items.push(DerivedStartingPoint {
                display_name: "Handled Panel",
                profile: Some(handled_panel.clone()),
                derived_from_label: "Flat Panel Primitive",
                derivation_summary: "Historical proof from Flat Panel + Hinge Edge + Handle.",
                status: StartingPointStatus::HistoricalProof,
                preset: false,
            });
        }
        groups.push(StartingPointGroup {
            source_primitive_slug: FLAT_PANEL_PRIMITIVE_PROFILE_ID,
            display_name: "Flat Panel Primitive",
            description: profile_description(FLAT_PANEL_PRIMITIVE_PROFILE_ID),
            status: StartingPointStatus::Active,
            primitive_profile: Some(flat_panel.clone()),
            derived_items,
        });
    }

    if let Some(sphere) = profile_for(SPHERE_PRIMITIVE_PROFILE_ID) {
        groups.push(StartingPointGroup {
            source_primitive_slug: SPHERE_PRIMITIVE_PROFILE_ID,
            display_name: "Sphere Primitive",
            description: profile_description(SPHERE_PRIMITIVE_PROFILE_ID),
            status: StartingPointStatus::Active,
            primitive_profile: Some(sphere.clone()),
            derived_items: vec![DerivedStartingPoint {
                display_name: "Knob-like Form",
                profile: None,
                derived_from_label: "Sphere Primitive",
                derivation_summary: "Preset from Sphere Primitive properties.",
                status: StartingPointStatus::Preview,
                preset: true,
            }],
        });
    }

    groups
}

pub(super) fn home_browser_width(available_width: f32) -> f32 {
    available_width.clamp(320.0, 420.0)
}

pub(super) fn show_home_browser_panel(
    ui: &mut egui::Ui,
    profiles: &[ProductHomeProfile],
    search_query: &mut String,
    filter: &mut HomeTemplateFilter,
    selected_slug: &mut Option<String>,
) {
    let colors = VisualFoundryTokens::dark().colors;
    let single_profile_mode = profiles.len() == 1 && HOME_TEMPLATE_FILTERS.is_empty();
    let starter_profile_mode = !profiles.is_empty()
        && HOME_TEMPLATE_FILTERS.is_empty()
        && profiles
            .iter()
            .all(|profile| is_visible_starter_profile(&profile.fixture.slug));
    product_card(ui, false, |ui| {
        ui.set_min_height(ui.available_height().max(420.0));
        if single_profile_mode {
            normalize_home_selection(profiles, "", HomeTemplateFilter::All, selected_slug);
            section_header(
                ui,
                SectionHeaderSpec {
                    eyebrow: "Choose",
                    title: "Start with Box Primitive.",
                    subtitle: Some(HOME_SUBTITLE),
                },
            );
            ui.add_space(8.0);
            ui.add(
                egui::Label::new(
                    RichText::new(HOME_CONTROL_COPY)
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
            if let Some(profile) = profiles.first() {
                ui.add_space(18.0);
                ui.label(
                    RichText::new(profile.label.as_str())
                        .color(colors.text)
                        .size(18.0)
                        .strong(),
                );
                ui.add(
                    egui::Label::new(
                        RichText::new(profile_description(&profile.fixture.slug))
                            .color(colors.text_muted),
                    )
                    .wrap(),
                );
            }
        } else {
            section_header(
                ui,
                SectionHeaderSpec {
                    eyebrow: "Choose",
                    title: if starter_profile_mode {
                        "Choose a starting point"
                    } else {
                        "Choose what to make"
                    },
                    subtitle: Some(if starter_profile_mode {
                        "Pick a primitive, or choose a derived starting point under its source."
                    } else {
                        HOME_SUBTITLE
                    }),
                },
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(if starter_profile_mode {
                    "Starting points are grouped by provenance so the source stays clear."
                } else {
                    "Choose the Box Primitive starting point below."
                })
                .color(colors.text_muted)
                .small(),
            );
            ui.add_space(12.0);
            if !starter_profile_mode {
                let response = ui.add_sized(
                    [ui.available_width(), 32.0],
                    egui::TextEdit::singleline(search_query)
                        .hint_text("Search starting point...")
                        .desired_width(f32::INFINITY),
                );
                if response.changed() {
                    *selected_slug =
                        default_filtered_home_profile_slug(profiles, search_query, *filter);
                }
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
                    for option in HOME_TEMPLATE_FILTERS {
                        if home_filter_button(ui, option, *filter).clicked() {
                            *filter = option;
                            *selected_slug =
                                default_filtered_home_profile_slug(profiles, search_query, *filter);
                        }
                    }
                });
                ui.add_space(12.0);
            }
            let selection_query = if starter_profile_mode {
                ""
            } else {
                search_query.as_str()
            };
            normalize_home_selection(profiles, selection_query, *filter, selected_slug);

            let filtered_indices =
                filtered_home_profile_indices(profiles, selection_query, *filter);
            if !starter_profile_mode {
                let count_label = home_profile_count_label(filtered_indices.len());
                ui.label(
                    RichText::new(count_label)
                        .color(colors.text_subtle)
                        .small()
                        .strong(),
                );
                ui.add_space(6.0);
            }
            egui::ScrollArea::vertical()
                .id_salt("foundry_home_template_list")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if starter_profile_mode {
                        let preview_mode = profiles
                            .iter()
                            .any(|profile| profile.fixture.slug == HANDLED_PANEL_PROFILE_ID);
                        let groups = product_home_starting_point_groups(profiles, preview_mode);
                        show_starting_point_provenance_library(ui, &groups, selected_slug);
                    } else {
                        if filtered_indices.is_empty() {
                            ui.label(
                                RichText::new("No matching starting point")
                                    .color(colors.text_muted)
                                    .small(),
                            );
                        }
                        for index in filtered_indices {
                            let profile = &profiles[index];
                            let selected =
                                selected_slug.as_deref() == Some(profile.fixture.slug.as_str());
                            if show_home_template_row(ui, profile, selected).clicked() {
                                *selected_slug = Some(profile.fixture.slug.clone());
                            }
                            ui.add_space(6.0);
                        }
                    }
                });
            if starter_profile_mode {
                *search_query = String::new();
                *filter = HomeTemplateFilter::All;
            }
        }
    });
}

pub(super) fn show_starting_point_provenance_library(
    ui: &mut egui::Ui,
    groups: &[StartingPointGroup],
    selected_slug: &mut Option<String>,
) {
    let colors = VisualFoundryTokens::dark().colors;
    ui.label(
        RichText::new("Primitives")
            .color(colors.accent_hover)
            .small()
            .strong(),
    );
    ui.add_space(8.0);
    for group in groups {
        let selected = selected_slug.as_deref() == Some(group.source_primitive_slug);
        if show_starting_point_group_row(ui, group, selected).clicked()
            && let Some(profile) = &group.primitive_profile
        {
            *selected_slug = Some(profile.fixture.slug.clone());
        }
        ui.add_space(5.0);
        for derived in &group.derived_items {
            let selected = derived.profile.as_ref().is_some_and(|profile| {
                selected_slug.as_deref() == Some(profile.fixture.slug.as_str())
            });
            if show_derived_starting_point_row(ui, derived, selected).clicked()
                && let Some(profile) = &derived.profile
            {
                *selected_slug = Some(profile.fixture.slug.clone());
            }
            ui.add_space(5.0);
        }
        ui.add_space(10.0);
    }
}

pub(super) fn show_starting_point_group_row(
    ui: &mut egui::Ui,
    group: &StartingPointGroup,
    selected: bool,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel_subtle
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(10, 9))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(group.display_name)
                        .color(colors.text)
                        .strong(),
                );
                ui.label(
                    RichText::new(group.status.label())
                        .color(colors.text_subtle)
                        .small(),
                );
            });
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(group.description)
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
        })
        .response
        .interact(egui::Sense::click())
}

pub(super) fn show_derived_starting_point_row(
    ui: &mut egui::Ui,
    derived: &DerivedStartingPoint,
    selected: bool,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let enabled = derived.profile.is_some();
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel_elevated
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    ui.horizontal(|ui| {
        ui.add_space(18.0);
        let response = egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width((ui.available_width() - 18.0).max(120.0));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(derived.display_name)
                            .color(if enabled {
                                colors.text
                            } else {
                                colors.text_subtle
                            })
                            .strong(),
                    );
                    if derived.preset {
                        ui.label(RichText::new("Preset").color(colors.accent_hover).small());
                    }
                    ui.label(
                        RichText::new(derived.status.label())
                            .color(colors.text_subtle)
                            .small(),
                    );
                });
                ui.add_space(4.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(format!("Derived from {}", derived.derived_from_label))
                            .color(colors.text_muted)
                            .small(),
                    )
                    .wrap(),
                );
            })
            .response;
        if enabled {
            response.interact(egui::Sense::click())
        } else {
            response
        }
    })
    .inner
}

pub(super) fn home_filter_button(
    ui: &mut egui::Ui,
    option: HomeTemplateFilter,
    selected: HomeTemplateFilter,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let is_selected = option == selected;
    let fill = if is_selected {
        colors.accent_soft
    } else {
        colors.panel_elevated
    };
    let stroke = if is_selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    let text = if is_selected {
        colors.text
    } else {
        colors.text_muted
    };
    ui.add(
        egui::Button::new(RichText::new(option.label()).color(text))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(6))
            .min_size(egui::vec2(58.0, 28.0)),
    )
}

pub(super) fn show_home_template_row(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    selected: bool,
) -> egui::Response {
    let colors = VisualFoundryTokens::dark().colors;
    let fill = if selected {
        colors.accent_soft
    } else {
        colors.panel_subtle
    };
    let stroke = if selected {
        egui::Stroke::new(1.0, colors.accent_hover)
    } else {
        egui::Stroke::new(1.0, colors.stroke)
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new(profile.label.as_str())
                    .color(colors.text)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.add(
                egui::Label::new(
                    RichText::new(profile_description(&profile.fixture.slug))
                        .color(colors.text_muted)
                        .small(),
                )
                .wrap(),
            );
        })
        .response
        .interact(egui::Sense::click())
}

pub(super) fn show_home_selected_template_stage(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    home_thumbnails: &mut HomeThumbnailCoordinator,
    texture_cache: &mut FoundryTextureCache,
) -> egui::Response {
    let mut action = None;
    product_stage(ui, |ui| {
        let colors = VisualFoundryTokens::dark().colors;
        ui.set_min_height(ui.available_height().max(480.0));
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(profile.label.as_str())
                    .color(colors.text)
                    .size(18.0)
                    .strong(),
            );
        });
        ui.add_space(8.0);
        ui.add(
            egui::Label::new(
                RichText::new(format!(
                    "{} {}",
                    profile_description(&profile.fixture.slug),
                    profile_control_copy(&profile.fixture.slug)
                ))
                .color(colors.text_muted),
            )
            .wrap(),
        );
        if let Some(provenance) = selected_profile_provenance_copy(&profile.fixture.slug) {
            ui.add_space(6.0);
            ui.label(RichText::new(provenance).color(colors.accent_hover).small());
        }
        ui.add_space(14.0);
        let preview_height = (ui.available_height() - 98.0).clamp(320.0, 620.0);
        show_home_selected_model_preview(
            ui,
            profile,
            home_thumbnails,
            texture_cache,
            preview_height,
        );
        ui.add_space(14.0);
        action = Some(start_template_button(ui));
    });
    action.expect("selected template stage always renders start action")
}

pub(super) fn show_home_selected_model_preview(
    ui: &mut egui::Ui,
    profile: &ProductHomeProfile,
    home_thumbnails: &mut HomeThumbnailCoordinator,
    texture_cache: &mut FoundryTextureCache,
    height: f32,
) {
    let colors = VisualFoundryTokens::dark().colors;
    let width = ui.available_width().max(260.0);
    home_thumbnails.ensure(profile);
    home_thumbnails.prewarm_turntable(&profile.fixture.slug);
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::click_and_drag());
    if response.dragged_by(egui::PointerButton::Secondary)
        && home_thumbnails.orbit_thumbnail(&profile.fixture.slug, response.drag_delta())
    {
        ui.ctx().request_repaint();
    }
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(6), colors.panel_subtle);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(6),
        egui::Stroke::new(1.0, colors.stroke_strong),
        egui::StrokeKind::Inside,
    );

    if let Some(frame) = home_thumbnails.thumbnail(&profile.fixture.slug)
        && let Some(size) =
            scaled_preview_size(frame.width, frame.height, (height - 24.0).min(width - 24.0))
    {
        let preview_id = format!(
            "home-template-{}-frame-{}",
            profile.fixture.slug, frame.frame_index
        );
        let texture = texture_cache.texture(
            ui.ctx(),
            &preview_id,
            None,
            &frame.rgba8,
            frame.width,
            frame.height,
        );
        let image_rect = egui::Rect::from_center_size(rect.center(), size);
        ui.painter().image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    } else {
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Preview building",
            egui::FontId::proportional(13.0),
            colors.text_muted,
        );
    }
}

pub(super) fn home_profile_count_label(count: usize) -> String {
    match count {
        1 => "1 starting point".to_owned(),
        count => format!("{count} starting points"),
    }
}

pub(super) fn selected_profile_provenance_copy(slug: &str) -> Option<&'static str> {
    match slug {
        BOX_PRIMITIVE_PROFILE_ID => Some("Primitive starting point."),
        LIDDED_BOX_PROFILE_ID => Some("Derived from Box Primitive + Lid Seam."),
        FLAT_PANEL_PRIMITIVE_PROFILE_ID => Some("Primitive starting point."),
        HINGED_PANEL_PROFILE_ID => Some("Derived from Flat Panel Primitive + Hinge Edge."),
        HANDLED_PANEL_PROFILE_ID => {
            Some("Historical proof from Flat Panel Primitive + Hinge Edge + Handle.")
        }
        PANEL_KNOB_PROFILE_ID => Some("Derived from Flat Panel Primitive + Sphere attachment."),
        SPHERE_PRIMITIVE_PROFILE_ID => Some("Primitive starting point with a Knob-like preset."),
        _ => None,
    }
}

pub(super) fn selected_home_profile<'a>(
    profiles: &'a [ProductHomeProfile],
    selected_slug: &Option<String>,
) -> Option<&'a ProductHomeProfile> {
    selected_slug
        .as_deref()
        .and_then(|slug| profiles.iter().find(|profile| profile.fixture.slug == slug))
}

pub(super) fn normalize_home_selection(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
    selected_slug: &mut Option<String>,
) {
    if selected_slug
        .as_deref()
        .is_some_and(|slug| home_profile_is_visible(profiles, slug, search_query, filter))
    {
        return;
    }
    *selected_slug = default_filtered_home_profile_slug(profiles, search_query, filter);
}

pub(super) fn default_home_profile_slug(profiles: &[ProductHomeProfile]) -> Option<String> {
    profiles.first().map(|profile| profile.fixture.slug.clone())
}

pub(super) fn default_filtered_home_profile_slug(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
) -> Option<String> {
    filtered_home_profile_indices(profiles, search_query, filter)
        .first()
        .map(|index| profiles[*index].fixture.slug.clone())
}

pub(super) fn home_profile_is_visible(
    profiles: &[ProductHomeProfile],
    slug: &str,
    search_query: &str,
    filter: HomeTemplateFilter,
) -> bool {
    filtered_home_profile_indices(profiles, search_query, filter)
        .into_iter()
        .any(|index| profiles[index].fixture.slug == slug)
}

pub(super) fn filtered_home_profile_indices(
    profiles: &[ProductHomeProfile],
    search_query: &str,
    filter: HomeTemplateFilter,
) -> Vec<usize> {
    profiles
        .iter()
        .enumerate()
        .filter_map(|(index, profile)| {
            (filter.matches(profile) && home_profile_matches_search(profile, search_query))
                .then_some(index)
        })
        .collect()
}

pub(super) fn home_profile_matches_search(
    profile: &ProductHomeProfile,
    search_query: &str,
) -> bool {
    let terms = search_query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>();
    if terms.is_empty() {
        return true;
    }
    let haystack = home_profile_search_haystack(profile);
    terms.iter().all(|term| haystack.contains(term))
}

pub(super) fn home_profile_search_haystack(profile: &ProductHomeProfile) -> String {
    format!(
        "{} {} {} {} {} {}",
        profile.label,
        profile.style_name,
        profile.family_id,
        profile.family_name,
        profile.category_chips.join(" "),
        profile_description(&profile.fixture.slug)
    )
    .to_ascii_lowercase()
}

impl HomeTemplateFilter {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::All => "All",
        }
    }

    pub(super) fn matches(self, _profile: &ProductHomeProfile) -> bool {
        match self {
            Self::All => true,
        }
    }
}

pub(super) fn start_template_button(ui: &mut egui::Ui) -> egui::Response {
    ui.horizontal(|ui| action_button(ui, &ActionSpec::enabled(ACTION_START, ButtonTone::Primary)))
        .inner
}

impl FoundryDesktopApp {
    pub(super) fn show_home(&mut self, ui: &mut egui::Ui) {
        let profiles = self.home_profiles.as_slice();
        if profiles.is_empty() {
            product_empty_state(
                ui,
                "Starting point is not available",
                "Open a saved project, or enable the clay starting points.",
            );
            return;
        }
        let mut selected_fixture = None;

        ui.horizontal_top(|ui| {
            let left_width = home_browser_width(ui.available_width());
            ui.allocate_ui_with_layout(
                egui::vec2(left_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    show_home_browser_panel(
                        ui,
                        profiles,
                        &mut self.home_search_query,
                        &mut self.home_filter,
                        &mut self.selected_home_profile_slug,
                    );
                },
            );
            ui.add_space(18.0);
            ui.allocate_ui_with_layout(
                egui::vec2(ui.available_width(), ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    let selected =
                        selected_home_profile(profiles, &self.selected_home_profile_slug);
                    if let Some(profile) = selected {
                        if show_home_selected_template_stage(
                            ui,
                            profile,
                            &mut self.home_thumbnails,
                            &mut self.texture_cache,
                        )
                        .clicked()
                        {
                            selected_fixture = Some(profile.fixture.clone());
                        }
                    } else {
                        product_empty_state(
                            ui,
                            "No matching starting point",
                            "Change the search to choose one of the clay starting points.",
                        );
                    }
                },
            );
        });

        if let Some(fixture) = selected_fixture {
            self.load_fixture(fixture, ui.ctx());
        }
    }
}
