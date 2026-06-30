# Box Primitive UI Cleanup

## Purpose

The current product baseline is Box Primitive only.

The UI should therefore stop implying broader catalog breadth, part-level editing, or
material-look workflows that are not part of this baseline.

## Changes

- Hide category filter chips on the Choose screen.
- Use “starting point” copy instead of broad “asset template” copy.
- Hide Box Primitive part-focus chips, including the misleading single `Body` chip.
- Hide the Material Looks tray for Box Primitive.
- Keep Pack and Export available, but keep Material Looks out of the baseline path.

## Rationale

A dead-simple baseline should prove one workflow:

```text
Choose Box Primitive
→ Make ready
→ Try box ideas
→ Use one box
→ Adjust the box
→ Add to Pack
→ Export
```

It should not ask the user to understand categories, part selection, surface-only
state, or future material support.

## Follow-up

Future rungs can reintroduce these concepts one at a time:

- Lid/top capability
- Trim band capability
- Feet/skids capability
- Panels
- Handles
- Material looks

Each rung needs its own screenshot/manual gate before becoming part of the default
baseline.
