# JSON save slots

`save_slot.gd` is a starter adapter, not a complete save UX. Preserve the project's existing save locations and data model.

- `save_data(data, slot)` writes a JSON dictionary to `user://save_<slot>.json`; check its returned Error before reporting success.
- `load_data(slot)` returns an empty dictionary for absent or invalid data. Let the game distinguish a new game from a corrupt save when that distinction matters.
- Define a versioned save schema and explicit migrations before integrating additional modules. For production durability, adapt writing to use a temporary file and atomic replacement.
- Acceptance: save and load the actual game state, isolate slot numbers, preserve old player data, and show a truthful failure when storage is unavailable.
