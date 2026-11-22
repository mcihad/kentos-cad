"""KentOS's BÖHHBÜY library (docs/adr/9120): the signs of Ek-3 “Harita özel işaretleri” of the Büyük Ölçekli Harita ve
Harita Bilgileri Üretim Yönetmeliği (RG 26.06.2018/30460 Mük.), sign by sign, written as
crates/native/style/assets/bohhbuy-library.kstil, the format the desktop's style files read. The desktop ships it in the
system library beside Temel, MPYY, Genel and Kent (crates/native/style/src/system.rs): read-only, copyable.

    python3 scripts/style/bohhbuy_library.py           # writes the file
    python3 scripts/style/bohhbuy_library.py --check   # fails when the file is not what this script writes
    python3 scripts/style/bohhbuy_library.py --svg out.json

Read from the Gazette's own scans (400 dpi, pages 271–321 of the issue; the consolidated copy's scans are too coarse):
- each symbol is the sign's 1/500–1/2000 drawing, its sizes in paper millimetres as the table writes them; the 1/5000
  drawing's thinner lines are said in the description where they differ;
- a size the table does not write is measured on the drawing against one it writes (the description says so);
- the regulation gives no colours: every sign is drawn in ink, black on paper;
- the sign's number leads its name (“0201 Sert satıhlı yol”), its id is `bohhbuy.<number>`, its category is its class.
"""

from __future__ import annotations

import importlib.util
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "crates/native/style/assets/bohhbuy-library.kstil"

_spec = importlib.util.spec_from_file_location("general_library", Path(__file__).with_name("general_library.py"))
G = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(G)
shape, sline, mline, hatch, pattern, sfill, centroid, text, beside, numbered = (
    G.shape, G.sline, G.mline, G.hatch, G.pattern, G.sfill, G.centroid, G.text, G.beside, G.numbered)
svg, path, stroke, circle, ring, ellipse, disc = G.svg, G.path, G.stroke, G.circle, G.ring, G.ellipse, G.disc

INK, PAPER = "ink", "paper"
BRANCH = "BÖHHBÜY"
CLASSES = [
    ("0000", "Eşyükseklik eğrileri ve kod numaraları"),
    ("0200", "Karayolları ve tesisleri"),
    ("0400", "Demiryolu, metro, tramvay ve tesisleri"),
    ("0600", "Hidrografik bilgiler"),
    ("0800", "Nakil hatları ve haberleşme tesisleri"),
    ("1000", "Sınırlar"),
    ("1200", "Yapılar"),
    ("1400", "Bitki örtüsü ve tarım alanları"),
    ("1600", "Taşlık ve kayalıklar"),
    ("1800", "Şevler"),
    ("2000", "Tamamlayıcı işaretler"),
    ("2400", "Kontrol noktaları işaretleri"),
    ("2600", "Yazı ve rakam işaretleri"),
]
# The scans are pages 83–133 of the Gazette's PDF; the issue's own page number is 188 more.
GAZETTE = 188

PICTOGRAMS: dict[str, str] = {}
ITEMS: list[dict] = []


def class_of(number: str) -> tuple[str, str]:
    head = number[:2] + "00"
    for code, name in reversed(CLASSES):
        if head >= code:
            return code, name
    raise KeyError(number)


def sign(number: str, name: str, kind: str, layers: list[dict], pdf_page: int, description: str,
         tags: list[str] | None = None) -> None:
    """One sign of Ek-3: its number and name (as the table writes it, in sentence case), its kind (Ç line, N point,
    A area, Y text), its layers, the PDF page it is read from and what was read or measured."""
    code, cls = class_of(number)
    symbol_kind = {"Ç": "line", "N": "marker", "A": "fill", "Y": "marker", "YÇ": "line"}[kind]
    ITEMS.append({
        "kind": "symbol", "id": f"bohhbuy.{number}", "name": f"{number} {name}", "path": [BRANCH, f"{code} {cls}"],
        "symbol": {"type": symbol_kind, "layers": numbered(layers)},
        "description": description,
        "reference": f"BÖHHBÜY Ek-3, RG 30460 Mük. s. {pdf_page + GAZETTE}",
        "tags": ["böhhbüy", number] + (tags or []),
    })


def bpattern(marker: list[dict], sx: float, sy: float, **extra) -> dict:
    """A pattern fill of `marker`. A pattern's tile places a mark at its offset and turns it there; point and line
    markers turn the offset with the mark (ADR 9118 §5). The layers here are written for the latter (`seg`, `beside`),
    so a turned layer's offset is turned back to where the mark lands."""
    flat = []
    for layer in marker:
        r, o = layer.get("rotation"), layer.get("offset")
        if isinstance(r, (int, float)) and r and o:
            a = math.radians(r)
            layer = dict(layer, offset=[round(o[0] * math.cos(a) - o[1] * math.sin(a), 4),
                                        round(o[0] * math.sin(a) + o[1] * math.cos(a), 4)])
        flat.append(layer)
    return pattern(flat, sx, sy, **extra)


def mm(x: float) -> str:
    """A length as Turkish writes it: 1,5."""
    return f"{x:g}".replace(".", ",")


def outline(width: float, **extra) -> dict:
    return sline(INK, width, **extra)


def ticks(length: float, interval: float, width: float, *, above: bool = False, **extra) -> dict:
    """Ticks across a line, or standing on its upper side."""
    offset = G.beside(0, length / 2, 90) if above else None
    marker = shape("line", length, stroke_=INK, width=width, rotation=90, **({"offset": offset} if offset else {}))
    return mline([marker], interval=interval, **extra)


def ends(length: float, angle: float, width: float) -> list[dict]:
    """Both ends of a line bent off to its upper side at `angle` degrees from its run (a bridge's wings)."""
    a = math.radians(angle)
    dx, dy = math.cos(a) * length / 2, math.sin(a) * length / 2
    first = shape("line", length, stroke_=INK, width=width, rotation=angle, offset=G.beside(dx, dy, angle))
    last = shape("line", length, stroke_=INK, width=width, rotation=180 - angle,
                 offset=G.beside(-dx, dy, 180 - angle))
    return [mline([first], placement="first"), mline([last], placement="last")]


def seg(x0: float, y0: float, x1: float, y1: float, width: float = 0.16) -> dict:
    """A straight stroke of a point sign from (x0, y0) to (x1, y1), millimetres from its point, y up."""
    dx, dy = x1 - x0, y1 - y0
    turn = round(math.degrees(math.atan2(dy, dx)), 4)
    return shape("line", round(math.hypot(dx, dy), 4), stroke_=INK, width=width, rotation=turn,
                 offset=G.beside((x0 + x1) / 2, (y0 + y1) / 2, turn))


def polyline(points: list[tuple[float, float]], width: float = 0.16) -> list[dict]:
    return [seg(*a, *b, width=width) for a, b in zip(points, points[1:])]


def dot(d: float = 0.3, x: float = 0.0, y: float = 0.0) -> dict:
    return shape("circle", d, fill=INK, **({"offset": [x, y]} if x or y else {}))


def ring_(d: float, width: float = 0.16, x: float = 0.0, y: float = 0.0, fill: str | None = PAPER) -> dict:
    return shape("circle", d, fill=fill, stroke_=INK, width=width, **({"offset": [x, y]} if x or y else {}))


class Box:
    """Millimetres of a pictogram drawn `size` mm wide, y up, as viewBox units. The box's middle is the point
    (`cx`, `cy`) of the sign's own drawing; its layer is moved by that much to put the drawing's origin on the point."""

    def __init__(self, size: float, cx: float = 0.0, cy: float = 0.0) -> None:
        self.k = 100.0 / size
        self.cx, self.cy = cx, cy

    def p(self, x: float, y: float) -> str:
        return f"{50 + (x - self.cx) * self.k:.2f} {50 - (y - self.cy) * self.k:.2f}"

    def tip(self, x: float, y: float, dx: float, dy: float, length: float, width: float) -> str:
        """A filled arrowhead with its tip at (x, y), pointing along (dx, dy)."""
        n = math.hypot(dx, dy)
        ux, uy = dx / n, dy / n
        bx, by = x - ux * length, y - uy * length
        hx, hy = -uy * width / 2, ux * width / 2
        return path(f"M{self.p(x, y)}L{self.p(bx + hx, by + hy)}L{self.p(bx - hx, by - hy)}Z")

    def w(self, mm: float) -> float:
        return round(mm * self.k, 2)

    def r(self, mm: float) -> str:
        return f"{mm * self.k:.2f}"


def bpic(name: str, size: float, **extra) -> dict:
    if name not in PICTOGRAMS:
        raise KeyError(f"no pictogram {name}")
    return {"type": "svg", "asset": f"bohhbuy.svg.{name}", "size": size, "fill": INK, **extra}


def head(size: float = 1.2, **extra) -> dict:
    """An arrowhead whose tip is at its place (the shape's tip is half its length ahead of its centre)."""
    return shape("arrowhead", size, fill=INK, offset=[-size / 2, 0], **extra)


def contours() -> None:
    p = 83
    sign("0001", "Ana eşyükseklik eğrisi", "Ç", [sline(INK, 0.25)], p,
         "0,25 mm. Her beşinci eğri ana eğridir (açıklama, m. 78(1)(d)). 1/5000'de de 0,25 mm.", ["eş yükselti"])
    sign("0002", "Ara eşyükseklik eğrisi", "Ç", [sline(INK, 0.13)], p, "0,13 mm.", ["eş yükselti"])
    sign("0003", "Yardımcı eşyükseklik eğrisi", "Ç", [sline(INK, 0.13, dash=[5, 1])], p,
         "0,13 mm; 5 mm çizgi, 1 mm boşluk. Ara eğriler arası çok açık olduğunda ya da yapısal bir değişiklik varsa çizilir.",
         ["eş yükselti"])
    sign("0004", "Kot noktası", "N", [shape("circle", 0.3, fill=INK),
                                      text({"expr": "varsayılan([Kot], '')"}, 1.2, INK, weight=400, anchor="left",
                                           offset=[0.35, 0])], p,
         "Kot 1,2 mm normal kalınlıkta yazılır. Yönetmelikte noktanın yeri kotun ondalık ayıracıdır; KentOS noktayı küçük "
         "bir noktayla gösterir, kotu (Kot alanı) sağına yazar.", ["kot", "nokta"])
    sign("0005", "Eşyükseklik eğrisi kotu", "YÇ", [
        sline(INK, 0.13),
        mline([text({"expr": "varsayılan([Kot], '')"}, 1.4, INK, weight=400, halo={"color": PAPER, "width": 1.0})],
              interval=200, offsetAlong=10)], p,
         "Kot eğriye paralel, 1,4 mm, normal kalınlıkta; eğriyle arası 1 mm. m. 78(1)(f): 200 mm'de bir. KentOS kotu (Kot "
         "alanı) eğrinin başından 1 cm sonra ve sonra 200 mm'de bir eğrinin üstüne yazar, eğriyi yazının altında zeminle "
         "örter.", ["eş yükselti", "kot"])
    sign("0006", "Kokurdan", "Ç", [sline(INK, 0.2), mline([head(1.2)], placement="last")], p,
         "0,2 mm ok; ok kokurdanın başlangıcından tabanına iner. Ok ucunun boyu yazılı değil, çizimden 1,2 mm ölçüldü.",
         ["eş yükselti", "çukur"])


def roads() -> None:
    sign("0201", "Sert satıhlı yol", "Ç", [sline(INK, 0.2)], 84, "0,2 mm (1/5000'de 0,16 mm). Asfalt, beton, parke ve benzeri yol.",
         ["yol"])
    sign("0202", "Toprak yol", "Ç", [sline(INK, 0.16, dash=[2, 1])], 84,
         "0,16 mm; 2 mm çizgi, 1 mm boşluk. Yolun kuzey kenarı bu kesik çizgiyle, güney kenarı düz çizgiyle çizilir.",
         ["yol", "toprak"])
    sign("0203", "Patika", "Ç", [sline(INK, 0.16, dash=[2, 1])], 84, "0,16 mm; 2 mm çizgi, 1 mm boşluk (1/5000'de 0,1 mm).",
         ["yol", "patika"])
    sign("0204", "Kaldırım kenarı bordürtaş", "Ç", [sline(INK, 0.1, dash=[2, 1])], 84, "0,1 mm; 2 mm çizgi, 1 mm boşluk.",
         ["yol", "kaldırım", "bordür"])
    sign("0205", "Yapılmakta olan yol", "Ç", [sline(INK, 0.16, dash=[2, 1])], 84,
         "0,16 mm; 2 mm çizgi, 1 mm boşluk. Yolun iki kenarı da kesik çizilir.", ["yol", "yapılmakta"])
    sign("0206", "Bisiklet yolu", "Ç", [sline(INK, 0.16, dash=[2, 0.95, 0.1, 0.95], cap="round")], 84,
         "0,16 mm; 2 mm çizgi ve ardından 2 mm içinde bir nokta. Noktanın aralıkları çizimden.", ["yol", "bisiklet"])
    sign("0207", "Merdiven", "A", [hatch(90, 1, 0.16, INK), outline(0.16)], 85,
         "0,16 mm; basamaklar 1 mm aralıkla taranır. Basamakların yönü merdivene göredir; KentOS dikey tarar.", ["merdiven"])
    sign("0208", "Altgeçit", "A", [outline(0.16, dash=[2, 1])], 85, "0,16 mm; 2 mm çizgi, 1 mm boşluk.", ["geçit"])
    sign("0209", "Üstgeçit (üstü açık)", "A", [outline(0.16)], 85, "0,16 mm.", ["geçit"])
    sign("0210", "Üstgeçit (üstü kapalı)", "A", [outline(0.16), centroid([shape("x", 3, stroke_=INK, width=0.16)])], 85,
         "0,16 mm; içi köşeden köşeye çarpı. KentOS çarpıyı alanın ortasına 3 mm çizer.", ["geçit"])
    sign("0211", "Tünel", "A", [outline(0.16, dash=[3, 1])], 85, "0,16 mm; 3 mm çizgi, 1 mm boşluk.", ["tünel"])
    sign("0212", "Durak (üstü açık)", "N", [shape("rectangle", 5, fill=PAPER, stroke_=INK, width=0.16, height=3),
                                           text("D", 2.0, INK, weight=400)], 85,
         "5 × 3 mm dikdörtgen, 0,16 mm; içinde D.", ["durak", "otobüs"])
    sign("0213", "Durak (üstü kapalı)", "A", [outline(0.16, dash=[2, 1]), centroid([shape("x", 3, stroke_=INK, width=0.16)])],
         86, "0,16 mm; 2 mm çizgi, 1 mm boşluk; içi kesik çarpı. 0212'den büyük duraklar ölçüsüyle çizilir. KentOS çarpıyı "
             "alanın ortasına 3 mm çizer.", ["durak", "otobüs"])
    sign("0214", "Yol numarası", "N", [shape("diamond", 7, fill=PAPER, stroke_=INK, width=0.16, height=10),
                                      text({"expr": "varsayılan([Numara], '')"}, 2.2, INK, weight=400)], 86,
         "1 cm yüksek, 7 mm geniş eşkenar dörtgen, 0,16 mm; içinde KGM'nin yol numarası (Numara alanı). Yazının boyu yazılı "
         "değil, çizimden 2,2 mm.", ["yol", "numara"])
    sign("0215", "Köprü/viyadük", "Ç", [sline(INK, 0.2)] + ends(1.25, 120, 0.2), 86,
         "0,2 mm; iki ucu 120° açıyla kıvrılır. Kıvrımın boyu yazılı değil, aynı sayfadaki 2 mm'ye göre 1,25 mm ölçüldü.",
         ["köprü", "viyadük"])
    sign("0216", "Büz", "N", [shape("line", 2, stroke_=INK, width=0.16, rotation=135, offset=beside(-0.707, 0.707, 135)),
                             shape("line", 2, stroke_=INK, width=0.16, rotation=45, offset=beside(0.707, 0.707, 45))], 86,
         "0,16 mm; kolları 2 mm, aralarında 90°. Genişliği 1 m'den az su geçitleri büzle çizilir (yolun iki yanına).",
         ["büz", "su geçidi"])
    sign("0217", "Menfez", "Ç", [sline(INK, 0.16)] + ends(2, 135, 0.16), 86,
         "0,16 mm; iki ucu 45° açıyla 2 mm kıvrılır. Genişliği 1–3 m su geçitleri menfez olarak çizilir.", ["menfez", "su geçidi"])
    sign("0218", "Yapılmakta olan köprü/viyadük", "Ç", [sline(INK, 0.2, dash=[0.6, 0.4])] + ends(1.25, 120, 0.2), 86,
         "0,2 mm kesik; uçları 0215 gibi 120°. Çizgi ve boşluk yazılı değil, çizimden 0,6 ve 0,4 mm.", ["köprü", "yapılmakta"])
    sign("0219", "Şarampol", "Ç", [sline(INK, 0.13), mline([shape("arrowhead", 1.5, fill=INK)], interval=20, offsetAlong=10)],
         87, "Yolun eğim yönünde su toplama çizgisi üzerine 1,5 mm ok. Çizginin kalınlığı yazılı değil, 0,13 mm.",
         ["şarampol", "hendek"])
    sign("0220", "Koşu/yürüyüş parkuru", "Ç", [sline(INK, 0.16, dash=[5, 0.77, 0.1, 0.77, 0.1, 0.77], cap="round")], 87,
         "0,16 mm; 5 mm çizgi, ardından 2,5 mm içinde iki nokta. Noktaların aralıkları çizimden.", ["parkur", "spor"])


