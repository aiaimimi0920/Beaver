"""Opt-in middle-row connection variants; never change protected source fields."""

MODES = ("baseline", "compact-outer")


def middle_row_connections(grid, mode="baseline"):
    if mode not in MODES:
        raise ValueError(f"Unknown band connection variant: {mode}")
    pairs = [(grid[1][1], grid[2][2]), (grid[1][2], grid[2][3])]
    if mode == "baseline":
        pairs.append((grid[1][4], grid[2][3]))
    return pairs
