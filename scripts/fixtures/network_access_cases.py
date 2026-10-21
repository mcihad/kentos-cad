#!/usr/bin/env python3
"""Independent reference of vehicle profiles and dated closures in network analysis (docs/adr/0567), written from the
ADR without KentOS code.

One scene: a street grid of straight edges meeting at their ends, each with a weight and a height limit, a kind, a
speed, a one-way field and a closure's first and last day; one edge closed by the closed edges' expression, a few with
values that cannot be read. Its definition has four profiles (Kamyon: weight and height limits, the footpaths
forbidden, 60 km/h; Yaya: the motorway forbidden, 5 km/h, both ways; Otobüs: a weight limit only; Ambulans: 120 km/h,
both ways) and the closures. The questions: routes between two corners by length and by time for a profile and a day,
and what the access closed. Written to fixtures/network/v1/access.json; the geometry core
(crates/shared/geometry-core/tests/all/network_access.rs) must give the same: what was closed and unread exactly, the
costs within 1e-9 (of their size above 1).

The reference is a plain Dijkstra over the corners with every edge's way and cost found from the ADR's rules: an edge
is closed to a profile where a limit field reads a number below its value (an empty field no limit; the first closing
reason ends the reading, so a value after it is not read), where a forbidding field holds one of its values (Turkish
case aside); closed on a day `d` where its first day `a` and last day `b` give `a ≤ d ≤ b` (an empty one open-ended,
both empty never). A top speed caps the speed cost's speed; both ways travels a one-way edge against its way (a closed
one stays closed). Every case's cheapest way is checked to be the only one, so the core has no tie to break otherwise.

    python3 scripts/fixtures/network_access_cases.py [--check]
"""

import argparse
import datetime
import heapq
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/access.json"
X0, Y0 = 500000.0, 4400000.0
STEP = 100.0
INF = math.inf

DEFINITION = {
    "id": "ag-1",
    "name": "Yollar",
    "kind": "road",
    "edges": [{"layer": "yol"}],
    "connect": "ends",
    "tolerance": 0.01,
    "direction": {"kind": "field", "field": "yon", "forward": ["FT"], "backward": ["TF"], "closed": ["N"]},
    "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0}],
    "profiles": [
        {"name": "Kamyon", "limits": [{"field": "agirlik", "value": 18.0}, {"field": "yukseklik", "value": 4.0}],
         "forbidden": [{"field": "tur", "values": ["yaya", "Bisiklet"]}], "topSpeed": 60.0},
        {"name": "Yaya", "forbidden": [{"field": "tur", "values": ["otoyol"]}], "topSpeed": 5.0, "bothWays": True},
        {"name": "Otobüs", "limits": [{"field": "agirlik", "value": 12.0}]},
        {"name": "Ambulans", "topSpeed": 120.0, "bothWays": True},
    ],
    "closures": {"from": "kapanis_bas", "to": "kapanis_son"},
}


def corner(i, j):
    """A corner of the grid, moved by a few metres (a fixed sequence) so that no two ways are of one length."""
    k = i * 4 + j
    return (X0 + i * STEP + ((k * k * 7 + k * 13) % 29) * 0.37, Y0 + j * STEP + ((k * k * 11 + k * 5) % 31) * 0.29)