def rail() -> None:
    sign("0401", "Demiryolu", "Ç", [sline(INK, 0.3), ticks(2, 15, 0.3)], 88,
         "0,3 mm; 1,5 cm'de bir 2 mm dik çizgi. Ölçüler iki ray ortasından alınır.", ["demiryolu"])
    sign("0402", "Yapılmakta olan demiryolu", "Ç", [sline(INK, 0.3, dash=[5, 1]), ticks(2, 12, 0.3)], 88,
         "0,3 mm; 5 mm çizgi, 1 mm boşluk, 2 mm dik çizgiler. Dik çizgilerin aralığı yazılı değil, çizimde iki çizgide bir.",
         ["demiryolu", "yapılmakta"])
    sign("0403", "Hemzemin geçit", "Ç", [sline(INK, 0.16),
                                         mline([shape("circle", 1, fill=PAPER, stroke_=INK, width=0.16)], placement="last")],
         88, "0,16 mm; ucunda 1 mm çaplı daire. Bariyersiz hemzemin geçitler işaretsiz çizilir.", ["demiryolu", "geçit"])
    pair = dict(group={"count": 2, "spacing": 1})
    sign("0404", "Metro (toprak altı)", "Ç", [sline(INK, 0.4, dash=[3, 1]), ticks(1, 10, 0.2, above=True, **pair)], 88,
         "0,4 mm; 3 mm çizgi, 1 mm boşluk; 1 cm'de bir aralarında 1 mm olan iki küçük çizgi (1 mm).", ["metro", "raylı"])
    sign("0405", "Tramvay/metro (toprak üstü)", "Ç", [sline(INK, 0.4), ticks(1, 10, 0.2, above=True, **pair)], 88,
         "0,4 mm; 1 cm'de bir aralarında 1 mm olan iki küçük çizgi (1 mm).", ["tramvay", "metro", "raylı"])
    sign("0406", "Yükleme platformu", "A", [outline(0.25)], 89, "0,25 mm (1/5000'de 0,1 mm).", ["demiryolu", "platform"])
    sign("0407", "Demiryolu makası", "N", [shape("circle", 1, fill=PAPER, stroke_=INK, width=0.16),
                                           shape("line", 2.6, stroke_=INK, width=0.16, rotation=30)], 89,
         "1 mm çaplı daire, içinden geçen 0,16 mm çizgi. Çizginin boyu yazılı değil, çizimden 2,6 mm.", ["demiryolu", "makas"])


def pictograms() -> None:
    """The signs drawn with curves, as pictograms in millimetres around their point (`Box`)."""
    PICTOGRAMS.clear()
    b = Box(4)
    PICTOGRAMS["havuz"] = svg(stroke(
        f"M{b.p(0, -1.875)}L{b.p(0, 1.125)}"
        f"M{b.p(-1.5, 1.125)}A{b.r(0.75)} {b.r(0.75)} 0 0 1 {b.p(0, 1.125)}"
        f"M{b.p(0, 1.125)}A{b.r(0.75)} {b.r(0.75)} 0 0 1 {b.p(1.5, 1.125)}", b.w(0.2)))
    b = Box(4.6)
    PICTOGRAMS["sarnic"] = svg(stroke(
        f"M{b.p(-2, -1)}L{b.p(2, -1)}M{b.p(-1.5, -1)}A{b.r(1.5)} {b.r(1.5)} 0 0 1 {b.p(1.5, -1)}"
        f"M{b.p(0, 0.5)}L{b.p(0, 1)}", b.w(0.16)))
    b = Box(3.4)
    rays = ""
    for a in (30, 60, 90, 120, 150):
        t = math.radians(a)
        rays += (f"M{b.p(0.75 * math.cos(t), -0.95 + 0.75 * math.sin(t))}"
                 f"L{b.p(1.6 * math.cos(t), -0.95 + 1.6 * math.sin(t))}")
    PICTOGRAMS["bataklik"] = svg(stroke(f"M{b.p(-1.5, -0.65)}L{b.p(1.5, -0.65)}" + rays, b.w(0.16)))
    b = Box(4)
    half = [(-1.707, 1.207), (-1, 0.5), (1, 0.5), (1.707, 1.207)]
    PICTOGRAMS["su-yolu-koprusu"] = svg(stroke(
        "M" + "L".join(b.p(x, y) for x, y in half) + "M" + "L".join(b.p(x, -y) for x, y in half), b.w(0.2)))
    # Şerare (0824): two 4 mm strokes joined by a 2 mm step, an arrowhead at the lower end; origin at the step.
    b = Box(9)
    t = (-1.07, -4.0)
    d = (-2.07, -4.0)
    n = math.hypot(*d)
    back = (t[0] - d[0] / n, t[1] - d[1] / n)
    PICTOGRAMS["serare"] = svg(stroke(f"M{b.p(1.07, 4)}L{b.p(-1, 0)}L{b.p(1, 0)}L{b.p(*back)}", b.w(0.2), cap="butt",
                                      join="miter") + b.tip(*t, *d, 1.0, 0.7))
    # Antennas over a station (0818, 0820), origin at the station's point.
    b = Box(7, cy=5)
    PICTOGRAMS["telsiz-anten"] = svg(stroke(
        f"M{b.p(0, 2)}L{b.p(0, 4)}Q{b.p(0.7, 4.5)} {b.p(0, 5)}Q{b.p(-0.7, 5.5)} {b.p(0, 6)}L{b.p(0, 7.2)}", b.w(0.2))
        + b.tip(0, 8, 0, 1, 0.8, 0.6))
    b = Box(5.5, cy=3.5)
    PICTOGRAMS["tv-anten"] = svg(stroke(f"M{b.p(0, 1.5)}L{b.p(0, 3.5)}L{b.p(0.5, 3.8)}L{b.p(0.5, 4.7)}", b.w(0.2))
                                 + b.tip(0.5, 5.5, 0, 1, 0.8, 0.6))
    # Radar (0819): a 4 mm stem from the station's circle to a 5 mm dish open to the east.
    b = Box(6, cy=8)
    a0, a1 = math.radians(70), math.radians(-30)
    PICTOGRAMS["radar"] = svg(stroke(
        f"M{b.p(0, 1.5)}L{b.p(0, 5.5)}M{b.p(2.5 * math.cos(a0), 8 + 2.5 * math.sin(a0))}"
        f"A{b.r(2.5)} {b.r(2.5)} 0 1 0 {b.p(2.5 * math.cos(a1), 8 + 2.5 * math.sin(a1))}", b.w(0.2)))
    # Nükleer (0828): a 1 cm N whose last stroke ends in an open 2 mm arrowhead.
    b = Box(12)
    PICTOGRAMS["nukleer"] = svg(stroke(f"M{b.p(-5, -5)}L{b.p(-5, 5)}L{b.p(5, -5)}L{b.p(5, 5)}"
                                       f"M{b.p(4.35, 3.1)}L{b.p(5, 5)}L{b.p(5.65, 3.1)}", b.w(0.16)))
    # Aidiyet (1024): a 2 mm squiggle laid along the line.
    b = Box(2.6)
    PICTOGRAMS["aidiyet"] = svg(stroke(f"M{b.p(-1, -0.2)}Q{b.p(-0.55, 1)} {b.p(0, 0)}Q{b.p(0.55, -1)} {b.p(1, 0.2)}",
                                       b.w(0.16)))
    # Yapılar (1200).
    b = Box(7.6, cy=2.4)
    PICTOGRAMS["cami"] = svg(stroke(
        f"M{b.p(-3.5, 0)}L{b.p(3.5, 0)}M{b.p(-2.25, 0)}A{b.r(2.25)} {b.r(2.25)} 0 0 1 {b.p(2.25, 0)}"
        f"M{b.p(0, 2.25)}L{b.p(0, 3.75)}M{b.p(-1, 4.75)}A{b.r(1)} {b.r(1)} 0 0 0 {b.p(1, 4.75)}", b.w(0.3)))
    b = Box(4.6, cy=1.5)
    PICTOGRAMS["turbe"] = svg(stroke(
        f"M{b.p(-2, 0)}L{b.p(2, 0)}M{b.p(0, 0)}L{b.p(0, 1.5)}M{b.p(-1.5, 3)}A{b.r(1.5)} {b.r(1.5)} 0 0 0 {b.p(1.5, 3)}",
        b.w(0.3)))
    b = Box(4.4)
    chords = ""
    for d in (-1.6, -0.8, 0.0, 0.8, 1.6):
        half_ = math.sqrt(4 - d * d)
        nx, ny, ux, uy = -0.7071, 0.7071, 0.7071, 0.7071
        chords += f"M{b.p(d * nx - half_ * ux, d * ny - half_ * uy)}L{b.p(d * nx + half_ * ux, d * ny + half_ * uy)}"
    PICTOGRAMS["akaryakit"] = svg(ring(50, 50, 2 * b.k, b.w(0.2)) + stroke(chords, b.w(0.1), cap="butt"))

    def teeth(b: Box, angles, r: float, depth: float = 0.45, base: float = 0.5) -> str:
        out = ""
        for a in angles:
            t = math.radians(a)
            ux, uy, nx, ny = math.cos(t), math.sin(t), -math.sin(t), math.cos(t)
            out += path(f"M{b.p((r - 0.05) * ux + base / 2 * nx, (r - 0.05) * uy + base / 2 * ny)}"
                        f"L{b.p((r + depth) * ux, (r + depth) * uy)}"
                        f"L{b.p((r - 0.05) * ux - base / 2 * nx, (r - 0.05) * uy - base / 2 * ny)}Z")
        return out
    b = Box(10, cx=2.3, cy=1.8)
    tip_ = (7.071, 6.071)
    PICTOGRAMS["degirmen-elektrik"] = svg(
        f'<circle cx="{50 - 2.3 * b.k:.2f}" cy="{50 + 1.8 * b.k:.2f}" r="{2 * b.k:.2f}" fill="none" stroke="currentColor" '
        f'stroke-width="{b.w(0.2)}"/>' + circle(50 - 2.3 * b.k, 50 + 1.8 * b.k, b.w(0.25))
        + teeth(b, range(0, 360, 45), 2.0)
        + stroke(f"M{b.p(1.414, 1.414)}L{b.p(4.243, 4.243)}L{b.p(4.243, 3.243)}L{b.p(6.364, 5.364)}", b.w(0.2))
        + b.tip(*tip_, 1, 1, 1.0, 0.7))
    b = Box(5.4, cy=1.2)
    PICTOGRAMS["degirmen-su"] = svg(
        stroke(f"M{b.p(-2, 0)}A{b.r(2)} {b.r(2)} 0 0 1 {b.p(2, 0)}", b.w(0.2)) + teeth(b, (0, 45, 90, 135, 180), 2.0)
        + path(f"M{b.p(-0.25, 0)}A{b.r(0.25)} {b.r(0.25)} 0 1 0 {b.p(0.25, 0)}A{b.r(0.25)} {b.r(0.25)} 0 1 0 "
               f"{b.p(-0.25, 0)}Z"))
    # Bitki örtüsü (1400): the motifs of the area signs and the single trees.
    b = Box(3.6)
    PICTOGRAMS["calilik"] = svg(stroke(
        f"M{b.p(-0.8, -1.2)}Q{b.p(-1.5, -0.8)} {b.p(-1.3, 0)}A{b.r(1.3)} {b.r(1.3)} 0 0 1 {b.p(1.3, 0)}"
        f"Q{b.p(1.5, -0.8)} {b.p(0.8, -1.2)}", b.w(0.2)))
    b = Box(2)
    PICTOGRAMS["meyvalik"] = svg(stroke(
        f"M{b.p(-0.75, -0.75)}L{b.p(-0.75, 0)}A{b.r(0.75)} {b.r(0.75)} 0 0 1 {b.p(0.75, 0)}L{b.p(0.75, -0.75)}", b.w(0.2)))
    b = Box(3.6)
    leaf = lambda m: (f"M{b.p(0, 0.5)}Q{b.p(m * 1.0, 0.55)} {b.p(m * 0.9, 1.5)}Q{b.p(m * 0.1, 1.45)} {b.p(0, 0.5)}Z")  # noqa: E731
    PICTOGRAMS["zeytin"] = svg(stroke(f"M{b.p(0, -1.5)}L{b.p(0, 0.5)}" + leaf(-1) + leaf(1), b.w(0.2)))
    b = Box(3.6, cx=0.5)
    PICTOGRAMS["findik"] = svg(stroke(f"M{b.p(0, -1.5)}L{b.p(0, 1.5)}Q{b.p(1.5, 1.5)} {b.p(1.5, 0.6)}L{b.p(0, 0.6)}",
                                      b.w(0.2)))
    b = Box(3.6)
    PICTOGRAMS["fistik"] = svg(stroke(f"M{b.p(0, -1.5)}L{b.p(0, 0.75)}M{b.p(-0.75, 0.75)}"
                                      f"A{b.r(0.75)} {b.r(0.75)} 0 0 1 {b.p(0.75, 0.75)}Z", b.w(0.2)))
    b = Box(3.6, cx=0.3)
    PICTOGRAMS["gul"] = svg(stroke(f"M{b.p(0, -1.5)}L{b.p(0, 0.9)}M{b.p(-0.5, 0.9)}L{b.p(0, 0.9)}"
                                   f"Q{b.p(0.15, 1.65)} {b.p(1.1, 1.5)}Q{b.p(0.95, 0.75)} {b.p(0, 0.9)}Z", b.w(0.2)))
    b = Box(4)
    rays = ""
    for a in range(0, 181, 30):
        t = math.radians(a)
        rays += f"M{b.p(1.2 * math.cos(t), -0.6 + 1.2 * math.sin(t))}L{b.p(1.8 * math.cos(t), -0.6 + 1.8 * math.sin(t))}"
    PICTOGRAMS["sazlik"] = svg(stroke(rays, b.w(0.2)))
    b = Box(7.6, cy=3.6)
    crown = [(0, 2.2), (-1.5, 2.4), (-2, 3.4), (-1.7, 4.6), (-1.1, 5.6), (-0.45, 6.6), (0, 7), (0.45, 6.6), (1.1, 5.6),
             (1.7, 4.6), (2, 3.4), (1.5, 2.4), (0, 2.2)]
    d = f"M{b.p(*crown[0])}"
    for (x0, y0), (x1, y1) in zip(crown, crown[1:]):
        mx, my = (x0 + x1) / 2, (y0 + y1) / 2
        ex, ey = x1 - x0, y1 - y0
        n = math.hypot(ex, ey)
        # The control point pushed outward (to the right of a clockwise walk) by a quarter of the edge.
        cx_, cy_ = mx - ey / n * 0.3, my + ex / n * 0.3
        d += f"Q{b.p(cx_, cy_)} {b.p(x1, y1)}"
    PICTOGRAMS["genis-agac"] = svg(stroke(d + f"M{b.p(0, 2.2)}L{b.p(0, 0.2)}", b.w(0.2))
                                   + ring(50, 50 + 3.6 * b.k, 0.2 * b.k, b.w(0.2)))
    b = Box(3.6, cy=0.3)
    PICTOGRAMS["cali"] = svg(stroke(
        f"M{b.p(-1.1, -0.6)}Q{b.p(-1.8, -0.5)} {b.p(-1.5, 0.1)}A{b.r(0.6)} {b.r(0.6)} 0 0 1 {b.p(-0.5, 0.6)}"
        f"A{b.r(0.55)} {b.r(0.55)} 0 0 1 {b.p(0.5, 0.6)}A{b.r(0.6)} {b.r(0.6)} 0 0 1 {b.p(1.5, 0.1)}"
        f"Q{b.p(1.8, -0.5)} {b.p(1.1, -0.6)}", b.w(0.2)))
    b = Box(2.4)
    PICTOGRAMS["kemer"] = svg(stroke(f"M{b.p(-1, 0)}A{b.r(1)} {b.r(1)} 0 0 1 {b.p(1, 0)}", b.w(0.2)))
    # Höyük (1811): sixteen rays around a dot.
    b = Box(3.4)
    rays = "".join(f"M{b.p(0.45 * math.cos(math.radians(a)), 0.45 * math.sin(math.radians(a)))}"
                   f"L{b.p(1.5 * math.cos(math.radians(a)), 1.5 * math.sin(math.radians(a)))}" for a in range(0, 360, 22))
    PICTOGRAMS["hoyuk"] = svg(stroke(rays, b.w(0.2)) + circle(50, 50, b.w(0.2)))
    # Tamamlayıcı işaretler (2000).
    b = Box(13, cx=2, cy=3.5)
    a0, a1 = math.radians(95), math.radians(47)
    PICTOGRAMS["vinc"] = svg(stroke(
        f"M{b.p(0, 1)}L{b.p(0, 6)}M{b.p(6 * math.cos(a0), 6 * math.sin(a0))}"
        f"A{b.r(6)} {b.r(6)} 0 0 1 {b.p(6 * math.cos(a1), 6 * math.sin(a1))}"
        f"M{b.p(0.574, 0.819)}L{b.p(3.441, 4.915)}", b.w(0.2)))
    b = Box(3.6)
    PICTOGRAMS["magara"] = svg(stroke(
        f"M{b.p(-1.5, -1.6)}L{b.p(-1.5, 0)}A{b.r(1.5)} {b.r(1.5)} 0 0 1 {b.p(1.5, 0)}L{b.p(1.5, -1.6)}", b.w(0.2)))
    b = Box(3)
    PICTOGRAMS["pompa"] = svg(
        f'<ellipse cx="50" cy="{50 - 1.5 * b.k:.2f}" rx="{b.k:.2f}" ry="{0.4 * b.k:.2f}" fill="none" stroke="currentColor" '
        f'stroke-width="{b.w(0.2)}"/><ellipse cx="50" cy="{50 + 1.1 * b.k:.2f}" rx="{b.k:.2f}" ry="{0.4 * b.k:.2f}" '
        f'fill="none" stroke="currentColor" stroke-width="{b.w(0.2)}"/>'
        + stroke(f"M{b.p(-1, 1.5)}L{b.p(-1, -1.1)}M{b.p(1, 1.5)}L{b.p(1, -1.1)}", b.w(0.2))
        + circle(50, 50 + 1.1 * b.k, b.w(0.25)))
    b = Box(7.6, cy=3.5)
    PICTOGRAMS["peri"] = svg(stroke(
        f"M{b.p(-0.3, 5.9)}C{b.p(-1.7, 5.9)} {b.p(-1.7, 7)} {b.p(0, 7)}C{b.p(1.7, 7)} {b.p(1.7, 5.9)} {b.p(0.3, 5.9)}"
        f"M{b.p(-0.3, 5.9)}L{b.p(-1, 0)}M{b.p(0.3, 5.9)}L{b.p(1, 0)}", b.w(0.2)) + circle(50, 50 + 3.2 * b.k, b.w(0.25)))
    b = Box(7.6)
    hammers = ""
    for sgn in (1, -1):
        x0, y0, x1, y1 = -1.5 * sgn, -3.5, 1.2 * sgn, 2.2
        hammers += f"M{b.p(x0, y0)}L{b.p(x1, y1)}"
    heads = ""
    for sgn in (1, -1):
        ux, uy = 2.7 * sgn, 5.7
        n = math.hypot(ux, uy)
        ux, uy = ux / n, uy / n
        nx, ny = uy * sgn, -ux * sgn
        cx_, cy_ = 1.2 * sgn + ux * 0.4, 2.2 + uy * 0.4
        corners = [(cx_ + nx * 0.75 * k + ux * 0.4 * m, cy_ + ny * 0.75 * k + uy * 0.4 * m)
                   for k, m in ((-0.4, -1), (1, -1), (1, 1), (-0.4, 1))]
        heads += path("M" + "L".join(b.p(x, y) for x, y in corners) + "Z")
    PICTOGRAMS["maden"] = svg(stroke(hammers, b.w(0.2)) + heads)
    b = Box(4.4, cy=-0.7)
    arch = [(-1.5, -3)] + [(-1.5, 0)] + [(1.5 * math.cos(math.radians(a)), 1.5 * math.sin(math.radians(a)))
                                         for a in range(180, -1, -10)] + [(1.5, -3)]
    d_ = "M" + "L".join(b.p(x, y) for x, y in arch)
    hach = ""
    walk = 0.0
    for (xa, ya), (xb, yb) in zip(arch, arch[1:]):
        L = math.hypot(xb - xa, yb - ya)
        ux, uy = (xb - xa) / L, (yb - ya) / L
        nx, ny = uy, -ux  # inside of a clockwise arch
        t = (0.4 - walk % 0.4) % 0.4
        while t < L:
            px, py = xa + ux * t, ya + uy * t
            hach += f"M{b.p(px, py)}L{b.p(px + nx * 0.45, py + ny * 0.45)}"
            t += 0.4
        walk += L
    PICTOGRAMS["maden-girisi"] = svg(stroke(d_, b.w(0.2)) + stroke(hach, b.w(0.1), cap="butt")
                                     + circle(50, 50 + 1.8 * b.k, b.w(0.25)))
    # Minare (2029): a 3 mm stem from the circle's top and a 2 mm crescent; origin at the circle's middle.
    b = Box(3, cy=4.25)
    PICTOGRAMS["minare"] = svg(stroke(f"M{b.p(0, 1.5)}L{b.p(0, 4.5)}M{b.p(-1, 5.5)}A{b.r(1)} {b.r(1)} 0 0 0 {b.p(1, 5.5)}",
                                      b.w(0.2)))
    # İstikamet oku (0628): a 3 mm head, slimmer than the shader's (as the drawing).
    b = Box(3.2)
    PICTOGRAMS["ok-basi"] = svg(path(f"M{b.p(1.5, 0)}L{b.p(-1.5, 0.7)}L{b.p(-1.5, -0.7)}Z"))
    b = Box(2.4)
    PICTOGRAMS["rayli-kanca"] = svg(stroke(
        f"M{b.p(-0.3, 1)}Q{b.p(0.35, 1)} {b.p(0.3, 0.3)}L{b.p(0.3, -0.3)}Q{b.p(0.3, -1)} {b.p(-0.3, -1)}", b.w(0.2)))


