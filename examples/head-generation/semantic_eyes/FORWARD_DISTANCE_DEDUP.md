# Exact forward candidate deduplication

`ChinFrozenDistance.distance` evaluates each distinct triangle/query pair once,
then restores every original candidate slot before the unchanged norm, invalid
slot mask and `argmin`. The first original slot still wins ties, even when that
triangle has a larger ID. Invalid slots still map to triangle zero before being
masked, preserving the prior return values for all-invalid/degenerate inputs.
This optimization does not make otherwise-invalid geometry valid.

The 12-nearest-vertex search, incidence ordering, 200-query chunk size, float
precision, sample grid, target domain, actual export diagonals, return formats
and caller-owned arrays are unchanged. Temporary grouping arrays are bounded
by one chunk's original incidence slots; no multi-pose cache is added. This is
exact reuse of repeated work in the existing metric, **not a new globally exact
forward-distance algorithm**. The independent reverse-distance routine is not
modified. All 1 mm, 11-pose and 729-orientation checks remain unchanged.

## Reproducible public regression

With NumPy and SciPy installed, from the repository root:

```sh
cd examples/head-generation/semantic_eyes
python -m unittest -v test_chin_source_distance test_chin_source_retile
```

The tests contain an independent copy of the pre-change candidate-slot routine
and deterministic synthetic geometry; no private model or environment is
required. They compare dtype, shape and raw bytes for distance, selected
triangle ID and hit coordinates, including non-contiguous queries, float32 and
float64 queries, chunk boundaries, vertices, edges, duplicate quads, reversed
ID tie order, invalid incidence and degenerate NaNs. They also check unchanged
inputs, unchanged small-tree failure, and bounded/reduced closest-point calls.

## Completed full-case parity evidence

The following historical frozen-fixture runs used Python 3.13, NumPy 2.3.5 and
SciPy 1.17.0. Both modes ran both per-owner AFTER semantics and both final shape
guards, including the original duplicate connected pass. Each mode retained
subdivision32, all11 poses, all729 orientations and the actual paired-export
selector. The only selected code difference was this forward-distance method.
These are performance/parity checks, **not passing shape or full-model claims**.

| Frozen case | Source owners | Current sampled faces | Forward calls | Original seconds |  Dedup seconds | Reduction |
| ----------- | ------------: | --------------------: | ------------: | ---------------: | -------------: | --------: |
| 536         |           236 |                   584 |            32 |   1257.532548555 |  990.204514818 |    21.26% |
| 545         |           236 |                   592 |            32 |   1346.682645854 | 1059.137878249 |    21.35% |

For each case the comparison required byte-identical JSON receipts for legacy
AFTER, connected AFTER and both final guards. In addition, every forward call
had matching dtype/shape/data hashes for its full query array, source tessellation
points and triangle indices, and full returned distance/triangle-ID/hit arrays.
Timing values alone were excluded. Both cases passed all comparisons.

To reproduce that protocol with an authorized fixture: freeze input and source
hashes; run original and optimized methods separately with identical numerical
dependencies and validation calls; hash every array including dtype and shape;
require identical ordered call traces and receipt bytes, not just equal maxima.
The fixture geometry is intentionally not included here. The public synthetic
regression above is independently runnable; this table alone is not a substitute
for obtaining the authorized fixtures when repeating the full experiment.

Comparison receipt SHA-256:

- Case536: `ef1fb0ffb0de4a0b222b96d56ad2b5dfe9469df861fc6d8e158f5a6b95255d67`
- Case545: `5f12fb7a95ee6dae257d4a33e34f57d4a33f36c92a7fc1d7abbc1c8ae73b3522`

Frozen experiment manifest SHA-256:
`5395c758a8d67425b4a7b31db775a31ccf97c3a8c0d962892115be5b19e0e111`.

Connected receipt reuse, tessellation caching, domain changes and metric changes
are deliberately excluded from this patch and require separate review/evidence.
