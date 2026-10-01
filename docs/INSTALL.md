# Install and run Looking Glass on Windows

Looking Glass is an unfinished, independently written Rust recreation using your own American McGee's Alice game data. All 39 visits have completed automated campaign routes, but original-game fidelity and broader device validation remain unfinished. The current launch opens the main menu; see [the campaign audit](CAMPAIGN_RUN_AUDIT.md) for exact evidence.

## Install the Windows preview

[Download the Windows x64 preview ZIP](https://github.com/skulitom/LookingGlass/releases/download/v0.32.0-preview.1/LookingGlass-Windows-x64-v0.32.0-preview.1.zip).
Choose this Windows package,
not GitHub's automatically generated **Source code** ZIP.

1. Right-click the Windows ZIP and choose **Extract All**. Keep the entire folder
   somewhere writable, such as `Documents\LookingGlass`. Do not run it inside the ZIP.
2. Open the extracted folder and double-click **Launch.cmd**.
3. On first launch, select **Set up** in the guided window. It looks for game
   files beside the game, in Downloads, and at a previously saved location.
   Choose **Download compatible game files** if you want setup to fetch them.
4. Setup checks and unpacks the files, then opens the game. Choose **New Game**.
   Next time, double-click **Launch.cmd** or the optional desktop shortcut.

You do not need to type commands, install Rust or Python, install 7-Zip separately,
or run as administrator. The Windows ZIP includes the small extractor it needs.
Setup uses the Windows PowerShell and .NET components already supplied with Windows.
It changes no machine-wide execution-policy setting.

Allow **3 GB free space** for the game, its download and saved progress. The
optional download is about **933 MB**, so its duration depends on your connection.
You can cancel from the setup window. If a connection fails, retry or use an
existing file; partial downloads are discarded rather than treated as complete.

## Where to get the game files

The supported data is the **English 2011 vanilla** package. Setup's Download option
fetches **Alice1_2011_vanilla.7z** from
[American McGee's Alice (2011) on Internet Archive](https://archive.org/details/alice_202106).
The [file listing](https://archive.org/download/alice_202106) and its archive
metadata were checked on **1 October 2026**. Use files you are entitled to use.
Original game data is downloaded separately and is never bundled in the engine ZIP.

For a manual download, select that exact `.7z` file under **Show all**. You do not
need the torrent, pictures or metadata files. In setup, use **Choose archive** and
select the downloaded file. You can also drag it onto **Setup.cmd**.

Already have extracted files? Select **Choose folder**, then choose any of:

- `Alice1`, containing `bin\base`;
- its parent folder, containing `Alice1\bin\base`;
- `bin\base` itself, containing the six PK3 files.

Your existing folder stays in place. Keep it there after setup. Compatibility with
2000 retail data, other languages and third-party texture mods is not established.
Looking Glass runs independently; you do not need to edit Steam configuration,
change Alice: Madness Returns settings, or run the original executable.

## Changing files, updates and saves

Open **Setup.cmd** to choose a different archive or folder. Setup validates the
six required packs and the 36-map layout before changing the remembered path.
Downloaded files must also match the expected size and SHA-256 checksum. Local
folders are checked for archive structure and map format, not every asset's bytes.

For an archive, setup imports only the six required PK3 files to
`private/game-data`. It does not extract or run the original executables. Imports
are isolated, and a validated import can be reused. An interrupted extraction may
leave an unused import folder; the previously configured game and saves are kept.

All local settings, imported files, downloads and saves are under `private/`.
A selected external data folder is read in place. Moving the whole Looking Glass
folder keeps imported data working; desktop shortcuts must be recreated after a
move. If an external folder moves, open Setup and choose its new location.

For updates, extract the new release into a **new folder** and keep the old one.
Follow that release's save-compatibility notes before copying your `private/`
folder. Setup never resets your saves. Original-game saves are not compatible.
Removing the Looking Glass folder removes its local saves/imports, so keep a backup;
an external data folder is unaffected.

## Troubleshooting

| Problem | What to do |
| --- | --- |
| Nothing starts from the ZIP | Choose **Extract All** first, then open **Launch.cmd** in the extracted folder. |
| No game files found | Use **Download compatible game files**, **Choose archive**, or **Choose folder**. |
| Download fails or is unavailable | Retry, or download the named archive yourself and choose it in setup. No previous game setting is replaced. |
| Integrity check fails | The download is incomplete or different. Obtain the compatible archive again, or select an existing compatible folder. |
| Missing pack / unsupported map format | Select the English 2011 vanilla archive or its Alice1 folder. |
| Unpacking fails | Check free space and that the download is complete. Retry setup. |
| Access denied | Move the whole folder outside Program Files to a writable location such as Documents. |
| Extractor missing | Extract the complete Windows ZIP again. Source users can install [7-Zip](https://www.7-zip.org/) or select already extracted files. |
| Source folder needs Rust | Use the Windows binary package, or follow the developer steps below. |
| A save fails after changing game data | Return to the previous data set. Saves identify the packs used to create them. |

This is an **unsigned preview**. Windows may identify it as an unknown publisher.
Check the release source and checksum; do not disable antivirus or system security.
On a managed computer, ask its administrator if policy blocks PowerShell.
Report launch errors with the exact message, Windows version and release name.
Do not post original game files, private paths or personal saves publicly.

## Build from source

Install a Windows Rust toolchain and the required Microsoft C++ build tools using the [official Rust installation instructions](https://www.rust-lang.org/tools/install). The project's minimum Rust version is declared in Cargo.toml (currently 1.86).

From the project folder:

```powershell
cargo build --release --locked
.\Setup.cmd
.\Launch.cmd
```

The compiler downloads the locked open-source dependencies. It does not fetch Alice game files. Advanced data selection is also available without setup:

```powershell
.\target\release\looking-glass.exe --data "D:\Games\Alice1\bin\base" --map gvillage
```
