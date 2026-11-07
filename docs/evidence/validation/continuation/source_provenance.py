"""Fingerprint checked source independently of documentation-only commits."""
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path('/data/agents/codex-resume')


def fingerprint(repo):
    repo = Path(repo)
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=repo).decode().split('\0')
    names = sorted(name for name in tracked if name and (
        name.endswith('.rs') or name.endswith('/Cargo.toml')
        or name in ['Cargo.toml', 'Cargo.lock', '.cargo/config.toml', 'miri.sh', 'check.sh']))
    result = hashlib.sha256()
    for name in names:
        result.update(name.encode() + b'\0')
        result.update(hashlib.sha256((repo / name).read_bytes()).digest())
    return dict(sha256=result.hexdigest(), files=names)


def require_miri(repo):
    source = fingerprint(repo)
    results = {}
    for name, filename in [('x86', 'miri-validation.json'), ('portable', 'miri-portable-validation.json')]:
        path = ROOT / filename
        if not path.is_file():
            raise RuntimeError('Current-source Miri validation is not complete: ' + name)
        facts = json.loads(path.read_text())
        if facts.get('status') != 'PASS' or facts['source'] != source:
            raise RuntimeError('Miri validation does not match the current source: ' + name)
        results[name] = facts
    return results
