#!/usr/bin/env python3
"""The shared cases of `road.terrains.sheetcontours` (Pafta eş yükselti eğrileri; plan/map-cad.md MAP-14,
docs/adr/1820), without KentOS code.

    uv run --no-project --python 3.11 --with triangle --with numpy python3 scripts/fixtures/road_sheet_contour_cases.py
    ... --check   # compares the file with the one on disk

The surface is Shewchuk's Triangle's (road_terrain_command_cases.py's build) of 336 points on a 20 m grid shaken by up
to 5 m: a plain rising 2 cm a metre, a scarp (15 m over a logistic of 4 m) and a hill (30 m, σ 60 m); a building and a
road drawn as areas and a slope boundary as a closed polyline are the details.

The rules, by hand (BÖHHBÜY 2018 m. 78(1)(c)–(g), Ek-3 0001–0005, the package's values):
- a triangle's spacing on the sheet is the interval over its slope (rise over run of its plane, exact fractions), in
  paper millimetres at the scale; under `steepBelow` it draws major lines only, over `auxiliaryAbove` the auxiliary
  line half an interval above each level;
- the level lines over the triangles taken, as road_terrain_command_cases.py's contours (a corner on the level counts
  above, pieces chained by their edges, a line from its fixed place, the lines by their first places); the levels from
  the base by the interval, every `majorEvery`-th major, the auxiliary ones between, all in height order;
- a line's segment split where it crosses a detail ring's edge (p's parameter strictly between 0 and 1, the edge's
  within [0, 1]), the parts whose middles lie inside a detail (even-odd over its rings) left out, their length counted;
  a closed line cut opens, its last part joined to its first through the closing segment;
- along each major part from its first place a height every `labelEvery` paper mm, the first at half that: in the
  middle of its segment's place, turned along it, its top uphill (against the fall direction of the triangle strictly
  around the segment's middle), its text three decimals at most.
"""

import copy
import json
import math
import random
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_terrain_command_cases as rt  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.terrains.sheetcontours.json"
RULES = json.loads((ROOT / "catalog/v1/rules/esyukselti-bohhbuy-2018.json").read_text("utf-8"))


def U(n):
    return f"0192f5c0-1820-7000-8000-{n:012x}"


def ground(x, y):
    z = 850.0 + 0.02 * x
    z += 30.0 * math.exp(-((x - 250.0) ** 2 + (y - 150.0) ** 2) / (2 * 60.0 ** 2))
    z += 15.0 / (1.0 + math.exp(-(x - 100.0) / 4.0))
    return z


def shaken_points():
    rnd = random.Random(1820)
    out = []
    for j in range(16):
        for i in range(21):
            x = min(400.0, max(0.0, i * 20.0 + rnd.uniform(-5.0, 5.0)))
            y = min(300.0, max(0.0, j * 20.0 + rnd.uniform(-5.0, 5.0)))
            x, y = round(x, 3), round(y, 3)
            out.append((x, y, round(ground(x, y), 3)))
    return out


POINTS = shaken_points()
BUILDING = [(230.0, 130.0), (272.0, 130.0), (272.0, 166.0), (230.0, 166.0)]
ROAD = [(-10.0, 58.5), (410.0, 70.5), (410.0, 82.5), (-10.0, 70.5)]
SLOPE = [(85.0, 205.0), (118.0, 205.0), (118.0, 236.0), (85.0, 236.0)]
TWO = [(500.0, 500.0, 850.0), (510.0, 500.0, 851.0)]

LAYERS = [rt.node("0", "0"), rt.node("zemin", "Zemin"), rt.node("bina", "Bina"), rt.node("yol", "Yol"),
          rt.node("sev", "Şev sınırı"), rt.node("az", "Az nokta"), rt.node("esyukselti", "Eş yükselti"),
          rt.node("arazi", "Arazi", [rt.node("ek", "Ek")]), {**rt.node("kilitli", "Kilitli"), "locked": True},
          {**rt.node("gizli", "Gizli"), "visible": False}]


def P(x, y):
    return {"x": float(x), "y": float(y)}


def entities():
    ents = [("point", "zemin", {"p": P(x, y), "z": z}) for x, y, z in POINTS]
    ents.append(("polygon", "bina", {"pts": [P(*p) for p in BUILDING]}))
    ents.append(("polygon", "yol", {"pts": [P(*p) for p in ROAD]}))
    ents.append(("polyline", "sev", {"pts": [P(*p) for p in SLOPE], "closed": True}))
    ents += [("point", "az", {"p": P(x, y), "z": z}) for x, y, z in TWO]
    return ents


