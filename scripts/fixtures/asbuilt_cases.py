#!/usr/bin/env python3
"""The shared cases of `cad.asbuilt.check` (Uygulama kontrolü; plan/map-cad.md MAP-19, docs/adr/1700), without KentOS
code.

    python3 scripts/fixtures/asbuilt_cases.py           # writes fixtures/commands/v1/cad.asbuilt.check.json
    python3 scripts/fixtures/asbuilt_cases.py --check   # writes nothing; compares

A Harita project at TUREF / TM36 coordinates (from X0, Y0): the design surface “Tasarım”, a plane
G(x, y) = 100 + 0.02 x + 0.01 y over [0, 100]², its corners every 10 m; the design points on `tasarim` (names in Ad,
heights on the plane, P8 without one, P5 and P6 0.6 m apart); the kerb lines on `bordur` (a polyline east along
y = 30, a line north along x = 50); the points measured on `olcum` with their σ in Hassasiyet.

The rules, by hand (Trimble Business Center's Point and String Conformance reports' questions):
- a measured point matches the one design point of its name; two of the name: ambiguous; none and a radius given:
  the design points within it that no measured point claims by name, one a proposal (`place`), two or more
  ambiguous;
- the differences are measured − design: east, north, height, the horizontal distance; the surface difference is the
  measured height − G; the normal offset is to the nearest point of the nearest design line, left of its direction
  positive, for a point no design point matches and (a radius given) within it of a line;
- a limit not given is not checked; σ beyond its limit flags `lowQuality` and leaves the status;
- the status: ambiguous; unmatched when neither a design point nor a line takes it (a surface alone takes every
  point); else outside (a limit broken), within;
- the digest: SHA-256 of `p<TAB>name<TAB>x<TAB>y<TAB>z` (six decimals, `-` for none) for each design point, then
  `l<TAB>x,y;x,y` for each design line, each line ended by a newline.
"""

import copy
import hashlib
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.asbuilt.check.json"
X0, Y0 = 487000.0, 4420000.0
W = 1e-6
FLAGS = ["proposed", "outsideHorizontal", "outsideVertical", "outsideSurface", "outsideLine", "lowQuality",
         "noHeight", "offSurface"]


def uid(n):
    return f"0192f700-1700-7000-8000-{n:012x}"


def G(x, y):
    if not (0.0 <= x <= 100.0 and 0.0 <= y <= 100.0):
        return None
    return 100.0 + 0.02 * x + 0.01 * y


DESIGN = [("P1", 10.0, 10.0, 100.3), ("P2", 20.0, 10.0, 100.5), ("P3", 30.0, 10.0, 100.7), ("P4", 40.0, 10.0, 100.9),
          ("P5", 60.0, 60.0, 101.8), ("P6", 60.6, 60.0, 101.812), ("P7", 80.0, 80.0, 102.4), ("P8", 90.0, 10.0, None)]
# (kind, points)
LINES = [("polyline", [(0.0, 30.0), (100.0, 30.0)]), ("line", [(50.0, 40.0), (50.0, 90.0)])]
# (name, x, y, z, σ)
MEASURED = [("P1", 10.01, 10.0, 100.31, "0.01"), ("P2", 20.0, 10.05, 100.5, "0.01"), ("P3", 30.0, 10.0, 100.74, None),
            ("p4x", 40.1, 10.05, 100.9, None), ("X9", 60.3, 60.1, 101.8, None), ("R1", 70.0, 20.0, 101.6, None),
            ("S1", 10.0, 20.0, None, None), ("P8", 90.0, 10.0, 101.9, "0.08"), ("B1", 25.0, 29.97, 100.8, None),
            ("B2", 50.01, 60.0, 101.6, "0,02")]


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def P(x, y):
    return {"x": X0 + x, "y": Y0 + y}