def water() -> None:
    p = 90
    flow = "Suyun akış yönü istikamet okuyla (0628) gösterilir."
    on_line = mline([head(1.2)], placement="center")
    tip = "Ok başının boyu yazılı değil; KentOS 1,2 mm çizer, çizginin ortasına koyar."
    sign("0601", "Yatağı geniş sulu dere (ölçeğe sığan)", "Ç", [sline(INK, 0.16)], p,
         f"0,16 mm (1/5000'de 0,1 mm); iki kıyı ayrı çizilir. {flow}", ["dere", "su"])
    sign("0602", "Yatağı dar sulu dere (ölçeğe sığmayan)", "Ç", [sline(INK, 0.16), on_line], p,
         f"0,16 mm; akış yönü çizginin üstünde ok başıyla gösterilir. {tip}", ["dere", "su"])
    sign("0603", "Yatağı geniş kuru dere (ölçeğe sığan)", "Ç", [sline(INK, 0.16, dash=[2, 1])], p,
         f"0,16 mm; 2 mm çizgi, 1 mm boşluk. {flow}", ["dere", "kuru"])
    sign("0604", "Yatağı dar kuru dere (ölçeğe sığmayan)", "Ç", [sline(INK, 0.16, dash=[2, 1]), on_line], p,
         f"0,16 mm; 2 mm çizgi, 1 mm boşluk; akış yönü ok başıyla. {tip}", ["dere", "kuru"])
    sign("0605", "Çeşme (ölçeğe sığan)", "A", [outline(0.16)], p,
         "0,16 mm. Çeşme yapısı ölçüsüyle çizilir, musluklar (0606) yerlerine konur.", ["çeşme", "su"])
    sign("0606", "Çeşme (ölçeğe sığmayan)", "N", polyline([(-0.5, 1), (-0.5, 0), (0.5, 0), (0.5, -1)]), p,
         "0,16 mm; 1 mm yukarı, 1 mm yana, 1 mm aşağı kırık çizgi (musluk).", ["çeşme", "musluk"])
    p = 91
    well = [ring_(2), dot()]
    sign("0607", "Pınar", "N", well + [seg(0, -1.2, 0, -2.5), seg(0, -3.45, 0, -4.75), seg(0, -5.7, 0, -7)], p,
         "2 mm çaplı daire, ortasında nokta, altında 6 mm kesik kuyruk; 0,16 mm. Kuyruğun kesikleri çizimden.",
         ["pınar", "kaynak", "su"])
    sign("0608", "Kuyu", "N", well, p, "2 mm çaplı daire, ortasında nokta; 0,16 mm.", ["kuyu", "su"])
    stem = seg(0, 1, 0, 7)
    sign("0609", "Tulumba", "N", well + [stem, seg(0, 7, 1.2, 9)]
         + polyline([(0, 7), (-1.5, 7), (-1.6, 6.5), (-1.2, 6.0)]), p,
         "Kuyu dairesi, 6 mm sap, tepesinde 3 mm genişliğinde kol ve ağız; 0,16 mm. Kolun ve ağzın biçimi çizimden.",
         ["tulumba", "pompa", "su"])
    sign("0610", "Artezyen", "N", well + [stem, seg(0, 7, 0.6, 6.0)] + polyline([(0, 7), (-0.6, 6.1), (-0.25, 5.75)]), p,
         "Kuyu dairesi, 6 mm sap, tepesinde fışkıran suyu gösteren ok; 0,16 mm. Okun biçimi çizimden.",
         ["artezyen", "su"])
    sign("0611", "Serenli kuyu", "N", well + [stem, seg(-1.061, 5.939, 1.061, 8.061), seg(1.061, 8.061, 1.061, 6.061),
                                               dot(0.4, 1.061, 6.061)], p,
         "Kuyu dairesi, 6 mm sap, tepesinde 3 mm seren, ucundan 2 mm sarkan ip ve 0,4 mm çaplı kova; 0,16 mm.",
         ["kuyu", "seren", "su"])
    sign("0612", "Kanal (ölçeğe sığan)", "Ç", [sline(INK, 0.16)], p,
         f"0,16 mm; iki kenar ayrı çizilir. {flow} Drenaj kanalları da bu işaretle gösterilir.", ["kanal", "su"])
    p = 92
    sign("0613", "Kanal (ölçeğe sığmayan)", "Ç", [sline(INK, 0.16), on_line], p,
         f"0,16 mm; akış yönü ok başıyla. {tip}", ["kanal", "su"])
    sign("0614", "Sulama cetveli", "Ç", [sline(INK, 0.16)], p,
         "0,16 mm. Tarım alanlarını sulayan küçük arklar. Yalnız gösterimdir.", ["sulama", "ark"])
    sign("0615", "Kanalet", "Ç", [sline(INK, 0.16),
                                  mline([seg(-1, 0, 0, -1.732), seg(0, -1.732, 1, 0)], interval=15, offsetAlong=7.5)], p,
         "0,16 mm; 1,5 cm'de bir ayakları gösteren V: kolları 2 mm, aralarında 60° (1/5000'de 1 cm'de bir).",
         ["kanalet", "sulama"])
    sign("0616", "Su sifonu", "N", [shape("square", 3, fill=PAPER, stroke_=INK, width=0.16), dot()], p,
         "3 × 3 mm kare, ortasında nokta; 0,16 mm (1/5000'de 2 mm).", ["sifon", "su"])
    sign("0617", "Toprak üstü su yolu", "Ç", [sline(INK, 0.16), mline([dot(1)], interval=10, offsetAlong=5)], p,
         "0,16 mm; 1 cm'de bir 1 mm çaplı dolu daire.", ["su yolu", "isale"])
    sign("0618", "Toprak altı su yolu", "Ç", [sline(INK, 0.16, dash=[10, 1]), mline([dot(1)], interval=11, offsetAlong=5)],
         p, "0,16 mm; 1 cm çizgi, 1 mm boşluk; her çizginin ortasında 1 mm çaplı dolu daire.", ["su yolu", "isale"])
    p = 93
    sign("0619", "Su yolu köprüsü (ölçeğe sığan)", "Ç", [sline(INK, 0.2)] + ends(1, 135, 0.2), p,
         "0,2 mm; iki ucu 45° dışa kıvrılır. Kıvrımın boyu yazılı değil, 0620'deki gibi 1 mm.", ["köprü", "su yolu"])
    sign("0620", "Su yolu köprüsü (ölçeğe sığmayan)", "N", [bpic("su-yolu-koprusu", 4, rotation=G.TURN)], p,
         "0,2 mm; 2 mm taban, iki ucunda 45° açılan 1 mm kanat. Örnekteki gibi su yolunun iki yanına karşılıklı "
         "çizilir; KentOS ikisini birlikte çizer, Açı alanıyla döner (0'da su yolu doğu-batı).", ["köprü", "su yolu"])
    sign("0621", "Havalandırma bacası", "N", [shape("square", 2, fill=PAPER, stroke_=INK, width=0.2), dot()], p,
         "2 × 2 mm kare, ortasında nokta; 0,2 mm. Toprak altı su yolları ve kanalizasyonun bacası.",
         ["baca", "havalandırma"])
    sign("0622", "Su deposu", "A", [outline(0.16)], p, "0,16 mm. İçme ve kullanma suyu deposu.", ["depo", "su"])
    sign("0623", "Su kulesi", "N", [ring_(1.5), seg(-0.75, 0, -0.75, 4.25), seg(0.75, 0, 0.75, 4.25),
                                    seg(-0.75, 4.25, 0, 5.75), seg(0.75, 4.25, 0, 5.75)], p,
         "Altta 1,5 mm çaplı daire, 5 mm gövde ve 1,5 mm sivri tepe; 0,16 mm. Deponun ayakları ölçüsüyle çizilir, "
         "araları kesik çizgiyle birleştirilir.", ["su kulesi", "depo"])
    sign("0624", "Su yolu tüneli (ölçeğe sığan)", "Ç", [sline(INK, 0.2, dash=[4, 2])], p,
         "0,2 mm; 4 mm çizgi, 2 mm boşluk. Tünelin iki kenarı bu çizgiyle çizilir, ağızlarda düz çizgiyle kapanır.",
         ["tünel", "su yolu"])
    p = 94
    mouth = [seg(0, -1, 0, 1), seg(0, 1, -0.707, 1.707), seg(0, -1, -0.707, -1.707)]
    sign("0625", "Su yolu tüneli (ölçeğe sığmayan)", "Ç",
         [sline(INK, 0.16, dash=[4, 2]), mline(mouth, placement="first"),
          mline([seg(0, -1, 0, 1), seg(0, 1, 0.707, 1.707), seg(0, -1, 0.707, -1.707)], placement="last")], p,
         "0,16 mm; 4 mm çizgi, 2 mm boşluk. Ağızlar örnekteki gibi dışa açılan kısa çizgilerle; ölçüleri çizimden.",
         ["tünel", "su yolu"])
    sign("0626", "Havuz", "N", [bpic("havuz", 4)], p,
         "3 mm sap, tepesinde 1,5 mm çaplı iki yay (3 mm geniş); 0,2 mm. Havuzun çevresi ölçüsüyle çizilir.",
         ["havuz", "fıskiye"])
    sign("0627", "Sarnıç", "N", [bpic("sarnic", 4.6)], p,
         "4 mm taban üstünde 3 mm çaplı yarım daire, tepesinde 0,5 mm çizgi; 0,16 mm. Yağmur suyunu biriktiren yapı.",
         ["sarnıç", "su"])
    sign("0628", "İstikamet oku", "Ç", [sline(INK, 0.16), mline([bpic("ok-basi", 3.2, offset=[-1.5, 0])], placement="last")], p,
         "0,16 mm, 1 cm ok; ok başı 3 mm (1/5000'de 7 mm ok, 2 mm baş), eni çizimden 1,4 mm. Akış ve yön gösterir; "
         "çizgi akış yönünde çizilir.", ["ok", "akış", "yön"])
    sign("0629", "Deniz ve göl kıyısı", "Ç", [sline(INK, 0.16)], p, "0,16 mm (1/5000'de 0,1 mm).", ["kıyı", "deniz", "göl"])
    sign("0630", "Değişken kıyı", "Ç", [sline(INK, 0.16, dash=[4, 1])], p,
         "0,16 mm; 4 mm çizgi, 1 mm boşluk. Yeri zamanla değişen kıyı.", ["kıyı"])
    p = 95
    sign("0631", "Geçici göl", "A", [outline(0.16, dash=[1, 0.5])], p,
         "0,16 mm; 1 mm çizgi, 0,5 mm boşluk. Kurak mevsimde suyu çekilen göl.", ["göl", "geçici"])
    sign("0632", "Çeltik", "A", [bpattern([dot(0.3, x, y) for x in (-1.5, 1.5) for y in (-1.5, 1.5)], 8, 6, stagger=True)], p,
         "Aralarında 3 mm olan dört noktalık öbekler (0,16 mm kalemle; KentOS 0,3 mm nokta çizer). Öbeklerin dizilişi "
         "örnekten: 8 mm arayla, sıralar 6 mm arayla ve yarım kaydırılmış. Alanın sınırı ayrıca çizilir.",
         ["çeltik", "pirinç", "tarım"])
    tuft = bpic("bataklik", 3.4)
    sign("0633", "Bataklık", "A", [bpattern(
        [dict(tuft, offset=[-3.75, 3.5])] + [seg(x - 1.5, y, x + 1.5, y) for x, y in ((3.75, 3.5), (-3.75, -3.5),
                                                                                           (3.75, -3.5))], 15, 14)], p,
         "Sıralar 7 mm arayla: bir sırada 1,5 cm arayla 3 mm geniş bataklık otu ve aralarında 3 mm çizgi, öbür sırada "
         "yalnız çizgiler; 0,16 mm. Otun yüksekliği çizimden.", ["bataklık", "sulak"])
    sign("0634", "Çağlayan", "Ç", [sline(INK, 0.2),
                                   sline(INK, 0.2, offset=-1, wave={"shape": "zigzag", "length": 2, "amplitude": 1,
                                                                     "connect": True})], p,
         "0,2 mm; bir yanında 2 mm aralıklı, 2 mm derin dişler. Akarsuyun yatağındaki ani düşüş; akışa dik çizilir.",
         ["çağlayan", "şelale"])
    sign("0635", "Kanal kapağı", "Ç", [sline(INK, 0.2)], p, "0,2 mm (1/5000'de 0,16 mm).", ["kanal", "kapak"])
    sign("0636", "Bent", "Ç", [sline(INK, 0.2, offset=0.75), sline(INK, 0.2, offset=-0.75), ticks(1.5, 2, 0.2)], p,
         "0,2 mm; aralarında 1,5 mm olan iki çizgi, 2 mm'de bir basamak (1/5000'de 1 mm).", ["bent", "set"])
    p = 96
    sign("0637", "Gölet/baraj", "A", [outline(0.2)], p, "0,2 mm (1/5000'de 0,16 mm). Akarsu önündeki engelin ardındaki "
         "yapay göl.", ["gölet", "baraj"])
    up = shape("triangle", 1, fill=INK, offset=[0, 0.289])
    down = shape("triangle", 1, fill=INK, rotation=180, offset=G.beside(0, -0.289, 180))
    sign("0638", "Dalgakıran/mendirek", "Ç",
         [sline(INK, 0.2), mline([up, down], interval=5, offsetAlong=2.5)]
         + [sline(INK, 0.2, offset=o) for o in (2, 3, -2, -3)], p,
         "0,2 mm; eksen üstünde 5 mm'de bir dolu üçgen, ondan 2 mm ötede iç kenar, 1 mm ötede dış kenar. Örnekteki gibi "
         "eksenin iki yanına simetrik; KentOS çizgiyi eksen sayar. Üçgenin boyu yazılı değil, 1 mm.",
         ["dalgakıran", "mendirek", "liman"])
    sign("0639", "Mahmuz", "Ç", [sline(INK, 0.16)], p, "0,16 mm. Akarsuyun alüvyonunu biriktiren tesis.", ["mahmuz"])
    sign("0640", "İskele", "Ç", [sline(INK, 0.2)], p, "0,2 mm (1/5000'de 0,1 mm).", ["iskele", "liman"])
    sign("0641", "Kıyı duvarı ve rıhtım", "Ç", [sline(INK, 0.35)], p, "0,35 mm (1/5000'de 0,1 mm).",
         ["rıhtım", "kıyı duvarı", "liman"])
    sign("0642", "Deniz feneri", "N", polyline([(-1.5, -1), (1.5, -1), (0, 2), (-1.5, -1)]) + [
        dot(), seg(0, 2, 0, 5), shape("x", 2, stroke_=INK, width=0.16, offset=[0, 5]), ring_(1, y=5)], p,
         "3 mm tabanlı, 3 mm yüksek üçgen, ortasında nokta; tepesinden 3 mm direk, ucunda 2 mm çarpı ve 1 mm çaplı "
         "daire; 0,16 mm. Fenerin binası ölçüsüyle resmî bina olarak çizilir.", ["fener", "deniz"])
    p = 97
    sign("0643", "Şamandıra", "N", polyline([(-3.5, -1.862), (3.5, -1.862), (0, 3.724), (-3.5, -1.862)]) + [
        dot(), seg(0, 3.724, 1.061, 4.785), ring_(1, x=1.414, y=5.138)], p,
         "7 mm tabanlı üçgen, ortasında nokta; tepesinden 45° eğik 1,5 mm direk, ucunda 1 mm çaplı daire; tabandan "
         "dairenin ortasına 7 mm; 0,16 mm. Sığlık sınırını gösteren sesli ya da ışıklı işaret.", ["şamandıra", "deniz"])
    ticks8 = [seg(1.4 * math.cos(math.radians(a)), 1.4 * math.sin(math.radians(a)),
                  2 * math.cos(math.radians(a)), 2 * math.sin(math.radians(a))) for a in range(0, 360, 45)]
    sign("0644", "Düden", "N", [ring_(4, fill=None), dot()] + ticks8 + [
        seg(4.243, 4.243, 1.98, 1.98), shape("arrowhead", 0.8, fill=INK, rotation=225, offset=G.beside(1.697, 1.697, 225))],
         p, "4 mm çaplı daire, ortasında nokta, içinde kısa çizgiler, dışından daireye inen 4 mm ok; 0,16 mm. Çizgilerin "
            "ve ok başının boyu çizimden.", ["düden", "çukur", "karst"])
    sign("0645", "Kıyı kenar çizgisi", "Ç", [sline(INK, 0.16),
                                             mline([shape("square", 2, fill=PAPER, stroke_=INK, width=0.16)],
                                                   placement="vertex")], p,
         "0,16 mm; her kırılma noktasında 2 mm kare.", ["kıyı", "kıyı kenar çizgisi"])
    sign("0646", "Pompa istasyonu", "A", [outline(0.16)], p, "0,16 mm. Su, gaz, yakıt gibi maddeleri basan yapı.",
         ["pompa", "istasyon"])
    sign("0647", "Regülatör", "A", [outline(0.16)], p, "0,16 mm. Su akış hızını düzenleyen motorlu bina.", ["regülatör"])


