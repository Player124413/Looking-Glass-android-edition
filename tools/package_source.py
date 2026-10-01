"""Create an audited local source-review ZIP and manifest. Never upload."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import zipfile

from check_source import ROOT, audit_data, working_files


def package(root, output):
    payload = {}
    for path in working_files(root):
        name = path.relative_to(root).as_posix()
        data = path.read_bytes()
        errors = audit_data(name, data)
        if errors:
            raise ValueError("\n".join(errors))
        payload[name] = data
    for name in ("Cargo.toml", "Cargo.lock", "build.rs", "src/main.rs", "Launch.cmd", "Setup.cmd"):
        if name not in payload:
            raise ValueError(f"Required source input missing: {name}")
    output = output.resolve()
    if output.exists() or output.with_suffix(".manifest.json").exists():
        raise ValueError(f"Refusing to overwrite an existing review package: {output}")
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in payload.items():
            archive.writestr("looking-glass/" + name, data)
    hashes = {name: hashlib.sha256(data).hexdigest() for name, data in payload.items()}
    with zipfile.ZipFile(output) as archive:
        if archive.testzip() is not None:
            raise ValueError("Source package integrity check failed")
        actual = {name.removeprefix("looking-glass/"): hashlib.sha256(archive.read(name)).hexdigest()
                  for name in archive.namelist()}
        if actual != hashes:
            raise ValueError("Source package differs from the audited inputs")
    manifest = {
        "kind": "local source review, not a frozen release commit",
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root).decode().strip(),
        "sha256": hashlib.sha256(output.read_bytes()).hexdigest(),
        "files": hashes,
    }
    output.with_suffix(".manifest.json").write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    return manifest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "private/looking-glass-source-review.zip")
    args = parser.parse_args()
    manifest = package(ROOT, args.output)
    print(f"Verified local source review: {args.output} ({len(manifest['files'])} files).")
    print("Includes uncommitted work. Review and freeze a commit before publication.")


if __name__ == "__main__":
    main()
