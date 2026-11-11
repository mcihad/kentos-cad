#!/usr/bin/env python3
"""The shared cases of `road.terrains.sectioncheck` (Kesit kontrolü; plan/map-cad.md MAP-14, docs/adr/5120), without
KentOS code.

    python3 scripts/fixtures/section_check_cases.py           # writes the file
    python3 scripts/fixtures/section_check_cases.py --check   # writes nothing; compares

BÖHHBÜY m. 93(1): a section between two control points of known height (a); points on it at most 20 m apart, their
heights carried by levelling (b); the RMS of the measured heights less those computed from the contour lines under
1.5 · KOH_Z for the scale (c; KOH_Z = 1.33 · KOH_X,Y, KOH_X,Y = −1.665e-6·S² + 0.01745·S − 1.166 cm, m. 91(2)). The
rules, by hand:

- a point is read at its foot on the section's line: how far along it from `from` (the dot product with the unit
  direction) and how far off it (the size of the cross product); its foot before `from` or after `to` is `outside`,
  farther than `corridor` (when given) `offLine`;
- the contour lines are the polylines on the contour layers whose every vertex (of every part) has one same height; any
  other polyline is counted `unlevelled`; each edge (a closed one's last edge too) is cut with the section by solving the
  two lines' equations (Cramer's rule), the crossing kept when it lies on both segments (ends included) and the two are
  not parallel; crossings within 1e-9 m along and of one height are one; they are ordered along the section;
- a point's height from the contours is the straight interpolation between the last crossing at or before it and the
  first at or after it (one crossing at the point: its height); without one on a side, `noContour`;
- the section's places are its two ends and its points that are on it (compared or noContour); two neighbours farther
  apart than the package's spacing (20 m) are a gap;
- the RMS and the largest size of the measured less the read heights over the compared points, against
  `factor` · KOH_Z; warnings `section_check_exceeded` (RMS not under the limit) and `section_spacing_exceeded`.

Numbers compare within 1e-9 (`within`).
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "road.terrains.sectioncheck.json"
RULES = json.loads((ROOT / "catalog/v1/rules/esyukselti-bohhbuy-2018.json").read_text("utf-8"))
SAME = {"revision": "same", "canUndo": False}
WITHIN = 1e-9


def uid(n):
    return f"0192f5c0-5120-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": float(x), "y": float(y)}


def point(x, y, z=None, layer_="kesit"):
    e = {"kind": "point", "layerId": layer_, "attrs": {}, "p": P(x, y)}
    if z is not None:
        e["z"] = z
    return e


def contour(pts, z, closed=False, parts=None, zs=None):
    e = {"kind": "polyline", "layerId": "esyukselti", "attrs": {}, "pts": [P(*p) for p in pts],
         "zs": zs if zs is not None else [z] * len(pts)}
    if closed:
        e["closed"] = True
    if parts:
        e["parts"] = [{"pts": [P(*p) for p in q], "zs": [z] * len(q)} for q in parts]
    return e


# The ground rises eastward, its contours north–south at uneven spacing (levels 100 … 105): 100 at x = 0, 101 at 15,
# 102 at 35 (a zigzag through (35, 0)), 103 at 50, 104 at 70, 105 at 95. A closed 106 ring (a knoll) round (120, 0),
# radius 8 as an octagon; a two-part 99 line west; a polyline of mixed heights.
OBJECTS = [
    ("C100", contour([(0, -60), (0, 60)], 100.0)),
    ("C101", contour([(15, -60), (15, 60)], 101.0)),
    ("C102", contour([(33, -60), (35, 0), (33, 60)], 102.0)),
    ("C103", contour([(50, -60), (50, 60)], 103.0)),
    ("C104", contour([(70, -60), (72, 0), (70, 60)], 104.0)),
    ("C105", contour([(95, -60), (95, 60)], 105.0)),
    ("C106", contour([(120 + 8 * math.cos(k * math.pi / 4), 8 * math.sin(k * math.pi / 4)) for k in range(8)], 106.0,
                     closed=True)),
    ("C99", contour([(-20, -60), (-20, -5)], 99.0, parts=[[(-20, 5), (-20, 60)]])),
    ("MIXED", contour([(60, -60), (60, 60)], 0.0, zs=[103.5, 103.6])),
    # The control points and the section's points (levelled heights).
    ("A", point(-5, 0, 99.62, "kontrol")),
    ("B", point(100, 0, 105.18, "kontrol")),
    ("D1", point(-10, -40, 99.4, "kontrol")),
    ("D2", point(90, 40, 104.9, "kontrol")),
    ("K1", point(10, 0, 100.69)),
    ("K2", point(25, 0, 101.47)),
    ("K3", point(42, 0, 102.49)),
    ("K4", point(60, 0, 103.47)),
    ("K5", point(80, 0, 104.31)),
    ("K6", point(98, 0, 105.10)),
    ("K7", point(35, 0, 102.03)),       # on the zigzag's vertex: a crossing at the point
    ("K8", point(-2, 0, 99.80)),        # before the first contour on this section: no crossing west of it
    ("K9", point(110, 0, 105.5)),       # after `to`
    ("K10", point(42, 3, 102.5)),       # 3 m off the line
    ("K11", point(25, 0, 102.40)),      # 0.93 m above: past the limit
    ("K12", point(42, 0)),              # no height
    ("L", {"kind": "line", "layerId": "kesit", "attrs": {}, "a": P(0, 0), "b": P(1, 1)}),
    ("A2", point(-5, 0, 98.0, "kontrol")),  # A's place again
    ("R1", point(112, 0, 105.6)),       # inside the knoll's ring on a section through it
    ("R2", point(120, 0, 106.4)),
    ("E", point(130, 0, 105.7, "kontrol")),
    ("W", point(-30, 0, 98.6, "kontrol")),
    ("W1", point(-20, 0, 98.98)),
]
SLOT = {name: i + 1 for i, (name, _) in enumerate(OBJECTS)}
ENTS = [dict(e, id=i + 1) for i, (_, e) in enumerate(OBJECTS)]
BY = dict(OBJECTS)


def U(name):
    return uid(SLOT[name])


def setup(scale=1000.0):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                "plotScale": scale, "workspace": "cad", "cadWork": "map"}
    return {"format": "kentos.document", "version": 2, "name": "Kesit kontrolü", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0},
            "layers": [layer("0", "0"), layer("esyukselti", "Eş yükselti"), layer("kontrol", "Kontrol"),
                       layer("kesit", "Kesit noktaları")],
            "activeLayer": "kesit", "entities": copy.deepcopy(ENTS), "uids": [uid(i + 1) for i in range(len(ENTS))],
            "styles": {"items": [], "categories": []}}


def at(name):
    e = BY[name]
    return (e["p"]["x"], e["p"]["y"]), e.get("z")


def meet(a, b, p, q):
    """Where the segments a–b and p–q meet, as the fraction along a–b; none when parallel or apart (Cramer)."""
    a11, a12 = b[0] - a[0], -(q[0] - p[0])
    a21, a22 = b[1] - a[1], -(q[1] - p[1])
    r1, r2 = p[0] - a[0], p[1] - a[1]
    det = a11 * a22 - a12 * a21
    if det == 0:
        return None
    t = (r1 * a22 - a12 * r2) / det
    u = (a11 * r2 - r1 * a21) / det
    return t if 0.0 <= t <= 1.0 and 0.0 <= u <= 1.0 else None


def koh(scale, check):
    s = float(scale)
    koh_z = check["zFactor"] * (check["xy"][0] * s * s + check["xy"][1] * s + check["xy"][2]) / 100.0
    return koh_z, check["factor"] * koh_z


def fmt(v):
    """A number as Rust's `{}` writes an f64 with no fraction (`20`), else its shortest form."""
    return str(int(v)) if v == int(v) else repr(v)


