"""Read-only inspector for the user's local Alice data packs.

Usage:
  python tools/alice_data.py [--data <base>] <command> [args]

Data base resolution: the global --data option (placed before the command), then the
LOOKING_GLASS_DATA environment variable, then private/data-path.txt (repo root, then the
current directory). The base is never hard-coded.

Commands:
  ls [prefix]            list pack files (later packs override earlier)
  cat <path>             print a text file with line numbers
  scripts <map>          list the map script, its includes and its cinematics scripts
  summary <map>          classname/model counts, entrances, exits, spawns, triggers
  ents <map> [regex]     one line per entity (optionally filter classname/targetname/model by regex)
  grep <regex> [prefix]  search text files in packs (default prefix maps/)
  facts <map>            write private/level-facts/<map>.json (identifiers, counts, coordinates,
                         threads, cameras, exits and init-call names only; no script bodies)

Nothing here writes to the data, the pk3 archives or tracked files. Only `facts` writes, and only
under private/level-facts/. Output is for local analysis: never copy script bodies, dialogue or
subtitle text into repository files (LEGAL.md, docs/CAMPAIGN_PLAN.md section 3.2).
"""
import json
import os
import re
import struct
import sys
import zipfile
from collections import Counter
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
TEXT_EXT = (".scr", ".st", ".tik", ".txt", ".cfg", ".shader", ".urc", ".tlk", ".cam", ".def")

_base = None
_files = None


def resolve_base(cli_base=None):
    if cli_base:
        return cli_base
    env = os.environ.get("LOOKING_GLASS_DATA")
    if env:
        return env
    for candidate in (REPO / "private" / "data-path.txt", Path("private") / "data-path.txt"):
        if candidate.is_file():
            value = candidate.read_text(encoding="utf-8").strip()
            if value:
                return value
    raise SystemExit("no data base: pass --data <base>, set LOOKING_GLASS_DATA, or write private/data-path.txt")


def mount():
    global _files
    if _files is None:
        files = {}
        packs = sorted((p for p in os.listdir(_base) if p.lower().endswith(".pk3")), key=str.lower)
        for p in packs:
            z = zipfile.ZipFile(os.path.join(_base, p))
            for n in z.namelist():
                if not n.endswith("/"):
                    files[n.lower().replace("\\", "/")] = (z, n, p)
        _files = files
    return _files


def read(path):
    key = path.lower().replace("\\", "/")
    files = mount()
    if key not in files:
        raise SystemExit(f"not found: {path}")
    z, n, _ = files[key]
    return z.read(n)


def text(path):
    return read(path).decode("latin1")


def entities(mapname):
    b = read(f"maps/{mapname}.bsp")
    assert b[:4] == b"FAKK", "not a FAKK bsp"
    off, length = struct.unpack_from("<ii", b, 12 + 14 * 8)
    raw = b[off:off + length].decode("latin1").rstrip("\0")
    ents = []
    for block in re.findall(r"\{(.*?)\}", raw, re.S):
        ents.append(dict(re.findall(r'"([^"]*)"\s+"([^"]*)"', block)))
    return ents


def resolve_include(path):
    """Pack path of an #include target: as written, then under maps/ or global/."""
    key = path.lower().replace("\\", "/").replace("../", "")
    files = mount()
    for candidate in (key, "maps/" + key, "global/" + key):
        if candidate in files:
            return candidate
    return None


def includes(path, seen=None):
    seen = seen if seen is not None else []
    key = resolve_include(path)
    if key is None or key in seen:
        return seen
    seen.append(key)
    for inc in re.findall(r'#include\s+"([^"]+)"', text(key), re.I):
        includes(inc, seen)
    return seen


def cmd_ls(prefix=""):
    files = mount()
    for k in sorted(files):
        if k.startswith(prefix.lower()):
            z, n, p = files[k]
            print(f"{k}\t{p}\t{z.getinfo(n).file_size}")


def cmd_cat(path):
    for i, line in enumerate(text(path).splitlines(), 1):
        print(f"{i:5}: {line}")


def script_files(mapname):
    files = mount()
    found = includes(f"maps/{mapname}.scr")
    for k in sorted(files):
        if k.startswith("maps/cinematics/") and k.split("/")[-1].startswith(mapname) and k not in found:
            found += includes(k)
    return found


def cmd_scripts(mapname):
    files = mount()
    for k in script_files(mapname):
        z, n, p = files[k]
        print(f"{k}\t{p}\t{z.getinfo(n).file_size} bytes")


