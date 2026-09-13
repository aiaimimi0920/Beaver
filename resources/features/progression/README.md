# Progression module

`progression.gd` supplies a small deterministic level curve. The curve, cap, rewards and UI are game-specific and must be adapted rather than treated as a finished RPG system.

- Grant experience from completed game actions. Handle each `level_changed` once to unlock rewards.
- Support a single grant crossing multiple levels, and stop at the level cap.
- Persist the level and remaining experience together. Do not reset existing player progress when adopting a new curve; write an explicit save migration.
- Acceptance: threshold crossing, multi-level gain, invalid grants and level cap; show the result in the actual game.
