use super::*;

pub(super) fn current_preview_pixels_for_context(ctx: &egui::Context) -> u32 {
    current_preview_pixels_for_scale(ctx.pixels_per_point())
}

pub(super) fn current_preview_pixels_for_scale(pixels_per_point: f32) -> u32 {
    let scale = pixels_per_point.max(1.0);
    ((DEFAULT_PREVIEW_PIXELS as f32 * scale).ceil() as u32)
        .clamp(DEFAULT_PREVIEW_PIXELS, MAX_CURRENT_PREVIEW_PIXELS)
}

pub(super) fn candidate_preview_texture_id(
    candidate: &crate::foundry::view_model::FoundryCandidateCard,
) -> String {
    candidate
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("candidate-{}", candidate.id.0))
}

pub(super) fn option_preview_texture_id(
    option: &crate::foundry::view_model::FoundryOptionCard,
) -> String {
    option
        .preview_id
        .clone()
        .unwrap_or_else(|| format!("option-{}-{}", option.control_id, option.label))
}

#[derive(Default)]
pub(super) struct FoundryTextureCache {
    textures: BTreeMap<String, CachedFoundryTexture>,
}

pub(super) struct CachedFoundryTexture {
    identity: FoundryTextureIdentity,
    texture: egui::TextureHandle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct FoundryTextureIdentity {
    preview_id: String,
    build_fingerprint: Option<String>,
    width: u32,
    height: u32,
}

impl FoundryTextureIdentity {
    pub(super) fn new(
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            preview_id: preview_id.to_owned(),
            build_fingerprint: build.map(|build| build.build_fingerprint.0.to_hex()),
            width,
            height,
        }
    }

    pub(super) fn texture_name(&self) -> String {
        format!(
            "foundry-preview-{}-{}x{}-{}",
            self.preview_id,
            self.width,
            self.height,
            self.build_fingerprint.as_deref().unwrap_or("no-build")
        )
    }
}

impl FoundryTextureCache {
    pub(super) fn clear(&mut self) {
        self.textures.clear();
    }

    pub(super) fn texture(
        &mut self,
        ctx: &egui::Context,
        preview_id: &str,
        build: Option<&FoundryBuildStamp>,
        rgba8: &[u8],
        width: u32,
        height: u32,
    ) -> egui::TextureHandle {
        let identity = FoundryTextureIdentity::new(preview_id, build, width, height);
        if let Some(cached) = self
            .textures
            .get(preview_id)
            .filter(|cached| cached.identity == identity)
        {
            return cached.texture.clone();
        }

        let preview_pixels = preview_pixels_with_transparent_matte(rgba8, width, height);
        let color_image =
            ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &preview_pixels);
        let texture =
            ctx.load_texture(identity.texture_name(), color_image, TextureOptions::LINEAR);
        self.textures.insert(
            preview_id.to_owned(),
            CachedFoundryTexture {
                identity,
                texture: texture.clone(),
            },
        );
        texture
    }
}

pub(super) fn preview_pixels_with_transparent_matte(
    rgba8: &[u8],
    width: u32,
    height: u32,
) -> Vec<u8> {
    let expected_len = (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4);
    if width == 0 || height == 0 || rgba8.len() != expected_len {
        return rgba8.to_vec();
    }
    let matte = [rgba8[0], rgba8[1], rgba8[2]];
    let is_dark_matte = matte.iter().all(|value| *value <= 48);
    if !is_dark_matte {
        return rgba8.to_vec();
    }
    let mut pixels = rgba8.to_vec();
    for chunk in pixels.chunks_exact_mut(4) {
        let dr = i32::from(chunk[0]) - i32::from(matte[0]);
        let dg = i32::from(chunk[1]) - i32::from(matte[1]);
        let db = i32::from(chunk[2]) - i32::from(matte[2]);
        let distance = dr * dr + dg * dg + db * db;
        if distance <= 20 * 20 {
            chunk[3] = 0;
        }
    }
    pixels
}

pub(super) struct FoundryPreviewDraw<'a> {
    pub(super) preview_id: &'a str,
    pub(super) build: Option<&'a FoundryBuildStamp>,
    pub(super) rgba8: &'a [u8],
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) max_edge: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CurrentPreviewStageStyle {
    Studio,
    CoordinateReference,
}

pub(super) fn current_preview_default_stage_style() -> CurrentPreviewStageStyle {
    current_preview_stage_style_for_axis_view(false)
}

