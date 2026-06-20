//! Viewport widget and camera interaction helpers.

#![allow(dead_code)]

use std::time::Duration;

use egui::{
    Align2, Color32, CursorIcon, FontId, PointerButton, Pos2, Rect, Response, Sense, Stroke,
    StrokeKind, TextureHandle, Ui, Vec2,
};
use shape_render::OrbitCamera;

const ORBIT_DEGREES_PER_POINT: f32 = 0.35;
const SCROLL_ZOOM_POINTS_PER_OCTAVE: f32 = 240.0;
const MIN_ZOOM_FACTOR: f32 = 0.2;
const MAX_ZOOM_FACTOR: f32 = 5.0;
const MIN_RENDER_SIDE: u32 = 32;
const MAX_FULL_RENDER_SIDE: u32 = 2048;
const MAX_INTERACTIVE_RENDER_SIDE: u32 = 384;
const DEFAULT_INTERACTIVE_INTERVAL: Duration = Duration::from_millis(90);
const DEFAULT_RESIZE_DEBOUNCE: Duration = Duration::from_millis(180);

/// UI action emitted by the viewport widget.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ViewportAction {
    Orbit {
        delta_yaw: f32,
        delta_pitch: f32,
        camera: OrbitCamera,
    },
    Pan {
        delta_right: f32,
        delta_up: f32,
        camera: OrbitCamera,
    },
    Zoom {
        factor: f32,
        camera: OrbitCamera,
    },
    SetCamera(OrbitCamera),
    FitToObject,
    ResetCamera,
    RequestInteractiveRender(ViewportRenderRequest),
    RequestFinalRender(ViewportRenderRequest),
}

/// Pixel size and camera for a rerender request.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ViewportRenderRequest {
    pub size: ViewportRenderSize,
    pub camera: OrbitCamera,
}

/// Positive viewport render size in physical pixels.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) struct ViewportRenderSize {
    pub width: u32,
    pub height: u32,
}

impl ViewportRenderSize {
    #[must_use]
    pub(crate) fn new(width: u32, height: u32) -> Self {
        Self {
            width: width.clamp(MIN_RENDER_SIDE, MAX_FULL_RENDER_SIDE),
            height: height.clamp(MIN_RENDER_SIDE, MAX_FULL_RENDER_SIDE),
        }
    }

    #[must_use]
    pub(crate) fn from_points(size: Vec2, pixels_per_point: f32) -> Self {
        let scale = if pixels_per_point.is_finite() && pixels_per_point > 0.0 {
            pixels_per_point
        } else {
            1.0
        };
        let width = finite_ceil_u32(size.x * scale);
        let height = finite_ceil_u32(size.y * scale);
        Self::new(width, height)
    }

    #[must_use]
    pub(crate) fn interactive(self) -> Self {
        self.fit_longest_side(MAX_INTERACTIVE_RENDER_SIDE)
    }

    #[must_use]
    fn fit_longest_side(self, max_side: u32) -> Self {
        let longest = self.width.max(self.height);
        if longest <= max_side {
            return self;
        }

        let scale = max_side as f32 / longest as f32;
        Self::new(
            (self.width as f32 * scale)
                .round()
                .max(MIN_RENDER_SIDE as f32) as u32,
            (self.height as f32 * scale)
                .round()
                .max(MIN_RENDER_SIDE as f32) as u32,
        )
    }
}

/// Metadata painted over the viewport image.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ViewportOverlayInfo {
    pub title: String,
    pub revision_id: Option<String>,
    pub selected_node_name: Option<String>,
    pub active_job_label: Option<String>,
    pub progress: Option<f32>,
    pub recoverable_error: Option<String>,
    pub rendering: bool,
}

impl Default for ViewportOverlayInfo {
    fn default() -> Self {
        Self {
            title: "Untitled shape".to_owned(),
            revision_id: None,
            selected_node_name: None,
            active_job_label: None,
            progress: None,
            recoverable_error: None,
            rendering: false,
        }
    }
}

/// Persistent viewport interaction state owned by the app coordinator.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ViewportInteractionState {
    pub render_policy: ViewportRenderPolicy,
}

