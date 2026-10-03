# Beaver code structure

Apply this rule to handwritten production code, tests, scripts and styles,
including GDScript, shaders, Rust and TypeScript. Keep source files UTF-8 without
BOM. Blank lines and comment-only lines do not count as effective code lines;
code followed by a comment still counts. Multiline strings containing code or
data count as code, not as comments.

- Aim for about 150 effective lines per file; prefer 100-250.
- At 251-500 lines, keep one clear responsibility and consider the next split.
- At 501-700 lines, split by default. A temporary exception must identify the
  responsibility, why a split would harm cohesion or verification, and the
  protective test commands. Pin it to the current effective count and SHA-256.
- New or modified files above 700 lines must be split before completion.
  Files above 1500 lines are hard violations; they cannot receive exceptions.
- Unchanged historical oversized files may remain as visible debt. Changing
  their contents, even without increasing their length, requires applying the
  current limits. Do not refresh a baseline to make a failing change pass.

Split around responsibilities, dependency direction, state ownership and
resource lifetime. For games, separate scene assembly, player movement,
interaction, enemies, UI and persistence when those responsibilities exist.
Keep scene paths, signals and resource references valid during extraction.
Do not create empty layers or split a small cohesive file just to hit 150.

Before editing, inspect the existing module and its callers. Before multiple
agents edit concurrently, assign disjoint file ownership and agree on their
interfaces. Keep one owner for shared orchestration and integration. Name each
new module for its responsibility; do not accumulate unrelated code in
common, utils or helpers. Do not evade the rule with minification, long lines,
renames, extension changes, embedded scripts or code hidden in strings.

Generated build output and immutable third-party sources require explicit
exclusions. A folder named addons, vendor or generated is not an exemption.
Beaver's bundled NPR package and pinned GUT 9.4.0 development dependency are
exempt only at their published paths with exact upstream content hashes. GUT
identities come from Beaver's compiled-in, checksum-verified validation archive
and apply only under addons/gut. Modified package sources follow the ordinary
limits; project-owned manifests cannot grant exclusions.
Godot .tscn/.tres files are serialized scene/resource data; keep handwritten
scripts in separate source files.

Preserve behavior while extracting. Run the relevant parser/compiler,
formatter and focused behavioral checks, then the code-structure check. Report
the measured counts, module responsibilities and actual verification results.
Do not declare the work complete while a changed file still violates the rule.

In a Beaver game task, check source sizes before completion and fix violations.
Use beaver_check_code_structure when supplied. Native Beaver also checks captured output before merging it into
the original project, including merge retries. A failed check preserves the
task workspace; continue the task to correct it. Do not edit application state,
task baselines or personal Codex configuration to bypass the check.

For the rare 501-700 line exception, create .beaver-code-structure.json at the
project root with schemaVersion 1 and a files object keyed by relative source
path. Each entry needs effectiveLines, sha256 (the checker's canonical source
hash), responsibility, reason and a nonempty tests array of protective commands.
Changing the source invalidates the exception. Above 700 lines no exception is
accepted. Prefer splitting over adding an exception.
