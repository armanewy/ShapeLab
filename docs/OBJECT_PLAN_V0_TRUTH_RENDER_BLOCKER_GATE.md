# ObjectPlan v0 Truth / Render Blocker Gate

Date: 2026-07-01

## Verdict

ObjectPlan v0 is an offline validation and internal review milestone. It is not
yet a broadly renderable asset-generation milestone.

## What Works

- Structured ObjectPlan JSON can describe supported primitive nodes.
- Structured ObjectPlan JSON can describe the supported panel-plus-sphere
  safe-anchor composition.
- The validator rejects unknown primitives, unknown properties, out-of-domain
  values, incompatible anchors, raw mesh payloads, arbitrary transform payloads,
  absolute paths, validation bypass attempts, and public catalog publishing
  requests.
- The CLI can validate one plan, normalize one plan, write product-safe
  summaries, and run batches.
- The CLI can emit honest renderability and visual-evidence reports.
- Batch reports keep `human_review_required: true` and `approved: false`.
- The ObjectPlan review drawer is internal-only and hidden unless
  `OBJECT_ORCHARD_OBJECT_PLAN_REVIEW` is enabled.

## What Validates

- Box Primitive plans validate.
- Flat Panel Primitive plans validate.
- Sphere Primitive plans validate.
- Panel with Knob-style plans validate when represented as a Flat Panel
  Primitive plus a Sphere Primitive attached through the approved
  `front_handle_zone` to `back_mount_point` anchor path.

## What Renders

ObjectPlan v0 does not yet materialize supported plans into renderable preview
geometry. Current ObjectPlan CLI evidence is validation and review evidence,
not a guarantee of generated asset geometry.

## Honest Render-Blocked Output

Supported plans currently emit render-blocked reports when render evidence is
requested. This is valid for ObjectPlan v0, but it is incomplete relative to the
next milestone.

The expected blocked outputs include:

- `renderability-report.json` with `renderable: false`
- `rendering-report.json` with blocked status
- `visual-evidence-report.json` with `rendered: false`,
  `user_review_required: true`, and `approved: false`
- no placeholder `contact-sheet.png`

## Validation-Only CLI Outputs

The following outputs prove validation and review preparation, not final asset
materialization:

- `validation-report.json`
- `primitive-summary.json`
- `normalized-object-plan.json`
- `plan-user-summary.md`
- `batch-validation-report.json`
- `batch-user-summary.md`
- `keep-regenerate-simplify.md`

## Internal-Only UI

The ObjectPlan review drawer is a dev-gated internal review surface. It may
show batch report targets, contact-sheet placeholders, and review labels, but
it does not publish, approve, or make ObjectPlan authoring public.

## Overclaim Audit

Current status docs must not claim that ObjectPlan v0 broadly generates visible
assets, renders every supported plan, approves plans automatically, publishes
to a public catalog, runs LLMs in the app, or supports material/surface,
UV/texturing, rigging, animation, or game-ready output.

## Next Work

The next milestone is ObjectPlan materialization and render evidence:

- convert valid ObjectPlans into materialized draft primitive/composition
  structures
- produce real preview/contact-sheet evidence for supported materialized plans
- keep unsupported nodes and attachments in honest blocked reports
- keep every output Draft and review-required
