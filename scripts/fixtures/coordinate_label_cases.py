"""The shared cases of Koordinat yaz (docs/adr/0185): the places and names a
selection's vertices give, a label's lines from its template, and where its
leader and lines stand.

    python3 scripts/fixtures/coordinate_label_cases.py           # writes the file
    python3 scripts/fixtures/coordinate_label_cases.py --check   # writes nothing; compares

Writes fixtures/coordinate-labels/v1/cases.json. The rules are written here
from the ADR on their own, not from an implementation's output; the geometry
core (crates/shared/geometry-core, `ops::coordinate_labels`, natively and
through WASM: `coordinatePlaces`, `coordinateLabels`) is held to them. Numbers
are written by the display rule (measure_cases.shown, docs/adr/0149); the
letters' advances are the drawing typefaces' measured widths
(paragraph_cases.advance).

- Places (§2, the coordinate schedule's, docs/adr/0184 §3): the objects in
  the order given; a point object's points (its own, then its other parts'),
  named by its label (spaces round it aside; the k-th part past the first
  "label (k+1)"); a line's, a polyline's and an area's vertices (their
  paths, each path's elevations), unnamed. A place within 1 µm (Euclidean) of
  one already taken is that one: it keeps its elevation and name, taking the
  later one's when it has none. Each place remembers the middle of the box
  of the vertices of the object it first came from. The unnamed are named
  1, 2, … in order, skipping the names the places already have.
- Lines (§3): the template's parts between `|`; in each, `{Y}`, `{X}` (the
  project's axis names: a CAD project's X east and Y north, a CBS project's Y
  east and X north), `{Z}` and `{ad}` (their letters in either case) stand
  for the place's values: a coordinate or the elevation in the project's unit
  (m, cm, mm) with the given decimals by the display rule, the name as it is.
  Any other `{…}` is kept as written. A part with a stand-in that has no
  value (no elevation, no name) is left out; so is a part that is only spaces
  once filled; spaces round a part are cut. A place with no part left gets no
  label and is counted.
- Layout (§4): h the height; a line's width the sum of its letters'
  advances over 1000 times h times the width factor; w the widest line plus
  h; the direction (sx, sy) given, or with `auto` from the place's box middle
  to the place (east and north when equal; a place with no middle north-east).
  With a leader: elbow e = p + (3h·sx, 3h·sy); the leader p, e, e + (w·sx, 0);
  the first line's baseline at e.y + 0.4h, the others' tops at
  e.y − 0.4h − (k − 1)·(5/3)h; every line from e.x + 0.5h·sx, to its left in
  the east (baseline: no alignment; top: topLeft), to its right in the west
  (baselineRight, topRight). Without: a = p + (0.5h·sx, 0.5h·sy); northward
  line k's baseline at a.y + (n − 1 − k)·(5/3)h, southward line k's top at
  a.y − k·(5/3)h; from a.x the same way.
"""
import json
import re
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import measure_cases as measure  # noqa: E402  (the display rule)
import paragraph_cases as paragraph  # noqa: E402  (the typefaces' measured advances)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/coordinate-labels/v1/cases.json"
TOUCH = F(1, 10**6)

# ── Places ──────────────────────────────────────────────────────────────


def name_of(o):
    label = (o.get("label") or "").strip(" ")
    return label or None


def vertices(o):
    """The object's places: (x, y, z, name), in order."""
    s = o["shape"]
    if s["kind"] == "point":
        label = name_of(o)
        pts = [(s["p"], s.get("z"))] + [(q["p"], q.get("z")) for q in s.get("parts") or []]
        return [(F(p["x"]), F(p["y"]), z, None if label is None else (label if k == 0 else f"{label} ({k + 1})")) for k, (p, z) in enumerate(pts)]
    out = []
    for path in o.get("paths") or []:
        zs = path.get("zs") or []
        for k, p in enumerate(path["pts"]):
            out.append((F(p["x"]), F(p["y"]), zs[k] if k < len(zs) else None, None))
    return out


def near(a, b):
    dx, dy = a[0] - b[0], a[1] - b[1]
    return dx * dx + dy * dy <= TOUCH * TOUCH


