# Inactive exact candidate-threshold witnesses

This **separate opt-in interface** does not enable any checker or replace floating
mesh-distance statistics. It proves existence of a point within a radius on ONE
supplied actual triangle. It does not require the triangle to be nearest and
never reports a whole-mesh FAIL.

```python
import numpy as np
from mesh_distance.exact_certificate import ExactThresholdTarget

vertices = np.array([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]])
identity = dict(baseline="AUTHOR", surface_kind="actual_triangles",
                surface_id="synthetic-example", pose_id="neutral",
                source_sha256="0" * 64, units="m")  # Example identity only.
target = ExactThresholdTarget(vertices, [[0, 1, 2]], identity=identity)
records = target.certify(np.array([[.25, .25, .001]]), [0], .001,
                         expected_target_digest=target.target_digest)
assert records[0]["status"] == "WITHIN"
```

Use the actual immutable source artifact identity in real work, not the example.
The API binds identity labels to geometry bytes; it cannot authenticate whether
an external RAW/AUTHOR, source or pose label is truthful. That remains the audit's
responsibility. Geometry triangle IDs NEVER imply field/UV ownership.

## Only two geometric outcomes

- **WITHIN**: an exact witness on the candidate triangle satisfies d² <= r².
  The recorded squared distance is a **witness upper bound / existence proof**,
  NOT the triangle's exact minimum distance or a nearest-hit report. The routine
  may return as soon as any edge proves within, even if the interior is closer.
- **UNRESOLVED**: no witness was produced on this candidate, or the integer
  resource policy prevented a certificate. Another triangle may prove within.
  UNRESOLVED is **not** geometric FAIL, outside-mesh proof or permission to skip
  the query. Whole-target/batch resource limits raise `CertificateUnsupported`,
  also explicitly classified UNRESOLVED rather than geometric FAIL.

Malformed/nonfinite/negative-radius input, wrong identity, bool/noninteger IDs or
invalid shapes raise `ValueError`; do not reinterpret input errors as model
results. No automatic fallback, threshold tolerance or radius inflation occurs.

## Exactness and original bits

Coordinates and threshold must already be binary64 (NumPy float64 arrays or
rows of Python float/NumPy float64 scalars), in metres. No float32/integer
coordinate coercion is hidden in the API. Sequence row widths and scalar types
are checked BEFORE NumPy promotion/allocation. Array shape/dtype/counts are also
checked first. Canonical little-endian snapshots use integer views, preserving
subnormals and signed zeros. Floating finiteness/sign predicates are not used
for certificate decisions; IEEE exponent/sign bits are decoded directly.

The threshold is the EXACT represented binary64 caller value, squared with
integers. For `.001`, this means that binary64 value, not decimal rational1/1000
and not the rounded binary64 product `.001 * .001`. Receipts bind the original
threshold bits and its exact ratio. Integer calculations do not use square
roots, epsilons or the floating distance/rounded hit as acceptance evidence.

Exact dyadic coordinates share an integer power-of-two grid. This is merely a
mathematical representation, not a modification/normalization of the asset.
Three exact clamped segment predicates cover edges and true degeneracies. For a
nonzero triangle, exact barycentric signs establish an interior projection, and
`((p-a) dot n)^2 <= R^2 * (n dot n)` proves the plane witness within. Products and
sign comparisons use Python integers; true zero area is exact, with no sliver
area tolerance. `exact_predicate.candidate_within` is an internal helper requiring
wrapper-validated finite coordinates, a NONNEGATIVE binary64 radius and the
validated integer budget; use `ExactThresholdTarget` as the public interface.

The exact certificate has its own integer resource domain and does not need the
floating backend's ±16m numerical cap. It does NOT silently route unsupported
floating queries here. Calling it is explicit, and its finite triangle witness
scope remains narrower than whole-model or continuous-surface acceptance.

## Immutable target, identity and optional estimator

Build copies vertices and indices into immutable byte-backed arrays. Later
changes to caller arrays or identity dictionaries do not change the target.
Queries are separately snapshotted; callers must not mutate input during a call.
Target identity includes complete geometry bytes/order, RAW/AUTHOR axis,
`surface_kind`, specific `surface_id`, pose, source SHA256 and metre units.
Every record binds that digest, query original bits, candidate ID, threshold
original bits/ratio, implementation digest and integer budget.

