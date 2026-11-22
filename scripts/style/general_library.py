"""KentOS's general symbol library (docs/adr/9118): point, line and area symbols for every kind of map, by theme, written
as crates/native/style/assets/general-library.kstil, the format the desktop's style files read. The desktop ships it in
the system library beside the basic symbols and MPYY's (crates/native/style/src/system.rs): read-only, copyable.

    python3 scripts/style/general_library.py            # writes the file
    python3 scripts/style/general_library.py --check    # fails when the file is not what this script writes
    python3 scripts/style/general_library.py --svg out.json   # the pictograms by name, for a contact sheet

Conventions (docs/adr/9118 §2):
- sizes are paper millimetres at the plot scale, as in the rest of the library; `ink` and `paper` are the theme's
  drawing and background colours (black and white on paper);
- a place symbol is a pictogram on its theme's coloured disc with a white rim, so it reads on any background;
  utilities, survey marks and nature symbols are drawn in ink or their own colour, without a disc;
- pictograms are drawn here (viewBox 0 0 100 100, about 6 units of margin, `currentColor` taking the layer's colour,
  holes by `fill-rule="evenodd"`), bold enough to read at 3 mm;
- the basic shapes take the colour-blind-safe eight-colour palette of Okabe and Ito (2008).
"""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates/native/style/assets/general-library.kstil"

INK, PAPER, WHITE = "ink", "paper", "#FFFFFF"
GEN = "Genel"
POINTS, LINES, AREAS = "Noktalar", "Çizgiler", "Alanlar"

# Theme colours of the place discs.
TRANSPORT = "#1F6FB2"
HEALTH = "#D32F2F"
EDUCATION = "#D9731A"
CULTURE = "#7B3FA0"
PUBLIC = "#455A64"
WORSHIP = "#5D6B76"
COMMERCE = "#C2185B"
TOURISM = "#00897B"
SPORT = "#2E7D32"
WARNING = "#F2B705"

# Okabe & Ito's colour-blind-safe palette.
OKABE_ITO = ["#E69F00", "#56B4E9", "#009E73", "#F0E442", "#0072B2", "#D55E00", "#CC79A7", "#000000"]

# Map colours.
WATER_LINE, WATER_FILL, WATER_DARK = "#2B7BB9", "#A9D3F2", "#1F5F99"
FOREST, FOREST_FILL = "#3E8E41", "#D3EBC6"
EARTH = "#8D6E4A"
CONTOUR = "#A0673A"
ELECTRIC, GAS, DRINKING, SEWER, STORM, TELECOM, FUEL = (
    "#D32F2F", "#E69F00", "#1E88E5", "#795548", "#00ACC1", "#43A047", "#424242")


# ── Pictograms ──────────────────────────────────────────────────────────────────────────────────────────────────────


def svg(body: str) -> str:
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">{body}</svg>'


def path(d: str, rule: str | None = None) -> str:
    extra = f' fill-rule="{rule}"' if rule else ""
    return f'<path fill="currentColor"{extra} d="{d}"/>'


def stroke(d: str, width: float, cap: str = "round", join: str = "round") -> str:
    return (f'<path fill="none" stroke="currentColor" stroke-width="{width:g}" stroke-linecap="{cap}" '
            f'stroke-linejoin="{join}" d="{d}"/>')


def circle(cx: float, cy: float, r: float) -> str:
    return f'<circle cx="{cx:g}" cy="{cy:g}" r="{r:g}" fill="currentColor"/>'


def ring(cx: float, cy: float, r: float, width: float) -> str:
    return f'<circle cx="{cx:g}" cy="{cy:g}" r="{r:g}" fill="none" stroke="currentColor" stroke-width="{width:g}"/>'


def ellipse(cx: float, cy: float, rx: float, ry: float, turn: float = 0.0) -> str:
    t = f' transform="rotate({turn:g} {cx:g} {cy:g})"' if turn else ""
    return f'<ellipse cx="{cx:g}" cy="{cy:g}" rx="{rx:g}" ry="{ry:g}" fill="currentColor"{t}/>'


def disc(cx: float, cy: float, r: float) -> str:
    """A full circle as path data, for holes in an even-odd path."""
    return f"M{cx:g} {cy - r:g}A{r:g} {r:g} 0 1 0 {cx:g} {cy + r:g}A{r:g} {r:g} 0 1 0 {cx:g} {cy - r:g}Z"


def star_points(cx: float, cy: float, outer: float, inner: float, n: int = 5, turn: float = -90.0) -> str:
    pts = []
    for k in range(2 * n):
        r = outer if k % 2 == 0 else inner
        a = math.radians(turn + k * 180.0 / n)
        pts.append(f"{cx + r * math.cos(a):.1f} {cy + r * math.sin(a):.1f}")
    return "M" + "L".join(pts) + "Z"


def polygon_points(cx: float, cy: float, r: float, n: int, turn: float = -90.0) -> list[tuple[float, float]]:
    return [(cx + r * math.cos(math.radians(turn + k * 360.0 / n)), cy + r * math.sin(math.radians(turn + k * 360.0 / n)))
            for k in range(n)]


def wave(y: float, x0: float = 8, x1: float = 92, step: float = 20, amp: float = 6) -> str:
    """A wavy stroke across: half periods of `step` up and down."""
    d, x, up = f"M{x0:g} {y:g}", x0, True
    while x < x1 - 0.01:
        nx = min(x + step, x1)
        h = -amp if up else amp
        d += f"C{x + step / 3:.1f} {y + h:.1f} {nx - step / 3:.1f} {y + h:.1f} {nx:.1f} {y:g}"
        x, up = nx, not up
    return d


def ball() -> str:
    """A football: the rim, a black pentagon in the middle, five black patches on the rim, seams between."""
    pent = polygon_points(50, 50, 14, 5)
    pent_d = "M" + "L".join(f"{x:.1f} {y:.1f}" for x, y in pent) + "Z"
    patches, seams = "", ""
    for k in range(5):
        a = math.radians(-90 + k * 72)
        inner = (50 + 29 * math.cos(a), 50 + 29 * math.sin(a))
        left = (50 + 42 * math.cos(a - math.radians(20)), 50 + 42 * math.sin(a - math.radians(20)))
        right = (50 + 42 * math.cos(a + math.radians(20)), 50 + 42 * math.sin(a + math.radians(20)))
        patches += (f"M{inner[0]:.1f} {inner[1]:.1f}L{left[0]:.1f} {left[1]:.1f}"
                    f"A42 42 0 0 1 {right[0]:.1f} {right[1]:.1f}Z")
        seams += f"M{pent[k][0]:.1f} {pent[k][1]:.1f}L{inner[0]:.1f} {inner[1]:.1f}"
    return ring(50, 50, 42, 6) + path(pent_d + patches) + stroke(seams, 5)


