"""Adapt shape-specific bridge repair to explicit zero/paired repair requirements."""


def once(source, before, after):
    if source.count(before) != 1:
        raise ValueError("Optional bridge source contract changed: " + before[:70])
    return source.replace(before, after, 1)


def adapt_bridge_pair_source(source):
    return once(
        source,
        "    assert len(initial) in (1,2),",
        "    if not initial:\n        return []\n    assert len(initial) in (1,2),",
    )


def adapt_ear_boundary_source(source):
    source = once(
        source,
        "assert len(initial_bridge_groups)==2, 'Expected two initial constrained bridges'",
        "assert len(initial_bridge_groups) in (0,2), 'Expected zero or two constrained bridges'",
    )
    source = once(
        source,
        "    bridge_groups=[list(data.polygons[i].vertices) for i in sorted(limiting_faces) if len(data.polygons[i].vertices)==3 and 40<=regions.data[i].value<50 and bridge_roots.intersection(data.polygons[i].vertices)]",
        "    limiting_bridge_indices=paired_bridge_indices(before,candidates,sorted(limiting_faces & candidates.keys()))\n"
        "    bridge_groups=[candidates[i] for i in limiting_bridge_indices]",
    )
    source = once(
        source,
        "assert len(bridge_groups)==2,('Expected symmetric constrained ear-root faces'",
        "assert len(bridge_groups) in (0,2),('Expected symmetric constrained ear-root faces'",
    )
    return once(
        source,
        "'bridge_preconditioned':True",
        "'bridge_preconditioned':bool(initial_bridge_groups)",
    )
