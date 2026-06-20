//! Background job request, event, and coordinator contracts.
//!
//! This module is deliberately UI-independent. Worker threads move only plain
//! data through channels and leave app state and texture ownership to the UI
//! coordinator.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fmt;
use std::panic::{self, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, unbounded};
use shape_core::ShapeDocument;
use shape_field::compile_document;
use shape_mesh::{MeshSettings, TriangleMesh, mesh_field, write_obj_to_path};
use shape_render::{OrbitCamera, RenderSettings, RenderedImage, fit_camera_to_bounds, render_mesh};
use shape_search::{Candidate, SearchRequest, generate_candidates};

const FIRST_JOB_ID: u64 = 1;
const FIRST_GENERATION_ID: u64 = 1;
const MAX_DEFAULT_WORKERS: usize = 4;

/// Monotonic job identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct JobId(pub u64);

/// Monotonic candidate generation identifier.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct GenerationId(pub u64);

/// Request executed off the UI thread.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JobRequest {
    BuildCurrentPreview {
        job_id: JobId,
        document: ShapeDocument,
        mesh_settings: MeshSettings,
        render_settings: RenderSettings,
        camera: Option<OrbitCamera>,
    },
    GenerateCandidates {
        job_id: JobId,
        generation_id: GenerationId,
        document: ShapeDocument,
        request: SearchRequest,
        candidate_mesh_settings: MeshSettings,
        thumbnail_settings: RenderSettings,
        camera: OrbitCamera,
    },
    RenderCurrentCamera {
        job_id: JobId,
        mesh: TriangleMesh,
        camera: OrbitCamera,
        render_settings: RenderSettings,
    },
    ExportCurrent {
        job_id: JobId,
        mesh: TriangleMesh,
        path: PathBuf,
    },
}

impl JobRequest {
    /// Return the request's job identifier.
    pub(crate) fn job_id(&self) -> JobId {
        match self {
            JobRequest::BuildCurrentPreview { job_id, .. }
            | JobRequest::GenerateCandidates { job_id, .. }
            | JobRequest::RenderCurrentCamera { job_id, .. }
            | JobRequest::ExportCurrent { job_id, .. } => *job_id,
        }
    }

    /// Return the request's generation identifier when it has one.
    pub(crate) fn generation_id(&self) -> Option<GenerationId> {
        match self {
            JobRequest::GenerateCandidates { generation_id, .. } => Some(*generation_id),
            JobRequest::BuildCurrentPreview { .. }
            | JobRequest::RenderCurrentCamera { .. }
            | JobRequest::ExportCurrent { .. } => None,
        }
    }
}

/// Candidate preview returned by worker threads.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CandidatePreview {
    pub slot: usize,
    pub candidate: Candidate,
    pub mesh: TriangleMesh,
    pub image: RenderedImage,
}

/// Background phase/progress event.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum JobEvent {
    Started {
        job_id: JobId,
        phase: JobPhase,
    },
    Progress {
        job_id: JobId,
        phase: JobPhase,
        progress: f32,
    },
    CurrentPreviewReady {
        job_id: JobId,
        mesh: TriangleMesh,
        image: RenderedImage,
        camera: OrbitCamera,
    },
    CandidatePreviewReady {
        job_id: JobId,
        generation_id: GenerationId,
        preview: CandidatePreview,
    },
    GenerationComplete {
        job_id: JobId,
        generation_id: GenerationId,
    },
    ExportComplete {
        job_id: JobId,
        path: PathBuf,
    },
    Failed {
        job_id: JobId,
        message: String,
    },
    Cancelled {
        job_id: JobId,
    },
}

/// Worker phase names.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum JobPhase {
    CompileField,
    Mesh,
    Render,
    Search,
    Export,
}

/// Error returned by the job coordinator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum JobCoordinatorError {
    /// No worker can receive more requests.
    Closed,
    /// Worker thread creation failed.
    SpawnFailed(String),
}

impl fmt::Display for JobCoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JobCoordinatorError::Closed => formatter.write_str("job coordinator is closed"),
            JobCoordinatorError::SpawnFailed(message) => {
                write!(formatter, "failed to spawn job worker: {message}")
            }
        }
    }
}

