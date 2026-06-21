//! Asset viewport overlays layered on the existing CPU preview widget.

#![allow(dead_code)]

use egui::{Align2, Color32, FontId, Pos2, Rect, Stroke, StrokeKind, TextureHandle, Ui, Vec2};
use shape_render::OrbitCamera;

use crate::asset::{AssetAppCommand, AssetValidationState};
use crate::viewport::{
    ViewportInteractionState, ViewportOverlayInfo, ViewportRenderSize, ViewportResponse,
    show_viewport,
};

/// Normalized screen-space rectangle for part bounds overlays.
#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) struct NormalizedRect {
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// Normalized screen-space point for socket markers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SocketMarker {
    pub name: String,
    pub position: [f32; 2],
}

/// Viewport-only asset overlay state.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetViewportOverlay {
    pub title: String,
    pub selected_part_name: Option<String>,
    pub selected_part_bounds: Option<NormalizedRect>,
    pub socket_markers: Vec<SocketMarker>,
    pub validation_marker: Option<AssetValidationState>,
    pub wireframe: bool,
    pub active_job_label: Option<String>,
    pub progress: Option<f32>,
}

impl Default for AssetViewportOverlay {
    fn default() -> Self {
        Self {
            title: "Untitled asset".to_owned(),
            selected_part_name: None,
            selected_part_bounds: None,
            socket_markers: Vec::new(),
            validation_marker: None,
            wireframe: false,
            active_job_label: None,
            progress: None,
        }
    }
}

/// Response from drawing the asset viewport.
pub(crate) struct AssetViewportResponse {
    pub viewport: ViewportResponse,
    pub commands: Vec<AssetAppCommand>,
}

/// Draw the current CPU preview and asset modeling overlays.
pub(crate) fn show_asset_viewport(
    ui: &mut Ui,
    state: &mut ViewportInteractionState,
    camera: &OrbitCamera,
    texture: Option<&TextureHandle>,
    overlay: &AssetViewportOverlay,
) -> AssetViewportResponse {
    let base_overlay = ViewportOverlayInfo {
        title: overlay.title.clone(),
        selected_node_name: overlay.selected_part_name.clone(),
        active_job_label: overlay.active_job_label.clone(),
        progress: overlay.progress,
        rendering: overlay.active_job_label.is_some(),
        ..ViewportOverlayInfo::default()
    };
    let viewport = show_viewport(ui, state, camera, texture, &base_overlay);
    paint_asset_overlays(ui, viewport.response.rect, overlay);

    let mut commands = Vec::new();
    ui.horizontal(|ui| {
        let mut wireframe = overlay.wireframe;
        if ui
            .checkbox(&mut wireframe, "Wireframe")
            .on_hover_text("Show polygon edges over the preview.")
            .changed()
        {
            commands.extend(wireframe_command(overlay.wireframe, wireframe));
        }
    });

    AssetViewportResponse { viewport, commands }
}

/// Build the wireframe toggle command only when changed.
#[must_use]
pub(crate) fn wireframe_command(current: bool, proposed: bool) -> Option<AssetAppCommand> {
    (current != proposed).then_some(AssetAppCommand::SetWireframe(proposed))
}

/// Stable overlay labels for tests and accessibility text.
#[must_use]
pub(crate) fn overlay_labels(overlay: &AssetViewportOverlay) -> Vec<String> {
    let mut labels = Vec::new();
    if let Some(part) = &overlay.selected_part_name {
        labels.push(format!("Selected part: {part}"));
    }
    if overlay.selected_part_bounds.is_some() {
        labels.push("Part bounds".to_owned());
    }
    if !overlay.socket_markers.is_empty() {
        labels.push(format!("{} socket marker(s)", overlay.socket_markers.len()));
    }
    if let Some(validation) = &overlay.validation_marker {
        labels.push(format!("Validation: {}", validation.label()));
    }
    if overlay.wireframe {
        labels.push("Wireframe on".to_owned());
    }
    labels
}

fn paint_asset_overlays(ui: &Ui, rect: Rect, overlay: &AssetViewportOverlay) {
    let painter = ui.painter_at(rect);
    if let Some(bounds) = overlay.selected_part_bounds {
        let bounds_rect = bounds.to_rect(rect);
        painter.rect_stroke(
            bounds_rect,
            0.0,
            Stroke::new(2.0, Color32::from_rgb(92, 190, 146)),
            StrokeKind::Inside,
        );
    }

    for marker in &overlay.socket_markers {
        let position = normalized_point(rect, marker.position);
        painter.circle_filled(position, 4.0, Color32::from_rgb(245, 190, 85));
        painter.text(
            position + Vec2::new(6.0, -6.0),
            Align2::LEFT_BOTTOM,
            &marker.name,
            FontId::proportional(11.0),
            Color32::from_rgb(250, 238, 210),
        );
    }

    if let Some(validation) = &overlay.validation_marker {
        let color = match validation {
            AssetValidationState::Valid => Color32::from_rgb(92, 190, 146),
            AssetValidationState::Warning(_) | AssetValidationState::Pending => {
                Color32::from_rgb(226, 164, 72)
            }
            AssetValidationState::Error(_) => Color32::from_rgb(219, 92, 82),
        };
        paint_badge(
            ui,
            rect.right_top() + Vec2::new(-12.0, 84.0),
            Align2::RIGHT_TOP,
            validation.label(),
            color,
        );
    }

    if overlay.wireframe {
        paint_wireframe_hint(ui, rect);
    }
}

impl NormalizedRect {
    fn to_rect(self, outer: Rect) -> Rect {
        let min = normalized_point(outer, self.min);
        let max = normalized_point(outer, self.max);
        Rect::from_min_max(min, max)
    }
}

fn normalized_point(rect: Rect, point: [f32; 2]) -> Pos2 {
    Pos2::new(
        rect.left() + rect.width() * point[0].clamp(0.0, 1.0),
        rect.top() + rect.height() * point[1].clamp(0.0, 1.0),
    )
}

fn paint_wireframe_hint(ui: &Ui, rect: Rect) {
    let painter = ui.painter_at(rect);
    let stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(230, 238, 242, 52));
    let size = ViewportRenderSize::from_points(rect.size(), ui.ctx().pixels_per_point());
    let columns = (size.width / 96).clamp(3, 12);
    let rows = (size.height / 96).clamp(3, 12);
    for column in 1..columns {
        let x = rect.left() + rect.width() * column as f32 / columns as f32;
        painter.line_segment(
            [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
            stroke,
        );
    }
    for row in 1..rows {
        let y = rect.top() + rect.height() * row as f32 / rows as f32;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            stroke,
        );
    }
}

fn paint_badge(ui: &Ui, anchor: Pos2, align: Align2, text: &str, fill: Color32) {
    let painter = ui.painter();
    let text_color = Color32::from_rgb(20, 24, 28);
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(12.0), text_color);
    let padding = Vec2::new(7.0, 4.0);
    let rect = align.anchor_rect(Rect::from_min_size(anchor, galley.size() + padding * 2.0));
    painter.rect_filled(rect, 4.0, fill);
    painter.galley(rect.min + padding, galley, text_color);
}
