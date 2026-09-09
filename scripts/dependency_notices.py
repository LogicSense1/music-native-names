#!/usr/bin/env python3
"""Collect dependency license metadata and original texts from Cargo's locked graph."""
import json
import shutil
import subprocess
import sys
from pathlib import Path

root = Path(__file__).resolve().parent.parent
out = Path(sys.argv[1])
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--locked', '--format-version', '1', '--filter-platform', 'aarch64-apple-darwin'], cwd=root))
workspace = set(metadata['workspace_members'])
nodes = {node['id']: node for node in metadata['resolve']['nodes']}
reachable = set()
pending = list(workspace)
while pending:
    node_id = pending.pop()
    if node_id in reachable:
        continue
    reachable.add(node_id)
    pending.extend(dep['pkg'] for dep in nodes[node_id]['deps'])
lines = ['# Third-party notices', '', 'Dependency versions and license texts from Cargo.lock.', '', '| Package | License | Source |', '| --- | --- | --- |']
for package in sorted(metadata['packages'], key=lambda p: (p['name'], p['version'])):
    if package['id'] in workspace or package['id'] not in reachable:
        continue
    name = package['name'] + '-' + package['version']
    path = Path(package['manifest_path']).parent
    target = out / 'LICENSES' / name
    target.mkdir(parents=True, exist_ok=True)
    files = set()
    for pattern in ['LICENSE*', 'LICENCE*', 'COPYING*']:
        files.update(p for p in path.glob(pattern) if p.is_file())
    if package.get('license_file'):
        files.add(path / package['license_file'])
    for source in sorted(files):
        shutil.copyfile(source, target / source.name)
    license_name = package.get('license') or 'See included license file'
    if not files:
        raise SystemExit(f'No license text found for {name}; review before release.')
    lines.append(f"| {name} | {license_name} | https://crates.io/crates/{package['name']}/{package['version']} |")
(out / 'THIRD_PARTY_NOTICES.md').write_text('\n'.join(lines) + '\n')