PICTOGRAMS: dict[str, str] = {
    # Transport
    "otobus": svg(path("M34 10H66A12 12 0 0 1 78 22V72A6 6 0 0 1 72 78H28A6 6 0 0 1 22 72V22A12 12 0 0 1 34 10Z"
                       "M33 22A3 3 0 0 0 30 25V44A3 3 0 0 0 33 47H67A3 3 0 0 0 70 44V25A3 3 0 0 0 67 22Z"
                       + disc(34, 62, 5) + disc(66, 62, 5), "evenodd")
                  + path("M27 76H40V86A4 4 0 0 1 36 90H31A4 4 0 0 1 27 86ZM60 76H73V86A4 4 0 0 1 69 90H64A4 4 0 0 1 60 86Z")),
    "tren": svg(path("M36 8H64A14 14 0 0 1 78 22V64A10 10 0 0 1 68 74H32A10 10 0 0 1 22 64V22A14 14 0 0 1 36 8Z"
                     "M34 18A4 4 0 0 0 30 22V38A4 4 0 0 0 34 42H66A4 4 0 0 0 70 38V22A4 4 0 0 0 66 18Z"
                     + disc(36, 57, 5) + disc(64, 57, 5), "evenodd")
                + stroke("M36 78L24 94M64 78L76 94", 7)),
    "tramvay": svg(path("M30 28H70A8 8 0 0 1 78 36V74A8 8 0 0 1 70 82H30A8 8 0 0 1 22 74V36A8 8 0 0 1 30 28Z"
                        "M31 36V54H69V36Z" + disc(36, 67, 5) + disc(64, 67, 5), "evenodd")
                   + stroke("M50 28L40 16L60 7M36 82L30 94M64 82L70 94", 6)),
    "ucak": svg(path("M50 4C54.5 4 57 8.5 57 15V37L92 57V66L57 55V75L68 83V90L50 85L32 90V83L43 75V55L8 66V57L43 37V15"
                     "C43 8.5 45.5 4 50 4Z")),
    "capa": svg(f'<g fill="none" stroke="currentColor" stroke-width="8" stroke-linecap="round" stroke-linejoin="round">'
                f'<circle cx="50" cy="15" r="8"/><path d="M50 23V90M33 36H67M17 60C19 78 33 90 50 90C67 90 81 78 83 60"/></g>'
                + path("M8 64L17 48L26 64ZM74 64L83 48L92 64Z")),
    "akaryakit": svg(path("M24 8H54A6 6 0 0 1 60 14V84H66V94H12V84H18V14A6 6 0 0 1 24 8ZM27 18V42H51V18Z", "evenodd")
                     + stroke("M60 46H68A5 5 0 0 1 73 51V74A6 6 0 0 0 85 74V36L75 26", 6)),
    "bisiklet": svg(ring(22, 66, 16, 7) + ring(78, 66, 16, 7)
                    + stroke("M22 66H48L40 36H68L48 66M22 66L40 36M68 36L78 66M68 36L65 26M61 26H74M40 36L39 30M32 30H46", 7)),
    "simsek": svg(path("M58 4L20 56H46L38 96L80 40H54Z")),
    "otomobil": svg(path("M38 16H62A8 8 0 0 1 70 21L78 44A8 8 0 0 1 86 52V72A4 4 0 0 1 82 76H18A4 4 0 0 1 14 72V52"
                         "A8 8 0 0 1 22 44L30 21A8 8 0 0 1 38 16ZM33 44H67L61 26H39Z" + disc(27, 61, 5) + disc(73, 61, 5),
                         "evenodd")
                    + path("M18 76H32V86A4 4 0 0 1 28 90H22A4 4 0 0 1 18 86ZM68 76H82V86A4 4 0 0 1 78 90H72A4 4 0 0 1 68 86Z")),
    # Health
    "arti": svg(path("M39 12H61V39H88V61H61V88H39V61H12V39H39Z")),
    "kapsul": svg('<g transform="rotate(-45 50 50)">'
                  + path("M30 32H70A18 18 0 0 1 70 68H30A18 18 0 0 1 30 32ZM50 39V61H70A11 11 0 0 0 70 39Z", "evenodd")
                  + "</g>"),
    "pati": svg(path("M50 50C62 50 76 62 76 74C76 84 68 88 60 86C55 85 53 83 50 83C47 83 45 85 40 86C32 88 24 84 24 74"
                     "C24 62 38 50 50 50Z")
                + ellipse(21, 45, 8, 10, -22) + ellipse(37, 26, 8, 11, -8) + ellipse(63, 26, 8, 11, 8)
                + ellipse(79, 45, 8, 10, 22)),
    # Education and culture
    "okul": svg(path("M50 20L88 42V48H12V42Z") + path("M18 52H82V90H58V70H42V90H18Z")
                + stroke("M50 20V4", 5) + path("M52 4H68L64 9L68 14H52Z")),
    "kep": svg(path("M50 16L96 38L50 60L4 38Z") + path("M24 50V68C24 80 76 80 76 68V50L50 63Z")
               + stroke("M88 42V70", 5) + circle(88, 74, 5)),
    "kitap": svg(path("M46 28C38 20 24 18 8 20V80C24 78 38 80 46 88Z") + path("M54 28C62 20 76 18 92 20V80C76 78 62 80 54 88Z")),
    "muze": svg(path("M50 6L94 28V36H6V28Z") + path("M10 84H90V94H10Z")
                + path("M16 42H27V78H16ZM36 42H47V78H36ZM53 42H64V78H53ZM73 42H84V78H73Z")),
    "nota": svg(path("M44 20L84 10V20L50 28V74H44ZM78 12H84V66H78Z") + ellipse(37, 75, 13, 10, -20)
                + ellipse(71, 67, 13, 10, -20)),
    # Public services
    "kamu-binasi": svg(path("M28 40C28 26 38 18 50 18C62 18 72 26 72 40Z")
                       + path("M14 44H86V52H14ZM18 56H28V82H18ZM38 56H48V82H38ZM52 56H62V82H52ZM72 56H82V82H72Z"
                              "M10 86H90V94H10Z")
                       + stroke("M50 18V4", 4) + path("M52 4H66V12H52Z")),
    "kalkan": svg(path("M50 6L84 18V44C84 66 70 82 50 94C30 82 16 66 16 44V18Z" + star_points(50, 47, 20, 8), "evenodd")),
    "alev": svg(path("M50 4C56 22 78 32 78 60C78 80 66 94 50 94C34 94 22 80 22 60C22 46 30 38 36 28C38 40 42 46 48 48"
                     "C46 34 44 20 50 4ZM50 60C44 67 40 73 40 79C40 86 45 90 50 90C55 90 60 86 60 79C60 73 56 67 50 60Z",
                     "evenodd")),
    "zarf": svg('<g fill="none" stroke="currentColor" stroke-width="8" stroke-linejoin="round">'
                '<rect x="12" y="24" width="76" height="52" rx="4"/><path d="M14 28L50 54L86 28"/></g>'),
    "terazi": svg(stroke("M50 12V84M30 86H70M20 24H80M20 24L8 54M20 24L32 54M80 24L68 54M80 24L92 54", 6)
                  + path("M5 54H35A15 11 0 0 1 5 54ZM65 54H95A15 11 0 0 1 65 54Z")),
    # Worship
    "cami": svg(path("M10 90V60H16C16 44 28 32 42 28C56 32 68 44 68 60H74V90ZM34 90V76A8 8 0 0 1 50 76V90Z", "evenodd")
                + stroke("M42 28V19", 3) + circle(42, 15, 3.5)
                + path("M80 90V36L85 18L90 36V90ZM76 46H94V51H76Z")),
    "kilise": svg(stroke("M50 4V26M41 13H59", 6, cap="butt")
                  + path("M50 26L82 48V92H18V48ZM42 92V72A8 8 0 0 1 58 72V92Z", "evenodd")),
    "alti-koseli-yildiz": svg('<g fill="none" stroke="currentColor" stroke-width="7" stroke-linejoin="round">'
                              '<path d="M50 10L88 74H12Z"/><path d="M50 90L12 26H88Z"/></g>'),
    "mezar": svg(path("M30 84V38C30 22 70 22 70 38V84ZM40 44H60V50H40ZM40 56H60V62H40Z", "evenodd")
                 + path("M20 84H80V94H20Z")),
    # Commerce
    "canta": svg(path("M20 34H80L86 92H14Z") + stroke("M36 42V26A14 14 0 0 1 64 26V42", 7)),
    "sepet": svg(stroke("M8 18H22L32 62H78L88 30H26", 7) + circle(38, 78, 7) + circle(72, 78, 7)),
    "banknot": svg(path("M6 26H94V74H6Z" + disc(50, 50, 13) + "M16 36H24V64H16ZM76 36H84V64H76Z", "evenodd")),
    "catal-bicak": svg(stroke("M24 8V30C24 38 30 42 36 42C42 42 48 38 48 30V8M36 8V92", 6)
                       + path("M64 8C80 14 84 36 76 52H71V92H64Z")),
    "fincan": svg(path("M14 34H70V58C70 74 58 84 42 84C26 84 14 74 14 58Z")
                  + stroke("M70 42H76A10 10 0 0 1 76 62H68M10 92H74", 6)
                  + stroke("M30 8C26 14 34 18 30 26M46 8C42 14 50 18 46 26", 4)),
    # Tourism
    "yatak": svg(path("M8 24H16V60H90V86H82V76H16V86H8Z") + path("M44 42H82A8 8 0 0 1 90 50V58H44Z") + circle(30, 48, 9)),
    "durbun": svg(path(disc(28, 64, 22) + disc(28, 64, 11) + disc(72, 64, 22) + disc(72, 64, 11), "evenodd")
                  + path("M14 18H34L40 46H8ZM66 18H86L92 46H60ZM42 46H58V62H42Z")),
    "cadir": svg(path("M50 16L92 88H8ZM50 50L36 88H64Z", "evenodd") + stroke("M50 16L40 4M50 16L60 4", 5)),
    "piknik": svg(path("M14 32H86V41H14ZM6 58H94V65H6Z") + stroke("M34 41L22 90M66 41L78 90", 6)),
    "dikilitas": svg(path("M41 18L50 4L59 18V76H41ZM28 80H72V92H28Z")),
    "kale": svg(path("M22 92V24H31V14H40V24H45V14H55V24H60V14H69V24H78V92ZM42 92V74A8 8 0 0 1 58 74V92ZM46 40H54V56H46Z",
                     "evenodd")),
    # Sport and recreation
    "agac": svg(ellipse(50, 38, 32, 30) + path("M44 60H56V92H44Z")),
    "konifer": svg(path("M50 4L74 38H62L82 64H58V92H42V64H18L38 38H26Z")),
    "salincak": svg(stroke("M10 90L26 14H74L90 90M26 14L42 90M74 14L58 90M40 14V58M60 14V58", 6)
                    + path("M34 58H66V66H34Z")),
    "top": svg(ball()),
    "yuzucu": svg(circle(72, 28, 9) + stroke("M16 52L40 36L56 50", 7)
                  + stroke(wave(70, 8, 92, 20, 6) + wave(86, 8, 92, 20, 6), 6)),
    "dambil": svg(path("M14 34H26V66H14ZM74 34H86V66H74ZM26 46H74V54H26ZM6 42H14V58H6ZM86 42H94V58H86Z")),
    # Nature
    "damla": svg(path("M50 8C62 28 76 44 76 62A26 26 0 0 1 24 62C24 44 38 28 50 8Z")),
    "magara": svg(path("M8 90C8 52 26 26 50 26C74 26 92 52 92 90H66C66 70 60 58 50 58C40 58 34 70 34 90Z")),
    "kaya": svg(stroke("M12 80L20 54L38 40L60 34L80 46L88 70L78 80Z", 7) + stroke("M38 40L46 58L60 34M46 58L60 66", 6)),
    "ot": svg(stroke("M20 90H80M30 84L24 50M50 84V34M70 84L76 50", 9)),
    "cali": svg(path(disc(30, 58, 20) + disc(52, 44, 24) + disc(72, 60, 18)) + path("M47 70H55V92H47Z")),
    "dalga": svg(stroke(wave(40, 6, 94, 22, 9) + wave(66, 6, 94, 22, 9), 8)),
    # Warning and general
    "insanlar": svg(circle(50, 20, 11) + circle(20, 36, 9) + circle(80, 36, 9)
                    + path("M38 36H62A7 7 0 0 1 69 43V70H59V93H41V70H31V43A7 7 0 0 1 38 36Z")
                    + path("M10 50H28V78H23V94H15V78H10Z") + path("M72 50H90V78H85V94H77V78H72Z")),
    "pin": svg(path("M50 6C32 6 18 20 18 38C18 62 50 94 50 94C50 94 82 62 82 38C82 20 68 6 50 6Z" + disc(50, 38, 12),
                    "evenodd")),
    "bayrak": svg(stroke("M24 92V8", 6) + path("M27 10H80L68 28L80 46H27Z")),
    # Utilities
    "anten": svg(path("M47 36H53L66 94H56L50 62L44 94H34Z") + circle(50, 30, 7)
                 + stroke("M36 18A20 20 0 0 0 36 42M64 18A20 20 0 0 1 64 42M26 9A33 33 0 0 0 26 51M74 9A33 33 0 0 1 74 51", 5)),
    "vana": svg(path("M10 34L50 54L10 74ZM90 34L50 54L90 74Z") + stroke("M50 54V24M36 22H64", 6)),
    "hidrant": svg(path("M36 30C36 18 64 18 64 30V34H72V42H64V80H70V90H30V80H36V42H28V34H36ZM46 8H54V18H46Z")
                   + path("M64 52H80V64H64ZM20 52H36V64H20Z")),
    "depo": svg(path("M22 28A28 10 0 0 1 78 28V74A28 10 0 0 1 22 74Z") + stroke("M22 40A28 10 0 0 0 78 40", 4)),
}


