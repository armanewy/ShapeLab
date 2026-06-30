# Object Orchard Family Authoring Vision

Status: technical vision draft  
Product name in code today: Shape Lab  
Intended product name: Object Orchard

## Purpose

Object Orchard is a visual family-authoring tool for reusable object kits.

The product is not being built to create one specific model. It is being built
to help people create and reuse families of coherent objects.

The first product proof is intentionally small:

```text
Box Primitive
→ one visible feature module
→ a small reusable kit
→ tested variations
→ local draft/personal use
```

The product must not jump directly from a box to a crate, cargo case, material
editor, rigging tool, or broad game-ready pipeline. Each new visible concept
must pass an end-to-end manual gate before the next one is added.

## Naming boundary

Shape Lab is still the current repository name and implementation code name.
The repository may still use `shape-*` crate and module names until the larger
rename happens. Product-facing docs and future UI may use Object Orchard.

The rename is a separate migration. Do not mix the rename with family-authoring
logic, geometry, material, rigging, or export changes.
Do not opportunistically rename files, crates, packages, executables, modules,
or commands in feature branches. See `docs/OBJECT_ORCHARD_NAMING_TRANSITION.md`
for the dedicated naming-transition scaffold.

## Core product promise

Object Orchard should let a non-modeler say:

```text
I want a reusable kit.
This is what the object always is.
These are the parts that can change.
Show me tested variations.
Help me fix weak spots.
Save it as my kit.
```

The user should not need to understand mesh modeling, topology, providers,
candidate strategy graphs, or technical asset-pipeline terms.

## User-facing language

Use these terms in the product:

- reusable kit
- starting point
- what stays the same
- what can change
- parts
- style
- style direction
- ideas
- test variations
- test results
- draft
- personal kit
- needs review

Avoid these terms in novice-facing UI:

- kernel
- module
- provider
- slot
- candidate strategy
- quality gate
- semantic role
- placement zone
- socket
- conformance
- fingerprint
- topology
- artifact
- UV
- rigging
- animation
- game-ready, unless saying explicitly blocked or not game-ready

## Internal architecture

The internal model remains rigorous and typed.

```text
Family kernel
+ Placement zones
+ Feature/capability modules
+ Capability graph
+ Capability cards
+ Style policy
+ Variation plans
+ Candidate generation
+ Quality gates
+ Contact sheets
+ Review tiers
```

The user sees capability cards. The system sees feature modules, a capability
graph, validators, renderers, contact sheets, and review tiers.

User-facing UI must not expose `kernel`, `module`, `provider`, or `slot` terms.
Those are implementation concepts, not product language.

Validators, renderers, and contact sheets decide legality and visibility. LLMs
may draft plans and repairs, but they do not decide final mesh taste. Humans
approve reviewed and showcase quality.

### Family kernel

A kernel defines what must always remain true.

For the current box baseline:

```text
BoxKernel
- closed box-like volume
- canonical width/depth/height axes
- support plane
- exterior faces
- preview camera policy
- identity constraints
```

The kernel must stay small. It must not accumulate every possible future field.
Do not add handle, latch, vent, material, rig, or animation fields to the base
box kernel.

### Placement zones

Placement zones describe where future capabilities may attach.

Examples for a box-like object:

- front face
- back face
- left face
- right face
- top face
- bottom support plane
- edge bands
- corner zones

Placement zones are internal. They answer questions like:

```text
Can a panel attach here?
Can a handle attach here?
Is there enough clearance?
Would this overlap another feature?
```

The UI should never show raw placement-zone IDs.

### Feature modules

A feature module is the scalable unit of family authoring.

A good module declares:

- what it requires
- what roles it provides
- what controls it owns
- what providers or choices it enables
- what placement zones it can use
- what candidate plans it can support
- what quality gates it must pass
- what user-facing capability card represents it

A module is not just a nullable field bundle.

Bad:

```text
BaseBox {
  handle_style: Option<...>,
  latch_style: Option<...>,
  vent_density: Option<...>,
  fastener_count: Option<...>,
}
```

Good:

```text
BoxKernel
+ LidSeamModule
+ TrimBandModule
+ FeetSkidModule
+ PanelFieldsModule
+ HandleModule
```

### Capability cards

Capability cards are the user-facing wrapper around feature modules.

Example:

```text
User sees:
  Add handles
  Lets this kit vary between no handles, side handles, and grip cutouts.

System sees:
  HandleModule
  requires side attachment zones
  provides handle role
  provides Handle Style control
  owns handle provider choices
  requires no-floating-handle gate
```

Capability cards should be graph-aware:

- included
- recommended
- available
- available later
- unavailable

Unavailable or later cards must explain themselves in plain language.

### Variation plans

LLMs and internal assistants may draft variation plans, but validators decide
whether those plans are legal.

For Box Primitive:

```text
Compact Box
- lower width/depth
- balanced height
- normal edge softness

Wide Box
- wider width/depth
- lower height
- normal edge softness

Soft Box
- balanced proportions
- higher edge softness
```

For later families, a plan may add or use modules, but only if the capability
graph says those modules are available and the quality gates pass.

## Family maturity ladder

The product should grow through a ladder, not a feature explosion.

Storage Crate, Cargo Case, and Sci-Fi Industrial work are future ladder
examples, not the current flagship. The current flagship baseline is the small
Box ladder: Box Primitive and Lidded Box.

### Rung 0 — Box Primitive

Purpose: prove the simplest Make loop.

Allowed:

- box-like volume
- proportions
- edge softness
- pack/export
- visible ideas

Blocked:

