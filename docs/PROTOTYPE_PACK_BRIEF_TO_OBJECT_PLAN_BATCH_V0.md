# Prototype Pack Brief-to-ObjectPlan Batch v0

Status: offline CLI only. This is not full Prototype Pack Mode.

This milestone turns a small Prototype Pack brief into Draft ObjectPlan files
when each request maps to supported primitives or the supported Panel with Knob
composition.

## CLI

```bash
shape-cli prototype-pack plan \
  --brief fixtures/prototype-pack/simple-room-primitives-v0.json \
  --out-dir target/prototype-pack-brief-v0/simple-room-primitives
```

Output:

- `object-plans/`
- `object-plan-batch.json`
- `prototype-pack-plan-report.json`
- `user-summary.md`

## Scope

Supported request mappings:

- Box Primitive
- Flat Panel Primitive
- Sphere Primitive
- Panel with Knob

Unsupported requests are reported as Blocked. The planner does not invent
unsupported assets.

## Product Boundary

Generated outputs are Draft ObjectPlans only. No runtime LLM integration,
automatic approval, public catalog publishing, material editor, UV editor,
rigging, animation, or game-ready claim is included.
