# Wave 5 Project I/O Notes

- Project saves now validate schema and documents before writing, serialize deterministic pretty JSON with a trailing newline, write a temporary sibling file, flush it, and atomically persist it over the requested path.
- Project loads first probe `schema_version`; versions newer than this build fail with `FutureSchemaVersion` before attempting full deserialization. Older unsupported versions fail without migration.
- Project and OBJ path I/O errors include the target path. Malformed project JSON includes the source path in the error.
- Existing project and OBJ targets are preserved when temporary writes fail or validation fails before replacement. The local temp-file guard creates sibling files with `create_new`, closes them before rename, and deletes the temp file on failed writes or failed replacement. Stale Shape Lab temp files are cleaned only when they match the target-specific prefix and are at least one hour old.
- The app already clears dirty state only after `save_json` succeeds and already loads into a validated `Project` before replacing current app state, so no app state changes were needed.
- Save/export dialogs now suggest conservative ASCII filenames derived from the project title, with Windows reserved names prefixed by `shape-`.
- Autosave/recovery snapshots were not added in this branch; the atomic replacement path keeps current-project files intact without adding new recovery UI or state ownership.
