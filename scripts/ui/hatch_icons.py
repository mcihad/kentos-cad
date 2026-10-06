"""The hatch patterns' icons (docs/adr/0186 §2, §11): each pattern drawn by
itself in the icon set's 20 × 20 square, from the library's own definitions
(fixtures/hatch/v1/cases.json, the ADR's numbers), its lines and dashes cut
to the square and its dots as small discs; Çizgili and Çapraz at their 3 mm,
Dolu filled as `fillSolid` is.

    python3 scripts/ui/hatch_icons.py           # writes the file
    python3 scripts/ui/hatch_icons.py --check   # writes nothing; compares

Writes apps/web/src/ui/hatchIcons.ts (`HATCH_ICONS`, joined into the icon
set; the desktop draws them from the inventory as it draws every icon). A
pattern is drawn at its group's size on the paper (px per mm) so that a few
of its repeats show; its anchor (the world's origin) at the square's middle.

`HATCH_PREVIEWS` are Desen's menus' wide samples (56 × 24 px, the owner's
choice): the same drawing in a 46.67 × 20 box of the icon grid's units (a
unit 1.2 px), so that both platforms draw them as they draw icons; the
gradients in the bands of their icons.
"""
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))
import hatch_pattern_cases as rule  # noqa: E402  (the pattern rule, written from the ADR)

CASES = ROOT / "fixtures/hatch/v1/cases.json"
OUT = ROOT / "apps/web/src/ui/hatchIcons.ts"

# The square the pattern fills, in icon pixels (the frame's inside).
LO, HI = 4.0, 16.0
# Pixels a paper millimetre takes, by group, and the patterns that want their own.
SCALE = {"ansi": 0.85, "iso": 0.45, "general": 0.85}
OWN = {"DOTS": 2.4, "ISO07W100": 0.9, "BRICK": 0.5, "CROSS": 0.65, "NET3": 1.2}
FRAME = '<rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2"/>'


def num(v):
    s = f"{v:.2f}".rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def icon_of(pattern, s):
    """The pattern's pieces in the square at s px per mm (y up on paper, down in the icon)."""
    half = (HI - LO) / 2 / s
    ring = [[-half, -half], [half, -half], [half, half], [-half, half]]
    p = dict(pattern)
    if p["type"] == "pattern":
        p["scale"] = 1.0
    cut = rule.pieces(ring, [], p, 100000)
    at = lambda q: (10 + q[0] * s, 10 - q[1] * s)
    d = []
    for a, b in cut["segments"]:
        (x0, y0), (x1, y1) = at(a), at(b)
        d.append(f"M{num(x0)} {num(y0)}L{num(x1)} {num(y1)}")
    out = FRAME
    if d:
        out += f'<path d="{"".join(d)}" stroke-width="1"/>'
    for q in cut["dots"]:
        x, y = at(q)
        out += f'<circle cx="{num(x)}" cy="{num(y)}" r=".75" fill="currentColor" stroke="none"/>'
    return out


# A wide sample's box in icon units (56 × 24 px at 1.2 px a unit) and its frame.
PW, PH = 56 / 1.2, 20.0
PFRAME = f'<rect x="0.5" y="0.5" width="{PW - 1:.2f}" height="19" rx="1.5" stroke-width="1"/>'


def preview_of(pattern, s):
    """The pattern's pieces in the wide box at s units per paper mm, its anchor at the middle."""
    hw, hh = (PW - 2) / 2 / s, (PH - 2) / 2 / s
    ring = [[-hw, -hh], [hw, -hh], [hw, hh], [-hw, hh]]
    p = dict(pattern)
    if p["type"] == "pattern":
        p["scale"] = 1.0
    cut = rule.pieces(ring, [], p, 100000)
    at = lambda q: (PW / 2 + q[0] * s, PH / 2 - q[1] * s)
    d = []
    for a, b in cut["segments"]:
        (x0, y0), (x1, y1) = at(a), at(b)
        d.append(f"M{num(x0)} {num(y0)}L{num(x1)} {num(y1)}")
    out = PFRAME
    if d:
        out += f'<path d="{"".join(d)}" stroke-width=".85"/>'
    for q in cut["dots"]:
        x, y = at(q)
        out += f'<circle cx="{num(x)}" cy="{num(y)}" r=".7" fill="currentColor" stroke="none"/>'
    return out


