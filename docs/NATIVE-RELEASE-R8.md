# R8: shared business API and MCP

Date: 2026-09-09. This is the first executable delivery of the agreed human- and
AI-friendly business interface. It is still an unsigned native integration preview.

## Delivered

- Desktop IPC, authenticated local HTTP and stdio MCP enter the same native
  `business_call` implementation, store and scheduler.
- 44 existing business operations are discoverable with descriptions and input
  schemas. Planning schemas are generated from the existing shared definitions.
- `asset.import` accepts explicit file paths and `game.importTemplates` accepts
  an explicit archive path; external clients never wait for file pickers.
- `GET /v1/capabilities` and `POST /v1/call` bind only to 127.0.0.1 when an explicit
  API token is configured. Missing/wrong tokens and browser origins are rejected.
- `Beaver.exe --business-mcp` connects to the running host without opening another
  database. Discovery, structured tool results and business errors are implemented.
- Body/concurrency limits, shutdown admission checks and outstanding-call tracking
  are in place. Disconnection does not drop an accepted transition midway.

Usage and the exact contract are in [BUSINESS-API.md](BUSINESS-API.md).

## Actual packaged artifact

`release/Beaver-native-0.1.19-preview-r8-win32-x64/Beaver.exe`

- Complete payload: 606 files, 15,664,213 bytes, below the 50,000,000-byte limit.
- EXE: 12,051,968 bytes.
- EXE SHA256: `7eef1705b8c5fdbf9e74aaa276c70e4aaca6ebfb093af6afb465bc18a8489999`.
- Release compilation: 7 minutes 26 seconds.
- Fresh post-runtime full payload/hash verification passed.
- Previous releases and production data were not overwritten.

## Fresh verification

1. `cargo test --locked -p beaver-desktop`: 3 passed, including missing/invalid
   credentials, browser-origin rejection and noninteractive import validation.
2. `cargo fmt --all -- --check`: passed.
3. `npm run typecheck`: passed.
4. Prettier checks on changed TypeScript build/test scripts and package.json: passed.
5. Actual packaged EXE API/MCP smoke: 11 checks passed. Evidence:
   `output/validation/native-business-1788911337618/proof.json`.
   This created a real Godot project, edited real Markdown files via HTTP and MCP,
   rejected stale revisions and path traversal, imported an explicit file, and
   checked both transports see the same stored project.
6. Actual packaged EXE Tauri/WebView2 regression smoke: 41 checks passed, zero
   page errors. Evidence:
   `output/validation/native-shell-1788911351410/proof.json`.
   This includes the mounted desktop, document editing, settings and credential
   handling, native scheduler task execution, interrupt/resume/steering,
   clarification, feature integration and tool/template setup paths.
7. Scoped residual Beaver and test-owned Node/Codex/Godot process count: zero.
8. Modified source/build files inspected for UTF-8 BOM: none found.

The desktop regression uses controlled protocol fixtures for creative execution;
the API/MCP smoke does not invoke a live model. These results do not establish
creative quality, paid-provider reliability, or separate API coverage of every
one of the 44 declared operations. Godot export runtime acceptance was not rerun
in this milestone. The previous migration-specific proofs remain historical.

## Remaining acceptance

- Human-facing integration settings and token lifecycle UI.
- Per-project/capability grants and caller provenance/audit records. The current
  token intentionally grants full local-owner business access.
- Durable request deduplication, generic long-operation IDs, incremental events,
  stronger typed output contracts and finer domain error codes.
- Shutdown and cancellation stress tests under concurrent API load.
- Full real-provider creative workflow through external callers and mixed
  human/AI interactions.
- Migration import UI, complete rollback acceptance, platform billing, clean
  Windows installation/signing/update/uninstall and cross-platform release work.

`RELEASE.json` keeps `runtimeVerified: false` and `migrationComplete: false`:
the specific checks above are evidenced, but the complete formal product gates
are not closed. The default Electron packaging command is unchanged.
