"""Assemble a local Windows candidate from explicit reviewed inputs; never publish."""
import argparse
import hashlib
import json
from pathlib import Path
import zipfile

ROOT = Path(__file__).resolve().parents[1]
PLAYER_FILES = ('Launch.cmd', 'Setup.cmd', 'tools/windows_setup.ps1', 'LICENSE-MIT', 'LEGAL.md',
                'docs/INSTALL.md', 'docs/KNOWN_ISSUES.md', 'docs/CONTROLS.md',
                'docs/CAMPAIGN_RUN_AUDIT.md', 'docs/SAVES.md')
QUICK_START = r"""LOOKING GLASS - WINDOWS PREVIEW

1. Extract the whole ZIP (right-click > Extract All).
2. Open Launch.cmd in the extracted folder.
3. On first launch, choose Set up. Existing game files are detected, or choose
   Download compatible game files. Setup checks and unpacks them, then plays.

Next time: just open Launch.cmd, or your optional desktop shortcut.
To change game files: open Setup.cmd.

No commands, Rust, Python, separate 7-Zip installation or administrator rights
are needed. Use a writable folder such as Documents\LookingGlass. Allow 3 GB
of free space; the optional game-data download is about 933 MB.

GAME FILES
Use files you are entitled to use. The engine ZIP includes no original game data.
Setup can fetch Alice1_2011_vanilla.7z from:
https://archive.org/details/alice_202106
Or use Choose archive / Choose folder for existing English 2011 vanilla files.
That download is hosted by a third party, not this project. Its availability
and setup's checks do not establish permission to download or use it.

SAVES AND UPDATES
Saves, settings and imported files are in private/. Keep a backup. Install a new
preview in a separate folder and follow its save compatibility notes before
copying private/. Do not delete your old installation until the new one works.
Moving the folder requires recreating a desktop shortcut, if you use one.

This is an unsigned, experimental preview. See docs/KNOWN_ISSUES.md and
docs/INSTALL.md for limitations and troubleshooting. Check the release source
and checksum if Windows flags an unknown publisher. Do not disable security.

Esc: menu. F5/F9: quick save/load. Controls can be changed in Settings.
Use normal exits for campaign progress; Tab's chapter chooser starts a new visit.

This project is not endorsed by or affiliated with EA or its licensors.
American McGee's Alice is an Electronic Arts property. Original game assets
and trademarks remain with EA and/or their respective rights holders.
Looking Glass claims no ownership of them and grants no licence to them.
See LEGAL.md, LICENSE-MIT and THIRD_PARTY_NOTICES.txt for rights and notices.
"""


def digest(data):
    return hashlib.sha256(data).hexdigest()


def package(exe, extractor, notices, source_manifest, output, build_info):
    if output.exists() or output.with_suffix('.sha256').exists():
        raise ValueError('Refusing to overwrite an existing candidate.')
    source = json.loads(source_manifest.read_text(encoding='utf-8'))
    info = json.loads(build_info.read_text(encoding='utf-8'))
    exe_data = exe.read_bytes()
    if info['executable_sha256'] != digest(exe_data):
        raise ValueError('Executable differs from the recorded build.')
    if info['engine_source_zip_sha256'] != source['sha256']:
        raise ValueError('Source record differs from the recorded build.')
    payload = {name: (ROOT / name).read_bytes() for name in PLAYER_FILES}
    payload['looking-glass.exe'] = exe_data
    payload['setup/7zr.exe'] = extractor.read_bytes()
    if info['extractor_sha256'] != digest(payload['setup/7zr.exe']):
        raise ValueError('Extractor differs from the reviewed helper.')
    payload['README.txt'] = QUICK_START.replace('\n', '\r\n').encode('utf-8')
    payload['BUILD_INFO.json'] = (json.dumps(info, indent=2) + '\n').encode('utf-8')
    payload['THIRD_PARTY_NOTICES.txt'] = (notices / 'THIRD_PARTY_NOTICES.txt').read_bytes()
    for path in sorted((notices / 'licenses').rglob('*')):
        if path.is_dir():
            continue
        if path.is_symlink() or not path.resolve().is_relative_to(notices.resolve()):
            raise ValueError('Unexpected license path: ' + str(path))
        if path.suffix.lower() in {'.exe', '.dll', '.pk3', '.7z', '.bsp', '.pdb'}:
            raise ValueError('Unexpected binary in notices: ' + str(path))
        payload[path.relative_to(notices).as_posix()] = path.read_bytes()
    payload['MANIFEST.json'] = (json.dumps({name: digest(data) for name, data in payload.items()}, indent=2) + '\n').encode()
    output.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(output, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for name, data in payload.items():
            archive.writestr('LookingGlass/' + name, data)
    with zipfile.ZipFile(output) as archive:
        assert archive.testzip() is None
        assert {name.removeprefix('LookingGlass/'): archive.read(name) for name in archive.namelist()} == payload
    checksum = digest(output.read_bytes())
    output.with_suffix('.sha256').write_text(checksum + '  ' + output.name + '\n', encoding='ascii')
    return {'file': str(output), 'sha256': checksum, 'files': len(payload), 'bytes': output.stat().st_size}


if __name__ == '__main__':
    p = argparse.ArgumentParser(description=__doc__)
    for arg in ('exe', 'extractor', 'notices', 'source-manifest', 'output', 'build-info'):
        p.add_argument('--' + arg, type=Path, required=True)
    args = p.parse_args()
    print(json.dumps(package(**vars(args)), indent=2))
