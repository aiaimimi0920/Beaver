import copy
import math
import struct
import unittest
from glb_affine import (transport_glb, read_glb, write_glb, accessor_values,
                        assert_blender_positions, unit)


def fixture(sparse=True, strided=True):
    binary = bytearray()
    doc = {'asset': {'version': '2.0'}, 'buffers': [{'byteLength': 0}],
           'bufferViews': [], 'accessors': [], 'nodes': [{'mesh': 0, 'name': 'Face'}],
           'scenes': [{'nodes': [0]}], 'scene': 0}

    def view(values, fmt, stride=0):
        binary.extend(b'\0' * (-len(binary) % 4))
        offset = len(binary)
        for value in values:
            binary.extend(struct.pack(fmt, *value))
            binary.extend(b'x' * max(0, stride - struct.calcsize(fmt)))
        item = {'buffer': 0, 'byteOffset': offset, 'byteLength': len(binary) - offset}
        if stride:
            item['byteStride'] = stride
        doc['bufferViews'].append(item)
        return len(doc['bufferViews']) - 1

    def accessor(values, width, stride=0):
        index = view(values, '<' + 'f' * width, stride)
        doc['accessors'].append({'bufferView': index, 'componentType': 5126,
                                 'count': len(values), 'type': f'VEC{width}'})
        return len(doc['accessors']) - 1

    position = accessor([(-.1, 1.5, .02), (.1, 1.7, .02), (0, 1.6, -.1)], 3, 16 if strided else 0)
    normal = accessor([unit((1, 2, 3))] * 3, 3)
    tangent = accessor([(*unit((2, -1, 0)), -1)] * 3, 4)
    uv = accessor([(0, 0), (1, 1), (.5, .5)], 2)
    if sparse:
        index_view, value_view = view([(1,)], '<B'), view([(.01, -.02, .03)], '<fff')
        morph_position = len(doc['accessors'])
        doc['accessors'].append({'componentType': 5126, 'count': 3, 'type': 'VEC3',
                                 'sparse': {'count': 1, 'indices': {'bufferView': index_view, 'componentType': 5121},
                                            'values': {'bufferView': value_view}}})
    else:
        morph_position = accessor([(0, 0, 0), (.01, -.02, .03), (0, 0, 0)], 3)
    morph_normal = accessor([(0, 0, 0), (.05, -.04, .01), (0, 0, 0)], 3)
    morph_tangent = accessor([(0, 0, 0), (.01, .02, -.03), (0, 0, 0)], 3)
    doc['meshes'] = [{'name': 'Face', 'extras': {'targetNames': ['MouthOpen']},
                      'primitives': [{'attributes': {'POSITION': position, 'NORMAL': normal,
                                                     'TANGENT': tangent, 'TEXCOORD_0': uv},
                                      'targets': [{'POSITION': morph_position, 'NORMAL': morph_normal,
                                                   'TANGENT': morph_tangent}]}]}]
    return write_glb(doc, binary)


