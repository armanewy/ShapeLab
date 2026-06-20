#![forbid(unsafe_code)]

#[path = "../src/jobs.rs"]
mod jobs;

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use crossbeam_channel::RecvTimeoutError;
use jobs::{GenerationId, JobCoordinator, JobEvent, JobId, JobRequest};
use shape_core::{Aabb, NodeId, NodeKind, ParamGroup, PrimitiveKind, ShapeDocument, ShapeNode};
use shape_mesh::{MeshSettings, TriangleMesh};
use shape_render::{OrbitCamera, RenderSettings};
use shape_search::{ExplorationMode, SearchRequest, TargetScope};

const EVENT_TIMEOUT: Duration = Duration::from_secs(8);

#[test]
fn current_preview_job_succeeds() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");
    let job_id = coordinator.next_job_id();

    coordinator
        .submit(JobRequest::BuildCurrentPreview {
            job_id,
            document: sphere_document(),
            mesh_settings: small_mesh_settings(),
            render_settings: small_render_settings(),
            camera: None,
        })
        .expect("request should submit");

    let events = collect_until(&coordinator, |event| {
        matches!(
            event,
            JobEvent::CurrentPreviewReady { .. } | JobEvent::Failed { .. }
        )
    });

    assert!(
        events.iter().any(|event| matches!(
            event,
            JobEvent::Started {
                job_id: id,
                phase: jobs::JobPhase::CompileField,
            } if *id == job_id
        )),
        "compile phase was not reported: {events:?}"
    );
    let ready = events
        .iter()
        .find_map(|event| match event {
            JobEvent::CurrentPreviewReady {
                job_id: id,
                mesh,
                image,
                camera,
            } if *id == job_id => Some((mesh, image, camera)),
            _ => None,
        })
        .expect("current preview should be ready");

    assert!(!ready.0.positions.is_empty());
    assert_eq!(ready.1.width, small_render_settings().width);
    assert_eq!(ready.1.height, small_render_settings().height);
    assert!(ready.2.distance.is_finite());
    assert_no_failures(&events);
}

#[test]
fn generation_job_sends_completion() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");
    let job_id = coordinator.next_job_id();
    let generation_id = coordinator.next_generation_id();

    coordinator
        .submit(JobRequest::GenerateCandidates {
            job_id,
            generation_id,
            document: sphere_document(),
            request: small_search_request(31, 3),
            candidate_mesh_settings: small_mesh_settings(),
            thumbnail_settings: small_render_settings(),
            camera: OrbitCamera::default(),
        })
        .expect("request should submit");

    let events = collect_until(&coordinator, |event| {
        matches!(
            event,
            JobEvent::GenerationComplete { .. } | JobEvent::Failed { .. }
        )
    });

    assert!(events.iter().any(|event| matches!(
        event,
        JobEvent::GenerationComplete {
            job_id: id,
            generation_id: generation,
        } if *id == job_id && *generation == generation_id
    )));
    assert!(
        events
            .iter()
            .any(|event| matches!(event, JobEvent::CandidatePreviewReady { .. })),
        "generation completed without any candidate previews: {events:?}"
    );
}

#[test]
fn invalid_document_sends_failure() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");
    let job_id = coordinator.next_job_id();
    let mut document = sphere_document();
    document.root = NodeId(999);

    coordinator
        .submit(JobRequest::BuildCurrentPreview {
            job_id,
            document,
            mesh_settings: small_mesh_settings(),
            render_settings: small_render_settings(),
            camera: None,
        })
        .expect("request should submit");

    let events = collect_until(&coordinator, |event| {
        matches!(event, JobEvent::Failed { .. })
    });

    assert!(events.iter().any(|event| matches!(
        event,
        JobEvent::Failed { job_id: id, message }
            if *id == job_id && message.contains("compile current preview failed")
    )));
}

#[test]
fn cancelled_job_stops_before_meaningful_work() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");
    let job_id = coordinator.next_job_id();

    coordinator
        .submit_cancelled(JobRequest::BuildCurrentPreview {
            job_id,
            document: sphere_document(),
            mesh_settings: small_mesh_settings(),
            render_settings: small_render_settings(),
            camera: None,
        })
        .expect("request should submit");

    let events = collect_until(&coordinator, |event| {
        matches!(
            event,
            JobEvent::Cancelled { .. }
                | JobEvent::CurrentPreviewReady { .. }
                | JobEvent::Failed { .. }
        )
    });

    assert!(
        events
            .iter()
            .any(|event| { matches!(event, JobEvent::Cancelled { job_id: id } if *id == job_id) })
    );
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, JobEvent::CurrentPreviewReady { .. })),
        "cancelled job produced a preview: {events:?}"
    );
}

