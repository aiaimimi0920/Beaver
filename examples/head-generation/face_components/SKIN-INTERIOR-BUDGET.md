# Optional skin/interior allocation and medial normals

This increment does not change the default nine-column, detail-budget-off profile.
It does not mark facial topology or visual acceptance complete.

With detail budgeting enabled, nine-column skin keeps a 12 x 7 tongue grid.
Thirteen-column skin uses 12 x 5 instead, retaining cardinal angular samples,
closed topology and all six coordinate extrema. This removes 48 hidden tongue
triangles relative to the 12 x 7 allocation. Candidate 85 contained 4,204 actual
Face triangles, compared with 4,252 before the interior reallocation. These are
whole Face triangle counts, not exported vertex counts or authoring polygon counts.

For this optional thirteen-column/detail-budget profile, both normal accumulation
and lookup no longer split continuous skin regions 1/2 at the mirror centre.
Other regions and nine-column/default profiles retain the previous behaviour.
An omitted sampling_shell_columns means the existing legacy default of 13,
not None. Setting side=0 also enables the existing skin boundary-normal blend;
this is a medial continuity adjustment, not just removal of duplicate groups.

## Executed evidence

- Candidate 85: strict generation, packaging, SDF and 36 actual Godot captures.
  Its mouth-side shading remained a visual regression relative to candidate 78.
- Candidates 86/87 experimentally moved the middle oral ring and then used linear
  radial depth interpolation. They improved geometric angle metrics but introduced
  visible radial ripples or a central nose/lip wedge and flatter lip relief.
  Those geometry experiments are NOT included in this increment.
- Candidate 89 isolated the normal change on candidate 87 geometry: all 4,204
  oriented triangles, three UV layers, two colours and six morph POSITION streams
  matched exactly. 122 corner normal/tangent records changed. The inspected nose
  and chin centre edges became normal-continuous; source geometry did not change.
  Its 36 captures showed softer centre shading, but the inherited oral shape issue
  remained. This is not evidence that the mouth shape was fixed.
- Candidate 89 GLB SHA-256:
  68be22e31ae53184567c48aa55e699a1a81e66b91869b069eb26a7aa8f627b72
- Candidate 85 GLB SHA-256:
  c999947bb5ef8922de8422a83e2ba97307561b37721ca27980575aa2e6294601

The source here retains the original oral ring coordinates and depth formula.
Candidate 90 checks the normal change on that original oral shape: 4,204
triangles; all geometry, morph positions, UVs and colours match 85. Its native
generation completion journal, packaging, SDF and 36 real captures are verified.
The centre seam is softer without the rejected 87 lip flattening, but the outer
mouth-side sharp shading and forehead/cheek artifacts remain. It is a partial
improvement for continued work, not full visual acceptance. Its GLB SHA-256 is
639cf910ef8db738a5bed5a3f3a749377fc36678eae44290cefd7d4b00efec74.

The two CPU test modules exercise the actual generated tongue geometry and both
actual generated normal-grouping assignments. They cover omitted/explicit 13,
legacy/default/9-column, non-skin and off-centre cases. Native visual checks are
still required; angle statistics and passing CI alone are not visual acceptance.

## Recovery evidence

Candidate 90 completed before its external run expired, but polling was lost.
The unchanged original managed job.json recorded success, exit code zero,
normal exit and completed owned-process cleanup; all 64 output hashes and the
planned script SHA matched. Source packaging now accepts that original durable
record as well as the existing RPC format, without inventing an RPC envelope or
replaying generation. Durable records must remain in their original workspace
run/request directory with a valid completed.json marker. Relocated records,
symlinks, path-bearing identifiers and incomplete/failed jobs are rejected.
The same original journal and completion marker must be retained for rebuilding.

The SDF job on valid UUID evidence completed before the negative-path hardening
of the retained binding helper. That hardening is verified by CPU tests and code
review, not a claim of native execution with malicious path inputs. It changes
no model/shader output. Final local suites: source_layout 29, semantic_eyes 223,
face_components 122 and facial_surface 117 passed (491 total).
