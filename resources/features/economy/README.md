# Wallet and transaction foundation

`wallet.gd` manages integer in-game currencies. This is not real-money billing, a shop UI or a marketplace.

- Keep one authoritative balance per currency. Merge with an existing economy instead of duplicating it.
- Check inventory capacity and all purchase prerequisites before `spend()`; make item delivery and spending atomic in the project controller or compensate failed delivery.
- Persist `snapshot()` using the existing save system; display prices and balances through localization.
- Acceptance: income, successful purchase, insufficient funds and negative input rejection, no duplicate click rewards, and save/reload.
