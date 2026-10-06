# Candidate 85 observed checkpoint

On 2026-10-06, Beaver job 01ea07c6-97e9-4f7d-9ee5-d61ee3e233e3 (request 2fde41dd-6859-417c-921d-417f841b9eff) completed with exit code 0. GLB SHA-256: 928c72de981d513b3767f3d3c4933bc9a18752c5509281038539690da9083eea.

## Retained changes

- Candidate 83 broad planar ocular rear caps retain geometric flat corner normals. Simplified pinnae use 144 vertices and 120 quads each, with original procedural ear pigment.
- Candidate 84 introduced real crease_edge center and side-face profiles. Candidate 85 reports 21 center edges, 13 side edges and 47 existing aperture edges. A matched evaluated-mesh ablation changes 369 of 3096 vertices, with maximum displacement 2.742 mm. This establishes that the attribute affects the evaluated mesh; it does not establish likeness.
- Candidate 85 separates nasal basal flare from tip/bridge lateral falloff, widens lower nasal support and narrows upper bridge sides using explicit semantic parameters.
- Twenty sparse nasal-section ray measurements improve RMS depth mismatch from 2.256 mm in candidate 84 to 0.769 mm in candidate 85. These sampled measurements are not a whole-surface comparison or aesthetic acceptance.

## Verification

Nineteen local recipe tests pass. Raw facial shell reports finite vertices, no zero-area faces and no reversed checked front/chin panels. Body/Hair primitives and the original baseline GLB binary prefix remain unchanged. Optional Draco and unused active vertex-color warnings remain; this is not a zero-warning claim.

Native MiDot comparison inspected front render, white left 45 degrees, both white side views, and render lighting at approximately -95 and +94 degrees. Nasal side contour remains continuous and lower nasal support is less pinched. Face/side boundary is visible in white view. The lip profile retains a slightly forward upper lip. MouthOpen 0, 0.5 and 1 shows a narrow rest seam and rounded dark oral opening with visible tongue; the previous triangular opening regression was not observed. Neutral mouth is restored after testing.

## Still open

The eyes and upper eyelid shape, oral rear volume, ear attachment outline, facial topology density and overall resemblance still need refinement. NPR facial shadow shape does not yet match the calibration reference. No final visual acceptance, universal-template acceptance, full wardrobe acceptance, deployment or release is claimed. The unpacked reference has split attributes and possible export artifacts; original Blender crease weights are unavailable, so these authored weights are not claimed as recovered source data.
