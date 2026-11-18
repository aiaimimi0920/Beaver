# Explicit generalized source transport (inactive utility)

`source_generalized_transport.py` validates a narrowly scoped transport contract
for sparse source-owned component changes. It is not registered in generation,
does not modify a mesh, and does not replace existing triangle/bilinear APIs.

## Semantics

A row has one to four distinct original donor IDs and exact integer numerator /
positive denominator weights. All weights are nonnegative and sum **exactly** to
one. No epsilon pruning, clamping or normalization is performed. Positive tiny
weights participate in categorical agreement. Conversion to float64 occurs only
for continuous-value evaluation; nonfinite derived results fail closed. Saved float32 rounding and its measured errors
are a separate caller responsibility.

Generalized affine weights are **not** triangle barycentrics or bilinear `(s,t)`.
In particular a nonplanar four-corner mixture need not lie on either source
surface. Both source-reference shape checks remain mandatory. Normalization of
normals, independent native deltas, actual saved key composition, native/GLB
rounding and renderer encoding are separate policies, not inferred here.

`generalized_source_binding` binds immutable raw, explicit author and frozen-field
SHA identities separately from current weights. `frozen_field_identity` refers to
the original cumulative-drift baseline, never the previous optimizer iterate.
Source hash contents are supplied/verified by the caller against actual files;
a well-formed hash alone is not an authentic-source proof.

## API and caller duties

- `generalized_source_weights`: exact rational validation, retained donor order.
- `generalized_source_values`: continuous affine evaluation; categorical mode
  requires all positive-weight donors agree in both scalar category type and value.
  Caller lists are checked before string/numeric coercion; an already coerced
  caller array cannot reveal its discarded history. No categorical averaging.
- `generalized_corner_values`: explicit layer, owner and source loop IDs mapped
  to the donor vertex IDs. Select each CORNER chart side independently. A seam
  ambiguity must remain explicit; it is not resolved by nearest position or by
  averaging sides. Layer-array provenance and any continuous chart transition
  proof remain caller responsibilities.
- `generalized_sparse_readset`: exact JSON readset comparison and collision-free
  append/replace identities; replacements cannot change baseline identities.
  It rejects nonfinite complete operation records and non-string JSON keys;
  it does not apply operations. Include every consumed point/face/boundary and
  full original-field record in the readset. Unrelated mesh records remain outside
  the operation list. Verify source files and edge-child inheritance separately.
- `generalized_fixed_quad_triangles`: exactly two oriented triangles per quad,
  exact directed boundary coverage, one oppositely traversed internal diagonal,
  no duplicate triangles. This local partition check is not a manifold, global
  overlap or duplicate-quad proof; callers must validate the whole patch. It does
  not infer a new diagonal per pose.
- `generalized_fixed_triangle_poses`: supplied states only, fixed triangle
  nondegeneracy and positive neutral-relative normal dot. Input and derived
  cross/norm/dot/product/cosine values must be finite; boolean/string geometry
  is rejected. `states_checked=11`
  is **not** a 729-state result. Enumerate the actual full six-control lattice
  explicitly for 729 evidence. Neutral+Open paired export eligibility, neutral
  25–150/aspect7, eleven-pose bidirectional dense1mm and native/visual gates are
  separate and unchanged.

## Evidence and limits

Private actual-component regression validates 64 rational rows (two four-donor
rows), 48 quads / 96 fixed triangles, eleven actual native poses and all729
explicit saved-key compositions. It is a transport/identity regression, not an
integrated face generator. Public tests are synthetic, reindexed and asset-free.
The skin+oral recovery proof separately distinguishes exact authoritative fields
from lossy renderer normal encoding: this utility makes no shader fidelity or
full-model acceptance claim. Default generation remains unchanged.
