# Faye V204 static replay

V204 is a local topology review of the transition beneath the right mouth corner. It is not the full face retopology, a runtime asset, or an approval to replace another asset.

## Reproduce the reviewed geometry

Use Blender **4.3.2** (official build) and the authorized private **V170 paired-quads static review** Blender file, whose exact SHA-256 is in `v204_recipe.json`. Obtain that file from the project owner; the repository does not contain it. R1 is an earlier protected reference, **not** a substitute replay input. Nothing from the external comparison reference is required or distributed.

From the repository root, substitute the local paths:

```sh
blender --background --factory-startup --python-exit-code 7 \
  --python scripts/faye/static_qa/replay_v204.py -- \
  /path/to/authorized-v170.blend /outside/repository/faye-v204-static-review.blend
```

On Windows use the corresponding absolute file paths and a single line if appropriate. The explicit output must be a new `.blend` outside this checkout in an existing directory. An existing file is never overwritten. The output directory must support hard links (for example, a local NTFS or ext4 directory). The script verifies a temporary copy beside the output before publishing it under the final name. If copying, verification, or publishing fails, it does not leave a partial `.blend` at the requested name. The command verifies the input hash before loading it and compares the rebuilt vertex positions, ordered author polygons and oriented Blender loop triangles to the reviewed geometry fingerprint before and after saving. The output file's byte hash can differ from the review original. No Python packages beyond Blender and its bundled Python are needed.

The recipe moves station 303 by a weighted combination on the original surface and replaces four polygons with three. It does not include a copy of the original mesh or embedded coordinates. Blender readback for this revision has 1,095 stored positions, 2,149 author edges, 1,121 author faces, and 1,920 loop triangles. In the original static review the patch's projected author-wire length fell 7.9% front, 8.9% right 45°, and 8.7% right; five checked views had no silhouette pixel change and the maximum measured first-hit depth difference was 0.675 mm. These historical measurements are not automatically recomputed by this script.

There is no UV/UV2, rig, expression, material parity, or engine validation. Do not use the result as production data. The script preserves the input camera setup; for a visual review use matched cameras, neutral white, and actual authored edges separately from tessellation.

```sh
python -m unittest discover -s scripts/faye/static_qa -p 'test*.py'
```
