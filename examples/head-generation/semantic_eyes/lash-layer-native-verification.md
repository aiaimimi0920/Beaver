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
