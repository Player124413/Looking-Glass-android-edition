"""Read-only original effect evidence. Extracted declarations stay private."""
from pathlib import Path
import zipfile, re, json, struct

root = Path(__file__).resolve().parents[1]
saved = root / 'private/data-path.txt'
base = Path(saved.read_text().strip()) if saved.exists() else root / 'alice_202106/Alice1/bin/base'
packs = [zipfile.ZipFile(p) for p in sorted(base.glob('*.pk3'))]
entries = {n.lower(): (p, n) for p in packs for n in p.namelist()}
out = root / 'private/render-fx'
out.mkdir(parents=True, exist_ok=True)
hits = []
for name, (pack, original) in entries.items():
    if not name.endswith(('.tik', '.shader')):
        continue
    txt = pack.read(original).decode('utf-8', errors='replace')
    relevant = [l.strip() for l in txt.splitlines() if re.search(r'(?i)\b(dlight|light|lensflare|flare|shadow|tagemitter|originemitter|emitteron|emitteroff|tagspawn|originspawn)\b', l.split('//')[0])]
    if relevant:
        hits.append({'file': name, 'commands': relevant})
        (out / name.replace('/', '__')).write_text(txt, encoding='utf-8')
(out/'declarations.json').write_text(json.dumps(hits, indent=2), encoding='utf-8')
for h in hits:
    if any(re.search(r'(?i)\b(dlight|light|lensflare|flare|shadow)\b', s) for s in h['commands']):
        print(h['file'], '\n  ' + '\n  '.join(h['commands'][:10]))
print('Effect files:', len(hits))
