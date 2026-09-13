---
name: godot-production
description: Create, modify, inspect, validate and export the user's Godot game inside the Beaver task workspace. Use for any game-production task.
---

# Godot production

You are the executor, not the user. Inspect project.godot, scenes and existing scripts before modifying them. Preserve imported projects and existing game behavior. Decompose the goal yourself; do not require a user-authored workflow graph.

- Work in the provided task workspace only. It is a copy; Beaver merges changes into the original after completion. Do not modify the original or Beaver state.
- The task carries project-relative references and optional normalized image regions. Inspect the actual files before making visual judgments. Use selected references consistently.
- Use existing Godot CLI tools or the enabled Godot MCP. Check executable version before assuming support. Godot and Blender tool paths are supplied in AGENTS.md.
- Use GDScript for portable templates. Validate with a headless editor import and a bounded launch. Run script quality tools when available. Clean up only test processes you started.
- Use beaver_media tools for configured image, music, speech and translation services. Missing provider configuration is a blocker, not permission to report fake generated media. Image edits support up to five reference files. Write generated files to new project-relative paths before replacing valuable originals.
- Use Blender yourself when modeling is needed; export glTF/GLB with required textures. Integrate the result into Godot scenes rather than delivering only loose models when the goal is an in-game feature.
- Export a runnable game only when export templates and the target preset are available. Place exports outside the source tree or under exports/. Verify the actual output exists and launches; an editor project is not a runnable game build.
- Honor stop conditions and limits. Report actual changes, checks, output locations and remaining failures in Chinese. AI completion is not user acceptance.
- For feature updates, compare .beaver-context/feature/old and new, then adapt changes into customized project code. Do not overwrite a locally modified feature wholesale.
- For dialogue rollback, read the provided before/after files and change manifest. Preserve unrelated later changes and explicitly requested retained outputs.
