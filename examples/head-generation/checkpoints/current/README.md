# Posterior opening arc checkpoint 503

Source-only recovery checkpoint, 2026-10-12. Responds to the owner's newly annotated posterior ear-to-chin opening contour. Partial geometric correction; final aesthetic acceptance remains pending. Eyelashes and other separate components remain unchanged.

The previous lower opening narrowed toward the midline too early. A compact lateral outward correction delays this narrowing and rounds the lower arc, instead of applying only posterior depth changes. It acts on the connected free skin boundary at normalized height .04 to .25, peaking near .11, with a smooth6mm maximum envelope. Displacement extends harmonically into adjacent permitted skin and is fixed to zero elsewhere. Shared ear-root topology is retained. This retains501 geometry and texture elsewhere; rejected502 texture changes are not included.

## Reproduction
Requires the owner's existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Package using python3 bundle.py --output request.json, then load in native Beaver blender.start and validate/preview through native NPR. Do not execute pipeline.py directly. No reference mesh or texture is included.

## Verified evidence
279 pure tests passed in21.207s in the complete working source. Bundled focused tests are only a subset. Native job aad7abc5-5d54-478f-8d79-4e62d3a0971c completed2026-10-12T12:24:17.497Z, exit0,51406ms,31 hashes verified. NPR12:25:32.576Z ok:true,errors:[]. Actual GLB501 to503:106 Face vertices changed, maximum5.558394mm,5696 unique vertices unchanged. Outside the bounded posterior lower-skin region exact position difference zero; body/hair unchanged. Both ear roots retain14 manifold shared edges each, incidence2.
Inspected rear render,180/150white and front render. The lower opening is less prematurely diagonal; no obvious new white-surface fold. Reference equivalence, final appearance acceptance, merge and deployment are not established. Tests do not imply zero warnings.
