# Current raw13 forehead-flow contract

This is a generation-time recipe for the existing opt-in `sampling_feature_flow`
mode. It does not edit a saved model after generation and does not contain
reference-character vertices. It is not a claim that all eye, face or ear preset
combinations have been validated.

## Two explicit geometry baselines

1. `regularize_forehead_author_surface` makes a bounded radial adjustment to the
   current raw13 skin surface. It preserves height, functional boundaries and
   open-edge vertices, mirrors the result and limits vertex displacement to
   2 mm. This is an intentional author-shape change, not a surface-preserving
   retile. The report records every changed point before and after the change.
2. Guide redistribution and conforming closed-star refinement use that new
   author surface as their frozen interpolation source. The independent retile
   check uses 32 subdivisions and a 1 mm sampled bidirectional distance limit.
   That limit does **not** describe total change from the original input model.

The native caller reports `author_shape_change`, its generated-pose
correspondence check, and the separately labelled `sampled_shape_guard`.
`source_lineage_baseline` explicitly says to apply the recorded author-point
changes before checking retile donor interpolation. Do not silently compare
that lineage to the pre-adjustment geometry.

## Point placement and ear attachment

The recipe first distributes the upper guide stations and nearby skin supports,
then adds conforming support stars where long spans would cut across the surface.
A second fit reduces adjacent point-spacing jumps. A fixed physical-scale
objective spreads the remaining purple-rail turn. Functional and mesh-quality
checks remain mandatory; adding samples alone is not evidence of a better curve.

The green rail ends at the highest **fitted** positive pinna root, resolved from
`ear_root_fit` in the current assembly frame. It does not use the authored
pre-fit ear position or treat an interior support point as the attachment.
A unique descending skin-edge route is required. Ambiguous layouts fail closed.

## Validation and scope

- Quad quality, chart orientation, crossings, boundary identities, Euler
  characteristic and guide-edge continuity are checked before native acceptance.
- Native point, deform, UV, face and edge fields use frozen donor transport.
- The final original-vertex MouthOpen field difference is checked against a
  cumulative 0.5 mm limit. Newly created vertices have no original same-ID field;
  their provenance is the recorded interpolation. Individual solver-stage limits
  alone are not a cumulative-field proof.
- Final curve evidence uses at least 20,001 chord-angle centers plus station
  breakpoints. The smaller sampling set inside the optimizer is not the final
  measurement. These are polyline measurements, not analytic curvature bounds.
- The author-shape step checks 729 combinations of the generated expression
  fields and both diagonals. The retile checks 11 named poses and 729 orientation
  states. These sampled checks are not a continuous collision certificate.
- Identical pose geometry may reuse an exact byte-keyed distance query. Reports
  retain all named cases and identify reuse and the number of unique queries.
- Reverse distance uses conservative enclosing-sphere pruning and the same
  exact point-to-triangle calculation. “Exact” applies to the discrete query
  points and triangle mesh, not continuous bilinear-surface Hausdorff distance.
- Optimizers can stop at their iteration budget with a selected feasible point.
  Reports retain the actual convergence status; successful hard checks do not
  imply global optimality.

All work happens on the existing detached native transaction. The original mesh
is not committed until validation passes. Acceptance still requires fresh saved
native meshes, field/export checks, engine pose checks and matched opaque white
wireframe views at front, right-front 45 degrees and right side. CPU evidence or
a passing test suite alone is not a native or visual acceptance result.

Run the focused regression tests with:

    python -m unittest discover -s examples/head-generation/semantic_eyes -p 'test_forehead*.py'
