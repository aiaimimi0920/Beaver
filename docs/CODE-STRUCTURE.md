# Beaver code structure and ownership

The canonical [rule](../resources/instructions/code-structure.md) follows Hook's
effective-line thresholds: target 150, prefer 100-250, one responsibility at
251-500, a documented exception at 501-700, and mandatory splitting above 700.
Above 1500 is a hard violation for new or modified code. Blank and comment-only
lines are excluded. Inline comments do not remove code from the count. Strings
remain code; only actual leading Python docstrings are treated as documentation.

## Developing Beaver

Run these commands from the repository root in the normal PowerShell toolchain:

```powershell
rtk npm run check:effective-lines
rtk npm run test:effective-lines
rtk proxy cargo fmt --all -- --check
```

The normal check and native build both run the size gate. The gate needs Rust
for development; installed native Beaver contains the same engine and needs no
Node.js, npm, Git, Hook checkout or developer configuration to run it.

The explicit baseline in `scripts/effective-code-lines-baseline.json` freezes
unrelated existing oversized sources, including archived UI prototypes. Each
entry pins its effective count and SHA-256 after newline normalization. Editing
a comment, replacing code without increasing the count, or renaming a file
invalidates that allowance. The baseline is independent of Git because source
distributions also need the gate. Do not regenerate it as part of a build.
Retire entries when their files are split; adding debt needs an explicit policy
decision. `check:effective-lines:strict` audits all debt and is expected to fail
until the remaining historical files are migrated.

The scanner covers handwritten sources throughout the checkout, including
tests, scripts, resources and executable documentation examples. It skips build
and task caches (`.git`, `.godot`, `.beaver`, `.beaver-context`, `node_modules`,
`target`, `release`, `exports`, and `.beaver-write-*`). Repository-only root
exclusions are `output`, `dist`, `dist-native`, `.codex` and `.playwright-cli`.
Game scans do not exclude an arbitrary `output` folder. Scene/resource data,
media, Markdown prose, manifests and lock files are not executable source.
Keep scripts external to serialized Godot scenes rather than embedding them.

The RoleNPR package is a pinned dependency: only exact paths and byte hashes in
its bundled provenance manifest are exempt. Newly added or modified files in
`addons/npr_characters` are checked normally. No broad `addons` or `vendor`
exclusion exists. GUT 9.4.0 may be installed as a project development dependency
at `addons/gut`: the checker derives exact file paths and byte hashes from the
compiled-in `resources/validation/gut-9.4.0.zip`, after verifying its pinned
SHA-256. Unknown files, changed bytes and alternate paths follow ordinary limits;
a project manifest cannot expand this list. The archive and its MIT/font licenses
remain unchanged. Other immutable dependencies require an explicit reviewed
exclusion; a filename or a generated-code comment cannot grant one.

## Module ownership

The native desktop entry point uses these ownership boundaries:

- `backend.rs`: shared backend state and durable API call logging.
- `business_routing.rs`: asynchronous operation dispatch and change notification.
- `task_control.rs`: answers, continuation, interruption and scheduler ordering.
- `asset_runtime.rs`: capture/import operations and their resource locks.
- `data_dispatch.rs`: operations performed while holding the store lock.
- `asset_protocol.rs`: local asset HTTP protocol and origin/range handling.
- `shell.rs`: startup, window/tray lifecycle and orderly shutdown.
- `main.rs`: executable modes and module wiring.

The checker is split into language recognition, lexical state, string handling,
policy, scanning, task snapshots, CLI and MCP presentation. The policy engine
is shared by the developer command and the installed application. When several
agents work concurrently, assign one of these scopes to each editor and have one
owner update shared interfaces and run integration checks. Read-only scouts may
locate callers without taking edit ownership.

## Installed Beaver game tasks

Native Beaver embeds the canonical rule in the executable. Every task HOME gets
it in `AGENTS.md`; both `thread/start` and `thread/resume` also receive it through
Codex `developerInstructions`. Existing user/project instructions and personal
Codex configuration are preserved. The Electron instruction generator reads the
same bundled resource and supplies the same new/resumed-thread instructions.

Native tasks receive the read-only `beaver_check_code_structure` MCP tool. Its
baseline is built from the original task's source blobs, so unchanged old code
does not block unrelated work. Use it before finishing, then resolve failures.
The native finalizer checks captured changed source blobs again before any
project merge. It does not reread large assets, recapture the project, or trust
the AI's report or a task-editable baseline. Merge retry validates recorded blobs
again; editing the live workspace cannot change the recorded result. Failed
checks preserve the workspace and report affected paths through the task error.
Use ordinary task continuation to split and verify the result.

For a cohesive 501-700 line source, `.beaver-code-structure.json` may contain:

```json
{
  "schemaVersion": 1,
  "files": {
    "scripts/parser.gd": {
      "effectiveLines": 540,
      "sha256": "copy the current canonical hash from the checker report",
      "responsibility": "One parser state machine",
      "reason": "Explain why splitting would harm cohesion or verification",
      "tests": ["the actual protective parser test command"]
    }
  }
}
```

Use actual evidence, not the example placeholders. Missing, stale, oversized or
incomplete exceptions fail. Splitting is the default. This adapts Hook's rule
without requiring unattended game tasks to invent a separate human approver.
