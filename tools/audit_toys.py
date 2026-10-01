"""Read-only evidence for the ten-toy audit. Original text stays under private/.

Run from the repository root: python tools/audit_toys.py [--data PATH]
This reads definitions and animation headers, never executes game scripts/binaries.
"""
import argparse
import hashlib
import json
import re
import struct
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOYS = "knife cards mallet jackbomb icewand jacks demondice eyestaff blunderbuss watch".split()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    saved = ROOT / "private/data-path.txt"
    parser.add_argument("--data", type=Path, default=Path(saved.read_text().strip()) if saved.exists() else ROOT / "alice_202106/Alice1/bin/base")
    args = parser.parse_args()
    output = ROOT / "private/toy-audit"
    output.mkdir(parents=True, exist_ok=True)
    packs = [zipfile.ZipFile(p) for p in sorted(args.data.glob("*.pk3"), key=lambda p: p.name.lower())]
    entries = {n.replace("\\", "/").lower(): (z, n) for z in packs for n in z.namelist() if not n.endswith("/")}
    evidence = {}

    def read(name):
        z, original = entries[name]
        assert z.getinfo(original).file_size <= 128 * 1024 * 1024
        data = z.read(original)
        evidence[name] = {"archive": Path(z.filename).name, "entry": original, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}
        return data

    def text(name):
        data = read(name).decode("utf-8", errors="replace").replace("\r\r\n", "\n").replace("\r\n", "\n")
        (output / name.replace("/", "__")).write_text(data, encoding="utf-8")
        return data

    def commands(data):
        # Keep line provenance and context; inactive // examples are excluded.
        depth, area, context = 0, "", ""
        out = []
        for number, raw in enumerate(data.splitlines(), 1):
            line = raw.split("//", 1)[0].strip()
            if not line:
                continue
            if depth == 0 and line in {"setup", "init", "animations"}:
                area = line
            if area == "init" and depth == 1 and line in {"server", "client"}:
                context = line
            if area == "init" and depth == 2 and context == "server" and line not in {"{", "}"}:
                out.append({"line": number, "command": line})
            depth += line.count("{") - line.count("}")
        return out

    models = {f"models/w_{toy}.tik": commands(text(f"models/w_{toy}.tik")) for toy in TOYS}
    # Follow active references (including caches), not similarly named prototypes.
    # Reachability is evidence of a dependency, not proof that a firing mode uses it.
    pending = list(models)
    followed = set()
    while pending:
        name = pending.pop()
        if name in followed:
            continue
        followed.add(name)
        data = text(name)
        models.setdefault(name, commands(data))
        active = "\n".join(line.split("//", 1)[0] for line in data.splitlines())
        for ref in re.findall(r"[\w/.-]+\.tik", active, re.I):
            ref = ref.lower()
            ref = ref if ref.startswith("models/") else "models/" + ref
            if ref in entries and ref not in followed:
                pending.append(ref)

    alice = text("models/alice.tik")
    text("global/alice_torso.st")
    text("ai/fx_icewall.st")
    animations = []
    for number, line in enumerate(alice.splitlines(), 1):
        match = re.match(r"\s*(\w+)\s+([\w/]+\.ska)\b", line)
        if not match:
            continue
        alias, filename = match.groups()
        if not alias.startswith(("knife_att", "cards_att", "mallet_att", "wand_att", "jbomb_att", "jacks_att", "dice_att", "staff_att", "buss_att", "deadtime_watch", "putawayweapon", "changeweapon")):
            continue
        data = read("models/alice/" + filename)
        assert data[:4] == b"SKAN" and struct.unpack_from("<i", data, 4)[0] == 3
        frames, bones, duration, frame_time = struct.unpack_from("<iiff", data, 76)
        assert frames > 0 and 0 < frame_time <= 1 and abs(duration - frames * frame_time) < 0.02
        animations.append({"alias": alias, "file": filename, "line": number, "frames": frames, "bones": bones, "frame_time": round(frame_time, 6), "duration": round(duration, 6)})

    prop_animations = []
    for name in ("models/weapons/blunderbuss/firing.tan",
                 "models/weapons/jackbomb/cranking.tan",
                 "models/weapons/jackbomb/open.tan",
                 "models/weapons/jackbomb/rotate.tan"):
        data = read(name)
        assert data[:4] == b"TAN " and struct.unpack_from("<i", data, 4)[0] == 2
        frames = struct.unpack_from("<i", data, 72)[0]
        duration = struct.unpack_from("<f", data, 84)[0]
        offset = struct.unpack_from("<i", data, 100)[0]
        assert frames > 0 and offset + frames * 68 <= len(data)
        interval = duration / frames
        assert 0 < interval <= 1
        assert all(abs(struct.unpack_from("<f", data, offset + i * 68 + 64)[0] - interval) < .001 for i in range(frames))
        prop_animations.append({"file": name, "frames": frames, "duration": round(duration, 6), "frame_time": round(interval, 6)})

    # Record the compared Rust snapshot, without copying code or assets into the report.
    implementation = {}
    for name in ("src/weapons.rs", "src/combat.rs", "src/dice.rs", "src/inventory.rs",
                 "src/hud.rs", "src/viewer.rs", "src/input.rs", "src/powerups.rs"):
        implementation[name] = hashlib.sha256((ROOT / name).read_bytes()).hexdigest()
    binary = args.data.parent / "fgamex86.dll"
    original_binary = {"file": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest()} if binary.exists() else None
    report = {"toys": TOYS, "definitions": models, "alice_animations": animations,
              "prop_animations": prop_animations, "evidence": evidence,
              "implementation_sha256": implementation, "original_binary": original_binary}
    (output / "evidence.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"Audited {len(TOYS)} toys, {len(models)} reachable definitions, {len(animations)} Alice attack/equip aliases; {len(evidence)} source hashes. Evidence: {output / 'evidence.json'}")
    for z in packs:
        z.close()


if __name__ == "__main__":
    main()
