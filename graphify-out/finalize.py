"""Refresh local Graphify labels, reports, diagnostics, and visualization."""

import json
import re
import sys
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

from graphify.analyze import god_nodes, suggest_questions, surprising_connections
from graphify.benchmark import run_benchmark
from graphify.build import build_from_json
from graphify.cluster import community_member_sigs, score_all
from graphify.detect import detect
from graphify.diagnostics import diagnose_extraction
from graphify.export import to_html, to_json
from graphify.report import generate


OUT = Path(__file__).resolve().parent
ROOT = OUT.parent


def save(name, value):
    (OUT / name).write_text(
        json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )


def community_label(graph, members):
    sources = Counter(
        graph.nodes[node].get("source_file")
        for node in members
        if graph.nodes[node].get("source_file")
    )
    if not sources:
        return "External Symbols"
    source = sorted(sources, key=lambda item: (-sources[item], item))[0]
    path = Path(source)
    stem = path.stem
    if stem in {"mod", "lib", "main", "types", "model", "index", "windows"}:
        stem = path.parent.name + " " + stem
    words = re.sub(r"([a-z])([A-Z])", r"\1 \2", stem)
    words = re.sub(r"[^a-zA-Z0-9]+", " ", words).strip().title().split()
    prefix = "Code"
    for location, name in (
        ("native/core/tests/", "Native Tests"),
        ("native/desktop/", "Desktop"),
        ("native/", "Native"),
        ("src/ui/", "UI"),
        ("src/shared/", "Shared"),
        ("src/core/", "Electron Core"),
        ("src/main/", "Electron Host"),
        ("tests/", "Tests"),
        ("scripts/", "Scripts"),
        ("resources/", "Resources"),
        ("docs/", "Prototypes"),
    ):
        if source.startswith(location):
            prefix = name
            break
    return " ".join((prefix.split() + words)[:5])


def main():
    raw = json.loads((OUT / "graph.json").read_text(encoding="utf-8"))
    directed = bool(raw.get("directed", False))
    graph = build_from_json(raw, directed=directed, root=ROOT)
    if not graph:
        raise SystemExit("Refusing to finalize an empty graph")
    communities = {}
    for node in raw["nodes"]:
        communities.setdefault(int(node["community"]), []).append(node["id"])
    if sum(map(len, communities.values())) != graph.number_of_nodes():
        raise SystemExit("Community membership does not match the graph")
    labels = {
        cid: community_label(graph, members) for cid, members in communities.items()
    }
    detection = detect(ROOT, follow_symlinks=False, google_workspace=False)
    code_files = detection["files"]["code"]
    represented = {
        node.get("source_file") for node in raw["nodes"] if node.get("source_file")
    }
    code_paths = {Path(path).resolve().relative_to(ROOT).as_posix() for path in code_files}
    code_words = sum(len((ROOT / path).read_text(encoding="utf-8-sig").split()) for path in code_paths)
    cohesion = score_all(graph, communities)
    gods = god_nodes(graph)
    surprises = surprising_connections(graph, communities)
    questions = suggest_questions(graph, communities, labels)
    commit = raw.get("built_at_commit")
    if not to_json(
        graph, communities, str(OUT / "graph.json"),
        community_labels=labels, built_at_commit=commit,
    ):
        raise SystemExit("Graphify refused the graph write")
    save(".graphify_labels.json", labels)
    save(".graphify_labels.json.sig", community_member_sigs(communities))
    save(".graphify_analysis.json", {
        "communities": communities, "cohesion": cohesion, "gods": gods,
        "surprises": surprises, "questions": questions,
    })
    report_detection = dict(detection, total_files=len(code_paths), total_words=code_words)
    report_detection["files"] = {"code": code_files}
    report = generate(
        graph, communities, cohesion, labels, gods, surprises, report_detection,
        {"input": 0, "output": 0}, str(ROOT),
        suggested_questions=questions, built_at_commit=commit,
    )
    (OUT / "GRAPH_REPORT.md").write_text(report, encoding="utf-8")
    diagnostics = diagnose_extraction(raw, directed=directed, root=ROOT)
    save("diagnostics.json", {"scope": "Persisted graph; pre-build edge loss is not recoverable", **diagnostics})
    save("coverage.json", {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "built_at_commit": commit,
        "mode": "Local code AST plus Cargo workspace; no semantic LLM extraction",
        "detected_code_files": len(code_paths),
        "represented_source_files": len(represented),
        "code_files_without_nodes": sorted(code_paths - represented),
        "source_extensions": dict(sorted(Counter(Path(path).suffix for path in represented).items())),
        "nodes": graph.number_of_nodes(), "edges": graph.number_of_edges(),
        "communities": len(communities), "directed": directed,
        "cohesion_min": min(cohesion.values()),
        "cohesion_max": max(cohesion.values()),
        "label_method": "Dominant source file and subsystem; mixed communities are approximate",
        "limits": [
            "GDScript, Godot scenes/resources/shaders, CSS and HTML lack AST coverage",
            "Documentation and images were deliberately skipped",
            "IPC strings, dynamic factories and process protocols may lack edges",
            "The graph is undirected; a path alone does not prove call direction",
            "Both native Rust and legacy Electron code are indexed",
        ],
    })
    save("benchmark.json", run_benchmark(str(OUT / "graph.json"), corpus_words=code_words))
    to_html(graph, communities, str(OUT / "graph.html"), community_labels=labels)
    (OUT / ".graphify_python").write_text(sys.executable + "\n", encoding="utf-8")
    print(f"Finalized: {len(graph)} nodes, {graph.number_of_edges()} edges, {len(communities)} named communities")
    print(f"Coverage: {len(represented)} source files; {len(code_paths - represented)} detected code files without nodes")


if __name__ == "__main__":
    main()
