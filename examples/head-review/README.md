# 664 head review and SDF controls

Latest accepted checkpoint: 2026-10-15. Candidate geometry 662 plus corrected SDF664. The source-only example includes no private/reference assets.

Copy showcase_extensions and showcase into an authorized project with the NPR package. Supply the candidate definition through the existing Beaver preview workflow (head_recovery_664/aster_definition.tres), or assign character_definition in the scene. No absent old candidate asset is loaded at scene import.

Reference comparison uses reference_definition_path. Configure your own authorized resource; if absent, the viewer reports this and keeps single-model review available. No reference asset is downloaded or bundled.

Controls include synchronized render/white/wireframe, zoom, full rotation, mouth deformation, per-5-degree eye capture, main light yaw, elevation, SDF toggle, black/white diagnostics, and 250-frame light capture. Frames are real viewport captures, not AI-generated illustrations. Capture completion restores the prior UI state and verifies source geometry identity.

The SDF implementation uses project-level extensions in sdf360, preserving pinned package integrity in existing workspaces. The upstream NPRCharacterFrame plugin also ships the matching optional material under shaders/face/sdf360. Do not substitute one without updating the definition resource path.

See ../head-generation/checkpoints/current/README.md for authorized baseline dependencies and ../head-generation/sdf360/README.md for native bake requests. The head-only scope does not claim full-wardrobe or general cast-shadow acceptance.

Historical implementation records remain in Git history; the current source is the 664 publication checkpoint. No task databases, credentials, imported reference textures, runtime logs or user-local absolute paths are part of this source delivery.
