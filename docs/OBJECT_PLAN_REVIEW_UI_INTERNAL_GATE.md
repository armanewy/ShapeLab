# ObjectPlan Review UI Internal Gate

Date: 2026-07-01

The ObjectPlan review UI is an internal-only review surface. It is hidden in
the default novice UI and appears only when `OBJECT_ORCHARD_OBJECT_PLAN_REVIEW` is
enabled.

## Scope

The internal drawer shows:

- fixed batch report target
- contact-sheet review area
- Keep, Regenerate, Simplify, and Blocked labels
- Draft only
- Not catalog published
- Human review required

The labels are review labels only. They do not publish, approve, or mutate the
catalog.

## Non-Goals

This gate does not add:

- plan authoring UI
- runtime LLM controls
- public catalog publishing
- material, surface, rigging, animation, or game-ready workflow

## Manual Evidence

Manual screenshots captured from the rebuilt macOS app bundle:

- `target/object-plan-review-ui/default-hidden.png`
  - review entry hidden in default UI
- `target/object-plan-review-ui/dev-entry-visible.png`
  - review entry visible when `OBJECT_ORCHARD_OBJECT_PLAN_REVIEW=1`
- `target/object-plan-review-ui/review-drawer.png`
  - batch report target visible
  - contact-sheet review area visible
  - Keep, Regenerate, Simplify, and Blocked labels visible
  - no publish button or catalog action

The drawer screenshot was taken after the app's screenshot scenario assertion
reported `ObjectPlanReviewDrawer: PASS`.

Computer Use validation note: `list_apps` confirmed the rebuilt Object Orchard app
was frontmost. The accessibility state
read timed out, so the manual gate used screenshots plus the app's internal
screenshot-state assertion instead of an accessibility tree.
