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


# City pictograms (docs/adr/9119). The plan views of plants (crowns, shrubs) fill their box: drawn at a size of N
# metres they are N metres across, as a landscape plan draws them.

def scallops(r: float, n: int, bulge: float = 1.25, cx: float = 50, cy: float = 50, turn: float = 0.0) -> str:
    """A crown's edge: n arcs bulging outward between n points on a circle of radius r."""
    pts = [(cx + r * math.cos(math.radians(turn + k * 360 / n)), cy + r * math.sin(math.radians(turn + k * 360 / n)))
           for k in range(n)]
    chord = 2 * r * math.sin(math.pi / n)
    a = chord / 2 * bulge
    d = f"M{pts[0][0]:.1f} {pts[0][1]:.1f}"
    for k in range(1, n + 1):
        x, y = pts[k % n]
        d += f"A{a:.1f} {a:.1f} 0 0 1 {x:.1f} {y:.1f}"
    return d + "Z"


def lobes(radii: list[float], cx: float = 50, cy: float = 50) -> str:
    """An irregular shrub: arcs between points at the given radii, evenly turned."""
    n = len(radii)
    pts = [(cx + r * math.cos(math.radians(-90 + k * 360 / n)), cy + r * math.sin(math.radians(-90 + k * 360 / n)))
           for k, r in enumerate(radii)]
    d = f"M{pts[0][0]:.1f} {pts[0][1]:.1f}"
    for k in range(1, n + 1):
        (x0, y0), (x1, y1) = pts[k - 1], pts[k % n]
        a = math.hypot(x1 - x0, y1 - y0) / 2 * 1.15
        d += f"A{a:.1f} {a:.1f} 0 0 1 {x1:.1f} {y1:.1f}"
    return d + "Z"


def outline(d: str, width: float, dash: str | None = None) -> str:
    extra = f' stroke-dasharray="{dash}"' if dash else ""
    return (f'<path fill="none" stroke="currentColor" stroke-width="{width:g}" stroke-linejoin="round"{extra} '
            f'd="{d}"/>')


def fronds(n: int = 8) -> str:
    """A palm from above: n pointed leaves from the middle."""
    d = ""
    for k in range(n):
        a = math.radians(-90 + k * 360 / n)
        tip = (50 + 47 * math.cos(a), 50 + 47 * math.sin(a))
        l = (50 + 22 * math.cos(a - 0.32), 50 + 22 * math.sin(a - 0.32))
        r = (50 + 22 * math.cos(a + 0.32), 50 + 22 * math.sin(a + 0.32))
        d += (f"M50 50Q{l[0]:.1f} {l[1]:.1f} {tip[0]:.1f} {tip[1]:.1f}Q{r[0]:.1f} {r[1]:.1f} 50 50Z")
    return path(d)


def fruits() -> str:
    return "".join(circle(50 + 26 * math.cos(math.radians(a)), 50 + 26 * math.sin(math.radians(a)), 5.5)
                   for a in (-60, 20, 100, 170, 250))


def flower() -> str:
    petals = "".join(ellipse(50 + 22 * math.cos(math.radians(a)), 50 + 22 * math.sin(math.radians(a)), 18, 13, a)
                     for a in range(-90, 270, 72))
    return f'<g>{petals}</g>' + '<circle cx="50" cy="50" r="11" fill="#F2B705"/>'


def spokes_ring() -> str:
    """A merry-go-round from above: the rim, four spokes, four seats."""
    seats = "".join(circle(50 + 30 * math.cos(math.radians(a)), 50 + 30 * math.sin(math.radians(a)), 8)
                    for a in (45, 135, 225, 315))
    return ring(50, 50, 42, 6) + stroke("M50 8V92M8 50H92", 5) + seats + circle(50, 50, 8)


def arrow_plan(kind: str) -> str:
    """A road arrow as painted on the lane, pointing up the box (its 100 units are its length)."""
    shaft = "M44 96V44"
    head = "M50 4L66 34H54V44H46V34H34Z"
    if kind == "ileri":
        return path("M50 4L66 34H55V96H45V34H34Z")
    if kind == "sag":
        return path("M45 96V52C45 40 52 34 62 34H66V22L86 40L66 58V46H62C58 46 55 49 55 54V96Z")
    if kind == "sol":
        return path("M55 96V52C55 40 48 34 38 34H34V22L14 40L34 58V46H38C42 46 45 49 45 54V96Z")
    # ileri ve sağa
    return path("M45 96V20H38L50 2L62 20H55V54C58 50 62 48 66 48V38L84 54L66 70V60C60 60 55 64 55 70V96Z") + (shaft and "")


