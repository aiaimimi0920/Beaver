# Source, licensing and distribution scope

This directory contains an independently authored implementation and synthetic
tests created for Beaver. No third-party BVH source, character geometry, images,
source-field captures or other model assets are embedded. Decimal is Python's
standard library; NumPy/SciPy and GCC are existing external development/runtime
dependencies, not vendored here.

The triangle kernel and traversal were extracted from a privately validated
engineering prototype. Initial extraction preserved the arithmetic and pruning
algorithm; the shared wrapper added explicit build identity, ABI/environment
checks, resource limits and owned-handle lifecycle validation. Independent review
then added an explicit restricted numerical domain, exact-zero product-residual
validation using the existing GCC quadmath runtime, actual MXCSR control checks
and pre-coercion scalar-index checks. This narrows unsupported former all-finite
claims rather than hiding a large numerical error as threshold rounding. Further
admitted-domain tests required stable cyclic origins, near-endpoint segment
construction, plane/edge candidate comparison and wide rechecking for every
Gram-classified interior. These intentionally change operation ordering;
old bitwise-compatibility observations are not carried over. Independent review
still found large-scale binary128 plane cancellation. The supported input domain
was therefore explicitly narrowed to original metre coordinates within [-16,16]
per axis, without internal normalization. High-scale failures and prior successes
are unsupported regressions, not claimed repairs; the narrower domain is not a
universal numerical error certificate. Private prototype
fixtures/results are deliberately not part of this distributable directory.
The synthetic legacy comparisons import existing Beaver CPU reference code;
they do not contain or load a character fixture.

At extraction the checkout had no tracked top-level LICENSE/COPYING or declared
package license. This addition does **not** invent a new open-source license or
relicense the repository. It remains subject to the repository owner's rights
and permissions. External compiler/runtime and Python-package licenses remain
those of their providers; they are not changed or bundled by this tool.

Only source, documentation and synthetic tests are intended for review/commit.
Compiled `.so`/objects, compiler outputs, private meshes/query captures and
platform build receipts are excluded. The generated build receipt binds every
native source hash, the ABI, required flags and compiled binary hash; root's
private review manifest separately binds all wrapper/test/documentation files.
