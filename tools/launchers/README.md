# Development launchers

These shortcuts were moved out of the repository root during release preparation.
They resolve `Launch.cmd` and private fixtures relative to the repository root,
so they can be double-clicked or called from another working directory.

For ordinary play, use the root **Launch.cmd** and its main menu/chapter chooser.
These developer shortcuts are not required in a Windows player package. Some
older enemy previews select locally built executables under `private/`; those
fixtures are deliberately absent from the source review archive.

Existing per-map save directories are retained. Moving the shortcuts does not
move, delete or migrate saves. Use a fresh isolated save directory when testing
new visit behavior.
