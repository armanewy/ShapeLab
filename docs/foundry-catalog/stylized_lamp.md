# Stylized Lamp Foundry Profile

The stylized lamp catalog entry defines a compact whole-model customizer for a theme-neutral clay lamp. It uses seven primary controls:

- Overall Height
- Base Weight
- Stem Curvature
- Joint Size
- Shade Style
- Shade Scale
- Edge Softness

## Construction

The assembled model uses a lathed base body with a connected slab foot, a swept curve-backed stem, two explicit pivot disc joints, and a shade provider selected by Shade Style. Required attachment rules keep stem-to-base, joint-to-stem, and shade-to-stem relationships present across provider swaps.

The profile avoids a capsule-chain fallback by requiring a lathe source for the base and a sweep source for the stem. Cylinders are limited to explicit joint and shade details. The base-weight control broadens the round lathed body, changes the slab-foot footprint, and thickens the support language enough to read from whole-model cards.

## Shade Providers

Shade Style maps to executable provider alternatives:

- `cone` -> `ribbed_cone_shade`
- `drum` -> `banded_drum_shade`
- `task` -> `angled_task_shade`
- `wide` -> `wide_reading_shade`
- `minimal` -> `minimal_shade`

The providers keep different authored body silhouettes instead of sharing one overwritten frustum. Shade Scale now scales the shade instance, preserving cone taper, drum straightness, task-shade angle, wide reading spread, and minimal compactness. Cone, drum, task, and wide providers include trim or bracket detail; the minimal provider keeps the same shade role socket without extra trim.

## Candidate Directions

The customizer profile exposes these strategy labels for candidate workflows:

- Compact Task Lamp
- Tall Reading Lamp
- Playful Curved Lamp
- Heavy Base
- Minimal Studio Lamp
- Wide Shade Lamp

The catalog tests exercise these directions through compiled control states and require at least four distinct whole-model proportions. The strategies deliberately combine height, base weight, curvature, shade style, shade scale, joint size, and edge softness so direction cards are not only tiny detail edits.

## HQ Authoring Notes

The reauthoring pass used the prompt roles as internal review lenses:

- Art Director: require readable compact/tall, light/heavy, straight/curved, narrow/wide silhouettes.
- Geometry Author: keep base, stem, joints, trim, bracket, and shade as explicit connected clay geometry.
- Variation Designer: preserve provider-specific silhouettes under Shade Scale and add a wide reading shade direction.
- Validation Engineer: assert model validation, attachment conformance, structural provider differences, and role-bound control visibility.
- Adversarial Critic: reject capsule-chain fallback, TooSubtle candidate labels, and disconnected shade/stem/base assemblies.
