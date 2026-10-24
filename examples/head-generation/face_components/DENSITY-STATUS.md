> Update 2026-10-24: see [GAME-FACE-BUDGET.md](GAME-FACE-BUDGET.md) for the current 4k source recipe and strict native evidence. The 8,056-triangle specimen below is historical, not the current reduction target.

# Shared source-density construction: development checkpoint

Status: partial, not final model acceptance (2026-10-24 UTC).

## Construction policy

The public composition route enables first-pass density layout v6 and nine
semantic shell columns. Explicit sampling overrides remain bounded and can
restore the legacy layout. No character identity, exported vertex index or
post-export decimation is used to select reductions.

- Keep all 22 calibrated height rows and the outer two chin control intervals.
- Coarsen three interior chin strips with conforming coarse-to-fine transitions.
- Coarsen only the cheek portion of the subnasal strip; keep nasal and ear-root
  row samples. Do not remove a complete horizontal row.
- Reduce the mouth patch from three construction rings to two, retaining the
  original inner support ring at blend .30. This changes
  subdivision support, so the lip/profile visual gate remains necessary.
- Keep evaluated eye/mouth, winding, clearance and ear quality checks active.
- Keep source-generation outputs within the native 64-output contract. Pure
  geometric sampling functions live with face geometry; curve/sampling helpers
  and ordered pipeline stages retain source-reconstruction hash verification.

The nine-column default does not automatically enable every optional reduction
in iris, lash or brow sampling. Do not extrapolate one specimen's triangle count
as a budget guarantee for every composition.

## Verified specimen, not a universal acceptance result

A controlled Faye comparison used identical explicit sampling controls except
for the density layout. Baseline: 8,620 triangles. Candidate21: 8,056 triangles,
GLB SHA-256 `58aa63c77394269b01ee7b455589e365f1c72139526acb4e6e4c123eeb03b89a`.
The same earlier fixed-meter diagnostic windows measured mouth 699→513 and chin
688→490 triangles; chin triangles below 5° decreased from 43 to 8. These windows
overlap and are not the v2 normalized diagnostic windows.

Candidate21 completed native generation, source packaging, SDF generation and
36 matched-camera NPR/white/wireframe screenshots. The lower ear retained 15
root samples per side; maximum corresponding root-position change was 8.3 μm.
All 36 camera/pose records matched baseline27 plus rear9. Review found no obvious
new ear silhouette regression or gross tearing in the tested discrete poses.
However, lip/philtrum/profile shading changed, and nose-level and chin/ear
creases remain. Continuous animation and speech-aligned motion were not tested.

The 7,836-triangle candidate18 was rejected for ear-region contour shift. Its
lower count must not be substituted for the retained candidate's result.

## Retained refinement and source checks

Candidate23 reproduced candidate21's exact GLB SHA after extracting source
modules, establishing geometry equivalence for that explicit specimen.
Candidate29 uses layout v6: the mouth support ring remains at original blend
.30 rather than v5's .50. It generated 8,056 triangles with GLB SHA-256
`5987a5d696a4eb028061ca86a34826eca8ff0d31e483a94cdb11a6e382389a9a`, completed
native source packaging/SDF and 36 matched captures. The fixed-meter chin
window counted 470 triangles (window membership changes with geometry).

Review preferred29 over21 for the philtrum/upper-lip transition; profile
projection approached baseline but was not identical. Ear outlines appeared
preserved. Nose-level transverse shading, chin patch and lower-ear crease still
remain; this is not final visual acceptance. No new full-motion claim is made.

The explicit opt-out now restores legacy angled-brow sampling/tessellation;
first-pass angled brows select32 divisions only when no explicit count exists.
Lash exact-station mode is also opt-in. The angular ink source edit is scoped to
the actual curve function, avoiding an existing conflict with a nasal curve copy.

Focused local tests passed: semantic eyes196, component contracts122, brows11,
source layout24, ear contracts21, accent extraction/volume12 and facial
outline/chin10. Generated-source size checks cover almond and segmented eye
compositions, including packaged helpers, at a conservative500-line bound.
`npm run check:effective-lines` could not run locally because Cargo is absent;
its formal repository check remains pending in CI. Do not interpret these CPU
contracts as a successful native render for every composition.

## Open cross-composition failure

A public compact-rounded face + line mouth + balanced nose composition generated
successfully without sampling overrides. A blunt-chin + lip-band mouth + elongated
nose composition failed the required lower-ear improving-diagonal check. The
same selection also failed with density disabled and 13 shell columns. This
control did not restore every historical brow/lash sampling choice; it proves
only that the density-layout change is not necessary for the failure. A bounded
one-ring edge-search experiment did not fix it and was not retained. No quality
threshold was relaxed. This is still an unresolved construction/quality issue,
not a successful cross-composition acceptance.

## Reusable diagnostics

`density_review.py` compares actual 3D triangle areas, angles and adjacency using
a common baseline normalization frame. Its v2 coarse windows differ from the
older fixed-meter numbers above. It rejects unsupported transforms/mesh layouts
rather than silently interpreting them. These measurements are diagnostics, not
visual, manifold, collision, animation or performance acceptance.
