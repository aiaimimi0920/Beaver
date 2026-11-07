"""Execute the native adapter against a minimal typed RNA stand-in, no Blender."""

from types import SimpleNamespace as N
import unittest
from fixtures import layout, triangulate
from shared_capture_contract import MARKERS
from shared_capture_native import ATTRIBUTE_PROPERTIES, begin_source_capture, finish_source_capture
from shared_capture_normals import set_captured_normals


class Attributes(list):
    def __init__(self, mesh):
        super().__init__()
        self.counts = {"POINT": len(mesh["vertices"]), "FACE": len(mesh["faces"]),
                       "CORNER": len(mesh["corners"]), "EDGE": len(mesh["edges"])}
        for name, attr in mesh["attributes"].items():
            prop = ATTRIBUTE_PROPERTIES[attr["type"]]
            self.append(N(name=name, domain=attr["domain"], data_type=attr["type"],
                          data=[N(**{prop: row}) for row in attr["values"]]))

    def __getitem__(self, item):
        return self.get(item) if isinstance(item, str) else super().__getitem__(item)

    def get(self, name):
        return next((attr for attr in self if attr.name == name), None)

    def new(self, name, type, domain):
        attr = N(name=name, domain=domain, data_type=type,
                 data=[N(value=0) for _ in range(self.counts[domain])])
        self.append(attr)
        return attr


def native(mesh, identities=None, shapes=None):
    groups = [N(index=0, name="Continuous facial shell")]
    data = N(
        vertices=[N(co=row["co"], groups=[N(group=0, weight=1.)],
                    select=row["select"], hide=row["hide"]) for row in mesh["vertices"]],
        edges=[N(vertices=row["vertices"], use_seam=row["seam"], use_edge_sharp=row["sharp"],
                 select=row["select"], hide=row["hide"]) for row in mesh["edges"]],
        polygons=[N(vertices=row["vertices"], loop_indices=row["loops"], material_index=row["material"],
                    use_smooth=row["smooth"], select=row["select"], hide=row["hide"]) for row in mesh["faces"]],
        loops=[N(vertex_index=row["vertex"], edge_index=row["edge"]) for row in mesh["corners"]],
        uv_layers=[N(name=name, data=[N(uv=row["uv"], pin_uv=row["pin"],
                                       select=row["select"], select_edge=row["select_edge"]) for row in rows])
                   for name, rows in mesh["uv_layers"].items()],
        materials=[N(name=name) for name in mesh["materials"]], attributes=Attributes(mesh),
        corner_normals=[N(vector=[0., 0., 1.]) for _ in mesh["corners"]], shape_keys=None)
    if identities is not None:
        for domain, name in MARKERS.items():
            attr = data.attributes.new(name, "INT", domain)
            for row, value in zip(attr.data, identities[domain]):
                row.value = value
    if shapes is not None:
        data.shape_keys = N(use_relative=True, key_blocks=[
            N(name=name, data=[N(co=p) for p in rows], relative_key=N(name="Basis"),
              value=0., mute=False, interpolation="KEY_LINEAR", vertex_group="",
              slider_min=0., slider_max=1.) for name, rows in shapes.items()])
    def set_normals(values):
        data.corner_normals = [N(vector=list(value)) for value in values]
        data.edges[0].use_edge_sharp = True
        normal = data.attributes.new("custom_normal", "INT16_2D", "CORNER")
        for row in normal.data:
            row.value = [0, 0]
        sharp = data.attributes.new("sharp_edge", "BOOLEAN", "EDGE")
        for i, row in enumerate(sharp.data):
            row.value = i == 0
    data.normals_split_custom_set = set_normals
    return N(data=data, vertex_groups=groups, matrix_world=[[int(i == j) for j in range(4)] for i in range(4)])


