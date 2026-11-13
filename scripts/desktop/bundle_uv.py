#!/usr/bin/env python3
"""Put the uv KentOS's installation carries into a packaged program (plan/eklenti.md ADIM 18A; docs/adr/8057 §5).

The owner's decision (13 November 2026): uv ships inside the KentOS installation; Python is downloaded later, per
user and only with their consent. This step reads uv's version and each target's SHA-256 from
crates/native/python-env/compat.json (the same manifest the program reads), downloads uv's official release archive,
checks its SHA-256 BEFORE opening it, and places only uv and its two licences:

  Windows, Linux:  <program folder>/tools/uv[.exe], tools/uv-LICENSE-MIT, tools/uv-LICENSE-APACHE
  macOS:           <KentOS.app>/Contents/MacOS/uv, flat beside the program, where Apple asks helper tools to be
                   (Resources is not for code; code in nested folders breaks signing); the licences in
                   Contents/Resources/licenses/. Run it after bundle_macos.py, and sign uv before the bundle.

  python3 scripts/desktop/bundle_uv.py x86_64-pc-windows-msvc build/KentOS
  python3 scripts/desktop/bundle_uv.py aarch64-apple-darwin build/KentOS.app
  python3 scripts/desktop/bundle_uv.py --list

--cache DIR keeps the downloads and reuses a file whose SHA-256 matches. --offline never downloads: a computer
without internet copies the archive and the licences (named as --list shows) into the cache beforehand. A file whose
SHA-256 differs is never used. Nothing in the archive but uv is written; uvx and uvw are left out.
"""
import argparse
import hashlib
import json
import os
import platform
import subprocess
import sys
import tarfile
import tempfile
import urllib.request
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = ROOT / 'crates/native/python-env/compat.json'
# Bytes read at most from a download or an archive member: uv is about 75 MB, its archive about 25 MB.
LIMIT = 300 * 1024 * 1024
CHUNK = 1 << 16


def fail(why):
    print(f'bundle_uv: {why}', file=sys.stderr)
    sys.exit(1)


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(CHUNK), b''):
            digest.update(chunk)
    return digest.hexdigest()


def fetch(url, dest, sha256, offline):
    """`dest` holding the file at `url` whose SHA-256 is `sha256`: reused when it matches, else downloaded."""
    if dest.is_file():
        if sha256_of(dest) == sha256:
            print(f'cache  {dest.name}')
            return dest
        if offline:
            fail(f'{dest} does not match its SHA-256 in the manifest; it is not used.')
    if offline:
        fail(f'--offline: {dest} is missing (copy it from {url}).')
    if not url.startswith('https://'):
        fail(f'not an https address: {url}')
    part = dest.with_name(dest.name + '.part')
    digest = hashlib.sha256()
    size = 0
    request = urllib.request.Request(url, headers={'User-Agent': 'kentos-packaging'})
    with urllib.request.urlopen(request, timeout=60) as response, open(part, 'wb') as out:
        for chunk in iter(lambda: response.read(CHUNK), b''):
            size += len(chunk)
            if size > LIMIT:
                out.close()
                part.unlink()
                fail(f'{url} is larger than {LIMIT} bytes.')
            digest.update(chunk)
            out.write(chunk)
    if digest.hexdigest() != sha256:
        part.unlink()
        fail(f'{url}: SHA-256 {digest.hexdigest()} is not the manifest\'s {sha256}; nothing was placed.')
    os.replace(part, dest)
    print(f'fetch  {dest.name} ({size} bytes, SHA-256 checked)')
    return dest


def read_uv(archive, name):
    """uv's bytes from a checked archive: the one regular file named `name`, nothing extracted beside it."""
    if archive.name.endswith('.zip'):
        with zipfile.ZipFile(archive) as z:
            members = [i for i in z.infolist() if not i.is_dir() and Path(i.filename).name == name]
            if len(members) != 1 or members[0].file_size > LIMIT:
                fail(f'{archive.name} does not hold one {name}.')
            return z.read(members[0])
    with tarfile.open(archive, 'r:gz') as t:
        members = [m for m in t.getmembers() if m.isfile() and Path(m.name).name == name]
        if len(members) != 1 or members[0].size > LIMIT:
            fail(f'{archive.name} does not hold one {name}.')
        return t.extractfile(members[0]).read()