def places(objects):
    out = []
    for o in objects:
        vs = vertices(o)
        if not vs:
            continue
        xs = [v[0] for v in vs]
        ys = [v[1] for v in vs]
        centre = ((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
        for x, y, z, name in vs:
            for place in out:
                if near(place["p"], (x, y)):
                    if place["z"] is None:
                        place["z"] = z
                    if place["name"] is None:
                        place["name"] = name
                    break
            else:
                out.append({"p": (x, y), "z": z, "name": name, "centre": centre})
    taken = {p["name"] for p in out if p["name"] is not None}
    n = 1
    for p in out:
        if p["name"] is None:
            while str(n) in taken:
                n += 1
            p["name"] = str(n)
            n += 1
    return out


# ── Lines ───────────────────────────────────────────────────────────────


def shown(v, u, decimals):
    k = {"m": 1, "cm": 100, "mm": 1000}[u["unit"]]
    return measure.shown(measure.mpf(F(v).numerator) / measure.mpf(F(v).denominator) * k, decimals)


def lines(template, place, u, decimals):
    east, north = ("x", "y") if u["axes"] == "cad" else ("y", "x")
    x, y = place["p"]
    values = {east: shown(x, u, decimals), north: shown(y, u, decimals), "z": None if place.get("z") is None else shown(F(place["z"]), u, decimals), "ad": place.get("name")}
    out = []
    for part in template.split("|"):
        missing = False

        def fill(m):
            nonlocal missing
            key = m.group(1).lower() if m.group(1).isascii() else m.group(1)
            if key not in values:
                return m.group(0)
            if values[key] is None:
                missing = True
                return ""
            return values[key]

        line = re.sub(r"\{([^{}]*)\}", fill, part).strip(" ")
        if missing or not line:
            continue
        out.append(line)
    return out


# ── Layout ──────────────────────────────────────────────────────────────

DIRECTIONS = {"ne": (1, 1), "nw": (-1, 1), "sw": (-1, -1), "se": (1, -1)}


def direction(o, place):
    if o["direction"] != "auto":
        return DIRECTIONS[o["direction"]]
    c = place.get("centre")
    if c is None:
        return (1, 1)
    x, y = place["p"]
    return (1 if x >= c[0] else -1, 1 if y >= c[1] else -1)


def width(text, o, h):
    return F(sum(paragraph.advance(o["font"], c, o.get("bold", False)) for c in text), 1000) * h * F(o.get("widthFactor", 1))


def label(place, o, u):
    ls = lines(o["template"], place, u, o["decimals"])
    if not ls:
        return None
    h = F(o["height"])
    sx, sy = direction(o, place)
    pitch = F(5, 3) * h
    x, y = place["p"]
    east = sx > 0
    texts = []
    leader = None
    if o["leader"]:
        ex, ey = x + 3 * h * sx, y + 3 * h * sy
        w = max(width(t, o, h) for t in ls) + h
        leader = [(x, y), (ex, ey), (ex + w * sx, ey)]
        tx = ex + h / 2 * sx
        for k, t in enumerate(ls):
            if k == 0:
                texts.append((tx, ey + F(2, 5) * h, t, None if east else "baselineRight"))
            else:
                texts.append((tx, ey - F(2, 5) * h - (k - 1) * pitch, t, "topLeft" if east else "topRight"))
    else:
        ax, ay = x + h / 2 * sx, y + h / 2 * sy
        n = len(ls)
        for k, t in enumerate(ls):
            if sy > 0:
                texts.append((ax, ay + (n - 1 - k) * pitch, t, None if east else "baselineRight"))
            else:
                texts.append((ax, ay - k * pitch, t, "topLeft" if east else "topRight"))
    return {"leader": leader, "texts": texts}


def labels(ps, o, u):
    out = []
    skipped = 0
    for p in ps:
        lb = label(p, o, u)
        if lb is None:
            skipped += 1
        else:
            out.append(lb)
    return {"labels": out, "skipped": skipped}


# ── Writing ─────────────────────────────────────────────────────────────


def num(v):
    v = F(v)
    return int(v) if v.denominator == 1 else float(v)


def pt(p):
    return {"x": num(p[0]), "y": num(p[1])}


def place_json(p):
    out = {"p": pt(p["p"])}
    if p.get("z") is not None:
        out["z"] = p["z"]
    if p.get("name") is not None:
        out["name"] = p["name"]
    if p.get("centre") is not None:
        out["centre"] = pt(p["centre"])
    return out


def labels_json(r):
    return {
        "labels": [
            {
                **({} if lb["leader"] is None else {"leader": [pt(q) for q in lb["leader"]]}),
                "texts": [{"p": pt((x, y)), "text": t, **({"align": a} if a else {})} for x, y, t, a in lb["texts"]],
            }
            for lb in r["labels"]
        ],
        "skipped": r["skipped"],
    }


def P(x, y):
    return {"x": x, "y": y}


def path(pts, zs=None, closed=False):
    return {"pts": [P(*q) for q in pts], "closed": closed, "zs": zs if zs is not None else [None] * len(pts)}


def point(x, y, z=None, label=None, parts=None):
    s = {"kind": "point", "p": P(x, y)}
    if z is not None:
        s["z"] = z
    if parts:
        s["parts"] = [{"p": P(*q[:2]), **({"z": q[2]} if len(q) > 2 else {})} for q in parts]
    return {"shape": s, "paths": [], "label": label, "attrs": {}}


def polygon(pts, zs=None, holes=None, label=None):
    s = {"kind": "polygon", "pts": [P(*q) for q in pts]}
    if holes:
        s["holes"] = [{"pts": [P(*q) for q in hole]} for hole in holes]
    paths = [path(pts, zs, True)] + [path(hole, None, True) for hole in holes or []]
    return {"shape": s, "paths": paths, "label": label, "attrs": {}}


def line(a, b, za=None, zb=None):
    return {"shape": {"kind": "line", "a": P(*a), "b": P(*b)}, "paths": [path([a, b], [za, zb])], "label": None, "attrs": {}}


CBS = {"axes": "gis", "unit": "m"}
CAD = {"axes": "cad", "unit": "m"}


def opts(**kw):
    o = {"template": "{Y}|{X}", "decimals": 3, "height": 1, "leader": True, "direction": "auto", "font": "barlow"}
    o.update(kw)
    return o


PARCEL = [(486970, 4419980), (487000, 4419980), (487000, 4420000), (486970, 4420000)]
NEIGHBOUR = [(487000, 4419980), (487020, 4419980), (487020, 4420000), (487000, 4420000)]


def place_cases():
    cases = []

    def case(name, objects):
        cases.append({"name": name, "objects": objects, "want": [place_json(p) for p in places(objects)]})

    case("iki komşu parsel: ortak köşeler bir kez, adsızlar 1'den, kutunun ortası ilk nesnenin", [polygon(PARCEL), polygon(NEIGHBOUR)])
    case(
        "adlı noktalar köşelerin önünde: köşe noktanın adını ve kotunu alır, numaralar adları atlar",
        [point(486970, 4419980, 812.4, "101"), point(487000, 4419980, None, "2"), polygon(PARCEL), polygon(NEIGHBOUR)],
    )
    case(
        "köşe önce: adsız ve kotsuz yer sonraki noktanın adını ve kotunu alır",
        [polygon(PARCEL, zs=[None, 811.5, None, None]), point(486970, 4419980, 812.4, " 101 "), point(487000, 4419980, 900.0, "B")],
    )
    case("çok noktalı nesne: parçalar “ad (2)”, adsız çok noktalı nesne numara alır", [point(10, 20, None, "P", parts=[(30, 20), (30, 40, 5.25)]), point(0, 0, None, None, parts=[(5, 5)])])
    case("1 µm içinde aynı, 2 µm ötesi ayrı yer; delik köşeleri de", [polygon([(0, 0), (10, 0), (10, 10)], holes=[[(2, 2), (4, 2), (4, 4)]]), line((0, 0.000001), (0, 0.000002))])
    case("boşluklu etiket ad değildir; nesnesiz liste boştur", [point(1, 1, None, "   "), line((5, 5), (7, 9), 1.5, None)])
    case("hiç nesne yok", [])
    return cases


def label_cases():
    cases = []

    def case(name, ps, o, u):
        cases.append({"name": name, "places": [place_json(p) for p in ps], "options": o, "units": u, "want": labels_json(labels(ps, o, u))})

    ring = places([polygon(PARCEL, zs=[812.4, None, 813.125, None]), polygon(NEIGHBOUR)])
    case("CBS, kollu, Otomatik: köşeler parselin dışına; doğu üstte, kuzey altta; kotsuz yerde Z satırı yok", ring, opts(template="{Y}|{X}|Z={Z}"), CBS)
    case("CAD, kollu, eksen adlarıyla: X doğu, Y kuzey; iki basamak", ring[:3], opts(template="X = {X}|Y = {Y}", decimals=2, height=0.5), CAD)
    side = {"ne": "kuzeydoğu", "nw": "kuzeybatı", "sw": "güneybatı", "se": "güneydoğu"}
    for d in ("ne", "nw", "sw", "se"):
        rule = ("son satırın tabanı köşenin yarım yükseklik üstünde" if d[0] == "n" else "ilk satırın üstü köşenin yarım yükseklik altında") + ("; sola dayalı" if d[1] == "e" else "; sağa dayalı")
        case(f"kolsuz, {side[d]}: {rule}", [{"p": (F(100), F(200)), "z": 5.5, "name": "7", "centre": None}], opts(template="{ad}|{Y}|{X}|{Z}", leader=False, direction=d), CBS)
    for d in ("nw", "sw", "se"):
        rule = "yatay çizgi batıya, satırlar sağa dayalı" if d[1] == "w" else "yatay çizgi doğuya, satırlar sola dayalı"
        case(f"kollu, {side[d]}: {rule}", [{"p": (F(-34.076), F(270.333)), "centre": None}], opts(direction=d, height=0.8), CBS)
    case(
        "şablonun harfleri büyük ya da küçük; bilinmeyen yer tutucu olduğu gibi; adsız yerde {ad}'lı satır yok; boş satır yok",
        [{"p": (F(486512.34), F(4420187.5)), "z": None, "name": None, "centre": None}],
        opts(template=" {y} m | {AD} || {x} {foo} |{Ad}"),
        CBS,
    )
    case("satırı kalmayan yer yazılmaz ve sayılır", [{"p": (F(1), F(2)), "z": None, "name": None, "centre": None}, {"p": (F(3), F(4)), "z": 2.0, "name": "N", "centre": None}], opts(template="{ad}: {Z}"), CBS)
    case("yerel proje, santimetre: eksi değer; sıfır basamak; kot da birimde", [{"p": (F(-12.3456), F(0.5)), "z": -0.25, "name": "A", "centre": None}], opts(template="{X}|{Y}|{Z}", decimals=0, leader=False, direction="ne"), {"axes": "cad", "unit": "cm"})
    case("milimetre, kalın yazı, genişlik çarpanı", [{"p": (F(1.2345), F(-2.5)), "z": None, "name": "K1", "centre": None}], opts(template="{ad}|{X}|{Y}", decimals=1, font="roboto" if "roboto" in paragraph.FONTS else "arimo", bold=True, widthFactor=0.8), {"axes": "cad", "unit": "mm"})
    case("Otomatik eşitlikte doğu ve kuzey: kutunun ortasındaki yer", [{"p": (F(5), F(5)), "z": None, "name": "M", "centre": (F(5), F(5))}], opts(), CBS)
    return cases


def build():
    return {
        "format": "kentos.coordinate-label-cases",
        "version": 1,
        "source": "scripts/fixtures/coordinate_label_cases.py; docs/adr/0185",
        "places": place_cases(),
        "labels": label_cases(),
    }


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def main():
    text = text_of(build())
    if "--check" in sys.argv:
        old = OUT.read_text("utf-8") if OUT.exists() else ""
        if old != text:
            print(f"{OUT} güncel değil: python3 {Path(__file__).relative_to(ROOT)} ile yeniden yazın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel.")
        return 0
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)}: {len(build()['places'])} yer, {len(build()['labels'])} yazı durumu")
    return 0


if __name__ == "__main__":
    sys.exit(main())