- crate claims
- part focus
- material looks
- panels
- handles
- latches
- UV/texturing
- rigging/animation

### Rung 1 — Lidded Box

Purpose: add exactly one visible feature.

Allowed:

- lid seam
- lid height or lid line variation

Gate:

- user can see the lid seam in pure clay
- object still reads as a box
- no crate claim yet

### Rung 2 — Trimmed Box

Purpose: add exactly one more visible feature after Lidded Box.

Allowed:

- trim band

Gate:

- trim band is visible in pure clay
- trim does not look like a material stripe
- no new dead-end UI states
- no hidden technical terms

### Rung 3 — Stool Primitive

Purpose: prove the family-authoring protocol is not a box generator.

Allowed:

- seat shape
- seat proportions
- leg count
- leg thickness
- edge softness

Gate:

- object reads as a stool, not a box
- legs reach the support plane
- stool appears stable
- at least four ideas are visually distinct
- the same user-facing kit workflow can drive the second kernel

### Rung 4 — Family Studio Lite

Purpose: expose the reusable-kit authoring flow after two kernels work.

User-facing flow:

- Create reusable kit
- Start from Box Primitive or Stool Primitive
- What stays the same?
- What can change?
- Test variations
- Save Draft / Use Personally

Gate:

- users do not see kernel/module/provider/slot language
- validators/renderers decide legality and visibility
- failed visual gates stop feature growth

### Future — Storage Crate / Utility Crate and Cargo Case

Purpose: earn richer object names only after the box and stool proofs.

Potential modules:

- feet or skids
- panel fields
- handles
- latches
- corner guards
- reinforcement bands

Gate:

- human reviewer agrees the name is visually earned
- at least four ideas are visually distinct
- module suggestions are deterministic and explainable
- no hidden bespoke profile fork

### Future — Profiles and styles

Purpose: apply style policies over proven family modules.

Examples:

- Clean Utility
- Rugged Field
- Sci-Fi Industrial

Style changes geometry defaults and provider choices first. It must not imply
textures or materials until surface evidence exists.

## LLM role

LLMs are assistants, not final art authority.

Allowed LLM tasks:

- draft kit plans
- propose capability cards
- propose module compositions
- name controls and ideas
- generate candidate strategy drafts
- explain failed tests
- suggest repairs
- write review checklists

Blocked LLM tasks:

- publish to novice catalog
- bypass validation
- inject raw mesh or vertex payloads
- silently mutate recipes
- declare art quality final
- claim game-ready status

The deterministic system must validate, compile, render, and reject. Humans or
explicit review records approve catalog/review tiers.

LLMs may draft plans and repairs, but they must not publish kits or generate
final mesh taste.

## Seed/reference role

Seed assets are optional calibration evidence.

Seeds may teach:

- common proportions
- where features usually attach
- silhouette expectations
- bad examples to reject
- provider ideas
- contact-sheet standards

Seeds must not:

- define the family identity
- become automatic editable imports
- become public provider geometry without license/review
- override kernel/module constraints
- bypass visual gates

## Future surface, rigging, animation, and export capabilities

The same user-facing pattern should extend later, but these capabilities are
not active product UI today.

Do not ask novice users:

```text
Do you want UVs?
Do you want a rig?
Do you want animation clips?
```

User asks:

```text
material looks
open/close
walkable
hold weapon
idle/walk/run
```

Internal mapping:

```text
material looks -> Surface capability
open/close -> mechanical rig/motion
walkable -> collision/gameplay metadata
hold weapon -> rig socket
idle/walk/run -> motion set
```

For now, only shape/clay capabilities are active. No broad
surface/material/rig/motion UI is approved. Surface/material looks,
UV/texturing, rigging, and animation remain blocked for the current baseline.

## One Visible Concept Per Gate

Every user-visible concept needs its own pass.

- Box Primitive must work before Lid Seam.
- Lid Seam must work before Trim Band.
- Trim Band must work before Stool Primitive.
- Stool Primitive must work before Family Studio Lite.
- Feet / Skids and crate language remain blocked until after the Stool
  Primitive proof.
- No branch may add multiple visible object concepts without a prior visual
  gate.
- If a visual gate fails, stop and fix; do not add architecture to compensate.

Good:

```text
Add Box Primitive.
Gate it.
Add Lid Seam.
Gate it.
Add Trim Band.
Gate it.
Stop the box ladder.
Add Stool Primitive.
Gate it.
```

Bad:

```text
Add box + lid + trim + feet + panels + handles + Family Studio + materials.
```

If a visible concept fails its screenshot/manual gate, stop and fix it before
adding another concept.

## Current next milestone

The next milestone is not a crate.

It is:

```text
Trim Band Feature Module v0
```

Current proven flow:

```text
Choose Lidded Box
-> Make ready
-> Try lidded box ideas
-> Use this box
-> Adjust Lid Seam, Proportions, or Edge Softness
-> Add to Pack
-> Export Lidded Box
```

Pass criteria:

- user can see the lid seam in pure clay
- object still reads as a box
- no crate language appears
- no parts/focus chips appear
- no material/surface distractions appear
- ideas visibly differ
- buttons and next actions are clear
- export remains truthful

Lid Seam Feature Module v0 and the Lidded Box Make baseline have local evidence
in `docs/LID_SEAM_FEATURE_MODULE_V0.md` and
`docs/LIDDED_BOX_MAKE_BASELINE_GATE.md`. The next branch may add Trim Band as
exactly one visible feature. Do not add Feet / Skids, panels, handles, latches,
materials, crate language, or Family Studio public UI in that gate.
