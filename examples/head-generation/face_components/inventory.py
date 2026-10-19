"""Separate morphology families from selectable within-family variations."""

import json
from compose import HERE, SLOTS


def family_inventory(catalog=None, registry=None):
    catalog = (
        json.loads((HERE / "catalog.json").read_text()) if catalog is None else catalog
    )
    registry = (
        json.loads((HERE / "families.json").read_text())
        if registry is None
        else registry
    )
    if set(registry.get("slots", {})) != set(SLOTS):
        raise ValueError("Family registry requires exactly six slots")
    result = {}
    statuses = {
        "implemented_with_historical_samples",
        "native_validation_pending",
        "planned",
    }
    for slot in SLOTS:
        entries = registry["slots"][slot]
        assigned = []
        for family_id, family in entries.items():
            if family.get("status") not in statuses or not family.get("sources"):
                raise ValueError(
                    "Family needs an explicit status and sources: " + family_id
                )
            presets = family.get("presets")
            if not isinstance(presets, list) or any(
                not isinstance(p, str) for p in presets
            ):
                raise ValueError("Family preset IDs must be strings")
            if family["status"] == "planned" and presets:
                raise ValueError("Planned family cannot expose executable presets")
            if family["status"] != "planned" and not presets:
                raise ValueError("Implemented family needs executable preset")
            assigned.extend(presets)
        if len(assigned) != len(set(assigned)) or set(assigned) != set(catalog[slot]):
            raise ValueError(
                "Every selectable preset must belong to exactly one family: " + slot
            )
        result[slot] = {
            "selectable_options": len(catalog[slot]),
            "implemented_families": sum(
                f["status"] == "implemented_with_historical_samples"
                for f in entries.values()
            ),
            "pending_native_families": sum(
                f["status"] == "native_validation_pending" for f in entries.values()
            ),
            "planned_families": sum(f["status"] == "planned" for f in entries.values()),
            "families": entries,
        }
    return result
