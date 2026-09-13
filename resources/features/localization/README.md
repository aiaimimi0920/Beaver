# Localization foundation

`localization.gd` resolves stable text IDs with fallback and named placeholders; `translations.json` contains original sample strings, not a complete translation pack.

- Load the table into `tables`, choose a supported locale and bind `changed` to refresh visible text.
- If the project already uses Godot TranslationServer / CSV / PO, convert these semantics into that system instead of maintaining a second localization authority.
- Keep IDs stable and preserve untranslated keys and placeholders during upgrades.
- Check CJK font coverage and layout in each supported language. Use the configured translation AI only when requested, not merely because the module is added.
- Acceptance: language switch, fallback, missing-key visibility and placeholder replacement; verify rendered text at the minimum window size.