def fmt(i, e):
    keys = ["classname", "targetname", "target", "model", "origin", "angle", "angles", "spawnflags", "map", "thread", "setthread", "killtarget", "anim", "name"]
    head = " ".join(f'{k}="{e[k]}"' for k in keys if k in e)
    rest = " ".join(f'{k}="{v}"' for k, v in e.items() if k not in keys)
    return f"#{i} {head} {rest}".rstrip()


def cmd_ents(mapname, pattern=None):
    rx = re.compile(pattern, re.I) if pattern else None
    for i, e in enumerate(entities(mapname)):
        hay = " ".join([e.get("classname", ""), e.get("targetname", ""), e.get("model", ""), e.get("target", "")])
        if rx is None or rx.search(hay):
            print(fmt(i, e))


def cmd_summary(mapname):
    ents = entities(mapname)
    cls = Counter(e.get("classname", "?") for e in ents)
    models = Counter(e["model"] for e in ents if "model" in e and not e["model"].startswith("*"))
    print(f"== {mapname}: {len(ents)} entities")
    print("classes:", ", ".join(f"{k}x{v}" for k, v in cls.most_common()))
    print("models:", ", ".join(f"{k}x{v}" for k, v in models.most_common()))
    interesting = re.compile(r"changelevel|player_start|start|teleport|func_spawn|trigger_(once|multiple|fall|catmessage|push|hurt|accelerate)|func_camera|boss|Enemies_|Characters_|Item_|func_(door|rotatingdoor|fulcrum|smashablewall|sinkobject|earthquake|fallingrock|spawnchain)", re.I)
    print("-- gameplay-relevant entities --")
    for i, e in enumerate(ents):
        c = e.get("classname", "")
        if interesting.search(c) or "map" in e or "thread" in e or "setthread" in e:
            print(fmt(i, e))
    print("-- scripts --")
    cmd_scripts(mapname)


def cmd_grep(pattern, prefix="maps/"):
    rx = re.compile(pattern, re.I)
    files = mount()
    for k in sorted(files):
        if k.startswith(prefix.lower()) and k.endswith(TEXT_EXT):
            for i, line in enumerate(text(k).splitlines(), 1):
                if rx.search(line):
                    print(f"{k}:{i}: {line.strip()[:220]}")


# --- facts: identifiers, counts and coordinates only -------------------------------------------

IDENT = re.compile(r"^[A-Za-z0-9_$.\-/*]{1,80}$")
COORD = re.compile(r"^-?\d+(\.\d+)?( -?\d+(\.\d+)?){0,2}$")
FUNC_DEF = re.compile(r"^\s*(?:void|float|string|vector|entity|int)\s+([A-Za-z_]\w*)\s*\(([^)]*)\)\s*$")
CAM_LOAD = re.compile(r'\bcam\.load\s*\(\s*"([^"]+)"\s*\)')
CALL = re.compile(r"\b([A-Za-z_]\w*)\s*\(")
THREAD_REF = re.compile(r"\bthread\s+([A-Za-z_][\w.]*)")
CALL_SKIP = {"if", "while", "for", "switch", "wait", "waitfor", "print", "cam", "sound", "local", "goto"}


class Sanitizer:
    """Keeps only identifier-shaped strings and coordinate triples; counts what it drops."""

    def __init__(self):
        self.dropped = 0

    def ident(self, value):
        if isinstance(value, str) and IDENT.match(value):
            return value
        self.dropped += 1
        return None

    def coord(self, value):
        if isinstance(value, str) and COORD.match(value.strip()):
            return value.strip()
        self.dropped += 1
        return None


def function_names(src):
    """(name, parameter count) of every function defined (not merely declared) in a script."""
    lines = src.splitlines()
    found = []
    for i, line in enumerate(lines):
        m = FUNC_DEF.match(line)
        if not m:
            continue
        j = i + 1
        while j < len(lines) and not lines[j].strip():
            j += 1
        if j < len(lines) and lines[j].strip().startswith("{"):
            params = m.group(2).strip()
            found.append((m.group(1), 0 if not params else params.count(",") + 1))
    return found