# ── Library items ───────────────────────────────────────────────────────────────────────────────────────────────────

CATEGORIES: list[dict] = []
ITEMS: list[dict] = []


def category(path_: list[str], order: int, description: str | None = None) -> None:
    c: dict = {"path": path_, "order": order}
    if description:
        c["description"] = description
    CATEGORIES.append(c)


def numbered(layers: list[dict]) -> list[dict]:
    """The layers with ids 0, 1, … (a marker inside a layer counts its own)."""
    out = []
    for k, layer in enumerate(layers):
        layer = dict(layer)
        if "marker" in layer:
            layer["marker"] = {"type": "marker", "layers": numbered(layer["marker"]["layers"])}
        out.append({**layer, "id": str(k)})
    return out


def symbol(kind: str, group: str, slug: str, name: str, path_: list[str], layers: list[dict],
           tags: list[str]) -> None:
    short = {"marker": "nokta", "line": "cizgi", "fill": "alan"}[kind]
    ITEMS.append({"kind": "symbol", "id": f"genel.{short}.{group}.{slug}", "name": name, "path": path_,
                  "symbol": {"type": kind, "layers": numbered(layers)}, "tags": tags})


# Layers.

def shape(kind: str, size: float, fill: str | None = None, stroke_: str | None = None, width: float | None = None,
          **extra) -> dict:
    layer: dict = {"type": "shape", "shape": kind, "size": size}
    layer["fill"] = fill
    if stroke_ is not None:
        layer["stroke"] = stroke_
        layer["strokeWidth"] = width if width is not None else 0.25
    layer.update(extra)
    return layer


def beside(along: float, across: float, turn: float) -> list[float]:
    """A marker's offset that lands it `along` the line and `across` it (up positive) once the marker is turned by
    `turn` degrees: the engine turns a marker's offset with the marker."""
    a = math.radians(-turn)
    return [round(along * math.cos(a) - across * math.sin(a), 4), round(along * math.sin(a) + across * math.cos(a), 4)]


def pic(name: str, size: float, color: str, **extra) -> dict:
    if name not in PICTOGRAMS:
        raise KeyError(f"no pictogram {name}")
    return {"type": "svg", "asset": f"genel.svg.{name}", "size": size, "fill": color, **extra}


def text(t: str, size: float, color: str, font: str = "sans", weight: int = 700, **extra) -> dict:
    return {"type": "text", "text": t, "size": size, "font": font, "weight": weight, "color": color, **extra}


def sline(color: str, width: float, **extra) -> dict:
    return {"type": "simpleLine", "color": color, "width": width, **extra}


def mline(marker: list[dict], interval: float | None = None, placement: str = "interval", **extra) -> dict:
    layer: dict = {"type": "markerLine", "placement": placement, "marker": {"type": "marker", "layers": marker}}
    if interval is not None:
        layer["interval"] = interval
    layer.setdefault("rotate", True)
    layer.update(extra)
    return layer


def sfill(color: str) -> dict:
    return {"type": "simpleFill", "color": color}


def hatch(angle: float, spacing: float, width: float, color: str, **extra) -> dict:
    return {"type": "hatchFill", "angle": angle, "spacing": spacing, "width": width, "color": color, **extra}


def pattern(marker: list[dict], sx: float, sy: float, **extra) -> dict:
    return {"type": "patternFill", "marker": {"type": "marker", "layers": marker}, "spacingX": sx, "spacingY": sy,
            **extra}


def centroid(marker: list[dict]) -> dict:
    return {"type": "centroidMarker", "marker": {"type": "marker", "layers": marker}}


def badge(icon: str, color: str, size: float = 5.0, icon_size: float = 3.2, kind: str = "circle") -> list[dict]:
    """A place pictogram on its theme's disc, white rimmed."""
    return [shape(kind, size, fill=color, stroke_=WHITE, width=0.35), pic(icon, icon_size, WHITE)]


def letter(t: str, color: str, size: float = 5.0, kind: str = "circle", text_size: float = 3.4,
           font: str = "sans") -> list[dict]:
    """A letter on a disc or square (H, P, i, M)."""
    return [shape(kind, size, fill=color, stroke_=WHITE, width=0.35), text(t, text_size, WHITE, font, 700)]


# ── Points ──────────────────────────────────────────────────────────────────────────────────────────────────────────

