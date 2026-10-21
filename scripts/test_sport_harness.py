import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import run_sport_harness as harness


class SportHarnessTests(unittest.TestCase):
    def test_empty_failed_ignored_and_missing_test_summaries_fail_closed(self):
        for output in ["", "build succeeded", "test result: ok. 0 passed; 0 failed; 0 ignored;",
                       "test result: FAILED. 1 passed; 1 failed; 0 ignored;",
                       "test result: ok. 1 passed; 0 failed; 1 ignored;"]:
            with self.subTest(output=output), self.assertRaises(ValueError):
                harness.counts(output)
        self.assertEqual(harness.counts("test result: ok. 2 passed; 0 failed; 0 ignored;"),
                         {"passed": 2, "failed": 0, "ignored": 0})

    def test_missing_duplicate_and_unasserted_eval_cases_fail_closed(self):
        expected = {"pickle-sideout": "pickleball", "padel-deuce": "padel"}
        valid = {"version": 1, "cases": [{"id": key, "sport": sport, "assertions": 3} for key, sport in expected.items()]}
        self.assertEqual(len(harness.eval_results("SPORT_EVAL_REPORT " + json.dumps(valid), expected)), 2)
        for changed in [dict(valid, cases=valid["cases"][:1]), dict(valid, cases=valid["cases"] * 2),
                        dict(valid, cases=[dict(c, assertions=0) for c in valid["cases"]])]:
            with self.assertRaises(ValueError):
                harness.eval_results("SPORT_EVAL_REPORT " + json.dumps(changed), expected)

    def test_fixture_cannot_silently_omit_a_sport(self):
        with tempfile.TemporaryDirectory() as root:
            path = Path(root) / "cases.json"
            path.write_text(json.dumps({"version": 1, "cases": [{"id": "one", "sport": "tennis"}]}))
            with self.assertRaises(ValueError):
                harness.expected_cases(path)

    def test_nonzero_exit_overrides_a_green_looking_log_and_preserves_it(self):
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / "gate.log"
            gate = harness.run_gate("regression", [sys.executable, "-c", "print('test result: ok. 9 passed; 0 failed; 0 ignored;'); raise SystemExit(7)"], log, 10, os.environ, {})
            self.assertEqual((gate["status"], gate["returncode"]), ("failed", 7))
            self.assertIn("9 passed", log.read_text())

    def test_e2e_requires_every_named_sport_not_just_a_green_count(self):
        with tempfile.TemporaryDirectory() as root:
            gate = harness.run_gate("e2e", [sys.executable, "-c", "print('test result: ok. 9 passed; 0 failed; 0 ignored;')"], Path(root) / "e2e.log", 10, os.environ, {})
            self.assertEqual(gate["status"], "failed")
            self.assertIn("nine named", gate["error"])

    def test_e2e_keeps_all_nine_journeys_required_when_controller_regressions_are_added(self):
        output = "\n".join(f"test ui::e2e::{sport.replace('-', '_')} ... ok" for sport in sorted(harness.SPORTS))
        output += "\ntest ui::operators::tests::keyboard ... ok\ntest result: ok. 10 passed; 0 failed; 0 ignored;"
        with tempfile.TemporaryDirectory() as root:
            gate = harness.run_gate("e2e", [sys.executable, "-c", f"print({output!r})"], Path(root) / "e2e.log", 10, os.environ, {})
            self.assertEqual(gate["status"], "passed")
            self.assertEqual(gate["tests"]["passed"], 10)
            output = output.replace("test ui::e2e::padel ... ok", "test ui::operators::tests::another ... ok")
            gate = harness.run_gate("e2e", [sys.executable, "-c", f"print({output!r})"], Path(root) / "missing.log", 10, os.environ, {})
            self.assertEqual(gate["status"], "failed")

    def test_timeout_is_a_failure_with_partial_output_preserved(self):
        with tempfile.TemporaryDirectory() as root, patch.object(harness.subprocess, "run", side_effect=subprocess.TimeoutExpired(["cargo"], 1, output=b"partial build\n")):
            log = Path(root) / "timeout.log"
            gate = harness.run_gate("eval", ["cargo"], log, 1, os.environ, {})
            self.assertEqual(gate["status"], "failed")
            self.assertIn("Timed out", gate["error"])
            self.assertEqual(log.read_text(), "partial build\n")

    def test_failed_preflight_replaces_old_green_report_and_returns_nonzero(self):
        with tempfile.TemporaryDirectory() as root:
            report = Path(root) / "report.json"
            fixture = Path(root) / "empty.json"
            report.write_text('{"status":"passed"}')
            fixture.write_text('{"version":1,"cases":[]}')
            self.assertEqual(harness.main(["--report", str(report), "--eval-fixture", str(fixture)]), 1)
            self.assertEqual(json.loads(report.read_text())["status"], "failed")


if __name__ == "__main__":
    unittest.main()
