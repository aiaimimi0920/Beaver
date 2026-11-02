# Opt-in CPU triangle-distance backend

This is a **default-inactive** tool. No existing checker, native generator, export
adapter or dispatch imports it. Building/loading is explicit; installation,
download, source rebinding and model acceptance are not performed here.

## Build and use

Inputs and returned distances use **metres**. Only original target/query
coordinates with every axis in **[-16, 16] metres** are supported, together with
the numerical restrictions below. This is a backend capability limit, not a
model-quality gate. Do not translate, scale or rebind an asset to evade it.

Requires existing x86_64 Linux GCC with `__float128`/libquadmath, Python and NumPy. The
comparison tests additionally use the repository's NumPy/SciPy legacy checker.
No dependency installer or binary is distributed. From the repository root:

```sh
python3 tools/mesh_distance/build_backend.py --output-dir output/mesh-distance
PYTHONPATH=tools python3 - <<'PY'
from mesh_distance import TriangleBVH
with TriangleBVH([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], [[0, 1, 2]],
                 build_receipt="output/mesh-distance/build.json") as target:
    distance, triangle_id, hit = target.query([[.25, .25, .001]], details=True)
    print(distance, triangle_id, hit)
PY
MESH_DISTANCE_BUILD="$PWD/output/mesh-distance/build.json" \
OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 \
python3 -m unittest discover -s tools/mesh_distance/tests -v
```

For a pinned environment, use its Python executable and NumPy/SciPy path; the
tool does not modify pinned dependencies. C ABI tests can be linked explicitly:

```sh
g++ -std=c++17 -Wall -Wextra -Werror tools/mesh_distance/tests/c_api_test.cpp \
  -L output/mesh-distance -lbeaver_mesh_distance \
  -Wl,-rpath,"$PWD/output/mesh-distance" -o output/mesh-distance/c_api_test
output/mesh-distance/c_api_test
```

`build.json` records ABI, compiler/version, flags, each native source SHA256,
aggregate source digest and binary SHA256. Loading verifies receipt, source,
binary and the native-exported ABI/digest before use. This checks accidental
mismatches, **not authenticity of untrusted native code**. Only load a trusted
local build. Do not rebuild/overwrite a binary while it is loaded in a process.

The GCC flags are fixed in `build_backend.py`: C++17, `-O3 -fPIC -shared`,
`-ffp-contract=off -fno-fast-math -frounding-math -pthread`, and warnings as errors.
There is no `-march=native` or fast-math. `-lquadmath` links the existing GCC
quadmath runtime for fused-product residuals in zero-area verification; no
installation or bundled runtime is added. Binary hashes are build-specific.

## Algorithm and numerical contract

- An immutable median-split AABB BVH; eight triangles per leaf. Bounds expand
  outward by `nextafter`. Positive box-distance subtraction/square/sum steps
  round their lower bound downward. Pruning upper bounds are upward-rounded
  distances to **actual input vertices**, not rounded projected hits.
- Strict `lower > upper` pruning cannot discard a geometrically closer triangle
  because of those bound-rounding operations under the checked IEEE environment.
  This property is not a proof that the finite-precision hit kernel is a
  real-arithmetic nearest-point oracle for every possible input.
- Every retained triangle considers its interior and edges/vertices. Nearest
  vertices are **never** substituted for mesh distance. Exact degenerates and
  slivers **inside the supported numeric domain below** are supported; Gram arithmetic switches to binary128
  cross/plane arithmetic at `den <= .01 * max(aa, cc)^2`. This checks both angle
  cancellation and edge-length imbalance after cyclic origin selection. It switches arithmetic,
  not a geometric area tolerance that deletes nonzero-area triangles. Unsupported
  conditioning now rejects the target rather than quietly treating it as a line.
