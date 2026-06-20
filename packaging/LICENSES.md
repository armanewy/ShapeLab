# License Notes

The Rust workspace declares:

```toml
license = "MIT OR Apache-2.0"
```

This means Shape Lab source crates are intended to be usable under either the MIT License or the Apache License, Version 2.0. Reference texts are included in `packaging/licenses/MIT.txt` and `packaging/licenses/APACHE-2.0.txt` for manual release archives.

## Packaging Checklist

- Include this file in release archives.
- Include `packaging/licenses/MIT.txt` and `packaging/licenses/APACHE-2.0.txt`.
- Include `packaging/THIRD_PARTY.md`.
- Keep generated demo assets separate from source licensing notes unless they become part of a formal release package.
- Do not place private signing certificates, tokens, or store credentials in the repository or release archive.

## Dependency Licenses

Rust third-party crate licenses are tracked by each package's published metadata and the resolved `Cargo.lock`. See `packaging/THIRD_PARTY.md` for the inventory workflow and the direct dependency summary.