def lines() -> None:
    p = 98
    pair = dict(group={"count": 2, "spacing": 0.4})
    above_ticks = lambda **kw: ticks(1, 20, 0.16, above=True, **pair, **kw)  # noqa: E731
    box = shape("square", 2, fill=PAPER, stroke_=INK, width=0.16, offset=[0, 1])
    sign("0801", "Telefon hattı toprak üstü", "Ç", [sline(INK, 0.16), above_ticks(offsetAlong=10)], p,
         "0,16 mm; 2 cm'de bir 1 mm yüksek iki küçük çizgi. Çiftin aralığı yazılı değil, çizimden 0,4 mm.",
         ["telefon", "haberleşme"])
    sign("0802", "Telefon hattı toprak altı", "Ç", [sline(INK, 0.16, dash=[19, 1]), above_ticks(offsetAlong=9.5)], p,
         "0,16 mm; 2 cm'de bir 1 mm boşluk, aradaki çizginin ortasında 0801'in iki küçük çizgisi.",
         ["telefon", "haberleşme", "yer altı"])
    sign("0803", "Radyo TV hattı toprak üstü", "Ç", [sline(INK, 0.16), mline([box], interval=20, offsetAlong=10)], p,
         "0,16 mm; 2 cm'de bir çizginin üstüne oturan 2 × 2 mm kare.", ["radyo", "televizyon", "haberleşme"])
    sign("0804", "Radyo TV hattı toprak altı", "Ç", [sline(INK, 0.16, dash=[19, 1]),
                                                     mline([box], interval=20, offsetAlong=9.5)], p,
         "0,16 mm; 2 cm'de bir 1 mm boşluk, aradaki çizginin ortasında 2 × 2 mm kare.",
         ["radyo", "televizyon", "haberleşme", "yer altı"])
    sign("0805", "Elektrik hattı toprak üstü", "Ç", [sline(INK, 0.16)], p, "0,16 mm.", ["elektrik", "enerji"])
    sign("0806", "Elektrik hattı toprak altı", "Ç", [sline(INK, 0.16, dash=[19, 1])], p,
         "0,16 mm; 2 cm'de bir 1 mm boşluk.", ["elektrik", "enerji", "yer altı"])
    p = 99
    sign("0807", "Yüksek gerilim hattı", "Ç", [
        sline(INK, 0.3), mline([bpic("serare", 9)], interval=60, offsetAlong=10),
        mline([text("ENH", 1.8, INK, weight=400, offset=[0, 1.6])], interval=120, offsetAlong=40)], p,
         "0,3 mm. Pilonlar arasına uygun sıklıkta şerare işareti (0824) konur, uygun bir yere ENH (enerji nakil hattı) "
         "yazılır. KentOS şerareyi çizginin başından 1 cm sonra ve 6 cm'de bir, ENH'yi 4 cm sonra ve 12 cm'de bir "
         "çizginin üstüne koyar; ENH'nin boyu yazılı değil, 1,8 mm.", ["yüksek gerilim", "enerji", "ENH"])
    sign("0808", "Boru hattı (zemin üstü)", "Ç", [sline(INK, 0.2), mline([dot(1)], interval=20, offsetAlong=10)], p,
         "0,2 mm; 2 cm'de bir 1 mm çaplı dolu daire (1/5000'de 1 cm'de bir). Su, petrol, doğal gaz boruları.",
         ["boru", "petrol", "doğal gaz"])
    sign("0809", "Boru hattı (zemin altı)", "Ç", [sline(INK, 0.2, dash=[19, 1]),
                                                  mline([dot(1)], interval=20, offsetAlong=9.5)], p,
         "0,2 mm; 2 cm'de bir 1 mm boşluk, aradaki çizginin ortasında 1 mm çaplı dolu daire.",
         ["boru", "petrol", "doğal gaz", "yer altı"])
    sign("0810", "Kanalizasyon", "Ç", [sline(INK, 0.2, dash=[5, 1]), mline([head(1.2)], interval=12, offsetAlong=5)], p,
         "0,2 mm; 5 mm çizgi, 1 mm boşluk; çizgilerin birinde bir akış yönünde ok başı (1/5000'de 3 mm). Ok başının boyu "
         "yazılı değil, çizimden 1,2 mm.", ["kanalizasyon", "atık su"])
    sign("0811", "Raylı nakil hattı", "Ç", [sline(INK, 0.2), mline([bpic("rayli-kanca", 2.4)], interval=20, offsetAlong=10)],
         p, "0,2 mm; 2 cm'de bir 2 mm yüksek kanca (1 mm yukarı, 1 mm aşağı). Maden ve yük taşıyan raylı nakil.",
         ["nakil", "maden", "teleferik"])
    small_bolt = centroid([bpic("serare", 4.5)])
    sign("0812", "Trafo (merkez tipi, indirici)", "A", [hatch(45, 1, 0.1, INK), outline(0.2), small_bolt], p,
         "0,2 mm, içi taralı. Taramanın aralığı ve kalınlığı yazılı değil, 1 mm ve 0,1 mm. Binanın üstüne şerare "
         "işareti (0824) konur; KentOS onu alanın ortasına 1/5000 boyunda (4 mm) çizer.", ["trafo", "elektrik"])
    p = 100
    bolt_up = bpic("serare", 9, rotation=180, offset=G.beside(1.07, 6, 180))
    sign("0813", "Trafo (hücre tipi)", "N", [shape("square", 4, fill=PAPER, stroke_=INK, width=0.2),
                                             seg(-2, -2, 2, 2, 0.2), seg(-2, 2, 2, -2, 0.2), bolt_up], p,
         "4 × 4 mm kare, köşeden köşeye çarpı; 0,2 mm; üstünde 8 mm şerare (1/5000'de 2 × 2 mm, 5 mm). Taban alanı "
         "5 m²'den küçük trafo yapısı.", ["trafo", "elektrik"])
    sign("0814", "Trafo (açık tip)", "N", [ring_(3, 0.2), dot(0.5), dict(bolt_up, offset=G.beside(1.07, 5.5, 180))], p,
         "3 mm çaplı daire, ortasında nokta; 0,2 mm; üstünde 8 mm şerare (1/5000'de 2 mm, 5 mm). Binası olmayan trafo.",
         ["trafo", "elektrik"])
    sign("0815", "Some noktası", "N", [shape("diamond", 4, fill=PAPER, stroke_=INK, width=0.2, height=4), dot(0.5)], p,
         "4 × 4 mm eşkenar dörtgen, ortasında 0,5 mm nokta; 0,2 mm (1/5000'de 2 × 2 mm, 0,2 mm). Enerji nakil hattının "
         "yön değiştirdiği nokta.", ["some", "enerji"])
    sign("0816", "Branşman noktası", "N", [shape("square", 4, fill=PAPER, stroke_=INK, width=0.2), dot(0.5)], p,
         "4 × 4 mm kare, ortasında 0,5 mm nokta; 0,2 mm. Enerji nakil hattından başka bir hattın ayrıldığı nokta.",
         ["branşman", "enerji"])
    sign("0817", "Elektrik santrali", "A", [outline(0.2), small_bolt], p,
         "0,2 mm (1/5000'de 0,1 mm). Örnekteki gibi içine şerare konur; KentOS onu alanın ortasına 4 mm çizer.",
         ["santral", "elektrik", "enerji"])
    sign("0818", "Radyo telsiz istasyonu", "N", [shape("square", 4, fill=PAPER, stroke_=INK, width=0.2), dot(0.5),
                                                 bpic("telsiz-anten", 7, offset=[0, 5])], p,
         "4 × 4 mm kare, ortasında nokta; üstünde 2 mm sap, 2 mm kıvrım ve 2 mm ok; 0,2 mm (1/5000'de 2 × 2 mm). Kıvrımın "
         "ve ok başının biçimi çizimden.", ["radyo", "telsiz", "istasyon"])
    p = 101
    sign("0819", "Radar istasyonu", "N", [ring_(3, 0.2), dot(0.5), bpic("radar", 6, offset=[0, 8])], p,
         "3 mm çaplı daire, ortasında nokta; 4 mm sapın ucunda doğuya açık 5 mm çaplı yay; 0,2 mm (1/5000'de 1 mm, 2 mm, "
         "3 mm).", ["radar", "istasyon"])
    sign("0820", "TV istasyonu", "N", [ring_(3, 0.2), dot(0.5), bpic("tv-anten", 5.5, offset=[0, 3.5])], p,
         "3 mm çaplı daire, ortasında nokta; üstünde 4 mm anten: 2 mm sap, kısa kırık ve okla biten çizgi; 0,2 mm "
         "(1/5000'de 2 mm, 3 mm).", ["televizyon", "istasyon"])
    sign("0821", "Anten (şebeke)", "N", polyline([(-1, 0), (0, 2), (1, 0)], 0.2)
         + [seg(0, 2, 0, 9, 0.2), seg(-1, 7, 1, 7, 0.2),
            text({"expr": "varsayılan([Boy] || ' m', '')"}, 1.6, INK, weight=400, offset=[0, -1.2])], p,
         "0,2 mm: 2 mm açılan ayaklar, 5 mm direk, 2 mm genişliğinde kol, üstünde 2 mm uç (1/5000'de 0,16 mm). Antenin "
         "boyu (Boy alanı, metre) işaretin altına yazılır; yazının boyu yazılı değil, 1,6 mm.", ["anten", "haberleşme"])
    sign("0822", "Teleferik (telekabin)", "Ç", [sline(INK, 0.2), mline([shape("diamond", 1.6, fill=PAPER, stroke_=INK,
                                                                              width=0.2, height=3)],
                                                                       interval=20, offsetAlong=10)], p,
         "0,2 mm; 2 cm'de bir eşkenar dörtgen kabin (1/5000'de 1 cm'de bir). Kabin 3 mm; eni yazılı değil, çizimden "
         "1,6 mm.", ["teleferik", "telekabin"])
    sign("0823", "Teleferik (telesiyej)", "Ç", [sline(INK, 0.16),
                                                mline(polyline([(0, 1), (0, -2), (2, -2)]), interval=20, offsetAlong=10)],
         p, "0,16 mm; 2 cm'de bir 3 mm boyunda, 2 mm oturaklı koltuk (1/5000'de 1 cm'de bir, 2 ve 1 mm).",
         ["teleferik", "telesiyej"])
    sign("0824", "Şerare", "N", [bpic("serare", 9)], p,
         "Enerjiyi simgeleyen işaret: 4 mm'lik iki eğik çizgi, aralarında 2 mm basamak, ucunda ok; 0,2 mm (1/5000'de 2 ve "
         "1 mm). Ok başının boyu çizimden.", ["şerare", "enerji", "elektrik"])
    p = 102
    sign("0825", "Pilon", "N", [shape("square", 4, fill=PAPER, stroke_=INK, width=0.16), seg(-2, -2, 2, 2), seg(-2, 2, 2, -2)],
         p, "4 × 4 mm kare, köşeden köşeye çarpı; 0,16 mm (1/5000'de 2 × 2 mm). Tabanı 1 m²'den büyük enerji direkleri.",
         ["pilon", "enerji", "direk"])
    valve = [ring_(3), shape("x", 3, stroke_=INK, width=0.16), seg(0, -1.5, 0, -4.5)]
    sign("0826", "Doğal gaz dağıtım istasyonu", "A", [bpattern(valve, 12, 8, stagger=True)], p,
         "3 mm çaplı, içi çarpılı daire, altında 3 mm sap; 0,16 mm; komşu vanalar arası 1 cm (sıralar yarım "
         "kaydırılmış). Alanın sınırı kendi işaretiyle ayrıca çizilir.", ["doğal gaz", "istasyon"])
    sign("0827", "Vana", "N", valve + [text({"expr": "varsayılan([Cins], '')"}, 1.6, INK, weight=400, offset=[0, -5.6])],
         p, "3 mm çaplı, içi çarpılı daire, altında 3 mm sap; 0,16 mm (1/5000'de 2 mm). Vananın cinsi (Cins alanı) "
            "altına yazılır; yazının boyu yazılı değil, 1,6 mm.", ["vana", "petrol", "doğal gaz"])
    sign("0828", "Nükleer enerji ve nükleer etki alanı", "A", [centroid([bpic("nukleer", 12)])], p,
         "Alanın ortasında 1 cm yüksek, 1 cm geniş N; son çizgisi 2 mm okla biter; 0,16 mm (1/5000'de 0,5 cm). Alanın "
         "sınırı kendi işaretiyle ayrıca çizilir.", ["nükleer", "enerji"])
    sign("0829", "GSM baz istasyonu", "N", [dot(1), seg(0, 0.5, 0, 2.5, 0.2)], p,
         "1 mm çaplı dolu daire, üstünde 2 mm çizgi; 0,2 mm. Gazetenin çiziminde var, işaret listesinde yok.",
         ["GSM", "baz istasyonu", "haberleşme"])


