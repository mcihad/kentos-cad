#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.corridors.water.json: the shared cases of KentOS Yol's platform water
(plan/road.md Y22, docs/adr/0923; ROAD-DEVER-09), their expectations built here without KentOS code. The drawing and
its corridors are road_corridor_command_cases.py's; a corridor's feature lines, top surface and its triangles' roles
road_corridor_cases.py's. A triangle's slope is its plane's steepest, the cross product's horizontal over its vertical;
its stations are its corners' (each corner a feature point); the crossfall the axis to the side's c1 over their plan
distance; a low point a platform edge's point lower than both neighbours. The least slopes are the cases' own.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_platform_water_cases.py [--check]
"""

import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
EDGE = re.compile(r"^(left|right)\.c\d+$")
MADE = {}


def built(c):
    if c["id"] not in MADE:
        _, warnings, _, regs = ccmd.make(c, True)
        MADE[c["id"]] = (cref.assemble(regs), warnings)
    return MADE[c["id"]]


def slope(t):
    u = [t[1][k] - t[0][k] for k in range(3)]
    v = [t[2][k] - t[0][k] for k in range(3)]
    n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
    return math.hypot(n[0], n[1]) / abs(n[2])


def plan_area(t):
    return 0.5 * abs((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1]))


def answer(inp):
    name = inp["corridor"]
    c = next((x for x in ccmd.START if x["id"] == name), None) or next(
        (x for x in ccmd.START if x["name"].lower() == name.lower()), None)
    if c is None:
        raise ccmd.Refused("unknown_corridor", f"“{name}” kimlikli ya da adlı koridor projede yok.", "corridor")
    least = inp["least"]
    if not least > 0:
        raise ccmd.Refused("invalid_platform_water", "En küçük eğim (least) 0'dan büyük bir oran olmalı.", "least")
    b, warnings = built(c)
    where = {}
    for f in b["features"]:
        for p in f["points"]:
            where[(p[1], p[2])] = (f["region"], p[0])
    strips = {}
    count = 0
    for t, role in zip(b["triangles"], b["roles"]):
        at = [where[(p[0], p[1])] for p in t]
        key = (at[0][0], min(a[1] for a in at), max(a[1] for a in at))
        strips.setdefault(key, None)
        if role in ("lane", "shoulder"):
            s = slope(t)
            if s < least:
                count += 1
                old = strips[key]
                strips[key] = (plan_area(t) + (old[0] if old else 0.0), min(s, old[1] if old else math.inf))
    flats = []
    for (region, frm, to), got in sorted(strips.items()):
        if got is None:
            continue
        area, low = got
        if flats and flats[-1]["region"] == region and flats[-1]["to"] == frm:
            flats[-1]["to"] = to
            flats[-1]["area"] += area
            flats[-1]["least"] = min(flats[-1]["least"], low)
        else:
            flats.append({"region": region, "from": frm, "to": to, "area": area, "least": low})
    zeros = []
    regions = max((f["region"] for f in b["features"]), default=-1) + 1
    for region in range(regions):
        axis = {p[0]: p for f in b["features"] if f["region"] == region and f["code"] == "axis" for p in f["points"]}
        for right in (False, True):
            code = "right.c1" if right else "left.c1"
            cf = []
            for f in b["features"]:
                if f["region"] == region and f["code"] == code:
                    for p in f["points"]:
                        a = axis.get(p[0])
                        if a is None:
                            continue
                        run = math.hypot(p[1] - a[1], p[2] - a[2])
                        if run > 0:
                            cf.append((p[0], (p[3] - a[3]) / run))
            cf.sort()
            for k, (s, x) in enumerate(cf):
                if x == 0:
                    zeros.append({"region": region, "right": right, "station": s})
                if k + 1 < len(cf) and x * cf[k + 1][1] < 0:
                    s1, x1 = cf[k + 1]
                    zeros.append({"region": region, "right": right, "station": s + (s1 - s) * x / (x - x1)})
    zeros.sort(key=lambda z: (z["region"], z["right"], z["station"]))
    lows = []
    for f in b["features"]:
        if f["code"] != "axis" and not EDGE.match(f["code"]):
            continue
        p = f["points"]
        for i in range(1, len(p) - 1):
            if p[i][3] < p[i - 1][3] and p[i][3] < p[i + 1][3]:
                lows.append({"code": f["code"], "region": f["region"], "station": p[i][0], "x": p[i][1], "y": p[i][2],
                             "z": p[i][3]})
    lows.sort(key=lambda x: (x["region"], x["station"], x["code"]))
    flat_area = sum(f["area"] for f in flats)
    warns = list(warnings)
    if flats:
        warns.append({"code": "corridor_flat",
                      "message": f"“{c['name']}” koridorunun platformunda {len(flats)} aralıkta {flat_area:.2f} m² yer "
                                 f"%{100 * least:.2f}'den yatık; su orada durur.", "path": "least"})
    return {"corridor": c["id"], "least": least, "flatTriangles": count, "flatArea": flat_area, "flats": flats,
            "zeros": zeros, "lows": lows}, warns


def cases():
    out = []
    unchanged = {**ccmd.UNCHANGED, "road": ccmd.road_of(ccmd.START)}

    def q(name, inp, result=None, setup_=None):
        if result is None:
            try:
                o, w = answer(inp)
                result = {"status": "completed", "output": o, "warnings": w}
            except ccmd.Refused as r:
                result = r.result
        expect = {**ccmd.UNCHANGED, "road": None} if setup_ else unchanged
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": expect, "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    for least in (0.001, 0.003, 0.005, 0.008, 0.01, 0.015, 0.02, 0.025, 0.03, 0.05):
        q(f"Dever geçişli koridor, %{100 * least:g}", {"corridor": "c-2", "least": least})
    for least in (0.005, 0.015, 0.02, 0.03, 0.05):
        q(f"Devrsiz koridor, %{100 * least:g}", {"corridor": "c-1", "least": least})
    q("Ad ile", {"corridor": "Yan koridor", "least": 0.01})
    q("Ad küçük harfle", {"corridor": "ana koridor", "least": 0.03})
    # Refusals.
    q("Eğim sıfır", {"corridor": "c-2", "least": 0.0})
    q("Eğim eksi", {"corridor": "c-2", "least": -0.01})
    q("Olmayan koridor", {"corridor": "yok", "least": 0.01})
    q("Yol projesi değil", {"corridor": "c-2", "least": 0.01}, ccmd.NOT_ROAD, setup_=ccmd.setup(work="map"))
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.corridors.water", "commandVersion": 1,
           "title": "Platform suyu: yatık yerler, sıfır enine eğim ve alçak noktalar",
           "note": "scripts/fixtures/road_platform_water_cases.py'den; koridor road_corridor_cases.py'nin, üçgenin eğimi "
                   "düzleminin en dik eğimi, enine eğim eksenle c1 arasında, alçak nokta iki komşusundan alçak kenar "
                   "noktası; KentOS kodu olmadan.",
           "setup": ccmd.setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.corridors.water.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.corridors.water.json")
    else:
        path.write_text(text)
    print(f"road.corridors.water.json: {len(found)} durum")


if __name__ == "__main__":
    main()