#[test]
fn candidate_slot_ordering_is_stable() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");
    let job_id = coordinator.next_job_id();
    let generation_id = GenerationId(9);

    coordinator
        .submit(JobRequest::GenerateCandidates {
            job_id,
            generation_id,
            document: sphere_document(),
            request: small_search_request(44, 4),
            candidate_mesh_settings: small_mesh_settings(),
            thumbnail_settings: small_render_settings(),
            camera: OrbitCamera::default(),
        })
        .expect("request should submit");

    let events = collect_until(&coordinator, |event| {
        matches!(event, JobEvent::GenerationComplete { .. })
    });
    let slots = events
        .iter()
        .filter_map(|event| match event {
            JobEvent::CandidatePreviewReady {
                generation_id: generation,
                preview,
                ..
            } if *generation == generation_id => Some(preview.slot),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        !slots.is_empty(),
        "no candidate previews produced: {events:?}"
    );
    assert_eq!(slots, (0..slots.len()).collect::<Vec<_>>());
}

#[test]
fn worker_events_contain_no_gui_types() {
    let source = include_str!("../src/jobs.rs");

    assert!(!source.contains("egui"));
    assert!(!source.contains("TextureHandle"));
}

#[test]
fn dropping_coordinator_shuts_workers_down_cleanly() {
    let start = Instant::now();

    {
        let coordinator = JobCoordinator::new(2).expect("coordinator should start");
        let job_id = coordinator.next_job_id();
        coordinator
            .submit(JobRequest::RenderCurrentCamera {
                job_id,
                mesh: empty_mesh(),
                camera: OrbitCamera::default(),
                render_settings: tiny_render_settings(),
            })
            .expect("request should submit");
    }

    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn coordinator_allocates_monotonic_ids() {
    let coordinator = JobCoordinator::new(1).expect("coordinator should start");

    assert_eq!(coordinator.next_job_id(), JobId(1));
    assert_eq!(coordinator.next_job_id(), JobId(2));
    assert_eq!(coordinator.next_generation_id(), GenerationId(1));
    assert_eq!(coordinator.next_generation_id(), GenerationId(2));
}

fn collect_until(
    coordinator: &JobCoordinator,
    mut stop: impl FnMut(&JobEvent) -> bool,
) -> Vec<JobEvent> {
    let deadline = Instant::now() + EVENT_TIMEOUT;
    let mut events = Vec::new();

    loop {
        let now = Instant::now();
        assert!(
            now < deadline,
            "timed out waiting for job event: {events:?}"
        );
        let remaining = deadline.saturating_duration_since(now);
        match coordinator.recv_timeout(remaining.min(Duration::from_millis(250))) {
            Ok(event) => {
                let should_stop = stop(&event);
                events.push(event);
                if should_stop {
                    return events;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                panic!("job event channel disconnected: {events:?}");
            }
        }
    }
}

fn assert_no_failures(events: &[JobEvent]) {
    let failures = events
        .iter()
        .filter(|event| matches!(event, JobEvent::Failed { .. }))
        .collect::<Vec<_>>();
    assert!(failures.is_empty(), "unexpected failures: {failures:?}");
}

fn sphere_document() -> ShapeDocument {
    let root = ShapeNode {
        id: NodeId(1),
        name: "Sphere".to_owned(),
        tags: BTreeSet::new(),
        enabled: true,
        transform: Default::default(),
        kind: NodeKind::Primitive(PrimitiveKind::Sphere { radius: 0.8 }),
    };
    ShapeDocument::new("Sphere", root)
}

fn small_search_request(seed: u64, result_count: usize) -> SearchRequest {
    SearchRequest {
        seed,
        proposal_count: 48,
        result_count,
        descriptor_resolution: 4,
        selected_node: Some(NodeId(1)),
        target_scope: TargetScope::WholeModel,
        enabled_groups: [
            ParamGroup::Form,
            ParamGroup::Placement,
            ParamGroup::Rotation,
            ParamGroup::Scale,
            ParamGroup::Blend,
        ]
        .into_iter()
        .collect(),
        mode: ExplorationMode::Explore,
    }
}

fn small_mesh_settings() -> MeshSettings {
    MeshSettings {
        resolution: 8,
        padding_fraction: 0.08,
        iso_value: 0.0,
    }
}

fn small_render_settings() -> RenderSettings {
    RenderSettings {
        width: 24,
        height: 24,
        ..RenderSettings::default()
    }
}

fn tiny_render_settings() -> RenderSettings {
    RenderSettings {
        width: 2,
        height: 2,
        ..RenderSettings::default()
    }
}

fn empty_mesh() -> TriangleMesh {
    TriangleMesh {
        positions: Vec::new(),
        normals: Vec::new(),
        indices: Vec::new(),
        bounds: Aabb::empty(),
    }
}
