# Control Endpoint Visibility Gate

Primary controls should be checked by rendering endpoint samples and comparing
their visible screen-space delta. This catches controls that technically change
values but do not read in a product preview.

Endpoint visibility classes are:

- Strong
- Clear
- SubtleButExplainable
- TooSubtle
- Unsupported

TooSubtle controls emit warnings and are de-prioritized for direction
generation, but they are not removed from customization. Unsupported means the
endpoint previews could not be compared honestly.

The gate ignores transparent background pixels, clamps finite scores to `0..1`,
and records plain-language warnings instead of internal diagnostics.