def points() -> None:
    root = [GEN, POINTS]
    category(root, 1, "Yer, hizmet, altyapı, ölçme ve doğa noktaları; temalarına göre renkli")
    order = iter(range(1, 100))

    def group(name: str, description: str) -> list[str]:
        p = root + [name]
        category(p, next(order), description)
        return p

    # Basic shapes in the colour-blind-safe palette.
    p = group("Temel şekiller", "Tematik haritalar için renk körlüğüne uygun dolgu renklerinde şekiller")
    shapes = [("daire", "Daire", "circle", 3.0), ("kare", "Kare", "square", 2.8), ("ucgen", "Üçgen", "triangle", 3.4),
              ("elmas", "Elmas", "diamond", 3.4), ("besgen", "Beşgen", "pentagon", 3.2),
              ("altigen", "Altıgen", "hexagon", 3.2), ("yildiz", "Yıldız", "star", 3.8),
              ("yarim-daire", "Yarım daire", "semicircle", 3.4)]
    for k, (slug, name, kind, size) in enumerate(shapes):
        symbol("marker", "sekil", slug, name, p, [shape(kind, size, fill=OKABE_ITO[k % 7], stroke_=INK, width=0.2)],
               ["şekil", "tematik"])
    symbol("marker", "sekil", "halka", "Halka", p, [shape("circle", 3.0, fill=None, stroke_=OKABE_ITO[4], width=0.5)],
           ["şekil", "tematik"])
    symbol("marker", "sekil", "hedef", "Hedef", p, [
        shape("circle", 3.6, fill=None, stroke_=INK, width=0.3),
        shape("circle", 1.6, fill=None, stroke_=INK, width=0.3),
        shape("cross", 4.4, stroke_=INK, width=0.2)], ["şekil", "konum"])
    symbol("marker", "sekil", "konum", "Konum işareti", p, [pic("pin", 5.0, OKABE_ITO[5], anchor="bottom")],
           ["konum", "pin", "işaret"])
    symbol("marker", "sekil", "bayrak", "Bayrak", p, [pic("bayrak", 4.4, OKABE_ITO[5], anchor="bottom-left")],
           ["bayrak", "işaret"])

    p = group("Ulaşım", "Duraklar, istasyonlar, otopark, akaryakıt ve araç hizmetleri")
    symbol("marker", "ulasim", "otobus-duragi", "Otobüs durağı", p, badge("otobus", TRANSPORT, kind="square"),
           ["ulaşım", "otobüs", "durak"])
    symbol("marker", "ulasim", "tren-istasyonu", "Tren istasyonu", p, badge("tren", TRANSPORT),
           ["ulaşım", "tren", "demiryolu", "istasyon"])
    symbol("marker", "ulasim", "tramvay-duragi", "Tramvay durağı", p, badge("tramvay", TRANSPORT, kind="square"),
           ["ulaşım", "tramvay", "durak"])
    symbol("marker", "ulasim", "metro", "Metro istasyonu", p, letter("M", "#C62828"),
           ["ulaşım", "metro", "raylı", "istasyon"])
    symbol("marker", "ulasim", "havalimani", "Havalimanı", p, badge("ucak", TRANSPORT, size=5.6, icon_size=3.8),
           ["ulaşım", "havalimanı", "uçak"])
    symbol("marker", "ulasim", "liman", "Liman, iskele", p, badge("capa", TRANSPORT),
           ["ulaşım", "liman", "iskele", "deniz"])
    symbol("marker", "ulasim", "otopark", "Otopark", p, letter("P", TRANSPORT, kind="square", text_size=3.6),
           ["ulaşım", "otopark", "park"])
    symbol("marker", "ulasim", "akaryakit", "Akaryakıt istasyonu", p, badge("akaryakit", TRANSPORT),
           ["ulaşım", "akaryakıt", "benzin"])
    symbol("marker", "ulasim", "sarj", "Elektrikli araç şarj", p, badge("simsek", "#2E9D5B"),
           ["ulaşım", "şarj", "elektrik"])
    symbol("marker", "ulasim", "bisiklet", "Bisiklet istasyonu", p, badge("bisiklet", TRANSPORT),
           ["ulaşım", "bisiklet"])
    symbol("marker", "ulasim", "taksi", "Taksi durağı", p, badge("otomobil", "#F2B705"),
           ["ulaşım", "taksi", "durak"])

    p = group("Sağlık ve acil durum", "Hastane, sağlık merkezi, eczane ve acil durum noktaları")
    symbol("marker", "saglik", "hastane", "Hastane", p, letter("H", HEALTH, kind="square", text_size=3.6),
           ["sağlık", "hastane"])
    symbol("marker", "saglik", "saglik-merkezi", "Sağlık merkezi", p, badge("arti", HEALTH),
           ["sağlık", "sağlık ocağı", "klinik"])
    symbol("marker", "saglik", "eczane", "Eczane", p, badge("kapsul", HEALTH), ["sağlık", "eczane", "ilaç"])
    symbol("marker", "saglik", "veteriner", "Veteriner", p, badge("pati", HEALTH), ["sağlık", "veteriner", "hayvan"])
    symbol("marker", "saglik", "toplanma-alani", "Acil toplanma alanı", p, badge("insanlar", "#2E7D32", kind="square"),
           ["acil", "afet", "toplanma", "deprem"])
    symbol("marker", "saglik", "itfaiye", "İtfaiye", p, badge("alev", "#E64A19"), ["acil", "itfaiye", "yangın"])

    p = group("Eğitim ve kültür", "Okul, üniversite, kütüphane, müze ve kültür merkezleri")
    symbol("marker", "egitim", "okul", "Okul", p, badge("okul", EDUCATION), ["eğitim", "okul"])
    symbol("marker", "egitim", "universite", "Üniversite", p, badge("kep", EDUCATION), ["eğitim", "üniversite"])
    symbol("marker", "egitim", "kutuphane", "Kütüphane", p, badge("kitap", CULTURE), ["kültür", "kütüphane", "kitap"])
    symbol("marker", "egitim", "muze", "Müze", p, badge("muze", CULTURE), ["kültür", "müze"])
    symbol("marker", "egitim", "kultur-merkezi", "Kültür merkezi, konser salonu", p, badge("nota", CULTURE),
           ["kültür", "konser", "tiyatro"])

    p = group("Kamu hizmetleri", "Resmi kurumlar, güvenlik, posta ve adalet")
    symbol("marker", "kamu", "resmi-kurum", "Resmi kurum", p, badge("kamu-binasi", PUBLIC),
           ["kamu", "belediye", "valilik"])
    symbol("marker", "kamu", "polis", "Emniyet, karakol", p, badge("kalkan", "#283593"), ["kamu", "polis", "güvenlik"])
    symbol("marker", "kamu", "posta", "Posta", p, badge("zarf", "#F2B705"), ["kamu", "posta", "ptt"])
    symbol("marker", "kamu", "adliye", "Adliye", p, badge("terazi", PUBLIC), ["kamu", "adliye", "mahkeme"])
    symbol("marker", "kamu", "bilgi", "Bilgi noktası", p, letter("i", TOURISM, font="serif", text_size=3.8),
           ["bilgi", "danışma", "turizm"])

    p = group("İbadet ve anıt", "İbadet yerleri, mezarlık ve anıtlar")
    symbol("marker", "ibadet", "cami", "Cami", p, badge("cami", WORSHIP), ["ibadet", "cami"])
    symbol("marker", "ibadet", "kilise", "Kilise", p, badge("kilise", WORSHIP), ["ibadet", "kilise"])
    symbol("marker", "ibadet", "sinagog", "Sinagog", p, badge("alti-koseli-yildiz", WORSHIP), ["ibadet", "sinagog"])
    symbol("marker", "ibadet", "mezarlik", "Mezarlık", p, badge("mezar", WORSHIP), ["mezarlık"])
    symbol("marker", "ibadet", "anit", "Anıt", p, badge("dikilitas", WORSHIP), ["anıt", "heykel"])
    symbol("marker", "ibadet", "tarihi-yapi", "Kale, tarihi yapı", p, badge("kale", "#8D6E4A"),
           ["tarihi", "kale", "kültür"])

    p = group("Ticaret ve turizm", "Alışveriş, yeme içme, konaklama ve gezi noktaları")
    symbol("marker", "ticaret", "alisveris", "Alışveriş merkezi, mağaza", p, badge("canta", COMMERCE),
           ["ticaret", "alışveriş"])
    symbol("marker", "ticaret", "market", "Market", p, badge("sepet", COMMERCE), ["ticaret", "market"])
    symbol("marker", "ticaret", "banka", "Banka, ATM", p, badge("banknot", COMMERCE), ["ticaret", "banka", "atm"])
    symbol("marker", "ticaret", "lokanta", "Lokanta", p, badge("catal-bicak", COMMERCE), ["yeme içme", "lokanta"])
    symbol("marker", "ticaret", "kafe", "Kafe", p, badge("fincan", COMMERCE), ["yeme içme", "kafe"])
    symbol("marker", "ticaret", "otel", "Otel, konaklama", p, badge("yatak", TOURISM), ["turizm", "otel"])
    symbol("marker", "ticaret", "seyir", "Seyir noktası", p, badge("durbun", TOURISM), ["turizm", "manzara"])
    symbol("marker", "ticaret", "kamp", "Kamp alanı", p, badge("cadir", TOURISM), ["turizm", "kamp"])
    symbol("marker", "ticaret", "piknik", "Piknik alanı", p, badge("piknik", TOURISM), ["turizm", "piknik", "mesire"])

    p = group("Spor ve rekreasyon", "Park, oyun alanı ve spor tesisleri")
    symbol("marker", "spor", "park", "Park", p, badge("agac", SPORT), ["park", "yeşil alan"])
    symbol("marker", "spor", "oyun-alani", "Çocuk oyun alanı", p, badge("salincak", SPORT), ["oyun", "çocuk", "park"])
    symbol("marker", "spor", "spor-sahasi", "Spor sahası", p, badge("top", SPORT), ["spor", "saha", "stadyum"])
    symbol("marker", "spor", "yuzme", "Yüzme havuzu, plaj", p, badge("yuzucu", "#0288D1"), ["spor", "yüzme", "plaj"])
    symbol("marker", "spor", "spor-salonu", "Spor salonu", p, badge("dambil", SPORT), ["spor", "salon"])

    p = group("Altyapı", "Enerji, su, atık su ve haberleşme tesisleri; kendi renklerinde")
    symbol("marker", "altyapi", "elektrik-diregi", "Elektrik direği", p, [
        shape("circle", 1.8, fill=PAPER, stroke_=ELECTRIC, width=0.3), shape("circle", 0.6, fill=ELECTRIC)],
        ["altyapı", "elektrik", "direk"])
    symbol("marker", "altyapi", "aydinlatma", "Aydınlatma direği", p, [
        shape("circle", 1.8, fill=PAPER, stroke_=INK, width=0.25), shape("x", 1.8, stroke_=INK, width=0.2)],
        ["altyapı", "aydınlatma", "direk"])
    symbol("marker", "altyapi", "trafo", "Trafo", p, [
        shape("square", 3.0, fill=PAPER, stroke_=ELECTRIC, width=0.3), pic("simsek", 2.4, ELECTRIC)],
        ["altyapı", "elektrik", "trafo"])
    symbol("marker", "altyapi", "baz-istasyonu", "Baz istasyonu, anten", p, [pic("anten", 4.0, TELECOM)],
           ["altyapı", "haberleşme", "anten"])
    symbol("marker", "altyapi", "rogar", "Rögar (atık su)", p, [
        shape("circle", 2.2, fill=PAPER, stroke_=SEWER, width=0.3), shape("x", 1.5, stroke_=SEWER, width=0.25)],
        ["altyapı", "atık su", "rögar", "kanalizasyon"])
    symbol("marker", "altyapi", "izgara", "Yağmur suyu ızgarası", p, [
        shape("square", 2.2, fill=PAPER, stroke_=STORM, width=0.3), shape("line", 1.6, stroke_=STORM, width=0.2),
        shape("line", 1.6, stroke_=STORM, width=0.2, offset=[0, 0.55]), shape("line", 1.6, stroke_=STORM, width=0.2,
                                                                              offset=[0, -0.55])],
        ["altyapı", "yağmur suyu", "ızgara"])
    symbol("marker", "altyapi", "vana", "Vana", p, [pic("vana", 3.0, DRINKING)], ["altyapı", "su", "vana"])
    symbol("marker", "altyapi", "hidrant", "Yangın musluğu", p, [pic("hidrant", 3.4, HEALTH)],
           ["altyapı", "yangın", "hidrant"])
    symbol("marker", "altyapi", "su-kuyusu", "Su kuyusu", p, [
        shape("circle", 2.4, fill=PAPER, stroke_=DRINKING, width=0.35), shape("circle", 0.9, fill=DRINKING)],
        ["altyapı", "su", "kuyu"])
    symbol("marker", "altyapi", "su-deposu", "Su deposu", p, [pic("depo", 3.4, DRINKING)], ["altyapı", "su", "depo"])
    symbol("marker", "altyapi", "dogalgaz", "Doğal gaz regülatörü", p, [
        shape("square", 3.0, fill=GAS, stroke_=INK, width=0.2), text("G", 2.2, INK)],
        ["altyapı", "doğal gaz", "regülatör"])

    p = group("Ölçme", "Nirengi, poligon, nivelman ve sınır noktaları; mürekkeple")
    symbol("marker", "olcme", "nirengi", "Nirengi noktası", p, [
        shape("triangle", 3.2, fill=None, stroke_=INK, width=0.25), shape("circle", 0.5, fill=INK)],
        ["ölçme", "nirengi", "kontrol noktası"])
    symbol("marker", "olcme", "gnss", "GNSS noktası", p, [
        shape("circle", 3.2, fill=None, stroke_=INK, width=0.25), shape("triangle", 1.8, fill=INK)],
        ["ölçme", "gnss", "gps", "kontrol noktası"])
    symbol("marker", "olcme", "poligon", "Poligon noktası", p, [
        shape("circle", 2.0, fill=None, stroke_=INK, width=0.25), shape("circle", 0.4, fill=INK)],
        ["ölçme", "poligon"])
    symbol("marker", "olcme", "nivelman", "Nivelman noktası", p, [
        shape("circle", 2.4, fill=None, stroke_=INK, width=0.25), shape("semicircle", 2.4, fill=INK)],
        ["ölçme", "nivelman", "röper"])
    symbol("marker", "olcme", "sinir-tasi", "Sınır taşı", p, [shape("square", 1.2, fill=INK)],
           ["ölçme", "sınır", "taş"])
    symbol("marker", "olcme", "kot", "Kot noktası", p, [shape("cross", 1.6, stroke_=INK, width=0.18)],
           ["ölçme", "kot", "yükseklik"])
    symbol("marker", "olcme", "detay", "Detay noktası", p, [shape("circle", 0.6, fill=INK)], ["ölçme", "detay"])

    p = group("Doğa", "Ağaç, çalı, kaynak, mağara, kaya ve zirve")
    symbol("marker", "doga", "agac", "Ağaç (yapraklı)", p, [pic("agac", 3.6, FOREST)], ["doğa", "ağaç"])
    symbol("marker", "doga", "konifer", "Ağaç (iğne yapraklı)", p, [pic("konifer", 3.8, "#2F6B3A")],
           ["doğa", "ağaç", "çam"])
    symbol("marker", "doga", "cali", "Çalı", p, [pic("cali", 3.0, "#6B8E23")], ["doğa", "çalı"])
    symbol("marker", "doga", "kaynak", "Kaynak, pınar", p, [pic("damla", 3.0, WATER_LINE)], ["doğa", "su", "kaynak"])
    symbol("marker", "doga", "magara", "Mağara", p, [pic("magara", 3.2, EARTH)], ["doğa", "mağara"])
    symbol("marker", "doga", "kaya", "Kaya", p, [pic("kaya", 3.0, "#6D6D6D")], ["doğa", "kaya"])
    symbol("marker", "doga", "zirve", "Zirve, tepe", p, [shape("triangle", 2.6, fill=EARTH),
                                                        shape("circle", 0.5, fill=PAPER, offset=[0, -0.2])],
           ["doğa", "zirve", "tepe"])

    p = group("Uyarı", "Tehlike, yasak ve dikkat işaretleri")
    symbol("marker", "uyari", "dikkat", "Dikkat", p, [
        shape("triangle", 5.0, fill=WARNING, stroke_=INK, width=0.3), text("!", 3.0, INK, offset=[0, -0.35])],
        ["uyarı", "dikkat", "tehlike"])
    symbol("marker", "uyari", "giris-yasak", "Girilmez", p, [
        shape("circle", 4.6, fill="#D32F2F", stroke_=WHITE, width=0.3),
        shape("rectangle", 3.0, fill=WHITE, height=0.8)], ["uyarı", "yasak"])
    symbol("marker", "uyari", "yangin", "Yangın tehlikesi", p, [
        shape("triangle", 5.0, fill=WARNING, stroke_=INK, width=0.3), pic("alev", 2.4, INK, offset=[0, -0.4])],
        ["uyarı", "yangın"])


