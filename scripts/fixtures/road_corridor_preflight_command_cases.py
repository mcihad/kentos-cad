#!/usr/bin/env python3
"""The shared cases of `road.corridors.preflight` (plan/road.md Y50, docs/adr/0956; ROAD-COR-13).

The corridor's crosses are road_corridor_command_cases.py's (no KentOS code). Over them, here:

- each cross's margin against folding, the least `1 + κ·o` over its top line's points, κ the alignment's curvature
  by the mpmath reference (road_alignment_command_cases.place);
- the top surface's strips again from the reference's own strip rule, and their overlaps by GEOS (shapely): pairs of
  strips that share no station, each pair of triangles intersected, the pieces' area and the two planes' heights at
  each piece's centroid;
- the seams, the crosses at a region's end and the next's start compared code by code.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj --with shapely \\
        python3 scripts/fixtures/road_corridor_preflight_command_cases.py [--check]
"""

import json
import math
import sys
from pathlib import Path

from shapely.geometry import Polygon

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_command_cases as acmd  # noqa: E402
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.corridors.preflight.json"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
W = 1e-6
SAME = 1e-3
LEAST_OVERLAP = 1e-6
SEAM = 1e-3


def code_words(code):
    side, _, point = code.partition(".") if "." in code else ("", "", code)
    side = {"left": "sol ", "right": "sağ "}.get(side, "")
    names = {"axis": "eksen", "hinge": "şev başı", "daylight": "şev ucu", "ditchToe": "hendek ayağı",
             "ditchBottom": "hendek tabanı"}
    if point in names:
        point = names[point]
    elif point.startswith("c") and point[1:].isdigit():
        point = f"{point[1:]}. bileşen"
    elif point.startswith("bench") and point.endswith("Top") and point[5:-3].isdigit():
        point = f"{point[5:-3]}. basamak içi"
    elif point.startswith("bench") and point.endswith("End") and point[5:-3].isdigit():
        point = f"{point[5:-3]}. basamak ucu"
    return side + point


def offset_of(cross, p):
    t = cross["direction"]
    return (p[0] - cross["at"][0]) * math.sin(t) - (p[1] - cross["at"][1]) * math.cos(t)


def margins(c, regs):
    b = pcmd.ALIGNED[c["alignment"]]
    out = []
    for ri, r in enumerate(regs):
        for x in r["crosses"]:
            if x["section"] is None:
                continue
            # The curvature just before and just after: where a curve meets a tangent the cross belongs to both.
            ks = [acmd.place(b, s)["curvature"] for s in (x["station"] - 1e-9, x["station"] + 1e-9)
                  if 0.0 <= s <= float(b.length)]
            best = None
            for p in cref.top(x, r["template"]):
                o = offset_of(x, p)
                for k in ks:
                    m = 1.0 + k * (float(b.offset) + o)
                    if best is None or m < best[0]:
                        best = (m, o, k)
            out.append({"region": ri, "station": x["station"], "margin": best[0], "offset": best[1], "curvature": best[2]})
    return out


def strips(regs):
    out = []
    for ri, r in enumerate(regs):
        for x, y in zip(r["crosses"], r["crosses"][1:]):
            if x["section"] is None or y["section"] is None:
                continue
            got = cref.strip(cref.top(x, r["template"]), cref.top(y, r["template"]))
            if any(cref.area(g) <= 0 for g in got):
                continue
            out.append((ri, x["station"], y["station"], got))
    return out


def plane_height(t, p):
    a, b, c = t
    det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])
    u = ((p[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (p[1] - a[1])) / det
    v = ((b[0] - a[0]) * (p[1] - a[1]) - (p[0] - a[0]) * (b[1] - a[1])) / det
    return a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2])


