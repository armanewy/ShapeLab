# Wave 3 Outliner And Inspector Notes

## UI decisions

- The outliner renders a stable flattened view from the document root plus a `Whole Model` pseudo-entry.
- Shared DAG references are shown as `shared` rows and are not recursively expanded again.
- Node kind labels use beginner-facing names such as `Group`, `Soft group`, and `Cut group`; technical graph details are limited to hover text.
- Disabled nodes remain selectable but are styled with weaker italic text and an `off` marker.
- The inspector groups parameters in the fixed order Shape, Position, Rotation, Size, and Soft joining.
- Locked parameters disable their value controls and still expose a lock toggle.
- Reset controls were omitted because the current state contract does not expose reliable preset or revision baseline values for individual parameters.
- Search controls expose target, enabled parameter groups, Refine/Explore mode, seed, proposal count, result count, generation, and cancellation.

## Contract issues

- `AppCommand` and `AppState` do not currently expose proposal-count or result-count fields. The inspector keeps those controls in `InspectorPanelState` so the UI can render bounded values, but `GenerateDirections` cannot yet carry those values to the state layer. The Wave 4 integration should add or reconcile that command/state surface.

## Scope

- No geometry, filesystem, worker, or project mutation work is performed by these panels.
- No graph editing controls were added.
