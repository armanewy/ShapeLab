# Product Claim Gate

Status: Phase A contract, active.

The product claim gate is the repo-wide rule that Object Orchard must describe only
capabilities that have passed their current proof gates. It exists so contracts,
reports, docs, and user summaries cannot quietly imply features that are still
blocked.

## Current Allowed Claims

Allowed product-facing claims:

- Direct primitive editing exists for the currently supported primitive flows.
- ObjectPlan can validate, materialize, and review supported draft plans.
- Supported ObjectPlans can export geometry-only GLB packages.
- Godot import proof exists as a harness and must report `Blocked` when Godot is
  unavailable.
- Human review remains required for drafts and exports.

## Blocked Claims

The following claims are blocked unless a later explicit gate changes the
contract:

- `Godot-ready`: blocked until a passed Godot import proof supports the claim.
- `game-ready`: blocked for all current outputs.
- `rigged`: blocked because rigging is not implemented.
- `animated`: blocked because animation is not implemented.
- `collision-enabled`: blocked because collision output is not implemented.
- `textured`: blocked because texture output is not implemented.
- `UV unwrapped`: blocked because UV support is not approved.
- `terrain-ready`: blocked because terrain output is not implemented.
- `public catalog publishing`: blocked because public publishing is not
  approved.
- `reviewed kit`: blocked unless the text explicitly says human review is still
  required and no public approval is implied.

Negative statements are allowed and encouraged when they clarify scope, such as
"not game-ready", "no rigging included", and "UV editing is not supported".

## Validation Helper

`orchard-foundry` exposes a reusable product claim helper with an includes
contract and a text scanner. It validates report include flags and scans
product-facing copy for blocked terms while allowing truthful negative
statements.

The helper is intentionally conservative. If a report says it includes UVs,
textures, material looks, collision, gameplay metadata, rigging, skinning,
animation, terrain collision, Godot scene output, or blocked game-ready status,
that report fails the gate until the corresponding phase has passed.

## Current Enforcement

The current implementation tests:

- valid geometry-only export report
- invalid `game_ready: true` report
- invalid texture/material-look report
- blocked Godot proof report
- blocked claim copy with uncaveated positive claims
- allowed negative copy that says unsupported features are not included

The gate does not implement a new feature. It only prevents unsupported product
claims from becoming normalized in contracts, reports, docs, or summaries.
