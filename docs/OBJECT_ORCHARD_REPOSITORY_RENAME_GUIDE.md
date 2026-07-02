# Object Orchard Repository Rename Status

Status: GitHub repository rename complete

The product, Rust packages, scripts, project suffixes, metadata fields, and
local output paths now use Object Orchard naming. The GitHub repository host is
now:

```text
https://github.com/armanewy/object-orchard
```

Workspace package metadata uses the same repository URL:

```text
https://github.com/armanewy/object-orchard
```

Local `origin` remotes should point at:

```text
https://github.com/armanewy/object-orchard.git
```

Use this command if a checkout still points at the old host name:

```bash
git remote set-url origin https://github.com/armanewy/object-orchard.git
```

Verify:

```bash
git remote -v
git ls-remote origin
```

In-repo technical cleanup is complete:

- durable schema and fingerprint IDs use Object Orchard namespaces
- generated DCC metadata fields use Object Orchard keys
- project file suffixes use Object Orchard names
- historical migration notes intentionally mention the old repository name

Prompt 10 audit classification:

- Renamed now: environment variables, package metadata, script copy, packaging
  icon paths, local temp-file prefixes, target/evidence path examples, and the
  GitHub repository host.
- Historical migration notes: `docs/OBJECT_ORCHARD_NAMING_TRANSITION.md` and
  completed cleanup reports may mention the old repository name.
- Generated artifact paths: target output is not committed.
- Completed by final purge: durable schema identifiers, fingerprint namespaces,
  project file suffixes, and DCC metadata field keys.