def bounds() -> None:
    p = 103
    def dots(n: int, gap: float = 0.5) -> list[float]:
        return [gap] + [0.5, gap] * n
    sign("1001", "Devlet sınırı", "Ç", [sline(INK, 0.5, dash=[6, 1, 1, 1], cap="butt")], p,
         "0,5 mm; 6 mm çizgi, 1 mm boşluk, 1 mm kısa çizgi, 1 mm boşluk.", ["sınır", "devlet"])
    note = ("Çizgiler 5 mm, noktalar 1/2 mm, aralar 1/2 mm (tablonun yazdığı ölçüler; çizimde aralar abartılı "
            "çizilmiş, noktaların sayısı çizimden).")
    sign("1002", "İl sınırı", "Ç", [sline(INK, 0.3, dash=[5] + dots(1, 1), cap="butt")], p,
         "0,3 mm; 5 mm çizgi, 1 mm ara, 1/2 mm nokta, 1 mm ara.", ["sınır", "il"])
    for number, name, n, tag in (("1003", "İlçe sınırı", 2, "ilçe"), ("1004", "Bucak sınırı", 3, "bucak"),
                                 ("1005", "Köy sınırı", 4, "köy"), ("1006", "Belediye sınırı", 5, "belediye")):
        sign(number, name, "Ç", [sline(INK, 0.3, dash=[5] + dots(n), cap="butt")], p,
             f"0,3 mm; 5 mm çizgi ve {n} nokta. {note}", ["sınır", tag])
    p = 104
    sign("1007", "Mücavir alan sınırı", "Ç", [sline(INK, 0.3, dash=[5] + dots(6), cap="butt")], p,
         f"0,3 mm; 5 mm çizgi ve 6 nokta. {note}", ["sınır", "mücavir alan"])
    sign("1008", "Maden işletme sınırı", "Ç", [sline(INK, 0.3, dash=[3, 1]), ticks(1, 4, 0.25, above=True, offsetAlong=1.5)],
         p, "0,3 mm; 3 mm çizgi, 1 mm boşluk; her çizginin ortasında 1 mm yüksek, 0,25 mm dik çizgi.", ["sınır", "maden"])
    sign("1009", "Mülkiyet sınırı", "Ç", [sline(INK, 0.25)], p, "0,25 mm (1/5000'de 0,16 mm).", ["sınır", "mülkiyet"])
    sign("1010", "İtirazlı mülkiyet sınırı", "Ç", [sline(INK, 0.25, dash=[6, 1])], p,
         "0,25 mm; 6 mm çizgi, 1 mm boşluk (1/5000'de 4 mm).", ["sınır", "mülkiyet", "itiraz"])
    def semi(d: float, down: bool = False, width: float = 0.25) -> dict:
        return shape("semicircle", d, fill=None, stroke_=INK, width=width, **({"rotation": 180} if down else {}))
    sign("1011", "İrtifak hakkı", "Ç", [sline(INK, 0.25), mline([semi(2, True)], interval=5, offsetAlong=1.25),
                                        mline([semi(2)], interval=5, offsetAlong=3.75)], p,
         "0,25 mm; 2,5 mm'de bir, bir altta bir üstte 2 mm çaplı yarım daire. Başkasının arazisinden yol, elektrik "
         "hattı gibi geçiş hakkı.", ["irtifak", "mülkiyet"])
    sign("1012", "Çit, bir tarafa ait", "Ç", [sline(INK, 0.25), mline([semi(1.5)], interval=10, offsetAlong=5)], p,
         "0,25 mm; 1 cm'de bir 1,5 mm çaplı yarım daire. Yarım dairenin yönü çitin hangi tarafa ait olduğunu gösterir.",
         ["çit", "mülkiyet"])
    p = 105
    sign("1013", "Çit, iki tarafa ait", "Ç", [sline(INK, 0.25), mline([semi(1.5)], interval=10, offsetAlong=2.5),
                                              mline([semi(1.5, True)], interval=10, offsetAlong=7.5)], p,
         "0,25 mm; 5 mm'de bir, bir üstte bir altta 1,5 mm çaplı yarım daire.", ["çit", "mülkiyet"])
    vee = polyline([(-0.75, 0.75), (0, 0), (0.75, 0.75)], 0.25)
    hat = polyline([(-0.75, -0.75), (0, 0), (0.75, -0.75)], 0.25)
    sign("1014", "Tel örgü, bir tarafa ait", "Ç", [sline(INK, 0.25), mline(vee, interval=10, offsetAlong=5)], p,
         "0,25 mm; 1 cm'de bir çizgiye oturan 1,5 mm genişliğinde, 90° açılı V. İşaretin yönü ait olduğu tarafı "
         "gösterir.", ["tel örgü", "mülkiyet"])
    sign("1015", "Tel örgü, iki tarafa ait", "Ç", [sline(INK, 0.25), mline(vee, interval=10, offsetAlong=2.5),
                                                   mline(hat, interval=10, offsetAlong=7.5)], p,
         "0,25 mm; 5 mm'de bir, bir üstte bir altta 1,5 mm genişliğinde, 90° açılı V.", ["tel örgü", "mülkiyet"])
    tee = [seg(0, 0, 0, 1, 0.25), seg(-0.5, 1, 0.5, 1, 0.25)]
    foot = [seg(0, 0, 0, -1, 0.25), seg(-0.5, -1, 0.5, -1, 0.25)]
    sign("1016", "Parmaklık, bir tarafa ait", "Ç", [sline(INK, 0.25), mline(tee, interval=10, offsetAlong=5)], p,
         "0,25 mm; 1 cm'de bir 1 mm yüksek, 1 mm başlıklı T. İşaretin yönü ait olduğu tarafı gösterir.",
         ["parmaklık", "mülkiyet"])
    sign("1017", "Parmaklık, iki tarafa ait", "Ç", [sline(INK, 0.25), mline(tee, interval=10, offsetAlong=2.5),
                                                    mline(foot, interval=10, offsetAlong=7.5)], p,
         "0,25 mm; 5 mm'de bir, bir üstte bir altta 1 mm yüksek, 1 mm başlıklı T.", ["parmaklık", "mülkiyet"])
    sign("1018", "Sınır ve aidiyet ayırım yeri işareti", "N",
         [shape("line", 2, stroke_=INK, width=0.2, rotation={"expr": "90 + varsayılan(sayı([Açı]), 0)"})], p,
         "2 mm çizgi, 0,2 mm; yapısı farklı iki engelin birleştiği yerde sınıra dik konur. Açı alanıyla döner (0'da "
         "sınır doğu-batı).", ["sınır", "aidiyet"])
    p = 106
    sign("1019", "Duvar", "Ç", [sline(INK, 0.25), sline(INK, 0.1, offset=1), ticks(1, 7, 0.1, above=True, offsetAlong=3.5)],
         p, "0,25 mm kalın ve 1 mm ötede 0,1 mm ince çizgi, 7 mm'de bir aralarında dik çizgi. Duvar sınırdaysa kalın "
            "çizgi mülkiyet sınırını izler, ince çizgi duvarın ait olduğu parselde (çizginin solunda) kalır.",
         ["duvar", "mülkiyet"])
    below = shape("line", 1, stroke_=INK, width=0.2, rotation=90, offset=G.beside(0, -0.5, 90))
    sign("1020", "Çift duvar", "Ç", [sline(INK, 0.3), sline(INK, 0.2, offset=1), sline(INK, 0.2, offset=-1),
                                     ticks(1, 7, 0.2, above=True, offsetAlong=3.5),
                                     mline([below], interval=7, offsetAlong=7)], p,
         "Ortada 0,3 mm kalın çizgi, 1 mm ötelerinde 0,2 mm çizgiler; iki yanda 7 mm'de bir, yarım kaydırılmış dik "
         "çizgiler. Sınır üzerindeki çizgi kalın çizgidir.", ["duvar", "mülkiyet"])
    sign("1021", "İstinat duvarı", "Ç", [sline(INK, 0.2, offset=0.5), sline(INK, 0.2, offset=-0.5), ticks(1, 7, 0.2)], p,
         "Aralarında 1 mm olan iki 0,2 mm çizgi, 7 mm'de bir aralarında dik çizgi; iki yan aynı kalınlıkta (1/5000'de "
         "0,5 mm, 4 mm).", ["duvar", "istinat"])
    dash2 = shape("line", 2, stroke_=INK, width=0.25)
    sign("1022", "Tonç (bir tarafa ait)", "Ç", [sline(INK, 0.25), mline([dict(dash2, offset=[0, 0.5])], interval=15,
                                                                         offsetAlong=7.5)], p,
         "0,25 mm; 1,5 cm'de bir, 0,5 mm ötede çizgiye koşut 2 mm kısa çizgi. Parseller arasındaki mülkiyet ayrım "
         "yükseltisi; kısa çizginin yönü ait olduğu tarafı gösterir.", ["tonç", "mülkiyet"])
    sign("1023", "Tonç (iki tarafa ait)", "Ç", [sline(INK, 0.25), mline([dict(dash2, offset=[0, 0.5]),
                                                                         dict(dash2, offset=[0, -0.5])],
                                                                        interval=15, offsetAlong=7.5)], p,
         "0,25 mm; 1,5 cm'de bir iki yanında, aralarında 1 mm olan 2 mm kısa çizgiler.", ["tonç", "mülkiyet"])
    sign("1024", "Aidiyet (ait olma) işareti", "N", [bpic("aidiyet", 2.6, rotation=G.TURN)], p,
         "2 mm kıvrım, 0,16 mm; konduğu çizginin mülkiyet sınırı olmadığını gösterir. Açı alanıyla döner (0'da çizgi "
         "doğu-batı).", ["aidiyet", "mülkiyet"])
    p = 107
    sign("1025", "Sit alanı sınırı", "Ç", [sline(INK, 0.16, dash=[2, 1, 15, 1]),
                                           mline([ring_(1, fill=None)], interval=19, offsetAlong=1)], p,
         "0,16 mm; 2 mm çizginin ortasında 1 mm çaplı daire, 1 mm boşluk, 1,5 cm çizgi, 1 mm boşluk (1/5000'de 1 cm).",
         ["sınır", "sit"])
    strokes = [shape("line", 2.128, stroke_=INK, width=0.16, rotation=70, offset=G.beside(a, 0, 70)) for a in (-0.5, 0, 0.5)]
    sign("1026", "Milli park sınırı", "Ç", [sline(INK, 0.16, dash=[15, 1]), mline(strokes, interval=16, offsetAlong=7.5)], p,
         "0,16 mm; 1,5 cm çizgi, 1 mm boşluk; her çizginin ortasında 2 mm yüksek üç eğik çizgi. Eğik çizgilerin açısı "
         "ve aralığı çizimden (70°, 0,5 mm).", ["sınır", "milli park"])
    sign("1027", "Harap duvar", "Ç", [sline(INK, 0.25), sline(INK, 0.1, offset=1, dash=[2, 1]),
                                      ticks(1, 7, 0.1, above=True, offsetAlong=3.5)], p,
         "0,25 mm kalın çizgi, 1 mm ötede 0,1 mm kesik ince çizgi (2 mm çizgi, 1 mm boşluk), 7 mm'de bir aralarında dik "
         "çizgi. Gazetenin çiziminde var, işaret listesinde yok.", ["duvar", "harap"])


