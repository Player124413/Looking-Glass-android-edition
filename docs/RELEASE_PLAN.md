# First public preview

**v0.32.0-preview.1**, 1 October 2026, is the first experimental Windows x64 preview.
[Repository](https://github.com/skulitom/LookingGlass) Â· [Release and download](https://github.com/skulitom/LookingGlass/releases/tag/v0.32.0-preview.1)

## Publication boundary

The public repository begins with a reviewed snapshot of current source. The older
local development history, private research, saves, game packs and raw captures
remain private. The six hash-checked README media files are the requested publicity
exception; the underlying game artwork is excluded from the MIT source license.

The Windows ZIP includes the new engine, launch/setup helpers, player instructions,
component notices, an extractor and required third-party source packages. It does
not contain the original game executable, DLLs or game packs. Setup fetches game
data separately only when the player selects Download.

## Build identity

All 479 engine build inputs (Rust source, compiled route JSON, Cargo manifests and
build.rs) match the previously tested static-runtime candidate, allowing only Git
line-ending normalization (152 files change from CRLF to LF). No code or data
content changes were made to the engine.
The public commit adds the current installer, player documentation, source audit
and curated gameplay previews. BUILD_INFO.json identifies both the original build
snapshot and the matching public source commit; it does not claim a rebuild from
that commit. MANIFEST.json hashes every other packaged file. The release also
provides a ZIP checksum and an engine-input manifest.

The executable uses x86_64-pc-windows-msvc and a statically linked C runtime. It is
unsigned. The release notes identify the compiler and verification evidence.

## Verification and remaining limits

- 665 Rust unit tests passed, one ignored, on the matching engine source.
- Installer tests cover synthetic inputs, invalid hashes, cancellation and retained
  configuration. Package tests check the source boundary and archive contents.
- The exact candidate passed campaign graph, registry and items checks. Its campaign
  legs run exceeded the earlier 180-second harness limit; that run is incomplete.
- Guided first launch, actual compatible-data download/import and second launch
  were checked on the development machine. A clean Windows account and audible
  playback have not been verified.
- Five earlier automated campaign lineages completed all 39 visits, with one fresh
  uninterrupted run. See [the campaign audit](CAMPAIGN_RUN_AUDIT.md). This does not
  certify every cinematic, sound, original behavior or hardware configuration.
- Windows CI runs source-boundary, synthetic setup/package and Rust unit checks.
  It does not download original game data or publish releases automatically.

[Known issues](KNOWN_ISSUES.md) accompany this preview. At the owner's request,
the Windows preview is listed as GitHub's **Latest** release so it appears in the
repository's Releases sidebar and at `/releases/latest`. GitHub requires clearing
its prerelease flag for this placement. The tag, title and player documentation
continue to identify it as an **experimental preview**; broader player/device and
fidelity validation are still needed before describing it as stable. This listing
change does not replace the tested Windows package or change its checksum.
Steam publicity is prepared separately as a draft for the owner to review.

## Future release checks

Freeze the intended source, preserve versioned save migrations, verify the build
against its commit, run relevant gameplay checks and repeat ZIP-only installation
on a clean account. Review dependency notices and exact public files. Record any
uncompleted checks honestly in release notes. Never upload private test evidence,
original assets or old development history as an incidental release artifact.
