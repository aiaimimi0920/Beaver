# Restore the verified 185 preview

This is a recovery checkpoint, not a new model generation or a visual acceptance.
The original asset remains byte-identical. The private reference asset is not
included in this repository.

## Required inputs

Use the authorized private source checkpoint and verified preview runtime archive.
The source checkpoint SHA-256 is
`2ab2939529b3c4c5869db595e4538c7b58ef129840fad7e683cadde786db45d2`.
The runtime archive SHA-256 is
`68b25e9cec8b20862cf42a6fde11ec396b4ca46c1d803e25d2b16970100a803d`.
Verify every runtime archive manifest entry before installation. In particular,
the MiDot executable must match
`888d345cda807f7dc6fae1a9aad6fbd256736e086a34d37819a39c19d311c5dd`.
Do not substitute a same-version executable with another hash.

The direct workbench expects its repository beside `restored-tools`,
`restored-projects`, and `recovered185`. Place the approved source checkpoint at
`recovered185/Aster-head-recovery-source-checkpoint.zip`.
Restore the saved Electron 44.2.0 distribution and pinned software Vulkan runtime.
Keep Electron sandboxing, task identity checks, and the preview executable hash
check enabled.

## Recovery sequence

1. Launch the existing direct workbench through the native desktop launcher.
   A shell script path may open in the text editor; explicitly invoke `sh` with
   the startup script from the launcher when necessary.
2. Use the workbench's recovery action. It creates/registers a fresh project and
   refuses to overwrite a target with task history. Restore model files and the
   exact pinned framework, including its `.ci_script` tools. Exclude old task
   databases, import caches, repository metadata, and credential directories.
3. Use a fresh Beaver external-agent display session. Do not restore historical
   run IDs. The same operator handles planning and execution; no model provider
   or Codex invocation is needed for the saved display.
4. Validate `assets/aster/head_recovery_185/aster_definition.tres`, then use the
   native preview button. The workflow rebuilds import caches from source.
5. Open model comparison, confirm candidate 185 and reference 900, and select a
   useful framing. Confirm the actual foreground interactive window, not merely
   a process start or an old screenshot.

For file-picker inputs, use a filesystem location shared with the desktop.
An executor-local `/tmp` file may be absent from the desktop. A selected filename
alone does not prove its contents loaded; check the populated fields.

## Evidence from 2026-10-07

- Native workflow validation passed at 15:53:36 UTC with no engine errors.
- Model SHA-256:
  `cc1831d51f5d8e7425054a92b98f90785d827506a293c150a730df6e547e21e3`.
- Definition and model hashes, plus framework package hashes, were verified.
  `assetDependencyHashesVerified` remained false; do not claim all dependency
  identities were verified by this workflow.
- Actual comparison display confirmed at 15:55 UTC: candidate 185/reference 900,
  front rendered view, framing 0.99, mouth 0, hair/body hidden.
- Restore helper: six focused tests passed for resource fidelity, excluded state,
  traversal rejection, archive symlinks, destination symlinks, and target checks.
- Preview guard: seven existing tests passed. JavaScript syntax check passed.
- Modified source files have 104, 45, and 68 nonblank/non-comment lines. The full
  Cargo-backed repository effective-line command was not rerun in this restored
  minimal workbench environment.

The final restore helper includes preflight path checks beyond the initial
successful recovery. Those checks were verified with synthetic archive tests;
the live project was not overwritten to retest them. Owner aesthetic acceptance,
PR merge, release, and production deployment are not established by this record.
