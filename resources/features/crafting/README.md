# Crafting and recipes

`crafting.gd` works with an object implementing atomic `exchange(costs, rewards)`, such as Beaver's inventory module. No hard-coded source path or automatic dependency installation is required.

- Reuse the game's existing inventory if it supports atomic exchanges; otherwise explicitly add/adapt inventory logic as part of this task.
- Author original recipes and ingredient IDs. Define a recipe once, and call `craft()` on a confirmed action.
- Bind availability, success and failure to the existing crafting/bar UI, without consuming ingredients on failure.
- Preserve customized recipes when merging updates.
- Acceptance: successful recipe consumes all inputs and produces output; any missing ingredient leaves the entire inventory unchanged; repeated crafting cannot duplicate output for free.
