# Stylized Lamp Foundry Profile

The stylized lamp catalog entry defines a compact whole-model customizer for a theme-neutral lamp. It uses seven primary controls:

- Overall Height
- Base Weight
- Stem Curvature
- Joint Size
- Shade Style
- Shade Scale
- Edge Softness

## Construction

The assembled model uses a lathed weighted base, a swept curve-backed stem, two explicit pivot disc joints, and a shade provider selected by Shade Style. The family has required attachment rules for stem-to-base, joint-to-stem, and shade-to-stem. The implementation binds those rules through exported socket ports so provider changes preserve attachment endpoints and keep a single connected assembly.

The profile avoids a capsule-chain fallback by requiring a lathe source for the base and a sweep source for the stem. Cylinders are limited to explicit joint and shade-style details rather than becoming the whole silhouette.

## Shade Providers

Shade Style maps to executable provider alternatives:

- `cone` -> `ribbed_cone_shade`
- `drum` -> `banded_drum_shade`
- `task` -> `angled_task_shade`
- `minimal` -> `minimal_shade`

The cone, drum, and task providers include authored trim or bracket detail inside the shade subtree. Shade Scale maps to a trim-safe frustum envelope so fixed trim rings remain outside the scaled shade body across exposed endpoints. The task provider keeps its bracket offset behind the shade body, which preserves the authored detail without bracket, trim, or frustum intersections. The minimal provider omits those details while retaining the same shade role socket, which keeps attachment behavior stable.

## Candidate Directions

The customizer profile exposes these strategy labels for candidate workflows:

- Compact Task Lamp
- Tall Reading Lamp
- Playful Curved Lamp
- Heavy Base
- Minimal

The current candidate API stores strategy metadata and editable control sets, so tests exercise six concrete candidate states through the compiled document path. The compact task candidate uses the task shade with compact dimensions so the candidate coverage samples the authored task detail, not only the default cone provider.
