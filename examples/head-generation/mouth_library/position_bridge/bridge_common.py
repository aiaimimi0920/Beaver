#!/usr/bin/env python3
"""Pinned private baseline loading, exactness checks and exclusive output helpers."""

from __future__ import annotations
import argparse, copy, hashlib, json, math, os, pathlib, struct
import numpy as np

ROOT = pathlib.Path(__file__).resolve().parent
SOURCE_SHA = "e736cb520f2eaac8d33e4ba9bb17c953db3cb38092f9f843096f4dec3a00d7eb"
BASELINE_PINS = {
    "native-after-reopen-snapshot.json": "b32d48857f4a2cc963b1f0a864e88b2410c1b93327a248bf5eccf6e8b316eb5d",
    "native-after-reopen-snapshot.npz": "d781fdc711433a990a01ae02cf83cb69a7f9d17086878784530e7dc116a12313",
    "mouth-ring-manifest.json": "8e131a3bc5d57b9be2b3f83dd68bb17a2a8786cb5f2979f439be569ca948d9c2",
}
TARGET_TOLERANCE_M = 2e-7
DTYPES = {5120: "i1", 5121: "u1", 5122: "<i2", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
WIDTHS = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}


class Rejected(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise Rejected(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def file_sha(path):
    return sha(pathlib.Path(path).read_bytes())


def array_sha(a):
    return sha(np.ascontiguousarray(a).tobytes())


def exact(a, b):
    return a.dtype == b.dtype and a.shape == b.shape and a.tobytes() == b.tobytes()


def exclusive_bytes(path, data):
    path = pathlib.Path(path)
    with path.open("xb") as f:
        f.write(data)


def exclusive_json(path, data):
    exclusive_bytes(
        path,
        (
            json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False) + "\n"
        ).encode(),
    )


def load_baseline(baseline_dir=None):
    baseline_dir = (
        pathlib.Path(baseline_dir) if baseline_dir is not None else ROOT / "baseline"
    )
    for name, pin in BASELINE_PINS.items():
        require(
            file_sha(baseline_dir / name) == pin,
            "Pinned baseline was modified: " + name,
        )
    info = json.loads((baseline_dir / "native-after-reopen-snapshot.json").read_text())
    with np.load(
        baseline_dir / "native-after-reopen-snapshot.npz", allow_pickle=False
    ) as f:
        arrays = {k: f[k] for k in f.files}
    manifest = json.loads((baseline_dir / "mouth-ring-manifest.json").read_text())
    return info, arrays, manifest


def to_gltf(a):
    out = np.asarray(a)[:, [0, 2, 1]].copy()
    out[:, 2] *= -1
    return out


def to_blender(a):
    out = np.asarray(a)[:, [0, 2, 1]].copy()
    out[:, 1] *= -1
    return out
