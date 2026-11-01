#!/usr/bin/env python3
"""Build a local macOS .app with the supplied KentOS icon. No download or source mutation.

python3 scripts/desktop/bundle_macos.py target/release/kentos-cad /Applications/KentOS.app
The output must not exist; --module can select mimari, harita or cbs (gis alias).
"""
import argparse
import plistlib
import shutil
import subprocess
import tempfile
from pathlib import Path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('binary', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--module', choices=['mimari', 'harita', 'cbs', 'gis'])
    parser.add_argument('--name')
    parser.add_argument('--identifier')
    args = parser.parse_args()
    if args.module == 'gis':
        args.module = 'cbs'
    args.name = args.name or {'mimari': 'KentOS Mimari', 'harita': 'KentOS Harita', 'cbs': 'KentOS GIS'}.get(args.module, 'KentOS CAD')
    args.identifier = args.identifier or 'com.kentos.' + (args.module or 'cad')
    if not args.binary.is_file() or args.output.exists() or args.output.suffix != '.app':
        parser.error('The binary must exist and the output must be a new .app path.')
    source = Path(__file__).resolve().parents[2] / 'assets/images/kentos_app_icon.png'
    # Convert into a temporary iconset before creating any output bundle.
    with tempfile.TemporaryDirectory(prefix='kentos-icon-') as folder:
        iconset = Path(folder) / 'KentOS.iconset'
        iconset.mkdir()
        for size in [16, 32, 128, 256, 512]:
            for factor in [1, 2]:
                target = iconset / f'icon_{size}x{size}{"@2x" if factor == 2 else ""}.png'
                subprocess.run(['sips', '-z', str(size*factor), str(size*factor), str(source), '--out', str(target)], check=True, stdout=subprocess.DEVNULL)
        icns = Path(folder) / 'KentOS.icns'
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(icns)], check=True)
        contents = args.output / 'Contents'
        binary_dir = contents / 'MacOS'
        resources = contents / 'Resources'
        binary_dir.mkdir(parents=True)
        resources.mkdir()
        shutil.copy2(args.binary, binary_dir / 'kentos-cad')
        (binary_dir / 'kentos-cad').chmod(0o755)
        executable = 'kentos-cad'
        if args.module:
            executable = 'start-kentos'
            wrapper = binary_dir / executable
            wrapper.write_text('#!/bin/sh\nexec "$(dirname "$0")/kentos-cad" --modul ' + args.module + ' "$@"\n')
            wrapper.chmod(0o755)
        shutil.copy2(icns, resources / 'KentOS.icns')
        info = dict(CFBundleName=args.name, CFBundleDisplayName=args.name,
                    CFBundleIdentifier=args.identifier, CFBundleExecutable=executable,
                    CFBundleIconFile='KentOS.icns', CFBundlePackageType='APPL',
                    NSHighResolutionCapable=True)
        (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
    print(args.output)


if __name__ == '__main__':
    main()
