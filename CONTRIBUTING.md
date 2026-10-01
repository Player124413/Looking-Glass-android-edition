# Contributing to Looking Glass

Looking Glass is an experimental public preview. Please
read the README and known issues before changing behavior or reporting a bug.

## Source boundary

Commit newly authored code, identifiers, numbers and paraphrased research only.
Do not add original game archives, executables, assets, scripts, subtitles,
transcripts or decompiler dumps. Keep local research and captures in ignored
`private/`. Do not upload these materials or personal saves to public issues.

The six curated README media files listed in `tools/check_source.py` are the
explicit exception for gameplay publicity. Their content hashes are checked;
this does not allow arbitrary captures or original asset files. See
[recording details](docs/media/gameplay/README.md). Footage is excluded from MIT.
The separately curated preview-showcase release attachment is documented there
with its exact hash; it does not expand the source-tree media allowlist.

The code reads data supplied locally by the player. A contribution must not
require an original executable or silently download game data.

## Changes and verification

Keep each change focused and explain the observed problem, resulting behavior,
test evidence and remaining limits. Preserve unrelated local work.

Build and run unit tests with the locked dependencies:

```powershell
cargo test --locked
cargo build --release --locked
python tools/check_source.py --worktree
python -m unittest discover -s tools -p "test_source_package.py"
```

Game-data checks need a compatible local installation; see `docs/VALIDATION.md`.
Use isolated test saves. Verify watched, skipped, paused and restored outcomes
when editing scenes, and real player traversal when editing world machinery.
Do not weaken combat or force progression to make a route check pass.

Save format 12, existing event keys and reserved per-visit hit IDs are shared
contracts. Review migration and route consequences before changing them.

Before publishing, audit the exact staged files with `tools/check_source.py`
and review the intended history. The working-tree mode includes new files but
does not stage or commit anything. A local package is a review snapshot, not a
claim that its uncommitted work is a frozen release.

## Bug reports

Include the build/version, Windows version, map and visit, reproducible steps,
expected and observed behavior, and whether the problem occurs on a fresh visit
or after loading. Describe the difficulty and any changed settings. Keep public
attachments limited to material you are permitted to share; remove personal
paths and do not include original scripts, game packs or saves.
