"""Integrator provenance check: does the added text of a diff repeat the original game text?

Usage:
  python tools/provenance_check.py [--data <base>] --range <a>...<b>     added lines of a git range
  python tools/provenance_check.py [--data <base>] --staged              added lines of the index

Every added line is normalised to lowercase words. Any run of N consecutive words (default 6) that also
occurs in a text file of the user's data packs (scripts, dialogue tables, AI and config text) is a hit.
Hits are reported by repository path and line only. The matched original text is never printed, so the
tool cannot leak it into logs or commits. Exit code 0 means no hit, 1 means at least one hit (a DG-10
provenance question: stop and rewrite the line in your own words), 2 means the check could not run.

A quoted literal naming an actual mounted asset is an identifier, which the repository's
provenance policy permits. Exclude that literal's path tokens, but continue checking the
surrounding line. Unknown paths and longer text containing a path are not excluded.

The word index is cached under private/provenance-index-<N>.pkl (gitignored) and rebuilt when the data
packs change. This reads the data only; it never writes to the data or the pk3 archives.
"""
import argparse
import hashlib
import os
import pickle
import re
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import alice_data  # noqa: E402

REPO = Path(__file__).resolve().parents[1]
WORD = re.compile(r"[a-z0-9']+")
SKIP_LINE_PREFIXES = ("+++",)
QUOTED_LITERAL = re.compile(r'"([^"\r\n]*)"')


def words(text):
    return WORD.findall(text.lower())


def without_asset_identifiers(line, known_paths):
    def replace(match):
        candidate = match.group(1).replace("\\", "/").lower()
        if candidate in known_paths:
            return '"<asset-identifier>"'
        return match.group(0)
    return QUOTED_LITERAL.sub(replace, line)


def pack_signature(base):
    h = hashlib.sha256()
    for name in sorted(os.listdir(base)):
        if name.lower().endswith(".pk3"):
            st = os.stat(os.path.join(base, name))
            h.update(f"{name}:{st.st_size}:{int(st.st_mtime)}".encode())
    return h.hexdigest()


def build_index(base, n):
    cache = REPO / "private" / f"provenance-index-{n}.pkl"
    sig = pack_signature(base)
    if cache.is_file():
        try:
            with cache.open("rb") as f:
                stored_sig, grams = pickle.load(f)
            if stored_sig == sig:
                return grams
        except Exception:
            pass
    files = alice_data.mount()
    stable = set()   # stable digests: Python's hash() of str tuples is salted per process
    for key in files:
        if key.endswith(alice_data.TEXT_EXT):
            w = words(alice_data.text(key))
            for i in range(len(w) - n + 1):
                stable.add(hashlib.blake2b(" ".join(w[i:i + n]).encode(), digest_size=8).digest())
    cache.parent.mkdir(parents=True, exist_ok=True)
    with cache.open("wb") as f:
        pickle.dump((sig, stable), f)
    return stable


def added_lines(args):
    cmd = ["git", "diff", "--no-color", "-U0"]
    cmd += ["--cached"] if args.staged else [args.range]
    out = subprocess.check_output(cmd, cwd=REPO).decode("utf-8", "replace")
    path, lineno = None, 0
    for line in out.splitlines():
        if line.startswith("+++ b/"):
            path = line[6:]
        elif line.startswith("@@"):
            m = re.search(r"\+(\d+)", line)
            lineno = int(m.group(1)) if m else 0
        elif line.startswith("+") and not line.startswith(SKIP_LINE_PREFIXES):
            yield path, lineno, line[1:]
            lineno += 1


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--data")
    ap.add_argument("--range")
    ap.add_argument("--staged", action="store_true")
    ap.add_argument("-n", type=int, default=6)
    args = ap.parse_args()
    if bool(args.range) == bool(args.staged):
        ap.error("give exactly one of --range or --staged")
    alice_data._base = alice_data.resolve_base(args.data)
    index = build_index(alice_data._base, args.n)
    known_paths = alice_data.mount()
    hits = []
    scanned = 0
    for path, lineno, line in added_lines(args):
        w = words(without_asset_identifiers(line, known_paths))
        scanned += 1
        for i in range(len(w) - args.n + 1):
            if hashlib.blake2b(" ".join(w[i:i + args.n]).encode(), digest_size=8).digest() in index:
                hits.append((path, lineno))
                break
    print(f"provenance check: {scanned} added lines scanned against {len(index):,} {args.n}-word runs from the data")
    for path, lineno in hits:
        print(f"HIT {path}:{lineno}")
    if hits:
        print(f"{len(hits)} hit(s): rewrite these lines in your own words (DG-10); the matched text is deliberately not printed")
        return 1
    print("no run of %d or more consecutive words matches the original text" % args.n)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except subprocess.CalledProcessError as e:
        print(f"could not run: {e}")
        sys.exit(2)
