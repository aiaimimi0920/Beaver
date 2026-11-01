"""Synthetic spec-bound decoder regressions; no private model fixtures."""
import struct
import unittest
import numpy as np
from source_field_accessor import decode_accessor, decode_position_target, MAX_DECODED_BYTES


def fixture(component, values, fmt, normalized=True):
    binary = struct.pack('<' + fmt * len(values), *values)
    document = {'buffers': [{'byteLength': len(binary)}],
                'bufferViews': [{'byteLength': len(binary)}],
                'accessors': [{'bufferView': 0, 'componentType': component,
                               'count': len(values), 'type': 'SCALAR',
                               'normalized': normalized}]}
    return document, binary


class AccessorTests(unittest.TestCase):
    def test_normalized_boundaries(self):
        for component, fmt, values, denominator in (
                (5120, 'b', [-128, -127, -1, 0, 1, 127], 127),
                (5121, 'B', [0, 1, 128, 255], 255),
                (5122, 'h', [-32768, -32767, -1, 0, 1, 32767], 32767),
                (5123, 'H', [0, 1, 32768, 65535], 65535)):
            with self.subTest(component=component):
                document, binary = fixture(component, values, fmt)
                expected = [max(value / denominator, -1) for value in values]
                np.testing.assert_array_equal(decode_accessor(document, binary, 0)[:, 0], expected)
                document['accessors'][0]['normalized'] = False
                np.testing.assert_array_equal(decode_accessor(document, binary, 0)[:, 0], values)

    def test_reject_bad_normalization(self):
        for value in (1, 0, 'true', None, [], {}):
            document, binary = fixture(5121, [0, 255], 'B', value)
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)
        for component, fmt in ((5125, 'I'), (5126, 'f')):
            document, binary = fixture(component, [0, 1], fmt)
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)

    def sparse_fixture(self):
        binary = struct.pack('<6f', 1, 2, 3, 4, 5, 6)
        binary += bytes([1, 0, 0, 0]) + struct.pack('<3f', 7, 8, 9)
        document = {'buffers': [{'byteLength': len(binary)}], 'bufferViews': [
            {'byteOffset': 0, 'byteLength': 24},
            {'byteOffset': 24, 'byteLength': 1},
            {'byteOffset': 28, 'byteLength': 12}], 'accessors': [
                {'componentType': 5126, 'type': 'VEC3', 'count': 2, 'sparse': {
                    'count': 1, 'indices': {'bufferView': 1, 'componentType': 5121},
                    'values': {'bufferView': 2}}}]}
        return document, binary

    def test_sparse_and_omitted_target(self):
        document, binary = self.sparse_fixture()
        np.testing.assert_array_equal(decode_accessor(document, binary, 0), [[0, 0, 0], [7, 8, 9]])
        document['accessors'][0]['bufferView'] = 0
        np.testing.assert_array_equal(decode_accessor(document, binary, 0), [[1, 2, 3], [7, 8, 9]])
        np.testing.assert_array_equal(decode_position_target(document, binary, {}, 2), np.zeros((2, 3)))
        with self.assertRaises(ValueError):
            decode_position_target(document, binary, {'POSITION': 0}, 3)

    def test_sparse_invalid_ranges(self):
        mutations = [
            lambda g: g['accessors'][0].update(count=True),
            lambda g: g['accessors'][0].update(count=0),
            lambda g: g['accessors'][0].update(byteOffset=4),
            lambda g: g['bufferViews'][2].update(byteStride=12),
            lambda g: g['bufferViews'][2].update(byteLength=8),
            lambda g: g['bufferViews'][1].update(byteOffset=-1),
            lambda g: g['accessors'][0]['sparse'].update(count=3),
            lambda g: g['accessors'][0]['sparse']['values'].update(byteOffset=1),
            lambda g: g['accessors'][0]['sparse']['indices'].update(componentType=5120)]
        for mutation in mutations:
            document, binary = self.sparse_fixture()
            mutation(document)
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)
        for indices in ((1, 1), (1, 0), (0, 2)):
            document, binary = self.sparse_fixture()
            document['accessors'][0]['sparse']['count'] = 2
            document['bufferViews'][1]['byteLength'] = 2
            document['bufferViews'][2]['byteLength'] = 24
            binary = binary[:24] + bytes(indices) + b'\0\0' + struct.pack('<6f', *range(6))
            document['buffers'][0]['byteLength'] = len(binary)
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)

    def test_sparse_normalization_before_conversion(self):
        document = {'buffers': [{'byteLength': 4}], 'bufferViews': [
            {'byteOffset': 0, 'byteLength': 1},
            {'byteOffset': 2, 'byteLength': 2}], 'accessors': [
                {'componentType': 5122, 'count': 3, 'type': 'SCALAR',
                 'normalized': True, 'sparse': {'count': 1,
                     'indices': {'bufferView': 0, 'componentType': 5121},
                     'values': {'bufferView': 1}}}]}
        binary = bytes([1, 0]) + struct.pack('<h', -32768)
        np.testing.assert_array_equal(decode_accessor(document, binary, 0)[:, 0], [0, -1, 0])

    def test_required_compression_cannot_become_zero_accessor(self):
        document = {'extensionsRequired': ['KHR_draco_mesh_compression'],
                    'accessors': [{'componentType': 5126, 'type': 'VEC3', 'count': 3}]}
        with self.assertRaises(ValueError):
            decode_accessor(document, b'', 0)
        for required in (False, None, {}, '', [1], ['KHR_texture_transform'] * 2):
            document['extensionsRequired'] = required
            with self.assertRaises(ValueError):
                decode_accessor(document, b'', 0)

    def test_texture_transform_does_not_change_accessor_storage(self):
        document, binary = fixture(5126, [1], 'f', False)
        document['extensionsRequired'] = ['KHR_texture_transform']
        np.testing.assert_array_equal(decode_accessor(document, binary, 0), [[1]])

    def test_allocation_budget_before_zero_or_normalized_allocation(self):
        for component, normalized, itemsize in ((5126, False, 4), (5121, True, 8)):
            document = {'accessors': [{'componentType': component, 'type': 'VEC4',
                        'count': MAX_DECODED_BYTES // (4 * itemsize) + 1,
                        'normalized': normalized}]}
            with self.assertRaisesRegex(ValueError, 'budget'):
                decode_accessor(document, b'', 0)
        with self.assertRaisesRegex(ValueError, 'budget'):
            decode_position_target({}, b'', {}, MAX_DECODED_BYTES // 12 + 1)

    def test_reviewed_fail_closed_cases(self):
        document, binary = fixture(5126, [1], 'f', False)
        document['bufferViews'][0]['byteStride'] = 252
        with self.assertRaises(ValueError):
            decode_accessor(document, binary, 0)
        for view in (1, 2):
            document, binary = self.sparse_fixture()
            document['bufferViews'][view]['target'] = 34963
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)
        for location in ('sparse', 'indices', 'values'):
            for extensions in ({'UNKNOWN_representation': {}}, False, None, []):
                document, binary = self.sparse_fixture()
                target = document['accessors'][0]['sparse']
                if location != 'sparse':
                    target = target[location]
                target['extensions'] = extensions
                with self.assertRaises(ValueError):
                    decode_accessor(document, binary, 0)
        for offset in (False, 0, 0.0, None):
            document, binary = self.sparse_fixture()
            document['accessors'][0]['byteOffset'] = offset
            with self.assertRaises(ValueError):
                decode_accessor(document, binary, 0)

    def test_stride_and_extension_rejection(self):
        document, binary = fixture(5126, [0, 1, 2, 3, 0, 4, 5, 6], 'f', False)
        document['bufferViews'][0]['byteStride'] = 16
        document['accessors'][0].update(type='VEC3', count=2, byteOffset=4)
        np.testing.assert_array_equal(decode_accessor(document, binary, 0), [[1, 2, 3], [4, 5, 6]])
        document['bufferViews'][0]['extensions'] = {'unsupported': {}}
        with self.assertRaises(ValueError):
            decode_accessor(document, binary, 0)


if __name__ == '__main__':
    unittest.main()