pub(super) fn current_preview_stage_style_for_axis_view(
    axis_view_active: bool,
) -> CurrentPreviewStageStyle {
    if axis_view_active {
        CurrentPreviewStageStyle::CoordinateReference
    } else {
        CurrentPreviewStageStyle::Studio
    }
}

#[derive(Default)]
pub(super) struct CurrentPreviewOrbitState {
    drag_start_camera: Option<OrbitCamera>,
}

impl CurrentPreviewOrbitState {
    pub(super) fn camera_for_response(
        &mut self,
        preview: &FoundryPreviewImage,
        response: &egui::Response,
    ) -> Option<OrbitCamera> {
        self.camera_for_drag_delta(
            preview,
            response.dragged_by(current_preview_orbit_button()),
            response.drag_delta(),
        )
    }

    pub(super) fn camera_for_drag_delta(
        &mut self,
        preview: &FoundryPreviewImage,
        dragging_secondary: bool,
        drag_delta: egui::Vec2,
    ) -> Option<OrbitCamera> {
        if !dragging_secondary {
            self.drag_start_camera = None;
            return None;
        }

        let drag_start_camera = self
            .drag_start_camera
            .get_or_insert_with(|| preview.camera.clone());
        current_preview_orbit_camera_from_base(drag_start_camera, drag_delta)
    }
}

pub(super) fn draw_quick_template_preview(ui: &mut egui::Ui, max_edge: f32, _asset_name: &str) {
    let edge = max_edge.clamp(180.0, 360.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(edge, edge), egui::Sense::hover());
    let colors = VisualFoundryTokens::dark().colors;
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, colors.panel_elevated);
    painter.rect_stroke(
        rect,
        8.0,
        egui::Stroke::new(1.0, colors.stroke),
        egui::StrokeKind::Inside,
    );

    let center = rect.center();
    let body = egui::Rect::from_center_size(center, egui::vec2(edge * 0.58, edge * 0.44));
    painter.rect_filled(body, 6.0, colors.accent_soft);
    painter.rect_stroke(
        body,
        6.0,
        egui::Stroke::new(2.0, colors.accent_hover),
        egui::StrokeKind::Inside,
    );
    for inset in [0.08, 0.18] {
        let outline = body.shrink(edge * inset);
        painter.rect_stroke(
            outline,
            4.0,
            egui::Stroke::new(1.25, colors.stroke),
            egui::StrokeKind::Inside,
        );
    }
}

pub(super) fn draw_studio_stage_background(ui: &egui::Ui, rect: egui::Rect) {
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }

    let painter = ui.painter().with_clip_rect(rect);
    let back_wall = egui::Color32::from_rgb(46, 44, 40);
    let stage_floor = egui::Color32::from_rgb(57, 55, 49);
    let horizon = egui::Color32::from_rgba_unmultiplied(155, 143, 119, 56);
    let shadow = egui::Color32::from_rgba_unmultiplied(14, 15, 16, 92);

    painter.rect_filled(rect, 4.0, back_wall);

    let horizon_y = rect.bottom() - rect.height() * 0.28;
    let floor_rect =
        egui::Rect::from_min_max(egui::pos2(rect.left(), horizon_y), rect.right_bottom());
    painter.rect_filled(floor_rect, 0.0, stage_floor);
    painter.line_segment(
        [
            egui::pos2(rect.left(), horizon_y),
            egui::pos2(rect.right(), horizon_y),
        ],
        egui::Stroke::new(1.0, horizon),
    );

    let plate = egui::Rect::from_center_size(
        egui::pos2(rect.center().x, rect.bottom() - rect.height() * 0.14),
        egui::vec2(rect.width() * 0.34, rect.height() * 0.045),
    );
    painter.rect_filled(plate, egui::CornerRadius::same(12), shadow);
}

