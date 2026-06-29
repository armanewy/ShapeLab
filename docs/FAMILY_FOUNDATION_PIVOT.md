# Family Foundation Pivot

Date: 2026-06-29

## Decision

Shape Lab is not being built for any one specific model. The product proof must
show that the system can author a small, reusable family and then grow that
family without hiding weak geometry behind later presentation layers.

Sci-Fi Crate was useful as a stress test for authored detail density,
material-look preview evidence, Cargo Case compatibility, and product dogfood
recovery. It is no longer the flagship proof.

The next flagship family-authoring proof starts from Simple Crate Primitive and
then matures through this sequence:

```text
Simple Crate Primitive
-> Utility Crate Family
-> Cargo Case
-> Product profiles
```

## Quality Order

Quality must be proven gradually:

1. Simple Crate proves the smallest object grammar and fast Make loop.
2. Utility Crate proves reusable practical controls and semantic part groups.
3. Cargo Case proves equipment-case richness, provider slots, and profile
   compatibility.
4. Product profiles prove style bias only after the clay family reads clearly.

Clay mesh quality comes before UVs, texturing, materials, decals, or other
surface presentation. Surface and material work remains narrow,
evidence-backed, and tied to existing validated geometry.

No texture, material, or color system may mask weak clay geometry. If the clay
mesh does not read in pure clay, the family is not mature enough for surface
work.

## Scope Boundaries

Sci-Fi Crate remains useful as:

- an advanced regression profile;
- a material-look preview test;
- a Cargo Case compatibility test.

Cargo Case remains valid but scoped to equipment cases only. It does not prove
a broad archetype library, a general Surface mode, a material editor, or full
game-ready output.

Roman Bridge remains `PreviewOnly`.

Broad UV/Texturing/Rigging/Animation UI remains blocked. Rigging, skinning, and
animation UI remain blocked entirely until a separate post-MVP proof explicitly
changes that boundary.
