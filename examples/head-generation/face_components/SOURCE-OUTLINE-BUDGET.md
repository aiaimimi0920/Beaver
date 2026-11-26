# Optional source detail and outline budget

Status: technically verified development controls, not visual acceptance.
The shared game-face defaults remain unchanged. These controls are opt-in and
operate before export; they do not decimate an exported mesh or omit Face parts
from the triangle count.

## Controls

- `sampling_detail_budget=1` reduces tongue grid and intermediate lash sampling.
  Lash sampling retains parent-rail breakpoints and merges numerically coincident
  grid stations. All legal odd parent counts from 9 through 21 have nondegenerate
  actual ribbon-function regression coverage.
- `sampling_brow_divisions=12` is now an available research-brow sample budget.
- `sampling_outline_refinement=1` requires the shared density layout and
  `sampling_surface_subdivision=0`. It adds a local source strip after cage
  fairing, before attachment fitting. It is not a general subdivision modifier.
- `sampling_surface_fairing=1` remains optional. A reflected boundary neighbor
  keeps the crown's center seam continuous without moving mirror-plane support.

The outline stage trials actual triangulated patches, including adjoining faces.
It preserves existing support coordinates, keeps front winding, and rejects
patches below its angle and orientation gates. Unsafe candidate segments are
skipped and reported, rather than reducing quality thresholds. Sorting input
edges and collecting all failed source faces makes selection deterministic.
Selection is conservative, not a maximum-refinement optimizer.

Blender's subdivision operator temporarily changes vertex CustomData and
invalidates Python vertex wrappers. The stage reacquires each original point
uniquely from its coordinate and restores its exact value; it does not mistake a
stale wrapper for physical vertex displacement. The report records any restored
roundoff and the actual selected curve displacement.

## Final checks and their scope

All original source faces must retain identity and normal-region coverage.
Every resulting source triangle is checked for finite coordinates and nonzero
area. Outline-affected faces additionally retain their source orientation bound.
Unmodified side faces can intentionally rotate during subsequent ear/jaw fitting;
their before/after normal dots are reported as diagnostics, not falsely tested as
outline changes. Newly generated posterior returns have distinct provenance and
are independently checked for backward orientation.

These checks are not a proof of self-intersection freedom, global smoothness,
continuous speech quality, or final appearance. The existing ear and blink
checks remain active; no diagnostic-failure mode was used for retained results.

## Actual development evidence, 2026-10-24

- Earlier full Face baseline: 8,056 triangles; comparison SilverWolf face: 4,210.
- Detail-only trial: 3,836 triangles, saving 328 against the 4,164 strict baseline.
- Retained optional-outline candidate: **3,878 complete Face triangles**.
  This includes brows, lashes, eyes, tongue, and mouth cavity.
- Two strict generations produced identical GLB bytes:
  `8a949c673771b99a2cff29ad11afc98ca9da19c41a25be0ed7af6d3673131102`.
- The retained candidate added 14 source points and skipped 13 unsafe candidate
  edges. Old support coordinates were unchanged. The affected source patch's
  minimum angle was 12.270 degrees.
- Source packaging and model-bound SDF calibration passed. Thirty-six actual
  NPR, white-model, wireframe and expression screenshots were captured with
  matching model, texture and renderer bindings before and after capture.
- A compact-rounded face/line-mouth combination also generated 3,878 triangles.
  A blunt-chin/elongated-nose/lip-band combination generated 3,878 after correcting
  the final check's distinction between outline changes and later intended fit.
  These combinations have technical generation evidence, not separate visual
  acceptance or universal library coverage.

## Remaining visual work

The severe added jaw fold in the discarded early-outline trial was removed:
its approximately 149-degree local bend returned to the earlier approximately
96-degree baseline. This is removal of a regression, **not** proof that the
baseline jaw is smooth. White-model forehead, cheek and jaw creases remain;
ear silhouettes are still angular. Small thin triangles and minor outline
speckling also remain. Keep this as a low-budget development baseline while
improving shared source geometry. Do not mark product acceptance complete or
merge solely because CI is green.
