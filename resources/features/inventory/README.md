# Inventory module

Copy/adapt `inventory.gd` into the project's own module directory. This is tested game-state logic, not an installed inventory screen.

- Store quantities by stable string item IDs; names, art and stack limits belong to the game.
- `exchange(costs, rewards)` validates all inputs before any mutation, so crafting cannot partially consume ingredients.
- Bind `changed` to the existing HUD; persist `snapshot()` through the project's save system.
- Preserve existing inventory data and custom item IDs during upgrades. Do not create a second authoritative inventory if one already exists.
- Acceptance: pick up, consume, reject insufficient quantities, craft atomically, save and reload; verify visible behavior in the running game.
