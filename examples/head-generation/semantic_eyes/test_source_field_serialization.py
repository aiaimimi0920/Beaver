"""Synthetic lossless fields/identity tests; no engine or private model needed."""
import copy
import json
import unittest
import numpy as np
from source_field_serialization import split_color_channels, recover_color_channels
from source_field_identity import recover_source_fields, verify_recovered_fields


def fixture():
    arrays = [np.array([[0., -0., .25, 1.], [.125, .5, .75, 1.]], '<f4'),
              np.array([[0.], [1.]], '<f4'),
              np.array([[0., 0., 1.], [0., 1., 0.]], '<f4')]
    binary = b''
    views, accessors = [], []
    for array, kind in zip(arrays, ('VEC4', 'SCALAR', 'VEC3')):
        views.append({'byteOffset': len(binary), 'byteLength': array.nbytes})
        accessors.append({'bufferView': len(views) - 1, 'componentType': 5126,
                          'count': len(array), 'type': kind})
        binary += array.tobytes()
    document = {'buffers': [{'byteLength': len(binary)}], 'bufferViews': views,
                'accessors': accessors, 'materials': [{'name': 'unchanged'}],
                'meshes': [{'primitives': [{'attributes': {
                    '_SOURCE_COLOR': 0, '_SOURCE_CORNER': 1, 'NORMAL': 2}}]}]}
    return document, binary, arrays[0]


class SourceSerializationTests(unittest.TestCase):
    def test_split_preserves_bits_and_nonowned_semantics(self):
        document, binary, expected = fixture()
        original = copy.deepcopy(document)
        result, receipt = split_color_channels(document, binary)
        self.assertEqual(document, original)
        self.assertEqual(result['materials'], original['materials'])
        self.assertEqual(result['accessors'][:3], original['accessors'])
        attributes = result['meshes'][0]['primitives'][0]['attributes']
        self.assertEqual(recover_color_channels(result, binary, attributes).tobytes(), expected.tobytes())
        self.assertTrue(receipt['binary_unchanged'])
        self.assertTrue(receipt['metadata_is_not_shader_binding'])

    def test_recovery_binds_identity_and_serialized_bytes(self):
        document, binary, _ = fixture()
        document, _ = split_color_channels(document, binary)
        attributes = document['meshes'][0]['primitives'][0]['attributes']
        receipt = recover_source_fields(
            document, binary, attributes, source_corner_loops=[7, 2],
            source_corner_vertices=[3, 3], source_hashes={'asset': 'a' * 64})
        self.assertEqual(receipt['native_loop_ids'], [7, 2])
        self.assertEqual(receipt['native_vertex_ids'], [3, 3])
        self.assertTrue(verify_recovered_fields(receipt, json.loads(json.dumps(receipt))))
        bad = copy.deepcopy(receipt)
        bad['native_loop_ids'].reverse()
        with self.assertRaises(ValueError):
            verify_recovered_fields(receipt, bad)
        bad = copy.deepcopy(receipt)
        bad['normal_sha256'] = 'b' * 64
        with self.assertRaises(ValueError):
            verify_recovered_fields(bad, bad)

    def test_native_loop_consistency_in_recovery_and_verification(self):
        document, binary, _ = fixture()
        document, _ = split_color_channels(document, binary)
        attributes = document['meshes'][0]['primitives'][0]['attributes']
        for vertices in ([3, 4], [3, 3]):
            # Same loop cannot mean two vertices OR conflicting field values.
            with self.assertRaises(ValueError):
                recover_source_fields(document, binary, attributes,
                                      source_corner_loops=[7, 7],
                                      source_corner_vertices=vertices,
                                      source_hashes={'asset': 'a' * 64})
        receipt = recover_source_fields(document, binary, attributes,
                                        source_corner_loops=[7, 2],
                                        source_corner_vertices=[3, 4],
                                        source_hashes={'asset': 'a' * 64})
        receipt['native_loop_ids'] = [7, 7]
        with self.assertRaises(ValueError):
            verify_recovered_fields(receipt, receipt)
        receipt['native_loop_ids'] = [7, 2]
        receipt['source_corner_ids'] = [0, 0]
        with self.assertRaises(ValueError):
            verify_recovered_fields(receipt, receipt)

    def test_reject_wrong_semantic_representation(self):
        document, binary, _ = fixture()
        document['meshes'][0]['primitives'][0]['attributes']['_OTHER_VECTOR'] = 2
        with self.assertRaises(ValueError):
            split_color_channels(document, binary)
        document, binary, _ = fixture()
        document['accessors'][0]['normalized'] = True
        with self.assertRaises(ValueError):
            split_color_channels(document, binary)

    def test_reject_malformed_other_custom_accessors(self):
        for index in (-1, True, 3, 100):
            document, binary, _ = fixture()
            document['meshes'][0]['primitives'][0]['attributes']['_OTHER'] = index
            with self.assertRaises(ValueError):
                split_color_channels(document, binary)
        for update in ({'normalized': True}, {'count': 1}):
            document, binary, _ = fixture()
            document['accessors'][1].update(update)
            with self.assertRaises(ValueError):
                split_color_channels(document, binary)

    def test_reject_corner_identity_corruption(self):
        for ids in ((0., 2.), (0., .5), (0., 0.)):
            document, binary, _ = fixture()
            binary = binary[:32] + np.array(ids, '<f4').tobytes() + binary[40:]
            document, _ = split_color_channels(document, binary)
            attributes = document['meshes'][0]['primitives'][0]['attributes']
            with self.assertRaises(ValueError):
                recover_source_fields(document, binary, attributes,
                                      source_corner_loops=[0, 1],
                                      source_corner_vertices=[0, 1],
                                      source_hashes={'asset': 'a' * 64})


if __name__ == '__main__':
    unittest.main()