# The grid's corners are (i, j), i 0–4 east, j 0–3 north. Each edge: its two corners (drawn from the first) and its
# attributes; an absent attribute is an empty field.
EDGES = [
    # The southern street: a motorway, fast, heavy-rated.
    ((0, 0), (1, 0), {"tur": "otoyol", "hiz": "110", "agirlik": "40", "yukseklik": "5"}),
    ((1, 0), (2, 0), {"tur": "otoyol", "hiz": "110", "agirlik": "40", "yukseklik": "5"}),
    ((2, 0), (3, 0), {"tur": "otoyol", "hiz": "110", "agirlik": "40", "yukseklik": "5",
                      "kapanis_bas": "2026-11-01", "kapanis_son": "2026-11-05"}),
    ((3, 0), (4, 0), {"tur": "otoyol", "hiz": "110", "agirlik": "40", "yukseklik": "5"}),
    # The middle street: one-way east, a low bridge, a light bridge.
    ((0, 1), (1, 1), {"tur": "cadde", "hiz": "50", "yon": "FT"}),
    ((1, 1), (2, 1), {"tur": "cadde", "hiz": "50", "yon": "FT", "yukseklik": "3.5"}),
    ((2, 1), (3, 1), {"tur": "cadde", "hiz": "50", "yon": "FT", "agirlik": "10"}),
    ((3, 1), (4, 1), {"tur": "cadde", "hiz": "50", "yon": "FT"}),
    # The northern street: a footpath at both ends, a street closed by the field, a street closed from a day on.
    ((0, 2), (1, 2), {"tur": "YAYA", "hiz": "5"}),
    ((1, 2), (2, 2), {"tur": "sokak", "hiz": "30", "yon": "N"}),
    ((2, 2), (3, 2), {"tur": "sokak", "hiz": "30", "kapanis_bas": "2026-12-01"}),
    ((3, 2), (4, 2), {"tur": "bisiklet", "hiz": "15"}),
    # The top street: a street closed until a day, an unreadable weight, an unreadable day, a one-way west.
    ((0, 3), (1, 3), {"tur": "sokak", "hiz": "30", "kapanis_son": "2026-10-15"}),
    ((1, 3), (2, 3), {"tur": "sokak", "hiz": "30", "agirlik": "ağır"}),
    ((2, 3), (3, 3), {"tur": "sokak", "hiz": "30", "kapanis_bas": "yarın", "kapanis_son": "2026-11-30"}),
    ((3, 3), (4, 3), {"tur": "sokak", "hiz": "30", "yon": "TF"}),
    # The avenues north: the western one-way north, a closed one by the expression, a slow one, a heavy-limit one.
    ((0, 0), (0, 1), {"tur": "cadde", "hiz": "50", "yon": "FT"}),
    ((0, 1), (0, 2), {"tur": "cadde", "hiz": "50"}),
    ((0, 2), (0, 3), {"tur": "cadde", "hiz": "50"}),
    ((1, 0), (1, 1), {"tur": "cadde", "hiz": "40"}),
    ((1, 1), (1, 2), {"tur": "cadde", "hiz": "40", "agirlik": "16"}),
    ((1, 2), (1, 3), {"tur": "cadde", "hiz": "40"}),
    ((2, 0), (2, 1), {"tur": "cadde", "hiz": "45"}),
    ((2, 1), (2, 2), {"tur": "cadde", "hiz": "45"}, True),
    ((2, 2), (2, 3), {"tur": "cadde", "hiz": "45", "yukseklik": "3.8"}),
    ((3, 0), (3, 1), {"tur": "cadde", "hiz": "35"}),
    ((3, 1), (3, 2), {"tur": "cadde", "hiz": "35", "agirlik": "20", "yukseklik": "x"}),
    ((3, 2), (3, 3), {"tur": "cadde", "hiz": "35"}),
    ((4, 0), (4, 1), {"tur": "cadde", "hiz": "50", "yon": "TF"}),
    ((4, 1), (4, 2), {"tur": "cadde", "hiz": "50", "kapanis_bas": "2026-11-03", "kapanis_son": "2026-11-03"}),
    ((4, 2), (4, 3), {"tur": "cadde", "hiz": "50"}),
]