ENTS = entities()
SURFACES = [{"id": "t-1", "name": "Mevcut zemin", "points": {"layers": ["zemin"]}},
            {"id": "t-9", "name": "Az nokta", "points": {"layers": ["az"]}}]


def setup(work="map"):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    # Surfaces are a Harita or Yol project's (an architecture project's file holds none).
    if work in ("map", "road"):
        settings["road"] = {"surfaces": copy.deepcopy(SURFACES)}
    return {"format": "kentos.document", "version": 2, "name": "Pafta eğrileri", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
            "entities": [{"kind": k, "id": i + 1, "layerId": l, "attrs": {}, **copy.deepcopy(g)}
                         for i, (k, l, g) in enumerate(ENTS)],
            "uids": [U(i + 1) for i in range(len(ENTS))], "styles": {"items": [], "categories": []}}


def surface_of(points):
    g = {"points": [(x, y, z, U(i + 1)) for i, (x, y, z) in enumerate(points)], "breaklines": [], "boundary": None,
         "voids": [], "tolerance": None, "maxEdge": None, "edges": [], "removed": []}
    return rt.build(g)


S = surface_of(POINTS)


def level_lines(s, z, keep):
    """road_terrain_command_cases.py's contours for one level, over the triangles `keep` takes."""
    above = lambda i: s["z"][i] >= z  # noqa: E731
    F = rt.F

    def cross(a, b):
        (ax, ay), (bx, by) = s["xy"][a], s["xy"][b]
        t = (F(z) - F(s["z"][a])) / (F(s["z"][b]) - F(s["z"][a]))
        return (float(F(ax) + t * (F(bx) - F(ax))), float(F(ay) + t * (F(by) - F(ay))))
    pieces = []
    for ti, t in enumerate(s["tris"]):
        if not keep(ti):
            continue
        e = [tuple(sorted((a, b))) for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])) if above(a) != above(b)]
        if len(e) == 2:
            pieces.append(e)
    at = {}
    for i, p in enumerate(pieces):
        for e in p:
            at.setdefault(e, []).append(i)
    used = [False] * len(pieces)
    order = sorted(range(len(pieces)), key=lambda i: min(len(at[e]) for e in pieces[i]))
    lines = []
    for start in order:
        if used[start]:
            continue
        e0, e1 = pieces[start]
        frm = e0 if len(at[e0]) == 1 else e1 if len(at[e1]) == 1 else e0
        chain, piece, edge = [frm], start, frm
        while True:
            used[piece] = True
            edge = pieces[piece][1] if pieces[piece][0] == edge else pieces[piece][0]
            chain.append(edge)
            nxt = [i for i in at[edge] if not used[i]]
            if not nxt:
                break
            piece = nxt[0]
        pts = [cross(*e) for e in chain]
        closed = len(chain) > 2 and chain[0] == chain[-1]
        pts = [p for i, p in enumerate(pts) if i == 0 or p != pts[i - 1]]
        if closed and len(pts) > 1 and pts[0] == pts[-1]:
            pts = pts[:-1]
        if len(pts) < (3 if closed else 2):
            continue
        if closed:
            n = len(pts)
            s0 = min(range(n), key=lambda i: pts[i])
            fwd = pts[(s0 + 1) % n] < pts[(s0 - 1) % n]
            pts = [pts[(s0 + j) % n] if fwd else pts[(s0 - j) % n] for j in range(n)]
        elif pts[-1] < pts[0]:
            pts.reverse()
        lines.append(([list(p) for p in pts], closed))
    lines.sort(key=lambda l: tuple(l[0][0]))
    return lines


def inside(rings, p):
    odd = False
    for ring in rings:
        n = len(ring)
        for i in range(n):
            a, b = ring[i], ring[(i + 1) % n]
            if (a[1] > p[1]) != (b[1] > p[1]):
                x = a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
                if p[0] < x:
                    odd = not odd
    return odd


def crossing(p, q, a, b):
    rx, ry = q[0] - p[0], q[1] - p[1]
    sx, sy = b[0] - a[0], b[1] - a[1]
    d = rx * sy - ry * sx
    if d == 0.0:
        return None
    wx, wy = a[0] - p[0], a[1] - p[1]
    t = (wx * sy - wy * sx) / d
    u = (wx * ry - wy * rx) / d
    return t if 0.0 < t < 1.0 and 0.0 <= u <= 1.0 else None