impl std::error::Error for JobCoordinatorError {}

/// Handle returned for a submitted background job.
#[derive(Debug, Clone)]
pub(crate) struct JobHandle {
    job_id: JobId,
    cancellation: Arc<AtomicBool>,
}

impl JobHandle {
    /// Return the job identifier.
    pub(crate) fn job_id(&self) -> JobId {
        self.job_id
    }

    /// Request cancellation. The worker observes this between expensive phases.
    pub(crate) fn cancel(&self) {
        self.cancellation.store(true, Ordering::Release);
    }

    /// Return whether cancellation has been requested.
    pub(crate) fn is_cancelled(&self) -> bool {
        self.cancellation.load(Ordering::Acquire)
    }
}

/// Native background job coordinator.
///
/// The coordinator owns a work queue, event receiver, worker threads, and
/// cancellation flags. It does not know about `AppState`; callers compare event
/// IDs against their active jobs and generations.
#[derive(Debug)]
pub(crate) struct JobCoordinator {
    request_tx: Option<Sender<WorkerMessage>>,
    event_rx: Receiver<JobEvent>,
    cancellations: Arc<Mutex<BTreeMap<JobId, Arc<AtomicBool>>>>,
    next_job_id: AtomicU64,
    next_generation_id: AtomicU64,
    workers: Vec<JoinHandle<()>>,
}

impl JobCoordinator {
    /// Create a coordinator with a small platform-appropriate worker count.
    pub(crate) fn with_default_workers() -> Result<Self, JobCoordinatorError> {
        let worker_count = thread::available_parallelism()
            .map(|count| count.get().saturating_sub(1).clamp(1, MAX_DEFAULT_WORKERS))
            .unwrap_or(1);
        Self::new(worker_count)
    }

    /// Create a coordinator with `worker_count` standard threads.
    pub(crate) fn new(worker_count: usize) -> Result<Self, JobCoordinatorError> {
        let worker_count = worker_count.max(1);
        let (request_tx, request_rx) = unbounded();
        let (event_tx, event_rx) = unbounded();
        let cancellations = Arc::new(Mutex::new(BTreeMap::new()));
        let mut workers = Vec::with_capacity(worker_count);

        for index in 0..worker_count {
            let request_rx = request_rx.clone();
            let event_tx = event_tx.clone();
            let cancellations = Arc::clone(&cancellations);
            let worker = thread::Builder::new()
                .name(format!("shape-job-worker-{index}"))
                .spawn(move || worker_loop(request_rx, event_tx, cancellations))
                .map_err(|error| JobCoordinatorError::SpawnFailed(error.to_string()))?;
            workers.push(worker);
        }

        Ok(Self {
            request_tx: Some(request_tx),
            event_rx,
            cancellations,
            next_job_id: AtomicU64::new(FIRST_JOB_ID),
            next_generation_id: AtomicU64::new(FIRST_GENERATION_ID),
            workers,
        })
    }

