# Asset App Contracts

This document defines the future explicit-asset app surface. It is a contract only; this change does not implement asset app mode.

## AssetAppState

`AssetAppState` is the client-visible state for explicit asset authoring.

- `recipe`: current `AssetRecipe` document.
- `selected_part_instance`: optional selected `PartInstanceId`.
- `selected_region`: optional selected `RegionId`.
- `selected_socket`: optional selected `SocketId`.
- `parameters`: reflected editable parameters with lock state.
- `timeline`: latest `ConstructionTimelineReport`.
- `validation`: latest compile and model validation summaries.
- `preview`: latest `AssetCandidatePreview`, when available.
- `active_job`: optional active `AssetJobRequest` ID.
- `stale_jobs`: completed or cancelled job IDs ignored by the state reducer.

## AssetAppCommand

`AssetAppCommand` is the user intent boundary. Commands must not mutate geometry directly; they produce recipe edits or job requests.

- `SelectPart(PartInstanceId)`
- `SelectRegion(PartInstanceId, RegionId)`
- `SelectSocket(PartInstanceId, SocketId)`
- `SetParameter(ParameterId, f32)`
- `SetTransform(PartInstanceId, Transform3)`
- `ToggleOptionalPart(PartInstanceId, bool)`
- `RequestPreview(AssetJobRequest)`
- `RequestCompile(AssetJobRequest)`
- `CancelJob(JobId)`
- `AcceptCandidate(CandidateId)`
- `RejectCandidate(CandidateId)`

## AssetJobRequest

`AssetJobRequest` describes deterministic background work.

- `job_id`: stable request ID.
- `recipe_revision`: recipe revision or source hash.
- `kind`: `Preview`, `Compile`, `Inspect`, or `CandidateSearch`.
- `recipe`: full recipe snapshot, not a mutable reference.
- `validation_limits`: optional model validation budgets.
- `output_policy`: preview only, package export, or both.

## AssetJobEvent

`AssetJobEvent` is append-only and reducible in order.

- `Queued(job_id)`
- `Started(job_id)`
- `Progress(job_id, phase, completed, total)`
- `PreviewReady(job_id, AssetCandidatePreview)`
- `CompileReady(job_id, package_paths, validation)`
- `Failed(job_id, message)`
- `Cancelled(job_id)`

Reducers must ignore events whose `job_id` is not the current active job for that request kind.

## AssetCandidatePreview

`AssetCandidatePreview` is a lightweight render and topology summary.

- `candidate_id`
- `recipe_revision`
- `thumbnail_rgba`
- `camera`
- `part_count`
- `triangle_count`
- `region_count`
- `validation_summary`
- `changed_parameters`
- `construction_timeline_summary`

## Texture Ownership Rules

Generated textures are owned by the job that produced them until the user accepts the candidate. Accepted textures become recipe-owned assets and must be copied into project storage with deterministic names. Rejected or stale candidate textures may be deleted after their job is no longer visible.

The app must never mutate a texture in place while another recipe revision references it. Texture references must include revision or content hashes so previews cannot silently point at stale pixels.

## Stale Job Rules

A job is stale when its source recipe hash no longer matches the current app state, when the user cancels it, or when a newer job of the same kind starts. Stale job events are safe to log but must not replace the current preview, validation, timeline, or compile output.

When a stale compile succeeds, its files remain external artifacts only. They must not become the active package unless the user explicitly reloads that exact recipe revision.
