#!/usr/bin/env python3
"""A road's superelevation's shared cases (docs/adr/0904, plan/road.md Y6.1), from an independent reference: the
alignment and its curves' stations are scripts/fixtures/road_alignment_cases.py's (mpmath at 40 digits); each curve's
transition rotates about the centreline in exact fractions of those stations: the full rate's station TS + (1 − p)·L,
the outside max(n_out, g), the inside min(n_in, −g), g(s) = e − (FS − s)·e/L, mirrored from ST; the critical stations
where g meets the normal slopes and 0; the widening ramping over the runoff. No KentOS code.

    uv run --no-project --python 3.11 --with mpmath python3 scripts/fixtures/road_superelevation_cases.py
    ... --check   # fails when fixtures/road/v1/superelevation.json is not what this writes
"""

import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as al  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "road" / "v1" / "superelevation.json"
P = al.pi


def rule(pi, rate, runoff, on_tangent, widening=0.0):
    return {"pi": pi, "rate": rate, "runoff": runoff, "onTangent": on_tangent, "widening": widening}


def along(points, s):
    if not points:
        return None
    if s <= points[0][0]:
        return points[0][1]
    if s >= points[-1][0]:
        return points[-1][1]
    for a, b in zip(points, points[1:]):
        if a[0] <= s <= b[0]:
            return b[1] if a[0] == b[0] else a[1] + (b[1] - a[1]) * (s - a[0]) / (b[0] - a[0])


def end(r, n_out, n_in, full, toward, exit_):
    e, l = F(r["rate"]), F(r["runoff"])
    at = lambda v: full - toward * (e - v) * l / e  # noqa: E731
    nc, rc = at(n_out), at(-n_in)
    out, ins = [[nc, n_out], [full, e]], [[rc, n_in], [full, -e]]
    wide = [[full - toward * l, F(0)], [full, F(r["widening"])]]
    if toward < 0:
        out.reverse(), ins.reverse(), wide.reverse()
    crit = [("normalCrown", nc)]
    if n_out < 0:
        crit.append(("levelCrown", at(F(0))))
    if n_in < 0 and -n_in > n_out:
        crit.append(("reverseCrown", rc))
    crit.append(("full", full))
    if exit_:
        crit.reverse()
    return out, ins, wide, [{"pi": r["pi"], "mark": m, "station": s, "exit": exit_} for m, s in crit]


def curves(pis, normal, rules):
    els, length, reports, _ = al.build(pis)
    length = F(float(length))
    nl, nr = F(normal[0]), F(normal[1])
    placed, seen = [], set()
    for r in rules:
        if r["pi"] in seen:
            return {"error": {"kind": "Twice", "pi": r["pi"]}}
        seen.add(r["pi"])
        c = next((c for c in reports if c["pi"] == r["pi"]), None)
        if c is None:
            return {"error": {"kind": "NoCurve", "pi": r["pi"]}}
        if not (r["rate"] > 0 and r["runoff"] > 0 and 0 <= r["onTangent"] <= 1 and r["widening"] >= 0):
            return {"error": {"kind": "Bad", "pi": r["pi"]}}
        if r["rate"] < abs(normal[0]) or r["rate"] < abs(normal[1]):
            return {"error": {"kind": "RateBelowNormal", "pi": r["pi"]}}
        lead = (1 - F(r["onTangent"])) * F(r["runoff"])
        fs_in, fs_out = F(c["stations"][0]) + lead, F(c["stations"][3]) - lead
        if fs_in > fs_out:
            return {"error": {"kind": "TooShort", "pi": r["pi"]}}
        placed.append((fs_in, fs_out, r, c))
    placed.sort(key=lambda t: t[0])
    sch = {"left": [], "right": [], "leftWidening": [], "rightWidening": []}
    crit, last = [], None
    for fs_in, fs_out, r, c in placed:
        left_out = c["delta"] < 0
        n_out, n_in = (nl, nr) if left_out else (nr, nl)
        a = end(r, n_out, n_in, fs_in, 1, False)
        b = end(r, n_out, n_in, fs_out, -1, True)
        first = min(a[0][0][0], a[1][0][0], a[2][0][0])
        end_s = max(b[0][-1][0], b[1][-1][0], b[2][-1][0])
        if first < 0 or end_s > length:
            return {"error": {"kind": "OffAlignment", "pi": r["pi"]}}
        if last and first < last[0]:
            return {"error": {"kind": "Overlap", "a": last[1], "b": r["pi"]}}
        last = (end_s, r["pi"])
        o, i = ("left", "right") if left_out else ("right", "left")
        sch[o] += a[0] + b[0]
        sch[i] += a[1] + b[1]
        if r["widening"] > 0:
            sch[i + "Widening"] += a[2] + b[2]
        crit += a[3] + b[3]
    for k in ("left", "right"):
        sch[k].sort(key=lambda p: p[0])
    return {"schedule": sch, "critical": crit, "length": length}


def fl(v):
    return [[float(a), float(b)] for a, b in v]


