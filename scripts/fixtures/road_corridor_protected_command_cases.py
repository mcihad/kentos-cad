#!/usr/bin/env python3
"""The shared cases of `road.corridors.protected` (plan/road.md Y46, docs/adr/0952; ROAD-GRADE-06).

The corridor's top surface and its sections are road_corridor_command_cases.py's (the corridor's reference, no
KentOS code); each protected area is set against them by GEOS (shapely): the triangles' overlaps, each section's
footprint (slope end to slope end, in plan) cut by the area. The areas are the cases' own, placed by the main
corridor's sections: one the right slope runs into, one across the road, one with a hole the road passes through,
an L the corridor's corner clips, one far off.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj --with shapely \\
        python3 scripts/fixtures/road_corridor_protected_command_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

from shapely.geometry import LineString, Polygon

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_guardrail_cases as grc  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.corridors.protected.json"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
W = 1e-6


def cross(c, s):
    """The main corridor's cross nearest station s."""
    _, _, _, regs = ccmd.make(c)
    return min((x for r in regs for x in r["crosses"]), key=lambda x: abs(x["station"] - s))


def at(cr, o, along=0.0):
    """A place o right of the cross and `along` metres ahead of it, rounded to the centimetre."""
    p = cref.place(cr, o)
    t = cr["direction"]
    return [round(p[0] + math.cos(t) * along, 2), round(p[1] + math.sin(t) * along, 2)]


def zones():
    c = ccmd.C1
    a, b, d, e = cross(c, 150.0), cross(c, 450.0), cross(c, 250.0), cross(c, 550.0)
    rect = lambda cr, o0, o1, l0, l1: [at(cr, o0, l0), at(cr, o1, l0), at(cr, o1, l1), at(cr, o0, l1)]  # noqa: E731
    out = [
        ("korunan", rect(a, 4.0, 40.0, -20.0, 20.0), []),
        ("korunan", rect(b, -40.0, 40.0, -12.0, 12.0), []),
        ("korunan", rect(d, -40.0, 40.0, -15.0, 15.0), [rect(d, -6.0, 6.0, -8.0, 8.0)]),
        ("korunan", [at(e, -40.0, -20.0), at(e, -10.0, -20.0), at(e, -10.0, 5.0), at(e, 25.0, 5.0),
                     at(e, 25.0, 20.0), at(e, -40.0, 20.0)], []),
        ("diger", [[590.0, 380.0], [598.0, 380.0], [598.0, 390.0], [590.0, 390.0]], []),
    ]
    return out


ZONES = zones()


def setup(corridors=ccmd.START, work="road"):
    base = ccmd.setup(corridors, work=work)
    base["layers"] = base["layers"] + [grc.node("korunan", "Korunan alan"), grc.node("diger", "Diğer")]
    n = max(e["id"] for e in base["entities"])
    pts = lambda ring: [{"x": p[0], "y": p[1]} for p in ring]  # noqa: E731
    ents = []
    for k, (layer, outer, holes) in enumerate(ZONES):
        e = {"kind": "polygon", "id": n + k + 1, "layerId": layer, "attrs": {}, "pts": pts(outer)}
        if holes:
            e["holes"] = [{"pts": pts(h)} for h in holes]
        ents.append(e)
    base["entities"] = base["entities"] + ents
    base["uids"] = base["uids"] + [f"0192f5a0-7c3e-7d4a-9b1e-{910000000000 + k:012d}" for k in range(len(ents))]
    return base


FIRST_ID = max(e["id"] for e in ccmd.setup()["entities"]) + 1


def answer(input_, corridors=ccmd.START):
    name = input_["corridor"]
    c = next((x for x in corridors if x["id"] == name), None) or next(
        (x for x in corridors if x["name"].lower() == name.lower()), None)
    if c is None:
        raise ccmd.Refused("unknown_corridor", f"“{name}” kimlikli ya da adlı koridor projede yok.", "corridor")
    layers = input_["layers"]
    bad = lambda words, path: ccmd.Refused("invalid_protected_check",  # noqa: E731
                                           f"Korunan alan denetimi sorusu geçerli değil: {words}", path)
    if not 1 <= len(layers) <= 20:
        raise bad("korunan alanların katmanları 1 ile 20 arası olur.", "layers")
    known = {l["id"] for l in setup(corridors)["layers"]}
    for i, l in enumerate(layers):
        if l not in known:
            raise bad(f"“{l}” kimlikli katman çizimde yok.", f"layers[{i}]")
    report, warnings, tris, regs = ccmd.make(c)
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    flat = [Polygon([(p[0], p[1]) for p in t]) for t in tris]
    footprint = sum(abs(p.area) for p in flat)
    feet = []
    for r in regs:
        for x in r["crosses"]:
            sec = x["section"]
            if sec is None:
                continue
            l, rr = sec["left"]["points"][-1][0], sec["right"]["points"][-1][0]
            feet.append((x["station"], cref.place(x, l), cref.place(x, rr)))
    areas = []
    for k, (layer, outer, holes) in enumerate(ZONES):
        if layer not in layers:
            continue
        zone = Polygon(outer, holes)
        covered = sum(p.intersection(zone).area for p in flat)
        reached = sorted(((s, LineString([a, b]).intersection(zone).length) for s, a, b in feet), key=lambda x: x[0])
        reached = [x for x in reached if x[1] > 1e-9]
        longest = None
        for x in reached:
            if longest is None or x[1] > longest[1]:
                longest = x
        rings = [outer] + holes
        out = {"entity": FIRST_ID + k, "part": 0, "layer": layer, "size": zone.area, "covered": covered,
               "share": covered / zone.area, "sections": len(reached)}
        if reached:
            out.update({"from": reached[0][0], "to": reached[-1][0], "fromText": text(reached[0][0]),
                        "toText": text(reached[-1][0])})
        out["reach"] = longest[1] if longest else 0.0
        if longest:
            out["reachAt"] = longest[0]
        out["rings"] = [[[float(p[0]), float(p[1])] for p in ring] for ring in rings]
        areas.append(out)
    hit = [a for a in areas if a["covered"] > 1e-6 or a["sections"] > 0]
    if hit:
        def words(a):
            if "fromText" in a and a["fromText"] != a["toText"]:
                km = f"km {a['fromText']} – {a['toText']}"
            elif "fromText" in a:
                km = f"km {a['fromText']}"
            else:
                km = "kesitsiz"
            return f"{a['entity']} numaralı alan {a['covered']:.1f} m² ({km}, en uzun giriş {a['reach']:.2f} m)"
        more = f" ve {len(hit) - 8} alan daha" if len(hit) > 8 else ""
        warnings.append({"code": "corridor_protected", "path": "layers",
                         "message": f"“{c['name']}” koridorunun izi korunan alanlara giriyor: "
                                    f"{'; '.join(words(a) for a in hit[:8])}{more}."})
    return {"corridor": c["id"], "footprint": footprint, "areas": areas,
            "covered": sum(a["covered"] for a in areas)}, warnings


