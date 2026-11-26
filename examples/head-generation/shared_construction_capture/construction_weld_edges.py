"""Mesh-bound edge markers; native copied data donors, not merged flag authority."""
from construction_markers import MARKERS, fields
from construction_topology import digest

EDGE_MARKER = "_BeaverConstructionWeldEdge"


def unmarked_fields(obj):
    state = fields(obj)
    state["attributes"].pop(EDGE_MARKER, None)
    return state


def attach_edge_mesh(obj):
    if obj.data.attributes.get(EDGE_MARKER) is not None:
        raise ValueError("Native weld edge marker collision")
    before = digest(fields(obj))
    try:
        obj.data.attributes.new(name=EDGE_MARKER, type="INT", domain="EDGE")
        for i, row in enumerate(obj.data.attributes[EDGE_MARKER].data):
            row.value = i + 1
        if digest(unmarked_fields(obj)) != before:
            raise ValueError("Native edge Mesh marker changed other fields")
    except Exception:
        layer = obj.data.attributes.get(EDGE_MARKER)
        if layer is not None:
            obj.data.attributes.remove(layer)
        if digest(fields(obj)) != before:
            raise ValueError("Native edge Mesh marker rollback changed other fields")
        raise


def clear_edge_mesh(obj):
    failure = None
    try:
        before = digest(unmarked_fields(obj))
    except Exception as error:
        failure = error
    layer = obj.data.attributes.get(EDGE_MARKER)
    if layer is not None:
        obj.data.attributes.remove(layer)
    if failure is not None:
        raise failure
    if digest(fields(obj)) != before:
        raise ValueError("Native edge Mesh marker removal changed other fields")


def bind_edges(bm, source):
    # BMesh layer allocation reallocates native elements. Allocate on Mesh before
    # import instead; never rebind surviving identities within the operator phase.
    layer = bm.edges.layers.int.get(EDGE_MARKER)
    if layer is None:
        raise ValueError("Native BMesh lost typed edge provenance storage")
    point_layer = bm.verts.layers.int.get(MARKERS["POINT"])
    by_pair = {tuple(sorted(edge)): i for i, edge in enumerate(source["edges"])}
    if len(by_pair) != len(source["edges"]):
        raise ValueError("Duplicate original source edges before weld")
    originals = {}
    for edge in bm.edges:
        pair = tuple(sorted(v[point_layer] - 1 for v in edge.verts))
        if pair not in by_pair or edge[layer] != by_pair[pair] + 1:
            raise ValueError("BMesh import changed source edge identity/incidence")
        originals[edge] = by_pair[pair]
    if len(originals) != len(source["edges"]):
        raise ValueError("BMesh import changed source edge coverage")
    return layer, originals


def verify_edge_donors(bm, layer, originals, quotient):
    donors = []
    for edge, candidates in zip(bm.edges, quotient["edges"]):
        donor = edge[layer] - 1
        if type(donor) is not int or donor not in candidates:
            raise ValueError("Native edge custom-data donor outside source endpoint quotient")
        if edge in originals and donor != originals[edge]:
            raise ValueError("Surviving original native edge changed its custom-data donor")
        donors.append({"custom_data_donor": donor, "source_edge_candidates": candidates,
                       "original_edge_object_survived": edge in originals})
    return {"rows": donors, "flags": "native flags may merge contributions from every source edge; not a single-donor field admission"}
