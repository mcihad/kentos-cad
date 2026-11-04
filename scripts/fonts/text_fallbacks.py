"""CAD-ADV-10's bundled Unicode fallback faces (ADR 0914).

Runtime never downloads a font. --check verifies shipped bytes and licences
without network access. --build rebuilds from pinned, hash-checked upstream
sources (FontTools version in manifest.json); --source-dir can supply an
existing offline source cache. Modified faces keep the upstream OFL notice.

  .run/py/bin/python scripts/fonts/text_fallbacks.py --check
  .run/py/bin/python scripts/fonts/text_fallbacks.py --build --source-dir CACHE
"""
import argparse
import hashlib
import importlib.metadata
import io
import json
import re
from pathlib import Path
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'crates/shared/geometry-core/data/text-fonts'


def checked_bytes(path, source, fetch):
    if not path.exists():
        if not fetch:
            raise SystemExit(f'Kaynak önbellekte yok: {path}; indirmek için --fetch kullanın.')
        path.parent.mkdir(parents=True, exist_ok=True)
        data = urllib.request.urlopen(source['url'], timeout=60).read()
    else:
        data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != source['sha256']:
        raise SystemExit(f'Kaynak özeti uyuşmuyor: {path}')
    if not path.exists():
        path.write_bytes(data)
    return data


def build_face(data, weight, date):
    from fontTools.ttLib import TTFont
    from fontTools.varLib.instancer import instantiateVariableFont
    from fontTools.subset import Options, Subsetter
    font = TTFont(io.BytesIO(data))
    font.recalcTimestamp = False
    if 'fvar' in font:
        axes = {a.axisTag: a.defaultValue for a in font['fvar'].axes}
        axes.update(wght=weight)
        font = instantiateVariableFont(font, axes, inplace=True)
    options = Options()
    options.recalc_timestamp = False
    options.name_IDs = ['*']
    options.name_legacy = True
    options.name_languages = ['*']
    subset = Subsetter(options=options)
    subset.populate(unicodes=[c for c in font.getBestCmap() if c <= 0xffff])
    subset.subset(font)
    font['head'].created = font['head'].modified = date
    out = io.BytesIO()
    font.save(out)
    return out.getvalue()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--build', action='store_true')
    parser.add_argument('--fetch', action='store_true')
    parser.add_argument('--source-dir', type=Path)
    args = parser.parse_args()
    manifest = json.loads((OUT / 'manifest.json').read_text())
    if args.build:
        version = importlib.metadata.version('fonttools')
        if version != manifest['fontTools']:
            raise SystemExit(f"FontTools {manifest['fontTools']} gerekli; kurulu: {version}")
        cache = args.source_dir or Path.home() / '.cache/kentos/fonts' / manifest['sourceCommit']
        for source in manifest['sources']:
            folder = source['folder']
            data = checked_bytes(cache / folder / source['font']['name'], source['font'], args.fetch)
            licence = checked_bytes(cache / folder / 'OFL.txt', source['licence'], args.fetch)
            for face in manifest['faces']:
                if face['file'].rsplit('-', 1)[0].lower() != folder:
                    continue
                made = build_face(data, face['weight'], manifest['date'])
                if hashlib.sha256(made).hexdigest() != face['sha256']:
                    raise SystemExit(f"Yeniden üretim özeti uyuşmuyor: {face['file']}")
                if not args.check:
                    (OUT / face['file']).write_bytes(made)
            name = next(f['file'].rsplit('-', 1)[0] for f in manifest['faces'] if f['file'].rsplit('-', 1)[0].lower() == folder)
            if not args.check:
                (OUT / (name + '-OFL.txt')).write_bytes(licence)
    # This is a frozen rendering profile, not a discovery list. A font update
    # requires another profile while this profile's bytes remain available.
    code = ROOT / 'crates/shared/geometry-core/src/text/font_data.rs'
    source = code.read_text()
    files = re.findall(r'include_bytes!\("([^"]+)"\)', source)
    keys = re.findall(r'\("([^"]+)", (\d+), (true|false)\)', source)
    profile = manifest['profileFaces']
    if manifest['profile'] != 'unicode-v1' or len(files) != len(keys) or len(files) != len(profile):
        raise SystemExit('Unicode v1 font kataloğunun sürümü/sayısı değişmiş.')
    for index, (relative, key, expected) in enumerate(zip(files, keys, profile)):
        path = (code.parent / relative).resolve()
        actual = dict(index=index, file=str(path.relative_to(ROOT)), family=key[0], weight=int(key[1]),
                      italic=key[2] == 'true', sha256=hashlib.sha256(path.read_bytes()).hexdigest())
        if actual != expected:
            raise SystemExit(f'Unicode v1 font profili değişmiş: {path}; eski profili koruyarak yeni sürüm tanımlayın.')
    for face in manifest['faces']:
        path = OUT / face['file']
        if hashlib.sha256(path.read_bytes()).hexdigest() != face['sha256']:
            raise SystemExit(f'Paketli yazı tipi değişmiş: {path}')
    for source in manifest['sources']:
        name = next(f['file'].rsplit('-', 1)[0] for f in manifest['faces'] if f['file'].rsplit('-', 1)[0].lower() == source['folder'])
        if hashlib.sha256((OUT / (name + '-OFL.txt')).read_bytes()).hexdigest() != source['licence']['sha256']:
            raise SystemExit(f'Lisans değişmiş: {name}')
    print(f"{len(profile)} sabit profil yüzü, {len(manifest['faces'])} üretilen font ve {len(manifest['sources'])} OFL lisansı doğrulandı.")


if __name__ == '__main__':
    main()