def cases():
    out = []

    def ok(name, input_, corridors=ccmd.START, op="execute"):
        o, warns = answer(input_, corridors)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        s = step(op, input_, result, {**UNCHANGED, "road": ccmd.road_of(corridors)})
        s["within"] = W
        out.append(case(name, [s], None if corridors is ccmd.START else setup(corridors)))
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
    o = ok("Ana koridor ve korunan alanlar", {"corridor": "c-1", "layers": ["korunan"]})
    assert all(a["covered"] > 0 for a in o["areas"][:4]), [a["covered"] for a in o["areas"]]
    ok("İki katman", {"corridor": "c-1", "layers": ["korunan", "diger"]})
    ok("Uzaktaki alan: giriş yok", {"corridor": "c-1", "layers": ["diger"]})
    ok("Adıyla yan koridor", {"corridor": "YAN koridor", "layers": ["korunan"]})
    ok("Dar şeritli koridor", {"corridor": "c-1", "layers": ["korunan"]}, [{**C1, "left": 10.0, "right": 10.0}, C2])
    ok("Geniş şeritli koridor", {"corridor": "c-1", "layers": ["korunan"]}, [{**C1, "left": 60.0, "right": 60.0}, C2])
    ok("Sık kesitler", {"corridor": "c-1", "layers": ["korunan"]}, [{**C1, "tangent": 10.0}, C2])
    ok("Bir bölge", {"corridor": "c-1", "layers": ["korunan"]},
       [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-1"}]}, C2])
    ok("Boşluklu bölgeler", {"corridor": "c-1", "layers": ["korunan"]},
       [{**C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 600.0, "template": "tk-2"}]}, C2])
    ok("Ters tip kesitler", {"corridor": "c-1", "layers": ["korunan"]},
       [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-2"}, {"from": 300.0, "to": 600.0, "template": "tk-1"}]}, C2])
    ok("Yüzeyi olmayan koridor", {"corridor": "c-1", "layers": ["korunan"]}, [{**C1, "surface": "t-9"}, C2])
    ok("Boş katman", {"corridor": "c-1", "layers": ["0"]})
    ok("Aynı katman iki kez", {"corridor": "c-1", "layers": ["korunan", "korunan"]})
    ok("Kıvrımda sık", {"corridor": "c-1", "layers": ["korunan"]}, [{**C1, "curve": 5.0}, C2])
    ok("Kısa bölge", {"corridor": "c-1", "layers": ["korunan"]},
       [{**C1, "regions": [{"from": 400.0, "to": 500.0, "template": "tk-2"}]}, C2])
    ok("Planı", {"corridor": "c-1", "layers": ["korunan"]}, op="plan")
    ok("Doğrulama", {"corridor": "c-1", "layers": ["korunan"]}, op="validate")
    no("Yol projesi değil", {"corridor": "c-1", "layers": ["korunan"]}, setup_=setup(work="map"), result=ccmd.NOT_ROAD)
    no("Olmayan koridor", {"corridor": "yok", "layers": ["korunan"]})
    no("Katmansız", {"corridor": "c-1", "layers": []})
    no("Olmayan katman", {"corridor": "c-1", "layers": ["korunan", "yok"]})
    no("Yirmi bir katman", {"corridor": "c-1", "layers": ["korunan"] * 21})
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.corridors.protected", "commandVersion": 1,
            "title": "Korunan alanlar",
            "note": "scripts/fixtures/road_corridor_protected_command_cases.py yazar; koridorun üst yüzeyi ve kesitleri "
                    "road_corridor_command_cases.py'nin, alanlarla karşılaştırma GEOS'un (shapely); KentOS kodu yok. "
                    "Alanlar durumlarındır, ana koridorun kesitlerinden yerleştirildi.",
            "setup": setup(), "cases": cases()}
    data = road_grades.fixture(data)
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
            print(f"  {c['name'][:30]:30} iz {o['footprint']:.0f} m², kaplanan {o['covered']:.1f} m² "
                  f"{[(a['entity'], round(a['covered'], 1), a['sections'], round(a['reach'], 2)) for a in o['areas']]} "
                  f"{[w['code'] for w in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:30]:30} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