def checked(frm, to, points, layers=("esyukselti",), scale=1000, corridor=None, rules=None):
    rules = rules or RULES
    check = rules["sectionCheck"]
    (a, za), (b, zb) = at(frm), at(to)
    length = math.hypot(b[0] - a[0], b[1] - a[1])
    crossings, unlevelled = [], 0
    for name, e in OBJECTS:
        if e["kind"] != "polyline" or e["layerId"] not in layers:
            continue
        rings = [(e["pts"], e["zs"])] + [(q["pts"], q.get("zs")) for q in e.get("parts", [])]
        heights = {z for _, zs in rings for z in (zs or [None])}
        if len(heights) != 1 or None in heights or any(zs is None or len(zs) != len(pts) for pts, zs in rings):
            unlevelled += 1
            continue
        z = heights.pop()
        for pts, _ in rings:
            n = len(pts)
            edges = n if e.get("closed") and n > 2 else n - 1
            for k in range(edges):
                p, q = pts[k], pts[(k + 1) % n]
                t = meet(a, b, (p["x"], p["y"]), (q["x"], q["y"]))
                if t is None:
                    continue
                s = t * length
                if not any(abs(c[0] - s) <= 1e-9 and c[1] == z for c in crossings):
                    crossings.append([s, z])
    crossings.sort(key=lambda c: (c[0], c[1]))
    ux, uy = (b[0] - a[0]) / length, (b[1] - a[1]) / length
    rows = []
    for name in points:
        (x, y), z = at(name)
        along = (x - a[0]) * ux + (y - a[1]) * uy
        off = abs((x - a[0]) * uy - (y - a[1]) * ux)
        row = {"uid": U(name), "at": [x, y], "along": along, "offset": off, "measured": z}
        if along < 0 or along > length:
            row["state"] = "outside"
        elif corridor is not None and off > corridor:
            row["state"] = "offLine"
        else:
            before = [c for c in crossings if c[0] <= along]
            after = [c for c in crossings if c[0] >= along]
            if before and after:
                p, q = before[-1], after[0]
                h = p[1] if q[0] == p[0] else p[1] + (q[1] - p[1]) * (along - p[0]) / (q[0] - p[0])
                row.update({"contour": h, "between": [p[1], q[1]], "difference": z - h, "state": "compared"})
            else:
                row["state"] = "noContour"
        rows.append(row)
    rows.sort(key=lambda r: r["along"])
    places = sorted([0.0, length] + [r["along"] for r in rows if r["state"] in ("compared", "noContour")])
    spacing = check.get("spacing")
    gaps = ([{"from": u, "to": v, "length": v - u} for u, v in zip(places, places[1:]) if v - u > spacing]
            if spacing is not None else [])
    diffs = [r["difference"] for r in rows if "difference" in r]
    koh_z, limit = koh(scale, check)
    out = {"length": length, "ends": [za, zb], "points": rows, "crossings": crossings, "unlevelled": unlevelled,
           "gaps": gaps, "scale": scale, "rules": f"{rules['id']} {rules['release']}", "compared": len(diffs)}
    warns = []
    if diffs:
        rms = math.sqrt(sum(d * d for d in diffs) / len(diffs))
        out["rms"] = rms
        out["largest"] = max(abs(d) for d in diffs)
    out["kohZ"] = koh_z
    out["limit"] = limit
    if spacing is not None:
        out["spacing"] = spacing
    if diffs:
        out["within"] = out["rms"] < limit
        if not out["within"]:
            warns.append({"code": "section_check_exceeded", "message":
                          f"Kesit noktalarının ölçülen kotlarıyla eğrilerden okunan kotlarının farklarının karesel "
                          f"ortalaması {out['rms']:.3f} m, m. 93(1)(c)'nin sınırını ({limit:.3f} m) aşıyor; eğrileri ve "
                          "noktaların kotlarını denetleyin.", "path": "points"})
    if spacing is not None and gaps:
        g = gaps[0]
        most = fmt(spacing)
        warns.append({"code": "section_spacing_exceeded", "message":
                      f"Kesitte {len(gaps)} yerde iki komşu nokta {most} m'den uzak (ilki {g['from']:.2f} m ile "
                      f"{g['to']:.2f} m arasında, {g['length']:.2f} m); m. 93(1)(b) noktaları en çok {most} m aralıkla "
                      "ister.", "path": "points"})
    return out, warns


