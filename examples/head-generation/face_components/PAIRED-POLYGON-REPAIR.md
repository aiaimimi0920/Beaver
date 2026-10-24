# Bounded source sampling and paired polygon triangulation

Status: partial geometry improvement, **not final visual acceptance**.
This change applies to the optional 13-column/detail-budget source profile.
The public 9-column first-pass profile is not silently switched to this profile.

## Shared implementation

- Evaluate native Catmull–Clark level-1 limit-position samples on temporary copies
  of the authored coarse source, in neutral and mouth-open states. Match original
  vertices through checked one-hot correspondence. Retain coarse connectivity;
  do not export the temporary dense reference or decimate an exported mesh.
- Use bounded manual placement as the starting proposal. Shared pose factors,
  protected aperture support, centerline constraints and displacement limits
  remain enforced. The native increment is capped at 1 mm from that proposal;
  this is not a 1 mm total displacement bound from the original raw cage.
- Validate complete source polygons in both poses. A fixed raw triangle winding
  is not a suitable safety reference when the raw quad is already folded.
- At runtime triangulation, synchronize the cleaned BMesh before reading Mesh
  attributes. For supported skin triangles/quads, require valid boundaries,
  nondegenerate interior triangles and a common neutral/posed diagonal. Verify
  the two selected triangles and winding after triangulation.
- Positive triangle-normal coherence is an admissibility condition, not a
  smoothness certificate. Chosen coherence below 0.1 is reported as a severe
  crease warning with both endpoint angles. Do not suppress those warnings.
- Larger polygons retain their existing triangulation path and are explicitly
  reported as lacking automatic polygon-boundary validation. Resulting skin
  triangles still pass hard endpoint checks. Existing deformation gates remain.

The implementation contains no Faye-specific vertex IDs or coordinate patches.

## Observed native evidence, 2026-10-24 UTC

The retained candidate has 4,264 Face triangles. Its GLB SHA-256 is:

`0b93d60d0a947254daa8cb0c7f8b29b8de243576e86ac10264fc5146d405664b`

The source-module split was regenerated as a separate candidate with the exact
same GLB hash. Native generation, separate source packaging and SDF construction
succeeded. Native import of the retained `ear_surface.py` entry and extracted
`source_mesh_rules.py` was verified; this is not an independent full rerun of the
retained pipeline launcher.

Compared with the retained 4,264-triangle baseline, using identical declared
neutral main-skin spatial windows:

- Jaw maximum adjacent-face angle: 160.506 to 94.990 degrees.
- Jaw edges above 45 degrees: 29 to 17.
- Jaw triangles with a minimum corner angle below 5 degrees: 54 to 49.
- The central under-chin subregion no longer has edges above 90 degrees.
- Mouth and forehead percentile metrics are not universal improvements.

The jaw window uses height [1.50, 1.58), absolute lateral coordinate below 0.09 m,
positive forward depth, and the existing main-skin mask. These figures must not
be mixed with earlier, differently defined regional windows.

Original Godot captures cover 36 standard/material/expression/reverse views and
9 low-angle chin views. Matched low-angle baseline captures show fewer fine
under-chin bright streaks, while faceted chin highlights, mouth-corner folds and
long lateral cheek-to-ear creases remain. Likeness is preserved; visual quality
is not fully accepted.

Two severe mouth-open crease warnings remain, approximately 86.53 and 87.77
degrees. Fourteen lateral six-sided polygons were additionally audited through
native final-mesh correspondence and exact GLB position/morph matches. All 56
resulting triangles passed neutral and MouthOpen projected boundary, winding and
area-coverage checks. This candidate-specific audit does not turn the generic
larger-polygon path into a fully validated automatic path, nor establish
continuous-pose or 3D collision freedom.

## Retained source structure and tests

The generated surface module is split into a 339-line surface entry and a
270-line rules module, rather than retaining one 608-line mixed-responsibility
file. Packaging verifies exact source reconstruction and helper bindings while
preserving the existing 64-output generation limit. Dependency guards reject
unsupported cross-module globals and definition-time semantics.

Focused suites passed: semantic eyes 239, source layout 47, face components 122,
facial surface 117 (525 total). Root discovery also passed 153 tests, overlapping
some focused coverage. Tests do not imply zero warnings or product acceptance.

Remaining work: improve mouth/chin and lateral transition geometry, validate
additional generated profiles, and keep visual acceptance separate from source,
CI and endpoint checks. Keep the associated PR in draft until actual acceptance
conditions are met.
