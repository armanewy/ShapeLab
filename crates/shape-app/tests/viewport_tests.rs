#[path = "../src/viewport.rs"]
mod viewport;

use std::time::Duration;

use egui::{Rect, pos2, vec2};
use shape_render::OrbitCamera;
use viewport::{
    ViewportAction, ViewportRenderPolicy, ViewportRenderSize, camera_after_zoom,
    fit_rect_preserve_aspect, orbit_delta_from_pixels, pan_delta_from_pixels,
    zoom_factor_from_scroll,
};

fn camera() -> OrbitCamera {
    OrbitCamera {
        yaw_degrees: 35.0,
        pitch_degrees: 25.0,
        distance: 10.0,
        vertical_fov_degrees: 60.0,
        ..OrbitCamera::default()
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 1.0e-5,
        "expected {expected}, got {actual}"
    );
}

#[test]
fn camera_delta_conversion_maps_pixels_to_orbit_and_pan() {
    let (yaw, pitch) = orbit_delta_from_pixels(vec2(10.0, -4.0));
    assert_close(yaw, 3.5);
    assert_close(pitch, 1.4);

    let size = ViewportRenderSize::new(800, 400);
    let (right, up) = pan_delta_from_pixels(vec2(20.0, 10.0), &camera(), size);
    assert!(right < 0.0, "dragging right pans the view right");
    assert!(up > 0.0, "dragging down keeps the scene under the pointer");
    assert_close(right.abs() / up.abs(), 2.0);
}

#[test]
fn zoom_factor_and_camera_distance_are_clamped() {
    let zoom_in = zoom_factor_from_scroll(10_000.0);
    let zoom_out = zoom_factor_from_scroll(-10_000.0);
    assert_close(zoom_in, 0.2);
    assert_close(zoom_out, 5.0);
    assert_close(zoom_factor_from_scroll(f32::NAN), 1.0);

    let mut near_camera = camera();
    near_camera.distance = 0.1;
    let zoomed = camera_after_zoom(&near_camera, 0.000_001);
    assert!(zoomed.distance.is_finite());
    assert!(zoomed.distance > 0.0);

    let mut far_camera = camera();
    far_camera.distance = 1.0e20;
    let zoomed = camera_after_zoom(&far_camera, 1.0e20);
    assert!(zoomed.distance.is_finite());
}

#[test]
fn interactive_render_requests_are_throttled_until_drag_stops() {
    let mut policy =
        ViewportRenderPolicy::with_timing(Duration::from_millis(100), Duration::from_millis(150));
    let size = ViewportRenderSize::new(1200, 800);
    let camera = camera();

    let first = policy.interaction_actions(Duration::ZERO, true, true, false, size, &camera);
    assert_eq!(first.len(), 1);
    assert!(matches!(
        first[0],
        ViewportAction::RequestInteractiveRender(_)
    ));

    let throttled =
        policy.interaction_actions(Duration::from_millis(50), true, true, false, size, &camera);
    assert!(throttled.is_empty());

    let second =
        policy.interaction_actions(Duration::from_millis(125), true, true, false, size, &camera);
    assert!(matches!(
        second.as_slice(),
        [ViewportAction::RequestInteractiveRender(_)]
    ));

    let final_request = policy.interaction_actions(
        Duration::from_millis(130),
        false,
        false,
        true,
        size,
        &camera,
    );
    assert!(matches!(
        final_request.as_slice(),
        [ViewportAction::RequestFinalRender(_)]
    ));
}

#[test]
fn resize_requests_wait_for_debounce_and_reset_when_size_changes() {
    let mut policy =
        ViewportRenderPolicy::with_timing(Duration::from_millis(100), Duration::from_millis(150));
    let camera = camera();
    let first_size = ViewportRenderSize::new(640, 480);
    let second_size = ViewportRenderSize::new(800, 600);

    assert!(
        policy
            .resize_action(Duration::ZERO, first_size, &camera, false)
            .is_none()
    );
    assert!(
        policy
            .resize_action(Duration::from_millis(100), first_size, &camera, false)
            .is_none()
    );
    assert!(
        policy
            .resize_action(Duration::from_millis(120), second_size, &camera, false)
            .is_none()
    );

    let action = policy.resize_action(Duration::from_millis(271), second_size, &camera, false);
    assert!(matches!(
        action,
        Some(ViewportAction::RequestFinalRender(_))
    ));

    assert!(
        policy
            .resize_action(Duration::from_millis(500), second_size, &camera, false)
            .is_none()
    );
}

#[test]
fn image_rect_is_fitted_without_aspect_stretching() {
    let bounds = Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 200.0));
    let square = fit_rect_preserve_aspect(bounds, vec2(100.0, 100.0));
    assert_close(square.width(), 200.0);
    assert_close(square.height(), 200.0);
    assert_close(square.center().x, 200.0);
    assert_close(square.center().y, 100.0);

    let wide = fit_rect_preserve_aspect(bounds, vec2(1600.0, 800.0));
    assert_close(wide.width(), 400.0);
    assert_close(wide.height(), 200.0);
}