# ── Lines ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def lines() -> None:
    root = [GEN, LINES]
    category(root, 2, "Yol, raylı sistem, sınır, su, altyapı hattı ve topoğrafya çizgileri")
    order = iter(range(1, 100))

    def group(name: str, description: str) -> list[str]:
        p = root + [name]
        category(p, next(order), description)
        return p

    p = group("Yollar", "Kademesine göre kenarlı yollar, toprak yollar, patika ve bisiklet yolu")
    roads = [("otoyol", "Otoyol", "#8C3B12", "#F0874A", 1.4, 1.0), ("devlet-yolu", "Devlet yolu", "#8A6A12", "#F6CF4A", 1.2, 0.85),
             ("il-yolu", "İl yolu, bağlantı yolu", "#7A6A3A", "#FFF1B8", 1.0, 0.7),
             ("cadde", "Cadde", "#808080", "#FFFFFF", 0.9, 0.62), ("sokak", "Sokak", "#9E9E9E", "#FFFFFF", 0.6, 0.38)]
    for slug, name, casing, core, w1, w2 in roads:
        symbol("line", "yol", slug, name, p, [sline(casing, w1, cap="round", join="round"),
                                              sline(core, w2, cap="round", join="round")], ["yol", "ulaşım"])
    symbol("line", "yol", "stabilize", "Stabilize yol", p, [
        sline(EARTH, 0.18, dash=[2.4, 1], offset=0.35), sline(EARTH, 0.18, dash=[2.4, 1], offset=-0.35)],
        ["yol", "toprak", "stabilize"])
    symbol("line", "yol", "arazi-yolu", "Arazi yolu", p, [sline(EARTH, 0.3, dash=[3, 1.2])], ["yol", "toprak", "arazi"])
    symbol("line", "yol", "patika", "Patika", p, [sline("#B5543C", 0.3, dash=[1, 0.9], cap="round")],
           ["yol", "patika", "yürüyüş"])
    symbol("line", "yol", "bisiklet", "Bisiklet yolu", p, [sline("#2E86C1", 0.4, dash=[2, 0.8])],
           ["yol", "bisiklet"])
    symbol("line", "yol", "yaya", "Yaya yolu", p, [sline("#5F5F5F", 0.5, dash=[0.01, 1.2], cap="round")],
           ["yol", "yaya"])
    symbol("line", "yol", "merdiven", "Merdiven", p, [
        mline([shape("line", 1.0, stroke_="#606060", width=0.18, rotation=90)], interval=0.6)],
        ["yol", "merdiven", "yaya"])
    # The deck's two edges, each end flaring outward on both sides.
    flare = lambda along, across, turn: shape("line", 0.7, stroke_=INK, width=0.25, rotation=turn,
                                              offset=beside(along, across, turn))
    symbol("line", "yol", "kopru", "Köprü", p, [
        sline(INK, 0.25, offset=0.75), sline(INK, 0.25, offset=-0.75),
        mline([flare(-0.25, 1.0, 135), flare(-0.25, -1.0, 45)], placement="first"),
        mline([flare(0.25, 1.0, 45), flare(0.25, -1.0, 135)], placement="last")],
        ["yol", "köprü"])
    symbol("line", "yol", "tunel", "Tünel", p, [
        sline("#707070", 0.2, dash=[1.2, 0.8], offset=0.6), sline("#707070", 0.2, dash=[1.2, 0.8], offset=-0.6)],
        ["yol", "tünel"])

    p = group("Raylı sistem", "Demiryolu, tramvay, metro ve teleferik")
    symbol("line", "rayli", "demiryolu", "Demiryolu", p, [sline(INK, 0.9), sline(PAPER, 0.55, dash=[3, 3])],
           ["raylı", "demiryolu", "tren"])
    symbol("line", "rayli", "cift-hat", "Demiryolu (çift hat)", p, [
        sline(INK, 0.25, offset=0.45), sline(INK, 0.25, offset=-0.45),
        mline([shape("line", 1.4, stroke_=INK, width=0.2, rotation=90)], interval=2.5)],
        ["raylı", "demiryolu", "çift hat"])
    symbol("line", "rayli", "tramvay", "Tramvay", p, [
        sline("#5C5C5C", 0.35), mline([shape("line", 1.1, stroke_="#5C5C5C", width=0.18, rotation=90)], interval=3)],
        ["raylı", "tramvay"])
    symbol("line", "rayli", "metro", "Metro (yer altı)", p, [sline("#C62828", 0.5, dash=[3, 1.5])],
           ["raylı", "metro"])
    symbol("line", "rayli", "teleferik", "Teleferik", p, [
        sline(INK, 0.2), mline([shape("circle", 0.8, fill=INK)], interval=6, offsetAlong=3)],
        ["raylı", "teleferik", "kablolu"])

    p = group("Sınırlar", "İdari, mülkiyet, plan ve koruma sınırları")
    symbol("line", "sinir", "ulke", "Ülke sınırı", p, [
        sline("#9B59B680", 2.2), sline(INK, 0.5, dash=[6, 1.2, 0.6, 1.2, 0.6, 1.2])], ["sınır", "idari", "ülke"])
    symbol("line", "sinir", "il", "İl sınırı", p, [
        sline("#B583D166", 1.6), sline(INK, 0.4, dash=[5, 1.2, 0.6, 1.2])], ["sınır", "idari", "il"])
    symbol("line", "sinir", "ilce", "İlçe sınırı", p, [sline(INK, 0.35, dash=[4, 1, 0.5, 1])],
           ["sınır", "idari", "ilçe"])
    symbol("line", "sinir", "mahalle", "Mahalle, köy sınırı", p, [sline(INK, 0.25, dash=[2.4, 0.9])],
           ["sınır", "idari", "mahalle", "köy"])
    symbol("line", "sinir", "ada", "Ada sınırı", p, [sline(INK, 0.35)], ["sınır", "ada", "mülkiyet"])
    symbol("line", "sinir", "parsel", "Parsel sınırı", p, [sline(INK, 0.18)], ["sınır", "parsel", "mülkiyet"])
    symbol("line", "sinir", "plan", "Plan sınırı", p, [sline("#C0392B", 0.45, dash=[6, 1.2, 0.8, 1.2])],
           ["sınır", "plan", "imar"])
    symbol("line", "sinir", "koruma", "Koruma alanı sınırı", p, [
        sline("#2E7D32", 0.35),
        mline([shape("line", 0.9, stroke_="#2E7D32", width=0.2, rotation=90)], interval=1.6, offset=0.5)],
        ["sınır", "koruma", "doğa"])
    symbol("line", "sinir", "sit", "Sit alanı sınırı", p, [
        sline("#8E44AD", 0.35, dash=[3, 1]), mline([shape("circle", 0.55, fill="#8E44AD")], interval=4, offsetAlong=3.5)],
        ["sınır", "sit", "koruma"])
    symbol("line", "sinir", "kiyi", "Kıyı kenar çizgisi", p, [sline("#1565C0", 0.35, dash=[4, 1, 0.6, 1])],
           ["sınır", "kıyı", "su"])

    p = group("Su", "Akarsu, kanal ve kıyı çizgileri")
    symbol("line", "su", "dere", "Dere", p, [sline(WATER_LINE, 0.3, cap="round", join="round")], ["su", "dere", "akarsu"])
    symbol("line", "su", "nehir", "Nehir, çay", p, [sline(WATER_LINE, 0.7, cap="round", join="round")],
           ["su", "nehir", "akarsu"])
    symbol("line", "su", "mevsimlik", "Mevsimlik dere", p, [sline(WATER_LINE, 0.3, dash=[2.2, 0.8, 0.4, 0.8])],
           ["su", "dere", "kuru dere"])
    symbol("line", "su", "akis-yonlu", "Akış yönlü dere", p, [
        sline(WATER_LINE, 0.35), mline([shape("arrowhead", 1.2, fill=WATER_LINE)], interval=15, offsetAlong=7.5)],
        ["su", "dere", "akış"])
    symbol("line", "su", "kanal", "Kanal", p, [
        sline(WATER_FILL, 0.9), sline(WATER_LINE, 0.2, offset=0.45), sline(WATER_LINE, 0.2, offset=-0.45)],
        ["su", "kanal"])
    symbol("line", "su", "sulama", "Sulama kanalı", p, [
        sline(WATER_LINE, 0.3), mline([shape("line", 0.8, stroke_=WATER_LINE, width=0.18, rotation=90)], interval=2.5)],
        ["su", "sulama", "kanal"])
    symbol("line", "su", "kiyi-cizgisi", "Kıyı çizgisi", p, [sline(WATER_DARK, 0.35)], ["su", "kıyı", "deniz"])

    p = group("Altyapı hatları", "Enerji, doğal gaz, su, atık su ve haberleşme hatları; kendi renklerinde")
    symbol("line", "hat", "yuksek-gerilim", "Yüksek gerilim hattı", p, [
        sline(ELECTRIC, 0.3), mline([shape("chevron", 1.4, stroke_=ELECTRIC, width=0.25)], interval=8, offsetAlong=4)],
        ["altyapı", "elektrik", "enerji nakil"])
    symbol("line", "hat", "orta-gerilim", "Orta gerilim hattı", p, [
        sline(ELECTRIC, 0.25), mline([shape("circle", 0.7, fill=ELECTRIC)], interval=6, offsetAlong=3)],
        ["altyapı", "elektrik"])
    symbol("line", "hat", "alcak-gerilim", "Alçak gerilim hattı", p, [sline(ELECTRIC, 0.2, dash=[2, 0.8])],
           ["altyapı", "elektrik"])
    for slug, name, color, mark, tags in [
        ("dogalgaz", "Doğal gaz hattı", GAS, "G", ["altyapı", "doğal gaz"]),
        ("icme-suyu", "İçme suyu hattı", DRINKING, "S", ["altyapı", "su", "içme suyu"]),
        ("yagmur-suyu", "Yağmur suyu hattı", STORM, "Y", ["altyapı", "yağmur suyu"]),
        ("haberlesme", "Haberleşme hattı", TELECOM, "T", ["altyapı", "haberleşme", "fiber"]),
        ("petrol", "Petrol boru hattı", FUEL, "P", ["altyapı", "petrol", "boru"]),
    ]:
        symbol("line", "hat", slug, name, p, [
            sline(color, 0.4, dash=[5, 1.4]),
            mline([text(mark, 1.8, color, halo={"color": PAPER, "width": 0.35})], interval=18, offsetAlong=9)], tags)
    symbol("line", "hat", "atik-su", "Atık su hattı", p, [
        sline(SEWER, 0.45), mline([shape("arrowhead", 1.1, fill=SEWER)], interval=12, offsetAlong=6)],
        ["altyapı", "atık su", "kanalizasyon"])

    p = group("Topoğrafya ve yapı", "Eş yükselti eğrileri, şev, duvar ve çitler")
    symbol("line", "topo", "es-yukselti-ana", "Eş yükselti eğrisi (ana)", p, [sline(CONTOUR, 0.25)],
           ["topoğrafya", "eş yükselti", "eğri"])
    symbol("line", "topo", "es-yukselti-ara", "Eş yükselti eğrisi (ara)", p, [sline(CONTOUR, 0.13)],
           ["topoğrafya", "eş yükselti", "eğri"])
    symbol("line", "topo", "es-yukselti-yardimci", "Eş yükselti eğrisi (yardımcı)", p,
           [sline(CONTOUR, 0.13, dash=[5, 1])], ["topoğrafya", "eş yükselti", "eğri"])
    symbol("line", "topo", "sev", "Şev", p, [
        sline(INK, 0.25),
        mline([shape("line", 1.6, stroke_=INK, width=0.15, rotation=90, offset=beside(0, -0.8, 90))], interval=1.6),
        mline([shape("line", 0.8, stroke_=INK, width=0.15, rotation=90, offset=beside(0, -0.4, 90))], interval=1.6,
              offsetAlong=0.8)], ["topoğrafya", "şev", "yarma", "dolgu"])
    symbol("line", "topo", "istinat", "İstinat duvarı", p, [
        sline(INK, 0.5), mline([shape("triangle", 0.8, fill=INK, offset=[0, -0.55], rotation=180)], interval=1.8)],
        ["yapı", "duvar", "istinat"])
    symbol("line", "topo", "duvar", "Duvar", p, [sline(INK, 0.6)], ["yapı", "duvar"])
    symbol("line", "topo", "cit", "Çit", p, [
        sline(INK, 0.18), mline([shape("line", 0.9, stroke_=INK, width=0.18, rotation=90)], interval=2)],
        ["yapı", "çit"])
    symbol("line", "topo", "tel-orgu", "Tel örgü", p, [
        sline(INK, 0.18), mline([shape("x", 0.9, stroke_=INK, width=0.15)], interval=2.2, offsetAlong=1.1)],
        ["yapı", "tel örgü", "çit"])

    p = group("Oklar ve genel", "Yön okları, akış, dalgalı ve kırık çizgiler")
    symbol("line", "genel", "ok", "Ok", p, [
        sline(INK, 0.35), mline([shape("arrowhead", 2.0, fill=INK)], placement="last")], ["ok", "yön"])
    symbol("line", "genel", "cift-ok", "Çift yönlü ok", p, [
        sline(INK, 0.35), mline([shape("arrowhead", 2.0, fill=INK)], placement="last"),
        mline([shape("arrowhead", 2.0, fill=INK, rotation=180)], placement="first")], ["ok", "yön"])
    symbol("line", "genel", "akis", "Akış yönü", p, [
        sline("#0072B2", 0.4), mline([shape("chevron", 1.4, stroke_="#0072B2", width=0.35)], interval=6, offsetAlong=3)],
        ["ok", "akış", "yön"])
    symbol("line", "genel", "dalgali", "Dalgalı çizgi", p, [
        sline(INK, 0.25, wave={"shape": "sine", "length": 3, "amplitude": 0.6, "connect": True})], ["dalga"])
    symbol("line", "genel", "zikzak", "Zikzak çizgi", p, [
        sline(INK, 0.25, wave={"shape": "zigzag", "length": 2.4, "amplitude": 0.7, "connect": True})], ["zikzak"])
    symbol("line", "genel", "uzun-kesikli", "Uzun kesikli", p, [sline(INK, 0.3, dash=[8, 2])], ["kesikli"])
    symbol("line", "genel", "rota", "Rota", p, [sline("#D55E00", 0.5, dash=[3, 1, 0.6, 1], cap="round")],
           ["rota", "güzergâh"])


