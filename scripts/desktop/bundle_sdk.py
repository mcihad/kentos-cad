#!/usr/bin/env python3
"""Put KentOS's Python library beside a packaged program (plan/eklenti.md ADIM 18A; docs/adr/8057 §5; PY-UV-31).

The library is the `kentos` package, whose native module maturin builds from crates/native/python. The program installs
it into the user's environments from this wheel alone, offline, after checking the description written here: its file,
version and SHA-256. Placed:

  Windows, Linux:  <program folder>/tools/kentos-<version>-cp310-abi3-<platform>.whl and tools/kentos-sdk.json
  macOS:           <KentOS.app>/Contents/Resources/python/ (a wheel is a file the program reads, not code the bundle
                   runs; Apple keeps Resources for such files)

  python3 scripts/desktop/bundle_sdk.py build/KentOS                         # builds the wheel (release) here
  python3 scripts/desktop/bundle_sdk.py build/KentOS --wheel dist/kentos-0.1.0-cp310-abi3-win_amd64.whl
  python3 scripts/desktop/bundle_sdk.py target/debug --debug                 # a quick wheel to try a debug program

The wheel's version must be the program's (the workspace's version in Cargo.toml): the program refuses another. maturin
builds for the computer it runs on; a wheel for another system is built on that system and passed with --wheel.
"""
import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DESCRIPTION = 'kentos-sdk.json'


def fail(why):
    print(f'bundle_sdk: {why}', file=sys.stderr)
    sys.exit(1)


def workspace_version():
    text = (ROOT / 'Cargo.toml').read_text(encoding='utf-8')
    section = re.search(r'^\[workspace\.package\]\s*$(.*?)^\[', text, re.S | re.M)
    found = section and re.search(r'^version\s*=\s*"([^"]+)"', section.group(1), re.M)
    if not found:
        fail('the workspace version is not in Cargo.toml')
    return found.group(1)


def sha256_of(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as f:
        for chunk in iter(lambda: f.read(1 << 16), b''):
            digest.update(chunk)
    return digest.hexdigest()


def maturin():
    own = ROOT / '.run/py/bin/maturin'
    return str(own) if own.exists() else (shutil.which('maturin') or fail('maturin is not found (pip install maturin)'))


def build(debug, out):
    command = [maturin(), 'build', '--strip', '-q', '-o', str(out)]
    if not debug:
        command.insert(2, '--release')
    print('build ', ' '.join(command[1:]))
    subprocess.run(command, cwd=ROOT / 'python', check=True)
    wheels = sorted(Path(out).glob('kentos-*.whl'))
    if len(wheels) != 1:
        fail(f'maturin left {len(wheels)} wheels in {out}')
    return wheels[0]


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('output', type=Path, help='the program folder, or KentOS.app on macOS')
    parser.add_argument('--wheel', type=Path, help='a wheel built already (on the target system)')
    parser.add_argument('--debug', action='store_true', help='build a debug wheel (quick, large) instead of release')
    args = parser.parse_args()
    if args.output.suffix == '.app':
        if not (args.output / 'Contents').is_dir():
            parser.error('the .app has no Contents (make it with bundle_macos.py)')
        place = args.output / 'Contents/Resources/python'
    elif args.output.is_dir():
        place = args.output / 'tools'
    else:
        parser.error(f'{args.output} is not a folder')
    version = workspace_version()
    with tempfile.TemporaryDirectory(prefix='kentos-sdk-') as scratch:
        wheel = args.wheel or build(args.debug, scratch)
        if not wheel.is_file():
            fail(f'{wheel} is not a file')
        if not re.match(rf'^kentos-{re.escape(version)}-cp\d+-abi3-[\w.]+\.whl$', wheel.name):
            fail(f'{wheel.name} is not kentos {version} (the program is {version})')
        place.mkdir(parents=True, exist_ok=True)
        # Only this version's wheel stays: an older one beside it would be a second library.
        for old in place.glob('kentos-*.whl'):
            if old.name != wheel.name:
                old.unlink()
        target = place / wheel.name
        part = target.with_name(target.name + '.part')
        shutil.copyfile(wheel, part)
        os.replace(part, target)
    described = {'format': 'kentos.sdk-wheel', 'version': version, 'file': target.name, 'sha256': sha256_of(target)}
    text = place / (DESCRIPTION + '.part')
    text.write_text(json.dumps(described, indent=2) + '\n', encoding='utf-8')
    os.replace(text, place / DESCRIPTION)
    print(f'placed {target} ({target.stat().st_size} bytes, SHA-256 {described["sha256"]})')
    print(f'placed {place / DESCRIPTION}')


if __name__ == '__main__':
    main()
