#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.profiles.sight.json: the shared cases of KentOS Yol's profile sight distance
(docs/adr/0921; ROAD-PROFILE-08), their expectations built here without KentOS code. The drawing and the profiles are
road_profile_command_cases.py's with two crest profiles added; a profile's elevation is road_vertical_cases.py's
(exact fractions, circles in mpmath). The sight: an eye `eye` metres over the profile at a station, an object `object`
metres over it a distance away; the object is hidden when the profile rises over the straight line between them. The
highest rise over a line is read on a 5 cm grid of the profile and refined by golden section on the exact elevation;
the first hidden distance is found by a 25 cm walk of the distance, refined by bisection to 1e-9 m. KentOS steps by the
user's step and bisects to a millimetre, so its distance is at most a millimetre short: the cases are compared within
2 mm.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_profile_sight_cases.py [--check]
"""

import json
import math
import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_profile_command_cases as cmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 2e-3
GRID = 0.05
WALK = 0.25
MOST = 100_000

CREST = {"id": "b-4", "name": "Tepe", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
         "pvis": [cmd.pv("a", 0, 100), cmd.pv("v1", 200, 108, length=160), cmd.pv("b", 400, 100)]}
ASYM = {"id": "b-5", "name": "Yatık tepe", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
        "pvis": [cmd.pv("a", 0, 100), cmd.pv("v1", 180, 106, lengthIn=60, lengthOut=100), cmd.pv("b", 400, 99)]}
PROFILES = [cmd.MAIN, cmd.SIDE, cmd.PART, CREST, ASYM]


class Profile:
    def __init__(self, d):
        self.d = d
        _, self.ev, S, _ = cmd.solve(d)
        self.start, self.end = float(S[0]), float(S[-1])
        n = int(round((self.end - self.start) / GRID))
        self.t = np.array([self.start + i * GRID for i in range(n)] + [self.end])
        self.z = np.array([self.ev(float(q))[0] for q in self.t])

    def zat(self, q):
        return self.ev(min(max(q, self.start), self.end))[0]

    def rise(self, s, ze, t1, zo):
        """The highest the profile rises over the line (s, ze)–(t1, zo) between them, and where."""
        lo, hi = min(s, t1), max(s, t1)
        m = (zo - ze) / (t1 - s)
        line = lambda q: ze + m * (q - s)  # noqa: E731
        best = (max(self.zat(lo) - line(lo), self.zat(hi) - line(hi)), lo)
        inside = (self.t > lo) & (self.t < hi)
        idx = np.nonzero(inside)[0]
        if len(idx):
            vals = self.z[idx] - (ze + m * (self.t[idx] - s))
            k = idx[int(np.argmax(vals))]
            a, b = max(lo, self.t[k] - GRID), min(hi, self.t[k] + GRID)
            g = (math.sqrt(5) - 1) / 2
            f = lambda q: self.zat(q) - line(q)  # noqa: E731
            c, e = b - g * (b - a), a + g * (b - a)
            fc, fe = f(c), f(e)
            for _ in range(60):
                if fc > fe:
                    b, e, fe = e, c, fc
                    c = b - g * (b - a)
                    fc = f(c)
                else:
                    a, c, fc = c, e, fe
                    e = a + g * (b - a)
                    fe = f(e)
            q = (a + b) / 2
            if f(q) > best[0]:
                best = (f(q), q)
        return best

    def sight(self, s, ahead, eye, obj, reach):
        room = self.end - s if ahead else s - self.start
        if room <= 0:
            return None
        most = min(reach, room)
        sign = 1 if ahead else -1
        ze = self.zat(s) + eye

        def hidden(d):
            t = s + sign * d
            r, at = self.rise(s, ze, t, self.zat(t) + obj)
            return r > 0, at

        d = 0.0
        while True:
            nd = min(d + WALK, most)
            h, _ = hidden(nd)
            if h:
                lo, hi = d, nd
                while hi - lo > 1e-9:
                    mid = (lo + hi) / 2
                    if hidden(mid)[0]:
                        hi = mid
                    else:
                        lo = mid
                return {"distance": lo, "until": "hidden", "blocked": hidden(hi)[1]}
            if nd >= most:
                return {"distance": most, "until": "end" if room <= reach else "reach"}
            d = nd


MADE = {d["id"]: Profile(d) for d in PROFILES}


def fmt(v):
    v = float(v)
    return str(int(v)) if v.is_integer() else repr(v)


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = cmd.failed(code, message, path)


def answer(inp):
    name = inp["profile"]
    d = next((p for p in PROFILES if p["id"] == name), None) or next(
        (p for p in PROFILES if p["name"].lower() == name.lower()), None)
    if d is None:
        raise Refused("unknown_road_profile", f"“{name}” kimlikli ya da adlı boykesit projede yok.", "profile")
    if not inp["eye"] > 0:
        raise Refused("invalid_sight", "Göz yüksekliği (eye) 0'dan büyük olmalı.", "eye")
    if not inp["object"] > 0:
        raise Refused("invalid_sight", "Nesne yüksekliği (object) 0'dan büyük olmalı: profilin üstündeki nokta her "
                                       "tepede çizgiye teğettir.", "object")
    for path, words in (("reach", "Bakılan uzaklık"), ("step", "Adım")):
        if not inp[path] > 0:
            raise Refused("invalid_sight", f"{words} ({path}) 0'dan büyük olmalı.", path)
    if "required" in inp and not inp["required"] > 0:
        raise Refused("invalid_sight", "Gereken görüş mesafesi (required) 0'dan büyük olmalı.", "required")
    p = MADE[d["id"]]
    frm, to = inp.get("from", p.start), inp.get("to", p.end)
    on = lambda v: p.start - 1e-9 <= v <= p.end + 1e-9  # noqa: E731
    if not (on(frm) and on(to) and frm < to):
        raise Refused("invalid_sight", f"Km aralığı boykesitin üstünde ve ileri olmalı: boykesit {p.start:.3f} ile "
                                       f"{p.end:.3f} m arası.", "to" if on(frm) else "from")
    step = inp["step"]
    if math.ceil((to - frm) / step) + 1 > MOST:
        raise Refused("invalid_sight", f"Bir soruda en çok {MOST} km olur; adımı büyütün.", "step")
    stations, k = [], 0
    while frm + k * step < to - 1e-9:
        stations.append(frm + k * step)
        k += 1
    stations.append(to)
    side = inp.get("side", "both")
    rows = []
    for s in stations:
        row = {"station": s, "elevation": p.zat(s)}
        for key, ahead in (("forward", True), ("backward", False)):
            if side in ("both", key):
                seen = p.sight(s, ahead, inp["eye"], inp["object"], inp["reach"])
                if seen is not None:
                    row[key] = seen
        rows.append(row)

    def least(key):
        best = None
        for r in rows:
            if key in r and r[key]["until"] == "hidden" and (best is None or r[key]["distance"] < best["distance"]):
                best = {"station": r["station"], "distance": r[key]["distance"]}
        return best

    short, warns = [], []
    if cmd.REV[d["alignment"]] != d.get("alignmentRevision", 0):
        warns.append(cmd.stale_warning(d))
    if "required" in inp:
        for key in ("forward", "backward"):
            run = None
            for r in rows:
                x = r.get(key)
                if x is not None and x["until"] == "hidden" and x["distance"] < inp["required"]:
                    if run is None:
                        run = {"side": key, "from": r["station"], "to": r["station"], "least": x["distance"]}
                        short.append(run)
                    else:
                        run["to"] = r["station"]
                        run["least"] = min(run["least"], x["distance"])
                else:
                    run = None
        if short:
            worst = min(short, key=lambda x: x["least"])
            warns.append({"code": "sight_short",
                          "message": f"“{d['name']}” boykesitinde {len(short)} aralıkta görüş {fmt(inp['required'])} "
                                     f"m'den kısa; en kısası {worst['least']:.1f} m ({worst['from']:.1f} – "
                                     f"{worst['to']:.1f} m arası).", "path": "required"})
    out = {"profile": d["id"], "eye": inp["eye"], "object": inp["object"], "reach": inp["reach"], "step": step,
           "resolution": 1e-3, "rows": rows}
    if least("forward"):
        out["leastForward"] = least("forward")
    if least("backward"):
        out["leastBackward"] = least("backward")
    out["short"] = short
    return out, warns


def cases():
    out = []
    unchanged = {**cmd.UNCHANGED, "road": cmd.road_of(PROFILES)}

    def q(name, inp, result=None, setup_=None):
        if result is None:
            try:
                o, w = answer(inp)
                result = {"status": "completed", "output": o, "warnings": w}
            except Refused as r:
                result = r.result
        expect = {**cmd.UNCHANGED, "road": cmd.road_of(None)} if setup_ else unchanged
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": expect,
                                      "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    base = {"eye": 1.08, "object": 0.6, "reach": 300.0}
    q("Daire tepeli boykesit, iki yön, gerekenle", {"profile": "b-1", **base, "step": 20.0, "from": 220.0, "to": 460.0,
                                                    "required": 150.0})
    q("Yalnız ileri", {"profile": "b-1", **base, "step": 20.0, "from": 220.0, "to": 460.0, "side": "forward"})
    q("Yalnız geri", {"profile": "b-1", **base, "step": 20.0, "from": 220.0, "to": 460.0, "side": "backward"})
    q("Alçak nesne", {"profile": "b-1", "eye": 1.08, "object": 0.15, "reach": 300.0, "step": 30.0,
                                "from": 250.0, "to": 430.0})
    q("Kamyon gözü", {"profile": "b-1", "eye": 2.4, "object": 0.6, "reach": 300.0, "step": 30.0, "from": 250.0,
                      "to": 430.0, "required": 200.0})
    q("Ad ile, bütün boykesit", {"profile": "Proje kotu", **base, "step": 60.0})
    q("Çukur boykesit, hep görünür", {"profile": "b-2", **base, "step": 50.0, "required": 100.0})
    q("Düz eğim, kısmi boykesit", {"profile": "b-3", **base, "step": 50.0})
    q("Kısa bakış, hep erim", {"profile": "b-1", "eye": 1.08, "object": 0.6, "reach": 40.0, "step": 20.0,
                               "from": 300.0, "to": 380.0})
    q("Parabol tepe", {"profile": "b-4", **base, "step": 20.0, "from": 60.0, "to": 340.0, "required": 140.0})
    q("Parabol tepe, sık adım", {"profile": "b-4", **base, "step": 5.0, "from": 150.0, "to": 250.0, "side": "forward"})
    q("Parabol tepe, alçak nesne", {"profile": "Tepe", "eye": 1.08, "object": 0.15, "reach": 250.0, "step": 25.0,
                                   "from": 100.0, "to": 300.0, "required": 120.0})
    q("Yatık tepe", {"profile": "b-5", **base, "step": 20.0, "from": 60.0, "to": 340.0, "required": 130.0})
    q("Yatık tepe, geri", {"profile": "b-5", **base, "step": 20.0, "from": 100.0, "to": 300.0, "side": "backward",
                           "required": 160.0})
    q("Gereken hep sağlanır", {"profile": "b-4", **base, "step": 40.0, "from": 40.0, "to": 360.0, "required": 20.0})
    q("Uçta tek yön", {"profile": "b-4", **base, "step": 50.0, "from": 350.0, "to": 400.0})
    # Refusals.
    q("Göz sıfır", {"profile": "b-1", "eye": 0.0, "object": 0.6, "reach": 300.0, "step": 20.0})
    q("Nesne eksi", {"profile": "b-1", "eye": 1.08, "object": -0.1, "reach": 300.0, "step": 20.0})
    q("Nesne yolun üstünde", {"profile": "b-1", "eye": 1.08, "object": 0.0, "reach": 300.0, "step": 20.0})
    q("Erim sıfır", {"profile": "b-1", "eye": 1.08, "object": 0.6, "reach": 0.0, "step": 20.0})
    q("Adım sıfır", {"profile": "b-1", "eye": 1.08, "object": 0.6, "reach": 300.0, "step": 0.0})
    q("Gereken sıfır", {"profile": "b-1", **base, "step": 20.0, "required": 0.0})
    q("Başı boykesitin dışında", {"profile": "b-3", **base, "step": 20.0, "from": 50.0})
    q("Geri giden aralık", {"profile": "b-1", **base, "step": 20.0, "from": 400.0, "to": 300.0})
    q("Çok km", {"profile": "b-1", **base, "step": 0.001})
    q("Olmayan boykesit", {"profile": "yok", **base, "step": 20.0})
    q("Yol projesi değil", {"profile": "b-1", **base, "step": 20.0},
      cmd.failed("not_a_road_project", "Boykesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil."),
      setup_=cmd.setup(work="map"))
    return out


def main():
    found = cases()
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.profiles.sight", "commandVersion": 1,
           "title": "Görüş mesafesi: boykesitte göz ve nesne yüksekliğiyle",
           "note": "scripts/fixtures/road_profile_sight_cases.py'den; boykesitin kotu road_vertical_cases.py'nin "
                   "kesirleriyle, çizgiye en çok yükselme 5 cm ızgarada ve altın oranla, ilk kapanan uzaklık 25 cm "
                   "yürüyüş ve ikiye bölmeyle; KentOS kodu olmadan. KentOS en çok 1 mm kısa bulur: 2 mm içinde.",
           "setup": cmd.setup(PROFILES), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.profiles.sight.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.profiles.sight.json")
    else:
        path.write_text(text)
    print(f"road.profiles.sight.json: {len(found)} durum")


if __name__ == "__main__":
    main()
