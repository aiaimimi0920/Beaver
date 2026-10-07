# Lash188 technical review

Owner feedback on185: upper iris/lash intersection, wrong outer lash silhouette, missing upper fork. The original recipe now includes unchanged generated ocular surfaces in local obstacle fitting, checks triangle interiors, adds a separate inner fork and narrows the outer companion slit.

186 removed the visible iris horns at the owner angle.187 tapered the blunt outer wing.188 narrowed the remaining over-wide loop.102 pure regression tests passed. All14 generated input source hashes matched. Original source and calibration protections remain in force; this does not claim byte-identical exported normal/tangent ordering outside the edited accents.

Actual native188 validation passed; the marked close view was inspected in render, white and wire at yaw-12.8/pitch-11.25/zoom0.25. Additional front, both45-degree, right90-degree, top55-degree white and top32.5-degree render views were inspected. No upper-iris protrusion through the ribbon was seen in these checks. Roots remained near their supporting surfaces; the inner fork and narrow outer slit are visible. This is a bounded visual inspection, not proof for every pose or an owner artistic approval. The reference silhouette is not claimed to be an exact duplicate.

GLB SHA256: ae4037d4d2495dd7c62c905b56d98e3cb1febbc6bce637b0bbc5ffd281f75fef
Minimum sampled obstacle clearance: 0.00034990906715393066 meters.

Model-stage GUT7977ec51-32ab-44f8-a554-59f6d46cce77 passed10 tests/198 assertions. The comparison suite explicitly targets188. The final record also updates the head-only initialization suite from185 to188 before the final child and parent gates. Final aggregate outcomes will be added after completion.

Captures:
[
  {
    "json": "artifacts/model_comparison/compare-1791394105-59812.json",
    "png": "artifacts/model_comparison/compare-1791394105-59812.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 0.0,
      "yaw": -45.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791394019-34261.json",
    "png": "artifacts/model_comparison/compare-1791394019-34261.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "white",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 55.0,
      "yaw": 0.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791393943-14815.json",
    "png": "artifacts/model_comparison/compare-1791393943-14815.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "white",
      "mouth": 0.0,
      "pan_x": 0.196078431372549,
      "pan_y": -0.0708061002178649,
      "pitch": -11.25,
      "yaw": -12.8,
      "zoom": 0.25
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791394074-96812.json",
    "png": "artifacts/model_comparison/compare-1791394074-96812.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 0.0,
      "yaw": 45.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791394125-79008.json",
    "png": "artifacts/model_comparison/compare-1791394125-79008.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 0.0,
      "yaw": 90.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791393922-64301.json",
    "png": "artifacts/model_comparison/compare-1791393922-64301.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.196078431372549,
      "pan_y": -0.0708061002178649,
      "pitch": -11.25,
      "yaw": -12.8,
      "zoom": 0.25
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791394048-15189.json",
    "png": "artifacts/model_comparison/compare-1791394048-15189.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 32.5,
      "yaw": 0.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791394157-43935.json",
    "png": "artifacts/model_comparison/compare-1791394157-43935.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "render",
      "mouth": 0.0,
      "pan_x": 0.0,
      "pan_y": 0.0,
      "pitch": 0.0,
      "yaw": 0.0,
      "zoom": 0.99
    }
  },
  {
    "json": "artifacts/model_comparison/compare-1791393969-6644.json",
    "png": "artifacts/model_comparison/compare-1791393969-6644.png",
    "state": {
      "body": false,
      "hair": false,
      "light": -45.0,
      "mode": "edges",
      "mouth": 0.0,
      "pan_x": 0.196078431372549,
      "pan_y": -0.0708061002178649,
      "pitch": -11.25,
      "yaw": -12.8,
      "zoom": 0.25
    }
  }
]


## Native close-out checks
The final verification child GUT39b052fa-0d7d-4618-85cd-fa13a1cf3fb9 passed10 tests and198 assertions. Both head-only initialization and comparison now explicitly load188. Parent aggregate2a9bd608-0b55-4cae-b4fa-b96c8d27cd90 subsequently passed10 tests/198 assertions, and the parent task completed.
Native NPR validation verified definition/model identity and the pinned framework package; assetDependencyHashesVerified remains false. No global all-pose collision proof, zero-warning claim or owner aesthetic acceptance is implied.


## Hosted Python compatibility
The first actual hosted recipe run used Python3.11 and exposed a serialization-only mismatch in the frozen AST contract: empty type_params metadata from Python3.12. The frozen fixture and protected source were not changed. The comparator now structurally ignores only that empty metadata field, retaining nonempty generics, function-body edits and literal text. Four regression cases cover those boundaries.106 pure tests pass locally. This test-only correction does not alter candidate188 geometry or its generation inputs. Hosted rerun status must be checked on the new commit.