def case(name, frm, to, points, note=None, op="execute", scale_setting=1000.0, **kw):
    input_ = {"from": U(frm), "to": U(to), "points": [U(p) for p in points],
              "contours": list(kw.get("layers", ("esyukselti",)))}
    if "scale" in kw:
        input_["scale"] = kw["scale"]
    else:
        kw["scale"] = int(scale_setting)
    if kw.get("corridor") is not None:
        input_["corridor"] = kw["corridor"]
    if kw.get("rules") is not None:
        input_["rules"] = kw["rules"]
    out, warns = checked(frm, to, points, **kw)
    c = {"name": name}
    if note:
        c["note"] = note
    if scale_setting != 1000.0:
        c["setup"] = setup(scale_setting)
    c["steps"] = [{"op": op, "input": input_, "result": {"status": "completed",
                                                         "output": None if op == "validate" else out,
                                                         "warnings": warns},
                   "within": WITHIN, "expect": SAME}]
    return c


def refused(name, input_, code, message, path_):
    base = {"from": U("A"), "to": U("B"), "points": [U("K1")], "contours": ["esyukselti"]}
    base.update(input_)
    return {"name": name, "steps": [{"op": "execute", "input": base, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


ALL = ["K1", "K2", "K3", "K4", "K5", "K6"]


def cases():
    no_spacing = copy.deepcopy(RULES)
    del no_spacing["sectionCheck"]["spacing"]
    no_check = copy.deepcopy(RULES)
    del no_check["sectionCheck"]
    loose = copy.deepcopy(RULES)
    loose["sectionCheck"]["spacing"] = 25.0
    return [
        case("Kesit: altı nokta, 1/1000; farkların KOH'u sınırın altında, aralık 20 m'yi aşmıyor", "A", "B", ALL,
             note="Kesit (−5, 0)–(100, 0); eğriler 100 (x=0), 101 (15), 102 (35, zikzak), 103 (50), 104 (72), 105 (95)."),
        case("Noktalar karışık sırada verilir: kesit boyunca sıralanır", "A", "B", ["K4", "K1", "K6", "K2", "K5", "K3"]),
        case("Bir nokta 0,93 m yukarıda: KOH sınırı aşar, uyarı", "A", "B", ["K1", "K11", "K3", "K4", "K5", "K6"]),
        case("İki nokta eksik: 20 m'yi aşan iki aralık, uyarı", "A", "B", ["K1", "K3", "K5", "K6"]),
        case("Nokta bir eğrinin köşesinde: kotu o eğrinin", "A", "B", ["K1", "K2", "K7", "K3", "K4", "K5", "K6"]),
        case("İlk eğriden önceki nokta: eğriden kot yok (noContour), aralıkta sayılır", "A", "B", ["K8"] + ALL),
        case("Kesitin ucundan sonraki nokta: dışında (outside), aralıkta sayılmaz", "A", "B", ALL + ["K9"]),
        case("Kesitten 3 m uzak nokta, sınır yok: karşılaştırılır, uzaklığı söylenir", "A", "B",
             ["K1", "K2", "K10", "K4", "K5", "K6"]),
        case("Kesitten 3 m uzak nokta, sınır 2 m: kesit dışı (offLine)", "A", "B", ["K1", "K2", "K10", "K4", "K5", "K6"],
             corridor=2.0),
        case("Kesitten 3 m uzak nokta, sınır 5 m: karşılaştırılır", "A", "B", ["K1", "K2", "K10", "K4", "K5", "K6"],
             corridor=5.0),
        case("Çapraz kesit (−10, −40)–(90, 40): zikzaklı eğriler de kesilir", "D1", "D2", ALL),
        case("Ters yönde kesit: uzaklık B'den ölçülür", "B", "A", ALL),
        case("Tepe halkasının içinden kesit: kapalı halka iki kez kesilir", "B", "E", ["R1", "R2"]),
        case("İki parçalı 99 eğrisi: kesit parçaların arasından geçer, kesişim yok", "W", "B", ["W1", "K1", "K2"]),
        case("Kotları farklı çoklu çizgi okunmaz, sayılır", "A", "B", ALL),
        case("1/5000: sınır büyür", "A", "B", ["K1", "K11", "K3", "K4", "K5", "K6"], scale=5000),
        case("Ölçek projeden (1/2000)", "A", "B", ALL, scale_setting=2000.0),
        case("Aralığı vermeyen paket: aralık denetlenmez", "A", "B", ["K1", "K3", "K5", "K6"], rules=no_spacing),
        case("Paketin aralığı 25 m: tek aralık aşar", "A", "B", ["K1", "K3", "K5"], rules=loose),
        case("Eğri katmanı yalnız kontrol: kesişim yok", "A", "B", ALL, layers=("kontrol",)),
        case("Plan aynı cevabı verir", "A", "B", ALL, op="plan"),
        case("Doğrulama", "A", "B", ALL, op="validate"),
        refused("Kimlik UUID değil", {"from": "x"}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir.", "from"),
        refused("Çizimde olmayan bitiş", {"to": uid(999)}, "entity_not_found",
                f"“{uid(999)}” kimlikli nesne çizimde yok.", "to"),
        refused("Kotsuz kesit noktası", {"points": [U("K1"), U("K12")]}, "not_a_height_point",
                f"“{U('K12')}” kendi kotu olan bir nokta değil; kesitin uçları ve noktaları nivelmanla ölçülmüş kotu "
                "olan noktalardır.", "points[1]"),
        refused("Çizgi başlangıç olamaz", {"from": U("L")}, "not_a_height_point",
                f"“{U('L')}” kendi kotu olan bir nokta değil; kesitin uçları ve noktaları nivelmanla ölçülmüş kotu "
                "olan noktalardır.", "from"),
        refused("Aynı nokta iki kez", {"points": [U("K1"), U("K1")]}, "invalid_input",
                f"“{U('K1')}” noktası iki kez ya da kesitin ucu olarak verildi.", "points[1]"),
        refused("Uç kesit noktası olarak", {"points": [U("B")]}, "invalid_input",
                f"“{U('B')}” noktası iki kez ya da kesitin ucu olarak verildi.", "points[0]"),
        refused("İki uç aynı yerde", {"to": U("A2")}, "invalid_input",
                "Kesitin iki ucu aynı yerde; kesit yüksekliği bilinen iki ayrı kontrol noktası arasında alınır.", "to"),
        refused("Kesit noktası yok", {"points": []}, "invalid_input",
                "Kesitte ölçülmüş nokta yok; kesit doğrultusunda nivelmanla kotu taşınmış noktaları verin.", "points"),
        refused("Kesitten uzaklık sınırı 0", {"corridor": 0.0}, "invalid_input",
                "Kesitten uzaklık sınırı 0'dan büyük olmalı.", "corridor"),
        refused("Eğri katmanı verilmemiş", {"contours": []}, "layer_not_found",
                "Eş yükselti eğrilerinin katmanını verin.", "contours"),
        refused("Olmayan eğri katmanı", {"contours": ["esyukselti", "yok"]}, "layer_not_found",
                "“yok” katmanı çizimde yok.", "contours[1]"),
        refused("Ölçek 0", {"scale": 0}, "invalid_contours", "Ölçek paydası 0 olamaz.", "scale"),
        refused("Kesit sınırını vermeyen paket", {"rules": no_check}, "invalid_contours",
                "bohhbuy-2018-m78 paketi kesit kontrolünün sınırını (sectionCheck) vermiyor ya da değerleri sonlu ve 0'dan "
                "büyük değil.", "rules"),
    ]


def main():
    check = "--check" in sys.argv[1:]
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.sectioncheck",
           "commandVersion": 1, "title": "Kesit kontrolü",
           "note": "ADR 5120. Bağımsız başvuru scripts/fixtures/section_check_cases.py: kesişimler Cramer kuralıyla, "
                   "enterpolasyon ve sınırlar betikte; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if check:
        same = OUT.exists() and OUT.read_text("utf-8") == text
        print("road.terrains.sectioncheck durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