def clip(points, closed, details):
    n = len(points)
    segments = n if closed else n - 1
    parts, current, cut, touched = [], [], 0.0, False
    for i in range(segments):
        p, q = points[i], points[(i + 1) % n]
        ts = [0.0, 1.0]
        for d in details:
            for ring in d:
                m = len(ring)
                for j in range(m):
                    t = crossing(p, q, ring[j], ring[(j + 1) % m])
                    if t is not None:
                        ts.append(t)
        ts = sorted(set(ts))
        for t0, t1 in zip(ts, ts[1:]):
            a = [p[0] + t0 * (q[0] - p[0]), p[1] + t0 * (q[1] - p[1])]
            b = [p[0] + t1 * (q[0] - p[0]), p[1] + t1 * (q[1] - p[1])]
            mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
            if any(inside(d, mid) for d in details):
                cut += math.hypot(b[0] - a[0], b[1] - a[1])
                touched = True
                if len(current) >= 2:
                    parts.append(current)
                current = []
            else:
                if not current:
                    current.append(a)
                current.append(b)
    if len(current) >= 2:
        parts.append(current)
    if closed and not touched:
        return [points], 0.0, True
    if closed and len(parts) >= 2 and parts[0][0] == points[0] and parts[-1][-1] == points[0]:
        head = parts.pop(0)
        parts[-1].extend(head[1:])
    return parts, cut, False


def level_text(z):
    r = math.floor(z * 1000.0 + 0.5) / 1000.0
    t = repr(r)
    return t[:-2] if t.endswith(".0") else t


def labels_of(s, points, closed, z, every):
    n = len(points)
    segs = [(points[i], points[(i + 1) % n]) for i in range(n if closed else n - 1)]
    out, s_at, walked = [], every / 2.0, 0.0
    for p, q in segs:
        ln = math.hypot(q[0] - p[0], q[1] - p[1])
        if ln == 0.0:
            continue
        while s_at <= walked + ln:
            f = (s_at - walked) / ln
            at = [p[0] + f * (q[0] - p[0]), p[1] + f * (q[1] - p[1])]
            d = [(q[0] - p[0]) / ln, (q[1] - p[1]) / ln]
            mid = [(p[0] + q[0]) / 2.0, (p[1] + q[1]) / 2.0]
            t = rt.strictly_in(s, mid)
            if t is not None:
                _, aspect = rt.slope_of(s, t)
                if aspect is not None:
                    uphill = [-math.sin(aspect), -math.cos(aspect)]
                    left = [-d[1], d[0]]
                    if left[0] * uphill[0] + left[1] * uphill[1] < 0.0:
                        d = [-d[0], -d[1]]
            out.append({"z": z, "text": level_text(z), "at": at, "rotation": math.degrees(math.atan2(d[1], d[0]))})
            s_at += every
        walked += ln
    return out


def num(v):
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def details_of(layers):
    out = []
    for kind, layer, g in ENTS:
        if layer not in layers:
            continue
        if kind == "polygon" or (kind == "polyline" and g.get("closed")):
            rings = [[(p["x"], p["y"]) for p in g["pts"]]]
            out.append([r for r in rings if len(r) >= 3])
    return [d for d in out if d]


