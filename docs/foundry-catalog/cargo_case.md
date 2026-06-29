# Cargo Case

Date: 2026-06-29

Cargo Case is the first reusable case/crate base-family proof. The base family
is an executable structural grammar for clay equipment cases; it is not a broad
archetype system.

## Roles

Required roles:

- body
- lid
- panel_fields
- edge_trim
- corner_guards
- base_feet_or_skids

Optional roles:

- handles
- latches
- vents
- fasteners
- reinforcement_bands
- utility_rails
- side_grilles
- label_plate_geometry
- hinge_or_closure_detail

## Primary Controls

- Overall Proportions
- Structural Heft
- Panel Complexity
- Handle Style
- Vent Density
- Trim Style
- Detail Density

Every primary control is intended to change visible clay geometry. Topology
changing controls use discrete choices; continuous controls preserve topology.

## Clay Metadata

Pure Clay uses one neutral gray display value. Semantic Clay uses neutral gray
display values for primary mass, secondary panels, structural trim, vents, and
fasteners/details. This is preview metadata only: no UVs, texture maps, decals,
labels, logos, material editor, or image-based detail is implied.

## Base Candidate Strategies

- Light Utility
- Reinforced
- Compact
- Wide
- Minimal
- Detailed

## Clean Utility Case Profile

Clean Utility Case is a Cargo Case family profile with a quieter practical
equipment-case bias. It uses the same Cargo Case family grammar, role set,
control vocabulary, provider slots, and clay metadata as the base family proof.

Clean Utility defaults:

- lower panel complexity
- lower detail density
- clean trim
- sparse vents
- flush grip handles
- modest corner guards
- light/medium structural heft

Clean Utility avoids dense sci-fi vents, heavy industrial rails, cargo-bar
handles as the primary default, heavy industrial bands as the primary default,
decals, logos, text, grime, UVs, texture maps, and material maps.

Clean Utility candidate strategies:

- Light Utility
- Compact Carry Case
- Clean Storage Case
- Reinforced Utility
- Minimal Field Case
