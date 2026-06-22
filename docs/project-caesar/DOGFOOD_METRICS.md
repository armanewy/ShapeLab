# Project Caesar Dogfood Metrics

The River Bend pack should be judged by whether it improves actual game production, not by generic asset-library breadth.

## Required Local Metrics

- template opened
- candidate generations requested
- candidates accepted
- manual scalar edits
- validation failures
- export duration
- whether Blender repair was needed
- whether the user returned to edit the asset after game integration

No external telemetry is required. These metrics should be written locally only when explicitly enabled.

## Mandatory Dogfood Questions

- Did the generated assets save time over manual modeling?
- Could all nine modules enter the game without Blender repair?
- Were the assets recognizable at the actual gameplay camera?
- Did Refine or Explore produce at least three variants worth comparing?
- Were pivots, sockets, and footprints correct?
- Were construction phases useful?
- Which three families will the game need repeatedly?
- Which Shape Lab controls were irrelevant?
- Which missing modeling operation blocked a real asset?

## First Acceptance Signals

- all nine River Bend module keys export from one pack command
- all templates compile without validation issues
- all gamekit metadata validates
- fixed-camera previews are nonempty
- 32-pixel silhouettes are recognizable enough for triage
- pack export is deterministic across repeated runs
- Godot import does not require gameplay-balance duplication
