import unittest
from unittest.mock import patch
from construction_topology import digest
from ear_lower_context import prepare_reports
class ContextTests(unittest.TestCase):
    def test_valid_seal_stale_membership_rejected(self):
        membership={'targets':{'old':'context'}};membership['sha256']=digest(membership)
        with patch('ear_lower_context.combined_targets',return_value={'new':'context'}):
            with self.assertRaisesRegex(ValueError,'another attachment context'):
                prepare_reports({'ear_membership':membership},[])
if __name__=='__main__':unittest.main()
