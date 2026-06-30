# Object Intent Brief Contract

`ObjectIntentBrief` captures user intent for turning a plain-language request into a safe Object Orchard path.

It is not mesh generation, and it is not an LLM prompt format.

## User-facing purpose

The user may say things like:

- “I want a simple box family.”
- “I want a door kit.”
- “I want a stove family.”
- “I want a simple car family.”

Object Orchard must respond with one of three outcomes:

- **Ready**: an existing proven starting point can be used today.
- **Draftable**: a new draft can be proposed, but it is not product-approved.
- **Blocked**: the request currently depends on unsupported complexity.

## Internal mapping

The object intent classifier maps user language to internal readiness reports:

| Request kind | Current result |
| --- | --- |
| box / cube / block | Ready: Box Primitive |
| panel / slab | Ready: Flat Panel Primitive |
| door / gate / hatch | Draftable from Flat Panel, but not Door-approved |
| stool / table / chair | Draftable standing-object proof |
| stove / appliance | Draftable appliance proof |
| car / vehicle | Blocked until vehicle support exists |

This is deliberately conservative.

## Forbidden behavior

The classifier must not:

- generate meshes;
- call an LLM;
- publish catalog profiles;
- expose kernel/module/provider/slot terminology to users;
- imply Door, vehicle, UV/texturing, rigging/animation, or game-ready support.

## Relationship to LLMs

LLMs may later draft an `ObjectIntentBrief` or propose repairs from a readiness report.

The deterministic classifier still decides whether the request is ready, draftable, or blocked; LLMs do not decide readiness.
