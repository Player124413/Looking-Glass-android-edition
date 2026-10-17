"""Audit the Git index before a source commit. Read-only; never uploads."""
from pathlib import Path, PurePosixPath
import argparse
import hashlib
import json
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
ROOT_FILES = {
    ".gitignore", ".gitattributes", "Cargo.toml", "Cargo.lock", "README.md",
    "LEGAL.md", "LICENSE-MIT", "Setup.cmd", "CONTRIBUTING.md", "SECURITY.md",
    "THIRD_PARTY_NOTICES.txt", "build.rs",
}
SUFFIXES = {"src": {".rs", ".json"}, "docs": {".md"}, "tools": {".py", ".ps1", ".java", ".cmd", ".md"}}
# Authored public website files only; media retains its exact hash boundary.
SITE_FILES = {
    "docs/site/index.html", "docs/site/styles.css", "docs/site/favicon.svg",
    "docs/site/sitemap.xml",
}
# User-requested README footage. Exact files/content only, never a general
# allowance for original game assets or raw research captures.
GAMEPLAY_MEDIA = {
    "docs/media/gameplay/story-preview.gif": "697ceef7f8108cbfd3521c7892b9db5a9a6da8a79eb169aefac346547440db5d",
    "docs/media/gameplay/story-poster.jpg": "51c96dff0690f5908a786cfe2b4eed7453db79fea8c89e9d364a64f47037eeed",
    "docs/media/gameplay/dry-landing.mp4": "e04e476846a5e12b947ffba3616eefd5a8759d82cbd927182c9d0dc3d2da08fe",
    "docs/media/gameplay/dry-landing.gif": "b1fca02df8d1714c007d00932f644c778fafd69d9883475129135cc7049bd6c4",
    "docs/media/gameplay/pool-of-tears.mp4": "5840a570947d79472e9b4087344f06a3d35110fdf1d64f9c29b9826f49f4dc2b",
    "docs/media/gameplay/pool-of-tears.gif": "707bae12ba4cdbbff78d0b4588c717d7ee988b7fd70c2d0c395aa0b22f1cc839",
    "docs/media/gameplay/pale-realm.mp4": "864262da7717b8e193be1f6f6f4f994de90b1781b8c804da62869f678b9102b0",
    "docs/media/gameplay/pale-realm.gif": "56e0ef4b94b96493212ac2fe39369b458960d6c2212c2bf512246e5cad74d5fe",
}
# Launcher icons (see tools/android/res/ICON-NOTICE.md). Exact hashes only.
ANDROID_ICONS = {
    "tools/android/res/mipmap-hdpi/ic_launcher.png": "2a8f521c841763f1fd94fbb6dbb2bca80d58ac27d6300a3ab583e105da6be674",
    "tools/android/res/mipmap-hdpi/ic_launcher_round.png": "2a8f521c841763f1fd94fbb6dbb2bca80d58ac27d6300a3ab583e105da6be674",
    "tools/android/res/mipmap-mdpi/ic_launcher.png": "b00559e2271a07f1e58a434651be9e64d490ce2ac318f0334eea4b490ccd9ec3",
    "tools/android/res/mipmap-mdpi/ic_launcher_round.png": "b00559e2271a07f1e58a434651be9e64d490ce2ac318f0334eea4b490ccd9ec3",
    "tools/android/res/mipmap-xhdpi/ic_launcher.png": "4e6ef9e6d7a8c1c3620c4f3f482faa56ca818172f88d73b31809875d60595812",
    "tools/android/res/mipmap-xhdpi/ic_launcher_round.png": "4e6ef9e6d7a8c1c3620c4f3f482faa56ca818172f88d73b31809875d60595812",
    "tools/android/res/mipmap-xxhdpi/ic_launcher.png": "93e153d662885e3807d8413a943d8747a70f535fe3292dcc78a261882205a5aa",
    "tools/android/res/mipmap-xxhdpi/ic_launcher_round.png": "93e153d662885e3807d8413a943d8747a70f535fe3292dcc78a261882205a5aa",
    "tools/android/res/mipmap-xxxhdpi/ic_launcher.png": "56feea87b4ccf4ea2681a21276bf8c9c3bc64fcfc273ba51095becc82ef9fd38",
    "tools/android/res/mipmap-xxxhdpi/ic_launcher_round.png": "56feea87b4ccf4ea2681a21276bf8c9c3bc64fcfc273ba51095becc82ef9fd38",
}
SECRET = re.compile(
    rb"(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,}|"
    rb"AKIA[A-Z0-9]{16}|-----BEGIN (?:RSA |EC |OPENSSH )?PRIVATE KEY-----)"
)