def fold(s):
    """Trimmed, case folded with Turkish I and İ."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower()) for c in s.strip())


def number(t):
    """A value as the statistics rule reads a number: an optional sign, digits, a point or a comma, an exponent."""
    t = t.strip().replace(",", ".")
    try:
        v = float(t)
    except ValueError:
        return None
    return v if math.isfinite(v) else None


def day_of(t):
    t = t.strip()
    if len(t) < 10 or t[4] != "-" or t[7] != "-" or (len(t) > 10 and t[10] not in "T "):
        return None
    try:
        return (datetime.date(int(t[0:4]), int(t[5:7]), int(t[8:10])) - datetime.date(1970, 1, 1)).days
    except ValueError:
        return None


def profile_of(name):
    return next((p for p in DEFINITION["profiles"] if fold(p["name"]) == fold(name)), None)


def closing(attrs, profile, day):
    """Why an edge is closed (`profile`, `date` or none) and whether a value read could not be read."""
    unread = False

    def fact(f):
        v = attrs.get(f, "").strip()
        return v or None

    if profile is not None:
        for lim in profile.get("limits", []):
            v = fact(lim["field"])
            if v is None:
                continue
            n = number(v)
            if n is None:
                unread = True
            elif n < lim["value"]:
                return "profile", unread
        for fb in profile.get("forbidden", []):
            v = fact(fb["field"])
            if v is not None and fold(v) in [fold(x) for x in fb["values"]]:
                return "profile", unread
    if day is not None:
        bounds = []
        ok = True
        for f in (DEFINITION["closures"]["from"], DEFINITION["closures"]["to"]):
            v = fact(f)
            if v is None:
                bounds.append(None)
                continue
            d = day_of(v)
            if d is None:
                unread = True
                ok = False
            bounds.append(d)
        a, b = bounds
        if ok and (a is not None or b is not None) and (a is None or a <= day) and (b is None or day <= b):
            return "date", unread
    return None, unread


def ways(e, profile, day):
    """An edge's way and costs: (forward allowed, backward allowed, length, minutes) or None when closed."""
    a, b, attrs = e[0], e[1], e[2]
    expr_closed = len(e) > 3 and e[3]
    why, _ = closing(attrs, profile, day)
    if why is not None or expr_closed:
        return None
    yon = fold(attrs.get("yon", "")) if attrs.get("yon") else None
    if yon == "ft":
        fw, bw = True, False
    elif yon == "tf":
        fw, bw = False, True
    elif yon == "n":
        return None
    else:
        fw, bw = True, True
    if profile is not None and profile.get("bothWays"):
        fw, bw = True, True
    (ax, ay), (bx, by) = corner(*a), corner(*b)
    length = math.hypot(bx - ax, by - ay)
    speed = number(attrs["hiz"]) if attrs.get("hiz") and number(attrs["hiz"]) and number(attrs["hiz"]) > 0 else 50.0
    if profile is not None and profile.get("topSpeed") is not None:
        speed = min(speed, profile["topSpeed"])
    minutes = length * 60.0 / (speed * 1000.0)
    return fw, bw, length, minutes


def route(a, b, cost, profile, day):
    """The cheapest way from corner a to b with cost 0 (length) or 1 (minutes): (cost, totals) or None; and whether
    it is the only cheapest."""
    adj = {}
    for e in EDGES:
        w = ways(e, profile, day)
        if w is None:
            continue
        fw, bw, length, minutes = w
        c = (length, minutes)
        if fw:
            adj.setdefault(e[0], []).append((e[1], c))
        if bw:
            adj.setdefault(e[1], []).append((e[0], c))
    best = {a: (0.0, (0.0, 0.0))}
    count = {a: 1}
    heap = [(0.0, a)]
    done = set()
    while heap:
        d, n = heapq.heappop(heap)
        if n in done:
            continue
        done.add(n)
        for m, c in adj.get(n, []):
            nd = d + c[cost]
            tot = (best[n][1][0] + c[0], best[n][1][1] + c[1])
            if m not in best or nd < best[m][0] - 1e-12:
                best[m] = (nd, tot)
                count[m] = count[n]
                heapq.heappush(heap, (nd, m))
            elif abs(nd - best[m][0]) <= 1e-12:
                count[m] += count[n]
    if b not in best:
        return None, True
    return best[b], count[b] == 1


QUESTIONS = [
    # (name, profile, day, from, to, cost)
    ("serbest-uzunluk", None, None, (0, 0), (4, 3), 0),
    ("serbest-sure", None, None, (0, 0), (4, 3), 1),
    ("kamyon-sure", "Kamyon", None, (0, 0), (4, 3), 1),
    ("kamyon-uzunluk", "kamyon", None, (0, 1), (4, 1), 0),
    ("kamyon-batiya", "KAMYON", None, (4, 1), (0, 1), 0),
    ("yaya-uzunluk", "Yaya", None, (4, 1), (0, 1), 0),
    ("yaya-sure", "yaya", None, (0, 0), (4, 3), 1),
    ("otobus", "Otobüs", None, (0, 1), (4, 1), 0),
    ("otobus-sure", "OTOBÜS", None, (0, 3), (4, 0), 1),
    ("ambulans", "Ambulans", None, (4, 3), (0, 0), 1),
    ("ambulans-tek-yon", "ambulans", None, (4, 1), (0, 1), 0),
    ("gun-kapali", None, "2026-11-03", (0, 0), (4, 0), 0),
    ("gun-acik", None, "2026-11-06", (0, 0), (4, 0), 0),
    ("gun-ilk-gun", None, "2026-11-01", (2, 0), (3, 0), 0),
    ("gun-son-gun", None, "2026-11-05", (2, 0), (3, 0), 0),
    ("gun-bir-gun", None, "2026-11-03", (4, 0), (4, 3), 0),
    ("gun-ertesi", None, "2026-11-04", (4, 0), (4, 3), 0),
    ("gun-suresiz", None, "2027-03-01", (2, 2), (3, 2), 0),
    ("gun-oncesi", None, "2026-11-30", (2, 2), (3, 2), 0),
    ("gun-her-zamandan", None, "2026-10-15", (0, 3), (1, 3), 0),
    ("gun-sonrasi", None, "2026-10-16", (0, 3), (1, 3), 0),
    ("gun-saat", None, "2026-11-03T08:30", (0, 0), (4, 0), 0),
    ("kamyon-gun", "Kamyon", "2026-11-03", (0, 0), (4, 3), 1),
    ("yaya-gun", "Yaya", "2026-11-02", (0, 2), (4, 2), 0),
    ("otobus-gun", "Otobüs", "2026-12-02", (0, 3), (4, 3), 0),
    ("ambulans-gun", "Ambulans", "2026-11-03", (0, 0), (4, 3), 1),
]


