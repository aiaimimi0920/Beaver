"""Bind explicit whole-model examples; this is not visual acceptance or composition."""

import hashlib
import json
from pathlib import Path

SLOTS = {"eyes", "brows", "nose", "mouth", "ears", "face"}


def build_catalog(workspace, records):
    root = Path(workspace).resolve()
    result, ids, candidates = [], set(), set()
    if not isinstance(records, list) or not records:
        raise ValueError("An explicit nonempty combination list is required")
    for record in records:
        if not isinstance(record, dict) or set(record) != {"id", "label", "candidate", "selection"}:
            raise ValueError("Combination record schema mismatch")
        for key in ("id", "label"):
            if not isinstance(record[key], str) or not record[key].strip():
                raise ValueError("Missing combination " + key)
        number = record["candidate"]
        if type(number) is not int or number < 1:
            raise ValueError("Candidate must be a positive integer")
        if record["id"] in ids or number in candidates:
            raise ValueError("Duplicate combination identity")
        selection = record["selection"]
        if (not isinstance(selection, dict) or set(selection) != SLOTS
                or any(not isinstance(v, str) or not v for v in selection.values())):
            raise ValueError("Six explicit component selections are required")
        prefix = f"assets/aster/general_face_eyes_combo{number}/"
        paths = {}
        for filename in ("aster_definition.tres", "aster_head.glb", "generation_report.json"):
            path = (root / prefix / filename).resolve()
            if not path.is_relative_to(root) or not path.is_file():
                raise ValueError("Missing or escaping combination artifact: " + filename)
            paths[filename] = path
        report = json.loads(paths["generation_report.json"].read_text())
        components = report.get("face_components") if isinstance(report, dict) else None
        if not isinstance(components, dict) or components.get("selection") != selection:
            raise ValueError("Catalog label selection differs from actual model report")
        def digest(name):
            return hashlib.sha256(paths[name].read_bytes()).hexdigest()

        if report.get("new_glb_sha256") != digest("aster_head.glb"):
            raise ValueError("Generation report does not identify the actual model")

        result.append({
            "id": record["id"], "label": record["label"],
            "definition": "res://" + prefix + "aster_definition.tres",
            "definition_sha256": digest("aster_definition.tres"),
            "model_sha256": digest("aster_head.glb"),
            "report_sha256": digest("generation_report.json"),
            "selection": dict(selection),
        })
        ids.add(record["id"])
        candidates.add(number)
    return {"schema_version": 1, "entries": result}


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", type=Path, required=True)
    parser.add_argument("--records", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    catalog = build_catalog(args.workspace, json.loads(args.records.read_text()))
    args.output.write_text(json.dumps(catalog, ensure_ascii=False, indent=2) + "\n")
