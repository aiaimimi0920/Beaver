# Lower-profile verification — 2026-10-18

These numeric controls preserve the procedural generation path. They do not
replace the Face with a reference mesh. The source baseline still provides the
inherited non-face roles and is not distributed here. No third-party reference
images are included.

## CPU and native checks

The isolated experimental recipe passed 112 semantic-eye tests and 35
facial-surface tests. Tests cover default/range behavior, monotonic shoulder
coordinates and interval containment, unchanged upper/corner/center geometry,
pocket boundary radius, closure seam, and frozen/preflight helper consistency.
The final repository copy reran both suites after formatting: 112 and 35
tests passed. Black check, Python compile and git diff --check passed.
The local npm effective-line command was attempted but could not run because
Cargo is absent; repository CI must provide that gate. A local simple
nonblank/non-comment count found each changed Python file below 300 lines;
this is not represented as a substitute for the repository checker.

Candidates755,756,757,759,760 and761 completed native generation and matched SDF
baking. Each had 84 Godot captures with verified hashes, requested/actual pose
weights, and unchanged source hashes. Reviewed captures include neutral,
±45-degree yaw, pitch and progressive/complete blink. This is scoped visual
inspection, not exhaustive intersection proof or final aesthetic acceptance.

Candidate758 was rejected by the unchanged full-blink triangle-orientation
check (triangle1007, ratio -0.0305284422). It is not a valid preset. Candidate759
restored the lower arch to .8; all gates then passed. The reviewed support bound
remained .0042m throughout. Do not relax a guard to retain a preferred still.

## Retained numeric case

The supplied759 preset retains the user-reported relative improvement. Its
native generation job was c76818c5-1869-406e-bab0-deb48d924250; matched SDF job
c9f45d15-8a63-4d3c-8e03-3634f35e05c8. This records reproducibility evidence,
not a promise that these private-job identifiers are public download links.

760 tested a clearer upper arch;761 additionally tested heavier upper lash
weight. They remain visual candidates and do not replace759 in this preset.
The original750 aesthetic baseline also remains preserved outside the repo.

## Reproduction checks

- python -m unittest discover -s examples/head-generation/semantic_eyes -p 'test_*.py'
- python -m unittest discover -s examples/head-generation/facial_surface -p 'test_*.py'
- npm run check:effective-lines

SDF dependency discovery must retain the repository-relative NPR path. The
absolute path used in the isolated experiment is intentionally not published.
