# Layered lash verification, 2026-10-18

Candidate 787 retains the previously reviewed face, aperture, iris and eyebrow
controls. `layered_lashes=1` restores the missing lateral/companion/lower groups.
The comparison used user-provided Silver Wolf screenshots; those reference
assets are not distributed here. Different screenshot views/scales support a
visible-group inventory, not exact geometry counts or proportion measurements.

## Native evidence

- Fresh generation through the Beaver executor and SDF from its success receipt.
- Runtime blend SHA-256: `c35dc6bd25b15e5f4c71add937319646396aadb8414de0c5ddbaea112a4d9c2f`.
- 14 optional retained lash components across both eyes. The visible design
  comprises a small upper tip, two outer tips, lateral/lower tips and companion.
- Six neutral lateral-root vertices per side checked against the wing: worst
  nearest-surface distance 0.000404591 m, below the unchanged 0.0008 m limit.
- 12,416 actual triangles: analytic blink/MouthOpen orientation minimum
  0.0591597395 over [0,1]. This is not a triangle-intersection proof.
- 84 native render/white/edge captures, source hash unchanged, each image hash
  and requested blend-shape weights verified. Manifest run:
  `blink-1792315472-50703`.
- Additional bilateral-blink proximity sampling at 0/.25/.5/.75/1: worst
  0.000157092 m. Samples are selected near each child's neutral closest point;
  this is not an authored all-root constraint or continuous collision proof.

## Local verification

Semantic tests (131), facial-surface tests (35), Black and diff whitespace checks
passed before retention. Native snapshots remain necessary for visual judgment.
The local root fit is approximate: it translates the six-point section rigidly,
refining XY about the initial centroid transport while keeping that transport's
Z. All original final connection, obstacle and orientation rejection gates stay.

## Outcome and remaining scope

Missing visible groups are restored in frontal and oblique views. The upper
leaf is narrowed about its lower attachment rail and its free tip rises; denser
curve stations avoid a coarse six-station outline. Earlier failed candidates
were rejected, not treated as accepted output. This is a face-only candidate
ready for owner review, not a claim of universal aesthetic acceptance or full
character/product completion. No hair/body generation or production release.

## Follow-up visual hierarchy: candidate 788

The owner liked 787 but requested another reference-based quantity pass. The
reference separates one dominant side leaf, a subtle side-band ending and an
independent lower-edge tuft. It does not justify adding equally prominent teeth.
Candidate 788 reduces the lower lateral child reach to 45%, moves the lower tuft
station from .56 to .40 without enlarging it, and uses secondary_lash_scale=.70.
Component inventory is unchanged; visible emphasis and spacing are the changes.

Native generation and receipt-bound SDF passed. All 84 source/image/morph capture
checks passed in `blink-1792316511-71974`; bilateral root proximity sampling
remained below 0.000157092 m. The same sampling limitations above apply.
Runtime blend SHA-256: `143d27be6e4812d2ab68955b9e73039968800a89c1215a0722c65c82d0ca8701`.
Semantic regression tests: 132 passed. Frontal and oblique native views retain
the face and eye shape while reducing the row-of-teeth appearance. Owner final
aesthetic acceptance remains separate from these checks.

## Intermediate upper layer: candidate 791

The owner requested more upper-lash detail after the 788 side/lower refinement.
Reference comparison identified a thin intermediate outward-sweeping layer
between the small upright tuft and outer fork. This is an interpretation of
visible layering, not a claim about the reference mesh's exact component count.
`upper_lash_layer=1` adds one independent sheet per eye (16 optional retained
components across both eyes, versus 14). Candidate 789 failed root attachment;
790 passed technical checks but its new silhouette looked like a rounded bump.
791 raises the sweep progressively toward a thin free tip, retaining 788's
lateral/lower settings and all aperture/face controls.

Native generation and receipt-bound SDF passed. The 16 closest-parent-band pairs
remained within 0.000157092 m over bilateral blink samples 0/.25/.5/.75/1.
This includes the new sheet, but does not prove continuous collision freedom or
all authored root vertices. Continuous triangle-orientation minimum remained
0.059159739495675545 over 12512 triangles. All 84 image/source/morph checks passed
in `blink-1792318472-96212`. Frontal, oblique, white-model and closed-eye views
were inspected; owner aesthetic approval remains pending.

Runtime blend SHA-256:
`1e60232da375a9816c3854e9c1911ee49d600eb7737edb3c1627dabf0280a085`.
135 semantic and 35 facial-surface CPU tests passed. New tests execute the
packaged seating code on both sides, including quarter/mid-span weights,
stationary free tips/parent geometry and refusal of missing or >=4 mm targets.
No reference assets are included in the repository.

## Reference-led sweep refinement: candidate 793

Six-game screenshot comparison motivated reducing the intermediate leaf's broad
root rather than adding components. Candidate 792 used a narrow 17-station
lens, but native views made its horizontal extension too prominent. 793 shortens
the sweep from .48 to .40 station units, limits its width to .008 calibrated H
units, and lowers the tip lift to .021 H. The root starts at .36. Smooth sampled
boundaries replace the old six-control polygon. All other controls and the
16-component retained inventory remain unchanged.

136 semantic and 35 facial-surface tests passed. A new test executes the actual
native outline_sections function, checking 17 monotone sections, closed end
points and positive interior widths in both illustrated modes. Native 793
creation, receipt-bound SDF, and 16-pair sampled blink proximity passed; maximum
sampled distance was 0.000157092 m. The same root-band sampling limitations apply.
All 84 captures in `blink-1792321255-1166` passed source/image/morph verification.
The continuous orientation check covers 12776 triangles, minimum ratio
0.059159739495675545; it remains an orientation check, not an intersection proof.
Runtime blend SHA-256:
`a6693d608d0a07230ca8ac9a0395ecbd354c329b2b56dd6c860fe1cd2d5f96f7`.

Frontal/oblique and closed-eye renders reduce the previous upper-root black
block; 793 is more compact than the 792 intermediate. This is an incremental
candidate for owner review, not final aesthetic acceptance. Reference images
remain outside the repository. No face aperture, lower lash or side leaf edits.