def overlaps(st):
    def touch(a, b):
        x, y = st[a], st[b]
        return abs(x[1] - y[2]) < SAME or abs(x[2] - y[1]) < SAME or abs(x[1] - y[1]) < SAME

    polys = [[(Polygon([(p[0], p[1]) for p in t]), t) for t in s[3]] for s in st]
    boxes = []
    for s in st:
        xs = [p[0] for t in s[3] for p in t]
        ys = [p[1] for t in s[3] for p in t]
        boxes.append((min(xs), min(ys), max(xs), max(ys)))
    pairs = []
    for a in range(len(st)):
        for b in range(a + 1, len(st)):
            if touch(a, b):
                continue
            p, q = boxes[a], boxes[b]
            if p[2] < q[0] or q[2] < p[0] or p[3] < q[1] or q[3] < p[1]:
                continue
            area, least, most = 0.0, math.inf, -math.inf
            for pa, ta in polys[a]:
                for pb, tb in polys[b]:
                    piece = pa.intersection(pb)
                    if piece.is_empty or piece.area <= LEAST_OVERLAP:
                        continue
                    area += piece.area
                    cen = piece.centroid
                    gap = abs(plane_height(ta, (cen.x, cen.y)) - plane_height(tb, (cen.x, cen.y)))
                    least, most = min(least, gap), max(most, gap)
            if area > LEAST_OVERLAP:
                pairs.append((a, b, area, least, most))
    merged = []
    for a, b, area, least, most in pairs:
        hit = next((m for m in merged if any(x == a or touch(x, a) for x in m[0]) and any(y == b or touch(y, b) for y in m[1])),
                   None)
        if hit is None:
            merged.append([[a], [b], area, least, most])
        else:
            if a not in hit[0]:
                hit[0].append(a)
            if b not in hit[1]:
                hit[1].append(b)
            hit[2] += area
            hit[3] = min(hit[3], least)
            hit[4] = max(hit[4], most)

    def span(ids):
        return [min(st[i][1] for i in ids), max(st[i][2] for i in ids)]
    return [{"first": span(m[0]), "second": span(m[1]), "area": m[2], "least": m[3], "most": m[4]} for m in merged]


def coded_points(t, x):
    sec = x["section"]
    if sec is None:
        return None
    xy = cref.place(x, 0)
    out = [("axis", (xy[0], xy[1], sec["axis"]))]
    for word in ("left", "right"):
        for code, o, z in cref.coded(t, sec[word], word):
            q = cref.place(x, o)
            out.append((code, (q[0], q[1], z)))
    return out


def seams(regs):
    out = []
    for i in range(len(regs) - 1):
        if not regs[i]["crosses"] or not regs[i + 1]["crosses"]:
            continue
        a, b = regs[i]["crosses"][-1], regs[i + 1]["crosses"][0]
        if abs(a["station"] - b["station"]) >= SAME:
            continue
        pa, pb = coded_points(regs[i]["template"], a), coded_points(regs[i + 1]["template"], b)
        if pa is None or pb is None:
            continue
        s = {"station": a["station"], "before": i, "after": i + 1, "shift": 0.0, "shiftCode": None, "step": 0.0,
             "stepCode": None, "onlyBefore": [], "onlyAfter": []}
        for code, p in pa:
            q = next((v for c, v in pb if c == code), None)
            if q is None:
                s["onlyBefore"].append(code)
                continue
            shift, st = math.hypot(p[0] - q[0], p[1] - q[1]), abs(p[2] - q[2])
            if shift > s["shift"]:
                s["shift"], s["shiftCode"] = shift, code
            if st > s["step"]:
                s["step"], s["stepCode"] = st, code
        s["onlyAfter"] = [c for c, _ in pb if not any(d == c for d, _ in pa)]
        out.append(s)
    return out


