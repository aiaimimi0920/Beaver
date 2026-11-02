# Inactive shared facial rail proposal utility

This standard-library-only package is **default-inactive**. No generator, checker,
export adapter or production dispatch imports it. It implements source-owned rail
proposals and synthetic regression policies, not a completed character optimizer.
Every result has `native_ready=false` and `full_model_success=false`.

## Run

From the repository root, with the project's existing Python interpreter:

```sh
PYTHONPATH=tools python3 -m unittest discover \
  -s tools/shared_facial_rails/tests -t tools -v
PYTHONPATH=tools python3 -m shared_facial_rails --synthetic-demo
```

The demonstration prints JSON to stdout. Optional `--output path.json` creates a
new file and refuses to overwrite an existing one. It constructs only synthetic
curved grids, never reads a character asset or writes a native model. No package
installation or numerical dependency change is required.

For explicit programmatic use:

```python
from shared_facial_rails import Source, digest, redistribute

# The caller supplies and authenticates the entire source capture and role map.
source = Source(payload, expected_source_sha256)
proposal = redistribute(source, source.readset())
assert not proposal["native_ready"]
```

`digest(payload)` produces the canonical packet digest; it does not authenticate
external source evidence. A production adapter must verify the genuine RAW and
AUTHOR captures independently. Distinct role records may legitimately reference
identical content hashes when no author adjustment occurred.

## Implemented boundaries

- Immutable JSON-byte source packets contain neutral points, six explicit source
  displacement fields, fixed source diagonals, actual ordered rail edges,
  protected identities, categories and three side-specific corner UV layers.
- Complete content-bound source readsets and frozen field baseline identities
  reject stale binding or rebasing. Donor rows must be convex adjacent rail-edge
  samples. Geometry and fields use those same declared rows. No XYZ-nearest
  ownership, category averaging, point birth or topology change is performed.
- Full-chain measurements include real edge max/min, worst relative length,
  atan2 3D turns, tangent bending/variation, transverse spacing and quad angles
  and edge aspect. Full transverse domains are independently derived from actual
  source edges for each unordered rail pair. Caller labels cannot partition a
  pair into favorable singleton groups or hide missing edges.
- Bounded equal-source-arc proposals retain protected stations and endpoints.
  Their source domains are split at protected interior stations. Nonfinite or
  overflowing derived points, fields, saved keys and UVs are rejected.
- `RAIL_PROPOSAL_V1` returns an **unadmitted** rail-only proposal. It reports
  transverse/support metrics even when they worsen.
- `SYNTHETIC_JOINT_NONREGRESSION_V1` also requires nonregressing along-rail ratio,
  worst-relative spacing and 3D peak turns, transverse ratio/worst-relative
  spacing, and worst quad aspect/minimum/maximum angle. At least one rail ratio
  must improve. Resumed inputs must satisfy the original frozen source bounds;
  an already-worsened input cannot establish looser limits.

The latter is an explicit **synthetic engineering policy**, not a user-approved
visual threshold. Float64 comparisons use a `1e-12` numerical tolerance. Neither
policy promises globally equal spacing, a smooth face, or optimization convergence.

## Regression coverage

The tests use two genuinely different curved quad layouts: 4×5 and 5×7 point
grids, with six fields, three UV layers, discontinuous corner sides and explicit
protected feature interfaces. They verify actual joint improvements, identity /
face / loop / owner permutations, immutable binding, field limits, protected
points, canonical transverse domains, numerical overflow and source-side failures.

Outer-pinned fixtures improve rails while damaging transverse supports. The joint
policy rejects them. Separate joint fixtures allow all horizontal rails to move
while retaining their end interfaces; they improve both rail and transverse
spacing and support quality. Changing the protected-role map is explicit fixture
construction, not a weakened admission of the failed fixtures.

The deterministic JSON regression is synthetic. It reproduces a former bug in
which splitting a transverse domain into singleton labels hid a full-pair spacing
regression. Both the split domain and the regrouped, genuinely worsening proposal
are now rejected.

## Not implemented or certified

- Only disjoint open rails are supported. Intersecting rails, arbitrary source-face
  movement, categorical seam crossing, support insertion, mirrored/closed role
  inference and automatic landmark discovery are not supported.
- Corner interpolation proves individual same-side source samples, **not** complete
  new-face source-area coverage. All native deform, normal, material, editor and
  other channel transport is not implemented.
- The synthetic float64 cumulative displacement-field limit is at most 0.5 mm.
  This is not the full native three-channel float32 field-budget proof. The local
  step bound is separate and never rebases the original field reference.
- Neutral fixed-triangle orientation is screened. The real-model independent
  11-pose bidirectional 1 mm tests, 729-state checks, actual saved-key arithmetic,
  native save/reopen, GLB, current-engine, visual and fresh-template acceptance
  remain required and are not performed here.

These utilities preserve a reusable development step. They do not change default
face generation or substitute synthetic tests for end-to-end model validation.
