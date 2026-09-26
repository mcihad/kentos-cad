"""The desktop's drawing typefaces, made from the web's own font files (docs/adr/0055).

The web draws the drawing's text (text objects, dimension values, labels) in
the project's typeface (DrawingFont: Barlow, Arimo, Overpass, Quicksand,
Architects Daughter, Courier Prime, IBM Plex Mono) from the WOFF2 files in
apps/web/src/assets/fonts, loaded by apps/web/src/styles/fonts.css in two
subsets each (latin, latin-ext). The desktop's text system reads TrueType,
not WOFF2, so this writes one TrueType file per face the drawing uses:
the two subsets merged, and variable fonts (Arimo, Overpass, Quicksand)
instanced at the weights the drawing asks for (400, 500, 600, 700). The
glyphs and their advances are the web's; nothing is taken from elsewhere.

    python3 scripts/fonts/drawing_fonts.py           # writes apps/desktop/assets/fonts/drawing
    python3 scripts/fonts/drawing_fonts.py --check   # writes nothing; compares what it would write

The files are not compared byte for byte (fontTools versions lay tables out
differently): `--check` compares every face's names, weight, style, the
characters it maps and each character's advance with what it would write.

Needs fontTools with brotli (WOFF2).
"""
import io
import json
import re
import sys
from pathlib import Path

from fontTools.merge import Merger
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = Path(__file__).resolve().parents[2]
WEB = ROOT / "apps/web/src/assets/fonts"
CSS = ROOT / "apps/web/src/styles/fonts.css"
OUT = ROOT / "apps/desktop/assets/fonts/drawing"

# DrawingFont → the CSS family and its folder (apps/web/src/model/projectSettings.ts).
FAMILIES = {
    "Barlow": "barlow",
    "Arimo": "arimo",
    "Overpass": "overpass",
    "Quicksand": "quicksand",
    "Architects Daughter": "architects-daughter",
    "Courier Prime": "courier-prime",
    "IBM Plex Mono": "ibm-plex-mono",
}
# The weights the drawing's text asks for: text 400 (italic), dimension values
# 500, labels 500 by default and up to 700.
WEIGHTS = (400, 500, 600, 700)
# A fixed date in every file (the head table's created and modified), so a run
# changes a file only when its glyphs do.
DATE = 3_841_516_800  # 2025-09-26 in seconds since 1904


def faces():
    """(family, style, weights or (lo, hi) for a variable font, [latin-ext file, latin file])."""
    css = CSS.read_text()
    found = {}
    for block in re.findall(r"@font-face\s*{([^}]*)}", css):
        family = re.search(r"font-family:\s*'([^']+)'", block).group(1)
        if family not in FAMILIES:
            continue
        style = re.search(r"font-style:\s*(\w+)", block).group(1)
        weight = re.search(r"font-weight:\s*([^;]+);", block).group(1).split()
        src = re.search(r"url\('([^']+)'\)", block).group(1).split("/")[-1]
        key = (family, style, tuple(int(w) for w in weight))
        found.setdefault(key, []).append(src)
    # The latin-ext subset first: merged later, latin's glyphs win where both have one.
    return [(f, s, w, sorted(files, key=lambda n: "latin-ext" not in n)) for (f, s, w), files in sorted(found.items())]


def load(family, name):
    return TTFont(WEB / FAMILIES[family] / name)


def as_static(font, weight):
    """A variable font's instance at `weight`, or the font as it is."""
    if "fvar" not in font:
        return font
    return instantiateVariableFont(font, {"wght": weight})


def ttf_bytes(font):
    buf = io.BytesIO()
    font.flavor = None
    font.save(buf, reorderTables=True)
    return buf.getvalue()


