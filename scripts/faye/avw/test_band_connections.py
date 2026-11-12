"""Connection choices are opt-in and leave source grid/support data untouched."""

import unittest
import numpy as np
from band_connections import middle_row_connections


class ConnectionTests(unittest.TestCase):
    def setUp(self):
        self.grid = np.arange(40).reshape(4, 5, 2)

    def test_default_is_original_connections(self):
        result = middle_row_connections(self.grid)
        expected = [
            (self.grid[1][1], self.grid[2][2]),
            (self.grid[1][2], self.grid[2][3]),
            (self.grid[1][4], self.grid[2][3]),
        ]
        np.testing.assert_array_equal(result, expected)

    def test_compact_only_removes_outer_link(self):
        original = self.grid.copy()
        base = middle_row_connections(self.grid)
        compact = middle_row_connections(self.grid, "compact-outer")
        np.testing.assert_array_equal(compact, base[:2])
        np.testing.assert_array_equal(self.grid, original)

    def test_unknown_mode_rejected(self):
        with self.assertRaises(ValueError):
            middle_row_connections(self.grid, "silently-relax-fields")


if __name__ == "__main__":
    unittest.main()