/// Non-window render request scheduler for camera interaction and resize.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ViewportRenderPolicy {
    interactive_interval: Duration,
    resize_debounce: Duration,
    last_interactive_request_at: Option<Duration>,
    last_requested_size: Option<ViewportRenderSize>,
    pending_resize: Option<PendingResize>,
    drag_was_active: bool,
}

impl Default for ViewportRenderPolicy {
    fn default() -> Self {
        Self {
            interactive_interval: DEFAULT_INTERACTIVE_INTERVAL,
            resize_debounce: DEFAULT_RESIZE_DEBOUNCE,
            last_interactive_request_at: None,
            last_requested_size: None,
            pending_resize: None,
            drag_was_active: false,
        }
    }
}

impl ViewportRenderPolicy {
    #[must_use]
    pub(crate) fn with_timing(interactive_interval: Duration, resize_debounce: Duration) -> Self {
        Self {
            interactive_interval,
            resize_debounce,
            ..Self::default()
        }
    }

    /// Return render requests caused by camera interaction.
    pub(crate) fn interaction_actions(
        &mut self,
        now: Duration,
        camera_changed: bool,
        dragging: bool,
        drag_stopped: bool,
        full_size: ViewportRenderSize,
        camera: &OrbitCamera,
    ) -> Vec<ViewportAction> {
        let mut actions = Vec::new();
        if camera_changed && dragging {
            if self.should_request_interactive(now) {
                let size = full_size.interactive();
                self.last_interactive_request_at = Some(now);
                self.last_requested_size = Some(size);
                actions.push(ViewportAction::RequestInteractiveRender(
                    ViewportRenderRequest {
                        size,
                        camera: camera.clamped(),
                    },
                ));
            }
        } else if camera_changed || drag_stopped || (self.drag_was_active && !dragging) {
            self.last_requested_size = Some(full_size);
            actions.push(ViewportAction::RequestFinalRender(ViewportRenderRequest {
                size: full_size,
                camera: camera.clamped(),
            }));
        }

        self.drag_was_active = dragging;
        actions
    }

    /// Return a full-resolution render request after viewport resize debounce.
    pub(crate) fn resize_action(
        &mut self,
        now: Duration,
        size: ViewportRenderSize,
        camera: &OrbitCamera,
        dragging: bool,
    ) -> Option<ViewportAction> {
        if dragging || self.last_requested_size == Some(size) {
            return None;
        }

        match self.pending_resize {
            Some(pending) if pending.size == size => {
                if now.saturating_sub(pending.started_at) < self.resize_debounce {
                    return None;
                }
                self.pending_resize = None;
                self.last_requested_size = Some(size);
                Some(ViewportAction::RequestFinalRender(ViewportRenderRequest {
                    size,
                    camera: camera.clamped(),
                }))
            }
            _ => {
                self.pending_resize = Some(PendingResize {
                    size,
                    started_at: now,
                });
                None
            }
        }
    }