- Both paths cyclically rebase to a nearby original vertex; segment hits are
  reconstructed from the nearer endpoint. Inexact binary64 input differences
  route to binary128 via an error-free subtraction-residual check. Plane hits
  in the wide path compete with edge/vertex hits by binary128 squared distance
  **before narrowing**; rounded off-surface hits must not spuriously win. Then
  the chosen hit is rounded and its returned distance is recomputed. Plane hits
  never unconditionally overwrite a closer candidate. Gram-classified
  interiors always recheck in the wide path: even the closest vertex can be
  distant, making binary64 barycentric reconstruction cancel a local feature.
  Ordinary edge queries retain the Gram classification formula,
  **not its bitwise operation ordering**. Hits are rounded
  to binary64; the returned distance is the Euclidean query-to-returned-hit
  norm. Computed-distance ties use the lowest input triangle ID. Binary128
  fallback can change legacy near-zero hits/IDs; blanket bitwise legacy identity
  is **not** promised. `exhaustive=True` disables pruning for bounded comparisons.
- IEEE round-to-nearest and gradual underflow are required. Both x87/fenv and
  **SSE MXCSR rounding bits** are checked; non-nearest SSE-only modes and FTZ/DAZ
  fail closed. Currently only x86_64 Linux is supported. Compilation requires
  binary64 IEEE doubles, 64-bit pointers/indices, GCC binary128 and wide long double.


### Supported numerical domain (version 5, deliberately narrower)

An independent review found a **major**, not threshold-rounding, failure outside
this domain: triangle `[(1e40,1e40,0),(0,0,0),(1,0,0)]`, query `(.5,.25,.001)`
returned approximately `.250002` instead of `.001`. Binary128 subtraction had
lost a feature and treated a nonzero triangle as a line. That input is now rejected
before index construction, for all triangle permutations and tested scales.

A subsequent admitted-domain stress test found that domain guards alone were
insufficient: reconstructing from a distant vertex gave 17.2679 for a true local
distance 0.552067. Stable cyclic origins and near-endpoint segment construction
now address that failure in both ordinary and wide arithmetic, with permutation
and randomized tests. The old bitwise legacy-witness compatibility observation
is not an API promise and does not survive these arithmetic-order improvements.
An all-remote, well-conditioned skew triangle also exposed binary64 interior
reconstruction error (1.03118 mm for a true 1 mm query). Interior queries now
recheck with the wide plane/edge path instead of reconstructing in binary64.
That was still insufficient at arbitrary scales: independently generated
vertices around 1e44 summed exactly to zero, yet a query at their true centroid
returned a distance around 8.66e9. Binary128 plane-dot cancellation was not
reliable there. **Those high-scale failures are not declared repaired.** They,
and previous high-scale successes, remain unsupported regression cases. Private
history retains the rejected versions and their results.

- Require every original target and query coordinate to satisfy `abs(value) <= 16`
  in metres. The exact power-of-two boundary is inclusive; its next representable
  outside value is rejected. No internal translation, scaling or rebinding occurs.
  The cap bounds coordinate magnitudes and cancellation scales for this optional
  backend; it is an engineering scope restriction, **not a sufficient mathematical
  accuracy proof**. Out-of-domain means unsupported, neither model pass nor fail.
  There is no automatic replacement/fallback or change to authoritative receipts.
- On each coordinate axis, use the lowest/highest **actual significant bit**
  positions L/H among nonzero target coordinates, including trailing-zero and
  subnormal handling. Require `H-L+2 <= 113`, accounting for a subtraction
  carry. Each query coordinate must fit that bound together with the target.
  Zero is exempt; query points are checked individually, so batching does not
  change this condition. This ensures coordinate differences fit binary128's
  113 significant bits without falsely treating float32-derived values as full
  53-bit mantissas. It is not translation/parameterization invariant.
- Each nonzero triangle must satisfy `|AB cross AC| / max_edge_length^2 >= 2^-60`,
  evaluated in binary128. More ill-conditioned inputs fail closed, not become
  line/point substitutes. Zero computed area is accepted only after exact-product
  residual comparisons confirm represented collinearity; otherwise it is rejected.
