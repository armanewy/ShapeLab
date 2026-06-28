# Evidence Manifest Policy

Evidence used for product-status claims must be reproducible and attributable.
Screenshot existence, hash differences, and generated reports are not enough by
themselves to claim product stability.

## Manifest Location

Generated local manifests should be written beside the evidence they describe,
for example:

```text
target/product-recovery-integration-v2/evidence-manifest.json
```

Do not commit large videos or screenshots unless a repo convention explicitly
allows it. Reports may reference generated artifacts under `target/`, but they
must include enough metadata for another reviewer to audit what was captured.

## Required Fields

Each manifest must include:

- `commit_hash`
- `app_binary_path`
- `os`
- `screen_size`
- `screenshot_filenames`
- `screenshot_hashes`
- `video_filename`
- `video_hash`
- `benchmark_root_path`
- `benchmark_summary_hash`
- `human_reviewer`
- `verdict`
- `timestamp`
- `notes`

## Verdict Rules

- `pass` means the named gate passed, not that the whole product is stable.
- `no_go` means human review found blockers.
- `blocked` means capture, execution, or review could not be completed.
- A product-stability claim requires both automated gates and a clean human
  dogfood pass.

## Hashing

Use SHA-256:

```bash
shasum -a 256 target/product-recovery-integration-v2/product-recovery-dogfood-video.mov
shasum -a 256 target/product-recovery-integration-v2/screenshots/*.png
shasum -a 256 target/starter-template-dogfood/dogfood-summary.json
```

## Reviewer Identity

Use a local identity such as a Git author, OS username, or reviewer initials.
The identity records who made the judgment; it is not a signature or approval
system.
