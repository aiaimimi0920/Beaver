"""Live authored ownership provenance, not region-number inference or shape admission."""
import copy
import json
import unittest
from types import SimpleNamespace
from source_ear_weld_ownership import capture_authored_skin_ownership
from forehead_shared_consumer import forehead_consumer_skin_ownership, forehead_consumer_binding


def member(name, regions):
    attribute = SimpleNamespace(domain='FACE', data_type='INT',
                                data=[SimpleNamespace(value=r) for r in regions])
    return SimpleNamespace(name=name, data=SimpleNamespace(attributes={'NormalRegion': attribute}))


class SkinOwnershipTests(unittest.TestCase):
    def receipt(self):
        shell = member('live skin', [71, 72, 71])
        return capture_authored_skin_ownership([shell, member('other', [94])], shell)

    def report(self, receipt):
        return {'ear_shell_membership': {'authored_skin_ownership': receipt}}

    def test_labels_derived_not_assumed_and_permutation(self):
        receipt = self.receipt()
        result = forehead_consumer_skin_ownership([94, 72, 71, 94], self.report(receipt))
        self.assertEqual(result['source_face_ids'], [1, 2])
        self.assertTrue(result['available'])
        self.assertEqual(forehead_consumer_skin_ownership([71, 94, 72], self.report(receipt))[
            'source_face_ids'], [0, 2])

    def test_all_non_shell_collisions_block_without_legacy_exception(self):
        shell = member('skin', [71])
        receipt = capture_authored_skin_ownership([shell, member('accessory', [71])], shell)
        self.assertFalse(receipt['validated'])
        self.assertFalse(forehead_consumer_skin_ownership([71], self.report(receipt))['available'])
        receipt['validated'] = True
        with self.assertRaises(ValueError):
            forehead_consumer_skin_ownership([71], self.report(receipt))

    def test_unknown_member_ownership_blocks(self):
        shell = member('skin', [71])
        unknown = member('unknown', [])
        unknown.data.attributes = {}
        receipt = capture_authored_skin_ownership([shell, unknown], shell)
        self.assertFalse(receipt['validated'])
        self.assertFalse(forehead_consumer_skin_ownership([71], self.report(receipt))['available'])

    def test_malformed_or_missing_shell_attribute_unavailable_without_value_read(self):
        for missing in (False, True):
            shell = member('skin', [71])
            if missing:
                shell.data.attributes = {}
            else:
                shell.data.attributes['NormalRegion'] = SimpleNamespace(
                    domain='FACE', data_type='FLOAT_VECTOR',
                    data=[SimpleNamespace(vector=(0., 0., 0.))])
            receipt = capture_authored_skin_ownership([shell, member('other', [94])], shell)
            self.assertFalse(receipt['validated'])
            self.assertEqual(receipt['skin_regions'], [])
            self.assertFalse(forehead_consumer_skin_ownership([94], self.report(receipt))['available'])

    def test_missing_receipt_legacy_unavailable(self):
        self.assertEqual(forehead_consumer_skin_ownership([1, 2], {}),
                         {'available': False, 'reason': 'authored_skin_ownership_receipt_missing'})

    def test_duplicate_false_receipt_or_missing_live_label_rejects(self):
        receipt = self.receipt()
        with self.assertRaises(ValueError):
            forehead_consumer_skin_ownership([71], self.report(receipt))
        receipt['skin_regions'] = [71, 71]
        with self.assertRaises(ValueError):
            forehead_consumer_skin_ownership([71], self.report(receipt))

    def test_json_identity_and_changed_role_or_labels_bind(self):
        report = self.report(self.receipt())
        author = {'report': report, 'regions': [71, 72, 94]}
        before = copy.deepcopy(author)
        digest = forehead_consumer_binding(author, author, {}, {})
        self.assertEqual(digest, forehead_consumer_binding(json.loads(json.dumps(author)),
                                                         author, {}, {}))
        for change in ('name', 'labels'):
            other = copy.deepcopy(author)
            if change == 'name':
                other['report']['ear_shell_membership']['authored_skin_ownership']['object_name'] = 'other'
            else:
                other['regions'] = [94, 72, 71]
            self.assertNotEqual(digest, forehead_consumer_binding(other, other, {}, {}))
        self.assertEqual(before, author)
        self.assertNotIn('pointer', json.dumps(report))


if __name__ == '__main__':
    unittest.main()