def answer(input_, corridors=ccmd.START, templates=None):
    name = input_["corridor"]
    c = next((x for x in corridors if x["id"] == name), None) or next(
        (x for x in corridors if x["name"].lower() == name.lower()), None)
    if c is None:
        raise ccmd.Refused("unknown_corridor", f"“{name}” kimlikli ya da adlı koridor projede yok.", "corridor")
    bad = lambda words, path: ccmd.Refused("invalid_preflight",  # noqa: E731
                                           f"Koridor ön denetimi sorusu geçerli değil: {words}", path)
    margin, tolerance = input_.get("margin"), input_.get("tolerance")
    if margin is not None and not 0.0 <= margin < 1.0:
        raise bad("istenen pay 0 ile 1 arası olur (1: hiç daralmayan).", "margin")
    if tolerance is not None and not 0.0 < tolerance <= 10.0:
        raise bad("dikiş toleransı 0'dan büyük, en çok 10 m olur.", "tolerance")
    report, warnings, tris, regs = ccmd.make(c) if templates is None else ccmd.make(c, templates=templates)
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    limit = margin if margin is not None else 0.0
    tol = tolerance if tolerance is not None else SEAM
    allm = margins(c, regs)
    least = min(allm, key=lambda m: m["margin"]) if allm else None
    tight, open_, prev = [], False, None
    for m in allm:
        if m["margin"] > limit:
            open_ = False
            continue
        radius = 1.0 / abs(m["curvature"]) if abs(m["curvature"]) > 1e-12 else None
        if open_ and prev == m["region"]:
            t = tight[-1]
            t["to"], t["toText"] = m["station"], text(m["station"])
            if m["margin"] < t["least"]:
                t.update({"least": m["margin"], "at": m["station"], "offset": m["offset"]})
                t.pop("radius", None)
                if radius is not None:
                    t["radius"] = radius
        else:
            t = {"region": m["region"], "from": m["station"], "to": m["station"], "fromText": text(m["station"]),
                 "toText": text(m["station"]), "least": m["margin"], "at": m["station"], "offset": m["offset"]}
            if radius is not None:
                t["radius"] = radius
            tight.append(t)
        open_, prev = True, m["region"]
    built = cref.assemble(regs)
    folds = [{"region": f[0], "from": f[1], "to": f[2], "fromText": text(f[1]), "toText": text(f[2])} for f in built["folded"]]
    st = strips(regs)
    over = [{"first": o["first"], "second": o["second"], "firstText": [text(o["first"][0]), text(o["first"][1])],
             "secondText": [text(o["second"][0]), text(o["second"][1])], "area": o["area"], "least": o["least"],
             "most": o["most"]} for o in overlaps(st)]
    joins = []
    for s in seams(regs):
        row = {"station": s["station"], "km": text(s["station"]), "before": s["before"], "after": s["after"],
               "shift": s["shift"]}
        if s["shiftCode"] is not None:
            row["shiftCode"] = s["shiftCode"]
        row["step"] = s["step"]
        if s["stepCode"] is not None:
            row["stepCode"] = s["stepCode"]
        if s["onlyBefore"]:
            row["onlyBefore"] = s["onlyBefore"]
        if s["onlyAfter"]:
            row["onlyAfter"] = s["onlyAfter"]
        row["meets"] = s["shift"] <= tol and s["step"] <= tol and not s["onlyBefore"] and not s["onlyAfter"]
        joins.append(row)
    name_ = c["name"]
    if tight:
        def word(t):
            km = f"km {t['fromText']}" if t["from"] == t["to"] else f"km {t['fromText']} – {t['toText']}"
            r = f", R {t['radius']:.1f} m" if "radius" in t else ""
            return f"{km} (pay {t['least']:.3f}, {t['offset']:.2f} m ofsette{r})"
        more = f" ve {len(tight) - 6} aralık daha" if len(tight) > 6 else ""
        how = f"yaklaşıyor (pay {limit:.2f} altında)" if limit > 0 else "ulaşıyor ya da onu aşıyor"
        warnings.append({"code": "corridor_tight", "path": "margin",
                         "message": f"“{name_}” koridorunda kesit kurbun iç tarafında yarıçapa {how}: "
                                    f"{'; '.join(word(t) for t in tight[:6])}{more}."})
    if over:
        def oword(o):
            return (f"km {o['firstText'][0]} – {o['firstText'][1]} ile km {o['secondText'][0]} – {o['secondText'][1]} "
                    f"({o['area']:.1f} m², aradaki kot {o['least']:.2f}–{o['most']:.2f} m)")
        more = f" ve {len(over) - 6} yer daha" if len(over) > 6 else ""
        warnings.append({"code": "corridor_overlap", "path": "corridor",
                         "message": f"“{name_}” koridorunun üst yüzeyi kendi üstüne biniyor: "
                                    f"{'; '.join(oword(o) for o in over[:6])}{more}."})
    apart = [s for s in joins if not s["meets"]]
    if apart:
        def sword(s):
            parts = []
            if s["step"] > tol:
                parts.append(f"kotta {s['step']:.3f} m ({code_words(s.get('stepCode', ''))})")
            if s["shift"] > tol:
                parts.append(f"planda {s['shift']:.3f} m ({code_words(s.get('shiftCode', ''))})")
            if s.get("onlyBefore"):
                parts.append(f"yalnız öncekinde {', '.join(code_words(c) for c in s['onlyBefore'])}")
            if s.get("onlyAfter"):
                parts.append(f"yalnız sonrakinde {', '.join(code_words(c) for c in s['onlyAfter'])}")
            return f"km {s['km']} ({s['before'] + 1}. ve {s['after'] + 1}. bölge: {', '.join(parts)})"
        warnings.append({"code": "corridor_seam", "path": "regions",
                         "message": f"“{name_}” koridorunun bölgeleri buluşmuyor: {'; '.join(sword(s) for s in apart)}."})
    out = {"corridor": c["id"], "crosses": sum(1 for r in regs for x in r["crosses"] if x["section"] is not None),
           "strips": len(st), "triangles": len(tris)}
    if least is not None:
        out["least"], out["leastAt"] = least["margin"], least["station"]
    out.update({"tight": tight, "folds": folds, "overlaps": over, "seams": joins,
                "clear": not tight and not folds and not over and not apart})
    return out, warnings


