use super::*;

#[derive(Default)]
pub(super) struct HomeThumbnailCoordinator {
    tx: Option<Sender<HomeThumbnailEvent>>,
    rx: Option<Receiver<HomeThumbnailEvent>>,
    active: BTreeSet<String>,
    failed: BTreeSet<String>,
    pending_frames: BTreeMap<String, i32>,
    thumbnails: BTreeMap<String, HomeTemplateThumbnail>,
}

#[derive(Debug)]
pub(super) struct HomeThumbnailEvent {
    slug: String,
    pub(super) frame_index: i32,
    result: Result<HomeThumbnailJobOutput, String>,
}

#[derive(Debug)]
pub(super) struct HomeThumbnailJobOutput {
    mesh: Option<Arc<TriangleMesh>>,
    base_camera: OrbitCamera,
    frame: HomeThumbnailFrame,
}

#[derive(Debug, Clone)]
pub(super) struct HomeTemplateThumbnail {
    mesh: Arc<TriangleMesh>,
    base_camera: OrbitCamera,
    selected_yaw_degrees: f32,
    selected_frame_index: i32,
    prewarm_cursor: usize,
    frames: BTreeMap<i32, HomeThumbnailFrame>,
}

#[derive(Debug, Clone)]
pub(super) struct HomeThumbnailFrame {
    pub(super) frame_index: i32,
    pub(super) rgba8: Vec<u8>,
    pub(super) width: u32,
    pub(super) height: u32,
}

pub(super) const HOME_THUMBNAIL_PIXELS: u32 = 512;
pub(super) const HOME_TURNTABLE_FRAME_COUNT: i32 = 24;
pub(super) const MAX_HOME_THUMBNAIL_JOBS: usize = 2;
pub(super) const HOME_THUMBNAIL_YAW_DEGREES_PER_POINT: f32 = 0.45;

impl HomeThumbnailCoordinator {
    pub(super) fn ensure(&mut self, profile: &ProductHomeProfile) {
        let slug = profile.fixture.slug.clone();
        if self.thumbnails.contains_key(&slug)
            || self.active.contains(&slug)
            || self.failed.contains(&slug)
            || self.active.len() >= MAX_HOME_THUMBNAIL_JOBS
        {
            return;
        }

        self.active.insert(slug.clone());
        let fixture = profile.fixture.clone();
        let tx = self.tx().clone();
        thread::spawn(move || {
            let resolver = BuiltInFoundryCatalogResolver::default();
            let result = compile_foundry_document(&fixture.document, &resolver)
                .map_err(|error| format!("{error:?}"))
                .and_then(|output| {
                    render_home_thumbnail_from_output(&output, 0)
                        .ok_or_else(|| "Could not render template thumbnail.".to_owned())
                });
            let _ = tx.send(HomeThumbnailEvent {
                slug,
                frame_index: 0,
                result,
            });
        });
    }

    pub(super) fn orbit_thumbnail(&mut self, slug: &str, delta: egui::Vec2) -> bool {
        let Some(thumbnail) = self.thumbnails.get_mut(slug) else {
            return false;
        };
        if delta == egui::Vec2::ZERO {
            return false;
        }

        let frame_index = {
            thumbnail.selected_yaw_degrees = home_turntable_yaw(
                thumbnail.selected_yaw_degrees + delta.x * HOME_THUMBNAIL_YAW_DEGREES_PER_POINT,
            );
            let frame_index = home_turntable_frame_index(thumbnail.selected_yaw_degrees);
            thumbnail.selected_frame_index = frame_index;
            frame_index
        };
        self.ensure_frame(slug, frame_index);
        true
    }

    pub(super) fn poll(&mut self) -> bool {
        let mut changed = false;
        let rx = self.rx().clone();
        loop {
            match rx.try_recv() {
                Ok(event) => {
                    self.active.remove(&event.slug);
                    match event.result {
                        Ok(output) => {
                            self.failed.remove(&event.slug);
                            self.store_frame(event.slug.clone(), output);
                            changed = true;
                        }
                        Err(_) => {
                            self.failed.insert(event.slug.clone());
                            changed = true;
                        }
                    }
                    if let Some(frame_index) = self.pending_frames.remove(&event.slug) {
                        self.spawn_frame_render(&event.slug, frame_index);
                    }
                    self.spawn_next_pending_frame();
                }
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(crossbeam_channel::TryRecvError::Disconnected) => {
                    self.active.clear();
                    break;
                }
            }
        }
        changed
    }

    pub(super) fn thumbnail(&self, slug: &str) -> Option<&HomeThumbnailFrame> {
        let thumbnail = self.thumbnails.get(slug)?;
        thumbnail
            .frames
            .get(&thumbnail.selected_frame_index)
            .or_else(|| nearest_home_thumbnail_frame(thumbnail))
    }

