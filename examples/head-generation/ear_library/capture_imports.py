"""Prepare public per-image Skip File settings for bound review screenshots only."""

import hashlib
import json
from pathlib import Path
import re

from PIL import Image

SKIP = '[remap]\n\nimporter="skip"\n'
CAPTURE = re.compile(r"artifacts/(blink_probe|face_support_probe|mouth_motion|ear_spatial)/[^/]+/frame_[0-9]{3}\.png")


def requests(workspace, manifests):
    root = Path(workspace).resolve()
    rows, seen = [], set()
    for manifest in manifests:
        data = json.loads(Path(manifest).read_text())
        for row in data["captures"]:
            image = row.get("image", row.get("path", ""))
            relative = image.removeprefix("res://")
            if not CAPTURE.fullmatch(relative) or relative in seen:
                raise ValueError("Not a unique review screenshot")
            path = (root / relative).resolve()
            if not path.is_relative_to(root) or not path.is_file():
                raise ValueError("Screenshot missing or escaping workspace")
            content = path.read_bytes()
            if not content.startswith(b'\x89PNG\r\n\x1a\n'):
                raise ValueError("Not a PNG capture")
            try:
                with Image.open(path) as png:
                    if png.format != "PNG":
                        raise ValueError("Not a PNG capture")
                    png.verify()
                with Image.open(path) as png:
                    png.load()
            except (OSError, SyntaxError) as error:
                raise ValueError("Invalid PNG capture") from error
            digest = hashlib.sha256(content).hexdigest()
            if digest != row.get("sha256"):
                raise ValueError("Screenshot differs from manifest")
            sidecar = root / (relative + '.import')
            if sidecar.is_symlink():
                raise ValueError("Linked sidecar rejected")
            previous = sidecar.read_bytes() if sidecar.exists() else None
            seen.add(relative)
            if previous == SKIP.encode():
                continue
            rows.append((relative, digest, hashlib.sha256(previous).hexdigest() if previous is not None else None))
    result = []
    for start in range(0, len(rows), 64):
        batch = rows[start:start + 64]
        code = 'import hashlib\nfrom pathlib import Path\n'
        for path, digest, _ in batch:
            code += 'assert hashlib.sha256(Path(beaver_input(' + repr(path) + ')).read_bytes()).hexdigest() == ' + repr(digest) + '\n'
            code += 'Path(beaver_output(' + repr(path + '.import') + ')).write_text(' + repr(SKIP) + ', encoding="utf-8")\n'
        result.append({'code': code, 'outputs': [{'path': p + '.import', 'expectedSha256': old} for p, _, old in batch], 'timeoutSeconds': 60})
    return result