# ── Areas ───────────────────────────────────────────────────────────────────────────────────────────────────────────

def areas() -> None:
    root = [GEN, AREAS]
    category(root, 3, "Arazi örtüsü, su, yapılaşma, kentsel kullanım, tarama desenleri ve kısıt alanları")
    order = iter(range(1, 100))

    def group(name: str, description: str) -> list[str]:
        p = root + [name]
        category(p, next(order), description)
        return p

    def edge(color: str, width: float = 0.25, **extra) -> dict:
        return sline(color, width, **extra)

    p = group("Arazi örtüsü", "Orman, çayır, tarım, bağ bahçe, kumluk ve kayalık")
    symbol("fill", "arazi", "orman", "Orman (yapraklı)", p, [
        sfill(FOREST_FILL), pattern([pic("agac", 2.2, FOREST)], 4.5, 4.5, stagger=True, jitter=0.25, seed=7),
        edge(FOREST, dash=[1.2, 0.6])], ["arazi örtüsü", "orman"])
    symbol("fill", "arazi", "orman-ibreli", "Orman (iğne yapraklı)", p, [
        sfill("#CDE5C7"), pattern([pic("konifer", 2.4, "#2F6B3A")], 4.5, 4.5, stagger=True, jitter=0.25, seed=11),
        edge("#2F6B3A", dash=[1.2, 0.6])], ["arazi örtüsü", "orman", "çam"])
    symbol("fill", "arazi", "orman-karisik", "Orman (karışık)", p, [
        sfill("#D0E8C6"), pattern([pic("agac", 2.1, FOREST)], 6, 6, stagger=True, jitter=0.2, seed=3),
        pattern([pic("konifer", 2.3, "#2F6B3A")], 6, 6, stagger=True, offset=[3, 0], jitter=0.2, seed=5),
        edge(FOREST, dash=[1.2, 0.6])], ["arazi örtüsü", "orman"])
    symbol("fill", "arazi", "fundalik", "Fundalık, çalılık", p, [
        sfill("#E4EFCB"), pattern([pic("cali", 1.8, "#6B8E23")], 4, 4, stagger=True, jitter=0.35, seed=13)],
        ["arazi örtüsü", "çalı", "fundalık"])
    symbol("fill", "arazi", "cayir", "Çayır, mera", p, [
        sfill("#E8F3D1"), pattern([pic("ot", 1.4, "#5E9C3F")], 3.5, 3.5, stagger=True, jitter=0.35, seed=17)],
        ["arazi örtüsü", "çayır", "mera"])
    symbol("fill", "arazi", "tarim", "Tarım arazisi", p, [
        sfill("#F5ECC4"), hatch(0, 1.8, 0.12, "#C4AE6A", dash=[3, 1.2])], ["arazi örtüsü", "tarım", "tarla"])
    symbol("fill", "arazi", "bag", "Bağ", p, [
        sfill("#EAF0C9"), pattern([shape("circle", 0.7, fill="#7A8F2E")], 2.2, 1.6, angle=0)],
        ["arazi örtüsü", "bağ", "üzüm"])
    symbol("fill", "arazi", "meyve-bahcesi", "Meyve bahçesi", p, [
        sfill("#E5EFC9"), pattern([shape("circle", 1.3, fill=None, stroke_="#5E8C2E", width=0.2)], 3.2, 3.2)],
        ["arazi örtüsü", "bahçe", "meyve"])
    symbol("fill", "arazi", "zeytinlik", "Zeytinlik", p, [
        sfill("#E2EAC7"), pattern([shape("circle", 1.2, fill=None, stroke_="#6B7B3A", width=0.2),
                                  shape("circle", 0.35, fill="#6B7B3A")], 3.4, 3.4, stagger=True)],
        ["arazi örtüsü", "zeytin"])
    symbol("fill", "arazi", "kumluk", "Kumluk", p, [
        sfill("#F6EBC9"), pattern([shape("circle", 0.28, fill="#B09060")], 1.1, 1.1, jitter=0.45, seed=19)],
        ["arazi örtüsü", "kum", "plaj"])
    symbol("fill", "arazi", "kayalik", "Kayalık", p, [
        sfill("#E6E2DA"), pattern([pic("kaya", 1.6, "#7A7468")], 3.4, 3.0, stagger=True, jitter=0.3, seed=23)],
        ["arazi örtüsü", "kaya"])
    symbol("fill", "arazi", "ciplak", "Çıplak arazi", p, [
        sfill("#EFEAE0"), pattern([shape("circle", 0.3, fill="#9C8F7A")], 2.4, 2.4, jitter=0.45, seed=29)],
        ["arazi örtüsü", "çıplak", "taşlık"])

    p = group("Su ve sulak alan", "Göl, deniz, sulak alan ve havuz")
    symbol("fill", "su", "gol", "Göl, gölet", p, [sfill(WATER_FILL), edge(WATER_LINE, 0.3)], ["su", "göl"])
    symbol("fill", "su", "deniz", "Deniz", p, [sfill("#9CC8EE"), edge(WATER_DARK, 0.3)], ["su", "deniz"])
    symbol("fill", "su", "baraj", "Baraj gölü", p, [
        sfill(WATER_FILL), hatch(0, 2.4, 0.12, WATER_LINE, dash=[2, 2]), edge(WATER_LINE, 0.3)], ["su", "baraj"])
    symbol("fill", "su", "sulak-alan", "Sulak alan, bataklık", p, [
        sfill("#E2F1F6"), hatch(0, 1.8, 0.15, WATER_LINE, dash=[1.4, 1.1], stagger=1.25),
        pattern([pic("ot", 1.3, "#4E8F5A")], 5, 5, stagger=True, jitter=0.3, seed=31)], ["su", "bataklık", "sulak alan"])
    symbol("fill", "su", "havuz", "Havuz", p, [sfill("#BFE6FA"), edge("#0288D1", 0.3)], ["su", "havuz"])

    p = group("Yapılaşma", "Bina, kamu binası, yıkıntı, inşaat ve sera")
    symbol("fill", "yapi", "bina", "Bina", p, [sfill("#D3CDC4"), edge("#6E6559", 0.25)], ["yapı", "bina"])
    symbol("fill", "yapi", "kamu-binasi", "Kamu binası", p, [sfill("#B9C7DD"), edge("#4A5B78", 0.3)],
           ["yapı", "bina", "kamu"])
    symbol("fill", "yapi", "yikinti", "Yıkıntı", p, [
        hatch(45, 1.4, 0.12, "#8A8070"), edge(INK, 0.25, dash=[1.5, 0.8])], ["yapı", "yıkıntı", "harabe"])
    symbol("fill", "yapi", "insaat", "İnşaat alanı", p, [
        sfill("#EFEAE0"), hatch(45, 2.2, 0.15, "#9E8F7A", dash=[2, 1]), edge("#9E8F7A", 0.25, dash=[3, 1])],
        ["yapı", "inşaat"])
    symbol("fill", "yapi", "sera", "Sera", p, [
        sfill("#E9F4E9"), hatch(0, 1.2, 0.1, "#7FA97F"), hatch(90, 1.2, 0.1, "#7FA97F"), edge("#5E8C5E", 0.25)],
        ["yapı", "sera", "tarım"])

    p = group("Kentsel kullanım", "Konut, ticaret, sanayi, yeşil alan ve donatı alanları")
    uses = [("konut", "Konut alanı", "#FFE7A3", "#C9A23C", ["kullanım", "konut"]),
            ("ticaret", "Ticaret alanı", "#F7B2B0", "#C0504D", ["kullanım", "ticaret"]),
            ("sanayi", "Sanayi alanı", "#D9C6EA", "#7E5A9E", ["kullanım", "sanayi"]),
            ("egitim", "Eğitim alanı", "#FFD9B0", "#D9731A", ["kullanım", "eğitim", "donatı"]),
            ("saglik", "Sağlık alanı", "#F8C6CD", "#C62828", ["kullanım", "sağlık", "donatı"]),
            ("kamu", "Kamu alanı", "#CBD7EC", "#4A5B78", ["kullanım", "kamu", "donatı"]),
            ("spor", "Spor alanı", "#B8E2C8", "#2E7D32", ["kullanım", "spor", "donatı"])]
    for slug, name, fill_, line_, tags in uses:
        symbol("fill", "kullanim", slug, name, p, [sfill(fill_), edge(line_, 0.25)], tags)
    symbol("fill", "kullanim", "park", "Park, yeşil alan", p, [
        sfill("#C6E6B0"), pattern([pic("agac", 1.8, "#4C9A42")], 7, 7, stagger=True), edge("#4C9A42", 0.25)],
        ["kullanım", "park", "yeşil alan"])
    symbol("fill", "kullanim", "mezarlik", "Mezarlık", p, [
        sfill("#C9D9C2"), pattern([pic("mezar", 1.3, "#5E7358")], 3.2, 3.2, stagger=True), edge("#5E7358", 0.25)],
        ["kullanım", "mezarlık"])
    symbol("fill", "kullanim", "otopark", "Otopark", p, [
        sfill("#E3E3E3"), edge("#7A7A7A", 0.25), centroid(letter("P", TRANSPORT, size=4.0, kind="square", text_size=2.9))],
        ["kullanım", "otopark"])
    symbol("fill", "kullanim", "askeri", "Askeri alan", p, [
        sfill("#F6E0E0"), hatch(45, 2, 0.2, "#C62828"), edge("#C62828", 0.35)], ["kullanım", "askeri"])

    p = group("Tarama desenleri", "Mürekkeple düz, çapraz, kesikli ve noktalı taramalar")
    hatches = [("yatay", "Yatay", [0]), ("dikey", "Dikey", [90]), ("egik-45", "Eğik 45°", [45]),
               ("egik-135", "Eğik 135°", [135]), ("izgara", "Kareli", [0, 90]), ("capraz", "Çapraz", [45, 135])]
    for slug, name, angles in hatches:
        symbol("fill", "tarama", slug, name, p, [hatch(a, 2, 0.13, INK) for a in angles] + [edge(INK, 0.25)],
               ["tarama", "desen"])
    symbol("fill", "tarama", "sik-45", "Sık eğik", p, [hatch(45, 1, 0.1, INK), edge(INK, 0.25)], ["tarama", "desen"])
    symbol("fill", "tarama", "kesikli-45", "Kesikli eğik", p, [hatch(45, 2, 0.13, INK, dash=[2, 1]), edge(INK, 0.25)],
           ["tarama", "desen"])
    symbol("fill", "tarama", "tugla", "Tuğla", p, [
        hatch(0, 1.6, 0.13, INK), hatch(90, 3.2, 0.13, INK, dash=[1.6, 1.6]),
        hatch(90, 3.2, 0.13, INK, dash=[1.6, 1.6], dashOffset=1.6, offset=1.6), edge(INK, 0.25)],
        ["tarama", "desen", "tuğla"])
    symbol("fill", "tarama", "seyrek-nokta", "Seyrek noktalı", p, [
        pattern([shape("circle", 0.35, fill=INK)], 2.4, 2.4, stagger=True), edge(INK, 0.25)], ["tarama", "nokta"])
    symbol("fill", "tarama", "sik-nokta", "Sık noktalı", p, [
        pattern([shape("circle", 0.3, fill=INK)], 1.2, 1.2, stagger=True), edge(INK, 0.25)], ["tarama", "nokta"])
    symbol("fill", "tarama", "dalgali", "Dalgalı", p, [
        pattern([pic("dalga", 2.4, INK)], 3, 2.4, stagger=True), edge(INK, 0.25)], ["tarama", "dalga"])

    p = group("Kısıt ve tehlike", "Taşkın, heyelan, afet riski, koruma, yasak ve tampon alanları")
    symbol("fill", "kisit", "taskin", "Taşkın alanı", p, [
        sfill("#DCEEFB80"), hatch(45, 1.6, 0.15, WATER_LINE), edge(WATER_LINE, 0.3, dash=[3, 1])],
        ["kısıt", "taşkın", "dere yatağı"])
    symbol("fill", "kisit", "heyelan", "Heyelan alanı", p, [
        sfill("#F1E3D080"), pattern([shape("arrowhead", 1.1, fill=EARTH, rotation=-90)], 3, 3, stagger=True),
        edge(EARTH, 0.3, dash=[3, 1])], ["kısıt", "heyelan", "jeoloji"])
    symbol("fill", "kisit", "afet-riskli", "Afet riskli alan", p, [
        sfill("#F8D7D766"), hatch(45, 2, 0.18, "#C62828"), hatch(135, 2, 0.18, "#C62828"), edge("#C62828", 0.4)],
        ["kısıt", "afet", "risk"])
    symbol("fill", "kisit", "koruma", "Koruma alanı", p, [
        sfill("#DDF0D840"), edge("#2E7D32", 0.4),
        mline([shape("line", 1.2, stroke_="#2E7D32", width=0.25, rotation=90)], interval=1.4, offset=0.7)],
        ["kısıt", "koruma", "doğa"])
    symbol("fill", "kisit", "yasak", "Yasak alan", p, [
        edge("#C62828", 0.45),
        mline([shape("line", 1.4, stroke_="#C62828", width=0.25, rotation=45)], interval=1.2, offset=0.8)],
        ["kısıt", "yasak", "askeri"])
    symbol("fill", "kisit", "tampon", "Tampon bölge", p, [sfill("#FFE08A40"), edge("#B8860B", 0.3, dash=[2, 1.2])],
           ["kısıt", "tampon", "etki alanı"])