def merged(family, style, weight, files):
    """One face from the latin-ext and latin subsets."""
    parts = []
    for name in files:
        font = as_static(load(family, name), weight)
        parts.append(io.BytesIO(ttf_bytes(font)))
    if len(parts) == 1:
        font = TTFont(parts[0])
    else:
        font = Merger().merge(parts)
    font["head"].created = DATE
    font["head"].modified = DATE
    font["OS/2"].usWeightClass = weight
    italic = style == "italic"
    fs = font["OS/2"].fsSelection & ~0b1100001  # italic, bold, regular bits
    font["OS/2"].fsSelection = fs | (0b1 if italic else 0) | (0b100000 if weight >= 700 else 0) | (0b1000000 if weight == 400 and not italic else 0)
    font["head"].macStyle = (0b10 if italic else 0) | (0b1 if weight >= 700 else 0)
    sub = {400: "Regular", 500: "Medium", 600: "SemiBold", 700: "Bold"}[weight]
    sub = f"{sub} Italic" if italic and weight != 400 else ("Italic" if italic else sub)
    name = font["name"]
    for rec in list(name.names):
        if rec.nameID in (1, 2, 4, 6, 16, 17):
            name.removeNames(nameID=rec.nameID)
    name.setName(family, 1, 3, 1, 0x409)
    name.setName(sub, 2, 3, 1, 0x409)
    name.setName(f"{family} {sub}", 4, 3, 1, 0x409)
    name.setName(f"{family.replace(' ', '')}-{sub.replace(' ', '')}", 6, 3, 1, 0x409)
    return font


def build():
    """{file name: TTFont} for every face the drawing uses."""
    out = {}
    for family, style, weights, files in faces():
        wanted = [w for w in WEIGHTS if weights[0] <= w <= weights[-1]] if len(weights) == 2 else list(weights)
        for weight in wanted:
            if style == "italic" and weight != 400:
                continue
            face = merged(family, style, weight, files)
            file = f"{family.replace(' ', '')}-{weight}{'-italic' if style == 'italic' else ''}.ttf"
            out[file] = face
    return out


def fingerprint(font):
    """What decides how text looks: names, weight, style, mapped characters and their advances."""
    cmap = font.getBestCmap()
    hmtx = font["hmtx"].metrics
    return {
        "family": font["name"].getDebugName(1),
        "subfamily": font["name"].getDebugName(2),
        "weight": font["OS/2"].usWeightClass,
        "italic": bool(font["OS/2"].fsSelection & 1),
        "unitsPerEm": font["head"].unitsPerEm,
        "advances": {str(c): hmtx[g][0] for c, g in sorted(cmap.items())},
    }


def main():
    faces_built = build()
    if "--check" in sys.argv[1:]:
        problems = []
        on_disk = {p.name for p in OUT.glob("*.ttf")} if OUT.is_dir() else set()
        for extra in sorted(on_disk - set(faces_built)):
            problems.append(f"{extra}: betiğin yazmadığı dosya")
        for file, font in faces_built.items():
            path = OUT / file
            if not path.is_file():
                problems.append(f"{file}: diskte yok (betiği --check olmadan çalıştırın)")
                continue
            want = fingerprint(TTFont(io.BytesIO(ttf_bytes(font))))
            have = fingerprint(TTFont(path))
            if want != have:
                problems.append(f"{file}: diskteki yazı tipi web'in dosyalarından yeniden üretilenle aynı değil")
        for p in problems:
            print(p, file=sys.stderr)
        if problems:
            return 1
        print(f"Çizim yazı tipleri güncel: {len(faces_built)} yüz, {len(FAMILIES)} aile.")
        return 0
    OUT.mkdir(parents=True, exist_ok=True)
    for old in OUT.glob("*.ttf"):
        old.unlink()
    for file, font in faces_built.items():
        (OUT / file).write_bytes(ttf_bytes(font))
    for family, folder in FAMILIES.items():
        licence = WEB / folder / "OFL.txt"
        (OUT / f"{family.replace(' ', '')}-OFL.txt").write_text(licence.read_text())
    summary = {f: {k: v for k, v in fingerprint(TTFont(OUT / f)).items() if k != "advances"} for f in sorted(faces_built)}
    print(json.dumps(summary, ensure_ascii=False, indent=1))
    print(f"{len(faces_built)} yüz yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.dont_write_bytecode = True
    sys.exit(main())