def sheet(scale=1000, interval=None, base=0.0, details=(), steep=True, auxiliary=True, labels=True, rules=None):
    rules = rules or RULES
    if interval is None:
        interval = next(i["interval"] for i in rules["intervals"] if i["scale"] == scale)
    s = S
    used = {i for t in s["tris"] for i in t}
    lo, hi = min(s["z"][i] for i in used), max(s["z"][i] for i in used)
    first, last = math.ceil((lo - base) / interval), math.floor((hi - base) / interval)
    mm = 1000.0 / scale
    spacing = []
    for t in s["tris"]:
        slope, _ = rt.slope_of(s, t)
        spacing.append(interval / slope * mm if slope > 0.0 else math.inf)
    steep_t = [steep and v < rules["steepBelow"] for v in spacing]
    gentle_t = [auxiliary and v > rules["auxiliaryAbove"] for v in spacing]
    dets = details_of(details)
    lines, cut = [], 0.0

    def add(z, kind, found):
        nonlocal cut
        for points, closed in found:
            parts, gone, still = clip(points, closed, dets)
            cut += gone
            for part in parts:
                lines.append({"z": z, "kind": kind, "closed": still, "points": part})
    for m in range(2 * first - 1, 2 * last + 2):
        half = m / 2.0
        z = base + half * interval
        if m % 2 == 0:
            k = m // 2
            major = rules["majorEvery"] > 0 and k % rules["majorEvery"] == 0
            found = level_lines(s, z, (lambda t: True) if major else (lambda t: not steep_t[t]))
            add(z, "major" if major else "minor", found)
        elif auxiliary:
            add(z, "auxiliary", level_lines(s, z, lambda t: gentle_t[t]))
    every = rules["labelEvery"] / mm
    labs = [lab for l in lines if l["kind"] == "major" for lab in labels_of(s, l["points"], l["closed"], l["z"], every)] \
        if labels else []
    n_steep, n_gentle = sum(steep_t), sum(gentle_t)
    notes = [f"Eğriler {rules['id']} paketine göre: aralık {num(interval)} m (1/{scale}), her {rules['majorEvery']}. "
             f"eğri kalın ({num(rules['majorWeight'])} mm), öbürleri {num(rules['minorWeight'])} mm."]
    if n_steep:
        notes.append(f"Eğri arası paftada {num(rules['steepBelow'])} mm'den az olan {n_steep} üçgende yalnız kalın "
                     "eğriler çizildi.")
    if n_gentle:
        notes.append(f"Eğri arası {num(rules['auxiliaryAbove'])} mm'yi aşan {n_gentle} üçgende yarım aralıkta kesik "
                     "yardımcı eğri çizildi.")
    if labels:
        notes.append(f"Kalın eğrilere her {num(rules['labelEvery'])} mm'de bir yükseklik yazıldı "
                     f"({num(rules['labelHeight'])} mm); yazının kutusu maskelenir, eğri bölünmez.")
    if details:
        notes.append(f"Detayların içinde kalan {cut:.2f} m eğri çizilmedi (m. 78(1)(g)).")
    return {"rules": rules["id"], "source": rules["source"], "scale": scale, "interval": interval, "lines": lines,
            "labels": labs, "steep": n_steep, "gentle": n_gentle, "cut": cut, "revision": "$current", "notes": notes}


N = len(ENTS)


def drawn(cases, name, input_, op="execute", layer="esyukselti", warns=(), note=None, **kw):
    out = sheet(**kw)
    written = op == "execute"
    if written:
        out["uids"] = "$any"
    count = len(out["lines"]) + len(out["labels"])
    expect = ({"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed",
               "ids": list(range(1, N + count + 1)), "road": rt.road_of(SURFACES)}
              if written else {**rt.UNCHANGED, "ids": list(range(1, N + 1))})
    result = rt.completed(out, warns) if op != "validate" else rt.completed(None, warns)
    cases.append({"name": name, **({"note": note} if note else {}),
                  "steps": [rt.step(op, {"surface": "t-1", "layerId": layer, **input_}, result, expect, rt.W)]})
    return out


def refused(cases, name, input_, result, setup_=None):
    c = {"name": name, "steps": [rt.step("execute", {"surface": "t-1", "layerId": "esyukselti", **input_}, result,
                                         {**rt.UNCHANGED, "ids": list(range(1, N + 1))})]}
    if setup_ is not None:
        c["setup"] = setup_
    cases.append(c)


