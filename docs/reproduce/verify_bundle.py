"""Check every distributed file against MANIFEST.sha256."""
from pathlib import Path
import hashlib
root=Path(__file__).resolve().parents[1]
count=0
for line in (root/"MANIFEST.sha256").read_text().splitlines():
    expected,name=line.split("  ",1)
    path=root/name
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest()!=expected:
        raise SystemExit("FAILED: "+name)
    count+=1
print(f"PASS: {count} distributed files verified")