PICTOGRAMS.update({
    # Trees and plants, plan view
    "tac": svg(path(scallops(41, 12))),
    "tac-cizgi": svg(outline(scallops(41, 12), 3.5)),
    "tac-kesikli": svg(outline(scallops(41, 12), 3.5, "6 5")),
    "tac-igne": svg(path(star_points(50, 50, 48, 34, 16))),
    "tac-igne-cizgi": svg(outline(star_points(50, 50, 48, 34, 16), 3.5)),
    "palmiye": svg(fronds(9) + circle(50, 50, 6)),
    "meyveler": svg(fruits()),
    "cift-halka": svg(ring(50, 50, 46, 3.5) + ring(50, 50, 40, 3.5)),
    "carpi": svg(stroke("M18 18L82 82M82 18L18 82", 9)),
    "ok-sag": svg(path("M14 42H62V26L90 50L62 74V58H14Z")),
    "servi": svg(ring(50, 50, 44, 5) + stroke("M22 30L70 78M30 22L78 70M18 46L54 82M46 18L82 54", 4)),
    "cali-plan": svg(path(lobes([44, 36, 46, 38, 44, 34, 42]))),
    "cali-plan-cizgi": svg(outline(lobes([44, 36, 46, 38, 44, 34, 42]), 4)),
    "ardic": svg(path(star_points(50, 50, 46, 26, 9))),
    "cicek": svg(flower()),
    "sarilgan": svg(stroke("M46 94C40 78 58 68 50 52C44 40 56 28 64 20", 6)
                    + ellipse(36, 74, 11, 6, -30) + ellipse(62, 60, 11, 6, 30) + ellipse(38, 44, 11, 6, -35)
                    + ellipse(66, 30, 10, 6, 25) + circle(66, 16, 5)),
    "bambu": svg(circle(30, 34, 9) + circle(56, 26, 8) + circle(72, 48, 9) + circle(44, 54, 10) + circle(28, 72, 8)
                 + circle(62, 74, 9)),
    "saksi": svg(path("M24 52H76L70 92H30Z") + ellipse(50, 30, 22, 18) + path("M47 44H53V52H47Z")),
    # Park, garden and street furniture
    "bank": svg(path("M8 46H92V56H8ZM8 24H92V34H8Z") + stroke("M16 56V82M84 56V82M16 34V46M84 34V46", 7, cap="butt")),
    "cop-kutusu": svg(path("M27 28H73L67 92H33ZM20 16H80V24H20ZM42 7H58V16H42Z")
                      + '<path fill="none" stroke="#FFFFFF" stroke-width="0" d="M0 0"/>'),
    "geri-donusum": svg(path("M27 28H73L67 92H33ZM20 16H80V24H20ZM42 7H58V16H42Z"
                             "M50 42L61 60H55L58 66H42L45 60H39Z", "evenodd")),
    "konteyner": svg(path("M10 30H90L84 76H16ZM6 20H94V28H6Z") + circle(28, 84, 7) + circle(72, 84, 7)),
    "fiskiye": svg(path("M8 78A42 12 0 0 0 92 78V84A42 12 0 0 1 8 84Z") + stroke("M50 72V18", 6)
                   + stroke("M50 22C38 22 28 34 26 52M50 22C62 22 72 34 74 52M50 34C42 36 38 44 37 56M50 34C58 36 62 44 63 56", 5)),
    "cesme": svg(path("M40 30H60V92H40ZM34 86H66V94H34Z") + stroke("M60 38H76V48", 7, cap="butt")
                 + path("M76 56C80 62 82 66 82 69A6 6 0 0 1 70 69C70 66 72 62 76 56Z")),
    "pergola": svg(stroke("M12 12H88V88H12ZM12 30H88M12 50H88M12 70H88", 6, cap="butt")),
    "kamelya": svg(path("M50 8L94 40H6Z") + stroke("M16 40V90M84 40V90M38 40V90M62 40V90", 6, cap="butt")
                   + path("M10 86H90V92H10Z")),
    "heykel": svg(circle(50, 14, 9) + path("M38 26H62L58 58H42Z") + path("M24 64H76V72H24ZM30 72H70V92H30Z")),
    "fener": svg(path("M38 10H62L66 16H34ZM38 18H62L58 40H42Z") + path("M46 40H54V86H46ZM34 86H66V94H34Z")),
    "pano": svg(path("M12 14H88V58H12Z") + stroke("M26 58V92M74 58V92", 7, cap="butt")),
    "bisiklet-park": svg(stroke("M22 88V40A14 14 0 0 1 50 40V88M50 88V40A14 14 0 0 1 78 40V88M8 90H92", 7)),
    "kaydirak": svg(stroke("M18 92V22M34 92V22M18 40H34M18 58H34M18 76H34M34 22L88 88", 6)),
    "tahterevalli": svg(path("M40 92L50 66L60 92Z") + stroke("M8 58L92 74", 7) + circle(16, 48, 7) + circle(84, 64, 7)),
    "kum-havuzu": svg(stroke("M10 14H90V86H10Z", 7)
                      + "".join(circle(x, y, 3.2) for x, y in [(26, 30), (42, 26), (58, 32), (74, 28), (30, 46), (48, 44),
                                                               (66, 48), (24, 62), (40, 64), (56, 60), (74, 66), (34, 76),
                                                               (52, 76), (68, 78)])),
    "tirmanma": svg(stroke("M10 90A40 70 0 0 1 90 90M26 90A24 50 0 0 1 74 90M42 90A8 30 0 0 1 58 90M14 66H86M24 46H76M38 30H62", 5)),
    "donence": svg(spokes_ring()),
    "pota": svg(path("M18 8H82V48H18ZM28 16V40H72V16Z", "evenodd") + stroke("M36 48H64M38 50L44 80M62 50L56 80M44 80H56M50 50V80", 4)),
    "futbol-kalesi": svg(stroke("M10 84V20H90V84M10 20L22 36H78L90 20M22 36V84M78 36V84M36 36V84M50 36V84M64 36V84M22 52H78M22 68H78", 4)),
    "masa-tenisi": svg(path("M8 34H92V66H8Z") + stroke("M50 26V74", 6) + path("M76 72A9 9 0 1 1 76 90A9 9 0 1 1 76 72Z")),
    "sokak-lambasi": svg(path("M44 22H52V92H44ZM32 88H64V94H32Z") + stroke("M48 24C48 14 58 10 70 10H78", 6)
                         + path("M70 10H90L86 18H74Z")),
    "sokak-lambasi-cift": svg(path("M46 22H54V92H46ZM34 88H66V94H34Z") + stroke("M50 24C50 14 40 10 28 10H22M50 24C50 14 60 10 72 10H78", 6)
                              + path("M8 10H28L24 18H12ZM72 10H92L88 18H76Z")),
    "projektor": svg(path("M46 50H54V92H46ZM32 88H68V94H32Z") + path("M22 18L60 8L66 38L28 48Z") + stroke("M50 50L46 44", 6)),
    "baba": svg(path("M38 24A12 8 0 0 1 62 24V88H38ZM30 88H70V94H30Z") + path("M38 38H62V44H38Z")),
    "durak-siginagi": svg(path("M4 14H96V24H4ZM10 24H18V92H10ZM82 24H90V92H82Z") + path("M24 30H76V62H24Z")
                          + path("M26 68H74V74H26Z")),
    "bufe": svg(path("M12 30H88V92H12ZM24 42V64H76V42Z", "evenodd") + path("M6 14H94L88 30H12Z")),
    "posta-kutusu": svg(path("M24 18H76A6 6 0 0 1 82 24V62H18V24A6 6 0 0 1 24 18ZM34 30V36H66V30Z", "evenodd")
                        + path("M46 62H54V94H46Z")),
    "atm": svg(path("M8 24H92V76H8ZM8 34V44H92V34Z", "evenodd") + path("M16 60H44V66H16Z")),
    "reklam-panosu": svg(path("M6 14H94V58H6Z") + path("M46 58H54V94H46Z")),
    "yonlendirme": svg(path("M46 8H54V94H46Z") + path("M54 16H82L92 26L82 36H54ZM46 42H18L8 52L18 62H46Z")),
    "saat": svg(ring(50, 50, 40, 8) + stroke("M50 50V24M50 50H68", 7)),
    "kamera": svg(path("M10 30L66 18L72 46L16 58Z") + path("M72 30L90 24V48L76 44Z") + stroke("M40 52L46 86M30 88H62", 6)),
    "parkmetre": svg(path("M30 8H70A6 6 0 0 1 76 14V52H24V14A6 6 0 0 1 30 8ZM36 18V34H64V18Z", "evenodd")
                     + path("M45 52H55V94H45Z")),
    "engelli": svg(circle(46, 12, 8) + stroke("M44 24V52H66L76 76M44 36H62", 8)
                   + '<path fill="none" stroke="currentColor" stroke-width="8" stroke-linecap="round" d="M34 46A22 22 0 1 0 64 74"/>'),
    "wifi": svg(stroke("M14 40A50 50 0 0 1 86 40M26 54A33 33 0 0 1 74 54M38 68A16 16 0 0 1 62 68", 8) + circle(50, 82, 7)),
    # Traffic
    "trafik-isigi": svg(path("M34 4H66A6 6 0 0 1 72 10V90A6 6 0 0 1 66 96H34A6 6 0 0 1 28 90V10A6 6 0 0 1 34 4Z"
                             + disc(50, 22, 10) + disc(50, 50, 10) + disc(50, 78, 10), "evenodd")),
    "yaya": svg(circle(54, 12, 9) + stroke("M50 26L40 56L26 92M50 26L58 58L74 92M46 36L28 48M50 34L70 46", 9)),
    "ok-yukari": svg(path("M50 8L80 44H60V92H40V44H20Z")),
    "zebra": svg(path("M12 10H24V90H12ZM32 10H44V90H32ZM52 10H64V90H52ZM72 10H84V90H72Z")),
    # Road arrows painted on lanes
    "yol-oku-ileri": svg(arrow_plan("ileri")),
    "yol-oku-sag": svg(arrow_plan("sag")),
    "yol-oku-sol": svg(arrow_plan("sol")),
    "yol-oku-ileri-sag": svg(arrow_plan("ileri-sag")),
    # Utility details
    "yagmurlama": svg(path("M44 62H56V92H44ZM36 88H64V94H36Z") + circle(50, 56, 9)
                      + stroke("M36 44C28 36 24 26 24 14M64 44C72 36 76 26 76 14M50 40V8", 5)),
    "sayac": svg(ring(50, 50, 40, 8) + stroke("M50 50L68 30", 7) + circle(50, 50, 7)),
    "pompa": svg(ring(50, 50, 40, 8) + path("M34 28L74 50L34 72Z")),
})


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
           tags: list[str], root: str = "genel") -> None:
    short = {"marker": "nokta", "line": "cizgi", "fill": "alan"}[kind]
    ITEMS.append({"kind": "symbol", "id": f"{root}.{short}.{group}.{slug}", "name": name, "path": path_,
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

    p = group("Ölçme", "Nirengi, poligon, nivelman ve sınır noktaları, balastro; mürekkeple")
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
    # Balastro (docs/adr/9315): the small circle a parcel's corner and break points are drawn with, 0.75 mm across on
    # paper (BÖHHBÜY: kadastral amaçlı ölçülerde parsellerin köşe ve kırık noktaları). Its paper fill hides the lines
    # that meet at the point, as the Turkish map standard shows them; the pen is 0.1 mm (the regulation names none).
    symbol("marker", "olcme", "balastro", "Balastro", p, [shape("circle", 0.75, fill=PAPER, stroke_=INK, width=0.1)],
           ["ölçme", "balastro", "kadastro", "parsel", "köşe noktası", "kırık noktası"])

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


# ── City ────────────────────────────────────────────────────────────────────────────────────────────────────────────

CITY = "Kent"
PARK, FURNITURE, PLAY = "#2E6B30", "#37474F", "#283593"
SIGN_RED, SIGN_BLUE, SIGN_YELLOW = "#D32F2F", "#1565C0", "#F2B705"
MARKING_EDGE = "#5F6368"


def by_field(field: str, default: float, times: float = 1.0) -> dict:
    """A size read from the object's field, in its unit, else `default` (and times a factor)."""
    k = f" * {times:g}" if times != 1.0 else ""
    return {"expr": f"varsayılan(sayı([{field}]), {default:g}){k}"}


TURN = {"expr": "varsayılan(sayı([Açı]), 0)"}


def city() -> None:
    root = [CITY]
    category(root, 4, "Kentin ağaçları, bitkileri, park ve sokak donatıları, trafik işaretleri, yol çizgileri, "
                      "kaplamalar ve altyapı ayrıntıları; MPYY gösterimlerinden ayrı")
    order = iter(range(1, 100))

    def group(name: str, description: str) -> list[str]:
        p = root + [name]
        category(p, next(order), description)
        return p

    def sym(kind: str, grp: str, slug: str, name: str, p: list[str], layers: list[dict], tags: list[str]) -> None:
        symbol(kind, grp, slug, name, p, layers, tags, root="kent")

    def crown(fill_asset: str, line_asset: str | None, fill: str, line: str, default: float) -> list[dict]:
        size = by_field("Taç", default)
        out = [pic(fill_asset, size, fill, unit="m")]
        if line_asset:
            out.append(pic(line_asset, size, line, unit="m"))
        return out

    trunk = shape("circle", 0.45, fill="#5D4037", unit="m")

    # Trees, plan view, at their real size.
    p = group("Ağaçlar", "Plan görünüşünde gerçek boyunda; taç çapı nesnenin Taç alanından (metre), yoksa varsayılan")
    sym("marker", "agac", "yaprakli", "Yapraklı ağaç", p, crown("tac", "tac-cizgi", "#B5DCA0", "#2E7D32", 6) + [trunk],
        ["ağaç", "yapraklı", "peyzaj"])
    sym("marker", "agac", "igne-yaprakli", "İğne yapraklı ağaç", p,
        crown("tac-igne", "tac-igne-cizgi", "#A3CBA8", "#1B5E20", 5) + [trunk], ["ağaç", "iğne yapraklı", "çam"])
    sym("marker", "agac", "palmiye", "Palmiye", p, [pic("palmiye", by_field("Taç", 5), "#3E8E41", unit="m")],
        ["ağaç", "palmiye"])
    sym("marker", "agac", "meyve", "Meyve ağacı", p, crown("tac", "tac-cizgi", "#CFE8A9", "#558B2F", 5)
        + [pic("meyveler", by_field("Taç", 5), "#D84315", unit="m"), trunk], ["ağaç", "meyve", "bahçe"])
    sym("marker", "agac", "servi", "Servi, sütun formlu ağaç", p, [pic("servi", by_field("Taç", 2), "#1B5E20", unit="m")],
        ["ağaç", "servi", "sütun"])
    sym("marker", "agac", "korunacak", "Mevcut ağaç (korunacak)", p, crown("tac", "tac-cizgi", "#B5DCA0", "#2E7D32", 6)
        + [pic("cift-halka", by_field("Taç", 6, 1.15), "#2E7D32", unit="m"), trunk], ["ağaç", "mevcut", "korunacak"])
    sym("marker", "agac", "anit", "Anıt ağaç", p, crown("tac", "tac-cizgi", "#B5DCA0", "#2E7D32", 10)
        + [pic("cift-halka", by_field("Taç", 10, 1.15), "#B71C1C", unit="m"),
           shape("star", 1.6, fill="#F2B705", stroke_="#7A5C00", width=0.08, unit="m")], ["ağaç", "anıt", "tescilli"])
    sym("marker", "agac", "kesilecek", "Kesilecek ağaç", p, [pic("tac-cizgi", by_field("Taç", 6), "#6D4C41", unit="m"),
                                                            pic("carpi", by_field("Taç", 6, 0.8), "#D32F2F", unit="m")],
        ["ağaç", "kesilecek", "sökülecek"])
    sym("marker", "agac", "tasinacak", "Taşınacak ağaç", p, [pic("tac-cizgi", by_field("Taç", 6), "#2E7D32", unit="m"),
                                                            pic("ok-sag", by_field("Taç", 6, 0.6), "#EF6C00", unit="m")],
        ["ağaç", "taşınacak", "nakil"])
    sym("marker", "agac", "dikilecek", "Dikilecek ağaç", p, [pic("tac-kesikli", by_field("Taç", 5), "#2E7D32", unit="m"),
                                                            shape("cross", 1.2, stroke_="#2E7D32", width=0.08, unit="m")],
        ["ağaç", "dikilecek", "yeni"])
    sym("marker", "agac", "fidan", "Fidan", p, crown("tac", "tac-cizgi", "#CFE8A9", "#558B2F", 1.5)
        + [shape("x", 1.6, stroke_="#5D4037", width=0.08, unit="m")], ["ağaç", "fidan"])
    sym("line", "agac", "agac-sirasi", "Ağaç sırası (yol ağacı)", p, [
        mline([pic("tac", 6, "#B5DCA0", unit="m"), pic("tac-cizgi", 6, "#2E7D32", unit="m"), trunk],
              interval=8, offsetAlong=4, unit="m", rotate=False)], ["ağaç", "sıra", "yol ağacı"])
    sym("fill", "agac", "agaclik", "Ağaçlık, koru", p, [
        sfill("#DCEFD2"), pattern([pic("tac", 5, "#A8D594", unit="m"), pic("tac-cizgi", 5, "#4C8C3F", unit="m")],
                                  6, 6, unit="m", stagger=True, jitter=0.3, seed=41), sline("#4C8C3F", 0.25)],
        ["ağaç", "koru", "ağaçlık"])
    sym("fill", "agac", "fidanlik", "Fidanlık", p, [
        sfill("#EEF6E2"), pattern([shape("circle", 1.0, fill=None, stroke_="#558B2F", width=0.08, unit="m")],
                                  2.5, 2.5, unit="m"), sline("#558B2F", 0.25)], ["ağaç", "fidanlık"])

    # Shrubs and plants.
    p = group("Çalı ve bitkiler", "Çalılar, çitler, çiçek tarhları, çim ve yer örtücüler; çalılar gerçek boyunda")
    sym("marker", "bitki", "cali", "Çalı", p, crown("cali-plan", "cali-plan-cizgi", "#C5E1A5", "#558B2F", 1.6),
        ["bitki", "çalı"])
    sym("marker", "bitki", "ardic", "İğne yapraklı çalı, ardıç", p, [pic("ardic", by_field("Taç", 1.4), "#4E7D4F", unit="m")],
        ["bitki", "çalı", "ardıç"])
    sym("marker", "bitki", "cicek", "Süs bitkisi, çiçek", p, [pic("cicek", by_field("Taç", 0.8), "#E91E63", unit="m")],
        ["bitki", "çiçek"])
    sym("marker", "bitki", "sarilgan", "Sarılıcı bitki", p, [pic("sarilgan", 3.2, "#558B2F")], ["bitki", "sarmaşık"])
    sym("marker", "bitki", "bambu", "Bambu", p, [pic("bambu", by_field("Taç", 2), "#7CB342", unit="m")], ["bitki", "bambu"])
    sym("line", "bitki", "cit-bitkisi", "Budanmış çit", p, [
        sline("#33691E", 1.1, unit="m", cap="round", join="round"), sline("#8BC34A", 0.85, unit="m", cap="round",
                                                                             join="round")],
        ["bitki", "çit", "hedge"])
    sym("line", "bitki", "dogal-cit", "Doğal çit", p, [
        sline("#7CB342", 0.9, unit="m", wave={"shape": "sine", "length": 1.6, "amplitude": 0.25, "connect": True})],
        ["bitki", "çit"])
    sym("line", "bitki", "bordur-bitkisi", "Bordür bitkisi", p, [
        mline([shape("circle", 0.5, fill="#8BC34A", stroke_="#558B2F", width=0.05, unit="m")], interval=0.7, unit="m")],
        ["bitki", "bordür"])
    sym("fill", "bitki", "cim", "Çim alan", p, [
        sfill("#CDE8B0"), pattern([pic("ot", 1.2, "#7CB342")], 4, 4, stagger=True, jitter=0.35, seed=43),
        sline("#7CB342", 0.18)], ["bitki", "çim"])
    sym("fill", "bitki", "spor-cimi", "Biçilmiş çim", p, [
        sfill("#A9D88D"), hatch(0, 4, 2, "#98CF79"), sline("#5E9C3F", 0.18)], ["bitki", "çim", "spor"])
    sym("fill", "bitki", "cicek-tarhi", "Çiçek tarhı", p, [
        sfill("#F8D7E3"), pattern([pic("cicek", 1.6, "#D81B60")], 3, 3, stagger=True), sline("#C2185B", 0.2)],
        ["bitki", "çiçek", "tarh"])
    sym("fill", "bitki", "mevsimlik", "Mevsimlik çiçek", p, [
        sfill("#FBE7EE"), pattern([shape("circle", 0.9, fill="#E53935")], 2.4, 2.4, stagger=True),
        pattern([shape("circle", 0.9, fill="#FDD835")], 2.4, 2.4, stagger=True, offset=[1.2, 0]),
        pattern([shape("circle", 0.9, fill="#8E24AA")], 2.4, 2.4, stagger=True, offset=[0.6, 1.2]), sline("#C2185B", 0.2)],
        ["bitki", "çiçek", "mevsimlik"])
    sym("fill", "bitki", "cali-grubu", "Çalı grubu", p, [
        sfill("#D7EBC1"), pattern([pic("cali-plan", 1.6, "#A5CF7E", unit="m"), pic("cali-plan-cizgi", 1.6, "#5B8C2A",
                                                                                   unit="m")],
                                  1.9, 1.9, unit="m", stagger=True, jitter=0.35, seed=47), sline("#5B8C2A", 0.2)],
        ["bitki", "çalı", "kitle"])
    sym("fill", "bitki", "yer-ortucu", "Yer örtücü bitki", p, [
        sfill("#E2F0CF"), pattern([shape("circle", 0.5, fill="#7CB342")], 1.2, 1.2, jitter=0.45, seed=53),
        sline("#7CB342", 0.18)], ["bitki", "yer örtücü"])
    sym("fill", "bitki", "otsu", "Süs çimi, otsu bitkiler", p, [
        sfill("#E6F2D4"), pattern([pic("ot", 1.6, "#6E9F3F")], 2.6, 2.6, stagger=True, jitter=0.3, seed=59),
        sline("#6E9F3F", 0.18)], ["bitki", "süs çimi"])
    sym("fill", "bitki", "kaya-bahcesi", "Kaya bahçesi", p, [
        sfill("#ECE8DF"), pattern([pic("kaya", 1.8, "#8D8578")], 3.2, 3.0, stagger=True, jitter=0.3, seed=61),
        pattern([pic("cicek", 1.2, "#EC407A")], 5, 5, offset=[2, 1]), sline("#8D8578", 0.2)], ["bitki", "kaya bahçesi"])

    # Park and garden.
    p = group("Park ve bahçe donatıları", "Fıskiye, çeşme, pergola, çardak, heykel ve park zeminleri")
    for slug, name, asset, color, size, tags in [
        ("fiskiye", "Süs havuzu, fıskiye", "fiskiye", "#0277BD", 3.4, ["park", "havuz", "fıskiye"]),
        ("cesme", "İçme suyu çeşmesi", "cesme", "#0277BD", 3.2, ["park", "çeşme", "su"]),
        ("pergola", "Pergola", "pergola", "#6D4C41", 3.2, ["park", "pergola"]),
        ("kamelya", "Çardak, kamelya", "kamelya", "#6D4C41", 3.4, ["park", "çardak", "kamelya"]),
        ("heykel", "Heykel", "heykel", "#5D6B76", 3.4, ["park", "heykel", "anıt"]),
        ("piknik", "Piknik masası", "piknik", PARK, 3.2, ["park", "piknik", "masa"]),
        ("bayrak", "Bayrak direği", "bayrak", "#5D6B76", 3.4, ["park", "bayrak"]),
        ("park-lambasi", "Park lambası", "fener", "#455A64", 3.4, ["park", "aydınlatma", "lamba"]),
        ("pano", "Bilgi panosu", "pano", PARK, 3.0, ["park", "pano", "bilgi"]),
        ("kopek-parki", "Köpek parkı", "pati", PARK, 3.0, ["park", "köpek", "evcil hayvan"]),
    ]:
        sym("marker", "park", slug, name, p, [pic(asset, size, color)], tags)
    sym("marker", "park", "wc", "Tuvalet (WC)", p, [shape("square", 3.4, fill="#455A64", stroke_=WHITE, width=0.25),
                                                   text("WC", 1.7, WHITE)], ["park", "tuvalet", "wc"])
    sym("fill", "park", "sus-havuzu", "Süs havuzu", p, [sfill("#B3E5FC"), sline("#0277BD", 0.3), sline("#81D4FA", 0.15,
                                                                                                    offset=0.5)],
        ["park", "havuz", "su"])
    sym("fill", "park", "cakil", "Çakıl", p, [
        sfill("#E8E2D6"), pattern([shape("circle", 0.6, fill=None, stroke_="#A39A8A", width=0.1)], 1.1, 1.1, jitter=0.4,
                                  seed=67), sline("#A39A8A", 0.18)], ["park", "çakıl", "zemin"])
    sym("fill", "park", "ahsap-deck", "Ahşap deck", p, [sfill("#D7B98E"), hatch(0, 1.2, 0.12, "#A1784C"),
                                                        sline("#8D6E4A", 0.25)], ["park", "ahşap", "zemin"])
    sym("fill", "park", "stabilize", "Stabilize park yolu", p, [
        sfill("#EADFC6"), pattern([shape("circle", 0.25, fill="#B59E78")], 1.0, 1.0, jitter=0.45, seed=71),
        sline("#B59E78", 0.2)], ["park", "yol", "stabilize"])

    # Play and sport.
    p = group("Oyun ve spor elemanları", "Çocuk oyun elemanları, açık hava spor aletleri ve zeminleri")
    for slug, name, asset, tags in [
        ("salincak", "Salıncak", "salincak", ["oyun", "salıncak"]),
        ("kaydirak", "Kaydırak", "kaydirak", ["oyun", "kaydırak"]),
        ("tahterevalli", "Tahterevalli", "tahterevalli", ["oyun", "tahterevalli"]),
        ("kum-havuzu", "Kum havuzu", "kum-havuzu", ["oyun", "kum"]),
        ("tirmanma", "Tırmanma elemanı", "tirmanma", ["oyun", "tırmanma"]),
        ("donence", "Dönence", "donence", ["oyun", "dönence"]),
        ("fitness", "Açık hava spor aleti", "dambil", ["spor", "fitness"]),
        ("pota", "Basketbol potası", "pota", ["spor", "basketbol"]),
        ("futbol-kalesi", "Futbol kalesi", "futbol-kalesi", ["spor", "futbol"]),
        ("masa-tenisi", "Masa tenisi", "masa-tenisi", ["spor", "masa tenisi"]),
    ]:
        sym("marker", "oyun", slug, name, p, [pic(asset, 3.4, PLAY)], tags)
    sym("fill", "oyun", "kaucuk-zemin", "Kauçuk zemin", p, [
        sfill("#EBA8A0"), pattern([shape("circle", 0.3, fill="#B23A2E")], 1.0, 1.0, jitter=0.45, seed=73),
        sline("#B23A2E", 0.25)], ["oyun", "zemin", "kauçuk"])
    sym("fill", "oyun", "kum-zemin", "Kum zemin", p, [
        sfill("#F3E3B5"), pattern([shape("circle", 0.28, fill="#C9A86A")], 0.9, 0.9, jitter=0.45, seed=79),
        sline("#C9A86A", 0.25)], ["oyun", "zemin", "kum"])
    sym("fill", "oyun", "tartan", "Tartan pist", p, [sfill("#C8553D"), sline("#8E3A28", 0.25)], ["spor", "pist", "tartan"])
    sym("fill", "oyun", "sentetik-cim", "Sentetik çim saha", p, [sfill("#6DBE5A"), hatch(90, 5, 2.5, "#62B04F"),
                                                                 sline("#3E8E41", 0.25)], ["spor", "saha", "çim"])

    # Street furniture.
    p = group("Kent mobilyaları", "Oturma, aydınlatma, atık, durak, büfe, yönlendirme ve erişim donatıları")
    for slug, name, asset, color, tags in [
        ("bank", "Bank", "bank", FURNITURE, ["mobilya", "bank", "oturma"]),
        ("cop-kutusu", "Çöp kutusu", "cop-kutusu", FURNITURE, ["mobilya", "çöp", "atık"]),
        ("geri-donusum", "Geri dönüşüm kutusu", "geri-donusum", "#2E7D32", ["mobilya", "geri dönüşüm", "atık"]),
        ("konteyner", "Çöp konteyneri", "konteyner", FURNITURE, ["mobilya", "konteyner", "atık"]),
        ("sokak-lambasi", "Sokak lambası", "sokak-lambasi", FURNITURE, ["mobilya", "aydınlatma", "lamba"]),
        ("sokak-lambasi-cift", "Sokak lambası (çift kollu)", "sokak-lambasi-cift", FURNITURE,
         ["mobilya", "aydınlatma", "lamba"]),
        ("projektor", "Projektör", "projektor", FURNITURE, ["mobilya", "aydınlatma", "projektör"]),
        ("baba", "Babası (bolard)", "baba", FURNITURE, ["mobilya", "babası", "bolard"]),
        ("saksi", "Çiçeklik, saksı", "saksi", "#6D4C41", ["mobilya", "saksı", "çiçeklik"]),
        ("durak", "Otobüs durağı (sığınak)", "durak-siginagi", TRANSPORT, ["mobilya", "durak", "otobüs"]),
        ("bisiklet-park", "Bisiklet park yeri", "bisiklet-park", TRANSPORT, ["mobilya", "bisiklet"]),
        ("bufe", "Büfe, kiosk", "bufe", COMMERCE, ["mobilya", "büfe", "kiosk"]),
        ("posta-kutusu", "Posta kutusu", "posta-kutusu", "#C99700", ["mobilya", "posta"]),
        ("atm", "ATM", "atm", COMMERCE, ["mobilya", "atm", "banka"]),
        ("reklam-panosu", "Reklam panosu", "reklam-panosu", FURNITURE, ["mobilya", "reklam", "pano"]),
        ("yonlendirme", "Yönlendirme tabelası", "yonlendirme", FURNITURE, ["mobilya", "tabela", "yönlendirme"]),
        ("saat", "Meydan saati", "saat", FURNITURE, ["mobilya", "saat"]),
        ("kamera", "Güvenlik kamerası", "kamera", FURNITURE, ["mobilya", "kamera", "güvenlik"]),
        ("parkmetre", "Otopark ödeme makinesi", "parkmetre", TRANSPORT, ["mobilya", "otopark", "ödeme"]),
        ("engelli", "Engelli erişimi, rampa", "engelli", SIGN_BLUE, ["mobilya", "engelli", "erişim", "rampa"]),
        ("wifi", "Kablosuz internet", "wifi", FURNITURE, ["mobilya", "internet", "wifi"]),
    ]:
        sym("marker", "mobilya", slug, name, p, [pic(asset, 3.2, color)], tags)
    sym("marker", "mobilya", "bilgi-ekrani", "Kent bilgi ekranı", p, letter("i", TOURISM, size=3.6, kind="square",
                                                                           text_size=2.6, font="serif"),
        ["mobilya", "bilgi", "totem"])
    sym("line", "mobilya", "babalar", "Babası dizisi", p, [
        mline([shape("circle", 0.3, fill=FURNITURE, unit="m")], interval=1.5, offsetAlong=0.75, unit="m")],
        ["mobilya", "babası", "bolard"])
    sym("line", "mobilya", "korkuluk", "Yaya korkuluğu", p, [
        sline(FURNITURE, 0.15, offset=0.25), sline(FURNITURE, 0.15, offset=-0.25),
        mline([shape("circle", 0.7, fill=FURNITURE)], interval=3, offsetAlong=1.5)], ["mobilya", "korkuluk"])
    sym("line", "mobilya", "bariyer", "Bariyer", p, [sline(SIGN_RED, 0.6), sline(WHITE, 0.6, dash=[1.5, 1.5])],
        ["mobilya", "bariyer"])

    # Traffic signs and lights.
    p = group("Trafik işaretleri ve ışıkları", "Uyarı, tanzim ve bilgi işaretleri, trafik ve yaya ışıkları")
    light = lambda color, dy: shape("circle", 0.85, fill=color, offset=[0, dy])
    sym("marker", "trafik", "trafik-isigi", "Trafik ışığı", p, [
        pic("trafik-isigi", 4.2, "#263238"), light("#E53935", 1.18), light("#FDD835", 0), light("#43A047", -1.18)],
        ["trafik", "ışık", "sinyalizasyon"])
    sym("marker", "trafik", "yaya-isigi", "Yaya ışığı", p, [
        shape("square", 3.2, fill="#263238", stroke_=WHITE, width=0.2), pic("yaya", 2.4, "#43A047")],
        ["trafik", "ışık", "yaya"])
    sym("marker", "trafik", "uyari", "Uyarı işareti", p, [
        shape("triangle", 4.6, fill=WHITE, stroke_=SIGN_RED, width=0.55), text("!", 2.6, INK, offset=[0, -0.35])],
        ["trafik", "işaret", "uyarı"])
    sym("marker", "trafik", "yaya-gecidi-uyari", "Yaya geçidi uyarısı", p, [
        shape("triangle", 4.6, fill=WHITE, stroke_=SIGN_RED, width=0.55), pic("zebra", 1.7, INK, offset=[0, -0.45])],
        ["trafik", "işaret", "uyarı", "yaya"])
    sym("marker", "trafik", "tanzim", "Tanzim işareti", p, [shape("circle", 4.2, fill=WHITE, stroke_=SIGN_RED, width=0.6)],
        ["trafik", "işaret", "tanzim", "yasak"])
    sym("marker", "trafik", "hiz-siniri", "Hız sınırı", p, [
        shape("circle", 4.4, fill=WHITE, stroke_=SIGN_RED, width=0.65),
        text({"expr": "varsayılan([Hız], '50')"}, 1.9, INK)], ["trafik", "işaret", "hız"])
    sym("marker", "trafik", "dur", "Dur", p, [shape("octagon", 4.4, fill=SIGN_RED, stroke_=WHITE, width=0.3),
                                             text("DUR", 1.4, WHITE)], ["trafik", "işaret", "dur"])
    sym("marker", "trafik", "yol-ver", "Yol ver", p, [
        shape("triangle", 4.6, fill=WHITE, stroke_=SIGN_RED, width=0.6, rotation=180)], ["trafik", "işaret", "yol ver"])
    sym("marker", "trafik", "park-yapilmaz", "Park yapılmaz", p, [
        shape("circle", 4.2, fill=SIGN_BLUE, stroke_=SIGN_RED, width=0.6),
        shape("line", 3.6, stroke_=SIGN_RED, width=0.55, rotation=45)], ["trafik", "işaret", "park"])
    sym("marker", "trafik", "mecburi-yon", "Mecburi yön", p, [
        shape("circle", 4.2, fill=SIGN_BLUE, stroke_=WHITE, width=0.2), pic("ok-yukari", 2.6, WHITE)],
        ["trafik", "işaret", "mecburiyet"])
    sym("marker", "trafik", "bilgi", "Bilgi işareti", p, [
        shape("square", 4.0, fill=SIGN_BLUE, stroke_=WHITE, width=0.2), text("i", 2.8, WHITE, font="serif")],
        ["trafik", "işaret", "bilgi"])
    sym("marker", "trafik", "yaya-gecidi", "Yaya geçidi işareti", p, [
        shape("square", 4.0, fill=SIGN_BLUE, stroke_=WHITE, width=0.2),
        shape("triangle", 3.0, fill=WHITE, offset=[0, -0.15]), pic("yaya", 1.6, INK, offset=[0, -0.3])],
        ["trafik", "işaret", "yaya geçidi"])
    sym("marker", "trafik", "ana-yol", "Ana yol", p, [
        shape("diamond", 4.6, fill=WHITE, stroke_=INK, width=0.15), shape("diamond", 3.2, fill=SIGN_YELLOW)],
        ["trafik", "işaret", "ana yol"])

    # Road markings and edges, at their real size.
    p = group("Yol çizgileri ve kenarları", "Şerit, durma ve yaya geçidi çizgileri, bordür, özel şeritler ve yol okları; "
                                            "gerçek boyunda")
    white = lambda width, **extra: [sline(MARKING_EDGE, width + 0.06, unit="m", **extra),
                                    sline(WHITE, width, unit="m", **extra)]
    sym("line", "yol", "serit-kesikli", "Şerit çizgisi (kesikli)", p, white(0.15, dash=[3, 5]),
        ["yol çizgisi", "şerit"])
    sym("line", "yol", "serit-surekli", "Şerit çizgisi (sürekli)", p, white(0.15), ["yol çizgisi", "şerit"])
    sym("line", "yol", "cift-surekli", "Çift sürekli çizgi", p, white(0.12, offset=0.15) + white(0.12, offset=-0.15),
        ["yol çizgisi", "orta çizgi"])
    sym("line", "yol", "gecici-serit", "Geçici şerit çizgisi (sarı)", p, [
        sline("#C79A00", 0.19, unit="m", dash=[3, 3]), sline("#FBC02D", 0.15, unit="m", dash=[3, 3])],
        ["yol çizgisi", "geçici", "yol çalışması"])
    sym("line", "yol", "durma-cizgisi", "Durma çizgisi", p, white(0.5), ["yol çizgisi", "durma"])
    sym("line", "yol", "yaya-gecidi", "Yaya geçidi (çizgi boyunca)", p, [
        mline([shape("rectangle", 0.5, fill=WHITE, stroke_=MARKING_EDGE, width=0.03, height=3.0, unit="m")],
              interval=1.0, offsetAlong=0.5, unit="m")], ["yol çizgisi", "yaya geçidi", "zebra"])
    sym("line", "yol", "bordur", "Bordür", p, [sline("#9E9E9E", 0.15, unit="m"), sline("#616161", 0.03, unit="m",
                                                                                     offset=0.075),
                                              sline("#616161", 0.03, unit="m", offset=-0.075)], ["yol", "bordür", "kaldırım"])
    sym("line", "yol", "bisiklet-seridi", "Bisiklet şeridi", p, [
        sline("#4CAF5066", 1.5, unit="m"), *white(0.1, offset=0.75), *white(0.1, offset=-0.75),
        mline([pic("bisiklet", 1.0, "#2E7D32", unit="m")], interval=20, offsetAlong=10, unit="m", rotate=False)],
        ["yol", "bisiklet", "şerit"])
    sym("line", "yol", "otobus-seridi", "Otobüs şeridi", p, [
        sline("#E5393559", 3.0, unit="m"),
        mline([text("OTOBÜS", 0.9, "#B71C1C", unit="m")], interval=30, offsetAlong=15, unit="m")],
        ["yol", "otobüs", "şerit"])
    sym("line", "yol", "hiz-kesici", "Hız kesici", p, [sline("#212121", 0.6, unit="m"),
                                                      sline("#FDD835", 0.6, unit="m", dash=[0.5, 0.5])],
        ["yol", "hız kesici", "kasis"])
    for slug, name, asset in [("ok-ileri", "Yol oku (ileri)", "yol-oku-ileri"), ("ok-sag", "Yol oku (sağa)", "yol-oku-sag"),
                              ("ok-sol", "Yol oku (sola)", "yol-oku-sol"),
                              ("ok-ileri-sag", "Yol oku (ileri ve sağa)", "yol-oku-ileri-sag")]:
        sym("marker", "yol", slug, name, p, [pic(asset, 5.25, MARKING_EDGE, unit="m", rotation=TURN),
                                            pic(asset, 5.0, WHITE, unit="m", rotation=TURN)], ["yol çizgisi", "ok"])

    # Paving.
    p = group("Yol ve zemin kaplamaları", "Asfalt, beton, parke, küp taş, kaldırım, refüj, park cebi ve çim taşı")
    sym("fill", "kaplama", "asfalt", "Asfalt", p, [sfill("#7A7D80"), sline("#5F6265", 0.18)], ["kaplama", "asfalt", "yol"])
    sym("fill", "kaplama", "beton", "Beton", p, [
        sfill("#D5D5D2"), pattern([shape("circle", 0.22, fill="#A8A8A3")], 1.0, 1.0, jitter=0.45, seed=83),
        sline("#9E9E9A", 0.18)], ["kaplama", "beton"])
    sym("fill", "kaplama", "kilit-parke", "Kilit parke taşı", p, [
        sfill("#C9B8A6"), hatch(0, 1.0, 0.08, "#9C8774"), hatch(90, 2.0, 0.08, "#9C8774", dash=[1, 1]),
        hatch(90, 2.0, 0.08, "#9C8774", dash=[1, 1], dashOffset=1, offset=1), sline("#8C7764", 0.2)],
        ["kaplama", "parke", "kilit taşı"])
    sym("fill", "kaplama", "kup-tas", "Küp taş, Arnavut kaldırımı", p, [
        sfill("#BDB8B0"), hatch(0, 0.8, 0.08, "#8E8A84"), hatch(90, 0.8, 0.08, "#8E8A84"), sline("#7E7A74", 0.2)],
        ["kaplama", "küp taş"])
    sym("fill", "kaplama", "kaldirim", "Kaldırım karosu", p, [
        sfill("#E4DED3"), hatch(0, 1.6, 0.08, "#B8B0A3"), hatch(90, 1.6, 0.08, "#B8B0A3"), sline("#9E9E9E", 0.25)],
        ["kaplama", "kaldırım", "yaya"])
    sym("fill", "kaplama", "bisiklet-yolu", "Bisiklet yolu kaplaması", p, [sfill("#C86B5A"), sline("#8E3A28", 0.2)],
        ["kaplama", "bisiklet"])
    sym("fill", "kaplama", "refuj", "Refüj, kavşak adası", p, [sfill("#B7DFA0"), sline("#7A7A7A", 0.3)],
        ["kaplama", "refüj", "ada"])
    sym("fill", "kaplama", "park-cebi", "Park cebi", p, [
        sfill("#B0B3B6"), hatch(90, 2.5, 0.12, WHITE, unit="m"), sline("#7A7D80", 0.2)], ["kaplama", "otopark", "park cebi"])
    sym("fill", "kaplama", "engelli-park", "Engelli park yeri", p, [
        sfill("#5C8FD6"), sline("#FFFFFF", 0.25), centroid([pic("engelli", 3.0, WHITE)])],
        ["kaplama", "otopark", "engelli"])
    sym("fill", "kaplama", "tarali", "Taralı alan (yol çizgisi)", p, [
        hatch(45, 1.5, 0.36, MARKING_EDGE, unit="m"), hatch(45, 1.5, 0.3, WHITE, unit="m"),
        sline(MARKING_EDGE, 0.19, unit="m"), sline(WHITE, 0.15, unit="m")], ["yol çizgisi", "taralı alan"])
    sym("fill", "kaplama", "cim-tasi", "Çim taşı", p, [
        sfill("#A5D68A"), hatch(0, 1.2, 0.18, "#A3A3A0"), hatch(90, 1.2, 0.18, "#A3A3A0"), sline("#7A7A77", 0.2)],
        ["kaplama", "çim taşı", "otopark"])
    sym("fill", "kaplama", "dogal-tas", "Doğal taş döşeme", p, [
        sfill("#CFC6B8"), pattern([pic("kaya", 2.2, "#9C9284")], 2.6, 2.4, stagger=True, jitter=0.25, seed=89),
        sline("#8C8274", 0.2)], ["kaplama", "doğal taş", "kayrak"])
    sym("fill", "kaplama", "toprak", "Toprak, mıcır", p, [
        sfill("#D9C7A5"), pattern([shape("circle", 0.3, fill="#A88C5E")], 1.6, 1.6, jitter=0.45, seed=97),
        sline("#A88C5E", 0.2)], ["kaplama", "toprak", "mıcır"])

    # Utility details.
    p = group("Altyapı ayrıntıları", "Bacalar, vanalar, sayaçlar, panolar, sulama ve drenaj; altyapı renklerinde")
    sym("marker", "altyapi", "yagmur-suyu-bacasi", "Yağmur suyu bacası", p, [
        shape("circle", 2.2, fill=PAPER, stroke_=STORM, width=0.3), shape("circle", 0.7, fill=STORM)],
        ["altyapı", "yağmur suyu", "baca"])
    sym("marker", "altyapi", "muayene-bacasi", "Muayene bacası", p, [
        shape("circle", 2.2, fill=PAPER, stroke_="#757575", width=0.3), shape("cross", 1.2, stroke_="#757575", width=0.2)],
        ["altyapı", "baca", "menhol"])
    sym("marker", "altyapi", "vana-odasi", "Vana odası", p, [
        shape("square", 3.0, fill=PAPER, stroke_=DRINKING, width=0.3), pic("vana", 2.2, DRINKING)],
        ["altyapı", "su", "vana"])
    sym("marker", "altyapi", "su-sayaci", "Su sayacı", p, [pic("sayac", 2.8, DRINKING)], ["altyapı", "su", "sayaç"])
    sym("marker", "altyapi", "sulama-vanasi", "Sulama vanası", p, [pic("vana", 2.8, "#2E7D32")],
        ["altyapı", "sulama", "vana"])
    sym("marker", "altyapi", "yagmurlama", "Yağmurlama başlığı", p, [pic("yagmurlama", 3.0, "#0288D1")],
        ["altyapı", "sulama", "yağmurlama"])
    sym("marker", "altyapi", "elektrik-panosu", "Elektrik dağıtım panosu", p, [
        shape("rectangle", 3.0, fill=PAPER, stroke_=ELECTRIC, width=0.3, height=2.0), pic("simsek", 1.6, ELECTRIC)],
        ["altyapı", "elektrik", "pano"])
    sym("marker", "altyapi", "aydinlatma-panosu", "Aydınlatma panosu", p, [
        shape("rectangle", 3.0, fill=PAPER, stroke_="#EF6C00", width=0.3, height=2.0), pic("simsek", 1.6, "#EF6C00")],
        ["altyapı", "aydınlatma", "pano"])
    sym("marker", "altyapi", "telekom-menholu", "Haberleşme menholü", p, [
        shape("square", 2.6, fill=PAPER, stroke_=TELECOM, width=0.3), text("T", 1.6, TELECOM)],
        ["altyapı", "haberleşme", "menhol"])
    sym("marker", "altyapi", "fiber-kutusu", "Fiber dağıtım kutusu", p, [
        shape("square", 2.2, fill=TELECOM, stroke_=WHITE, width=0.15), text("F", 1.4, WHITE)],
        ["altyapı", "haberleşme", "fiber"])
    sym("marker", "altyapi", "dogalgaz-vanasi", "Doğal gaz vanası", p, [pic("vana", 2.8, GAS)],
        ["altyapı", "doğal gaz", "vana"])
    sym("marker", "altyapi", "pompa-istasyonu", "Pompa istasyonu", p, [pic("pompa", 3.0, DRINKING)],
        ["altyapı", "su", "pompa"])
    sym("line", "altyapi", "drenaj", "Drenaj hattı", p, [
        sline(STORM, 0.3, dash=[2, 1]), mline([shape("chevron", 1.0, stroke_=STORM, width=0.25)], interval=8, offsetAlong=4)],
        ["altyapı", "drenaj"])
    sym("line", "altyapi", "damlama", "Damla sulama hattı", p, [sline("#2E7D32", 0.25, dash=[0.01, 0.8], cap="round")],
        ["altyapı", "sulama", "damlama"])
    sym("line", "altyapi", "kablo-kanali", "Kablo kanalı", p, [
        sline("#616161", 0.18, offset=0.35), sline("#616161", 0.18, offset=-0.35),
        mline([shape("line", 0.7, stroke_="#616161", width=0.15, rotation=90)], interval=3)], ["altyapı", "kablo"])
    sym("line", "altyapi", "menfez", "Menfez", p, [
        sline(INK, 0.4), mline([shape("line", 2.4, stroke_=INK, width=0.4, rotation=90)], placement="first"),
        mline([shape("line", 2.4, stroke_=INK, width=0.4, rotation=90)], placement="last")], ["altyapı", "menfez", "su"])
    for slug, name, color, mark, tags in [
        ("aydinlatma-kablosu", "Aydınlatma kablosu", "#F9A825", "A", ["altyapı", "aydınlatma", "kablo"]),
        ("bolgesel-isitma", "Bölgesel ısıtma hattı", "#8E2424", "I", ["altyapı", "ısıtma"]),
    ]:
        sym("line", "altyapi", slug, name, p, [
            sline(color, 0.4, dash=[5, 1.4]),
            mline([text(mark, 1.8, color, halo={"color": PAPER, "width": 0.35})], interval=18, offsetAlong=9)], tags)
    sym("line", "altyapi", "basincli-atik-su", "Basınçlı atık su hattı", p, [
        sline(SEWER, 0.45, dash=[3, 1]), mline([shape("arrowhead", 1.1, fill=SEWER)], interval=12, offsetAlong=6)],
        ["altyapı", "atık su", "basınçlı"])


def library() -> dict:
    CATEGORIES.clear()
    ITEMS.clear()
    category([GEN], 3, "Her türden harita için kategorili nokta, çizgi ve alan sembolleri; ölçüler kâğıt milimetresidir")
    points()
    lines()
    areas()
    city()
    category([GEN, "Piktogramlar"], 9, "Genel ve Kent sembollerinin piktogramları (SVG)")
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
