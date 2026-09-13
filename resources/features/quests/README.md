# Quest journal module

Adapt `quest_journal.gd` to the game's existing event flow. It tracks counts and one-time claim eligibility, not authored quest content or reward delivery.

- Register stable quest IDs and positive targets. Advance from actual game events, not every frame.
- `claim()` must succeed before granting a reward. If granting can fail, coordinate journal and reward state transactionally in the game's controller.
- Bind `changed` to an objective panel, dialogue conditions or chapter progression.
- Save `snapshot()` with the game's save schema; preserve claimed flags when migrating old saves.
- Acceptance: incomplete claim fails, completion clamps to target, the second claim fails, and the displayed objective matches the state.
