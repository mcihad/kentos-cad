"""The desktop's interface typefaces the web offers and KentOS UI did not
carry, made from the web's own font files (TODOS.md UX-13; the web's
`appearance.uiFont`: Noto Sans, Roboto).

Source Sans 3 is not made here: its licence reserves the name “Source”, so
a changed font (merged, instanced) may not carry it. KentOS UI carries
Adobe's own static files unchanged (crates/ui/assets/fonts/README.md).

The web loads them from the WOFF2 files in apps/web/src/assets/fonts
(apps/web/src/styles/fonts.css), variable fonts in two subsets each (latin,
latin-ext). Iced reads TrueType, not WOFF2, so this writes the two faces
KentOS UI draws with, as its other families have them (Regular 400 and
SemiBold 600): the two subsets merged, the variable font instanced at the
weight. The glyphs and their advances are the web's; nothing is taken from
elsewhere. It works as scripts/fonts/drawing_fonts.py does (docs/adr/0055).

    python3 scripts/fonts/ui_fonts.py           # writes crates/ui/assets/fonts
    python3 scripts/fonts/ui_fonts.py --check   # writes nothing; compares what it would write
    python3 scripts/fonts/ui_fonts.py --advance # the families' average advance (em), as typography.rs keeps it

Needs fontTools with brotli (WOFF2).
"""
import io
import re
import sys
from pathlib import Path

from fontTools.merge import Merger
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

ROOT = Path(__file__).resolve().parents[2]
WEB = ROOT / "apps/web/src/assets/fonts"
CSS = ROOT / "apps/web/src/styles/fonts.css"
OUT = ROOT / "crates/ui/assets/fonts"

# The web's families KentOS UI did not carry, and their folders (apps/web/src/app/appearance.ts).
FAMILIES = {
    "Noto Sans": "noto-sans",
    "Roboto": "roboto",
}
# KentOS UI draws the interface in two weights (typography.rs `ui`, `ui_strong`).
WEIGHTS = {400: "Regular", 600: "SemiBold"}
# A fixed date in every file, so a run changes a file only when its glyphs do.
DATE = 3_841_516_800  # 2025-09-26 in seconds since 1904

# Turkish interface text the average advance is measured on: the ribbon's,
# the panels' and the windows' words (typography.rs `Family::advance`).
SAMPLE = (
    "Dosya Giriş Çizim Değiştir Harita Görünüm İşlemler Araçlar Katmanlar Öznitelikler "
    "Kapalı alan Çizgi Çoklu çizgi Daire Yay Dikdörtgen Taşı Kopyala Döndür Ölçekle "
    "Aynala Ötele Buda Uzat Köşe yuvarla Seçim Pano Açıklama Özellikler Kenetleme "
    "Proje ayarları Uygulama ayarları Koordinat sistemi Birimler ve hassasiyet Kaydet "
    "Vazgeç Bu bölümü varsayılana döndür Yeni katman Nesneyi seç Komut ya da koordinat yazın"
)


def files_of(family):
    """The family's normal-style files, the latin-ext subset first (merged later, latin's glyphs win)."""
    css = CSS.read_text()
    found = []
    for block in re.findall(r"@font-face\s*{([^}]*)}", css):
        if re.search(r"font-family:\s*'([^']+)'", block).group(1) != family:
            continue
        if re.search(r"font-style:\s*(\w+)", block).group(1) != "normal":
            continue
        found.append(re.search(r"url\('([^']+)'\)", block).group(1).split("/")[-1])
    return sorted(found, key=lambda n: "latin-ext" not in n)


def ttf_bytes(font):
    buf = io.BytesIO()
    font.flavor = None
    font.save(buf, reorderTables=True)
    return buf.getvalue()


