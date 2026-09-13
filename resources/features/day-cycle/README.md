# In-game clock

`day_cycle.gd` tracks deterministic game minutes, not wall-clock time. Codex chooses whether a game action or an owned Timer advances it.

- Use `paused` for menus/dialogues when the design requires time to stop.
- Listen to `changed` and compare day/time transitions to trigger schedules exactly once; do not re-trigger events every frame within a phase.
- Persist the clock alongside event completion flags. Avoid advancing offline time unless explicitly requested.
- Acceptance: midnight crosses to the next day, phase boundaries are correct, paused time remains unchanged, and existing narrative pacing is preserved.