def cases():
    out = []

    def ok(name, input_, corridors=ccmd.START, op="execute", templates=None):
        o, warns = answer(input_, corridors, templates)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        s = step(op, input_, result, {**UNCHANGED, "road": ccmd.road_of(corridors, templates=templates)})
        s["within"] = W
        same = corridors is ccmd.START and templates is None
        out.append(case(name, [s], None if same else ccmd.setup(corridors, templates=templates)))
        return o

    def no(name, input_, corridors=ccmd.START, setup_=None, result=None):
        if result is None:
            try:
                answer(input_, corridors)
            except ccmd.Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        road = None if setup_ is not None else ccmd.road_of(corridors)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    C1, C2 = ccmd.C1, ccmd.C2
    tight = {"id": "c-3", "name": "Dar kurp koridoru", "alignment": "g-4", "alignmentRevision": 1, "revision": 1,
             "profile": "b-4", "surface": "t-1", "left": 40.0, "right": 40.0, "tangent": 10.0, "curve": 2.0,
             "regions": [{"from": 0.0, "to": round(ccmd.G4_LENGTH - 1.0, 3), "template": "tk-1"}]}
    ok("Ana koridor: iki tip kesitin dikişi", {"corridor": "c-1"})
    ok("Ana koridor, yüzde 95 pay: kurpta dar", {"corridor": "c-1", "margin": 0.95})
    ok("Ana koridor, yüzde 50 pay: dar yer yok", {"corridor": "Ana koridor", "margin": 0.5})
    ok("Dikiş toleransı 10 m", {"corridor": "c-1", "tolerance": 10.0})
    ok("Aynı tip kesitin dikişi buluşur", {"corridor": "c-1"},
       [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-1"}, {"from": 300.0, "to": 600.0, "template": "tk-1"}]}, C2])
    ok("Boşluklu bölgelerde dikiş yok", {"corridor": "c-1"},
       [{**C1, "regions": [{"from": 0.0, "to": 280.0, "template": "tk-1"}, {"from": 320.0, "to": 600.0, "template": "tk-2"}]}, C2])
    ok("Ters tip kesitler", {"corridor": "c-1"},
       [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-2"}, {"from": 300.0, "to": 600.0, "template": "tk-1"}]}, C2])
    ok("Üç bölge", {"corridor": "c-1"},
       [{**C1, "regions": [{"from": 0.0, "to": 200.0, "template": "tk-1"}, {"from": 200.0, "to": 400.0, "template": "tk-2"},
                           {"from": 400.0, "to": 600.0, "template": "tk-1"}]}, C2])
    ok("Yan koridor: düz aks", {"corridor": "c-2"})
    # Docs/adr/0958: the first template's left cut slope benched; the seam names the benches only before it.
    benched = [{**scmd.TK1, "left": {**scmd.TK1["left"], "bench": {"height": 2.0, "width": 2.5, "fall": 5.0}}}, scmd.TK2]
    ok("Basamaklı yarma şevinin dikişi", {"corridor": "c-1"}, templates=benched)
    ok("Dar kurp: katlanma ve üst üste binme", {"corridor": "c-3"}, ccmd.START + [tight])
    ok("Dar kurp, yüzde 50 pay", {"corridor": "c-3", "margin": 0.5}, ccmd.START + [tight])
    ok("Dar kurp, dar şerit", {"corridor": "c-3"}, ccmd.START + [{**tight, "left": 3.0, "right": 3.0}])
    ok("Geniş şeritli ana koridor", {"corridor": "c-1", "margin": 0.6}, [{**C1, "left": 60.0, "right": 60.0}, C2])
    ok("Sık kesitler", {"corridor": "c-1", "margin": 0.95}, [{**C1, "tangent": 10.0, "curve": 5.0}, C2])
    ok("Yüzeyi olmayan koridor", {"corridor": "c-1"}, [{**C1, "surface": "t-9"}, C2])
    ok("Sıfır pay açıkça", {"corridor": "c-1", "margin": 0.0})
    ok("Planı", {"corridor": "c-1"}, op="plan")
    ok("Doğrulama", {"corridor": "c-3"}, ccmd.START + [tight], op="validate")
    no("Yol projesi değil", {"corridor": "c-1"}, setup_=ccmd.setup(work="map"), result=ccmd.NOT_ROAD)
    no("Olmayan koridor", {"corridor": "yok"})
    no("Pay 1", {"corridor": "c-1", "margin": 1.0})
    no("Eksi pay", {"corridor": "c-1", "margin": -0.1})
    no("Sıfır tolerans", {"corridor": "c-1", "tolerance": 0.0})
    no("Çok büyük tolerans", {"corridor": "c-1", "tolerance": 11.0})
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.corridors.preflight", "commandVersion": 1,
            "title": "Koridor ön denetimi",
            "note": "scripts/fixtures/road_corridor_preflight_command_cases.py yazar; koridorun kesitleri "
                    "road_corridor_command_cases.py'nin, eğrilik aksın mpmath referansından, örtüşmeler GEOS'un "
                    "(shapely); KentOS kodu yok.",
            "setup": ccmd.setup(), "cases": cases()}
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")
    for c in data["cases"]:
        r = c["steps"][0]["result"]
        o = r.get("output")
        if o:
            print(f"  {c['name'][:34]:34} en az {o.get('least', 0):.3f} dar {len(o['tight'])} kat {len(o['folds'])} "
                  f"üst {[(round(x['area'], 1), round(x['least'], 2)) for x in o['overlaps']]} "
                  f"dikiş {[(round(s['step'], 3), round(s['shift'], 3), s['meets']) for s in o['seams']]} "
                  f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:34]:34} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
