# Manual Test Checklist

Use a release build unless explicitly testing debug behavior:

```bash
cargo run -p shape-app --release
```

## Desktop Workflow

- App opens without any network, server, browser, DCC, or hosted AI dependency.
- Desk Lamp appears on startup after background preview generation.
- Toy Submarine, Alien Plant, and Sky Shrine load from `New From Preset`.
- Orbit, pan, zoom, and fit update the viewport.
- Selecting a primitive shows relevant beginner-facing parameters.
- Editing a scalar parameter visibly rebuilds the object.
- Locking a parameter prevents that parameter from changing in generated candidates.
- Explore produces visibly distinct candidates.
- Refine produces subtler candidates than Explore.
- Choosing a candidate promotes it to the current model.
- Undo returns to the parent revision.
- Accepting another candidate after undo creates a branch.
- Branch selection rebuilds the selected revision.
- Save and reload preserve project history.
- OBJ export writes a nonempty mesh that opens in a standard mesh viewer.
- Candidate generation can be cancelled without corrupting state.
- Loading a project while old jobs are returning does not overwrite the newer state.
- Closing or replacing a dirty project warns or clearly communicates risk where current `eframe` integration permits it.

## Failure Behavior

- Malformed project JSON fails clearly.
- Future schema-version JSON fails clearly.
- Invalid export paths report an error without replacing previous output.
- Generation with all mutable parameters locked reports failure or no candidates without panicking.
- Extremely small windows remain usable enough to recover.
- Idle app does not repaint continuously without active jobs or interaction.