    fn should_request_interactive(&self, now: Duration) -> bool {
        self.last_interactive_request_at
            .is_none_or(|last| now.saturating_sub(last) >= self.interactive_interval)
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct PendingResize {
    size: ViewportRenderSize,
    started_at: Duration,
}

/// Result from drawing the viewport widget.
pub(crate) struct ViewportResponse {
    pub response: Response,
    pub actions: Vec<ViewportAction>,
    pub render_size: ViewportRenderSize,
}

/// Draw the viewport image, handle camera input, and return app-level actions.
pub(crate) fn show_viewport(
    ui: &mut Ui,
    state: &mut ViewportInteractionState,
    camera: &OrbitCamera,
    texture: Option<&TextureHandle>,
    overlay: &ViewportOverlayInfo,
) -> ViewportResponse {
    let desired_size = desired_viewport_size(ui.available_size_before_wrap());
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());
    let pixels_per_point = ui.ctx().pixels_per_point();
    let render_size = ViewportRenderSize::from_points(rect.size(), pixels_per_point);
    let now = Duration::from_secs_f64(ui.input(|input| input.time).max(0.0));
    let mut actions = Vec::new();

    if ui.is_rect_visible(rect) {
        paint_viewport(ui, rect, texture, overlay, response.hovered());
    }

    let mut working_camera = camera.clamped();
    let mut camera_changed = false;
    let primary_drag = response.dragged_by(PointerButton::Primary);
    let secondary_drag = response.dragged_by(PointerButton::Secondary);
    let dragging = primary_drag || secondary_drag;
    let pointer_active = response.hovered() || dragging || state.render_policy.drag_was_active;

    if primary_drag {
        let delta = response.drag_motion();
        let (delta_yaw, delta_pitch) = orbit_delta_from_pixels(delta);
        if is_meaningful_delta(delta_yaw) || is_meaningful_delta(delta_pitch) {
            working_camera.orbit(delta_yaw, delta_pitch);
            actions.push(ViewportAction::Orbit {
                delta_yaw,
                delta_pitch,
                camera: working_camera.clone(),
            });
            camera_changed = true;
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }
    } else if secondary_drag {
        let delta = response.drag_delta();
        let (delta_right, delta_up) = pan_delta_from_pixels(delta, &working_camera, render_size);
        if is_meaningful_delta(delta_right) || is_meaningful_delta(delta_up) {
            working_camera.pan(delta_right, delta_up);
            actions.push(ViewportAction::Pan {
                delta_right,
                delta_up,
                camera: working_camera.clone(),
            });
            camera_changed = true;
            ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
        }
    } else if response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::Grab);
    }

    if pointer_active {
        let scroll_y = ui.input(|input| input.smooth_scroll_delta().y);
        if scroll_y.abs() > f32::EPSILON {
            let factor = zoom_factor_from_scroll(scroll_y);
            working_camera.zoom(factor);
            actions.push(ViewportAction::Zoom {
                factor,
                camera: working_camera.clone(),
            });
            camera_changed = true;
        }
    }

    if response.double_clicked() {
        actions.push(ViewportAction::FitToObject);
    }
    if response.hovered() && ui.input(|input| input.key_pressed(egui::Key::F)) {
        actions.push(ViewportAction::FitToObject);
    }
    if response.hovered() && ui.input(|input| input.key_pressed(egui::Key::Home)) {
        working_camera = OrbitCamera::default();
        actions.push(ViewportAction::ResetCamera);
        actions.push(ViewportAction::SetCamera(working_camera.clone()));
        camera_changed = true;
    }

    actions.extend(state.render_policy.interaction_actions(
        now,
        camera_changed,
        dragging,
        response.drag_stopped(),
        render_size,
        &working_camera,
    ));

    if let Some(action) =
        state
            .render_policy
            .resize_action(now, render_size, &working_camera, dragging)
    {
        actions.push(action);
    }

    ViewportResponse {
        response,
        actions,
        render_size,
    }
}

/// Convert primary-drag pixel movement into orbit deltas in degrees.
#[must_use]
pub(crate) fn orbit_delta_from_pixels(delta: Vec2) -> (f32, f32) {
    if !delta.x.is_finite() || !delta.y.is_finite() {
        return (0.0, 0.0);
    }
    (
        delta.x * ORBIT_DEGREES_PER_POINT,
        -delta.y * ORBIT_DEGREES_PER_POINT,
    )
}

/// Convert secondary-drag pixel movement into world-space camera target deltas.
#[must_use]
pub(crate) fn pan_delta_from_pixels(
    delta: Vec2,
    camera: &OrbitCamera,
    viewport_size: ViewportRenderSize,
) -> (f32, f32) {
    if !delta.x.is_finite() || !delta.y.is_finite() {
        return (0.0, 0.0);
    }
    let camera = camera.clamped();
    let viewport_height = viewport_size.height.max(1) as f32;
    let fov_radians = camera.vertical_fov_degrees.to_radians();
    let visible_height = 2.0 * camera.distance * (fov_radians * 0.5).tan();
    let units_per_point = visible_height / viewport_height;
    (-delta.x * units_per_point, delta.y * units_per_point)
}