def library() -> dict:
    CATEGORIES.clear()
    ITEMS.clear()
    category([GEN], 3, "Her türden harita için kategorili nokta, çizgi ve alan sembolleri; ölçüler kâğıt milimetresidir")
    points()
    lines()
    areas()
    category([GEN, "Piktogramlar"], 9, "Genel sembollerin piktogramları (SVG)")
    used = {layer["asset"].removeprefix("genel.svg.") for item in ITEMS for layer in walk(item["symbol"]["layers"])
            if layer.get("type") == "svg"}
    assets = [{"kind": "asset", "id": f"genel.svg.{name}", "name": name, "path": [GEN, "Piktogramlar"], "format": "svg",
               "data": PICTOGRAMS[name], "width": 100, "height": 100} for name in PICTOGRAMS if name in used]
    unused = sorted(set(PICTOGRAMS) - used)
    if unused:
        raise SystemExit(f"pictograms no symbol uses: {', '.join(unused)}")
    ids = [i["id"] for i in ITEMS]
    if len(ids) != len(set(ids)):
        raise SystemExit("duplicate symbol ids")
    return {"format": "kentos-style", "version": 1, "exported": "system", "items": ITEMS + assets,
            "categories": CATEGORIES}


def walk(layers: list[dict]):
    for layer in layers:
        yield layer
        if "marker" in layer:
            yield from walk(layer["marker"]["layers"])


def main() -> int:
    if "--svg" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--svg") + 1])
        out.write_text(json.dumps(PICTOGRAMS, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"{out}: {len(PICTOGRAMS)} pictograms")
        return 0
    lib = library()
    text_ = json.dumps(lib, ensure_ascii=False, separators=(",", ":")) + "\n"
    kinds = {"marker": 0, "line": 0, "fill": 0}
    for item in ITEMS:
        kinds[item["symbol"]["type"]] += 1
    summary = (f"{len(ITEMS)} symbols ({kinds['marker']} point, {kinds['line']} line, {kinds['fill']} area), "
               f"{len(lib['items']) - len(ITEMS)} pictograms, {len(CATEGORIES)} categories")
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_:
            print(f"{OUT.relative_to(ROOT)} is stale: run python3 scripts/style/general_library.py", file=sys.stderr)
            return 1
        print(f"up to date: {summary}")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {summary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
