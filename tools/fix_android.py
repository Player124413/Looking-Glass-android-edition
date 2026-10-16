# tools/fix_android.py
# Uso: python tools/fix_android.py   (na raiz do projeto)
import re
from pathlib import Path
from PIL import Image

ROOT = Path(".")
SRC_DIR = ROOT / "src"

# ---------- 1. Corrige placeholders inline nos macros do miniquad ----------
MACRO = re.compile(r'(miniquad::(?:info|warn|error|debug|trace)!\(\s*)"((?:[^"\\]|\\.)*)"')
NAMED = re.compile(r'\{([A-Za-z_][A-Za-z0-9_]*)(:[^}]*)?\}')

def fix_call(m):
    prefix, text = m.group(1), m.group(2)
    names = NAMED.findall(text)
    if not names:
        return m.group(0)
    # Não mexe se já houver placeholders posicionais misturados ({} ou {:?})
    if re.search(r'\{(:[^}]*)?\}', NAMED.sub("", text)):
        return m.group(0)
    new_text = NAMED.sub(lambda n: "{" + (n.group(2) or "") + "}", text)
    args = ", ".join(n for n, _ in names)
    return f'{prefix}"{new_text}", {args}'

changed = 0
for rs in SRC_DIR.rglob("*.rs"):
    code = rs.read_text(encoding="utf-8")
    new = MACRO.sub(fix_call, code)
    if new != code:
        rs.write_text(new, encoding="utf-8")
        changed += 1
        print(f"[logs] corrigido: {rs}")
print(f"[logs] {changed} arquivo(s) alterado(s)")

# ---------- 2. Gera ícones a partir de src/icon.png (512x512) ----------
ICON = SRC_DIR / "icon.png"
OUT = ROOT / "android-res"
SIZES = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}

img = Image.open(ICON).convert("RGBA")
for dpi, px in SIZES.items():
    d = OUT / f"mipmap-{dpi}"
    d.mkdir(parents=True, exist_ok=True)
    icon = img.resize((px, px), Image.LANCZOS)
    icon.save(d / "ic_launcher.png", optimize=True)
    icon.save(d / "ic_launcher_round.png", optimize=True)
    print(f"[icon] {d}/ic_launcher.png ({px}px)")

# ---------- 3. Adiciona android:icon no manifest (se encontrado) ----------
ATTR = 'android:icon="@mipmap/ic_launcher" android:roundIcon="@mipmap/ic_launcher_round"'
for f in list(ROOT.glob("**/AndroidManifest.xml")) + list((ROOT / "tools").glob("*.py")):
    if "target" in f.parts or f.name == "fix_android.py":
        continue
    txt = f.read_text(encoding="utf-8")
    if "<application" in txt and "android:icon" not in txt:
        txt = txt.replace("<application", f"<application {ATTR}", 1)
        f.write_text(txt, encoding="utf-8")
        print(f"[manifest] ícone adicionado em: {f}")

print("Pronto. Recompile o APK.")
