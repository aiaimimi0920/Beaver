# F8 saved PNG feedback replay

Date: 2026-09-25

## Delivered workflow

Current publication approval and saved publication operations now show original
feedback, frozen PNG identity, numbered rectangles and regional comments.
Opening the image reads the original feedback attempt output, even when the
publication belongs to a later attempt. Published records use their persisted
preview without requiring a current publication preview.

Reads are lazy and independent per viewer. Collapse and unmount cancel pending
responses and release Blob URLs. Existing attempt-file transport validates the
complete request identity. The overlay appears only after PNG decode and exact
saved-dimension validation. Missing blobs, unsupported or oversized content and
decode/dimension failures show errors without a misleading overlay.
No new Core mutation, schema or Desktop endpoint was needed.

## Verification

- Focused publication and image tests: 8 passed, 0 failed.
- TypeScript typecheck and changed-file Prettier checks passed.
- Effective-line check: 1104 sources, 17 unchanged legacy files, 0 violations.
- Independent read-only review found no concrete correctness defects.
- Chromium with production React components and mocked API passed current approval
  and published-history replay, lazy reads, original-attempt identity, rectangle
  placement, numbering, collapse/reopen, dimension mismatch and missing-blob
  handling. Recorded calls contain reads only.
- Screenshot inspection prompted a larger image-space number label; browser,
  type, formatter and structural checks passed again after that adjustment.

Logs: `output/f8-feedback-replay-*.log`.
Browser fixture and replay script: `output/playwright/f8-feedback-replay/`.
Screenshot: `output/playwright/f8-feedback-replay/replay.png`.
The first smoke assertion used HTMLElement innerText on SVG text; changing it to
textContent fixed the harness. The successful browser log is
`output/f8-feedback-replay-browser-proof.log`.

## Remaining scope

This closes saved PNG region replay within publication review. Native WebView,
real model execution, engine rendering/picking, camera provenance, contextual
thumbnail navigation and later-medium feedback routing remain unverified or open.
F8.1-F8.5 stay unchecked. No full native acceptance, release, version bump,
commit or push was performed.
