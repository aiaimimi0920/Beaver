# Upper ear attachment and nose contour checkpoint520

Source-only incremental repair,2026-10-13. Not final visual acceptance. No third-party reference mesh or texture included.

## Reproduction
Requires existing Beaver project, private assets/aster/head_recovery_49/aster_head_editable.blend, authoring/head_generation_guide.md and NPR runtime dependencies. Not standalone. Use bundle.py --output request.json through native Beaver blender.start, then NPR validation and actual preview. Editable blend is intermediate; runtime_morph blend and GLB contain final post-join changes.

## Change and diagnosis
519 repairs the annotated upper ear attachment notch. The old recipe connected the lower return to adjacent facial boundary but omitted the equivalent upper bridge. Add one triangle per side using existing evaluated upper-root, upper-neighbor and return-rim positions. Do not close the entire intentionally open ear basin. Topology verification maps vertices within1micrometre: every prior triangle retained and exactly2 added; each new triangle has2 manifold shared attachment edges and1 intentional free contour edge. Other positions effectively unchanged (max0.249micrometre).
520 retains that repair and adjusts only nasal stations. Peak lower shoulder .280/.0945 -> .286/.099; upper shoulder .300/.095 -> .303/.095. Lower .250 relief .055 -> .050 and .220 .016 -> .014. These raise/sharpen the tip slightly and retract the lower transition; lip parameters, eyes and eyelashes are unchanged.

## Verification
519:283 pure tests passed17.414s; native job f4207ab1-7bd8-4f90-9147-e7ba6015050e exit0,43192ms,32 hashes verified.520:284 tests passed19.327s; job6abb1ca0-b153-478d-8639-befc8bc48b40 exit0,51416ms,32 hashes verified. Native NPR validation and actual GUI520 preview succeeded. Actual519->520 nose-region maximum nearest vertex displacement1.0497mm; outside a30mm half-width nose/subdivision support envelope under1micrometre, body/hair unchanged. Native shared-root and collapsed/inverted face checks passed. Ear source241 effective lines; pipeline455. Legacy unchanged oversized modules remain technical debt.

## Visual review and limitations
Rendered comparison at0,+/-45,+/-90,+/-135,180 degrees, plus right90 and135 white/wire checks. Upper attachment notch now has a bridge in wire view with no observed new black seam at examined angles. Nose underside transition is closer at side90; pixel samples are diagnostic only. Ear shape/texture and existing cheek white-mode shading differ from reference; broader aesthetic equality has not been achieved. Draft PR remains unmerged; CI success is not zero findings or product acceptance.
