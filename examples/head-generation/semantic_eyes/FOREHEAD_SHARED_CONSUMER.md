# Opt-in shared forehead checkpoints

The normal generation path remains unchanged. `stage_forehead_retile` accepts
an explicit `shared_workflow` inside its existing forehead context. This is a
bounded component workflow, not a completed replacement for normal generation.

The context must also provide an owned `shared_checkpoint` sink. The sink writes
`forehead_consumer_checkpoint_bytes(checkpoint)` and returns `persisted: true`
with the exact bytes' SHA-256. Missing or mismatched receipts fail closed.
After checkpointing, `ForeheadSharedPending` unwinds detached native staging;
the original mesh must remain unchanged. Catching it does not authorize a model
commit. Every current result has `native_ready=False` and
`full_model_success=False`.

## Fresh and resume modes

- `fresh` captures the live source before allocation, distinguishes original raw
  geometry from the explicitly regularized author surface, initializes shared
  topology, and bootstraps working caps once. Optional bounded fairing, along-flow
  and transverse-gap stages then consume that proposal.
- `resume` requires a matching source binding, an unmirrored positive proposal
  and existing working caps. It never reruns initialization or refreezes original
  field/length references. Its restoration is explicitly an **optimizing** step,
  even with zero requested progression; movement and geometry hashes are recorded.
  All earlier geometry-dependent acceptance evidence is invalidated.
- Raw-to-author changed points and posed correspondence are retained separately.
  Source binding includes stable native UV, pin, attribute, color, deform and
  material-slot values, rather than session-specific pointers.

Fresh initialization or restoration can return a pending checkpoint. A checkpoint
without admitted working caps cannot be resumed as a cap-admitted proposal.
Native reconstruction must use the matching original source, not an already
retiled or mirrored export mesh.

## Execution budgets and publication

The optional `seam_wall_seconds` and `seam_initial_held_seconds` settings preserve
the default 120-second split when omitted. Extended settings require active-set
growth, a total no greater than 300 seconds, and a positive initial held budget
strictly below the total, reserving fallback time. They do not change iteration
limits, geometric constraints, or scalar/quantized admission gates.

A sink's filesystem receipt proves only local storage. Beaver callers must
declare the exact output file set, complete normal output publication, and verify
captured bytes before claiming cross-job recovery. Archive and verify temporary
history before removing redundant temporary copies, or declare them explicitly.
Do not bypass cancellation, cleanup, or publication protections.

## Remaining integration gates

Support recovery, complete independent bidirectional 1 mm checks, posed-state
coverage, final smoothness, bilateral transport, saved native/GLB readback and
current-engine/visual acceptance remain separate requirements. Shared helper
registration and a successful checkpoint do not establish these requirements.
The final native-ready path is deliberately unavailable until those contracts
are integrated and verified.
