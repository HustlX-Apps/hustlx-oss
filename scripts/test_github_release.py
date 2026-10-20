import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

SPEC = importlib.util.spec_from_file_location("github_release", Path(__file__).with_name("github_release.py"))


class ReleaseTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tool = importlib.util.module_from_spec(SPEC)
        SPEC.loader.exec_module(cls.tool)

    def packages(self, root, version="0.1.0"):
        for target in self.tool.TARGETS:
            folder = root / target
            folder.mkdir()
            archive = folder / ("hustlx-oss-" + version + "-" + target + ".zip")
            with zipfile.ZipFile(archive, "w") as z:
                z.writestr("fixture.txt", "synthetic package")
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            (folder / "SHA256SUMS").write_text(digest + "  " + archive.name + "\n")

    def test_collect_requires_every_target_and_matching_checksums(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.packages(root)
            output = root / "out"
            assets = self.tool.collect(root, output, "0.1.0")
            self.assertEqual(len(assets), 4)
            self.assertEqual(len((output / "SHA256SUMS").read_text().splitlines()), 3)
            self.assertEqual(len(list(output.glob("*.zip"))), 3)
            (root / self.tool.TARGETS[0] / "SHA256SUMS").write_text("0" * 64 + "  wrong.zip\n")
            with self.assertRaises(ValueError):
                self.tool.collect(root, root / "bad", "0.1.0")
            self.assertFalse((root / "bad").exists())

    def test_missing_extra_and_tampered_archives_are_rejected(self):
        for mode in ["missing", "extra", "tampered"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                self.packages(root)
                archive = next(root.rglob("*.zip"))
                if mode == "missing":
                    archive.unlink()
                elif mode == "extra":
                    (archive.parent / "unexpected.zip").write_bytes(archive.read_bytes())
                else:
                    archive.write_bytes(b"tampered")
                with self.assertRaises(ValueError):
                    self.tool.collect(root, root / "out", "0.1.0")

    def test_publish_context_requires_manual_main_exact_sha_and_explicit_true(self):
        env = {"GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/main",
               "GITHUB_SHA": "a" * 40, "RELEASE_SHA": "a" * 40, "RELEASE_PUBLISH": "true",
               "GITHUB_REPOSITORY": "HustlX-Apps/hustlx-oss"}
        self.tool.require_publish_context(env)
        for key, value in [("RELEASE_PUBLISH", "false"), ("RELEASE_PUBLISH", "True"),
                           ("GITHUB_EVENT_NAME", "pull_request"), ("GITHUB_REF", "refs/heads/develop"),
                           ("RELEASE_SHA", "b" * 40), ("GITHUB_REPOSITORY", "someone/fork")]:
            with self.subTest(key=key, value=value), self.assertRaises(ValueError):
                self.tool.require_publish_context(dict(env, **{key: value}))

    def test_dispatch_defaults_to_build_only_and_dry_run_never_calls_github(self):
        with patch.object(self.tool, "run") as run, patch("builtins.print"):
            command = self.tool.dispatch("0.1.0", "a" * 40, "develop", False, True)
            run.assert_not_called()
            self.assertIn("publish=false", command)
            self.assertNotIn("publish=true", command)
            with self.assertRaises(ValueError):
                self.tool.dispatch("0.1.0", "a" * 40, "develop", True, True)

    def test_validation_rejects_wrong_version_or_sha_before_remote_operations(self):
        with tempfile.TemporaryDirectory() as directory:
            env = {"GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_REF": "refs/heads/develop",
                   "GITHUB_SHA": "a" * 40, "RELEASE_SHA": "a" * 40, "RELEASE_PUBLISH": "false",
                   "RELEASE_VERSION": "0.1.0", "GITHUB_OUTPUT": str(Path(directory) / "output")}
            with patch.object(self.tool, "run", return_value="0.1.0"), patch.object(self.tool, "gh") as gh:
                for key, value in [("RELEASE_VERSION", "0.2.0"), ("RELEASE_SHA", "b" * 40)]:
                    with self.subTest(key=key), self.assertRaises(ValueError):
                        self.tool.validate_context(dict(env, **{key: value}))
                self.tool.validate_context(env)
                self.assertEqual(Path(env["GITHUB_OUTPUT"]).read_text(), "version=0.1.0\nsha=" + "a" * 40 + "\n")
                gh.assert_not_called()

    def test_version_and_sha_reject_shell_and_path_input(self):
        for value in ["v0.1.0", "01.1.0", "1.0.0-01", "../x", "1.0.0;echo x", "1.0.0\n"]:
            with self.subTest(value=value), self.assertRaises(ValueError):
                self.tool.validate_version(value)
        self.tool.validate_version("1.2.3-beta.1")
        with self.assertRaises(ValueError):
            self.tool.dispatch("0.1.0", "main", "main", True, True)

    def test_draft_is_uploaded_before_publication_and_upload_failure_stays_draft(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.packages(root)
            assets = self.tool.collect(root, root / "out", "0.1.0")
            for fail_upload in [False, True]:
                calls = []

                def github(args, payload=None):
                    calls.append((args, payload))
                    if "upload" in args and fail_upload:
                        raise RuntimeError("upload failed")
                    if "matching-refs" in "/".join(args):
                        return []
                    if args[-1].endswith("generate-notes"):
                        return {"body": "Changes"}
                    if payload and payload.get("draft") is True:
                        return {"id": 123}
                    return {}

                with patch.object(self.tool, "gh", side_effect=github), patch("builtins.print"):
                    if fail_upload:
                        with self.assertRaises(RuntimeError):
                            self.tool.publish("0.1.0", "a" * 40, assets)
                    else:
                        self.tool.publish("0.1.0", "a" * 40, assets)
                creates = [p for a, p in calls if p and p.get("draft") is True]
                patches = [p for a, p in calls if p and p.get("draft") is False]
                self.assertEqual(len(creates), 1)
                self.assertEqual(creates[0]["target_commitish"], "a" * 40)
                self.assertEqual(len(patches), 0 if fail_upload else 1)
                if not fail_upload:
                    upload_at = next(i for i, (a, p) in enumerate(calls) if "upload" in a)
                    public_at = next(i for i, (a, p) in enumerate(calls) if p and p.get("draft") is False)
                    self.assertLess(upload_at, public_at)

    def test_existing_tag_is_never_overwritten(self):
        with patch.object(self.tool, "gh", return_value=[{"ref": "refs/tags/v0.1.0"}]) as gh:
            with self.assertRaises(ValueError):
                self.tool.publish("0.1.0", "a" * 40, [])
            self.assertEqual(gh.call_count, 1)

    def test_prerelease_never_becomes_latest(self):
        calls = []

        def github(args, payload=None):
            calls.append((args, payload))
            if "matching-refs" in "/".join(args):
                return []
            if args[-1].endswith("generate-notes"):
                return {"body": "Changes"}
            if payload and payload.get("draft") is True:
                return {"id": 123}
            return {}

        with patch.object(self.tool, "gh", side_effect=github), patch("builtins.print"):
            self.tool.publish("0.2.0-beta.1", "a" * 40, [])
        self.assertTrue(next(p for a, p in calls if p and p.get("draft") is True)["prerelease"])
        self.assertEqual(next(p for a, p in calls if p and p.get("draft") is False)["make_latest"], "false")


if __name__ == "__main__":
    unittest.main()
