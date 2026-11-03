"""Consumer wiring preserves disabled behavior and fails closed on identity loss."""
import unittest
from unittest.mock import patch
import forehead_cache_fit as module
from forehead_cache_schema import FOREHEAD_CACHE_NAMES


class CacheFitTests(unittest.TestCase):
    def test_disabled_stage_keeps_derivative_identity(self):
        derivative = lambda values: values
        with patch.object(module, 'forehead_cache_source_manifest', side_effect=AssertionError('unexpected manifest')):
            wrapped, metadata = module.forehead_install_jacobian_cache(None, derivative, {}, {})
        self.assertIs(wrapped, derivative)
        self.assertIsNone(metadata)

    def test_missing_verified_source_never_falls_back_uncached(self):
        state = dict.fromkeys(FOREHEAD_CACHE_NAMES)
        with patch.object(module, 'forehead_cache_source_manifest', side_effect=ValueError('source mismatch')):
            with self.assertRaisesRegex(ValueError, 'source mismatch'):
                module.forehead_install_jacobian_cache(object(), lambda x: x, state, {})

    def test_missing_dependency_is_not_silently_ignored(self):
        with self.assertRaises(KeyError):
            module.forehead_install_jacobian_cache(object(), lambda x: x, {}, {})


if __name__ == '__main__':
    unittest.main()