def buildings() -> None:
    p = 108
    sign("1201", "Mesken (konut) bina", "A", [outline(0.2)], p, "0,2 mm (1/5000'de 0,16 mm).", ["bina", "konut"])
    official = [hatch(45, 1.5, 0.1, INK), outline(0.2)]
    hatch_note = "0,2 mm, içi 1,5 mm aralıkla taranır (1/5000'de 1 mm). Taramanın kalınlığı yazılı değil, 0,1 mm."
    sign("1202", "Resmî bina", "A", official, p, f"{hatch_note} Kamu binasının adı içine, sığmazsa uygun yere yazılır.",
         ["bina", "resmî", "kamu"])
    sign("1203", "Okul", "A", official, p, f"{hatch_note} Okulun adı binanın uygun yerine yazılır.", ["bina", "okul"])
    sign("1204", "Radyo TV binası", "A", official, p, f"{hatch_note} Binanın adı (kullanılış amacı) uygun yere yazılır.",
         ["bina", "radyo", "televizyon"])
    sign("1205", "Fabrika", "A", [outline(0.2)], p, "0,2 mm (1/5000'de 0,16 mm).", ["bina", "fabrika"])
    sign("1206", "İmalathane", "A", [outline(0.2)], p, "0,2 mm. İmalathanenin adı binanın uygun yerine yazılır.",
         ["bina", "imalathane"])
    p = 109
    sign("1207", "Cami-mescit", "N", [bpic("cami", 7.6)], p,
         "7 mm taban üstünde 4,5 mm çaplı kubbe, tepesinde 1,5 mm sap ve 2 mm çaplı hilal; 0,3 mm (1/5000'de 3, 2,5, 1 ve "
         "1 mm; 0,2 mm).", ["cami", "mescit", "ibadet"])
    sign("1208", "Türbe", "N", [bpic("turbe", 4.6)], p,
         "4 mm taban üstünde 1,5 mm sap ve 3 mm çaplı hilal; 0,3 mm (1/5000'de 2, 1 ve 2 mm; 0,2 mm).", ["türbe"])
    sign("1209", "Kilise", "N", [seg(-3, 0, 3, 0, 0.2), seg(1, -1.5, 1, 1.5, 0.2)], p,
         "Yatık haç: 4 + 2 mm uzun kol, 3 mm kısa kol; 0,2 mm (1/5000'de 3 + 1,5 ve 2 mm).", ["kilise", "ibadet"])
    sign("1210", "Havra", "N", [shape("triangle", 4, fill=None, stroke_=INK, width=0.2),
                                shape("triangle", 4, fill=None, stroke_=INK, width=0.2, rotation=180), dot(0.5)], p,
         "4 mm genişliğinde altı köşeli yıldız, ortasında nokta; 0,2 mm (1/5000'de 3 mm).", ["havra", "sinagog", "ibadet"])
    sign("1211", "İnşa hâlinde veya harap bina", "A", [outline(0.2, dash=[3, 1])], p,
         "0,2 mm; 3 mm çizgi, 1 mm boşluk (1/5000'de 2 mm).", ["bina", "inşaat", "harap"])
    sign("1212", "Sundurma", "A", [outline(0.16), centroid([shape("x", 3, stroke_=INK, width=0.16)])], p,
         "0,16 mm; içi köşeden köşeye çarpı. Üstü kapalı, duvarlarından en az biri olmayan yapı. KentOS çarpıyı alanın "
         "ortasına 3 mm çizer.", ["sundurma"])
    p = 110
    sign("1213", "Sera", "A", [outline(0.16), sline(INK, 0.16, offset=1),
                               mline([seg(0, 0, 0, 1.414)], placement="vertex")], p,
         "0,16 mm; içinde kenarlardan a/4 içeride ikinci bir dikdörtgen, köşeleri birleştiren çizgiler. KentOS iç "
         "çizgiyi 1 mm içeride çizer.", ["sera"])
    sign("1214", "Tahıl silosu (ölçeğe sığan)", "A", [outline(0.2)], p, "0,2 mm. Silolar ölçüsüyle çizilir.", ["silo"])
    sign("1215", "Tahıl silosu (ölçeğe sığmayan)", "N", [ring_(4, 0.2), dot(0.5)], p,
         "4 mm çaplı daire, ortasında nokta; 0,2 mm (1/5000'de 2 mm).", ["silo"])
    sign("1216", "Akaryakıt tankı (ölçeğe sığan)", "A", [hatch(45, 1, 0.1, INK), outline(0.2)], p,
         "0,2 mm, içi taralı. Taramanın aralığı ve kalınlığı yazılı değil, 1 mm ve 0,1 mm.", ["tank", "akaryakıt"])
    sign("1217", "Akaryakıt tankı (ölçeğe sığmayan)", "N", [bpic("akaryakit", 4.4)], p,
         "4 mm çaplı, içi taralı daire; 0,2 mm (1/5000'de 2 mm). Taramanın aralığı çizimden 0,8 mm.", ["tank", "akaryakıt"])
    sign("1218", "Ağıl", "Ç", [sline(INK, 0.2), mline([shape("square", 1, fill=None, stroke_=INK, width=0.2,
                                                             offset=[0, -0.5])], placement="segmentCenter")], p,
         "0,2 mm; kenarına bitişik 1 × 1 mm kare (1/5000'de 0,5 mm). Karenin yeri yazılı değil; örnekteki gibi KentOS her "
         "kenarın ortasına koyar.", ["ağıl", "hayvancılık"])
    p = 111
    sign("1219", "Otopark", "N", [text("P", 3, INK, weight=400)], p,
         "3 mm yüksek P, 0,4 mm kalınlıkta (1/5000'de 2 mm, 0,2 mm). Otopark binası ya da ayrılmış alanı.",
         ["otopark", "park"])
    sign("1220", "Değirmen (elektrikli)", "N", [bpic("degirmen-elektrik", 10, offset=[2.3, 1.8])], p,
         "4 mm çaplı dişli daire, ortasında nokta; dışına 4 + 1 + 4 mm şerare; 0,2 mm (1/5000'de 3 mm).",
         ["değirmen", "elektrik"])
    sign("1221", "Değirmen (su ile çalışan)", "N", [bpic("degirmen-su", 5.4, offset=[0, 1.2])], p,
         "4 mm çaplı dişli yarım daire, ortasında nokta; 0,2 mm (1/5000'de 3 mm).", ["değirmen", "su"])
    sign("1222", "Değirmen (yel)", "N", [dot(0.3), seg(-1, -0.6, -0.75, 1.5, 0.2), seg(1, -0.6, 0.75, 1.5, 0.2),
                                        seg(-1.5, 0.75, 1.5, 3.75, 0.2), seg(-1.5, 3.75, 1.5, 0.75, 0.2)], p,
         "2 mm ayaklar ve tepelerinde 3 mm çarpı (kanatlar), ortada nokta; 0,2 mm (1/5000'de 1 ve 2 mm).",
         ["değirmen", "yel"])
    sign("1223", "Havaalanı pisti", "Ç", [sline(INK, 0.2)], p, "0,2 mm (1/5000'de 0,16 mm).", ["havaalanı", "pist"])
    sign("1224", "Helikopter pisti", "N", [ring_(10, 0.2, fill=None), text("H", 4.5, INK, weight=400)], p,
         "1 cm çaplı daire, içinde H; 0,2 mm (1/5000'de 5 mm). H'nin boyu yazılı değil, çizimden.", ["helikopter", "pist"])
    p = 112
    sign("1225", "Spor tesisleri", "A", [outline(0.16)], p, "0,16 mm. Ölçülerine uygun çizilir.", ["spor", "stadyum"])
    sign("1226", "Kale-hisar (sağlam)", "Ç", [sline(INK, 0.2, offset=0.5), sline(INK, 0.2, offset=-0.5),
                                              ticks(1, 7, 0.2)], p,
         "Aralarında 1 mm olan iki 0,2 mm çizgi, 7 mm'de bir aralarında dik çizgi (1/5000'de 2 mm). Ölçüsüyle çizilir, "
         "varsa adı yazılır. İki çizginin arası yazılı değil, 1 mm.", ["kale", "hisar", "tarihî"])
    sign("1227", "Kale-hisar (harap)", "Ç", [sline(INK, 0.2, offset=0.5, dash=[7, 1]),
                                             sline(INK, 0.2, offset=-0.5, dash=[7, 1]), ticks(1, 5, 0.2)], p,
         "1226'nın kesik hâli: 7 mm çizgi, 1 mm boşluk, 5 mm'de bir dik çizgi; 0,2 mm (1/5000'de 2, 1 ve 3 mm). Ölçülerin "
         "yeri çizimde belirsiz.", ["kale", "hisar", "harap", "tarihî"])
    sign("1228", "Tarihî harabe (şekli belli)", "A", [outline(0.25, dash=[4, 1])], p,
         "0,25 mm; 4 mm çizgi, 1 mm boşluk (1/5000'de 2 mm).", ["harabe", "tarihî"])
    sign("1229", "Tarihî harabe (şekli belli olmayan)", "N",
         [dot(0.5, x, y) for x, y in ((-1, 1), (1, 1), (0, 0), (0, -1.5))], p,
         "Aralarında yaklaşık 2 mm olan 0,5 mm çaplı noktalar (1/5000'de 0,3 mm). Dizilişi çizimden.", ["harabe", "tarihî"])
    sign("1230", "Harap yerleşim alanı veya harap bina", "A", [outline(0.2, dash=[2, 1])], p,
         "0,2 mm; 2 mm çizgi, 1 mm boşluk. Kullanılmaz hâldeki yerleşim alanı.", ["harabe", "yerleşim"])
    p = 113
    grave = polyline([(-1.5, 0.5), (-1.5, -0.5), (1.5, -0.5), (1.5, 0.5)])
    cross = [seg(0, -1.5, 0, 1.5), seg(-1, 0.9, 1, 0.9)]
    jewish = polyline([(-1.5, 0.5), (-1.5, -0.5), (1.5, -0.5)])
    sign("1231", "Mezarlık (İslam)", "A", [bpattern(grave, 5.5, 2.5, stagger=True)], p,
         "Sıralarda 2,5 mm arayla 3 × 1 mm mezar işareti, sıralar arası 1,5 mm, yarım kaydırılmış; 0,16 mm. Mezar işareti "
         "1234'ün ölçüsündedir.", ["mezarlık", "İslam"])
    sign("1232", "Mezarlık (Hristiyan)", "A", [bpattern(cross, 4.5, 4, stagger=True)], p,
         "Sıralarda 4,5 mm arayla 3 mm haç (2 mm kol), sıralar arası 1 mm, yarım kaydırılmış; 0,16 mm.",
         ["mezarlık", "Hristiyan"])
    sign("1233", "Mezarlık (Musevi)", "A", [bpattern(jewish, 4.5, 2.5, stagger=True)], p,
         "Sıralarda 1,5 mm arayla 3 × 1 mm mezar işareti, sıralar arası 1,5 mm, yarım kaydırılmış; 0,16 mm.",
         ["mezarlık", "Musevi"])
    sign("1234", "Tek mezar (İslam)", "N", polyline([(-1.5, 0.5), (-1.5, -0.5), (1.5, -0.5), (1.5, 0.5)], 0.2), p,
         "3 mm taban, iki ucunda 1 mm; 0,2 mm (1/5000'de 2 mm).", ["mezar", "İslam"])
    sign("1235", "Tek mezar (Hristiyan)", "N", [seg(0, 0, 0, 3, 0.2), seg(-1, 2.4, 1, 2.4, 0.2), dot(0.3)], p,
         "3 mm haç, 2 mm kol; ayağında mezarın yerini gösteren nokta; 0,2 mm (1/5000'de 2 ve 1 mm).", ["mezar", "Hristiyan"])
    sign("1236", "Tek mezar (Musevi)", "N", polyline([(-1.5, 0.5), (-1.5, -0.5), (1.5, -0.5)], 0.2), p,
         "3 mm taban, sol ucunda 1 mm; 0,2 mm (1/5000'de 2 mm).", ["mezar", "Musevi"])


def vegetation() -> None:
    p = 114
    every = "1 cm arayla, sıralar yarım kaydırılmış"
    edge = "Alanın sınırı orman sınırıyla (1401) ya da kendi işaretiyle ayrıca çizilir."
    def deciduous(x: float = 0, y: float = 0) -> list[dict]:
        return [ring_(1.5, 0.2, x=x, y=y, fill=None), seg(x, y - 0.75, x + 1.5, y - 0.75, 0.2)]

    def needle(x: float = 0, y: float = 0) -> list[dict]:
        return [shape("triangle", 2, fill=None, stroke_=INK, width=0.2, **({"offset": [x, y]} if x or y else {})),
                seg(x + 1, y - 0.577, x + 2, y - 0.577, 0.2)]
    tree, conifer = deciduous(), needle()
    sign("1401", "Orman sınırı", "Ç", [mline([dot(0.5)], interval=2, offsetAlong=1)], p,
         "2 mm'de bir 0,5 mm çaplı nokta.", ["orman", "sınır"])
    sign("1402", "Yaprağını döken ağaç (grup)", "A", [bpattern(tree, 10, 10, stagger=True)], p,
         f"1,5 mm çaplı daire ve altından sağa kuyruk, {every}; 0,2 mm (1/5000'de 1 mm, 4 mm). {edge}",
         ["orman", "yaprak döken", "ağaç"])
    sign("1403", "Yaprağını dökmeyen ağaç (grup)", "A", [bpattern(conifer, 10, 10, stagger=True)], p,
         f"2 mm üçgen ve tabanından sağa kuyruk, {every}; 0,2 mm. {edge}", ["orman", "iğne yapraklı", "ağaç"])
    sign("1404", "Çalılık", "A", [bpattern([bpic("calilik", 3.6)], 10, 10, stagger=True)], p,
         f"3 mm genişliğinde, 2,5 mm yüksek kıvrık kemer, {every}; 0,2 mm (1/5000'de 4 mm). {edge}", ["çalılık", "çalı"])
    sign("1405", "Meyvalık", "A", [bpattern([bpic("meyvalik", 2)], 10, 10, stagger=True)], p,
         f"1,5 mm genişliğinde kemer, {every}; 0,2 mm (1/5000'de 1 mm, 4 mm). {edge}", ["meyvalık", "bahçe"])
    sign("1406", "Bağlık", "A", [bpattern([seg(0, -1.5, 0, 1.5, 0.2)], 10, 10, stagger=True)], p,
         f"3 mm dik çizgi, {every}; 0,2 mm (1/5000'de 2 mm, 4 mm). {edge}", ["bağ", "üzüm"])
    p = 115
    for number, name, pic_, tags in (("1407", "Zeytinlik", "zeytin", ["zeytin"]), ("1408", "Fındıklık", "findik", ["fındık"]),
                                     ("1409", "Fıstıklık", "fistik", ["fıstık"]), ("1412", "Gül bahçesi", "gul", ["gül"])):
        sign(number, name, "A", [bpattern([bpic(pic_, 3.6)], 10, 10, stagger=True)], p,
             f"3 mm yüksek simge, {every}; 0,2 mm (1/5000'de 2 mm, 4 mm). Simgenin biçimi çizimden. {edge}",
             tags + ["bahçe"])
    sign("1410", "Turunçgil bahçesi", "A", [bpattern([seg(0, -1.5, 0, 0, 0.2), ring_(1.5, 0.2, y=0.75, fill=None)], 10, 10,
                                                    stagger=True)], p,
         f"1,5 mm sap üstünde 1,5 mm çaplı daire (3 mm), {every}; 0,2 mm. {edge}", ["turunçgil", "narenciye", "bahçe"])
    sign("1411", "Çay bahçesi", "A", [bpattern(polyline([(0, 1.5), (0, -1.5)], 0.2)
                                              + polyline([(-0.5, -0.7), (0, -1.5), (0.5, -0.7)], 0.2), 10, 10,
                                              stagger=True)], p,
         f"3 mm aşağı ok, {every}; 0,2 mm (1/5000'de 2 mm, 4 mm). {edge}", ["çay", "bahçe"])
    p = 116
    sign("1413", "Sebze bahçesi", "A", [bpattern([dot(0.6, -0.9, 0), dot(0.6, 0.9, 0)], 10, 10, stagger=True)], p,
         f"Aralarında 1,8 mm olan 0,6 mm çaplı iki nokta, {every}; 0,16 mm. Noktaların dizilişi çizimden. {edge}",
         ["sebze", "bahçe"])
    sign("1414", "Çayır", "A", [bpattern([seg(-0.75, -1, 0.75, -1, 0.2), seg(-0.8, 1, 0.3, -1, 0.2), seg(0.8, 1, -0.3, -1, 0.2)],
                                        10, 10, stagger=True)], p,
         f"2 mm yüksek ot öbeği, {every}; 0,2 mm (1/5000'de 1,5 mm, 4 mm). {edge}", ["çayır", "mera"])
    sign("1415", "Sazlık", "A", [bpattern([bpic("sazlik", 4)], 10, 7, stagger=True)], p,
         f"3 mm çaplı yarım daire üstünde kısa çizgiler; sıralarda 1 cm, sıralar 7 mm arayla, yarım kaydırılmış; 0,2 mm "
         f"(1/5000'de 2, 4 ve 3 mm). {edge}", ["sazlık", "sulak"])
    sign("1416", "Kamışlık", "A", [bpattern([seg(-0.5, -1.5, -0.5, 1.5, 0.2), seg(0.5, -1.5, 0.5, 1.5, 0.2)], 10, 10,
                                           stagger=True)], p,
         f"Aralarında 1 mm olan 3 mm iki dik çizgi, {every}; 0,2 mm (1/5000'de 2 mm, 4 mm). {edge}", ["kamışlık", "sulak"])
    sign("1417", "Karışık orman", "A", [bpattern(deciduous(-2.5, 2.5) + needle(2.5, -2.5), 10, 10)], p,
         f"Yaprağını döken (1402) ve dökmeyen (1403) ağaç işaretleri sıra sıra, 1 cm arayla; 0,2 mm. {edge}",
         ["orman", "karışık", "ağaç"])
    sign("1418", "Yanmış orman", "A", [bpattern(polyline([(0, -1), (0, 0.2)], 0.2) + polyline([(-0.6, 1), (0, 0.2), (0.6, 1)], 0.2),
                                               10, 10, stagger=True)], p,
         f"2 mm Y, {every}; 0,2 mm (1/5000'de 1,5 mm, 4 mm). {edge}", ["orman", "yanmış"])
    p = 117
    sign("1419", "Fidanlık", "A", [bpattern(tree, 10, 10, stagger=True)], p,
         f"1,5 mm çaplı daire ve kuyruk, {every}; 0,2 mm. Çizimi 1402 ile aynıdır. {edge}", ["fidanlık", "ağaç"])
    sign("1420", "Yangın önleme şeridi", "Ç", [mline([dot(0.3)], interval=2, offsetAlong=1)], p,
         "2 mm'de bir nokta. Noktanın çapı yazılı değil, 0,3 mm. Şeridin iki kenarı bu çizgiyle çizilir.",
         ["orman", "yangın"])
    sign("1421", "Park bahçe", "A", [outline(0.2, dash=[1, 0.5])], p, "0,2 mm; 1 mm çizgi, 0,5 mm boşluk.", ["park", "bahçe"])
    sign("1422", "Büyük münferit ağaç (iğne yapraklı)", "N",
         [seg(0, 0, 0, 7, 0.2)] + polyline([(-0.8, 5.6), (0, 7), (0.8, 5.6)], 0.2)
         + polyline([(-1, 3.2), (0, 5), (1, 3.2)], 0.2) + [ring_(0.4, 0.2, fill=None)], p,
         "7 mm gövde, üstünde üst üste iki ok ucu, ayağında ağacın yerini gösteren küçük daire; 0,2 mm (1/5000'de 4 mm). "
         "Ok uçlarının boyu çizimden.", ["ağaç", "iğne yapraklı", "münferit"])
    sign("1423", "Büyük münferit ağaç (geniş yapraklı)", "N", [bpic("genis-agac", 7.6, offset=[0, 3.6])], p,
         "7 mm yüksek, 4 mm geniş dalgalı taç ve gövde, ayağında ağacın yerini gösteren küçük daire; 0,2 mm (1/5000'de 4 "
         "ve 2 mm). Tacın biçimi çizimden.", ["ağaç", "geniş yapraklı", "münferit"])
    sign("1424", "Tek ağaç", "N", tree, p, "1,5 mm çaplı daire, altından sağa 1,5 mm kuyruk; 0,2 mm (1/5000'de 1 mm).",
         ["ağaç", "tek"])
    p = 118
    sign("1425", "Tek çalı", "N", [bpic("cali", 3.6, offset=[0, 0.3]), dot(0.3)], p,
         "3 mm genişliğinde, 2 mm yüksek üç dilimli çalı, ortasında nokta; 0,2 mm (1/5000'de 2 ve 1 mm).", ["çalı", "tek"])
    sign("1426", "Yeşil çit", "Ç", [mline([bpic("kemer", 2.4)], interval=2, offsetAlong=1)], p,
         "2 mm çaplı yarım daireler art arda; 0,2 mm (1/5000'de 1,5 mm). Bitkilerin yetiştirilmesiyle oluşan çit.",
         ["çit", "yeşil çit", "bitki"])


