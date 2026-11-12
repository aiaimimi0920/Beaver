# Explicit source-chart and quad transport utilities

These are **inactive utilities**, not a new default recipe, mesher, model checkpoint or whole-model completion. No Gmsh/GPL algorithm is embedded. Legacy consumers and dependency policy are unchanged. NumPy is an existing numeric dependency; SciPy sparse solve imports are lazy inside `facial_source_chart` only.

## Contract

`facial_source_chart(vertices, quads, triangles, triangle_owners, boundaries, original_ids=...)` receives one explicit owned, connected genus-zero quad surface. Each quad must be partitioned into exactly two consistently oriented chosen triangles; callers must select them using their actual paired-export semantics. Quads/triangles contain original IDs; owners index the supplied quads. Vertices are in the explicit original-ID order. The first ordered boundary is the outer boundary; subsequent loops are holes. No implicit skin-region labels, nearest-point ownership, character IDs or file paths exist in this API.

A convex unit-circle boundary and positive uniform graph weights define the UV solve. Temporary abstract centers cap the holes **only during parameterization**. Original XYZ, topology, fields and UV corner data are not edited. Returned domain faces/IDs exclude every virtual center/cap; source quad owners and canonical triangle coordinates are retained.

Validation rejects missing/duplicate quad partitions, inconsistent winding, malformed IDs, disconnected/nonmanifold topology or vertex links, wrong boundaries and Euler count. Numeric certification checks strictly positive actual and virtual UV triangle areas, triangle-overlap area, and noncrossing/nontouching boundary segments. The explicit overlap tolerance is measured in unit-chart area (default1e−13, maximum1e−10), not a geometry/field/shape acceptance waiver. It is a floating-point certificate, not exact-predicate proof. High metric distortion can remain despite injectivity; callers must assess the world Jacobian before using UV for meshing.

## Transport

`facial_quad_weights(canonical_triangle, barycentric)` maps located chart-triangle coordinates to original quad `(s,t)` and four bilinear weights. `facial_quad_transport(quad_values, canonical_triangle, barycentric)` applies those weights to explicitly continuous floating values. It **does not replace the original bilinear surface with planar XYZ triangles**.

The caller retains owner/weights as provenance and selects UV corner values from the actual source face/island. Categorical labels are not interpolated. `facial_transport_candidates` rejects conflicting candidate values rather than silently averaging across seams; it does not choose ownership. Discrete fields represented as floats remain the caller's responsibility and must not be passed as continuous values.

No helper performs donor lookup, remeshing, guide rebinding, birth-field freezing, or native field/UV transport. Any downstream topology still needs exact original pin/boundary/chain preservation, all guide/field baselines and caps, actual paired-export validity,25–150°/aspect7, cumulative0.5mm field and independent bidirectional1mm/all-pose gates. No current rejected Gmsh or other candidate becomes accepted by using these utilities.

## Verification

Focused tests cover capped annulus, arbitrary/reindexed IDs, immutable inputs, invalid partitions/winding/boundaries, bilinear versus planar interpolation, diagonal continuity and explicit seam ambiguity. Separate private real-source regression verifies all938quad/1876triangle source domain without including private geometry in this library or its public tests. Sourcepack validation exercises the unchanged existing recipe; these unregistered utilities are not automatically flattened or dispatched.
