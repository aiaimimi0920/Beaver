# Beaver versions and development checks

Development work uses focused functional tests for the changed behavior and its
direct dependencies, with relevant compilation, formatting and effective-line
checks. Full application/game-creation acceptance runs only for a formal external
release or an explicit user request. A development build or package command does
not request a full acceptance round.

## Version source

`package.json` is the source of product versions:

- `version: "0.1.19"` is the public three-part base, `X.X.X`.
- `beaverBuild: 5` is the internal iteration. The current development product
  version is `0.1.19.5`, following `X.X.X.N`.
- Increment `beaverBuild` for the next internal delivery: `0.1.19.6`,
  `0.1.19.7`, etc. Builds and tests never increment it automatically.
- Change the public base only as an intentional release-version decision.
  Synchronize `package.json`, both first-party Cargo manifests,
  `native/desktop/tauri.conf.json`, `package-lock.json` and `Cargo.lock` when
  changing that base; reset the internal iteration to 1 for the new base.

npm, Cargo and Tauri require SemVer, so their package metadata retains the
three-part base. The product version is recorded in build metadata, package
directory names, `RELEASE.json` and Windows executable version strings. Windows
fixed numeric version fields always contain four components: development uses
`X.X.X.N`; an external release uses `X.X.X.0`, with `X.X.X` display strings.
All components must fit Windows version fields (0-65535); internal iterations
start at 1. Protocol client/server identities keep their existing package version
semantics.

## Commands

The default channel is `development`. In a shell without `BEAVER_CHANNEL` set:

```powershell
npm run build:native
npm run package:native
# Default destination: release/Beaver-native-0.1.19.5-win32-x64
```

`build:native` stamps the newly compiled Windows executable. A bare `cargo build`
retains Cargo/Tauri package metadata and is sufficient for compilation checks;
use `npm run build:native` to produce a versioned development executable.
Packaging verifies that the frontend metadata and executable match the selected
version/channel before creating a new release directory. An existing directory
is never overwritten; increment the internal iteration for a new delivery.

For a deliberately selected external release, after the required acceptance:

```powershell
$previousChannel = $env:BEAVER_CHANNEL
try {
  $env:BEAVER_CHANNEL = "release"
  npm run package:native
  # Default destination: release/Beaver-native-0.1.19-win32-x64
} finally {
  $env:BEAVER_CHANNEL = $previousChannel
}
```

Selecting `release` changes version formatting; it does not publish, sign, run
acceptance, or mark runtime verification complete. Switching channels requires
building again. Historical package directories and acceptance records keep their
original versions.

The reachable Electron prototype commands (`npm run build`, `npm run package`)
use the same product version. Run its standalone verifier with
`npx tsx scripts/verify-release.mjs <release-directory>`; build/package scripts
also use the existing `tsx` dependency to share the TypeScript version logic.

For versioning changes, the focused checks are:

```powershell
npx tsx --test tests/product-version.test.ts tests/native-release.test.ts
npm run typecheck
npm run check:effective-lines
```

Run Prettier on changed JavaScript/TypeScript/JSON/Markdown files. Other changes
should choose their own related functional tests rather than use this list as a
new mandatory test suite.