def rocks() -> None:
    p = 119
    def rock(x: float = 0, y: float = 0, w: float = 3, h: float = 4, width: float = 0.16) -> list[dict]:
        return polyline([(x - w / 2, y + h / 2), (x + w / 2, y + h / 2), (x, y - h / 2), (x - w / 2, y + h / 2)], width)
    sign("1601", "Tek kaya", "N", rock(), p, "3 mm genişliğinde, 4 mm yüksek ters üçgen; 0,16 mm (1/5000'de 2 ve 3 mm).",
         ["kaya"])
    sign("1602", "Sıra kaya", "Ç", [mline(rock(width=0.2), interval=10, offsetAlong=2.5, rotate=False),
                                    mline(rock(y=-1.5, width=0.2), interval=10, offsetAlong=7.5, rotate=False)], p,
         "Aralarında 2 mm olan 3 × 4 mm ters üçgenler, biri yukarıda biri aşağıda; 0,2 mm (1/5000'de 1, 2 ve 3 mm). "
         "Çizgi yalnız üçgenleri dizer, kendisi çizilmez; üçgenler çizgiyle dönmez, hep aşağı bakar.", ["kaya", "sıra"])
    sign("1603", "Taşlık", "A", [bpattern([dot(0.2)], 4, 4, stagger=True),
                                 bpattern([ring_(0.5, 0.16, fill=None)], 10, 10, stagger=True)], p,
         "4 mm arayla küçük noktalar (0,16 mm kalemle; KentOS 0,2 mm çizer) ve 1 cm arayla 0,5 mm çaplı halkalar "
         "(1/5000'de 2 mm ve 5 mm).", ["taşlık", "taş"])
    sign("1604", "Kumluk", "A", [bpattern([dot(0.2)], 4, 4)], p,
         "4 mm arayla düzgün sıralı 0,2 mm çaplı noktalar (1/5000'de 2 mm).", ["kumluk", "kum"])
    plus = shape("cross", 2, stroke_=INK, width=0.2)
    sign("1605", "Suda kaya (ölçeğe sığan)", "A", [outline(0.2), centroid([plus])], p,
         "0,2 mm; içine 1606'nın artısı yaklaşık zirveye konur (KentOS alanın ortasına koyar). Ada sayılmayacak kadar "
         "küçük, ölçeğe sığan kayalar.", ["kaya", "su"])
    sign("1606", "Suda kaya (ölçeğe sığmayan)", "N", [plus], p,
         "2 × 2 mm artı, 0,2 mm (1/5000'de 0,16 mm). Denizde, gölde ve nehirde ölçeğe sığmayan kayalar.", ["kaya", "su"])
    p = 120
    sign("1607", "Heyelan, göçük bölgesi", "A", [hatch(90, 2, 0.16, INK), outline(0.2)], p,
         "0,2 mm; içi 2 mm arayla dikey taranır (1/5000'de 1 mm). Taramanın kalınlığı yazılı değil, 0,16 mm.",
         ["heyelan", "göçük"])
    sign("1608", "Lavlı ve leçelik arazi", "A", [outline(0.2, dash=[3, 1]),
                                                 bpattern([seg(0, -2.5, 0, 2.5, 0.2)], 10, 10, stagger=True)], p,
         "Kesik sınır (1 mm boşluk; çizginin boyu çizimden 3 mm), içinde 1 cm arayla 5 mm dik çizgiler, sıralar yarım "
         "kaydırılmış; 0,2 mm (1/5000'de 4 mm).", ["lav", "volkanik"])
    sign("1609", "Traverten", "A", [bpattern([seg(-2, 0, 2, 0, 0.2)], 10, 4, stagger=True)], p,
         "4 mm kısa çizgiler, sırada aralarında 6 mm, sıralar 4 mm arayla ve yarım kaydırılmış; 0,2 mm (1/5000'de 2, 3 ve "
         "2 mm).", ["traverten"])
    sign("1610", "Blok kaya", "A", [outline(0.2, dash=[3, 1]), bpattern(rock(w=3, h=3, width=0.2), 15, 10, stagger=True)], p,
         "Kesik sınır (1 mm boşluk; çizginin boyu çizimden 3 mm), içinde 1,5 cm arayla 3 mm ters üçgenler, sıralar 1 cm "
         "arayla ve yarım kaydırılmış; 0,2 mm.", ["kaya", "blok"])


def slopes() -> None:
    p = 121
    def hanging(length: float, side: int = -1, width: float = 0.1) -> dict:
        return shape("line", length, stroke_=INK, width=width, rotation=90, offset=G.beside(0, side * length / 2, 90))
    def hachures(side: int = -1) -> list[dict]:
        return [mline([hanging(5, side)], interval=2.5), mline([hanging(2.5, side)], interval=2.5, offsetAlong=1.25)]
    slope_note = ("Uzun (5 mm) ve kısa (2,5 mm) 0,1 mm taramalar art arda; taramanın aralığı yazılı değil, KentOS 1,25 mm "
                  "çizer. Tabloda alan (A) işaretidir; KentOS şev kenarına çizilen çizgi işareti olarak verir, taramalar "
                  "çizginin sağına düşer.")
    sign("1801", "Hendek (ölçeğe sığan)", "A", [outline(0.2, dash=[4, 1]), mline([hanging(1, 1)], interval=2.5)], p,
         "Hendeğin üstü ve tabanı kesik çizgiyle (1 mm boşluk), şev kuralıyla taranır; 0,2 mm, 5 mm geniş (1/5000'de 3 mm). "
         "KentOS kenarı kesik çizer, içine 2,5 mm'de bir 1 mm tarama koyar.", ["hendek"])
    sign("1802", "Hendek (ölçeğe sığmayan)", "Ç", [sline(INK, 0.2), mline([shape("line", 4, stroke_=INK, width=0.2,
                                                                                offset=[0, 0.6])],
                                                                         interval=20, offsetAlong=10)], p,
         "0,2 mm; üstünde ona koşut 4 mm kısa çizgi (1/5000'de 3 mm). KentOS kısa çizgiyi 2 cm'de bir yineler.", ["hendek"])
    sign("1803", "Set (ölçeğe sığan)", "A", [outline(0.2, dash=[4, 1]), centroid([shape("cross", 2, stroke_=INK, width=0.2)])],
         p, "Setin üstü ve tabanı kesik çizgiyle (1 mm boşluk), şev kuralıyla taranır; 0,2 mm, 5 mm geniş. KentOS kenarı "
            "kesik çizer, ortasına 2 mm artı koyar.", ["set"])
    sign("1804", "Set (ölçeğe sığmayan)", "Ç", [sline(INK, 0.2), mline([hanging(2, 1, 0.2)], interval=10),
                                                mline([hanging(2, -1, 0.2)], interval=10, offsetAlong=5)], p,
         "0,2 mm; 5 mm'de bir, bir üstte bir altta 2 mm dik çizgi (1/5000'de 3 ve 1 mm).", ["set"])
    sign("1805", "Şev (yarma)", "Ç", [sline(INK, 0.2, dash=[7, 1])] + hachures(), p,
         "Şev kenarı 0,2 mm; duvar, yol gibi ayrıntı yoksa kesik çizgi (7 mm çizgi, 1 mm boşluk). " + slope_note,
         ["şev", "yarma"])
    sign("1806", "Şev (dolma)", "Ç", [sline(INK, 0.2, dash=[7, 1])] + hachures(), p,
         "Çizimi 1805 ile aynıdır. " + slope_note, ["şev", "dolma"])
    p = 122
    sign("1807", "Şev (alt kenarı belirsiz)", "Ç", [sline(INK, 0.2)] + hachures(), p,
         "Alt kenarı ölçülmemiş şev: üst kenar düz 0,2 mm, taramalar serbest biter. " + slope_note, ["şev"])
    sign("1808", "Şev (üst kenarı belirsiz)", "Ç", [sline(INK, 0.2, dash=[7, 1])] + hachures(1), p,
         "Üst kenarı ölçülmemiş şev: çizgi alt kenardır, taramalar ondan yukarı (çizginin soluna) çıkar. " + slope_note,
         ["şev"])
    sign("1809", "Dik şev", "Ç", [sline(INK, 0.2), mline([hanging(1.5, -1, 0.2)], interval=2.5, offsetAlong=1.25)], p,
         "0,2 mm; 2,5 mm'de bir 1,5 mm dik çizgi (1/5000'de 1,5 ve 1 mm).", ["şev", "dik"])
    sign("1810", "Şevle gösterilen ocak (kum, taş, kireç, kil, tuğla)", "A",
         [outline(0.2), mline([shape("line", 1, stroke_=INK, width=0.1, rotation=90, offset=G.beside(0, 0.5, 90))],
                              interval=1)], p,
         "0,2 mm; kenarı boyunca içe dönük 1 mm taramalar. Taramaların aralığı çizimden 1 mm.", ["ocak", "taş ocağı"])
    sign("1811", "Höyük (ölçeğe sığmayan)", "N", [bpic("hoyuk", 3.4)], p,
         "3 mm çaplı ışınlı yıldız, ortasında nokta; 0,2 mm (1/5000'de 1 mm).", ["höyük", "tarihî"])


