# Object Orchard Repository Rename Guide

Status: manual GitHub repository setting required

The product, Rust packages, scripts, and local output paths now use Object
Orchard naming. The GitHub repository host name may still need a manual rename.

Manual steps:

1. Go to repository Settings.
2. Rename repository from ShapeLab to ObjectOrchard or object-orchard.
3. Update local remotes:
   `git remote set-url origin git@github.com:armanewy/ObjectOrchard.git`
   or HTTPS equivalent.
4. Verify:
   `git remote -v`
   `git ls-remote origin`

Until the GitHub setting is changed, the existing remote URL can continue to
work through GitHub redirects after rename. Do not claim the repository host
rename is complete until `git ls-remote origin` succeeds against the new URL.

Remaining technical cleanup belongs to the final legacy-name purge:

- durable schema and fingerprint IDs that still use legacy namespaces
- generated DCC metadata fields that still use legacy keys
- historical migration notes that intentionally mention the old repository name

Prompt 10 audit classification:

- Renamed now: environment variables, package metadata, script copy, packaging
  icon paths, local temp-file prefixes, and target/evidence path examples.
- Historical migration notes: this guide and
  `docs/OBJECT_ORCHARD_NAMING_TRANSITION.md`.
- Generated artifact paths: target output is not committed.
- External repository URL: pending the manual GitHub Settings rename above.
- Deferred by contract: durable schema identifiers, fingerprint namespaces, and
  DCC metadata field keys.
