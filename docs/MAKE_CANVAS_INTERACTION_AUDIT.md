# Make Canvas Interaction Audit

Prompt 1 required this audit before app code edits. The inspection covered:

- `crates/shape-app/src/foundry/app.rs`
- `crates/shape-app/src/foundry/view_model.rs`
- `crates/shape-app/src/foundry/ui/widgets.rs`
- `crates/shape-app/src/foundry/panels/**`
- `docs/MAKE_CANVAS_RECOVERY_PASS.md`
- `docs/FOUNDRY_UI_MANUAL_GATE.md`

## Current Product-State Findings

The Make canvas already has an early `MakeCanvasMode` and
`MakeCanvasViewState`, but the state is incomplete against Prompt 1. Missing
or under-specified fields include explicit asset name, local busy visibility,
focused-part visibility, candidate tray visibility, selected comparison
visibility, local warning vs local error split, and separate candidate rejection
copy. The app also still exposes some Make-critical actions through app-bar
state or old panel wrappers instead of deriving them consistently from the Make
view state.

Template start already switches to Make and schedules a build. Preview rendering
is automatically requested after compile completion in `poll_jobs`. The required
recovery work is therefore mostly about making that path visibly local,
disabling conflicting clicks, and proving states through tests/screenshots.

## Action / State / Screenshot Matrix

| User-visible Make action | Current affordance | Local busy feedback | Disabled reason | Visible state after click | Screenshot proof needed |
| --- | --- | --- | --- | --- | --- |
| Choose Template | Filled primary button in empty Make state and home cards | No Make-local busy until the new document appears | Needs project/open asset copy in empty state | Switches to Make, build starts, current preview waits | `01_choose.png` on home and `02_make_ready.png` after auto-prepare |
| Start | Filled primary button on home profile cards | Home thumbnail jobs are implicit only | None when profile is selectable | Loads chosen fixture, switches to Make, requests build | `02_make_ready.png` shows chosen asset and ready model |
| Try 6 whole-asset ideas | Filled primary button in inspector | Model overlay and tray skeletons exist only while generation is active | Preparing/active-job reason is present but not part of full view-state contract | Candidate generation job starts, tray skeletons show, conflicting controls should be disabled | `03_generating_ideas.png` must show local overlay and skeleton tray |
| Trying ideas... | Filled primary button label while generating | Yes, through model-stage overlay and skeleton tray | Active idea job reason | Conflicting actions remain disabled until job finishes | `03_generating_ideas.png` with `local_busy_visible = true` |
| Select/Compare candidate | Filled secondary-style card/action in candidate grid | None needed | Candidate unavailable details are card-local | Selected card changes, comparison should show current vs candidate | `05_selected_comparison.png` with selected comparison visible |
| Use This Idea / Use this idea | Filled button in selected comparison/candidate card | Apply-edit job becomes local preparing/building state after click | Unknown or unavailable candidate currently surfaces through state error/status | Candidate edit applies, model rebuilds, old ideas clear | Comparison before click, then preparing overlay after acceptance |
| Reject | Filled secondary button in comparison/card | None needed | Disabled when card is not selectable | Candidate removed; selection advances or clears | Candidate count visibly changes in tray |
| Focus part chip | Chip-like button, stronger selected fill when active | No busy; should be disabled during generation | Active job or unavailable part reason | Focus state changes title/action/tray/controls/callout | `06_focus_handles.png` and `09_focus_vents.png` |
| Focus | Filled action required by prompt; current focus is mostly chip based | No busy | Active job/unavailable reason | Same as part-chip focus state | Focus screenshots show selected chip and callout |
| Try handle ideas / Try vent ideas | Filled primary button in focused tray | Model overlay and tray skeletons during focused generation | Preparing/active-job reason | Focused candidate request starts | `07_generating_handle_ideas.png` |
| Lock handles / Lock focused part | Filled secondary button in focused local tray | Apply-edit job should trigger local preparing state | Active job reason | Focused part lock appears and build refreshes | Focus tray screenshot with visible Lock button; later lock state visible |
| Clear focus | Filled secondary button in focused local tray | Apply-edit job should trigger local preparing state | Active job reason | Title/action return to whole asset; controls unfilter | Before/after focus screenshots |
| Use Option | Filled button on option/filmstrip cards | Preview/build jobs should be locally visible through preparing state | Locked/unavailable/active-job reason | Control value changes, build/preview refreshes, stale candidates clear | Control card screenshot with disabled reason and then preparing overlay |
| Reset | Filled button on control cards | Apply-edit job should trigger local preparing state | Already-at-default, locked, or active-job reason | Control returns to default and preview rebuilds | Control card before/after or disabled reason screenshot |
| Lock / Unlock | Filled button on control cards | Apply-edit job should trigger local preparing state | Locked/active-job reason | Lock state visibly changes on control | Control card with selected/locked styling |
| Add to Pack | Filled secondary/top action and pack panel action | Pack compile runs but drawer should open immediately | Needs current project/asset or stale build reason | Pack drawer opens with pack title/count/member cards | `10_pack_drawer.png` with `pack_drawer_visible = true` |
| Open Pack | Required Make action; current app bar effectively opens via Add to Pack | Pack compile state is secondary | Needs asset or stale build reason | Pack drawer opens even when existing pack is shown | `10_pack_drawer.png` |
| Export / Open Export | Filled primary app-bar action | Export job state only after destination chosen | Needs model/current build or stale build reason | Export drawer opens with readiness and honest blocked notes | `11_export_drawer.png` with `export_drawer_visible = true` |
| Export Current Asset | Filled button in export panel | Export job status is bottom-strip oriented | Missing output path/model reason | Native destination flow starts, export job runs | Export drawer screenshot plus blocked/ready note |
| Export Pack | Filled secondary button in pack/export panel | Pack export job status is bottom-strip oriented | Need pack member/coherence reason | Native destination flow starts, pack export job runs | Pack/export drawer screenshots |
| Close drawer | Quiet button in right drawer | None needed | None | Drawer closes, Make canvas returns | Drawer open then closed state |
| Save / Undo / Project actions | App-bar/menu buttons, some quiet | Save/load state in app/status | Save location/history/project reasons | Project state changes, may switch tab | Outside core Make recovery except top app bar must remain visible |

## Required Recovery Targets

- Expand `MakeCanvasViewState` so every Make render can read product state from
  a single derived snapshot.
- Move busy, stale, candidate count, comparison, pack, and export readiness into
  local Make surfaces instead of relying on bottom status.
- Disable generation, candidate acceptance, focus switching, pack, and export
  when they would create or act on stale work.
- Keep the model stage visible while only the right inspector scrolls.
- Keep the candidate tray visible above the fold whenever generation is active
  or candidates exist.
- Make focused part selection visually change title, chip state, stage callout,
  local action tray, and filtered controls.
- Ensure core workflow actions use filled button tones, not quiet/text-only
  styling.
- Add screenshot-state assertions and image sanity reporting before claiming a
  visual pass.