def complements() -> None:
    p = 123
    base = [ring_(1, 0.2), dot(0.25)]
    sign("2001", "Telefon direği", "N", base + [seg(0, 0.5, 0, 5.5, 0.2), seg(-0.5, 4.2, 0.5, 5.0, 0.2)], p,
         "1 mm çaplı daire (direğin yeri), 5 mm sap, ucuna yakın eğik kısa çizgi; 0,2 mm (1/5000'de 3 mm).",
         ["direk", "telefon"])
    sign("2002", "Elektrik direği", "N", base + [seg(0, 0.5, 0, 5.5, 0.2), seg(0, 5.5, 2, 5.5, 0.2), seg(2, 5.5, 2, 5.1, 0.2)]
         + polyline([(1.7, 5.1), (2.3, 5.1), (2, 4.6), (1.7, 5.1)], 0.2), p,
         "1 mm çaplı daire, 5 mm sap, ucunda 2 mm kol ve kolun ucunda küçük üçgen (izolatör); 0,2 mm (1/5000'de 1,5 ve "
         "3 mm).", ["direk", "elektrik"])
    sign("2003", "Cadde ve sokak lambası", "N", base + [seg(0, 0.5, 0, 5.5, 0.2),
                                                        shape("semicircle", 2, fill=None, stroke_=INK, width=0.2,
                                                              rotation=180, offset=G.beside(0, 6.5, 180))], p,
         "1 mm çaplı daire, 5 mm sap, ucunda 2 mm çaplı yarım daire (lamba); 0,2 mm (1/5000'de 1,5 ve 3 mm).",
         ["lamba", "aydınlatma"])
    diag = [seg(0.6 * math.cos(math.radians(a)), 0.6 * math.sin(math.radians(a)), 1.5 * math.cos(math.radians(a)),
                1.5 * math.sin(math.radians(a)), 0.2) for a in (45, 135, 225, 315)]
    sign("2004", "Pist ışıklandırma lambası", "N", [ring_(1.2, 0.2), dot(0.25)] + diag, p,
         "3 mm çapında: küçük daire, ortasında nokta, köşegenlerde dört kısa çizgi; 0,2 mm (1/5000'de 2 mm).",
         ["lamba", "pist", "havaalanı"])
    sign("2005", "Vinç (sabit)", "N", [ring_(2, 0.2), dot(0.3), bpic("vinc", 13, offset=[2, 3.5])], p,
         "2 mm çaplı daire, 5 mm direk, tepesinde dönüş yayı ve eğik kol; 0,2 mm (1/5000'de 1 ve 3 mm).", ["vinç"])
    sign("2006", "Vinç (raylı)", "N", [ring_(2, 0.2), dot(0.3), bpic("vinc", 13, offset=[2, 3.5]), seg(-4, -1, 4, -1, 0.2)],
         p, "2005'in altında ray çizgisi; 0,2 mm.", ["vinç", "raylı"])
    p = 124
    sign("2007", "Abide", "N", [seg(-2, -2.5, 2, -2.5, 0.2)]
         + polyline([(-1.2, -2.5), (-1.2, -0.5), (1.2, -0.5), (1.2, -2.5)], 0.2)
         + polyline([(-0.6, -0.5), (0, 2.5), (0.6, -0.5)], 0.2), p,
         "Zemin çizgisi üstünde 2 mm yüksek kaide ve 3 mm sivri gövde; 0,2 mm (1/5000'de 1 ve 3 mm). Genişlikler çizimden.",
         ["abide", "anıt"])
    sign("2008", "Heykel", "N", [ring_(2, 0.2, y=2), seg(0, 1, -1.2, -3, 0.2), seg(0, 1, 1.2, -3, 0.2), dot(0.3, 0, -2.6)], p,
         "2 mm çaplı baş, 4 mm açılan iki bacak, aralarında nokta; 0,2 mm (1/5000'de 1 ve 2 mm).", ["heykel", "anıt"])
    sign("2009", "Tramvay troleybüs direği", "N", [ring_(1.5, 0.2), dot(0.25), seg(0, 0.75, 0, 5.75, 0.2),
                                                   shape("triangle", 1, fill=INK, rotation=-90,
                                                         offset=G.beside(0.29, 5.25, -90))], p,
         "1,5 mm çaplı daire, 5 mm sap, ucunda dolu bayrakçık; 0,2 mm (1/5000'de 1 ve 3 mm).", ["direk", "tramvay"])
    sign("2010", "Kara ve demiryolu trafik sinyali", "N", [seg(-1, 0, 1, 0, 0.2), seg(0, 0, 0, 5, 0.2),
                                                           seg(0, 4.2, 0.6, 4.9, 0.2), seg(0, 4.2, 0.6, 3.5, 0.2)], p,
         "2 mm taban, 5 mm direk, tepesinde kollar (K); 0,2 mm (1/5000'de 3 ve 0,5 mm).", ["sinyal", "trafik"])
    sign("2011", "Mareograf istasyonu", "N", [ring_(5, 0.2), shape("x", 5, stroke_=INK, width=0.2)], p,
         "5 mm çaplı, içi çarpılı daire; 0,2 mm (1/5000'de 3 mm). Yaklaşık deniz yüzeyini belirleyen yapı.",
         ["mareograf", "deniz"])
    sign("2012", "Reklam panosu", "N", [ring_(4, 0.2), seg(-1, -1.732, -1, 1.732, 0.2), seg(1, -1.732, 1, 1.732, 0.2),
                                        dot(0.3)], p,
         "4 mm çaplı daire, içinde iki dik çizgi ve nokta; 0,2 mm (1/5000'de 3 mm).", ["reklam", "pano"])
    p = 125
    sign("2013", "Taş sütun", "N", [seg(-0.5, -3.5, -0.5, 3.5, 0.2), seg(0.5, -3.5, 0.5, 3.5, 0.2),
                                    seg(-1, 3.5, 1, 3.5, 0.2), seg(-1, -3.5, 1, -3.5, 0.2)], p,
         "Aralarında 1 mm olan 7 mm iki çizgi, alt ve üstte 2 mm başlık; 0,2 mm (1/5000'de 0,5, 3 ve 1 mm).",
         ["sütun", "tarihî"])
    sign("2014", "Meteoroloji istasyonu", "N", [seg(-2.5, 1, 2.5, 1, 0.2), seg(0, 1, 0, -1, 0.2)], p,
         "5 mm geniş, 2 mm yüksek T; 0,2 mm (1/5000'de 3 ve 1 mm).", ["meteoroloji", "istasyon"])
    sign("2015", "Trafik kontrol istasyonu", "N", [seg(-1, 1.5, 1, 1.5, 0.2), seg(0, 1.5, 0, -1.5, 0.2)], p,
         "2 mm geniş, 3 mm yüksek T; 0,2 mm (1/5000'de 1,5 ve 2 mm).", ["trafik", "istasyon"])
    sign("2016", "Ağırlık kontrol istasyonu", "A", [outline(0.2)], p,
         "0,2 mm. Bina kendi özelliğinde resmî ya da özel bina olarak çizilir.", ["kantar", "istasyon"])
    sign("2017", "Çan kulesi", "N", [ring_(1.5, 0.2), dot(0.25), seg(0, 0.75, 0, 3.75, 0.2), seg(-0.75, 3.1, 0.75, 3.1, 0.2)],
         p, "1,5 mm çaplı daire, 3 mm sap, ucunda 1,5 mm kol (haç); 0,2 mm (1/5000'de 0,5 ve 2 mm).", ["çan kulesi", "kilise"])
    sign("2018", "Mağara", "N", [bpic("magara", 3.6), dot(0.3, 0.3, -0.9)], p,
         "3 mm genişliğinde kemer, içinde nokta; 0,2 mm (1/5000'de 2 mm).", ["mağara"])
    p = 126
    sign("2019", "Pompa istasyonu", "N", [bpic("pompa", 3)], p,
         "2 mm çaplı, 3 mm yüksek silindir, tabanında nokta; 0,2 mm (1/5000'de 1 ve 2 mm).", ["pompa", "istasyon"])
    sign("2020", "Petrol kuyusu", "N", [ring_(3, 0.2), dot(0.3), seg(0, 1.5, 0, 6.2, 0.2),
                                        shape("arrowhead", 0.8, fill=INK, rotation=90, offset=G.beside(0, 6.6, 90))], p,
         "3 mm çaplı daire, ortasında nokta, üstünde ucu 7 mm yükseklikte ok; 0,2 mm (1/5000'de 1,5 ve 5 mm). Ok başının "
         "boyu çizimden.", ["petrol", "kuyu"])
    tower = [shape("square", 5, fill=PAPER, stroke_=INK, width=0.2), shape("square", 4, fill=None, stroke_=INK, width=0.2),
             seg(-2, -2, 2, 2, 0.2), seg(-2, 2, 2, -2, 0.2)]
    sign("2021", "Petrol kulesi", "N", tower + [seg(0, 2.5, 0, 4.5, 0.2), ring_(2, 0.2, y=5.5), seg(0, 6.5, 0, 9.1, 0.2),
                                                shape("arrowhead", 0.8, fill=INK, rotation=90,
                                                      offset=G.beside(0, 9.1, 90))], p,
         "5 × 5 mm kare içinde çarpılı 4 × 4 mm kare; üstünde 2 mm çaplı daire ve 3 mm ok (karenin üstünden dairenin "
         "üstüne 4 mm); 0,2 mm (1/5000'de 3 × 3, 2 × 2, 1 ve 2 mm).", ["petrol", "kule"])
    sign("2022", "Hudut taşı (devlet)", "N", [ring_(3, 0.4), seg(-1.5, 0, 1.5, 0, 0.4)], p,
         "3 mm çaplı, ortasından çizgi geçen daire; 0,4 mm (1/5000'de 2 mm, 0,2 mm).", ["hudut taşı", "sınır"])
    sign("2023", "Gözetleme kulesi", "N", tower + polyline([(2.5, -1.2), (3.2, -1.2), (3.2, -2.2), (2.5, -2.2)], 0.2)
         + [seg(2.5, -1.7, 3.2, -1.7, 0.2)], p,
         "5 × 5 mm kare içinde çarpılı 4 × 4 mm kare, yanında merdiven; 0,2 mm (1/5000'de 3 × 3 ve 2 × 2 mm). Merdivenin "
         "biçimi çizimden.", ["kule", "gözetleme"])
    sign("2024", "Fabrika bacası", "N", [ring_(2.5, 0.2), dot(0.3)], p,
         "Daire, ortasında nokta; 0,2 mm. Baca ölçüsüyle çizilir; çapı yazılı değil, KentOS 2,5 mm çizer.", ["baca", "fabrika"])
    p = 127
    sign("2025", "Bekçi kulübesi", "N", [shape("square", 4, fill=PAPER, stroke_=INK, width=0.2),
                                         shape("square", 3, fill=None, stroke_=INK, width=0.2),
                                         seg(-1.5, -1.5, 1.5, 1.5, 0.2), seg(-1.5, 1.5, 1.5, -1.5, 0.2)], p,
         "4 × 4 mm kare içinde çarpılı 3 × 3 mm kare; 0,2 mm (1/5000'de 3 × 3 ve 2 × 2 mm).", ["kulübe", "bekçi"])
    phone = polyline([(1, 1.2), (1, 1.5), (0, 1.5), (0, -1.5), (1, -1.5), (1, -1.2)], 0.2) \
        + polyline([(0, 0.3), (-0.5, 0.3), (-0.5, -2)], 0.2)
    sign("2026", "Telefon kulübesi", "N", phone, p,
         "3 mm ahize ve kordonu; 0,2 mm (1/5000'de 0,16 mm). Ahizenin kolları çizimden 1 mm.", ["telefon", "kulübe"])
    sign("2027", "Rögar kapağı", "N", [ring_(2, 0.2), shape("cross", 2, stroke_=INK, width=0.2)], p,
         "2 mm çaplı, içi artılı daire; 0,2 mm (1/5000'de 1,5 mm).", ["rögar", "kanalizasyon"])
    sign("2028", "Telefon santrali", "A", [hatch(45, 1, 0.1, INK), outline(0.2), centroid([dict(l) for l in phone])], p,
         "0,2 mm, içi taralı; içine telefon işareti (2026) konur. Taramanın aralığı ve kalınlığı yazılı değil, 1 mm ve "
         "0,1 mm.", ["telefon", "santral"])
    sign("2029", "Minare", "N", [ring_(3, 0.2), dot(0.3), bpic("minare", 3, offset=[0, 4.25])], p,
         "3 mm çaplı daire, ortasında nokta, üstünde 3 mm sap ve 2 mm çaplı hilal; 0,2 mm (1/5000'de 1, 2 ve 1 mm).",
         ["minare", "cami"])
    sign("2030", "Bayrak direği", "N", [ring_(1.5, 0.2), dot(0.25), seg(0.75, 0, 0.75, 4, 0.2)], p,
         "1,5 mm çaplı daire, yanından 4 mm direk; 0,2 mm (1/5000'de 1 ve 3 mm).", ["bayrak", "direk"])
    p = 128
    sign("2031", "Peri bacası", "N", [bpic("peri", 7.6)], p,
         "7 mm yüksek, 3 mm geniş başlıklı koni, tabanında nokta; 0,2 mm (1/5000'de 1,5 ve 3 mm).", ["peri bacası"])
    sign("2032", "Maden (işleyen)", "N", [bpic("maden", 7.6)], p,
         "7 mm çapraz çekiçler, başları (1,5 mm) yukarıda; 0,2 mm (1/5000'de 1 ve 5 mm).", ["maden", "işleyen"])
    sign("2033", "Maden (işlemeyen)", "N", [bpic("maden", 7.6, rotation=180)], p,
         "2032'nin ters çevrilmişi: çekiçlerin başları aşağıda; 0,2 mm.", ["maden", "işlemeyen"])
    sign("2034", "Maden girişi", "N", [bpic("maden-girisi", 4.4, offset=[0, -0.7])], p,
         "3 mm çaplı kemer ve 3 mm ayaklar, içinde kısa taramalar ve nokta; 0,2 mm (1/5000'de 2 mm). Taramaların aralığı "
         "çizimden.", ["maden", "giriş"])
    sign("2035", "Km taşı veya levhası", "N", [shape("square", 3, fill=PAPER, stroke_=INK, width=0.2, rotation=45), dot(0.3)],
         p, "Kenarı 3 mm eşkenar dörtgen, ortasında nokta; 0,2 mm (1/5000'de 2 mm).", ["kilometre", "levha"])
    sign("2036", "Yangın vanası", "N", [seg(0, -2.7, 0, 1.3, 0.2), seg(0, 1.3, -1.41, 2.71, 0.2),
                                        seg(0, 1.3, 1.41, 2.71, 0.2)], p,
         "4 mm sap ve 2 mm iki kol (Y); 0,2 mm (1/5000'de 2 ve 1 mm).", ["yangın", "vana", "hidrant"])
    p = 129
    sign("2037", "Benzin istasyonu pompası", "N", [shape("rectangle", 3, fill=PAPER, stroke_=INK, width=0.2, height=5),
                                                   dot(0.3), seg(1.5, 1, 2.7, -1.4, 0.2)], p,
         "3 × 5 mm dikdörtgen, ortasında nokta, yanından eğik hortum; 0,2 mm (1/5000'de 1 × 3 mm).",
         ["akaryakıt", "benzin", "pompa"])


def control() -> list[list[dict]]:
    """The control point signs (2400); their marks are reused by the number and height signs (2600)."""
    p = 130
    tri = [shape("triangle", 3.464, fill=PAPER, stroke_=INK, width=0.2), dot(0.3)]
    poly = [ring_(2.5, 0.2), dot(0.3)]
    aux = [ring_(2, 0.2), dot(0.3)]
    level = [ring_(3, 0.2), ring_(1.6, 0.2, fill=None), dot(0.3)]
    wall = [shape("semicircle", 3, fill=PAPER, stroke_=INK, width=0.2), dot(0.3, 0, 0.4)]
    sign("2401", "Nirengi noktası", "N", tri, p, "3 mm yüksek eşkenar üçgen, ortasında nokta; 0,2 mm (1/5000'de 0,16 mm).",
         ["nirengi", "kontrol noktası"])
    sign("2402", "Poligon noktası", "N", poly, p, "2,5 mm çaplı daire, ortasında nokta; 0,2 mm (1/5000'de 2 mm).",
         ["poligon", "kontrol noktası"])
    sign("2403", "Yardımcı poligon noktası", "N", aux, p, "2 mm çaplı daire, ortasında nokta; 0,2 mm (1/5000'de 1,5 mm).",
         ["poligon", "kontrol noktası"])
    sign("2404", "Nivelman noktası", "N", level, p,
         "3 mm çaplı iç içe iki daire, ortasında nokta; 0,2 mm (1/5000'de 2 mm). İç dairenin çapı çizimden 1,6 mm.",
         ["nivelman", "röper", "kontrol noktası"])
    sign("2405", "Duvar madeni röperi", "N", wall, p,
         "3 mm çaplı yarım daire, içinde nokta; 0,2 mm (1/5000'de 2 mm). İşaretin tabanı duvarın yüzüne oturur.",
         ["röper", "duvar", "kontrol noktası"])
    return [tri, poly, aux, level, wall]


def names() -> None:
    tri, poly, aux, level, wall = control()
    p = 131
    number = lambda y, **kw: text({"expr": "varsayılan([Numara], '')"}, 2, INK, weight=400, offset=[0, y], **kw)  # noqa: E731
    height = lambda y, **kw: text({"expr": "varsayılan([Kot], '')"}, 1.5, INK, weight=400, offset=[0, y], **kw)  # noqa: E731
    both = "Numarası (Numara alanı) 2 mm, kotu (Kot alanı) 1,5 mm yazılır (1/5000'de 2 ve 1,2 mm)."
    sign("2601", "Nirengi noktası numarası ve kotu", "Y", tri + [number(3.6), height(-2.3)], p,
         f"Nirengi işaretinin üstüne numarası, altına kotu. {both}", ["nirengi", "numara", "kot", "yazı"])
    sign("2602", "Poligon noktası numarası ve kotu", "Y", poly + [number(2.9), height(-2.5)], p,
         f"Poligon işaretinin üstüne numarası, altına kotu. {both}", ["poligon", "numara", "kot", "yazı"])
    sign("2603", "Yardımcı poligon noktası numarası ve kotu", "Y", aux + [number(2.6), height(-2.3)], p,
         f"Yardımcı poligon işaretinin üstüne numarası, altına kotu. {both}", ["poligon", "numara", "kot", "yazı"])
    sign("2604", "Nivelman noktası numarası ve kotu", "Y", level + [number(3.1), height(-2.8)], p,
         f"Nivelman işaretinin üstüne numarası (örnekte “Rs. 73”), altına kotu. {both}",
         ["nivelman", "numara", "kot", "yazı"])
    sign("2605", "Duvar madeni röperi numarası ve kotu", "Y",
         wall + [number(2.8), text({"expr": "varsayılan([Kot], '')"}, 1.5, INK, weight=400, anchor="left",
                                    offset=[1.9, 0.6])], p,
         f"Röper işaretinin üstüne numarası, sağına kotu. {both}", ["röper", "numara", "kot", "yazı"])

    def label(number_: str, name: str, field: str, size: float, pen: float, small: str, page: int, tags: list[str],
              italic: bool = False, extra: str = "") -> None:
        # The sans face that ships has no italic; the UI face has one.
        face = {"italic": True, "font": "ui"} if italic else {}
        layer = text({"expr": f"varsayılan([{field}], '{field}')"}, size, INK, weight=400, **face)
        sign(number_, name, "Y", [layer], page,
             f"{mm(size)} mm yüksek yazı, {mm(pen)} mm kalemle ({small}).{extra} Yazı {field} alanından okunur; alan "
             f"yoksa alanın adı yazılır.", tags + ["yazı"])
    label("2606", "Bucak ve köy isimleri", "Ad", 4, 0.4, "1/5000'de 2 mm", 131, ["bucak", "köy", "isim"])
    p = 132
    label("2607", "Cadde ve yol isimleri", "Ad", 2, 0.3, "1/5000'de 1,5 mm", p, ["cadde", "yol", "isim"])
    label("2608", "Yapı isimleri", "Ad", 2, 0.3, "1/5000'de 1,5 mm", p, ["yapı", "bina", "isim"])
    label("2609", "Yöre isimleri", "Ad", 3, 0.3, "1/5000'de 2 mm", p, ["yöre", "mevki", "isim"])
    label("2610", "Unsur isimleri", "Ad", 1.5, 0.2, "1/5000'de 1,2 mm", p, ["unsur", "isim"], extra=" Yalnız gösterimdir.")
    label("2611", "Kat adedi", "Kat", 1.5, 0.2, "1/5000'de 1,5 mm", p, ["kat", "bina"])
    label("2612", "Ada numarası", "Ada", 4, 0.5, "1/5000'de 3 mm", p, ["ada", "kadastro"])
    p = 133
    label("2613", "Parsel numarası", "Parsel", 2, 0.3, "1/5000'de 1,7 mm", p, ["parsel", "kadastro"])
    label("2614", "Deniz isimleri", "Ad", 4, 0.4, "1/5000'de 3 mm", p, ["deniz", "isim"], True, " Sağa yatık yazılır.")
    label("2615", "Göl-nehir isimleri", "Ad", 2, 0.2, "1/5000'de 1,5 mm", p, ["göl", "nehir", "isim"], True,
          " Sağa yatık yazılır.")
    label("2616", "Çay-dere isimleri", "Ad", 2, 0.2, "1/5000'de 1,5 mm", p, ["çay", "dere", "isim"], True,
          " Sağa yatık yazılır.")


def library() -> dict:
    ITEMS.clear()
    categories = [{"path": [BRANCH], "order": 5,
                   "description": "Büyük Ölçekli Harita ve Harita Bilgileri Üretim Yönetmeliği Ek-3 harita özel işaretleri; "
                                  "1/500–1/2000 çizimleri, ölçüler kâğıt milimetresi"}]
    for k, (code, name) in enumerate(CLASSES):
        categories.append({"path": [BRANCH, f"{code} {name}"], "order": k + 1})
    pictograms()
    contours()
    roads()
    rail()
    water()
    lines()
    bounds()
    buildings()
    vegetation()
    rocks()
    slopes()
    complements()
    names()
    # Only the classes read so far are listed.
    categories = categories[:1] + [c for c in categories[1:] if any(i["path"] == c["path"] for i in ITEMS)]
    categories.append({"path": [BRANCH, "Piktogramlar"], "order": 99, "description": "BÖHHBÜY işaretlerinin piktogramları (SVG)"})
    assets = [{"kind": "asset", "id": f"bohhbuy.svg.{name}", "name": name, "path": [BRANCH, "Piktogramlar"], "format": "svg",
               "data": data, "width": 100, "height": 100} for name, data in PICTOGRAMS.items()]
    if not assets:
        categories.pop()
    ids = [i["id"] for i in ITEMS]
    if len(ids) != len(set(ids)):
        raise SystemExit("duplicate sign numbers")
    return {"format": "kentos-style", "version": 1, "exported": "system", "items": ITEMS + assets, "categories": categories}


def main() -> int:
    if "--svg" in sys.argv:
        out = Path(sys.argv[sys.argv.index("--svg") + 1])
        out.write_text(json.dumps(PICTOGRAMS, ensure_ascii=False, indent=1), encoding="utf-8")
        print(f"{out}: {len(PICTOGRAMS)} pictograms")
        return 0
    lib = library()
    text_ = json.dumps(lib, ensure_ascii=False, separators=(",", ":")) + "\n"
    summary = f"{len(ITEMS)} signs, {len(lib['items']) - len(ITEMS)} pictograms, {len(lib['categories'])} categories"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text_:
            print(f"{OUT.relative_to(ROOT)} is stale: run python3 scripts/style/bohhbuy_library.py", file=sys.stderr)
            return 1
        print(f"up to date: {summary}")
        return 0
    OUT.write_text(text_, encoding="utf-8")
    print(f"wrote {OUT.relative_to(ROOT)}: {summary}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