    /// Allocate a monotonically increasing job identifier.
    pub(crate) fn next_job_id(&self) -> JobId {
        JobId(self.next_job_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Allocate a monotonically increasing generation identifier.
    pub(crate) fn next_generation_id(&self) -> GenerationId {
        GenerationId(self.next_generation_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Submit a request using the `JobId` already stored in the request.
    pub(crate) fn submit(&self, request: JobRequest) -> Result<JobHandle, JobCoordinatorError> {
        self.submit_with_cancel_state(request, false)
    }

    /// Submit a request that is already cancelled before a worker can start it.
    pub(crate) fn submit_cancelled(
        &self,
        request: JobRequest,
    ) -> Result<JobHandle, JobCoordinatorError> {
        self.submit_with_cancel_state(request, true)
    }

    /// Request cancellation for an active or queued job.
    pub(crate) fn cancel(&self, job_id: JobId) -> bool {
        with_cancellations(&self.cancellations, |cancellations| {
            if let Some(cancellation) = cancellations.get(&job_id) {
                cancellation.store(true, Ordering::Release);
                true
            } else {
                false
            }
        })
    }

    /// Request cancellation for every queued or active job.
    pub(crate) fn cancel_all(&self) {
        with_cancellations(&self.cancellations, |cancellations| {
            for cancellation in cancellations.values() {
                cancellation.store(true, Ordering::Release);
            }
        });
    }

    /// Return a clone of the event receiver for integration with polling loops.
    pub(crate) fn event_receiver(&self) -> Receiver<JobEvent> {
        self.event_rx.clone()
    }

    /// Try to receive one event without blocking.
    pub(crate) fn try_recv(&self) -> Result<JobEvent, crossbeam_channel::TryRecvError> {
        self.event_rx.try_recv()
    }

    /// Receive one event with a timeout.
    pub(crate) fn recv_timeout(&self, timeout: Duration) -> Result<JobEvent, RecvTimeoutError> {
        self.event_rx.recv_timeout(timeout)
    }

    fn submit_with_cancel_state(
        &self,
        request: JobRequest,
        cancelled: bool,
    ) -> Result<JobHandle, JobCoordinatorError> {
        let job_id = request.job_id();
        let cancellation = Arc::new(AtomicBool::new(cancelled));
        with_cancellations(&self.cancellations, |cancellations| {
            cancellations.insert(job_id, Arc::clone(&cancellation));
        });

        let Some(request_tx) = &self.request_tx else {
            remove_cancellation(&self.cancellations, job_id);
            return Err(JobCoordinatorError::Closed);
        };

        if request_tx
            .send(WorkerMessage::Run {
                request,
                cancellation: Arc::clone(&cancellation),
            })
            .is_err()
        {
            remove_cancellation(&self.cancellations, job_id);
            return Err(JobCoordinatorError::Closed);
        }

        Ok(JobHandle {
            job_id,
            cancellation,
        })
    }
}

impl Drop for JobCoordinator {
    fn drop(&mut self) {
        self.cancel_all();
        self.request_tx.take();
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

#[derive(Debug)]
enum WorkerMessage {
    Run {
        request: JobRequest,
        cancellation: Arc<AtomicBool>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
struct JobCancelled;

#[derive(Debug)]
struct JobRunner {
    job_id: JobId,
    cancellation: Arc<AtomicBool>,
    events: Sender<JobEvent>,
}

impl JobRunner {
    fn new(job_id: JobId, cancellation: Arc<AtomicBool>, events: Sender<JobEvent>) -> Self {
        Self {
            job_id,
            cancellation,
            events,
        }
    }

    fn started(&self, phase: JobPhase) {
        self.send(JobEvent::Started {
            job_id: self.job_id,
            phase,
        });
    }

    fn progress(&self, phase: JobPhase, progress: f32) {
        self.send(JobEvent::Progress {
            job_id: self.job_id,
            phase,
            progress: progress.clamp(0.0, 1.0),
        });
    }

    fn fail(&self, message: impl Into<String>) {
        self.send(JobEvent::Failed {
            job_id: self.job_id,
            message: message.into(),
        });
    }

    fn check_cancelled(&self) -> Result<(), JobCancelled> {
        if self.cancellation.load(Ordering::Acquire) {
            self.send(JobEvent::Cancelled {
                job_id: self.job_id,
            });
            Err(JobCancelled)
        } else {
            Ok(())
        }
    }

    fn send(&self, event: JobEvent) {
        let _ = self.events.send(event);
    }
}

fn worker_loop(
    request_rx: Receiver<WorkerMessage>,
    event_tx: Sender<JobEvent>,
    cancellations: Arc<Mutex<BTreeMap<JobId, Arc<AtomicBool>>>>,
) {
    while let Ok(message) = request_rx.recv() {
        match message {
            WorkerMessage::Run {
                request,
                cancellation,
            } => {
                let job_id = request.job_id();
                let worker_event_tx = event_tx.clone();
                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    run_request(request, cancellation, worker_event_tx);
                }));
                if result.is_err() {
                    let _ = event_tx.send(JobEvent::Failed {
                        job_id,
                        message: "worker panicked while running job".to_owned(),
                    });
                }
                remove_cancellation(&cancellations, job_id);
            }
        }
    }
}

fn run_request(request: JobRequest, cancellation: Arc<AtomicBool>, event_tx: Sender<JobEvent>) {
    let job_id = request.job_id();
    let runner = JobRunner::new(job_id, cancellation, event_tx);
    if runner.check_cancelled().is_err() {
        return;
    }

    match request {
        JobRequest::BuildCurrentPreview {
            document,
            mesh_settings,
            render_settings,
            camera,
            ..
        } => run_current_preview(&runner, document, mesh_settings, render_settings, camera),
        JobRequest::GenerateCandidates {
            generation_id,
            document,
            request,
            candidate_mesh_settings,
            thumbnail_settings,
            camera,
            ..
        } => run_candidate_generation(
            &runner,
            generation_id,
            document,
            request,
            candidate_mesh_settings,
            thumbnail_settings,
            camera,
        ),
        JobRequest::RenderCurrentCamera {
            mesh,
            camera,
            render_settings,
            ..
        } => run_camera_render(&runner, mesh, camera, render_settings),
        JobRequest::ExportCurrent { mesh, path, .. } => run_export(&runner, mesh, path),
    }
}

fn run_current_preview(
    runner: &JobRunner,
    document: ShapeDocument,
    mesh_settings: MeshSettings,
    render_settings: RenderSettings,
    camera: Option<OrbitCamera>,
) {
    let Some((mesh, camera)) =
        build_mesh_and_camera(runner, "current preview", &document, mesh_settings, camera)
    else {
        return;
    };
    if runner.check_cancelled().is_err() {
        return;
    }

    runner.started(JobPhase::Render);
    let image = match render_mesh(&mesh, &camera, &render_settings) {
        Ok(image) => image,
        Err(error) => {
            runner.fail(format!("render current preview failed: {error}"));
            return;
        }
    };
    if runner.check_cancelled().is_err() {
        return;
    }

    runner.send(JobEvent::CurrentPreviewReady {
        job_id: runner.job_id,
        mesh,
        image,
        camera,
    });
    runner.progress(JobPhase::Render, 1.0);
}

#[allow(clippy::too_many_arguments)]
fn run_candidate_generation(
    runner: &JobRunner,
    generation_id: GenerationId,
    document: ShapeDocument,
    request: SearchRequest,
    candidate_mesh_settings: MeshSettings,
    thumbnail_settings: RenderSettings,
    camera: OrbitCamera,
) {
    runner.started(JobPhase::Search);
    let candidates = match generate_candidates(&document, &request) {
        Ok(candidates) => candidates,
        Err(error) => {
            runner.fail(format!("candidate search failed: {error}"));
            return;
        }
    };
    if runner.check_cancelled().is_err() {
        return;
    }
    runner.progress(JobPhase::Search, 0.20);

    let total_candidates = candidates.len().max(1);
    for (slot, candidate) in candidates.into_iter().enumerate() {
        if runner.check_cancelled().is_err() {
            return;
        }
        let progress_base = 0.20 + 0.80 * (slot as f32 / total_candidates as f32);
        runner.progress(JobPhase::Search, progress_base);

        let Some(field) = compile_candidate_field(runner, slot, &candidate.document) else {
            continue;
        };
        if runner.check_cancelled().is_err() {
            return;
        }

        runner.started(JobPhase::Mesh);
        let mesh = match mesh_field(&field, candidate_mesh_settings) {
            Ok(mesh) => mesh,
            Err(error) => {
                runner.fail(format!("candidate slot {slot} mesh failed: {error}"));
                continue;
            }
        };
        if runner.check_cancelled().is_err() {
            return;
        }

        runner.started(JobPhase::Render);
        let image = match render_mesh(&mesh, &camera, &thumbnail_settings) {
            Ok(image) => image,
            Err(error) => {
                runner.fail(format!("candidate slot {slot} render failed: {error}"));
                continue;
            }
        };
        if runner.check_cancelled().is_err() {
            return;
        }

        runner.send(JobEvent::CandidatePreviewReady {
            job_id: runner.job_id,
            generation_id,
            preview: CandidatePreview {
                slot,
                candidate,
                mesh,
                image,
            },
        });
        runner.progress(
            JobPhase::Search,
            0.20 + 0.80 * ((slot + 1) as f32 / total_candidates as f32),
        );
    }

    if runner.check_cancelled().is_err() {
        return;
    }
    runner.send(JobEvent::GenerationComplete {
        job_id: runner.job_id,
        generation_id,
    });
}

fn run_camera_render(
    runner: &JobRunner,
    mesh: TriangleMesh,
    camera: OrbitCamera,
    render_settings: RenderSettings,
) {
    runner.started(JobPhase::Render);
    let image = match render_mesh(&mesh, &camera, &render_settings) {
        Ok(image) => image,
        Err(error) => {
            runner.fail(format!("render camera failed: {error}"));
            return;
        }
    };
    if runner.check_cancelled().is_err() {
        return;
    }

    runner.send(JobEvent::CurrentPreviewReady {
        job_id: runner.job_id,
        mesh,
        image,
        camera,
    });
    runner.progress(JobPhase::Render, 1.0);
}

fn run_export(runner: &JobRunner, mesh: TriangleMesh, path: PathBuf) {
    runner.started(JobPhase::Export);
    if runner.check_cancelled().is_err() {
        return;
    }

    match write_obj_to_path(&mesh, &path) {
        Ok(()) => {
            if runner.check_cancelled().is_err() {
                return;
            }
            runner.send(JobEvent::ExportComplete {
                job_id: runner.job_id,
                path,
            });
            runner.progress(JobPhase::Export, 1.0);
        }
        Err(error) => runner.fail(format!("export failed: {error}")),
    }
}

fn build_mesh_and_camera(
    runner: &JobRunner,
    label: &str,
    document: &ShapeDocument,
    mesh_settings: MeshSettings,
    camera: Option<OrbitCamera>,
) -> Option<(TriangleMesh, OrbitCamera)> {
    runner.started(JobPhase::CompileField);
    let field = match compile_document(document) {
        Ok(field) => field,
        Err(error) => {
            runner.fail(format!("compile {label} failed: {error}"));
            return None;
        }
    };
    if runner.check_cancelled().is_err() {
        return None;
    }
    runner.progress(JobPhase::CompileField, 1.0);

    runner.started(JobPhase::Mesh);
    let mesh = match mesh_field(&field, mesh_settings) {
        Ok(mesh) => mesh,
        Err(error) => {
            runner.fail(format!("mesh {label} failed: {error}"));
            return None;
        }
    };
    if runner.check_cancelled().is_err() {
        return None;
    }
    runner.progress(JobPhase::Mesh, 1.0);

    let camera = camera.unwrap_or_else(|| fit_camera_to_bounds(mesh.bounds));
    Some((mesh, camera))
}

fn compile_candidate_field(
    runner: &JobRunner,
    slot: usize,
    document: &ShapeDocument,
) -> Option<shape_field::CompiledField> {
    runner.started(JobPhase::CompileField);
    match compile_document(document) {
        Ok(field) => Some(field),
        Err(error) => {
            runner.fail(format!("candidate slot {slot} compile failed: {error}"));
            None
        }
    }
}

fn with_cancellations<T>(
    cancellations: &Arc<Mutex<BTreeMap<JobId, Arc<AtomicBool>>>>,
    operation: impl FnOnce(&mut BTreeMap<JobId, Arc<AtomicBool>>) -> T,
) -> T {
    match cancellations.lock() {
        Ok(mut guard) => operation(&mut guard),
        Err(poisoned) => {
            let mut guard = poisoned.into_inner();
            operation(&mut guard)
        }
    }
}

fn remove_cancellation(
    cancellations: &Arc<Mutex<BTreeMap<JobId, Arc<AtomicBool>>>>,
    job_id: JobId,
) {
    with_cancellations(cancellations, |cancellations| {
        cancellations.remove(&job_id);
    });
}