def write_atomic(path, data, mode=None):
    path.parent.mkdir(parents=True, exist_ok=True)
    part = path.with_name(path.name + '.part')
    part.write_bytes(data)
    if mode is not None:
        part.chmod(mode)
    os.replace(part, path)


def host_target():
    machine = platform.machine().lower()
    arch = {'amd64': 'x86_64', 'x86_64': 'x86_64', 'arm64': 'aarch64', 'aarch64': 'aarch64'}.get(machine, machine)
    system = platform.system()
    tail = {'Linux': 'unknown-linux-gnu', 'Windows': 'pc-windows-msvc', 'Darwin': 'apple-darwin'}.get(system)
    return f'{arch}-{tail}' if tail else None


def main():
    bundled = json.loads(MANIFEST.read_text(encoding='utf-8'))['bundledUv']
    targets = bundled['targets']
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('target', nargs='?', choices=sorted(targets))
    parser.add_argument('output', nargs='?', type=Path, help='the program folder, or KentOS.app on macOS')
    parser.add_argument('--cache', type=Path, help='keep and reuse the downloads here')
    parser.add_argument('--offline', action='store_true', help='never download; use only --cache')
    parser.add_argument('--list', action='store_true', help='the targets and the files each needs')
    args = parser.parse_args()
    licenses = [(Path(l['url']).name, l['url'], l['sha256']) for l in bundled['licenses']]
    if args.list:
        print(f'uv {bundled["version"]}')
        for target, asset in sorted(targets.items()):
            print(f'  {target:28} {asset["file"]}  {asset["sha256"]}')
        for name, _, sha256 in licenses:
            print(f'  {"(licence)":28} uv-{name}  {sha256}')
        return
    if not args.target or not args.output:
        parser.error('a target and an output folder are needed (or --list)')
    if args.offline and not args.cache:
        parser.error('--offline needs --cache')
    macos = args.target.endswith('apple-darwin')
    windows = args.target.endswith('pc-windows-msvc')
    if macos:
        if args.output.suffix != '.app' or not (args.output / 'Contents/MacOS').is_dir():
            parser.error('on macOS the output is a KentOS.app with Contents/MacOS (make it with bundle_macos.py)')
        tools = args.output / 'Contents/MacOS'
        licence_dir = args.output / 'Contents/Resources/licenses'
    else:
        if not args.output.is_dir():
            parser.error(f'{args.output} is not a folder')
        tools = args.output / 'tools'
        licence_dir = tools
    asset = targets[args.target]
    with tempfile.TemporaryDirectory(prefix='kentos-uv-') as scratch:
        cache = args.cache or Path(scratch)
        cache.mkdir(parents=True, exist_ok=True)
        archive = fetch(bundled['download'] + asset['file'], cache / asset['file'], asset['sha256'], args.offline)
        texts = [(name, fetch(url, cache / f'uv-{name}', sha256, args.offline)) for name, url, sha256 in licenses]
        name = 'uv.exe' if windows else 'uv'
        data = read_uv(archive, name)
        uv = tools / name
        write_atomic(uv, data, None if windows else 0o755)
        for licence, source in texts:
            write_atomic(licence_dir / f'uv-{licence}', source.read_bytes())
    print(f'placed {uv} ({len(data)} bytes, SHA-256 {hashlib.sha256(data).hexdigest()})')
    print(f'placed {", ".join(f"uv-{n}" for n, _, _ in licenses)} in {licence_dir}')
    if args.target != host_target():
        print(f'not run: built for {args.target}, this computer is {host_target()}')
        return
    ran = subprocess.run([str(uv), '--version'], capture_output=True, text=True, timeout=30)
    said = ran.stdout.strip()
    if ran.returncode != 0 or not said.startswith(f'uv {bundled["version"]}'):
        fail(f'{uv} --version said “{said or ran.stderr.strip()}”, not uv {bundled["version"]}')
    print(f'ran    {said}')


if __name__ == '__main__':
    main()
