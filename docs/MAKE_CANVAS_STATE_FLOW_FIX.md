# Make Canvas State Flow Fix

## Problem

The failed screenshot gate showed that Make Canvas state could change internally
without changing the visible workspace:

- generated ideas were reported only in the status strip;
- Focus Handles, Handle Ideas, and Focus Vents could capture identical screens;
- Pack and Export scenarios changed state without a visible drawer.

## Fix

This branch treats visible Make Canvas state as a first-class UI contract.

The app now derives a `MakeCanvasVisibleState` with:

- active scope label;
- candidate board visibility;
- selected comparison visibility;
- focused part label;
- local action tray visibility;
- Pack drawer visibility;
- Export drawer visibility.

The screenshot scenarios wait for those visible states instead of accepting
status text as proof.

## Layout Changes

Generated idea states render the candidate tray and comparison above the control
deck. This keeps generated ideas visible in a first-viewport screenshot.

Focused states change the heading to `Focused: Handles` or `Focused: Vents`,
keep the local part chips connected to the model stage, and show focused empty
states when no part-specific candidates survive.

Pack and Export drawers are mounted before the central panel so the central
workspace no longer covers them.

## Guardrails

The branch adds focused tests for:

- visible candidate board and comparison state;
- focus scope changing visible labels;
- focused generation not rendering as Whole asset;
- Pack and Export drawer visibility;
- screenshot focus scenario progression;
- product-visible string restrictions.

It also adds `crates/shape-app/check_make_canvas_screenshots.ps1`, which checks
the nine screenshot files, dimensions, and hash differences between critical
states.
