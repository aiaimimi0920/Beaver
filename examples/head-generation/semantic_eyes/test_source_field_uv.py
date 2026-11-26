"""Behavioral mock forbids legacy lazy-flag setters; native proof stays private."""
from types import SimpleNamespace
import unittest
from source_field_uv import write_uv_coordinates


class Element:
    def __init__(self):
        self.uv = [0., 0.]
        self.flags = {'pin_uv': False, 'select': False, 'select_edge': False}

    def __getattr__(self, name):
        if name in self.flags:
            return self.flags[name]
        raise AttributeError(name)

    def __setattr__(self, name, value):
        if name in ('pin_uv', 'select', 'select_edge'):
            raise AssertionError('Lazy flag setter must not be used')
        object.__setattr__(self, name, value)


class Data(list):
    def foreach_set(self, name, flat):
        if name != 'uv' or len(flat) != 2 * len(self):
            raise AssertionError('Unexpected UV write')
        for index, element in enumerate(self):
            element.uv = flat[2 * index:2 * index + 2]


class Layers(dict):
    def __init__(self, attributes):
        super().__init__()
        self.attributes = attributes
        self.rename = False

    def new(self, name, do_init=False):
        if do_init:
            raise AssertionError('No inherited coordinate initialization')
        name = name + '.001' if self.rename else name
        layer = SimpleNamespace(name=name, data=Data([Element(), Element()]))
        self[name] = layer
        self.attributes[name] = SimpleNamespace(data_type='FLOAT2', domain='CORNER')
        return layer

    def remove(self, layer):
        del self[layer.name]
        del self.attributes[layer.name]


class SafeUVTests(unittest.TestCase):
    def mesh(self):
        attributes = {}
        return SimpleNamespace(loops=[0, 1], attributes=attributes,
                               is_editmode=False, uv_layers=Layers(attributes))

    def test_coordinates_only_and_float32_conversion(self):
        mesh = self.mesh()
        write_uv_coordinates(mesh, {'UV': [[.25, .75], [.1, -.0]],
                                    'OtherSide': [[.5, .5], [1., 0.]]})
        self.assertEqual(mesh.uv_layers['UV'].data[0].uv, [.25, .75])
        self.assertNotEqual(mesh.uv_layers['UV'].data[1].uv[0], .1)
        self.assertEqual(mesh.uv_layers['OtherSide'].data[0].uv, [.5, .5])

    def test_existing_pin_rejected_before_write(self):
        mesh = self.mesh()
        mesh.uv_layers.new('UV')
        mesh.uv_layers['UV'].data[0].flags['pin_uv'] = True
        with self.assertRaises(ValueError):
            write_uv_coordinates(mesh, {'UV': [[.5, .5]] * 2})
        self.assertEqual(mesh.uv_layers['UV'].data[0].uv, [0., 0.])

    def test_allocation_preflight_and_rollback(self):
        mesh = self.mesh()
        mesh.attributes['UV'] = SimpleNamespace(data_type='FLOAT', domain='POINT')
        with self.assertRaises(ValueError):
            write_uv_coordinates(mesh, {'UV': [[0., 0.]] * 2})
        self.assertFalse(mesh.uv_layers)
        mesh = self.mesh()
        mesh.uv_layers.rename = True
        with self.assertRaises(ValueError):
            write_uv_coordinates(mesh, {'UV': [[0., 0.]] * 2})
        self.assertFalse(mesh.uv_layers)
        self.assertFalse(mesh.attributes)
        for name in ('x' * 64, 'has\0nul'):
            with self.assertRaises(ValueError):
                write_uv_coordinates(self.mesh(), {name: [[0., 0.]] * 2})
        mesh = self.mesh()
        mesh.is_editmode = True
        with self.assertRaises(ValueError):
            write_uv_coordinates(mesh, {'UV': [[0., 0.]] * 2})
        mesh = self.mesh()
        with self.assertRaises(ValueError):
            write_uv_coordinates(mesh, {str(i): [[0., 0.]] * 2 for i in range(9)})
        self.assertFalse(mesh.uv_layers)

    def test_invalid_payload_does_not_allocate(self):
        for rows in ([[0, 1]], [[0, 1], [float('nan'), 0]], [[0, 1], [1]]):
            mesh = self.mesh()
            with self.assertRaises(ValueError):
                write_uv_coordinates(mesh, {'UV': rows})
            self.assertFalse(mesh.uv_layers)


if __name__ == '__main__':
    unittest.main()