def function_body(src, name):
    m = re.search(rf"^\s*(?:void|float|string|vector|entity|int)\s+{re.escape(name)}\s*\([^)]*\)\s*\{{", src, re.M | re.I)
    if not m:
        m = re.search(rf"^\s*(?:void|float|string|vector|entity|int)\s+{re.escape(name)}\s*\([^)]*\)\s*$", src, re.M | re.I)
        if not m:
            return ""
        start = src.find("{", m.end())
        if start < 0:
            return ""
    else:
        start = m.end() - 1
    depth = 0
    for i in range(start, len(src)):
        if src[i] == "{":
            depth += 1
        elif src[i] == "}":
            depth -= 1
            if depth == 0:
                return src[start:i + 1]
    return src[start:]


def strip_comments(src):
    src = re.sub(r"/\*.*?\*/", "", src, flags=re.S)
    return re.sub(r"//[^\n]*", "", src)


def build_facts(mapname):
    san = Sanitizer()
    ents = entities(mapname)
    classes = Counter(e.get("classname", "?") for e in ents)
    facts = {
        "map": san.ident(mapname),
        "note": "identifiers, counts, coordinates and names only; no script bodies or dialogue",
        "entity_count": len(ents),
        "classes": {san.ident(k) or "?": v for k, v in sorted(classes.items())},
        "entrances": [], "exits": [], "cameras": [], "thread_keys": [],
        "scripts": [], "functions": {}, "main_calls": [], "cam_loads": [], "cinematics": [],
    }
    for i, e in enumerate(ents):
        cls = e.get("classname", "")
        if cls == "info_player_start":
            facts["entrances"].append({"index": i, "targetname": san.ident(e.get("targetname", "")),
                                       "origin": san.coord(e.get("origin", "")), "angle": san.coord(e.get("angle", ""))})
        elif cls == "trigger_changelevel":
            facts["exits"].append({"index": i, "map": san.ident(e.get("map", "")), "targetname": san.ident(e.get("targetname", "")),
                                   "origin": san.coord(e.get("origin", "")), "spawnflags": san.coord(e.get("spawnflags", "0"))})
        elif cls == "func_camera":
            facts["cameras"].append({"index": i, "targetname": san.ident(e.get("targetname", "")), "origin": san.coord(e.get("origin", ""))})
        for key in ("thread", "setthread"):
            if e.get(key):
                facts["thread_keys"].append({"index": i, "classname": san.ident(cls), "key": key, "thread": san.ident(e[key])})
    cam_names = set()
    main_calls = set()
    for path in script_files(mapname):
        src = strip_comments(text(path))
        funcs = function_names(src)
        facts["scripts"].append({"file": san.ident(path), "functions": len(funcs)})
        facts["functions"][path] = [{"name": san.ident(n), "params": p} for n, p in funcs]
        cam_names.update(CAM_LOAD.findall(src))
        if path == f"maps/{mapname}.scr":
            body = function_body(src, "main")
            for c in CALL.findall(body):
                if c.lower() not in CALL_SKIP:
                    main_calls.add(c)
            main_calls.update(THREAD_REF.findall(body))
        if path.startswith("maps/cinematics/"):
            facts["cinematics"].append({"file": san.ident(path), "functions": [san.ident(n) for n, _ in funcs]})
    facts["cam_loads"] = sorted(n for n in (san.ident(c) for c in cam_names) if n)
    facts["main_calls"] = sorted(n for n in (san.ident(c) for c in main_calls) if n)
    facts["dropped_non_identifier_values"] = san.dropped
    return facts


def cmd_facts(mapname):
    facts = build_facts(mapname)
    out_dir = REPO / "private" / "level-facts"
    out_dir.mkdir(parents=True, exist_ok=True)
    out = out_dir / f"{mapname}.json"
    out.write_text(json.dumps(facts, indent=2), encoding="utf-8")
    print(f"wrote {out} ({len(facts['exits'])} exits, {len(facts['entrances'])} entrances, "
          f"{sum(len(v) for v in facts['functions'].values())} functions, {len(facts['cam_loads'])} camera loads, "
          f"{facts['dropped_non_identifier_values']} values dropped)")


COMMANDS = {"ls": cmd_ls, "cat": cmd_cat, "scripts": cmd_scripts, "summary": cmd_summary,
            "ents": cmd_ents, "grep": cmd_grep, "facts": cmd_facts}


def main(argv):
    global _base
    args = list(argv)
    cli_base = None
    if args and args[0] == "--data":
        if len(args) < 3:
            print(__doc__)
            return 1
        cli_base, args = args[1], args[2:]
    if not args or args[0] not in COMMANDS:
        print(__doc__)
        return 1
    _base = resolve_base(cli_base)
    COMMANDS[args[0]](*args[1:])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