- This intentionally narrows the former unsupported all-finite/sliver wording.
  Ordinary 1e-15 slivers, exact degenerates, tiny scales and translations within
  the cap remain tested; arbitrary finite coordinates/condition numbers are **not**
  promised. A domain error means this backend cannot provide the requested
  query, not that the model passed or that the geometry may be simplified.
  Do not silently change coordinates, rescale/rebind sources or loosen gates to
  evade it; retain the authoritative reference checker and explicit review.

These input guards reject the identified out-of-domain failures; they are not a
formal bound on all hit errors inside the domain. Finite-precision edge/interior
decisions and represented hit coordinates still require witness review.

**This is finite-query, finite-precision geometry, not continuous Hausdorff
certification, exact real arithmetic, or a model pass.** Rotated/translated
synthetic points near exactly 1 mm can disagree with a Decimal true-distance
oracle because the returned hit is rounded. Tests disclose these disagreements;
no threshold epsilon hides them. Investigate a threshold witness rather than
loosening acceptance. The backend applies no threshold itself.

A future caller must retain its original finite sampling, thresholds, reference
surfaces, bidirectional coverage and witness semantics. Keep RAW and AUTHOR
positions separate, and keep triangle and bilinear surface semantics separate.
These are different axes. Geometry-nearest triangle IDs **never** authorize
field, material, normal-chart or UV ownership. Fixed-triangle expression checks,
saved native fields, export/engine and visual acceptance remain separate.

## Resources and ownership

- One computational thread; native calls serialize with a mutex and launch no
  workers. At most 128 live handles. Targets are copied and immutable; no caller
  pointers survive build/query. Do not concurrently mutate input NumPy arrays.
- Hard maximum 2,000,000 vertices/triangles and 1,000,000 queries per call. Batch
  a larger finite query set without dropping/replacing samples.
- A 512 MiB aggregate native allocation-accounting cap includes triangle/order
  arrays, reserved node capacities and 4 KiB overhead per index. A smaller
  per-build budget is accepted. This is **not** a process-RSS cap: Python/NumPy,
  caller-owned buffers, outputs, allocator/runtime and other tasks are separate.
  Release old pose indexes rather than accumulating many targets.
- Native handles are non-recycled process-local uint64 tokens. Stale/unknown
  handles, repeat native free, null nonempty buffers, invalid counts/indices,
  nonfinite inputs and unrepresentable output distances fail explicitly. C
  callers must supply allocated buffers of the declared extent: arbitrary
  invalid pointers cannot be made safe by this ABI. Outputs on C errors must
  not be used. Python validates shapes/dtypes/alignment and discards failed output.
  List/tuple triangle scalars are checked before NumPy promotion; bool and float
  identities are rejected. Already typed ndarrays require an integer dtype; a
  previous caller-side conversion cannot be retroactively recovered.
- Use the Python context manager or `close()`; Python close is idempotent.
  Destructor cleanup is only a fallback. No hidden library build, approximate
  fallback, network, model import or automatic checker replacement occurs.

## Tests and review boundary

The public suite contains only self-authored synthetic data: legacy brute and
sphere-reference comparisons; exhaustive same-kernel comparisons; duplicates,
reversed and equidistant triangles; edges/interiors; degenerates/slivers;
Decimal(100) oracle; tiny/bounded translated coordinates; explicit unsupported
historical high-scale inputs; cap/conditioning boundaries, exact-centroid and
rotated all-remote triangles; exact 1 mm and adjacent values;
resource/input/lifetime/build-identity negatives. Set `MESH_DISTANCE_RESULTS` to
an external output directory to retain Decimal diagnostics.

Private actual-query regressions belong outside this directory and must not be
published as fixtures. Review/validation and explicit caller integration are
still required before substituting this backend in a real acceptance run.
See [PROVENANCE.md](PROVENANCE.md) for source and licensing scope.