def cases():
    out = []
    all3 = ["bina", "yol", "sev"]
    main = drawn(out, "1/1000: m. 78(1)(c)–(g) ilk değerleriyle, bina, yol ve şev kesilir", {"details": all3},
                 details=all3)
    kinds = {l["kind"] for l in main["lines"]}
    assert kinds == {"major", "minor", "auxiliary"} and main["labels"] and main["cut"] > 0, (kinds, main["cut"])
    assert main["steep"] > 0 and main["gentle"] > 0, (main["steep"], main["gentle"])
    drawn(out, "Plan yazmaz", {"details": all3}, op="plan", details=all3)
    drawn(out, "Doğrulama yazmaz", {}, op="validate")
    drawn(out, "Detaysız: eğriler kesilmez", {})
    drawn(out, "1/2000: aralık 2 m, yazı her 400 m", {"scale": 2000}, scale=2000)
    drawn(out, "1/500: aralık 1 m, yazı her 100 m", {"scale": 500}, scale=500)
    drawn(out, "Kurallar kapalı: yalnız aralık ve kalınlık", {"steep": False, "auxiliary": False, "labels": False},
          steep=False, auxiliary=False, labels=False)
    drawn(out, "Elle aralık 0,5 m", {"interval": 0.5}, interval=0.5)
    drawn(out, "Taban kotu 0,5", {"base": 0.5}, base=0.5)
    drawn(out, "1/1500: paket aralık vermez, elle 1 m", {"scale": 1500, "interval": 1.0}, scale=1500, interval=1.0)
    other = dict(copy.deepcopy(RULES), id="deneme-10-50", source="Deneme paketi (sentetik)", steepBelow=10.0,
                 auxiliaryAbove=50.0)
    drawn(out, "Başka kural paketi: dik 10 mm, yardımcı 50 mm", {"rules": other}, rules=other)
    drawn(out, "Gizli katmana uyarıyla", {}, layer="gizli",
          warns=[{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.",
                  "path": "layerId"}])
    bad = rt.failed("invalid_contours", "Eş yükselti aralığı 0'dan büyük bir sayı, taban kotu sonlu bir sayı olmalı.",
                    "interval")
    refused(out, "Yol ya da Harita projesi değil", {}, rt.NOT_ROAD, setup(work="architecture"))
    refused(out, "1/1500 aralıksız", {"scale": 1500},
            rt.failed("invalid_contours", "bohhbuy-2018-m78 paketi 1/1500 ölçeği için eğri aralığı vermiyor; aralığı "
                      "yazın.", "interval"))
    refused(out, "Ölçek 0", {"scale": 0}, rt.failed("invalid_contours", "Ölçek paydası 0 olamaz.", "scale"))
    refused(out, "Sıfır aralık", {"interval": 0.0}, bad)
    refused(out, "Kural paketi eksik", {"rules": dict(copy.deepcopy(RULES), id="")},
            rt.failed("invalid_contours", "Eğri kural paketi eksik: kimliği ve kaynağı yazılı, her değeri sonlu ve "
                      "sıfırdan büyük olmalı.", "rules"))
    refused(out, "Çok sık düzey", {"interval": 0.001},
            rt.failed("invalid_contours", "Bu aralıkla yüzeyi 10 000'den çok düzey keser; aralığı büyütün.", "interval"))
    refused(out, "Kesen düzey yok", {"interval": 1000.0},
            rt.failed("no_contours", "Bu aralıkla yüzeyi kesen eş yükselti eğrisi yok; aralığı ya da taban kotunu "
                      "değiştirin.", "interval"))
    refused(out, "Olmayan katman", {"layerId": "yok"},
            rt.failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                      "layerId"))
    refused(out, "Grup katman değil", {"layerId": "arazi"},
            rt.failed("not_a_layer", "“Arazi” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir "
                      "katman seçin.", "layerId"))
    refused(out, "Kilitli katman", {"layerId": "kilitli"},
            rt.failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir "
                      "katmanı etkinleştirin.", "layerId"))
    refused(out, "Olmayan detay katmanı", {"details": ["yok"]},
            rt.failed("layer_not_found", "“yok” katmanı çizimde yok.", "details"))
    refused(out, "Olmayan yüzey", {"surface": "yok"},
            rt.failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    refused(out, "Kurulamayan yüzey", {"surface": "t-9"},
            rt.failed("terrain_unbuildable", f"Kaynaklardan yüzey kurulamıyor: {rt.PROBLEM['too_few_points']}."))
    refused(out, "Eski sürüme göre hazırlanmış", {"expectedRevision": "41"}, rt.CONFLICT)
    refused(out, "Geçersiz beklenen sürüm", {"expectedRevision": "x"},
            rt.failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                      "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.sheetcontours",
           "commandVersion": 1, "title": "Pafta eş yükselti eğrileri (BÖHHBÜY m. 78)",
           "note": "ADR 1820. Bağımsız başvuru scripts/fixtures/road_sheet_contour_cases.py: yüzey Triangle'ın, eğri arası "
                   "kuralı, kırpma ve yazılar elle; KentOS kodu yok. Yazma tek geri alma adımıdır.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text("utf-8") == text
        print("Pafta eğrileri ortak durumları güncel." if ok else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum, {len(text) // 1024} KB)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
