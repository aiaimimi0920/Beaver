"""Explicit offline GCC build; never called by import or a generator dispatch."""
import argparse
import ctypes
import hashlib
import json
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ABI = 1
SOURCE_NAMES = ("distance_api.h", "distance_api.cpp", "bvh_index.hpp", "triangle_kernel.hpp", "numeric_domain.hpp")
FLAGS = ("-std=c++17", "-O3", "-fPIC", "-shared", "-ffp-contract=off",
         "-fno-fast-math", "-frounding-math", "-pthread", "-Wall", "-Wextra", "-Werror")
LINK_FLAGS = ("-lquadmath",)
DIRECTORY = Path(__file__).resolve().parent


def sha256(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def source_identity():
    sources = {name: sha256(DIRECTORY / name) for name in SOURCE_NAMES}
    encoded = json.dumps(sources, sort_keys=True, separators=(",", ":")).encode()
    return sources, hashlib.sha256(encoded).hexdigest()


def build(output_dir, compiler="g++"):
    if (platform.system() != "Linux" or ctypes.sizeof(ctypes.c_void_p) != 8
            or platform.machine() != "x86_64"):
        raise RuntimeError("Only x86_64 Linux/GCC is currently supported")
    compiler = shutil.which(compiler)
    if compiler is None:
        raise RuntimeError("Existing GCC compiler required; no installation is performed")
    output = Path(output_dir).resolve()
    output.mkdir(parents=True, exist_ok=True)
    sources, digest = source_identity()
    with tempfile.TemporaryDirectory(prefix="bmd-build-", dir=output) as temporary:
        binary = Path(temporary) / "libbeaver_mesh_distance.so"
        command = [compiler, *FLAGS, f'-DBEAVER_DISTANCE_SOURCE_SHA="{digest}"',
                   str(DIRECTORY / "distance_api.cpp"), *LINK_FLAGS, "-o", str(binary)]
        subprocess.run(command, check=True)
        receipt = {"schema": "BEAVER_MESH_DISTANCE_BUILD_V1", "abi": ABI,
                   "sources": sources, "source_digest": digest,
                   "compiler": str(Path(compiler).resolve()),
                   "compiler_version": subprocess.check_output(
                       [compiler, "--version"], text=True).splitlines()[0],
                   "flags": list(FLAGS), "link_flags": list(LINK_FLAGS),
                   "numeric_domain_version": 5, "machine": platform.machine(),
                   "system": platform.system(), "binary": binary.name,
                   "binary_sha256": sha256(binary)}
        target = output / binary.name
        binary.replace(target)
        manifest = output / "build.json"
        staged = Path(temporary) / manifest.name
        staged.write_text(json.dumps(receipt, indent=2) + "\n", encoding="utf-8")
        staged.replace(manifest)
    return manifest


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", required=True)
    parser.add_argument("--compiler", default="g++")
    arguments = parser.parse_args()
    print(build(arguments.output_dir, arguments.compiler))
