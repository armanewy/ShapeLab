# Semantic Clay Preview Mode

Date: 2026-06-29

Semantic Clay Preview Mode adds neutral gray viewport shading by semantic
role/part group for clay readability. It is untextured display shading only.

## Display Modes

- Clay: strict Pure Clay, one neutral gray display class.
- Part Contrast: Semantic Clay, neutral gray values by role/part group.
- DiagnosticPartColor: developer/author diagnostic mode only; it is not a
  novice default.

The novice default is Semantic Clay when a profile provides semantic gray
assignments and Pure Clay otherwise.

## Boundaries

Semantic Clay does not imply UV/texturing support, texture files, material maps,
decals, labels, logos, a material editor, or broad Surface mode. It does not
affect export payloads.

Pure Clay remains the strict mesh gate. Semantic Clay can make part groups
easier to read, but it cannot hide weak geometry and must never be used to pass
a failed Pure Clay gate.

Quality reports must record Pure Clay pass/fail separately from Semantic Clay
readability pass/fail.
