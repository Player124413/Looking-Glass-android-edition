"""Fetch a pinned official portable release compatible with the installed JDK 21."""
from pathlib import Path
import hashlib
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / ".tools"
NAME = "ghidra_11.4.2_PUBLIC_20250826.zip"
URL = "https://github.com/NationalSecurityAgency/ghidra/releases/download/Ghidra_11.4.2_build/" + NAME
SHA256 = "795a02076af16257bd6f3f4736c4fc152ce9ff1f95df35cd47e2adc086e037a6"
DEST.mkdir(exist_ok=True)
archive = DEST / NAME
if not archive.exists():
    print("Downloading official portable Ghidra release", flush=True)
    urllib.request.urlretrieve(URL, archive)
actual = hashlib.file_digest(archive.open("rb"), "sha256").hexdigest()
if actual != SHA256:
    raise SystemExit("Checksum mismatch: " + actual)
print("Verified SHA-256", actual, flush=True)
with zipfile.ZipFile(archive) as z:
    z.extractall(DEST)
print(DEST / "ghidra_11.4.2_PUBLIC", flush=True)
