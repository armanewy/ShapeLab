# Showcase Gear Review

Wave 38 review status:

| Kit | Target tier | Current gate | Showcase status |
| --- | --- | --- | --- |
| Fantasy Sword | Usable | Automated HQ benchmark with export/reopen | Blocked pending human approval |
| Round Shield | Usable | Automated HQ benchmark with export/reopen | Blocked pending human approval |
| Hero Helmet | Usable | Automated HQ benchmark with export/reopen | Blocked pending human approval |
| Pauldron Pair | Usable | Automated HQ benchmark with export/reopen | Blocked pending human approval |
| Chest Armor | Usable | Automated HQ benchmark with export/reopen | Blocked pending human approval |

Review checklist:

- compile succeeds through the Foundry catalog path;
- model validation has no blocking errors;
- six direction candidates survive compile, validation, and non-placeholder
  preview checks where Usable is claimed;
- each primary control produces rendered whole-model difference evidence;
- export/reopen verification succeeds;
- novice-visible kit metadata avoids internal compiler, provider, semantic,
  scalar-path, and operation terminology;
- no kit is labeled Showcase without a human approval marker.

The review manifests intentionally keep `human_approval_marker: false` and
`adversarial_review_marker: false`. That prevents automated evidence from being
mistaken for final Showcase approval.
