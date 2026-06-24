# HQ Contact Sheet Gate

Usable and Showcase content must have contact-sheet evidence. A contact sheet
is the review artifact that lets a human compare the whole model across views
or candidate directions without relying on product screenshots.

The Wave 32 benchmark writes:

- `contact-sheet.png`
- `front.png`
- `three-quarter.png`
- `side.png`
- `back.png`
- `wireframe.png`
- `silhouette.png`
- `quality-report.json`

The contact sheet is product-truth only when it is a clay/matcap/studio-style
mesh preview from Shape Lab output. Photoreal renders, painted concept art,
reference boards, and environment beauty shots are inspiration only and cannot
replace the contact-sheet gate.

Reviewers should reject or downgrade a kit when:

- the silhouette is unreadable
- the model only looks good from one angle
- direction candidates collapse into the same shape
- a primary control has no visible whole-model effect
- control evidence is based only on recipe or metadata changes instead of
  rendered whole-model pixel deltas
- export/reopen proof is missing for a Usable or Showcase claim
