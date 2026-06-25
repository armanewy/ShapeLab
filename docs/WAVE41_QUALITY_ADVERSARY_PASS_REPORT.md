# Wave 41 Quality Adversary Pass Report

Wave 41 adds an adversarial HQ review gate. It does not add new profiles,
provider packs, LLM generation, materials, UVs, rigging, animation, GPU
rendering, marketplace packaging, arbitrary import, or public IP comparison.

## Implementation Summary

- Added `shape-cli hq-adversarial-review`.
- Added deterministic `adversarial-review.json` schema with visual, mesh, and
  UX questions.
- Marked subjective art judgments as manual-required.
- Recorded missing benchmark dirs and missing `quality-report.json` as blockers
  instead of passing them.
- Added tier-correction rules for Advanced Recipe dependency, missing evidence,
  missing export/reopen proof, Showcase approval, and Draft/Prototype visibility.
- Added tests for schema serialization, missing evidence, subjective manual
  fields, downgrade logic, Showcase approval, novice visibility, determinism,
  and public IP-claim boundaries.

## Final Decision Answers

Does Roman Bridge HQ actually look materially better?

The current workspace does not contain a fresh `roman-bridge-hq`
`quality-report.json`, so Wave 41 does not claim this from local evidence. The
adversarial review records missing evidence and recommends Draft until the HQ
benchmark is regenerated and manually reviewed.

Do showcase gear kits deserve their tier?

No Showcase claim is approved by Wave 41 automation. The reviewed gear benchmark
directories are missing locally, so they are recorded as Draft for this pass.
Even with Usable automation, Showcase still requires human/pro approval and
adversarial visual review.

Does prepared hero template have enough evidence?

No. `prepared-hero-template-v1` is contract-only until it has clay mesh output,
contact sheets, candidate evidence, and export/reopen proof.

Does MOBA Hero Clay MVP look like a real clay asset family or procedural filler?

Automation cannot honestly answer that subjective question. The local
`moba-hero-clay` benchmark evidence recommends Usable with zero automated
blockers, but manual art review must decide whether it looks toy-like,
procedural, or embarrassing next to a private clay-render reference board.

Which profiles are hidden from noobs?

Draft and Prototype profiles must remain hidden. The hidden profiles include
contract-only prepared hero content, the developer-preview MOBA hero clay MVP,
and any missing-evidence profile until review approval is recorded.

Which profiles are Usable?

`moba-hero-clay` has a local automated adversarial recommendation of Usable.
Default novice exposure remains blocked until manual review approval.

Which profiles are Showcase?

None. No profile may be marked Showcase without human/pro approval and
adversarial visual review.

What must be fixed before public demo?

- Regenerate Roman Bridge HQ and Wave 38 gear benchmark evidence.
- Run adversarial review after every generated HQ benchmark.
- Complete manual art review over contact sheets.
- Keep hidden profiles out of the default novice catalog until approval.
- Do not claim materials, UVs, textures, rigging, animation, marketplace output,
  arbitrary mesh editability, or Dota/IP reconstruction.

Did this wave avoid feature creep?

Yes. Wave 41 adds a review gate and docs only. It does not add new content or
new modeling capabilities.

## Verification

Focused verification:

```bash
cargo test -p shape-cli hq_adversarial
cargo run -p shape-cli -- hq-adversarial-review --benchmark-dir target/hq-benchmark/roman-bridge-hq --out target/hq-benchmark/roman-bridge-hq/adversarial-review.json
```

Full verification is recorded in the milestone closeout.
