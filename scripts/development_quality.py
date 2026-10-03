"""Classify development quality reports without masking tool failures."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def require(condition, message):
    if not condition:
        raise ValueError(message)


def classify(kind, code, stdout, stderr, report=None):
    # rust-toolchain enables ANSI color in hosted CI; retain raw logs unchanged.
    stdout = re.sub(r"\x1b\[[0-9;]*m", "", stdout)
    stderr = re.sub(r"\x1b\[[0-9;]*m", "", stderr)
    if kind == "rustfmt":
        require(code in (0, 1), "rustfmt execution failed")
        require(not stderr.strip(), "rustfmt emitted diagnostics on stderr")
        if code == 0:
            require(not stdout.strip(), "unexpected rustfmt success output")
            return 0
        diffs = re.findall(r"(?m)^Diff in .+:\d+:\s*$", stdout)
        require(diffs, "rustfmt failed without a formatting diff")
        require(all(not line or line[0] in " +-" or
                    re.fullmatch(r"Diff in .+:\d+:\s*", line)
                    for line in stdout.splitlines()),
                "rustfmt emitted output outside the diff format")
        return len(diffs)
    if kind == "eslint":
        require(not stderr.strip(), "ESLint emitted an unexpected diagnostic")
        require(code in (0, 1), "ESLint execution failed")
        rows = json.loads(stdout)
        require(isinstance(rows, list) and rows, "ESLint report is empty")
        errors = warnings = 0
        for row in rows:
            require(isinstance(row, dict) and row.get("filePath"),
                    "invalid ESLint file entry")
            for key in ("errorCount", "warningCount", "fatalErrorCount"):
                require(type(row.get(key)) is int and row[key] >= 0,
                        "invalid ESLint counts")
            require(row["fatalErrorCount"] == 0, "ESLint parsing failed")
            require(isinstance(row.get("messages"), list), "missing ESLint messages")
            require(not any(m.get("fatal") for m in row["messages"]),
                    "ESLint parsing failed")
            require(len(row["messages"]) == row["errorCount"] + row["warningCount"],
                    "ESLint report counts disagree")
            errors += row["errorCount"]
            warnings += row["warningCount"]
        require((code == 1) == (errors > 0), "ESLint exit/report mismatch")
        return errors + warnings
    if kind == "clippy":
        require(code == 0, "Clippy compilation or execution failed")
        require(all(not line.strip() or re.match(
            r"^\s*(Checking|Compiling|Finished|Fresh|Updating|Downloading|Downloaded|Locking|Adding|Blocking|warning:)(?:\s|:)", line)
                    for line in stderr.splitlines()), "unknown Cargo/Clippy diagnostic")
        events = [json.loads(line) for line in stdout.splitlines() if line.strip()]
        require(events and events[-1].get("reason") == "build-finished"
                and events[-1].get("success") is True, "incomplete Clippy build report")
        messages = [x["message"] for x in events if x.get("reason") == "compiler-message"]
        require(not any(m.get("level") == "error" for m in messages),
                "Clippy compiler error")
        return sum(m.get("level") == "warning" for m in messages)
    require(kind in ("loom-lines", "beaver-lines"), "unknown report kind")
    require(code in (0, 1), "line scanner execution failed")
    require(isinstance(report, dict), "missing line report")
    violations = report.get("violations")
    require(isinstance(violations, list) and all(isinstance(v, str) for v in violations),
            "invalid line findings")
    require((code == 1) == bool(violations), "line exit/report mismatch")
    if kind == "loom-lines":
        require(report.get("schemaVersion") == 1 and report.get("mode") == "ratchet",
                "unexpected Loom report schema/mode")
        require(report.get("summary", {}).get("scanned", 0) > 0,
                "Loom scanned no sources")
        # Fail closed on scanner diagnostics and on future unknown finding forms.
        patterns = (
            r".+: oversized baseline file changed before reaching 700 lines",
            r".+: \d+ lines requires a current 501-700 exception",
        )
        require(all(any(re.fullmatch(p, v) for p in patterns) for v in violations),
                "Loom scan completeness/configuration diagnostic")
        require(isinstance(report.get("warnings"), list), "missing Loom debt report")
        return len(violations) + len(report["warnings"])
    require(isinstance(report.get("files"), list) and report["files"],
            "Beaver scanned no sources")
    require(type(report.get("ok")) is bool and report["ok"] == (code == 0),
            "Beaver report status mismatch")
    require(all(re.fullmatch(
        r".+: \d+ effective lines; (split into responsibility-owned modules \(no exception above 700\)|"
        r"split, or document a current 501-700 line exception with protective tests)", v)
                for v in violations), "Beaver scan configuration diagnostic")
    return len(violations) + sum(f.get("status") == "legacy" for f in report["files"])


def run(args):
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=False)
    result = {"kind": args.kind, "command": args.command, "status": "tool_error",
              "commit": os.environ.get("GITHUB_SHA"), "exit_code": None}
    exit_code = 2
    try:
        if args.report:
            require(not Path(args.report).exists(), "refusing a stale report")
        require(bool(args.command), "missing scanner command")
        process = subprocess.run(args.command, capture_output=True, timeout=args.timeout,
                                 check=False)
        result["exit_code"] = process.returncode
        # Raw bytes survive even when decoding/parsing fails.
        (output / "stdout.log").write_bytes(process.stdout)
        (output / "stderr.log").write_bytes(process.stderr)
        stdout = process.stdout.decode("utf-8-sig")
        stderr = process.stderr.decode("utf-8-sig")
        report = None
        if args.report:
            raw = Path(args.report).read_bytes()
            (output / "report.json").write_bytes(raw)
            report = json.loads(raw)
        count = classify(args.kind, process.returncode, stdout, stderr, report)
        result.update(status="findings" if count else "clean", findings=count)
        exit_code = 1 if count and os.environ.get("QUALITY_STRICT") == "true" else 0
    except (OSError, ValueError, TypeError, KeyError, subprocess.SubprocessError) as error:
        if isinstance(error, subprocess.TimeoutExpired):
            (output / "stdout.log").write_bytes(error.stdout or b"")
            (output / "stderr.log").write_bytes(error.stderr or b"")
        result["error"] = str(error)
        print(f"Quality reporting failed: {error}", file=sys.stderr)
    (output / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    summary = f"### {args.kind}: {result['status']}\n\n"
    summary += f"Original exit: {result['exit_code']}; findings/debt: {result.get('findings', 'unknown')}.\n"
    summary += "Raw output and structured evidence are retained in the quality artifact.\n"
    (output / "summary.md").write_text(summary, encoding="utf-8")
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a", encoding="utf-8") as handle:
            handle.write(summary)
    print(summary)
    return exit_code


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("kind", choices=("rustfmt", "eslint", "clippy", "loom-lines", "beaver-lines"))
    parser.add_argument("--output", required=True)
    parser.add_argument("--report")
    parser.add_argument("--timeout", type=int, default=1200)
    before, command = sys.argv[1:], []
    if "--" in before:
        index = before.index("--")
        before, command = before[:index], before[index + 1:]
    args = parser.parse_args(before)
    args.command = command
    return run(args)


if __name__ == "__main__":
    sys.exit(main())