def make(name, pis, normal, rules=None, rows=None, at=()):
    out = {"name": name, "pis": pis, "normal": normal}
    if rows is not None:
        out["rows"] = rows
        bad = next((i for i, r in enumerate(rows) if i and r["station"] <= rows[i - 1]["station"]), None)
        if bad is not None:
            out["error"] = {"kind": "BadRow", "row": bad}
            return out
        sch = {"left": [[F(r["station"]), F(r["left"])] for r in rows], "right": [[F(r["station"]), F(r["right"])] for r in rows],
               "leftWidening": [], "rightWidening": []}
        crit = []
    else:
        out["rules"] = rules
        got = curves(pis, normal, rules)
        if "error" in got:
            out["error"] = got["error"]
            return out
        sch, crit = got["schedule"], got["critical"]
    out["schedule"] = {k: fl(v) for k, v in sch.items()}
    out["critical"] = [{**c, "station": float(c["station"])} for c in crit]
    rows_at = []
    for s in at:
        s = F(s)
        left = along(sch["left"], s)
        right = along(sch["right"], s)
        rows_at.append({"station": float(s), "left": float(left if left is not None else F(normal[0])),
                        "right": float(right if right is not None else F(normal[1])),
                        "leftWidening": float(along(sch["leftWidening"], s) or 0),
                        "rightWidening": float(along(sch["rightWidening"], s) or 0)})
    out["at"] = rows_at
    return out


RIGHT = [P("a", 0, 0), P("p1", 400, 0, 300, 60, 60), P("b", 800, -300)]
LEFT = [P("a", 0, 0), P("p1", 400, 0, 300, 60, 60), P("b", 800, 300)]
ROUND = [P("a", 0, 0), P("p1", 400, 0, 350), P("b", 800, -250)]
TWO = [P("a", 0, 0), P("p1", 400, 0, 250, 50, 50), P("p2", 900, 350, 250, 50, 50), P("b", 1400, 350)]
REVERSE = [P("a", 0, 0), P("p1", 300, 0, 200, 40, 40), P("p2", 520, 120, 200, 40, 40), P("b", 800, 120)]
SAMPLES = [0, 150, 300, 330, 345, 360, 400, 450, 500, 600, 700]
CASES = [
    make("Sağa dönen kurp, klotoidli, geçiş klotoidde", RIGHT, [-2.5, -2.5], [rule("p1", 6.0, 60.0, 0.0)], at=SAMPLES),
    make("Sola dönen kurp: sağ yan dışta", LEFT, [-2.5, -2.5], [rule("p1", 6.0, 60.0, 0.0)], at=SAMPLES),
    make("Klotoidsiz kurp, geçişin 0,7'si teğette", ROUND, [-2.5, -2.5], [rule("p1", 7.0, 50.0, 0.7)], at=SAMPLES),
    make("İki kurp, aralarında teğet", TWO, [-2.0, -2.0],
         [rule("p2", 5.0, 50.0, 0.0), rule("p1", 5.0, 50.0, 0.0)], at=[0, 300, 330, 600, 700, 900, 1000, 1200]),
    make("Genişleme iç yanda", RIGHT, [-2.5, -2.5], [rule("p1", 6.0, 60.0, 0.0, 0.8)], at=SAMPLES),
    make("Tek yöne eğimli normal: dış yan zaten yükselir", RIGHT, [2.0, -2.0], [rule("p1", 4.0, 40.0, 0.5)], at=SAMPLES),
    make("Normal eğimle aynı oran", ROUND, [-3.0, -3.0], [rule("p1", 3.0, 30.0, 0.6)], at=SAMPLES),
    make("Kurbu olmayan PI", RIGHT, [-2.5, -2.5], [rule("b", 6.0, 60.0, 0.0)]),
    make("Sıfır oran", RIGHT, [-2.5, -2.5], [rule("p1", 0.0, 60.0, 0.0)]),
    make("Teğet payı 1'den büyük", RIGHT, [-2.5, -2.5], [rule("p1", 6.0, 60.0, 1.2)]),
    make("Oran normal eğimden küçük", RIGHT, [-2.5, -2.5], [rule("p1", 2.0, 60.0, 0.0)]),
    make("Tam dever kurba sığmıyor", ROUND, [-2.5, -2.5], [rule("p1", 6.0, 400.0, 0.0)]),
    make("Ters kurplarda geçişler biniyor", REVERSE, [-2.5, -2.5], [rule("p1", 6.0, 80.0, 1.0), rule("p2", 6.0, 80.0, 1.0)]),
    make("Geçiş aksın başından taşıyor", RIGHT, [-2.5, -2.5], [rule("p1", 6.0, 300.0, 1.0)]),
    make("Aynı PI iki kez", RIGHT, [-2.5, -2.5], [rule("p1", 6.0, 60.0, 0.0), rule("p1", 5.0, 60.0, 0.0)]),
    make("Yazılan km satırları", RIGHT, [-2.5, -2.5], rows=[
        {"station": 200.0, "left": -2.5, "right": -2.5}, {"station": 260.0, "left": 0.0, "right": -2.5},
        {"station": 300.0, "left": 6.0, "right": -6.0}, {"station": 500.0, "left": 6.0, "right": -6.0},
        {"station": 560.0, "left": -2.5, "right": -2.5}], at=SAMPLES),
    make("Km satırları artmıyor", RIGHT, [-2.5, -2.5], rows=[
        {"station": 200.0, "left": -2.5, "right": -2.5}, {"station": 200.0, "left": 0.0, "right": -2.5}]),
]


def main():
    text = json.dumps({"format": "kentos.road-superelevation-cases", "version": 1, "cases": CASES},
                      ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"güncel değil: {OUT}")
            sys.exit(1)
        print("güncel")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT} ({len(CASES)} durum)")


if __name__ == "__main__":
    main()
