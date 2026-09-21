# Object preview interaction

## Current UI build: 0.1.19.15

The object gallery still uses demo objects and SVG artwork. No gallery object
has a real Godot project path, Blender file, or engine session binding. This
iteration implements selection and feedback UI; it does not implement engine
rendering, camera control, or real task submission.

The gallery footer explaining object organization is removed. Both galleries
retain rectangular cards, type-colored borders, and the expanded 3:2 division
between the main gallery and the child-content drawer.

The toolbar provides point selection, rectangular region selection, and a larger
observation dialog. Selection coordinates are normalized to the preview image,
with reverse drags normalized to top-left and bottom-right. A click in box mode
falls back to a point. Keyboard activation selects the whole item. Selection
overlays are interaction marks; the previous decorative card labels stay absent.

The feedback bar identifies the object, component or reference, version, intended
preview source, and selected image coordinates. Submission creates an on-screen
demo receipt only. Changing the target or preview source resets the prompt. A
reference keeps its pinned version in feedback, and double-click or Enter opens
that referenced object. Task-rail navigation clears the previous feedback target.

## Intended engine sources

- Main object: Godot game rendering, using the object's imported scene, materials,
  lighting, and configured acceptance camera.
- Model component or model reference: Blender production view by default, with
  an optional Godot view to inspect engine-specific materials.
- Image component: original image. Other components use their suitable viewer;
  the current demo has no real document or resource binding.

The observation dialog shows these intended source choices, explicitly states
that the artwork is a demo, and disables camera controls until an engine is
connected. Selecting Godot or Blender does not start either application.

## Real integration requirements

Resource bindings must identify the project, object revision, component or
reference revision, engine, and resource path. Real previews must report source
provenance rather than accepting a source label from the client as proof.

The existing Blender Observer is a reusable implementation boundary:
`native/core/src/asset_preview.rs`, `native/core/src/blender_session.rs`,
`resources/workflows/asset_observer*.py`, and `src/ui/asset-task/use-observer.ts`.
It is task-scoped today; gallery previews need object-scoped ownership and
lifecycle handling before they can use it safely.

Godot currently provides external game playback and validation captures. A live
gallery service still needs scene loading, camera commands, frame transport,
resource cleanup, and rendering of the selected object's real game context.
Opening an external game window or displaying SVG artwork does not satisfy that
contract.

When connected, camera operations should orbit, pan, zoom, and restore the
acceptance camera without silently changing the saved game scene. Point and box
feedback must freeze a frame together with session, generation, object revision,
camera revision, image dimensions, and normalized selection coordinates.
Two-dimensional marks must not be represented as mesh or topology selections;
actual 3D picking needs engine-provided hits.

Real feedback submissions must enter the object task queue with a base revision
and idempotency key. The single-writer policy in
`OBJECT-CREATION-FRAMEWORK-DESIGN.md` remains applicable. Demo object identifiers
must never be sent to the existing production task API.