/// Convert mouse-wheel motion into a positive distance scale factor.
#[must_use]
pub(crate) fn zoom_factor_from_scroll(scroll_y: f32) -> f32 {
    if !scroll_y.is_finite() {
        return 1.0;
    }
    (-scroll_y / SCROLL_ZOOM_POINTS_PER_OCTAVE)
        .exp2()
        .clamp(MIN_ZOOM_FACTOR, MAX_ZOOM_FACTOR)
}

#[must_use]
pub(crate) fn camera_after_zoom(camera: &OrbitCamera, factor: f32) -> OrbitCamera {
    let mut camera = camera.clamped();
    camera.zoom(factor);
    camera
}

/// Fit an image inside a rectangle without changing its aspect ratio.
#[must_use]
pub(crate) fn fit_rect_preserve_aspect(bounds: Rect, image_size: Vec2) -> Rect {
    if bounds.width() <= 0.0
        || bounds.height() <= 0.0
        || image_size.x <= 0.0
        || image_size.y <= 0.0
        || !bounds.width().is_finite()
        || !bounds.height().is_finite()
        || !image_size.x.is_finite()
        || !image_size.y.is_finite()
    {
        return bounds;
    }

    let bounds_aspect = bounds.width() / bounds.height();
    let image_aspect = image_size.x / image_size.y;
    let size = if bounds_aspect > image_aspect {
        Vec2::new(bounds.height() * image_aspect, bounds.height())
    } else {
        Vec2::new(bounds.width(), bounds.width() / image_aspect)
    };
    Rect::from_center_size(bounds.center(), size)
}

fn desired_viewport_size(available: Vec2) -> Vec2 {
    let width = if available.x.is_finite() {
        available.x.max(320.0)
    } else {
        640.0
    };
    let height = if available.y.is_finite() {
        available.y.max(240.0)
    } else {
        480.0
    };
    Vec2::new(width, height)
}

fn paint_viewport(
    ui: &Ui,
    rect: Rect,
    texture: Option<&TextureHandle>,
    overlay: &ViewportOverlayInfo,
    hovered: bool,
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(26, 28, 30));

    if let Some(texture) = texture {
        let image_rect = fit_rect_preserve_aspect(rect, texture.size_vec2());
        painter.image(
            texture.id(),
            image_rect,
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            Color32::WHITE,
        );
    } else {
        paint_checker_placeholder(&painter, rect);
        paint_center_label(&painter, rect, "No preview yet");
    }
    paint_reference_grid(&painter, rect);

    if overlay.rendering {
        paint_badge(
            &painter,
            rect.right_top() + Vec2::new(-12.0, 12.0),
            Align2::RIGHT_TOP,
            "Rendering...",
            Color32::from_rgba_unmultiplied(20, 24, 28, 210),
            Color32::from_rgb(235, 238, 241),
        );
    }

    paint_metadata(&painter, rect, overlay);
    paint_help(&painter, rect);

    let stroke_color = if hovered {
        Color32::from_rgb(120, 170, 210)
    } else {
        Color32::from_rgb(64, 68, 72)
    };
    painter.rect_stroke(
        rect,
        0.0,
        Stroke::new(1.0, stroke_color),
        StrokeKind::Inside,
    );
}

fn paint_reference_grid(painter: &egui::Painter, rect: Rect) {
    let spacing = 48.0;
    let grid = Stroke::new(1.0, Color32::from_rgba_unmultiplied(190, 204, 210, 24));
    let axis = Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 224, 232, 42));
    let center = rect.center();

    let mut x = center.x - ((center.x - rect.left()) / spacing).floor() * spacing;
    while x <= rect.right() {
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            grid,
        );
        x += spacing;
    }

    let mut y = center.y - ((center.y - rect.top()) / spacing).floor() * spacing;
    while y <= rect.bottom() {
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            grid,
        );
        y += spacing;
    }

    painter.line_segment(
        [
            Pos2::new(center.x, rect.top()),
            Pos2::new(center.x, rect.bottom()),
        ],
        axis,
    );
    painter.line_segment(
        [
            Pos2::new(rect.left(), center.y),
            Pos2::new(rect.right(), center.y),
        ],
        axis,
    );
}

