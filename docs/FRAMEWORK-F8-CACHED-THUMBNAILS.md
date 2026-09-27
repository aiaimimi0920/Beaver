# F8 Frozen catalog thumbnails

Date: 2026-09-26

## Delivered workflow

The production object catalog displays an existing frozen scene preview for
the first scene in the latest version's immutable manifest. Each image is
labelled with its version and cache origin. Browsing the catalog only queries
saved previews; it never starts rendering. Missing and invalid evidence have
explicit placeholders instead of falling back to another version or live scene.

Visible cards read on demand, with at most three concurrent cache queries.
Hidden documents defer new reads. Unmounted cards discard late responses, and
cancelled queued cards do not call the API. There is no background polling.

## Implementation

- `src/ui/object-preview/object-thumbnail-cache.ts` owns frozen target selection,
  bounded reads, cancellation, and reuse of the existing scene preview session.
- `src/ui/object-preview/FrozenCatalogThumbnail.tsx` owns visibility, source
  labels, validated asset display, dimension checks, and explicit placeholders.
- `src/ui/object-preview/ObjectCatalogGrid.tsx` integrates the production cards
  and retains registered thumbnails for objects without a frozen scene target.
- `tests/object-thumbnail-cache.test.ts` covers latest-version identity,
  read-only behavior, evidence rejection, concurrency, and disposal races.

The existing Core lookup validates ownership, evidence hash, and dimensions;
the validation asset route checks the hash again. No new Core or Desktop API
was needed. The UI also checks decoded image dimensions against saved metadata.

## Verification

- `npx tsx --test tests/object-thumbnail-cache.test.ts tests/object-scene-preview.test.ts`:
  7 passed, 0 failed.
- `npm run typecheck`: passed.
- Targeted Prettier check for the four changed files: passed.
- `npm run check:effective-lines`: 1129 sources, 0 violations;
  unchanged legacy debt retained.
- `git diff --check`: passed.
- Chromium smoke using the production catalog and mocked API: displayed the
  real 1280 x 720 PNG, showed missing/corrupt placeholders, made exactly three
  read-only `object.scenePreview.get` calls, and restored the image on reload.
  The named browser session and local fixture server were closed afterward.

Logs: `output/thumbnail-{tests,types,lines,browser,diff}.log`.
Browser proof: [screenshot](../output/playwright/f8-thumbnails/catalog.png),
[fixture](../output/playwright/f8-thumbnails/harness.tsx),
[workflow](../output/playwright/f8-thumbnails/smoke.js).
The PNG comes from the previous [real capture](FRAMEWORK-F8-PREVIEW-RESOLUTION.md).

## Boundaries and remaining work

The thumbnail displays the full immutable PNG; no separate downsampled asset
is generated. Only the first scene of the latest version is selected. If the
latest preview run is pending or failed, an older completed run is not used.
Cache updates appear after catalog refresh or remount, without live polling.
Non-scene registered thumbnails retain their existing live-project behavior.

Browser API responses were mocked. No native host or release acceptance was
run for this UI slice. This closes cached catalog display only; F8.1-F8.5 remain
open for real live session/camera behavior, Blender binding, engine picking,
and topology-aware feedback. F9 and the wider plan audit remain pending.
