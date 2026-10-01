"""Read-only local inventory. Output belongs in private/, never the source release."""
from pathlib import Path
import collections
import hashlib
import json
import struct
import zipfile

root = Path(__file__).resolve().parents[1]
game = root / "alice_202106/Alice1/bin"
result = {"format": 1, "binaries": [], "archives": []}
for p in sorted(list(game.glob("*.exe")) + list(game.glob("*.dll"))):
    data = p.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    result["binaries"].append({"name": p.name, "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
        "machine": hex(struct.unpack_from("<H", data, pe + 4)[0])})
for p in sorted((game / "base").rglob("*.pk3")):
    with zipfile.ZipFile(p) as z:
        names = z.namelist()
        result["archives"].append({"name": p.relative_to(game).as_posix(),
            "size": p.stat().st_size,
            "sha256": hashlib.file_digest(p.open("rb"), "sha256").hexdigest(),
            "entries": len(names),
            "extensions": dict(collections.Counter(Path(n).suffix for n in names)),
            "maps": [n for n in names if n.lower().endswith(".bsp")]})
out = root / "private/inventory.json"
out.parent.mkdir(exist_ok=True)
out.write_text(json.dumps(result, indent=2), encoding="utf-8")
print(out)