class NativeCaptureTests(unittest.TestCase):
    def test_real_adapter_two_mesh_shapes_cleanup_and_unchanged_fields(self):
        for rows, cols in ((3, 4), (5, 7)):
            mesh, ownership, recipe = layout(rows, cols, .003)
            obj = native(mesh)
            frozen = begin_source_capture(obj, ownership, recipe)
            self.assertTrue(all(obj.data.attributes.get(name) for name in MARKERS.values()))
            runtime, ids, shapes, _ = triangulate(mesh)
            obj = native(runtime, ids, shapes)
            set_captured_normals(obj, frozen, [[0., 0., 1.]] * len(obj.data.loops))
            packet = finish_source_capture(obj, frozen)
            self.assertTrue(packet["packet_sha256"])
            self.assertTrue(all(obj.data.attributes.get(name) is None for name in MARKERS.values()))
            self.assertEqual(set(packet["saved_minus_basis_by_source_point"]), set(shapes) - {"Basis"})

    def test_missing_saved_key_aborts_and_cleans_markers(self):
        mesh, owners, recipe = layout(3, 4, .002)
        frozen = begin_source_capture(native(mesh), owners, recipe)
        runtime, ids, shapes, _ = triangulate(mesh)
        shapes.pop("MouthRound")
        obj = native(runtime, ids, shapes)
        set_captured_normals(obj, frozen, [[0., 0., 1.]] * len(obj.data.loops))
        with self.assertRaisesRegex(ValueError, "six saved keys"):
            finish_source_capture(obj, frozen)
        self.assertTrue(all(obj.data.attributes.get(name) is None for name in MARKERS.values()))

    def test_normal_phase_rejects_authored_sharp_mutation(self):
        mesh, owners, recipe = layout(3, 4, .002)
        frozen = begin_source_capture(native(mesh), owners, recipe)
        runtime, ids, shapes, _ = triangulate(mesh)
        obj = native(runtime, ids, shapes)
        setter = obj.data.normals_split_custom_set
        def bad_setter(values):
            setter(values)
            obj.data.attributes["SourceAuthorSharp"].data[0].value = True
        obj.data.normals_split_custom_set = bad_setter
        with self.assertRaisesRegex(ValueError, "authored attribute"):
            set_captured_normals(obj, frozen, [[0., 0., 1.]] * len(obj.data.loops))
        self.assertTrue(all(obj.data.attributes.get(name) is None for name in MARKERS.values()))

    def test_marker_schema_rejects_before_normal_setter(self):
        for key, value in (("domain", "FACE"), ("data_type", "FLOAT")):
            mesh, owners, recipe = layout(3, 4, .002)
            frozen = begin_source_capture(native(mesh), owners, recipe)
            runtime, ids, shapes, _ = triangulate(mesh)
            obj = native(runtime, ids, shapes)
            setattr(obj.data.attributes[MARKERS["POINT"]], key, value)
            called = []
            obj.data.normals_split_custom_set = lambda rows: called.append(rows)
            with self.assertRaisesRegex(ValueError, "identity missing or changed"):
                set_captured_normals(obj, frozen, [[0., 0., 1.]] * len(obj.data.loops))
            self.assertEqual(called, [])

    def test_after_normal_phase_changes_are_not_generated_sharp(self):
        mesh, owners, recipe = layout(3, 4, .002)
        frozen = begin_source_capture(native(mesh), owners, recipe)
        runtime, ids, shapes, _ = triangulate(mesh)
        obj = native(runtime, ids, shapes)
        set_captured_normals(obj, frozen, [[0., 0., 1.]] * len(obj.data.loops))
        obj.data.edges[1].use_edge_sharp = True
        with self.assertRaisesRegex(ValueError, "after native normal setter"):
            finish_source_capture(obj, frozen)

    def test_reserved_identity_collision_preserves_existing_attribute(self):
        mesh, owners, recipe = layout(3, 4, .002)
        obj = native(mesh)
        attr = obj.data.attributes.new(MARKERS["POINT"], "INT", "POINT")
        with self.assertRaisesRegex(ValueError, "collision"):
            begin_source_capture(obj, owners, recipe)
        self.assertIs(obj.data.attributes.get(MARKERS["POINT"]), attr)


if __name__ == "__main__":
    unittest.main()
