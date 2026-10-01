# Windows level-switch crash

The Windows executable used the linker's 1 MiB main-thread stack default.
Selecting Skool's Out through Tab reproduced a stack overflow (Windows exit
code `0xC00000FD`) while loading the destination, before replacing the current
scene. The same loading path serves every chapter and console map change.

`build.rs` reserves 8 MiB for the Windows executable's main thread, for both
MSVC and GNU builds. Windows commits stack pages as needed. This applies to
ordinary Cargo builds and shipped executables; no launcher environment setting
is required. `RUST_MIN_STACK` would not fix the window's main thread.

Regression verification must include the running viewer: start Skool Daze,
open Tab, select Skool's Out and begin, then switch to other maps and back.
Direct map startup and the standalone `--level-swap-check` run under different
call stacks and cannot alone prove this fix. Inspect the final PE header to
confirm its stack reserve is 8 MiB.

Verified on Windows in the rebuilt release: 40 consecutive Tab selections in
one running viewer, covering all 39 chapter visits, including the three return
visits and two entries into Skool's Out. The viewer then closed with exit code
zero. The final PE header reports an 8 MiB reserve. Evidence and exact build
inputs are in `private/map-switch-fix/` (local, ignored). No save format or
gameplay source changes were needed.
