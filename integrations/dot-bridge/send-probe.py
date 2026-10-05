"""Send one explicitly approved, hash-pinned smoke-test outbox to a private Site.

Run only after verifying the target Site is owner-private. Supply an existing,
one-use platform credential through stdin; never put it in a file or argument.
The receipt proves transport only, not dot processing or Beaver Core import.
"""
import argparse
import hashlib
import json
from pathlib import Path
import sys
import termios
import urllib.error
import urllib.parse
import urllib.request


def validate(outbox, expected_hash, expected_id, site):
    raw = Path(outbox).read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != expected_hash:
        raise ValueError("Outbox hash mismatch")
    task = json.loads(raw)
    required = {
        "request_id": expected_id,
        "queue_id": "smoke-test",
        "kind": "connectivity_check",
        "expected_result": "BEAVER_DOT_SMOKE_OK",
    }
    if any(task.get(k) != v for k, v in required.items()):
        raise ValueError("Not the approved fixed-marker request")
    url = urllib.parse.urlsplit(site)
    if (url.scheme != "https" or not (url.hostname or "").endswith(".chatgpt.site")
            or url.username or url.password or url.port or url.query or url.fragment
            or url.path not in ("", "/")):
        raise ValueError("Expected the verified private Sites origin")
    return site.rstrip("/") + "/api/demo", digest


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        raise RuntimeError("Redirect refused")


def read_credential():
    old = termios.tcgetattr(sys.stdin.fileno()) if sys.stdin.isatty() else None
    if old:
        hidden = list(old)
        hidden[3] &= ~termios.ECHO
        termios.tcsetattr(sys.stdin.fileno(), termios.TCSANOW, hidden)
    print("Ready for one-use Site access on stdin; input is hidden.", flush=True)
    try:
        return json.loads(sys.stdin.readline())["token"]
    finally:
        if old:
            termios.tcsetattr(sys.stdin.fileno(), termios.TCSANOW, old)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ("outbox", "sha256", "request-id", "site", "receipt"):
        parser.add_argument("--" + name, required=True)
    args = parser.parse_args()
    url, digest = validate(args.outbox, args.sha256, args.request_id, args.site)
    token = read_credential()
    request = urllib.request.Request(url, method="POST",
        data=json.dumps({"request_id": args.request_id}).encode(), headers={
            "Content-Type": "application/json",
            "OAI-Sites-Authorization": "Bearer " + token,
        })
    try:
        with urllib.request.build_opener(NoRedirect).open(request, timeout=25) as response:
            result = {"http_status": response.status, "response": json.load(response)}
    except urllib.error.HTTPError as error:
        result = {"http_status": error.code, "error": "Site rejected the request"}
    except Exception as error:
        result = {"error_type": type(error).__name__, "error": "Delivery unconfirmed"}
    result.update(request_id=args.request_id, beaver_outbox_sha256=digest)
    with Path(args.receipt).open("x") as output:
        json.dump(result, output, indent=2)
    print(json.dumps(result), flush=True)


if __name__ == "__main__":
    main()
