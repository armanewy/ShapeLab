# Wave 3 Jobs Notes

## Concurrency Model

- `JobCoordinator` owns a crossbeam work queue, a crossbeam event receiver, and a fixed set of standard worker threads.
- Callers allocate monotonic `JobId` and `GenerationId` values through the coordinator, store those IDs in `JobRequest`, and compare all returned `JobEvent` IDs against app state before applying results.
- Cancellation is represented by an `Arc<AtomicBool>` per job. The coordinator stores those flags by `JobId`, and workers check them before starting and between compile, search, mesh, render, and export phases.
- Dropping the coordinator requests cancellation for all known jobs, closes the work queue, and joins worker threads.
- Worker threads only send plain data events. They do not access app state, textures, or GUI types.

## Behavior

- Current preview jobs compile the document, mesh it, fit the camera when no camera is supplied, render an image, and send the mesh, image, and camera together.
- Candidate generation runs `shape-search` first, then compiles, meshes, and renders each survivor into a `CandidatePreview`.
- Candidate previews use the survivor index as the stable slot number. Candidate failures are reported as recoverable job failures and do not stop later survivors.
- Camera render jobs rerender an existing mesh with the supplied camera and render settings, allowing callers to pass lower interactive resolutions.
- Export jobs write OBJ through `shape-mesh` and report completion or failure.

## Contract Issues

- `JobRequest` already carries IDs, so the coordinator exposes monotonic ID allocation but does not rewrite request IDs during `submit`. The state layer must allocate IDs through `JobCoordinator::next_job_id` and `next_generation_id` before submitting effects.
- Candidate-specific failure has no dedicated event variant. The coordinator uses `JobEvent::Failed` with a slot-specific message while continuing generation.
