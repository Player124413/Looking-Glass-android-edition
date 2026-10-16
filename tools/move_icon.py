# tools/move_icon.py
# Uso: python tools/move_icon.py   (na raiz do projeto)
import shutil
from pathlib import Path
from PIL import Image

ROOT = Path(".")
OLD = ROOT / "src" / "icon.png"
NEW = ROOT / "private" / "icon.png"
OUT = ROOT / "target" / "android-res"
SIZES = {"mdpi": 48, "hdpi": 72, "xhdpi": 96, "xxhdpi": 144, "xxxhdpi": 192}

# 1. Move o ícone para fora do código-fonte
NEW.parent.mkdir(parents=True, exist_ok=True)
if OLD.exists():
    shutil.move(str(OLD), str(NEW))
    print(f"[icon] movido: {OLD} -> {NEW}")

# 2. Remove pastas que a auditoria rejeita
for p in [ROOT / "android-res", ROOT / "src" / "icon.ico"]:
    if p.is_dir():
        shutil.rmtree(p)
    elif p.exists():
        p.unlink()
    print(f"[limpeza] removido: {p}")

# 3. Gera os ícones em target/android-res
img = Image.open(NEW).convert("RGBA")
for dpi, px in SIZES.items():
    d = OUT / f"mipmap-{dpi}"
    d.mkdir(parents=True, exist_ok=True)
    icon = img.resize((px, px), Image.LANCZOS)
    icon.save(d / "ic_launcher.png", optimize=True)
    icon.save(d / "ic_launcher_round.png", optimize=True)
    print(f"[icon] {d / 'ic_launcher.png'} ({px}px)")
print("Pronto.")