pub(super) fn draw_coordinate_reference_overlay(ui: &egui::Ui, rect: egui::Rect) {
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }

    let painter = ui.painter().with_clip_rect(rect);
    let viewport_bg = egui::Color32::from_rgb(45, 48, 50);
    let grid_minor = egui::Color32::from_rgba_unmultiplied(134, 140, 146, 52);
    let grid_major = egui::Color32::from_rgba_unmultiplied(154, 160, 166, 86);
    let x_axis = egui::Color32::from_rgba_unmultiplied(222, 75, 87, 178);
    let y_axis = egui::Color32::from_rgba_unmultiplied(116, 190, 72, 178);

    painter.rect_filled(rect, 4.0, viewport_bg);

    let origin = rect.center();
    let x_vector = egui::vec2(rect.width() * 0.50, rect.height() * 0.42);
    let y_vector = egui::vec2(rect.width() * 0.68, -rect.height() * 0.14);
    let grid_steps = COORDINATE_REFERENCE_GRID_STEPS;
    let grid_extent = COORDINATE_REFERENCE_GRID_EXTENT;

    for index in -grid_steps..=grid_steps {
        let offset = index as f32 / grid_steps as f32;
        let stroke = if index == 0 {
            egui::Stroke::new(1.0, grid_major)
        } else if index % 4 == 0 {
            egui::Stroke::new(0.85, grid_major)
        } else {
            egui::Stroke::new(0.65, grid_minor)
        };
        painter.line_segment(
            [
                origin + x_vector * offset - y_vector * grid_extent,
                origin + x_vector * offset + y_vector * grid_extent,
            ],
            stroke,
        );
        painter.line_segment(
            [
                origin + y_vector * offset - x_vector * grid_extent,
                origin + y_vector * offset + x_vector * grid_extent,
            ],
            stroke,
        );
    }

    painter.line_segment(
        [origin - x_vector * 1.18, origin + x_vector * 1.18],
        egui::Stroke::new(1.5, x_axis),
    );
    painter.line_segment(
        [origin - y_vector * 1.18, origin + y_vector * 1.18],
        egui::Stroke::new(1.5, y_axis),
    );
}

pub(super) fn draw_viewport_orientation_cue(ui: &egui::Ui, rect: egui::Rect) {
    if rect.width() <= 1.0 || rect.height() <= 1.0 {
        return;
    }

    let painter = ui.painter().with_clip_rect(rect);
    draw_viewport_axis_gizmo(&painter, rect);
}

pub(super) fn draw_viewport_axis_gizmo(painter: &egui::Painter, rect: egui::Rect) {
    let center = egui::pos2(rect.right() - 38.0, rect.top() + 38.0);
    let axis_specs = [
        (
            egui::vec2(22.0, 8.0),
            "X",
            egui::Color32::from_rgb(222, 75, 87),
        ),
        (
            egui::vec2(10.0, -22.0),
            "Y",
            egui::Color32::from_rgb(116, 190, 72),
        ),
        (
            egui::vec2(0.0, -30.0),
            "Z",
            egui::Color32::from_rgb(82, 144, 238),
        ),
    ];

    painter.circle_filled(center, 3.0, egui::Color32::from_rgb(188, 194, 200));
    for (vector, label, color) in axis_specs {
        let end = center + vector;
        painter.line_segment([center, end], egui::Stroke::new(1.6, color));
        painter.circle_filled(end, 5.0, color);
        painter.text(
            end,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(9.0),
            egui::Color32::WHITE,
        );
    }
}

pub(super) fn show_current_rgba_preview(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    preview: FoundryPreviewDraw<'_>,
) -> egui::Response {
    let width_usize = preview.width as usize;
    let height_usize = preview.height as usize;
    let expected_len = width_usize.saturating_mul(height_usize).saturating_mul(4);
    if preview.width == 0 || preview.height == 0 || preview.rgba8.len() != expected_len {
        return draw_preview_pending_placeholder(ui, preview.max_edge);
    }

    let texture = texture_cache.texture(
        ui.ctx(),
        preview.preview_id,
        preview.build,
        preview.rgba8,
        preview.width,
        preview.height,
    );
    if let Some(size) = scaled_preview_size(preview.width, preview.height, preview.max_edge) {
        let viewport_size = current_preview_viewport_size(ui.available_width(), preview.max_edge);
        let model_size = current_preview_model_image_size(size);
        let (rect, response) = ui.allocate_exact_size(viewport_size, egui::Sense::hover());
        let image_rect = egui::Rect::from_center_size(rect.center(), model_size);
        match current_preview_default_stage_style() {
            CurrentPreviewStageStyle::Studio => draw_studio_stage_background(ui, rect),
            CurrentPreviewStageStyle::CoordinateReference => {
                draw_coordinate_reference_overlay(ui, rect);
            }
        }
        ui.painter().image(
            texture.id(),
            image_rect,
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        draw_viewport_orientation_cue(ui, rect);
        return response;
    }
    let size = egui::vec2(preview.max_edge.max(1.0), preview.max_edge.max(1.0));
    ui.allocate_exact_size(size, egui::Sense::hover()).1
}

pub(super) fn current_preview_viewport_size(available_width: f32, max_edge: f32) -> egui::Vec2 {
    egui::vec2(available_width.max(max_edge).max(1.0), max_edge.max(1.0))
}

pub(super) fn current_preview_model_image_size(size: egui::Vec2) -> egui::Vec2 {
    size * CURRENT_PREVIEW_MODEL_IMAGE_SCALE
}

pub(super) fn draw_preview_pending_placeholder(ui: &mut egui::Ui, max_edge: f32) -> egui::Response {
    let size = egui::vec2(max_edge, max_edge);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let colors = VisualFoundryTokens::dark().colors;
    ui.painter().rect_filled(rect, 6.0, colors.panel_elevated);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, colors.stroke),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Preview pending",
        egui::FontId::proportional(12.0),
        colors.text_muted,
    );
    response
}

