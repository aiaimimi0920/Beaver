# Character relationships

`relationships.gd` provides bounded relationship values and stable stage IDs. Character writing and thresholds may be customized by Codex; the module does not generate narrative.

- Use stable character IDs rather than translated names. Connect authored decisions to `adjust()`.
- Adapt thresholds to the game, retaining migrated values and existing dialogue conditions.
- Use `snapshot()` / `restore()` for persistence, and `changed` to refresh relevant UI.
- Acceptance: correct character changes once per choice, values stay between -100 and 100, branches follow the expected stage, and save/reload preserves state.
