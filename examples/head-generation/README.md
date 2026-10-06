# Reproducible head recipe, candidate147

This is the five-feature refinement following the owner's145 review: ears, eyes, eyebrows, a local frontal contour concavity, and nasal ramp slope. Other accepted form, center-face division, open wrapped ear construction and small chin follow are regression targets.

Workflow: natural-language specification → explicit semantic parameters → Beaver generation → actual MiDot/NPR comparison. The generator does not read reference mesh, UVs or pixels. Read-only sparse reference diagnostics remain separate.

146 adjusted the nasal ramp without moving its matched tip, eyebrow center/thickness, eye corner/pupil shape, a continuous lash wing instead of loose triangles, and the original ear cup/pigment.147 fixes the cause of an extra frontal pinch: previous SIDE-jaw fairing also moved lateral x. The new separate lateral relaxation defaults to zero, preserving front width while retaining depth/height fairing. Modest width support and a short procedural ear fold fork complete this candidate.

62 pure Python tests pass,13/13 generated input hashes match, and native MiDot validation passes. The model has actually run, with front, right90, both45-degree render, oblique white, rear runtime wireframe and MouthOpen0/.5/1 inspected. User aesthetic confirmation remains pending. Body/Hair and pinned MiDot/NPR1.3.0 are unchanged.

Project aggregate cf22a325-4f6c-450a-8625-9e8ba5d75e1b passed 7 tests with 0 failures at 2026-10-06T18:57:38UTC. Details are recorded in review-results.md. Keep PR draft until all applicable gates are satisfied. No release, production deployment or full-character/wardrobe acceptance is claimed. Run unittest discovery in this directory; bundle.py only packages a request for Beaver blender.start, never runs Blender itself.
