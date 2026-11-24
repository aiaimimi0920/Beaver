# Static face mesh QA

This read-only checkpoint separates the actual author wires from Blender's
native per-polygon tessellation. It compares line length, edge-length spacing
percentiles and optional directional length shares, minimum triangle quality,
boundary incidence/coordinates and unchanged polygon indices. It does not
modify a project or choose a retopology candidate.

## Run locally

Use an isolated Blender file, the exact mesh object name, and a new output
file outside the repository. The exporter uses object-local base coordinates,
ignores modifiers and shape states, records ordered author polygons and reads
Blender's actual loop triangles. It does not save the scene or overwrite an
existing report. Pose, coordinate units, axes and vertex indices must match
between inputs; for example, do not interpret common numeric IDs after
independent index compaction. Inputs and geometry are not included here.

```sh
blender -b control.blend --python-exit-code 7 \
  --python scripts/faye/static_qa/export_blender.py -- Face control.json
blender -b candidate.blend --python-exit-code 7 \
  --python scripts/faye/static_qa/export_blender.py -- Face candidate.json
python scripts/faye/static_qa/mesh_readback.py control.json candidate.json --axis 0 1 0
python -m unittest discover -s scripts/faye/static_qa -p "test_*.py"
```

A JSON input contains `vertices` (XYZ triples), `polygons` (ordered zero-based
vertex-index rings), and `triangles` (the actual tessellation in the same
index space). For a named region, both inputs must include the comparable
author polygons and their complete triangle sets. Direction shares are
length-weighted unsigned 3D angles relative to the optional supplied axis;
they are not a substitute for anatomical registration or a visibility mask.
Lengths are in the mesh's original units. Triangle quality is
`4*sqrt(3)*area / sum(edge_length²)`, 1 for equilateral and 0 for degenerate.

The report is evidence, not a pass/fail modeling verdict. It does not check
triangle-to-polygon correspondence, surface contact/intersection, projected
silhouette, material, UV, animation, or runtime import. For visual review,
also verify those applicable properties separately using matched cameras
and approved assets. Never commit private geometry inputs or rendered assets.