pub(super) fn show_rgba_preview(
    ui: &mut egui::Ui,
    texture_cache: &mut FoundryTextureCache,
    preview: FoundryPreviewDraw<'_>,
) -> egui::Response {
    let width_usize = preview.width as usize;
    let height_usize = preview.height as usize;
    let expected_len = width_usize.saturating_mul(height_usize).saturating_mul(4);
    if preview.width == 0 || preview.height == 0 || preview.rgba8.len() != expected_len {
        return draw_preview_pending_placeholder(ui, preview.max_edge);
    }

    let texture = texture_cache.texture(
        ui.ctx(),
        preview.preview_id,
        preview.build,
        preview.rgba8,
        preview.width,
        preview.height,
    );
    if let Some(size) = scaled_preview_size(preview.width, preview.height, preview.max_edge) {
        return ui.image((texture.id(), size));
    }
    let size = egui::vec2(preview.max_edge.max(1.0), preview.max_edge.max(1.0));
    ui.allocate_exact_size(size, egui::Sense::hover()).1
}

pub(super) fn scaled_preview_size(width: u32, height: u32, max_edge: f32) -> Option<egui::Vec2> {
    if width == 0 || height == 0 || max_edge <= 0.0 {
        return None;
    }
    let scale = (max_edge / width as f32).min(max_edge / height as f32);
    Some(egui::vec2(width as f32 * scale, height as f32 * scale))
}

impl FoundryDesktopApp {
    pub(super) fn show_current_preview_sized(
        &mut self,
        ui: &mut egui::Ui,
        max_edge: f32,
    ) -> Vec<FoundryAppCommand> {
        let mut commands = Vec::new();
        let asset_name = self.current_project_title();
        let preview = self.state.current_preview.clone();
        let has_output = self.state.current_output.is_some();
        let rendering_preview = self
            .state
            .active_jobs
            .values()
            .any(|request| request.slot() == crate::foundry::FoundryJobSlot::RenderPreview);
        let preview_is_stale = preview
            .as_ref()
            .is_some_and(|preview| preview.build != self.state.current_build);
        let draw_edge = max_edge.min(ui.available_width().max(1.0));
        ui.vertical_centered(|ui| {
            if let Some(preview) = &preview {
                let preview_id = format!("current-{}", preview.preview_id);
                let response = show_current_rgba_preview(
                    ui,
                    &mut self.texture_cache,
                    FoundryPreviewDraw {
                        preview_id: &preview_id,
                        build: preview.build.as_ref(),
                        rgba8: &preview.rgba8,
                        width: preview.width,
                        height: preview.height,
                        max_edge: draw_edge,
                    },
                );
                let drag_response = ui.interact(
                    response.rect,
                    ui.id().with("current_preview_orbit_drag"),
                    egui::Sense::click_and_drag(),
                );
                if let Some(camera) = self
                    .current_preview_orbit
                    .camera_for_response(preview, &drag_response)
                    && preview.camera != camera
                {
                    commands.push(FoundryAppCommand::RequestPreview {
                        width: preview.width,
                        height: preview.height,
                        camera: Some(camera),
                    });
                }
                if let Some(message) = current_preview_stage_status_message(
                    true,
                    has_output,
                    preview_is_stale,
                    rendering_preview,
                ) {
                    ui.label(
                        RichText::new(message)
                            .color(VisualFoundryTokens::dark().colors.warning)
                            .small(),
                    );
                }
            } else if has_output {
                if let Some(message) = current_preview_stage_status_message(
                    false,
                    has_output,
                    preview_is_stale,
                    rendering_preview,
                ) {
                    ui.weak(message);
                }
            } else {
                draw_quick_template_preview(ui, draw_edge, &asset_name);
                ui.add_space(8.0);
                ui.weak("Preparing your asset...");
            }
        });
        commands
    }
}