fn paint_checker_placeholder(painter: &egui::Painter, rect: Rect) {
    let cell = 32.0;
    let dark = Color32::from_rgb(31, 34, 37);
    let light = Color32::from_rgb(38, 41, 45);
    let mut row = 0;
    let mut y = rect.top();
    while y < rect.bottom() {
        let mut column = 0;
        let mut x = rect.left();
        while x < rect.right() {
            let color = if (row + column) % 2 == 0 { dark } else { light };
            let cell_rect = Rect::from_min_max(
                Pos2::new(x, y),
                Pos2::new((x + cell).min(rect.right()), (y + cell).min(rect.bottom())),
            );
            painter.rect_filled(cell_rect, 0.0, color);
            column += 1;
            x += cell;
        }
        row += 1;
        y += cell;
    }
}

fn paint_center_label(painter: &egui::Painter, rect: Rect, label: &str) {
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(15.0),
        Color32::from_rgb(190, 195, 198),
    );
}

fn paint_metadata(painter: &egui::Painter, rect: Rect, overlay: &ViewportOverlayInfo) {
    let mut lines = vec![overlay.title.clone()];
    if let Some(revision_id) = &overlay.revision_id {
        lines.push(format!("Revision {revision_id}"));
    }
    if let Some(selected) = &overlay.selected_node_name {
        lines.push(format!("Selected: {selected}"));
    }
    paint_badge(
        painter,
        rect.left_top() + Vec2::new(12.0, 12.0),
        Align2::LEFT_TOP,
        &lines.join("\n"),
        Color32::from_rgba_unmultiplied(16, 18, 20, 205),
        Color32::from_rgb(235, 238, 241),
    );

    if let Some(progress_text) = progress_text(overlay) {
        paint_badge(
            painter,
            rect.right_bottom() + Vec2::new(-12.0, -12.0),
            Align2::RIGHT_BOTTOM,
            &progress_text,
            Color32::from_rgba_unmultiplied(16, 18, 20, 205),
            Color32::from_rgb(230, 235, 238),
        );
    }

    if let Some(error) = &overlay.recoverable_error {
        paint_badge(
            painter,
            rect.right_top() + Vec2::new(-12.0, 48.0),
            Align2::RIGHT_TOP,
            error,
            Color32::from_rgba_unmultiplied(90, 34, 28, 220),
            Color32::from_rgb(255, 226, 216),
        );
    }
}

fn paint_help(painter: &egui::Painter, rect: Rect) {
    paint_badge(
        painter,
        rect.left_bottom() + Vec2::new(12.0, -12.0),
        Align2::LEFT_BOTTOM,
        "Drag: orbit   Right drag: pan   Wheel: zoom   F: fit",
        Color32::from_rgba_unmultiplied(16, 18, 20, 180),
        Color32::from_rgb(218, 222, 225),
    );
}

fn paint_badge(
    painter: &egui::Painter,
    anchor: Pos2,
    align: Align2,
    text: &str,
    fill: Color32,
    text_color: Color32,
) {
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(13.0), text_color);
    let padding = Vec2::new(8.0, 6.0);
    let rect = align.anchor_rect(Rect::from_min_size(anchor, galley.size() + padding * 2.0));
    painter.rect_filled(rect, 4.0, fill);
    painter.galley(rect.min + padding, galley, text_color);
}

fn progress_text(overlay: &ViewportOverlayInfo) -> Option<String> {
    let job = overlay.active_job_label.as_deref()?;
    let progress = overlay
        .progress
        .filter(|value| value.is_finite())
        .map(|value| format!(" {:.0}%", value.clamp(0.0, 1.0) * 100.0))
        .unwrap_or_default();
    Some(format!("{job}{progress}"))
}

fn finite_ceil_u32(value: f32) -> u32 {
    if value.is_finite() && value > 0.0 {
        value.ceil() as u32
    } else {
        MIN_RENDER_SIDE
    }
}

fn is_meaningful_delta(value: f32) -> bool {
    value.is_finite() && value.abs() > f32::EPSILON
}
