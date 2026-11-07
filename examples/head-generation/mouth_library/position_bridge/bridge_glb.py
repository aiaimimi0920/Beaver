"""Pure-data GLB accessor reader and byte-preserving JSON span locator."""

import json, struct
import numpy as np
from bridge_common import *


def parse_glb(data):
    require(len(data) >= 20, "Truncated GLB")
    require(
        struct.unpack_from("<III", data) == (0x46546C67, 2, len(data)),
        "Invalid GLB header",
    )
    chunks = []
    off = 12
    while off < len(data):
        require(off + 8 <= len(data), "Truncated chunk header")
        length, typ = struct.unpack_from("<II", data, off)
        off += 8
        require(length % 4 == 0 and off + length <= len(data), "Invalid chunk length")
        chunks.append((typ, data[off : off + length]))
        off += length
    require(
        [x[0] for x in chunks] == [0x4E4F534A, 0x004E4942], "Expected JSON and BIN only"
    )
    text = chunks[0][1].decode("utf-8")
    g = json.loads(text)
    b = chunks[1][1]
    require(
        len(g["buffers"]) == 1 and "uri" not in g["buffers"][0],
        "Expected one embedded buffer",
    )
    require(
        all("uri" not in x for x in g.get("images", [])),
        "External images are unsupported",
    )
    require(
        len(b) - 3 <= g["buffers"][0]["byteLength"] <= len(b), "BIN length mismatch"
    )
    return g, b, text


def accessor(g, b, index):
    a = g["accessors"][index]
    require(
        "sparse" not in a and "bufferView" in a, "Sparse/missing bufferView unsupported"
    )
    v = g["bufferViews"][a["bufferView"]]
    require(v["buffer"] == 0, "Unexpected buffer")
    dt = np.dtype(DTYPES[a["componentType"]])
    w = WIDTHS[a["type"]]
    stride = v.get("byteStride", w * dt.itemsize)
    off = v.get("byteOffset", 0) + a.get("byteOffset", 0)
    require(
        stride >= w * dt.itemsize
        and off + (a["count"] - 1) * stride + w * dt.itemsize <= len(b),
        "Accessor overrun",
    )
    out = np.ndarray(
        (a["count"], w), dtype=dt, buffer=b, offset=off, strides=(stride, dt.itemsize)
    ).copy()
    return out[:, 0] if w == 1 else out


class JsonSpans:
    """Locate values without reserializing original JSON or changing untouched bytes."""

    def __init__(self, text):
        self.text = text
        self.spans = {}
        self.decoder = json.JSONDecoder()
        self.i = 0
        self.read(())

    def ws(self):
        while self.i < len(self.text) and self.text[self.i].isspace():
            self.i += 1

    def read(self, path):
        self.ws()
        start = self.i
        c = self.text[self.i]
        if c == "{":
            self.i += 1
            self.ws()
            if self.text[self.i] != "}":
                while True:
                    key, end = self.decoder.raw_decode(self.text, self.i)
                    self.i = end
                    self.ws()
                    require(self.text[self.i] == ":", "Invalid JSON colon")
                    self.i += 1
                    self.read(path + (key,))
                    self.ws()
                    if self.text[self.i] == "}":
                        break
                    require(self.text[self.i] == ",", "Invalid JSON comma")
                    self.i += 1
                    self.ws()
            self.i += 1
        elif c == "[":
            self.i += 1
            self.ws()
            j = 0
            if self.text[self.i] != "]":
                while True:
                    self.read(path + (j,))
                    j += 1
                    self.ws()
                    if self.text[self.i] == "]":
                        break
                    require(self.text[self.i] == ",", "Invalid JSON comma")
                    self.i += 1
                    self.ws()
            self.i += 1
        else:
            _, self.i = self.decoder.raw_decode(self.text, self.i)
        self.spans[path] = (start, self.i)