def unread_ids(profile, day):
    out = []
    for i, e in enumerate(EDGES):
        _, u = closing(e[2], profile, day)
        if u:
            out.append(i + 1)
    return out


def counts(profile, day):
    by = {"profile": 0, "date": 0}
    for e in EDGES:
        why, _ = closing(e[2], profile, day)
        if why:
            by[why] += 1
    return by["profile"], by["date"]


def build():
    edges = []
    for i, e in enumerate(EDGES):
        a, b, attrs = e[0], e[1], e[2]
        edges.append({"id": i + 1, "pts": [list(corner(*a)), list(corner(*b))], "attrs": attrs,
                      **({"closed": True} if len(e) > 3 and e[3] else {})})
    cases = []
    for name, pname, day_text, a, b, cost in QUESTIONS:
        profile = profile_of(pname) if pname else None
        assert pname is None or profile is not None, pname
        day = day_of(day_text) if day_text else None
        assert day_text is None or day is not None, day_text
        found, only = route(a, b, cost, profile, day)
        assert only, f"{name}: more than one cheapest way"
        by_profile, by_date = counts(profile, day)
        case = {"name": name, "profile": pname, "day": day_text, "from": list(corner(*a)), "to": list(corner(*b)),
                "cost": cost, "byProfile": by_profile, "byDate": by_date, "unread": unread_ids(profile, day)}
        if found is None:
            case["unreachable"] = True
        else:
            case["total"] = found[0]
            case["totals"] = list(found[1])
        cases.append(case)
    refusals = [
        {"profile": "Bisikletli", "error": "Ağda “Bisikletli” araç profili yok; profiller: Kamyon, Yaya, Otobüs, Ambulans."},
        {"day": "3.11.2026", "error": "Tarih “3.11.2026” okunamadı; 2026-11-03 gibi yazın."},
        {"day": "2026-02-30", "error": "Tarih “2026-02-30” okunamadı; 2026-11-03 gibi yazın."},
    ]
    return {"format": "kentos.network-access", "version": 1, "definition": DEFINITION, "edges": edges,
            "cases": cases, "refusals": refusals}


def self_checks():
    # The rules' corners: the day bounds are inclusive, a limit's equal value passes, Turkish case folds.
    assert closing({"kapanis_bas": "2026-11-03", "kapanis_son": "2026-11-03"}, None, day_of("2026-11-03"))[0] == "date"
    assert closing({"kapanis_bas": "2026-11-03"}, None, day_of("2026-11-02"))[0] is None
    assert closing({"agirlik": "18"}, profile_of("Kamyon"), None)[0] is None
    assert closing({"agirlik": "17.9"}, profile_of("Kamyon"), None)[0] == "profile"
    assert closing({"tur": "BİSİKLET"}, profile_of("Kamyon"), None)[0] == "profile"
    assert closing({"agirlik": "?"}, profile_of("Kamyon"), None) == (None, True)
    assert closing({"agirlik": "10", "yukseklik": "?"}, profile_of("Kamyon"), None) == ("profile", False)
    assert day_of("2026-11-03") == 20760


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    self_checks()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if OUT.read_text("utf-8") != text:
            print(f"{OUT} güncel değil; betiği --check'siz çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(QUESTIONS)} soru, 3 ret.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(QUESTIONS)} soru, 3 ret.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
