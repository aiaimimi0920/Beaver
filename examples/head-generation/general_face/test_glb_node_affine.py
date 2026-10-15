import copy
import importlib.util
import math
from pathlib import Path
import unittest
from controls import PRESETS
from glb_affine import read_glb, write_glb, unit
from glb_node_affine import (RUNTIME_POLICY, transport_face_node, merge_runtime_face,
                             affine_parameters, assert_native_world_positions)
from test_glb_affine import fixture


def legacy_merger():
    path = Path(__file__).resolve().parents[1] / 'checkpoints/current/glb_merge.py'
    spec = importlib.util.spec_from_file_location('tested_legacy_glb_merge', path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.merge_face


def original_body_hair():
    doc, binary = read_glb(fixture())
    for name in ('Body', 'Hair'):
        mesh = copy.deepcopy(doc['meshes'][0])
        mesh['name'] = name
        mesh.pop('extras')
        mesh['primitives'][0].pop('targets')
        doc['nodes'].append({'name': name, 'mesh': len(doc['meshes'])})
        doc['meshes'].append(mesh)
    doc['scenes'][0]['nodes'] = [0, 1, 2]
    return write_glb(doc, binary)


class NodeAffineTests(unittest.TestCase):
    pivot = (0, 0, 1.641)

    def test_mesh_streams_and_identity_remain_byte_exact(self):
        raw = fixture()
        original, binary = read_glb(raw)
        for config in PRESETS.values():
            result = transport_face_node(raw, config, self.pivot, RUNTIME_POLICY)
            if config == PRESETS['baseline']:
                self.assertIs(result, raw)
            doc, transformed_binary = read_glb(result)
            self.assertEqual(transformed_binary, binary)
            self.assertEqual({key: value for key, value in doc.items() if key != 'nodes'},
                             {key: value for key, value in original.items() if key != 'nodes'})
            scale, translation = affine_parameters(config, self.pivot, RUNTIME_POLICY)
            self.assertEqual(doc['nodes'][0].get('scale', [1, 1, 1]), scale)
            self.assertEqual(doc['nodes'][0].get('translation', [0, 0, 0]), translation)

    def test_merge_propagates_only_face_node_and_preserves_body_hair(self):
        original_raw, face_raw = original_body_hair(), fixture()
        original, oldbin = read_glb(original_raw)
        merge = legacy_merger()
        for config in PRESETS.values():
            face = transport_face_node(face_raw, config, self.pivot, RUNTIME_POLICY)
            result, report = merge_runtime_face(original_raw, face, merge, config, self.pivot, RUNTIME_POLICY)
            doc, binary = read_glb(result)
            scale, translation = affine_parameters(config, self.pivot, RUNTIME_POLICY)
            self.assertEqual(doc['nodes'][0].get('scale', [1, 1, 1]), scale)
            self.assertEqual(doc['nodes'][0].get('translation', [0, 0, 0]), translation)
            self.assertEqual(doc['nodes'][1:], original['nodes'][1:])
            self.assertEqual(doc['meshes'][1:], original['meshes'][1:])
            self.assertEqual(binary[:len(oldbin)], oldbin)
            self.assertEqual(report['runtime_affine_policy'], RUNTIME_POLICY)
            if config == PRESETS['baseline']:
                self.assertEqual(result, merge(original_raw, face_raw)[0])

    def test_policy_existing_transform_and_shared_node_rejected(self):
        raw = fixture()
        with self.assertRaisesRegex(ValueError, 'runtime affine policy'):
            transport_face_node(raw, {}, self.pivot, 'baked_accessor')
        doc, binary = read_glb(raw)
        doc['nodes'][0]['scale'] = [1.06, 1, 1]
        with self.assertRaises(ValueError):
            transport_face_node(write_glb(doc, binary), {}, self.pivot, RUNTIME_POLICY)
        original, binary = read_glb(original_body_hair())
        original['nodes'].append({'name': 'Face copy', 'mesh': 0})
        with self.assertRaisesRegex(ValueError, 'shared'):
            merge_runtime_face(write_glb(original, binary), raw, legacy_merger(), {}, self.pivot, RUNTIME_POLICY)
        with self.assertRaisesRegex(ValueError, 'does not match'):
            merge_runtime_face(original_body_hair(), raw, legacy_merger(), PRESETS['soft_wide'], self.pivot, RUNTIME_POLICY)

    def test_node_affine_preserves_imported_baseline_all_weights(self):
        # Simulate engine-decoded unit endpoints, not lost raw-GLB magnitudes.
        n0, n1 = unit((.3, .8, .2)), unit((-.2, .4, .9))
        t0, t1 = unit((.8, -.3, .1)), unit((.6, .7, -.2))
        p0, p1 = (.04, 1.62, -.08), (.035, 1.60, -.09)
        for config in PRESETS.values():
            scales, translation = affine_parameters(config, self.pivot, RUNTIME_POLICY)
            for weight in (0, .25, .5, .75, 1):
                for base, end, factors in ((n0, n1, [1 / s for s in scales]), (t0, t1, scales)):
                    mixed = tuple((1 - weight) * a + weight * b for a, b in zip(base, end))
                    imported = unit(mixed)
                    node_result = unit(tuple(a * f for a, f in zip(imported, factors)))
                    expected = unit(tuple(a * f for a, f in zip(mixed, factors)))
                    self.assertLess(max(abs(a - b) for a, b in zip(node_result, expected)), 1e-14)
                mixed_position = tuple((1 - weight) * a + weight * b for a, b in zip(p0, p1))
                world = tuple(s * p + t for s, p, t in zip(scales, mixed_position, translation))
                expected = tuple((1 - weight) * (s * a + t) + weight * (s * b + t)
                                 for s, a, b, t in zip(scales, p0, p1, translation))
                self.assertLess(max(abs(a - b) for a, b in zip(world, expected)), 1e-14)