    pub(super) fn prewarm_turntable(&mut self, slug: &str) {
        if self.active.len() >= MAX_HOME_THUMBNAIL_JOBS {
            return;
        }
        let Some(frame_index) = ({
            let Some(thumbnail) = self.thumbnails.get_mut(slug) else {
                return;
            };
            let frame_order = home_turntable_prewarm_order(thumbnail.selected_frame_index);
            let mut frame_index = None;
            for offset in 0..frame_order.len() {
                let cursor = (thumbnail.prewarm_cursor + offset) % frame_order.len();
                let candidate = frame_order[cursor];
                if !thumbnail.frames.contains_key(&candidate) {
                    thumbnail.prewarm_cursor = (cursor + 1) % frame_order.len();
                    frame_index = Some(candidate);
                    break;
                }
            }
            frame_index
        }) else {
            return;
        };
        self.spawn_frame_render(slug, frame_index);
    }

    pub(super) fn spawn_next_pending_frame(&mut self) {
        while self.active.len() < MAX_HOME_THUMBNAIL_JOBS {
            let next = self.pending_frames.iter().find_map(|(slug, frame_index)| {
                let thumbnail = self.thumbnails.get(slug)?;
                (!self.active.contains(slug) && !thumbnail.frames.contains_key(frame_index))
                    .then(|| (slug.clone(), *frame_index))
            });
            let Some((slug, frame_index)) = next else {
                self.pending_frames.retain(|slug, frame_index| {
                    self.thumbnails
                        .get(slug)
                        .is_some_and(|thumbnail| !thumbnail.frames.contains_key(frame_index))
                });
                return;
            };
            self.pending_frames.remove(&slug);
            self.spawn_frame_render(&slug, frame_index);
        }
    }

    pub(super) fn has_active_jobs(&self) -> bool {
        !self.active.is_empty()
    }

    pub(super) fn ensure_frame(&mut self, slug: &str, frame_index: i32) {
        let Some(thumbnail) = self.thumbnails.get(slug) else {
            return;
        };
        if thumbnail.frames.contains_key(&frame_index) {
            return;
        }
        if self.active.contains(slug) {
            self.pending_frames.insert(slug.to_owned(), frame_index);
            return;
        }
        self.spawn_frame_render(slug, frame_index);
    }

    pub(super) fn spawn_frame_render(&mut self, slug: &str, frame_index: i32) {
        if self.active.len() >= MAX_HOME_THUMBNAIL_JOBS {
            self.pending_frames.insert(slug.to_owned(), frame_index);
            return;
        }
        let Some(thumbnail) = self.thumbnails.get(slug) else {
            return;
        };
        self.active.insert(slug.to_owned());
        let mesh = Arc::clone(&thumbnail.mesh);
        let camera = home_turntable_camera(&thumbnail.base_camera, frame_index);
        let base_camera = thumbnail.base_camera.clone();
        let tx = self.tx().clone();
        let slug = slug.to_owned();
        thread::spawn(move || {
            let result = render_home_thumbnail(mesh, base_camera, camera, frame_index, None)
                .ok_or_else(|| "Could not render template thumbnail.".to_owned());
            let _ = tx.send(HomeThumbnailEvent {
                slug,
                frame_index,
                result,
            });
        });
    }

    pub(super) fn store_frame(&mut self, slug: String, output: HomeThumbnailJobOutput) {
        if let Some(thumbnail) = self.thumbnails.get_mut(&slug) {
            thumbnail
                .frames
                .insert(output.frame.frame_index, output.frame);
            return;
        }
        let Some(mesh) = output.mesh else {
            return;
        };
        let mut frames = BTreeMap::new();
        frames.insert(output.frame.frame_index, output.frame);
        self.thumbnails.insert(
            slug,
            HomeTemplateThumbnail {
                mesh,
                base_camera: output.base_camera,
                selected_yaw_degrees: 0.0,
                selected_frame_index: 0,
                prewarm_cursor: 0,
                frames,
            },
        );
    }

    pub(super) fn reset(&mut self) {
        let (tx, rx) = unbounded();
        self.tx = Some(tx);
        self.rx = Some(rx);
        self.active.clear();
        self.failed.clear();
        self.pending_frames.clear();
        self.thumbnails.clear();
    }

    pub(super) fn tx(&mut self) -> &Sender<HomeThumbnailEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.tx.as_ref().expect("home thumbnail tx initialized")
    }

    pub(super) fn rx(&mut self) -> &Receiver<HomeThumbnailEvent> {
        if self.tx.is_none() || self.rx.is_none() {
            self.reset();
        }
        self.rx.as_ref().expect("home thumbnail rx initialized")
    }
}

