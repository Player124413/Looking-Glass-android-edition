"""Read-only map/sky/LOD evidence. Original excerpts are written only to private/."""
from pathlib import Path
import zipfile, re, struct, json

root = Path(__file__).resolve().parents[1]
saved = root / 'private/data-path.txt'
base = Path(saved.read_text().strip()) if saved.exists() else root / 'alice_202106/Alice1/bin/base'
packs = [zipfile.ZipFile(p) for p in sorted(base.glob('*.pk3'))]
entries = {n.lower(): (p, n) for p in packs for n in p.namelist()}
out = root / 'private/sky-performance'
out.mkdir(parents=True, exist_ok=True)
report = []
for name, (pack, original) in entries.items():
    if not name.endswith('.scr'):
        continue
    text = pack.read(original).decode('utf-8', errors='replace')
    lines = text.splitlines()
    hits = [(i+1, l.strip()) for i, l in enumerate(lines) if re.search(r'(?i)(skyorigin|setfarplane|fog|sky_.*(?:rotate|move|follow)|\$sky)', l.split('//')[0])]
    if hits:
        report.append({'file':name,'lines':hits})
        (out/name.replace('/','__')).write_text(text, encoding='utf-8')
for h in report:
    print(h['file'], '\n  '+'\n  '.join(f'{n}: {line}' for n,line in h['lines']))
(out/'script-references.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
for name in ['skool1','potears1','hedge1','hatter1']:
    pack, key = entries[f'maps/{name}.bsp']; b=pack.read(key)
    lumps=[b[o:o+n] for o,n in [struct.unpack_from('<2i',b,12+i*8) for i in range(20)]]
    (out/f'{name}-entities.txt').write_text(lumps[14].decode('utf8',errors='replace'),encoding='utf-8')
    print(name, 'lumps', [(i,len(l)) for i,l in enumerate(lumps)])
    for i in [6,7,8,9,15,16,17,18,19]:
        print(i, struct.unpack_from('<'+'i'*min(12,len(lumps[i])//4),lumps[i]))