If a floating candidate estimate is wanted, call
`target.build_bvh(build_receipt=...)` explicitly. It constructs the opt-in BVH
from this SAME immutable target snapshot. The caller owns and closes the returned
BVH. Its numerical capability domain remains separate and can reject the build;
there is no automatic replacement. Candidate IDs from elsewhere are untrusted
suggestions: a WITHIN witness still proves only the certificate object's actual
bound triangle. `expected_target_digest` must match the audit's intended target.

`verify_within` recomputes a record against caller-supplied original query,
candidate ID and threshold. It does not trust a stored status or squared-distance
claim. External records are compared against the bounded expected schema without
serializing arbitrary input; extra keys, wrong types/lengths and bool-as-int are
rejected. Hashes detect identity mismatch, not hostile-code authenticity. Only run
trusted local source.

## Resources and complexity

No native build/load, network, install or worker thread is needed for certificates.
The immutable target snapshot byte budget is128MiB, with at most2million vertices
and2million triangles; this is NOT a peak/process-RSS cap. Caller arrays,
conversion temporaries, NumPy/runtime and multiple target objects are separate.
At most4096 queries per call; batch without dropping samples. Python object
lifetime owns the immutable bytes, with no native certificate handle to free.

The configurable integer budget is128–16384 bits. Conservative bounds on
coordinate differences, degree-six expressions and squared-distance denominators
are checked BEFORE integer products. Exceeding it yields UNRESOLVED, never FAIL.
Proof fractions use hexadecimal encoding to avoid Python's decimal-digit limit.
One candidate per query takes a fixed number of bounded big-integer operations:
O(Q*M(b)) bit work up to constant degree factors, plus O(V+T) initial snapshot/hash.
No full-target Python-integer conversion cache grows without bound.

A true whole-mesh FAIL would require exact checks of every triangle or separately
proved conservative pruning. Neither is provided here. No experience-based error
margin turns a floating nearest candidate into a FAIL certificate.

## Surface / acceptance limits

A certificate concerns the given finite triangle target and one query point.
`bilinear_tessellation_triangles` explicitly means that TRIANGLE APPROXIMATION,
not the exact authored bilinear surface. Exact authored-bilinear targets are not
an accepted representation label. Bilinear-generated query points may be checked
against actual triangles, but a bilinear target needs separate conservative/exact
surface treatment. RAW/AUTHOR and triangle/bilinear axes remain independent.

This is NOT continuous Hausdorff certification or whole-model admission. Required
samples, both directions, all poses, native saved fields/fixed triangles, export,
engine and visual checks remain separate. The V5 floating backend's real boundary
false-pass/false-fail examples and bitwise-compatibility FAIL remain valid history;
this interface does not relabel those results as passing.

## Explicit synthetic tests

From the repository root, with existing Python/NumPy:

```sh
PYTHONPATH=tools python3 -m unittest discover -s tools/mesh_distance/tests \
  -p 'test_exact_*.py' -v
```

Optional integration and fenv tests require explicitly supplied trusted local
builds, never compiled by test import:

```sh
g++ -std=c++17 -O2 -shared -fPIC -Wall -Wextra -Werror \
  tools/mesh_distance/tests/certificate_fenv_helper.cpp -o /tmp/certificate-fenv.so
EXACT_CERT_FENV_HELPER=/tmp/certificate-fenv.so \
MESH_DISTANCE_BUILD=/path/to/explicit/build.json \
PYTHONPATH=tools python3 -m unittest discover -s tools/mesh_distance/tests \
  -p 'test_exact_*.py' -v
```

The test-only MXCSR helper is x86_64 Linux and restores caller state in `finally`.
Coverage includes exact boundaries, permutations, degenerate/sliver/large integer
cases, Fraction crosschecks, non-nearest witnesses, identity tampering, malformed
inputs, preallocation guards, endian/signed-zero bits, integer limits and
DAZ/FTZ/rounding environments. Public tests contain no character assets.
