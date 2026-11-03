"""Independent HarfBuzz/FriBidi oracle for CAD-ADV-10's opt-in glyph layout.

Requires the tools named in the fixture's versions; never calls KentOS.
--check compares generated reference data without changing the repository.
These cases exercise established Latin/Arabic/Hebrew characters; they do not
claim full Unicode-version conformance (the tools' data versions differ).
"""
import argparse
import ctypes
import ctypes.util
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'fixtures/text/v1/shaping.json'


def run(args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs).stdout


def line_order(text, start, end, base):
    """FriBidi C paragraph levels + actual wrapped-line L1/L2, no KentOS."""
    library = ctypes.util.find_library('fribidi')
    if library is None:
        directory = Path(run(['pkg-config', '--variable=libdir', 'fribidi']).strip())
        library = next((str(directory / name) for name in ['libfribidi.dylib', 'libfribidi.so']
                        if (directory / name).is_file()), None)
    if library is None: raise RuntimeError('FriBidi C kütüphanesi bulunamadı.')
    api = ctypes.CDLL(library)
    u32, i32, level = ctypes.c_uint32, ctypes.c_int, ctypes.c_int8
    pointer = ctypes.POINTER
    api.fribidi_get_bidi_type.argtypes = [u32]
    api.fribidi_get_bidi_type.restype = u32
    api.fribidi_get_bidi_types.argtypes = [pointer(u32), i32, pointer(u32)]
    api.fribidi_get_bracket_types.argtypes = [pointer(u32), i32, pointer(u32), pointer(u32)]
    api.fribidi_get_par_embedding_levels_ex.argtypes = [pointer(u32), pointer(u32), i32, pointer(u32), pointer(level)]
    api.fribidi_get_par_embedding_levels_ex.restype = level
    api.fribidi_reorder_line.argtypes = [u32, pointer(u32), i32, i32, u32, pointer(level), pointer(u32), pointer(i32)]
    api.fribidi_reorder_line.restype = level
    n = len(text)
    chars = (u32 * n)(*map(ord, text))
    types, brackets, levels = (u32 * n)(), (u32 * n)(), (level * n)()
    mapping = (i32 * n)(*range(n))
    direction = u32(api.fribidi_get_bidi_type(ord({'auto': '!', 'ltr': 'A', 'rtl': 'א'}[base])))
    api.fribidi_get_bidi_types(chars, n, types)
    api.fribidi_get_bracket_types(chars, n, types, brackets)
    assert api.fribidi_get_par_embedding_levels_ex(types, brackets, n, ctypes.byref(direction), levels)
    assert api.fribidi_reorder_line(0, types, end - start, start, direction, levels, chars, mapping)
    return list(mapping)[start:end]


def build():
    from fontTools.ttLib import TTFont
    source = (ROOT / 'crates/shared/geometry-core/src/text/font_data.rs').read_text()
    files = re.findall(r'include_bytes!\("([^"]+)"\)', source)
    keys = re.findall(r'\("([^"]+)", (\d+), (true|false)\)', source)
    assert len(files) == len(keys) == 29
    cases = []
    for relative, (family, weight, italic) in zip(files, keys):
        path = (ROOT / 'crates/shared/geometry-core/src/text' / relative).resolve()
        text = ('العربية' if family == 'noto-sans-arabic' else
                'שָׁלוֹם' if family == 'noto-sans-hebrew' else
                '⌀ ≤ ≥ ± ×' if family == 'noto-sans-math' else 'AV — Türkçe Ög\u0306ç')
        direction = 'rtl' if family in ['noto-sans-arabic', 'noto-sans-hebrew'] else 'ltr'
        glyphs = json.loads(run(['hb-shape', str(path), text, '--no-glyph-names', '--output-format=json',
                                 '--utf8-clusters', '--cluster-level=0', '--shapers=ot', '--language=und',
                                 '--features=kern=1,liga=1,clig=1', '--direction=' + direction]))
        assert all(g['g'] != 0 for g in glyphs), (family, text, glyphs)
        cases.append(dict(name=path.name, family=family, weight=int(weight), italic=italic == 'true',
                          text=text, direction=direction, unitsPerEm=TTFont(path)['head'].unitsPerEm,
                          fontSha256=hashlib.sha256(path.read_bytes()).hexdigest(), glyphs=glyphs))
    bidi = []
    for text in ['ABC אבג 123', 'Türkçe / العربية / עברית 123', '(אבג) ABC 12', 'אבג 12 ABC',
                 'A [אבג] B', 'A\u2067אבג 123\u2069 B']:
        for base in ['auto', 'ltr', 'rtl']:
            args = ['fribidi', '--vtol', '--novisual', '--nopad', '--nobreak']
            if base != 'auto': args.append('--' + base)
            positions = [int(p) for p in run(args, input=text + '\n').split()]
            assert sorted(positions) == list(range(len(text)))
            bidi.append(dict(text=text, direction=base, visualToLogical=positions))
    wrapped = []
    for text, start, end in [('אבג ABC 12 (XYZ) סוף', 4, 10),
                             ('אבג ABC 12 (XYZ) סוף', 8, 17),
                             ('A אבג 12 XYZ Z', 6, 12),
                             ('A \u2067אבג 123\u2069 B', 4, 10)]:
        for base in ['auto', 'ltr', 'rtl']:
            wrapped.append(dict(text=text, start=start, end=end, direction=base,
                                visualToLogical=line_order(text, start, end, base)))
    styled = []
    for text, family, rtl, split in [('سلام', 'noto-sans-arabic', True, 2),
                                     ('AVATAR', 'arimo', False, 2)]:
        spans = []
        for start, end, weight in [(0, split, 400), (split, len(text), 600)]:
            face = next(c for c in cases if c['family'] == family and c['weight'] == weight and not c['italic'])
            path = next((ROOT / 'crates/shared/geometry-core/src/text' / f).resolve()
                        for f in files if Path(f).name == face['name'])
            glyphs = json.loads(run(['hb-shape', str(path), text[start:end], '--no-glyph-names',
                '--output-format=json', '--utf8-clusters', '--cluster-level=0', '--shapers=ot',
                '--language=und', '--features=kern=1,liga=1,clig=1', '--direction=' + ('rtl' if rtl else 'ltr'),
                '--text-before=' + text[:start], '--text-after=' + text[end:]]))
            spans.append(dict(start=start, end=end, weight=weight, glyphs=glyphs, unitsPerEm=face['unitsPerEm']))
        styled.append(dict(text=text, family=family, direction='rtl' if rtl else 'ltr', spans=spans))
    return dict(format='kentos.text-shaping-reference', version=1,
                harfBuzz=run(['hb-shape', '--version']).splitlines()[0],
                friBidi=run(['fribidi', '--version']).splitlines()[:3], cases=cases, bidi=bidi, wrapped=wrapped, styled=styled)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    made = json.dumps(build(), ensure_ascii=False, indent=2) + '\n'
    if args.check:
        if OUT.read_text() != made: raise SystemExit('Metin şekillendirme referansı güncel değil.')
    else: OUT.write_text(made)
    print('29 HarfBuzz yüzü, 18 yön, 12 bağlamlı satır ve 2 biçimli metin referansı doğrulandı.')


if __name__ == '__main__': main()
