# ObjectPlan Materializer CLI v1

Date: 2026-07-01

## Command

```bash
orchard-cli object-plan materialize \
  --plan object-plan.json \
  --out-dir target/object-plan-materialized/example-plan
```

The command validates the ObjectPlan and writes a review-required materialized
draft for supported primitive/composition plans.

## Outputs

Always written for decoded plans:

- `materialized-object-draft.json`
- `materialization-report.json`
- `materialized-user-summary.md`
- `normalized-object-plan.json`

Written when needed:

- `unresolved-nodes.json`
- `unresolved-attachments.json`

Raw mesh payloads and other unknown fields fail during ObjectPlan decoding and
do not produce materialization outputs.

## Report Contract

`materialization-report.json` includes:

- `status`: `Passed`, `Partial`, or `Failed`
- `primitive_count`
- `materialized_primitive_count`
- `attachment_count`
- `materialized_attachment_count`
- `unresolved_nodes`
- `unresolved_attachments`
- `user_review_required: true`
- `publish_allowed: false`

## Boundaries

Materialization is a draft conversion step only. It is not approval, not public
catalog publishing, not runtime LLM integration, not material/surface work, not
UV/texturing, not rigging, and not animation.

Unsupported pieces are reported honestly. A failed materialization may still
write reports if the plan decoded, but it exits non-zero.