def setup(work="map"):
    layers = [layer("0", "0"), layer("zemin", "Tasarım yüzeyi"), layer("tasarim", "Tasarım noktaları"),
              layer("bordur", "Bordür"), layer("olcum", "Ölçülen"), layer("bos", "Boş")]
    ents = []

    def add(e):
        e["id"] = len(ents) + 1
        ents.append(e)

    for j in range(11):
        for i in range(11):
            x, y = 10.0 * i, 10.0 * j
            add({"kind": "point", "layerId": "zemin", "attrs": {}, "p": P(x, y), "z": G(x, y)})
    for name, x, y, z in DESIGN:
        e = {"kind": "point", "layerId": "tasarim", "attrs": {"Ad": name}, "p": P(x, y)}
        if z is not None:
            e["z"] = z
        add(e)
    for kind, pts in LINES:
        if kind == "line":
            add({"kind": "line", "layerId": "bordur", "attrs": {}, "a": P(*pts[0]), "b": P(*pts[1])})
        else:
            add({"kind": "polyline", "layerId": "bordur", "attrs": {}, "pts": [P(*p) for p in pts]})
    for name, x, y, z, s in MEASURED:
        attrs = {"Ad": name}
        if s is not None:
            attrs["Hassasiyet"] = s
        e = {"kind": "point", "layerId": "olcum", "attrs": attrs, "p": P(x, y)}
        if z is not None:
            e["z"] = z
        add(e)
    settings = {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work in ("map", "road"):
        settings["road"] = {"surfaces": [{"id": "t-1", "name": "Tasarım", "points": {"layers": ["zemin"]}},
                                         {"id": "t-2", "name": "Boş", "points": {"layers": ["bos"]}}]}
    if work == "gis":
        settings["workspace"] = "gis"
        settings.pop("cadWork")
    return {"format": "kentos.document", "version": 2, "name": "Uygulama", "settings": settings,
            "origin": {"x": X0, "y": Y0}, "layers": layers, "activeLayer": "0", "entities": ents,
            "uids": [uid(e["id"]) for e in ents], "styles": {"items": [], "categories": []}}


def ids():
    """The persistent ids of the design points, the lines and the measured points, in the drawing's order."""
    n = 121
    design = [uid(n + 1 + i) for i in range(len(DESIGN))]
    n += len(DESIGN)
    lines = [uid(n + 1 + i) for i in range(len(LINES))]
    n += len(LINES)
    measured = [uid(n + 1 + i) for i in range(len(MEASURED))]
    return design, lines, measured


def number(text):
    if text is None:
        return None
    t = text.strip().replace(",", ".")
    body = t[1:] if t.startswith("-") else t
    whole, sep, frac = body.partition(".")
    if not whole.isdigit() or (sep and not frac.isdigit()):
        return None
    return float(t)


def digest(with_design=True, with_lines=True):
    text = ""
    if with_design:
        for name, x, y, z in DESIGN:
            zt = "-" if z is None else f"{z:.6f}"
            text += f"p\t{name}\t{X0 + x:.6f}\t{Y0 + y:.6f}\t{zt}\n"
    if with_lines:
        for _, pts in LINES:
            text += "l\t" + ";".join(f"{X0 + x:.6f},{Y0 + y:.6f}" for x, y in pts) + "\n"
    return hashlib.sha256(text.encode()).hexdigest()


def signed(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    l2 = dx * dx + dy * dy
    t = 0.0 if l2 == 0 else max(0.0, min(1.0, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2))
    d = math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy)
    return -d if dx * (p[1] - a[1]) - dy * (p[0] - a[0]) < 0 else d


def report(design=True, surface=True, lines=True, search=None, horizontal=None, vertical=None, quality=None,
           expected=None, quality_field=True):
    d_ids, l_ids, m_ids = ids()
    names = [n for n, *_ in DESIGN] if design else []
    claimed = set()
    for name, *_ in MEASURED:
        hits = [i for i, n in enumerate(names) if n == name]
        if len(hits) == 1:
            claimed.add(hits[0])
    used = set()
    out, counts = [], {"within": 0, "outside": 0, "unmatched": 0, "ambiguous": 0}
    low = 0
    hs, vs = [], []
    for k, (name, x, y, z, s) in enumerate(MEASURED):
        flags = []
        hits = [i for i, n in enumerate(names) if n == name] if name else []
        match, by, amb = None, None, False
        if len(hits) == 1:
            match, by = hits[0], "name"
        elif len(hits) > 1:
            amb = True
        elif search is not None and design:
            near = [i for i, (_, dx, dy, _) in enumerate(DESIGN) if i not in claimed and math.hypot(dx - x, dy - y) <= search]
            if len(near) == 1:
                match, by = near[0], "place"
                flags.append("proposed")
            elif len(near) > 1:
                amb = True
        sigma = number(s) if quality_field else None
        row = {"uid": m_ids[k], "name": name}
        outside = False
        if match is not None:
            used.add(match)
            dn_, dx, dy, dz = DESIGN[match]
            de, dn = x - dx, y - dy
            h = math.hypot(de, dn)
            row.update({"designUid": d_ids[match], "designName": dn_, "by": by, "de": de, "dn": dn, "horizontal": h})
            hs.append(h)
            if horizontal is not None and h > horizontal:
                flags.append("outsideHorizontal")
                outside = True
            if z is not None and dz is not None:
                dh = z - dz
                row["dh"] = dh
                vs.append(dh)
                if vertical is not None and abs(dh) > vertical:
                    flags.append("outsideVertical")
                    outside = True
        if surface:
            g = G(x, y)
            if g is None:
                flags.append("offSurface")
            elif z is not None:
                row["surface"] = z - g
                if vertical is not None and abs(z - g) > vertical:
                    flags.append("outsideSurface")
                    outside = True
        on_line = False
        if lines and match is None and not amb:
            best = None
            for li, (_, pts) in enumerate(LINES):
                for a, b in zip(pts, pts[1:]):
                    d = signed((x, y), a, b)
                    if best is None or abs(d) < abs(best[0]):
                        best = (d, li)
            if search is None or abs(best[0]) <= search:
                on_line = True
                row["normal"] = best[0]
                row["lineUid"] = l_ids[best[1]]
                if horizontal is not None and abs(best[0]) > horizontal:
                    flags.append("outsideLine")
                    outside = True
        if sigma is not None:
            row["sigma"] = sigma
        if quality is not None and sigma is not None and sigma > quality:
            flags.append("lowQuality")
            low += 1
        if z is None and (surface or (match is not None and DESIGN[match][3] is not None)):
            flags.append("noHeight")
        only_surface = not design and not lines
        if design and amb:
            status = "ambiguous"
        elif match is None and not on_line and not only_surface:
            status = "unmatched"
        elif outside:
            status = "outside"
        else:
            status = "within"
        counts[status] += 1
        row["status"] = status
        row["flags"] = sorted(flags, key=FLAGS.index)
        out.append(row)
    missing = [{"uid": d_ids[i], "name": DESIGN[i][0]} for i in range(len(DESIGN)) if design and i not in used]
    dig = digest(design, lines)
    result = {"designDigest": dig, "points": out, "missing": missing, **counts, "lowQuality": low}
    if expected is not None and expected != dig:
        result["designChanged"] = True
    if hs:
        result["rmsHorizontal"] = math.sqrt(sum(h * h for h in hs) / len(hs))
        result["maxHorizontal"] = max(hs)
    if vs:
        result["rmsVertical"] = math.sqrt(sum(v * v for v in vs) / len(vs))
        result["maxVertical"] = max(abs(v) for v in vs)
    return result


# ── The cases ─────────────────────────────────────────────────────────────────────────────────────────────────────
SAME = {"revision": "same", "canUndo": False}
FULL = {"measured": "olcum", "design": "tasarim", "surface": "Tasarım", "lines": "bordur", "search": 0.5,
        "horizontal": 0.02, "vertical": 0.03, "qualityField": "Hassasiyet", "quality": 0.05}
NOT_MAP = ("not_a_road_project", "Arazi yüzeyi Harita ve Yol projelerinde olur (KentOS Harita, KentOS Yol); bu çizim "
                                 "ikisinden biri değil.")


def inp(**over):
    d = dict(FULL)
    for k, v in over.items():
        if v is None:
            d.pop(k, None)
        else:
            d[k] = v
    return d


def answer(i):
    return report(design="design" in i, surface="surface" in i, lines="lines" in i, search=i.get("search"),
                  horizontal=i.get("horizontal"), vertical=i.get("vertical"), quality=i.get("quality"),
                  expected=i.get("expectedDesign"), quality_field="qualityField" in i)


def ok(name, i, op="plan", setup_=None):
    step = {"op": op, "input": i, "result": {"status": "completed", "output": None if op == "validate" else answer(i),
                                             "warnings": []}, "expect": SAME, "within": W}
    case = {"name": name, "steps": [step]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, i, code, message, path_=None, setup_=None):
    err = {"code": code, "message": message}
    if path_ is not None:
        err["path"] = path_
    case = {"name": name, "steps": [{"op": "execute", "input": i, "result": {"status": "failed", "error": err},
                                     "expect": SAME}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def cases():
    full = answer(FULL)
    st = {p["name"]: (p["status"], p["flags"]) for p in full["points"]}
    assert st["P1"] == ("within", []), st["P1"]
    assert st["P2"][0] == "outside" and "outsideHorizontal" in st["P2"][1]
    assert "outsideVertical" in st["P3"][1] and "outsideSurface" in st["P3"][1]
    assert st["p4x"][1][0] == "proposed"
    assert st["X9"][0] == "ambiguous"
    assert st["R1"][0] == "unmatched" and st["S1"][1] == ["noHeight"]
    assert st["B1"] == ("outside", ["outsideLine"]) and st["B2"] == ("within", []), (st["B1"], st["B2"])
    assert "lowQuality" in st["P8"][1] and st["P8"][0] == "within"
    assert [m["name"] for m in full["missing"]] == ["P5", "P6", "P7"]
    bad = "invalid_asbuilt_check"
    return [
        ok("tam: noktalar adla ve yerle, yüzey, bordür; toleranslar ve kalite sınırı", FULL),
        ok("uygula: salt okunur, çizim değişmez", FULL, op="execute"),
        ok("doğrulama", FULL, op="validate"),
        ok("yalnız adla: yarıçapsız, öneri ve belirsiz yok", inp(search=None)),
        ok("tasarım noktası yok: yüzey ve bordür, durum toleranstan", inp(design=None, search=None)),
        ok("toleranssız: yalnız farklar", inp(horizontal=None, vertical=None, quality=None, qualityField=None)),
        ok("yalnız bordür, 0,5 m içindekiler", {"measured": "olcum", "lines": "bordur", "horizontal": 0.02, "search": 0.5}),
        ok("yalnız bordür, yarıçapsız: her nokta en yakın çizgiye", {"measured": "olcum", "lines": "bordur"}),
        ok("yalnız yüzey: her nokta yüzeye", {"measured": "olcum", "surface": "Tasarım", "vertical": 0.03}),
        ok("aynı tasarım özeti: değişmedi", inp(expectedDesign=digest())),
        ok("başka tasarım özeti: tasarım değişti, yine karşılaştırılır", inp(expectedDesign="0" * 64)),
        ok("CBS projesinde yüzeysiz", inp(surface=None), setup_=setup("gis")),
        ok("Yol projesinde", FULL, setup_=setup("road")),
        refused("tasarım yok", {"measured": "olcum"}, bad, "Karşılaştırılacak tasarım yok: tasarım noktalarının "
                "katmanını, yüzeyi ya da tasarım çizgilerinin katmanını verin.", "design"),
        refused("sıfır arama yarıçapı", inp(search=0.0), bad,
                "Arama yarıçapı 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "search"),
        refused("eksi yatay tolerans", inp(horizontal=-0.01), bad,
                "Yatay tolerans 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "horizontal"),
        refused("sıfır düşey tolerans", inp(vertical=0.0), bad,
                "Düşey tolerans 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.", "vertical"),
        refused("boş Ad özniteliği", inp(field=" "), bad,
                "Ad özniteliği boş; özniteliğin adını yazın ya da boş bırakın (Ad).", "field"),
        refused("boş kalite özniteliği", inp(qualityField=""), bad,
                "Kalite özniteliği boş; σ'nın yazıldığı özniteliğin adını yazın.", "qualityField"),
        refused("kalite sınırı özniteliksiz", inp(qualityField=None), bad,
                "Kalite sınırı verildi ama σ'nın özniteliği yok; qualityField'ı da verin.", "qualityField"),
        refused("olmayan ölçülen katman", inp(measured="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "measured"),
        refused("olmayan tasarım katmanı", inp(design="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "design"),
        refused("olmayan bordür katmanı", inp(lines="yok"), "layer_not_found", "“yok” kimlikli katman çizimde yok.",
                "lines"),
        refused("olmayan yüzey", inp(surface="Kazı"), "unknown_terrain", "“Kazı” kimlikli ya da adlı yüzey projede yok.",
                "surface"),
        refused("kurulamayan yüzey", inp(surface="Boş"), "terrain_unbuildable", "$any", None),
        refused("CBS projesinde yüzey", FULL, NOT_MAP[0], NOT_MAP[1], None, setup_=setup("gis")),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.asbuilt.check", "commandVersion": 1,
           "title": "Uygulama kontrolü",
           "note": "ADR 1700. Bağımsız başvuru scripts/fixtures/asbuilt_cases.py: tasarım yüzeyi düzlem, eşleme, farklar, "
                   "işaretli dik uzaklık, durum ve özet elle; KentOS kodu yok. Toleranslar durumların kendi değerleridir.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Uygulama kontrolü ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
