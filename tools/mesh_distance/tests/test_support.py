"""Synthetic test setup; no model assets and no implicit build/download."""
import os
from pathlib import Path
import sys

DIRECTORY = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(DIRECTORY.parent))
from mesh_distance import TriangleBVH

RECEIPT = os.environ.get("MESH_DISTANCE_BUILD")
if not RECEIPT:
    raise RuntimeError("Set MESH_DISTANCE_BUILD to an explicit build.json receipt")


def mesh(vertices, triangles, **kwargs):
    return TriangleBVH(vertices, triangles, build_receipt=RECEIPT, **kwargs)