def allowed(path):
    p = PurePosixPath(path)
    if p.is_absolute() or ".." in p.parts or "\\" in path:
        return False
    if path in GAMEPLAY_MEDIA or path in SITE_FILES or path in ANDROID_ICONS:
        return True
    if len(p.parts) == 1:
        return path in ROOT_FILES or (path.startswith("Launch") and p.suffix == ".cmd")
    if p.parts[0] in SUFFIXES:
        return p.suffix in SUFFIXES[p.parts[0]] and "__pycache__" not in p.parts
    return p.parts[:2] in {(".github", "workflows"), (".github", "ISSUE_TEMPLATE")} and p.suffix in {".yml", ".yaml", ".md"}


def size_limit(path):
    return (8 if path in GAMEPLAY_MEDIA and path.endswith(".mp4") else 2) * 1024 * 1024


def audit_data(path, data):
    errors = []
    if not allowed(path):
        errors.append(f"Not an approved source path: {path}")
    if len(data) > size_limit(path):
        errors.append(f"Unexpectedly large source file: {path}")
    reviewed = GAMEPLAY_MEDIA.get(path) or ANDROID_ICONS.get(path)
    if reviewed:
        if hashlib.sha256(data).hexdigest() != reviewed:
            errors.append(f"Unreviewed media content: {path}")
    elif b"\0" in data:
        errors.append(f"Binary content in source file: {path}")
    if SECRET.search(data):
        errors.append(f"Possible credential/private key in: {path}")
    if path.endswith(".json"):
        try:
            json.loads(data)
        except (ValueError, UnicodeDecodeError):
            errors.append(f"Invalid JSON source data: {path}")
    return errors


def working_files(root=ROOT):
    """Only Git-visible files; include new source, omit deleted and ignored files."""
    names = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root
    ).decode("utf-8").split("\0")
    result = []
    for name in sorted(set(names) - {""}):
        p = root / name
        if not p.exists() and not p.is_symlink():
            continue
        if p.is_symlink() or not p.is_file() or not p.resolve().is_relative_to(root.resolve()):
            raise ValueError(f"Not an ordinary source file: {name}")
        result.append(p)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--worktree", action="store_true", help="audit current tracked and new files without staging")
    args = parser.parse_args()
    if args.worktree:
        files = working_files()
        errors = []
        for p in files:
            errors.extend(audit_data(p.relative_to(ROOT).as_posix(), p.read_bytes()))
        if errors:
            print("Source worktree audit failed:\n" + "\n".join(errors))
            return 1
        print(f"Source worktree audit passed: {len(files)} ordinary allowlisted files.")
        print("This does not establish full-history or original-text provenance review.")
        return 0
    index = subprocess.check_output(["git", "ls-files", "--stage", "-z"], cwd=ROOT)
    errors = []
    count = total = 0
    for record in index.split(b"\0"):
        if not record:
            continue
        metadata, raw_path = record.split(b"\t", 1)
        mode, blob, stage = metadata.decode("ascii").split()
        path = raw_path.decode("utf-8")
        count += 1
        if mode not in {"100644", "100755"} or stage != "0" or not allowed(path):
            errors.append(f"Not an approved ordinary source file: {path}")
            continue
        size = int(subprocess.check_output(["git", "cat-file", "-s", blob], cwd=ROOT))
        if size > size_limit(path):
            errors.append(f"Unexpectedly large source file: {path}")
            continue
        data = subprocess.check_output(["git", "cat-file", "blob", blob], cwd=ROOT)
        total += len(data)
        errors.extend(audit_data(path, data))
    if errors:
        print("Source audit failed:\n" + "\n".join(errors))
        return 1
    print(f"Source index audit passed: {count} files, {total:,} bytes; no disallowed paths/binaries or recognised credential patterns.")
    print("This checks the index, not all history, provenance or every possible secret.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
