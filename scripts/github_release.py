"""GitHub release dispatch and CI gates. Local dispatch defaults to build-only."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import zipfile

REPOSITORY = "HustlX-Apps/hustlx-oss"
ROOT = Path(__file__).resolve().parent.parent
TARGETS = ("aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc")


def validate_version(value):
    number = r"(?:0|[1-9][0-9]*)"
    identifier = r"[0-9A-Za-z-]+"
    pattern = number + r"\." + number + r"\." + number
    pattern += r"(?:-(" + identifier + r"(?:\." + identifier + r")*))?"
    pattern += r"(?:\+" + identifier + r"(?:\." + identifier + r")*)?"
    match = re.fullmatch(pattern, value)
    if not match:
        raise ValueError("Use a SemVer version without a v prefix.")
    if any(int(part) > 4294967295 for part in value.split("-")[0].split("+")[0].split(".")):
        raise ValueError("Version component overflow.")
    if match.group(1) and any(p.isdigit() and len(p) > 1 and p[0] == "0" for p in match.group(1).split(".")):
        raise ValueError("Numeric prerelease identifiers cannot have leading zeros.")
    return value


def validate_sha(value):
    if not re.fullmatch(r"[0-9a-f]{40}", value):
        raise ValueError("Use the full 40-character commit SHA.")
    return value


def run(args, payload=None):
    result = subprocess.run(args, cwd=ROOT, input=payload, text=True, capture_output=True, check=False)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip() or "Command failed: " + args[0])
    return result.stdout.strip()


def gh(args, payload=None):
    if payload is not None:
        args = args + ["--input", "-"]
    result = run(["gh"] + args, None if payload is None else json.dumps(payload))
    return json.loads(result) if result else None


def dispatch(version, sha, ref, publish=False, dry_run=False):
    validate_version(version)
    validate_sha(sha)
    if ref not in ("develop", "main") or (publish and ref != "main"):
        raise ValueError("Build from develop or main; publishing requires main.")
    command = ["gh", "workflow", "run", "release.yml", "--repo", REPOSITORY, "--ref", ref,
               "-f", "version=" + version, "-f", "source_sha=" + sha,
               "-f", "publish=" + str(publish).lower()]
    print(json.dumps(command))
    if not dry_run:
        run(command)
        print("Dispatched. Track the run with: gh run list --repo " + REPOSITORY + " --workflow release.yml")
    return command


def require_publish_context(env):
    if (env.get("GITHUB_EVENT_NAME") != "workflow_dispatch"
            or env.get("GITHUB_REF") != "refs/heads/main"
            or env.get("GITHUB_REPOSITORY") != REPOSITORY
            or env.get("RELEASE_PUBLISH") != "true"):
        raise ValueError("Publishing requires explicit publish=true dispatch on main in " + REPOSITORY)
    if validate_sha(env.get("RELEASE_SHA", "")) != env.get("GITHUB_SHA"):
        raise ValueError("Release SHA must equal the workflow's exact main commit.")


def validate_context(env):
    actual = validate_version(run(["cargo", "xtask", "version"]))
    sha = validate_sha(env["GITHUB_SHA"])
    if env["GITHUB_EVENT_NAME"] == "workflow_dispatch":
        if env.get("RELEASE_VERSION") != actual or env.get("RELEASE_SHA") != sha:
            raise ValueError("Requested version/SHA must exactly match this workflow checkout.")
        if env.get("RELEASE_PUBLISH") == "true":
            require_publish_context(env)
            refuse_existing_tag(actual)
    elif env["GITHUB_EVENT_NAME"] != "pull_request":
        raise ValueError("Only PR verification or manual dispatch is supported.")
    with Path(env["GITHUB_OUTPUT"]).open("a", encoding="utf-8") as output:
        output.write("version=" + actual + "\nsha=" + sha + "\n")
    return actual


def collect(source, destination, version):
    validate_version(version)
    expected = {"hustlx-oss-" + version + "-" + target + ".zip" for target in TARGETS}
    archives = list(source.rglob("*.zip"))
    if len(archives) != len(expected) or {p.name for p in archives} != expected:
        raise ValueError("Require exactly one version-matched archive for each release target.")
    entries = []
    for archive in sorted(archives):
        if archive.is_symlink() or not zipfile.is_zipfile(archive):
            raise ValueError("Invalid ZIP: " + archive.name)
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        entry = digest + "  " + archive.name
        if archive.with_name("SHA256SUMS").read_text(encoding="utf-8").strip() != entry:
            raise ValueError("Checksum mismatch: " + archive.name)
        with zipfile.ZipFile(archive) as package:
            if not package.namelist() or package.testzip() is not None:
                raise ValueError("Corrupt or empty ZIP: " + archive.name)
        entries.append(entry)
    destination.mkdir(parents=True, exist_ok=False)
    assets = []
    for archive in sorted(archives):
        dest = destination / archive.name
        shutil.copyfile(archive, dest)
        assets.append(dest)
    checksum = destination / "SHA256SUMS"
    checksum.write_text("\n".join(entries) + "\n", encoding="utf-8")
    return assets + [checksum]


def refuse_existing_tag(version):
    refs = gh(["api", "repos/" + REPOSITORY + "/git/matching-refs/tags/v" + version])
    if any(ref["ref"] == "refs/tags/v" + version for ref in refs):
        raise ValueError("Release tag already exists. Never overwrite it; inspect any partial draft before recovery.")


def publish(version, sha, assets):
    validate_version(version)
    validate_sha(sha)
    refuse_existing_tag(version)
    tag = "v" + version
    endpoint = "repos/" + REPOSITORY
    generated = gh(["api", "--method", "POST", endpoint + "/releases/generate-notes"],
                   {"tag_name": tag, "target_commitish": sha})
    # Atomic ref creation refuses a concurrent/existing tag instead of moving it.
    gh(["api", "--method", "POST", endpoint + "/git/refs"], {"ref": "refs/tags/" + tag, "sha": sha})
    prerelease = "-" in version.split("+")[0]
    body = (generated["body"] + "\n\nSource commit: `" + sha + "`.\n\n"
            "macOS Apple Silicon and Intel app ZIPs; Windows x64 portable EXE ZIP. "
            "Verify downloads with SHA256SUMS. macOS is ad-hoc signed, not Developer ID signed "
            "or notarized. Windows is unsigned. No installers are included. "
            "This application is in development; packaging is not certification for official game operation.\n")
    release = gh(["api", "--method", "POST", endpoint + "/releases"],
                 {"tag_name": tag, "target_commitish": sha, "name": "HustlX OSS " + tag,
                  "body": body, "draft": True, "prerelease": prerelease})
    # No --clobber: a failed upload leaves the draft for explicit maintainer recovery.
    gh(["release", "upload", tag, "--repo", REPOSITORY] + [str(p) for p in assets])
    gh(["api", "--method", "PATCH", endpoint + "/releases/" + str(release["id"])],
       {"draft": False, "make_latest": "false" if prerelease else "true"})
    print("Published https://github.com/" + REPOSITORY + "/releases/tag/" + tag)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    dispatch_parser = commands.add_parser("dispatch", help="Start builds; publishing requires --publish and --ref main")
    dispatch_parser.add_argument("--version", required=True)
    dispatch_parser.add_argument("--sha", required=True)
    dispatch_parser.add_argument("--ref", choices=["develop", "main"], default="develop")
    dispatch_parser.add_argument("--publish", action="store_true")
    dispatch_parser.add_argument("--dry-run", action="store_true")
    commands.add_parser("validate", help="CI-only exact source/version validation")
    for command in ("collect", "publish"):
        sub = commands.add_parser(command, help="CI-only verified asset " + command)
        sub.add_argument("--version", required=True)
        sub.add_argument("--source", type=Path, default=Path("artifacts"))
        sub.add_argument("--destination", type=Path, default=Path("release-assets"))
    args = parser.parse_args()
    if args.command == "dispatch":
        dispatch(args.version, args.sha, args.ref, args.publish, args.dry_run)
    elif args.command == "validate":
        validate_context(os.environ)
    else:
        if args.command == "publish":
            require_publish_context(os.environ)
        assets = collect(args.source, args.destination, args.version)
        if args.command == "publish":
            publish(args.version, os.environ["RELEASE_SHA"], assets)
        else:
            print("Verified " + str(len(assets) - 1) + " archives and combined SHA256SUMS.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, RuntimeError, OSError, KeyError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