def face(family, weight):
    """One face from the latin-ext and latin subsets, at `weight`."""
    parts = []
    for name in files_of(family):
        font = TTFont(WEB / FAMILIES[family] / name)
        if "fvar" in font:
            font = instantiateVariableFont(font, {"wght": weight})
        parts.append(io.BytesIO(ttf_bytes(font)))
    font = TTFont(parts[0]) if len(parts) == 1 else Merger().merge(parts)
    font["head"].created = DATE
    font["head"].modified = DATE
    font["OS/2"].usWeightClass = weight
    fs = font["OS/2"].fsSelection & ~0b1100001
    font["OS/2"].fsSelection = fs | (0b1000000 if weight == 400 else 0)
    font["head"].macStyle = 0
    sub = WEIGHTS[weight]
    name = font["name"]
    for rec in list(name.names):
        if rec.nameID in (1, 2, 4, 6, 16, 17):
            name.removeNames(nameID=rec.nameID)
    name.setName(family, 1, 3, 1, 0x409)
    name.setName(sub, 2, 3, 1, 0x409)
    name.setName(f"{family} {sub}", 4, 3, 1, 0x409)
    name.setName(f"{family.replace(' ', '')}-{sub}", 6, 3, 1, 0x409)
    return font


def build():
    """{file name: TTFont} for every face."""
    return {
        f"{family.replace(' ', '')}-{sub}.ttf": face(family, weight)
        for family in FAMILIES
        for weight, sub in WEIGHTS.items()
    }


def fingerprint(font):
    """What decides how text looks: names, weight, mapped characters and their advances."""
    cmap = font.getBestCmap()
    hmtx = font["hmtx"].metrics
    return {
        "family": font["name"].getDebugName(1),
        "subfamily": font["name"].getDebugName(2),
        "weight": font["OS/2"].usWeightClass,
        "unitsPerEm": font["head"].unitsPerEm,
        "advances": {str(c): hmtx[g][0] for c, g in sorted(cmap.items())},
    }


def advance(font):
    """The average advance of SAMPLE's characters, in em."""
    cmap = font.getBestCmap()
    hmtx = font["hmtx"].metrics
    em = font["head"].unitsPerEm
    widths = [hmtx[cmap[ord(c)]][0] / em for c in SAMPLE if ord(c) in cmap]
    return sum(widths) / len(widths)


def main():
    args = sys.argv[1:]
    if "--advance" in args:
        # Every family KentOS UI has: the ones it carried, and these.
        files = sorted(p.name for p in OUT.glob("*-Regular.ttf") if "Mono" not in p.name)
        for regular in files:
            semibold = regular.replace("-Regular", "-SemiBold")
            a = advance(TTFont(OUT / regular))
            b = advance(TTFont(OUT / semibold)) if (OUT / semibold).is_file() else float("nan")
            print(f"{regular.removesuffix('-Regular.ttf'):<18} ({a:.3f}, {b:.3f})")
        return 0
    faces = build()
    if "--check" in args:
        problems = []
        for file, font in faces.items():
            path = OUT / file
            if not path.is_file():
                problems.append(f"{file}: diskte yok (betiği --check olmadan çalıştırın)")
                continue
            if fingerprint(TTFont(io.BytesIO(ttf_bytes(font)))) != fingerprint(TTFont(path)):
                problems.append(f"{file}: diskteki yazı tipi web'in dosyalarından yeniden üretilenle aynı değil")
        for p in problems:
            print(p, file=sys.stderr)
        if problems:
            return 1
        print(f"Arayüz yazı tipleri güncel: {len(faces)} yüz, {len(FAMILIES)} aile.")
        return 0
    for file, font in faces.items():
        (OUT / file).write_bytes(ttf_bytes(font))
    for family, folder in FAMILIES.items():
        (OUT / f"OFL-{family.replace(' ', '')}.txt").write_text((WEB / folder / "OFL.txt").read_text())
    print(f"{len(faces)} yüz yazıldı: {OUT.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.dont_write_bytecode = True
    sys.exit(main())