class GlbAffineTests(unittest.TestCase):
    config = {'width': 1.06, 'height': .96, 'depth': 1.02}
    pivot = (0, 0, 1.641)

    def test_identity_is_byte_exact_and_sparse_matches_dense(self):
        source = fixture()
        self.assertIs(transport_glb(source, {}, self.pivot), source)
        sparse = read_glb(transport_glb(source, self.config, self.pivot))
        dense = read_glb(transport_glb(fixture(False), self.config, self.pivot))
        for index, width in [(0, 3), (1, 3), (2, 4), (4, 3), (5, 3), (6, 3)]:
            self.assertEqual(accessor_values(*sparse, index, width), accessor_values(*dense, index, width))

    def test_sparse_normal_and_tangent_implicit_zeros(self):
        source = fixture()
        doc, binary = read_glb(source)
        binary = bytearray(binary)
        index_view = doc['accessors'][4]['sparse']['indices']['bufferView']
        for index in (5, 6):
            rows = accessor_values(doc, binary, index, 3)
            binary.extend(b'\0' * (-len(binary) % 4))
            view_id = len(doc['bufferViews'])
            doc['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': 12})
            binary.extend(struct.pack('<fff', *rows[1]))
            doc['accessors'][index].pop('bufferView')
            doc['accessors'][index]['sparse'] = {
                'count': 1, 'indices': {'bufferView': index_view, 'componentType': 5121},
                'values': {'bufferView': view_id}}
        sparse = read_glb(transport_glb(write_glb(doc, binary), self.config, self.pivot))
        dense = read_glb(transport_glb(source, self.config, self.pivot))
        for index in (5, 6):
            self.assertEqual(accessor_values(*sparse, index, 3), accessor_values(*dense, index, 3))

    def test_transport_endpoints_directions_bounds_and_frozen_uv(self):
        raw = fixture()
        old, oldbin = read_glb(raw)
        new, newbin = read_glb(transport_glb(raw, self.config, self.pivot))
        scales, pivot = (1.06, .96, 1.02), (0, 1.641, 0)
        def rows(index, width=3, transformed=False):
            return accessor_values(new if transformed else old, newbin if transformed else oldbin, index, width)
        for before, after in zip(rows(0), rows(0, transformed=True)):
            for actual, expected in zip(after, (c + (v - c) * s for c, v, s in zip(pivot, before, scales))):
                self.assertAlmostEqual(actual, expected, places=6)
        for base_id, delta_id, width, inverse in [(1, 5, 3, True), (2, 6, 4, False)]:
            for base, delta, target_base, target_delta in zip(rows(base_id, width), rows(delta_id), rows(base_id, width, True), rows(delta_id, transformed=True)):
                factors = tuple(1 / s for s in scales) if inverse else scales
                expected = unit(tuple((a + d) * f for a, d, f in zip(base, delta, factors)))
                for actual, correct in zip(unit(tuple(a + d for a, d in zip(target_base, target_delta))), expected):
                    self.assertAlmostEqual(actual, correct, places=6)
                self.assertAlmostEqual(sum(x * x for x in target_base[:3]), 1, places=6)
                if width == 4:
                    self.assertEqual(base[3], target_base[3])
        self.assertEqual(rows(3, 2), rows(3, 2, True))
        self.assertEqual(oldbin, newbin[:len(oldbin)])
        for i in (0, 1, 2, 4, 5, 6):
            a = new['accessors'][i]
            values = rows(i, 4 if i == 2 else 3, True)
            self.assertEqual(a['min'], [min(row[k] for row in values) for k in range(len(values[0]))])
            self.assertEqual(a['max'], [max(row[k] for row in values) for k in range(len(values[0]))])
        self.assertEqual(rows(4, transformed=True)[0], (0, 0, 0))
        self.assertEqual(rows(5, transformed=True)[0], (0, 0, 0))
        self.assertEqual(rows(6, transformed=True)[0], (0, 0, 0))

    def test_direction_interpolation_uses_shared_base_scale(self):
        for sparse in (False, True):
            doc, binary = read_glb(fixture())
            binary = bytearray(binary)
            index_view = doc['accessors'][4]['sparse']['indices']['bufferView']
            # Deliberately nonunit endpoints distinguish shared-scale transport
            # from the old independently normalized endpoint implementation.
            for index, delta in [(5, (.4, -.2, .7)), (6, (.3, .8, -.4))]:
                if sparse:
                    binary.extend(b'\0' * (-len(binary) % 4))
                    view_id = len(doc['bufferViews'])
                    doc['bufferViews'].append({'buffer': 0, 'byteOffset': len(binary), 'byteLength': 12})
                    binary.extend(struct.pack('<fff', *delta))
                    doc['accessors'][index].pop('bufferView')
                    doc['accessors'][index]['sparse'] = {
                        'count': 1, 'indices': {'bufferView': index_view, 'componentType': 5121},
                        'values': {'bufferView': view_id}}
                else:
                    offset = doc['bufferViews'][doc['accessors'][index]['bufferView']]['byteOffset']
                    struct.pack_into('<fff', binary, offset + 12, *delta)
            source = write_glb(doc, binary)
            old, oldbin = read_glb(source)
            target = transport_glb(source, self.config, self.pivot)
            new, newbin = read_glb(target)
            scales = (self.config['width'], self.config['height'], self.config['depth'])
            for base_id, delta_id, dimensions, inverse in [(1, 5, 3, True), (2, 6, 4, False)]:
                bases = accessor_values(old, oldbin, base_id, dimensions)
                deltas = accessor_values(old, oldbin, delta_id, 3)
                new_bases = accessor_values(new, newbin, base_id, dimensions)
                new_deltas = accessor_values(new, newbin, delta_id, 3)
                factors = tuple(1 / value for value in scales) if inverse else scales
                endpoint = tuple(a + b for a, b in zip(bases[1], deltas[1]))
                self.assertGreater(abs(math.sqrt(sum(v * v for v in endpoint)) - 1), .1)
                for base, delta, mapped_base, mapped_delta in zip(bases, deltas, new_bases, new_deltas):
                    length = math.sqrt(sum((a * f) ** 2 for a, f in zip(base, factors)))
                    for actual, expected in zip(mapped_delta, (d * f / length for d, f in zip(delta, factors))):
                        self.assertAlmostEqual(actual, expected, places=6)
                    for weight in (0, .25, .5, .75, 1):
                        expected = unit(tuple((a + weight * d) * f for a, d, f in zip(base, delta, factors)))
                        actual = unit(tuple(a + weight * d for a, d in zip(mapped_base, mapped_delta)))
                        self.assertLess(max(abs(a - b) for a, b in zip(actual, expected)), 1e-7)
                    if dimensions == 4:
                        self.assertEqual(mapped_base[3], base[3])
                self.assertEqual(new_deltas[0], (0, 0, 0))
                self.assertEqual(new_deltas[2], (0, 0, 0))
            self.assertIs(transport_glb(source, {}, self.pivot), source)

    def test_invalid_layout_frames_semantics_and_sparse(self):
        raw = fixture()
        original, binary = read_glb(raw)
        edits = [lambda d: d['nodes'][0].update(scale=[-1, 1, 1]),
                 lambda d: d['accessors'][0].update(componentType=5123),
                 lambda d: d['bufferViews'][0].update(byteStride=4),
                 lambda d: d['meshes'][0]['primitives'][0]['targets'][0].update(UNKNOWN=4),
                 lambda d: d['accessors'][4]['sparse'].update(count=4),
                 lambda d: d['accessors'][4]['sparse']['indices'].update(componentType=5126)]
        for edit in edits:
            doc = copy.deepcopy(original)
            edit(doc)
            with self.assertRaises(ValueError):
                transport_glb(write_glb(doc, binary), self.config, self.pivot)
        with self.assertRaises(ValueError):
            transport_glb(raw, {'width': float('nan')}, self.pivot)
        with self.assertRaises(ValueError):
            transport_glb(raw[:-1], self.config, self.pivot)

    def test_preserved_semantic_alias_rejected(self):
        for semantic in ('COLOR_0', 'TEXCOORD_1'):
            for config in ({}, self.config):
                doc, binary = read_glb(fixture())
                # COLOR_0 VEC3 is valid, but shares NORMAL's accessor storage.
                doc['meshes'][0]['primitives'][0]['attributes'][semantic] = 1
                with self.assertRaisesRegex(ValueError, 'aliases a preserved semantic'):
                    transport_glb(write_glb(doc, binary), config, self.pivot)

    def test_swapped_morph_endpoint_correspondence_rejected(self):
        raw = fixture(sparse=False)
        doc, binary = read_glb(raw)
        base = accessor_values(doc, binary, 0, 3)
        delta = accessor_values(doc, binary, 4, 3)
        endpoints = [tuple(a + b for a, b in zip(p, d)) for p, d in zip(base, delta)]
        swapped = [endpoints[1], endpoints[0], endpoints[2]]
        wrong_delta = [tuple(b - a for a, b in zip(p, endpoint)) for p, endpoint in zip(base, swapped)]
        binary = bytearray(binary)
        accessor = doc['accessors'][4]
        offset = doc['bufferViews'][accessor['bufferView']]['byteOffset']
        for i, row in enumerate(wrong_delta):
            struct.pack_into('<fff', binary, offset + 12 * i, *row)
        wrong = write_glb(doc, binary)
        # Both separate point sets match within float32 rounding, but their
        # pairings differ, so the new record-level contract must reject it.
        wd, wb = read_glb(wrong)
        decoded = accessor_values(wd, wb, 4, 3)
        wrong_endpoints = [tuple(a + b for a, b in zip(p, d)) for p, d in zip(base, decoded)]
        for expected, actual in zip(sorted(endpoints), sorted(wrong_endpoints)):
            self.assertLess(max(abs(a - b) for a, b in zip(expected, actual)), 3e-7)
        with self.assertRaisesRegex(ValueError, 'paired positions disagree'):
            assert_blender_positions(raw, wrong)

    def test_position_agreement_rejects_wrong_geometry(self):
        raw = fixture()
        result = transport_glb(raw, self.config, self.pivot)
        self.assertEqual(assert_blender_positions(result, result)['base_and_mouthopen_endpoint_max_component_error_m'], 0)
        with self.assertRaises(ValueError):
            assert_blender_positions(result, raw)


if __name__ == '__main__':
    unittest.main()
