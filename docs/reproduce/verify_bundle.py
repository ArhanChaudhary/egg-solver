"""Verify documentation file bytes; --update explicitly refreshes the manifest."""
from pathlib import Path
import argparse
import hashlib

ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / 'MANIFEST.sha256'


def documentation_files():
    return sorted(p for p in ROOT.rglob('*')
                  if p.is_file() and p != MANIFEST
                  and '__pycache__' not in p.parts and p.suffix != '.pyc')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--update', action='store_true', help='replace the checksum list after intentional edits')
    args = parser.parse_args()
    files = documentation_files()
    if args.update:
        MANIFEST.write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest() + '  '
                                   + str(p.relative_to(ROOT)) + '\n' for p in files))
        print(f'Updated {len(files)} documentation checksums')
        return
    expected = {}
    for line in MANIFEST.read_text().splitlines():
        digest, name = line.split('  ', 1)
        path = (ROOT / name).resolve()
        if not path.is_relative_to(ROOT.resolve()) or name in expected:
            raise SystemExit('Invalid manifest path: ' + name)
        expected[name] = digest
    actual = {str(p.relative_to(ROOT)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files}
    changed = sorted(name for name in actual.keys() | expected.keys()
                     if actual.get(name) != expected.get(name))
    if changed:
        raise SystemExit('Documentation checksum mismatch:\n' + '\n'.join(changed))
    print(f'PASS: {len(files)} documentation files verified')


if __name__ == '__main__':
    main()