def bands(opacities):
    """Upright bands across the wide box, each its opacity (a gradient's look, as its icon)."""
    w = (PW - 1) / len(opacities)
    out = ""
    for k, o in enumerate(opacities):
        out += f'<rect x="{num(0.5 + k * w)}" y="0.5" width="{num(w + 0.05)}" height="19" fill="currentColor" fill-opacity="{num(o)}" stroke="none"/>'
    return out + PFRAME


def previews():
    library = json.loads(CASES.read_text("utf-8"))["library"]
    # A unit per paper mm a little under the icon's px: the wide box shows more repeats.
    k = 0.96
    out = {
        "hatchPreviewLines": preview_of({"type": "lines", "angle": 45.0, "spacing": 3.0}, 0.85 * k),
        "hatchPreviewCross": preview_of({"type": "cross", "angle": 45.0, "spacing": 3.0}, 0.85 * k),
        "hatchPreviewSolid": PFRAME.replace('/>', ' fill="currentColor" fill-opacity=".35"/>'),
    }
    for p in library:
        s = OWN.get(p["name"], SCALE[p["group"]]) * k
        out[f"hatchPreview{p['name']}"] = preview_of({"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": p["name"], "lines": p["lines"]}, s)
    n = 14
    out["hatchPreviewGradientLinear"] = bands([0.04 + 0.6 * i / (n - 1) for i in range(n)])
    out["hatchPreviewGradientCylinder"] = bands([0.06 + 0.56 * (1 - abs(2 * i / (n - 1) - 1)) for i in range(n)])
    rings = ""
    for r, o in ((1.0, 0.1), (0.74, 0.22), (0.5, 0.38), (0.26, 0.58)):
        rings += f'<ellipse cx="{num(PW / 2)}" cy="10" rx="{num((PW / 2 - 0.5) * r)}" ry="{num(9.5 * r)}" fill="currentColor" fill-opacity="{num(o)}" stroke="none"/>'
    out["hatchPreviewGradientSpherical"] = rings + PFRAME
    return out


def build():
    library = json.loads(CASES.read_text("utf-8"))["library"]
    icons = {
        "hatchSwatchLines": icon_of({"type": "lines", "angle": 45.0, "spacing": 3.0}, 0.85),
        "hatchSwatchCross": icon_of({"type": "cross", "angle": 45.0, "spacing": 3.0}, 0.85),
        "hatchSwatchSolid": '<rect x="3.5" y="3.5" width="13" height="13" rx="1" stroke-width="1.2" fill="currentColor" fill-opacity=".35"/>',
    }
    for p in library:
        s = OWN.get(p["name"], SCALE[p["group"]])
        icons[f"hatchSwatch{p['name']}"] = icon_of({"type": "pattern", "angle": 0.0, "spacing": 1.0, "name": p["name"], "lines": p["lines"]}, s)
    return icons


def entries(icons):
    return [f"  {k}: {json.dumps(v, ensure_ascii=False)},".replace('\\"', '"').replace(': "', ": '").replace('",', "',") for k, v in icons.items()]


def text_of(icons, wide):
    lines = [
        "/**",
        " * The hatch patterns' icons (docs/adr/0186 §2): each pattern drawn by itself, its lines cut to the square. Written by",
        " * scripts/ui/hatch_icons.py from the library's definitions (fixtures/hatch/v1/cases.json); do not edit by hand.",
        " */",
        "export const HATCH_ICONS = {",
        *entries(icons),
        "};",
        "",
        "/**",
        " * Desen's menus' wide samples (56 × 24 px; the owner's choice): the same drawings in a 46.67 × 20 box of the icon",
        " * grid's units, the gradients in their icons' bands. Drawn by `iconPreview` (icons.ts) and the desktop's menus.",
        " */",
        "export const HATCH_PREVIEWS = {",
        *entries(wide),
        "};",
    ]
    return "\n".join(lines) + "\n"


def main():
    text = text_of(build(), previews())
    if "--check" in sys.argv:
        old = OUT.read_text("utf-8") if OUT.exists() else ""
        if old != text:
            print(f"{OUT} güncel değil: python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(build())} ikon, {len(previews())} geniş örnek")
    return 0


if __name__ == "__main__":
    sys.exit(main())
