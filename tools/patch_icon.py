# tools/patch_icon.py
# Uso: python tools/patch_icon.py   (na raiz do projeto)
from pathlib import Path

PKG = Path("tools/package_android.py")
ANCHOR = 'res_main_path.write_text(RES_MAIN_XML, encoding="utf-8", newline="\\n")'
MARK = "# [icon-patch]"

code = PKG.read_text(encoding="utf-8")
if MARK in code:
    print("Já aplicado.")
    raise SystemExit(0)

lines = code.splitlines(keepends=True)
for i, line in enumerate(lines):
    if ANCHOR in line:
        ind = line[: len(line) - len(line.lstrip())]
        block = [
            f"{MARK} copia/gera ícones mipmap-* para res/\n",
            "import shutil as _sh\n",
            "_icon_src = ROOT / 'target' / 'android-res'\n",
            "_icon_png = ROOT / 'private' / 'icon.png'\n",
            "if not _icon_src.exists() and _icon_png.exists():\n",
            "    from PIL import Image as _Img\n",
            "    _im = _Img.open(_icon_png).convert('RGBA')\n",
            "    for _dpi, _px in {'mdpi': 48, 'hdpi': 72, 'xhdpi': 96, 'xxhdpi': 144, 'xxxhdpi': 192}.items():\n",
            "        _d = _icon_src / f'mipmap-{_dpi}'\n",
            "        _d.mkdir(parents=True, exist_ok=True)\n",
            "        _r = _im.resize((_px, _px), _Img.LANCZOS)\n",
            "        _r.save(_d / 'ic_launcher.png')\n",
            "        _r.save(_d / 'ic_launcher_round.png')\n",
            "if not _icon_src.exists():\n",
            "    raise SystemExit('Ícones não encontrados: rode tools/move_icon.py')\n",
            "for _m in _icon_src.glob('mipmap-*'):\n",
            "    _sh.copytree(_m, res_layout_dir.parent / _m.name, dirs_exist_ok=True)\n",
        ]
        lines[i + 1:i + 1] = [ind + b for b in block]
        PKG.write_text("".join(lines), encoding="utf-8")
        print("Patch aplicado em", PKG)
        break
else:
    raise SystemExit("Âncora não encontrada (linha do RES_MAIN_XML).")
