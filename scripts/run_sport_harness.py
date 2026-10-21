#!/usr/bin/env python3
"""Local-only sport regression, App E2E and deterministic eval evidence."""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent
SPORTS = {"basketball", "volleyball", "pickleball", "padel", "tennis", "badminton", "table-tennis", "football", "futsal"}
SUMMARY = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;")


def counts(output):
    rows = SUMMARY.findall(output)
    if not rows:
        raise ValueError("No Rust test summary; execution is unproven")
    passed = sum(int(row[1]) for row in rows)
    failed = sum(int(row[2]) for row in rows)
    ignored = sum(int(row[3]) for row in rows)
    if passed == 0 or failed or ignored or any(row[0] != "ok" or int(row[1]) == 0 for row in rows):
        raise ValueError("Empty, failed or ignored Rust test suite")
    return {"passed": passed, "failed": failed, "ignored": ignored}


def expected_cases(fixture):
    suite = json.loads(fixture.read_text())
    if suite.get("version") != 1 or not suite.get("cases"):
        raise ValueError("Unsupported or empty eval fixture")
    cases = {case["id"]: case["sport"].replace("_", "-") for case in suite["cases"]}
    if len(cases) != len(suite["cases"]) or set(cases.values()) != SPORTS:
        raise ValueError("Eval fixture requires unique IDs and all nine sports")
    return cases


def eval_results(output, expected):
    reports = re.findall(r"SPORT_EVAL_REPORT (\{[^\n]+\})", output)
    if len(reports) != 1:
        raise ValueError("Expected exactly one complete sport eval report")
    report = json.loads(reports[0])
    cases = report.get("cases", [])
    actual = {case["id"]: case["sport"] for case in cases}
    if report.get("version") != 1 or len(actual) != len(cases) or actual != expected:
        raise ValueError("Missing, duplicate or unexpected evaluated case")
    if any(type(case.get("assertions")) is not int or case["assertions"] <= 0 for case in cases):
        raise ValueError("Every evaluated case needs positive assertion evidence")
    return cases


def run_gate(name, command, log, timeout, env, expected):
    started = time.monotonic()
    result = {"name": name, "command": command, "status": "failed", "log": str(log)}
    output = ""
    try:
        process = subprocess.run(command, cwd=ROOT, env=env, text=True, stdout=subprocess.PIPE,
                                 stderr=subprocess.STDOUT, timeout=timeout, check=False)
        output = process.stdout
        result["returncode"] = process.returncode
        if process.returncode:
            raise ValueError(f"Command exited {process.returncode}")
        result["tests"] = counts(output)
        if name == "e2e":
            ran = {name.replace("_", "-") for name in re.findall(r"test ui::e2e::([a-z_]+) \.\.\. ok", output)}
            if ran != SPORTS or result["tests"]["passed"] != 9:
                raise ValueError("All nine named App E2E journeys must execute")
        if name == "eval":
            result["cases"] = eval_results(output, expected)
        result["status"] = "passed"
    except subprocess.TimeoutExpired as error:
        output = error.stdout or ""
        if isinstance(output, bytes):
            output = output.decode(errors="replace")
        result["error"] = f"Timed out after {timeout} seconds"
    except (OSError, ValueError, KeyError, TypeError) as error:
        result["error"] = str(error)
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(output)
    result["seconds"] = round(time.monotonic() - started, 3)
    return result


def write_report(path, report):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(report, indent=2) + "\n")
    temporary.replace(path)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lane", choices=["all", "regression", "e2e", "eval"], default="all")
    parser.add_argument("--report", type=Path, default=ROOT / "artifacts/sport-harness/report.json")
    parser.add_argument("--eval-fixture", type=Path, default=ROOT / "tests/fixtures/sport-evals.json")
    parser.add_argument("--timeout", type=int, default=180, help="Per-lane timeout in seconds")
    args = parser.parse_args(argv)
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    report_path = args.report.resolve()
    report = {"version": 1, "status": "running", "scope": args.lane,
              "host": f"{platform.system()} {platform.release()} {platform.machine()}",
              "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "gates": []}
    write_report(report_path, report)  # A new interrupted run must not leave an old green report.
    try:
        fixture = args.eval_fixture.resolve()
        expected = expected_cases(fixture)
        revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, check=True, text=True, capture_output=True, timeout=5).stdout.strip()
        dirty = subprocess.run(["git", "status", "--porcelain"], cwd=ROOT, check=True, text=True, capture_output=True, timeout=5).stdout
        report["source"] = {"head": revision, "dirty": bool(dirty), "fixture_sha256": hashlib.sha256(fixture.read_bytes()).hexdigest()}
        cargo = shutil.which("cargo")
        if not cargo:
            raise ValueError("cargo is unavailable; configure the Rust toolchain PATH")
        base = [cargo, "+stable", "test", "--locked"]
        regression = base + ["--no-default-features"]
        for suite in ["racquet_operators", "operators", "sports", "sport_storage", "operator_storage", "field_sports", "field_session"]:
            regression.extend(["--test", suite])
        commands = {"regression": regression,
                    "e2e": base + ["--bin", "hustlx-oss", "ui::e2e::", "--", "--nocapture", "--test-threads=1"],
                    "eval": base + ["--no-default-features", "--test", "sport_evals", "--", "--nocapture", "--test-threads=1"]}
        env = dict(os.environ, CARGO_TERM_COLOR="never", HUSTLX_OSS_EVAL_FIXTURE=str(fixture))
        # Test-only child-process mode must never leak into a parent harness run.
        env.pop("HUSTLX_OSS_E2E_REOPEN", None)
        for name, command in commands.items():
            if args.lane not in ("all", name):
                continue
            gate = run_gate(name, command, report_path.parent / f"{name}.log", args.timeout, env, expected)
            report["gates"].append(gate)
            print(f"{name}: {gate['status']}" + (f" ({gate['error']})" if "error" in gate else ""))
            write_report(report_path, report)
            if gate["status"] != "passed":
                break
        required = 3 if args.lane == "all" else 1
        report["status"] = "passed" if len(report["gates"]) == required and all(g["status"] == "passed" for g in report["gates"]) else "failed"
    except (OSError, ValueError, KeyError, TypeError, subprocess.SubprocessError) as error:
        report.update(status="failed", error=str(error))
    write_report(report_path, report)
    print(f"Report: {report_path}")
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
