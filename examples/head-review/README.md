# 664 head review and SDF controls

Latest accepted checkpoint: 2026-10-15. Candidate geometry 662 plus corrected SDF664. The source-only example includes no private/reference assets.

Copy showcase_extensions and showcase into an authorized project with the NPR package. Supply the candidate definition through the existing Beaver preview workflow (head_recovery_664/aster_definition.tres), or assign character_definition in the scene. No absent old candidate asset is loaded at scene import.

Reference comparison uses reference_definition_path. Configure your own authorized resource; if absent, the viewer reports this and keeps single-model review available. No reference asset is downloaded or bundled.

Controls include synchronized render/white/wireframe, zoom, full rotation, mouth deformation, per-5-degree eye capture, main light yaw, elevation, SDF toggle, black/white diagnostics, and 250-frame light capture. Frames are real viewport captures, not AI-generated illustrations. Capture completion restores the prior UI state and verifies source geometry identity.

The SDF implementation uses project-level extensions in sdf360, preserving pinned package integrity in existing workspaces. The upstream NPRCharacterFrame plugin also ships the matching optional material under shaders/face/sdf360. Do not substitute one without updating the definition resource path.

See ../head-generation/checkpoints/current/README.md for authorized baseline dependencies and ../head-generation/sdf360/README.md for native bake requests. The head-only scope does not claim full-wardrobe or general cast-shadow acceptance.

Historical implementation records remain in Git history; the current source is the 664 publication checkpoint. No task databases, credentials, imported reference textures, runtime logs or user-local absolute paths are part of this source delivery.

## Single-model close-up review

The default orthographic viewer now changes camera size on mouse-wheel zoom,
keeping the picked surface under the cursor. The range is0.5–6× the initial
head framing. Reset View restores1×; perspective mode keeps its existing zoom
behavior. Model geometry, materials and definition resources are not changed.

`tests/head_zoom_probe.gd` is a temporary native GUI regression harness extending
the ordinary viewer. In an authorized test workspace, point a copy of the review
scene at this script, open through the existing Beaver preview, and inspect
`HEAD_ZOOM_PROBE` in the owned preview log. Restore the ordinary scene script
afterward. It checks actual camera projection, picked-surface anchoring, clamps
and perspective fallback; source parsing alone does not establish those results.

## Directional SDF control verification

Comparison controls detect the actual face shader's enabled/debug uniforms,
so both the project-level and upstream package SDF material paths are supported.
The light sweep records both applied material states for each frame and fails
if the candidate lacks SDF controls or a capable material cannot apply the
requested state. Unsupported references are explicitly recorded, without
claiming a paired SDF diagnostic. A UI toggle alone is not
proof of a changed render. Plain materials without these uniforms remain
untouched and cannot pass the directional-SDF sweep.

`tests/head_sdf_controls_probe.gd` is a temporary native GUI regression harness.
Use an authorized candidate and reference definition with bound SDF atlases,
point a temporary review scene at the probe, and open it through Beaver. It
checks shader capabilities, actual uniform round trips, active materials,
unsupported references, input locking and cancellation. Restore the ordinary
scene afterward. The retained observation records69 native checks and a separate
250-frame light sweep with500 actual material-state records. Cancelled captures
are not counted as completed sweeps.

## Optional eyelid review controls

Candidates with both `blink.L` and `blink.R` expose independent [0,1] sliders and
an eye-only reset button in the comparison panel. The reference remains neutral;
`MouthOpen` remains independent. Assets missing either blink shape show no blink
controls and do not receive implicit blink writes. These controls modify only
preview-instance weights, never the source model.

F12 and both automated sweeps record actual candidate/reference morph weights
alongside requested state. Capture locks include these sliders; cancellation
restores their prior values and labels. F12 reports metadata-write failure rather
than presenting an incomplete evidence pair as success.

`tests/blink_controls_probe.gd` is an explicit native GUI regression, not the
ordinary scene. The retained observation records 135 checks on the primary
three-morph asset and six on a legacy MouthOpen-only asset, with no failures.
It covers real capture/cancel/restore and persisted weight metadata, in addition
to render/white/wire synchronization and source identity. A separate supported
variation passed 127 earlier checks. Manual slider clicks and reset were also
captured. Restore the ordinary head-review scene after running the probe.
