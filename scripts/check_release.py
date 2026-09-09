#!/usr/bin/env python3
"""Local, offline checks for source links and a versioned release bundle."""
import hashlib
import json
import plistlib
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parent.parent


def version():
    return subprocess.check_output([str(ROOT / 'scripts/version.sh')], text=True).strip()


def fingerprint():
    files = [ROOT / p for p in ('Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml', 'build.command')]
    files += sorted((ROOT / 'src').rglob('*.rs'))
    files += sorted((ROOT / 'native').glob('*'))
    digest = hashlib.sha256()
    for path in files:
        digest.update(str(path.relative_to(ROOT)).encode())
        digest.update(b'\0')
        digest.update(path.read_bytes())
        digest.update(b'\0')
    return digest.hexdigest()


def links():
    documents = sorted(ROOT.glob('*.md')) + sorted((ROOT / 'docs').rglob('*.md'))
    errors = []
    for doc in documents:
        text = re.sub(r'```.*?```', '', doc.read_text(), flags=re.S)
        for dest in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', text):
            if re.match(r'^[a-zA-Z][a-zA-Z0-9+.-]*:', dest) or dest.startswith('#'):
                continue
            target = unquote(dest.split('#', 1)[0])
            if not (doc.parent / target).exists():
                errors.append(f'{doc.relative_to(ROOT)}: missing {target}')
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Checked local links in {len(documents)} Markdown files.')


def bundle():
    app = ROOT / 'dist/Music Native Names.app'
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    assert info['CFBundleShortVersionString'] == version(), 'Stale bundle version'
    stamp = json.loads((ROOT / 'dist/build-info.json').read_text())
    assert stamp == {'version': version(), 'source_sha256': fingerprint()}, 'Sources changed; rebuild first'
    binary = app / 'Contents/MacOS/music-native-names'
    arches = subprocess.check_output(['lipo', '-archs', str(binary)], text=True).strip()
    assert arches == 'arm64', f'Unexpected binary architectures: {arches}'
    subprocess.run(['codesign', '--verify', '--strict', str(app)], check=True)
    print(f'Verified {version()} arm64 bundle, signature, and source fingerprint.')


if __name__ == '__main__':
    if sys.argv[1:] == ['--stamp']:
        (ROOT / 'dist/build-info.json').write_text(json.dumps({'version': version(), 'source_sha256': fingerprint()}, indent=2) + '\n')
    elif sys.argv[1:] == ['--bundle']:
        links()
        bundle()
    else:
        links()