pub(super) fn render_home_thumbnail_from_output(
    output: &FoundryCompilationOutput,
    frame_index: i32,
) -> Option<HomeThumbnailJobOutput> {
    let mesh = &output.artifact.combined_preview.mesh;
    let mesh = Arc::new(TriangleMesh {
        positions: mesh.positions.clone(),
        normals: mesh.normals.clone(),
        indices: mesh.indices.clone(),
        bounds: Aabb {
            min: mesh.bounds.min.into(),
            max: mesh.bounds.max.into(),
        },
    });
    let base_camera = fit_camera_to_bounds(mesh.bounds);
    let camera = home_turntable_camera(&base_camera, frame_index);
    render_home_thumbnail(
        Arc::clone(&mesh),
        base_camera,
        camera,
        frame_index,
        Some(Arc::clone(&mesh)),
    )
}

pub(super) fn render_home_thumbnail(
    mesh: Arc<TriangleMesh>,
    base_camera: OrbitCamera,
    camera: OrbitCamera,
    frame_index: i32,
    include_mesh: Option<Arc<TriangleMesh>>,
) -> Option<HomeThumbnailJobOutput> {
    let settings = clay_readability_render_settings(HOME_THUMBNAIL_PIXELS, HOME_THUMBNAIL_PIXELS);
    let image = render_mesh(mesh.as_ref(), &camera, &settings).ok()?;
    Some(HomeThumbnailJobOutput {
        mesh: include_mesh,
        base_camera,
        frame: HomeThumbnailFrame {
            frame_index,
            rgba8: image.rgba8,
            width: image.width,
            height: image.height,
        },
    })
}

#[cfg(test)]
pub(super) fn orbit_home_thumbnail_camera(camera: &OrbitCamera, delta: egui::Vec2) -> OrbitCamera {
    let mut camera = camera.clone();
    camera.orbit(delta.x * HOME_THUMBNAIL_YAW_DEGREES_PER_POINT, 0.0);
    camera
}

pub(super) fn current_preview_orbit_camera(
    preview: &FoundryPreviewImage,
    delta: egui::Vec2,
) -> Option<OrbitCamera> {
    current_preview_orbit_camera_from_base(&preview.camera, delta)
}

pub(super) fn current_preview_orbit_camera_from_base(
    base_camera: &OrbitCamera,
    delta: egui::Vec2,
) -> Option<OrbitCamera> {
    if delta.length_sq() <= 0.01 {
        return None;
    }

    let mut camera = base_camera.clone();
    camera.target = Default::default();
    camera.orbit(
        delta.x * CURRENT_PREVIEW_ORBIT_DEGREES_PER_POINT,
        -delta.y * CURRENT_PREVIEW_ORBIT_DEGREES_PER_POINT,
    );
    Some(camera)
}

pub(super) fn current_preview_orbit_button() -> egui::PointerButton {
    egui::PointerButton::Secondary
}

pub(super) fn home_turntable_yaw(yaw_degrees: f32) -> f32 {
    yaw_degrees.rem_euclid(360.0)
}

pub(super) fn home_turntable_frame_index(yaw_degrees: f32) -> i32 {
    let frame_width = 360.0 / HOME_TURNTABLE_FRAME_COUNT as f32;
    ((home_turntable_yaw(yaw_degrees) / frame_width).round() as i32)
        .rem_euclid(HOME_TURNTABLE_FRAME_COUNT)
}

pub(super) fn home_turntable_camera(base_camera: &OrbitCamera, frame_index: i32) -> OrbitCamera {
    let mut camera = base_camera.clone();
    let frame_width = 360.0 / HOME_TURNTABLE_FRAME_COUNT as f32;
    camera.yaw_degrees = home_turntable_yaw(frame_index as f32 * frame_width);
    camera.clamped()
}

pub(super) fn home_turntable_prewarm_order(selected_frame_index: i32) -> Vec<i32> {
    let selected = selected_frame_index.rem_euclid(HOME_TURNTABLE_FRAME_COUNT);
    let mut frames = Vec::with_capacity(HOME_TURNTABLE_FRAME_COUNT as usize);
    frames.push(selected);
    for distance in 1..=HOME_TURNTABLE_FRAME_COUNT / 2 {
        frames.push((selected + distance).rem_euclid(HOME_TURNTABLE_FRAME_COUNT));
        let opposite = (selected - distance).rem_euclid(HOME_TURNTABLE_FRAME_COUNT);
        if opposite != *frames.last().expect("distance frame was inserted") {
            frames.push(opposite);
        }
    }
    frames.truncate(HOME_TURNTABLE_FRAME_COUNT as usize);
    frames
}

pub(super) fn nearest_home_thumbnail_frame(
    thumbnail: &HomeTemplateThumbnail,
) -> Option<&HomeThumbnailFrame> {
    thumbnail.frames.values().min_by_key(|frame| {
        home_turntable_frame_distance(frame.frame_index, thumbnail.selected_frame_index)
    })
}

pub(super) fn home_turntable_frame_distance(left: i32, right: i32) -> i32 {
    let direct = (left - right).abs();
    direct.min(HOME_TURNTABLE_FRAME_COUNT - direct)
}
