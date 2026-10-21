#!/usr/bin/env python3
"""Independent reference of a closure's weekdays and hours in network analysis (docs/adr/0568), written from the ADR
without KentOS code.

One scene: a street grid of straight two-way edges meeting at their ends, a speed cost whose speed halves from 07:00
to 09:00 (docs/adr/0521's time profile), and closures read from four edge fields: the first and the last day, the
weekdays and the hours. A weekday rush closure, a night closure every day, a weekend closure all day, a dated closure in
hours, a closure after midnight on Wednesdays, and two values that cannot be read. The questions: routes between two
corners by length and by time for a day and a time of day, and what the access closed. Written to
fixtures/network/v1/timed-closures.json; the geometry core (crates/shared/geometry-core/tests/all/network_access.rs)
must give the same: what was closed and unread exactly, the costs within 1e-9 (of their size above 1).

The ADR's rules: a closure applies on a day `d` when its first and last day hold it (an empty one open-ended) and its
weekday is one the weekday field names (empty: every day); without hours the edge is closed that whole day (the
question's day only); with hours it is closed in them, a window ending before it starts going on into the next day.
Hours are read only with a time: the time cost goes at no speed in them (a vehicle waits or goes round) and the length
passes none closed at the question's moment. The reference's time search is an earliest-arrival Dijkstra over the
corners (first in, first out), each edge's minutes found by stepping through the speed's changes and the windows from
the moment it is entered; the windows are the actual days around the question's (the day before to five after), so
no question here runs past them. Every case's cheapest way is checked to be the only one.

    python3 scripts/fixtures/network_timed_closures_cases.py [--check]
"""

import argparse
import datetime
import heapq
import json
import math
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/network/v1/timed-closures.json"
X0, Y0 = 400000.0, 4500000.0
STEP = 100.0
DAY = 1440.0

DEFINITION = {
    "id": "ag-1",
    "name": "Yollar",
    "kind": "road",
    "edges": [{"layer": "yol"}],
    "connect": "ends",
    "tolerance": 0.01,
    "direction": {"kind": "both"},
    "costs": [{"name": "Süre", "kind": "speed", "field": "hiz", "speed": 50.0,
               "periods": [{"from": 420, "to": 540, "factor": 0.5}]}],
    "closures": {"from": "kapanis_bas", "to": "kapanis_son", "days": "kapanis_gun", "hours": "kapanis_saat"},
}
PERIODS = [(420.0, 540.0, 0.5)]


def corner(i, j):
    """A corner of the grid, moved by a few metres (a fixed sequence) so that no two ways are of one length."""
    k = i * 3 + j
    return (X0 + i * STEP + ((k * k * 5 + k * 11) % 23) * 0.41, Y0 + j * STEP + ((k * k * 13 + k * 3) % 29) * 0.31)


# The corners are (i, j), i 0–3 east, j 0–2 north. Each edge: its two corners and its attributes (absent: empty).
EDGES = [
    # The southern street: fast, closed on weekday mornings in the middle.
    ((0, 0), (1, 0), {"hiz": "50"}),
    ((1, 0), (2, 0), {"hiz": "50", "kapanis_gun": "Pzt-Cum", "kapanis_saat": "07:00-09:00"}),
    ((2, 0), (3, 0), {"hiz": "50"}),
    # The middle street: closed every night in the middle.
    ((0, 1), (1, 1), {"hiz": "30"}),
    ((1, 1), (2, 1), {"hiz": "30", "kapanis_saat": "22:00-06:00"}),
    ((2, 1), (3, 1), {"hiz": "30"}),
    # The northern street: closed at weekends, closed in two hours of three days, closed after midnight on Wednesdays.
    ((0, 2), (1, 2), {"hiz": "30", "kapanis_gun": "Cmt, Paz"}),
    ((1, 2), (2, 2), {"hiz": "30", "kapanis_bas": "2026-11-02", "kapanis_son": "2026-11-04",
                      "kapanis_saat": "10:00-12:00"}),
    ((2, 2), (3, 2), {"hiz": "30", "kapanis_gun": "Çarşamba", "kapanis_saat": "00:00-02:00"}),
    # The avenues north: slower; two values that cannot be read.
    ((0, 0), (0, 1), {"hiz": "40"}),
    ((0, 1), (0, 2), {"hiz": "40", "kapanis_gun": "Pazartesi-Cumaa"}),
    ((1, 0), (1, 1), {"hiz": "40"}),
    ((1, 1), (1, 2), {"hiz": "40"}),
    ((2, 0), (2, 1), {"hiz": "40"}),
    ((2, 1), (2, 2), {"hiz": "40"}),
    ((3, 0), (3, 1), {"hiz": "40"}),
    ((3, 1), (3, 2), {"hiz": "40", "kapanis_saat": "sabah"}),
]

WEEKDAYS = [
    ["1", "pazartesi", "pzt", "pts"],
    ["2", "salı", "sali", "sal"],
    ["3", "çarşamba", "carsamba", "çar", "car", "çrş"],
    ["4", "perşembe", "persembe", "per", "prş"],
    ["5", "cuma", "cum"],
    ["6", "cumartesi", "cmt", "cts"],
    ["7", "pazar", "paz", "pzr"],
]


def fold(s):
    """Trimmed, case folded with Turkish I and İ."""
    return "".join({"I": "ı", "İ": "i"}.get(c, c.lower()) for c in s.strip())


def day_of(t):
    t = t.strip()
    if len(t) < 10 or t[4] != "-" or t[7] != "-" or (len(t) > 10 and t[10] not in "T "):
        return None
    try:
        return (datetime.date(int(t[0:4]), int(t[5:7]), int(t[8:10])) - datetime.date(1970, 1, 1)).days
    except ValueError:
        return None


def weekday(d):
    """0 Monday … 6 Sunday."""
    return (datetime.date(1970, 1, 1) + datetime.timedelta(days=d)).weekday()


def weekdays_of(t):
    def named(n):
        k = fold(n)
        return next((i for i, names in enumerate(WEEKDAYS) if k in names), None)

    out = set()
    for part in t.replace(";", ",").split(","):
        if "-" in part:
            a, b = (named(x) for x in part.split("-", 1))
            if a is None or b is None:
                return None
            d = a
            while True:
                out.add(d)
                if d == b:
                    break
                d = (d + 1) % 7
        else:
            d = named(part)
            if d is None:
                return None
            out.add(d)
    return out


def hours_of(t):
    def clock(s):
        s = s.strip()
        if ":" not in s:
            return None
        h, m = s.split(":", 1)
        if not (h.isdigit() and 1 <= len(h) <= 2 and m.isdigit() and len(m) == 2):
            return None
        h, m = int(h), int(m)
        if m >= 60 or h > 24 or (h == 24 and m != 0):
            return None
        return h * 60 + m

    out = []
    for part in t.replace(";", ",").split(","):
        if "-" not in part:
            return None
        a, b = (clock(x) for x in part.split("-", 1))
        if a is None or b is None or a == 1440 or a == b:
            return None
        out.append((a, b))
    return out


def closing(attrs, day):
    """Why an edge is closed on `day` (`date`, `hours` with its windows in minutes from the day's midnight, or none)
    and whether a value could not be read."""
    if day is None:
        return None, [], False
    vals = {f: (attrs.get(f, "").strip() or None) for f in DEFINITION["closures"].values()}
    c = DEFINITION["closures"]
    first = last = mask = hours = None
    if vals[c["from"]] is not None:
        first = day_of(vals[c["from"]])
        if first is None:
            return None, [], True
    if vals[c["to"]] is not None:
        last = day_of(vals[c["to"]])
        if last is None:
            return None, [], True
    if vals[c["days"]] is not None:
        mask = weekdays_of(vals[c["days"]])
        if mask is None:
            return None, [], True
    if vals[c["hours"]] is not None:
        hours = hours_of(vals[c["hours"]])
        if hours is None:
            return None, [], True
    if all(v is None for v in vals.values()):
        return None, [], False

    def on(d):
        return (first is None or first <= d) and (last is None or d <= last) and (mask is None or weekday(d) in mask)

    if hours is None:
        return ("date" if on(day) else None), [], False
    windows = []
    for k in range(-1, 6):
        if on(day + k):
            for s, e in hours:
                windows.append((k * DAY + s, k * DAY + (e if e > s else e + DAY)))
    return ("hours" if windows else None), windows, False


def speed_at(t, kmh, windows):
    """Metres a minute at minute `t` (from the question's midnight)."""
    if any(a <= t < b for a, b in windows):
        return 0.0
    m = t - math.floor(t / DAY) * DAY
    f = next((f for a, b, f in PERIODS if a <= m < b), 1.0)
    return kmh * f * 1000.0 / 60.0


def next_change(t, windows):
    """The first moment after `t` the speed may change."""
    base = math.floor(t / DAY) * DAY
    cands = [base + x for a, b, _ in PERIODS for x in (a, b)] + [base + DAY]
    cands += [x for w in windows for x in w]
    return min(x for x in cands if x > t)


def minutes_forward(t0, metres, kmh, windows):
    t, left = t0, metres
    while True:
        end = next_change(t, windows)
        v = speed_at(t, kmh, windows)
        if v > 0.0:
            need = left / v
            if t + need <= end:
                return t + need - t0
            left -= v * (end - t)
        t = end


def edge_info(e, day):
    a, b, attrs = e
    (ax, ay), (bx, by) = corner(*a), corner(*b)
    why, windows, _ = closing(attrs, day)
    return {"a": a, "b": b, "len": math.hypot(bx - ax, by - ay), "kmh": float(attrs["hiz"]), "why": why,
            "windows": windows}


def route(a, b, cost, day, time):
    """The cheapest way from corner a to b by length (0) or by time (1): (cost, length, minutes) and whether it is the
    only cheapest; None when there is none."""
    infos = [edge_info(e, day) for e in EDGES]
    timed = time is not None

    def open_for(info, c):
        if info["why"] == "date":
            return False
        if info["why"] == "hours" and c == 0 and timed:
            return not any(w0 <= time < w1 for w0, w1 in info["windows"])
        return True

    def windows_of(info):
        return info["windows"] if timed and info["why"] == "hours" else []

    def minutes(info, t):
        if not timed:
            return info["len"] * 60.0 / (info["kmh"] * 1000.0)
        return minutes_forward(t, info["len"], info["kmh"], windows_of(info))

    adj = {}
    for k, info in enumerate(infos):
        if open_for(info, cost):
            adj.setdefault(info["a"], []).append((info["b"], k))
            adj.setdefault(info["b"], []).append((info["a"], k))
    t0 = time if timed else 0.0
    # Labels: the cost so far, the length and the clock (minutes from the question's midnight).
    best = {a: (0.0, 0.0, t0)}
    count = {a: 1}
    heap = [(0.0, a)]
    done = set()
    while heap:
        d, n = heapq.heappop(heap)
        if n in done:
            continue
        done.add(n)
        _, length, clock = best[n]
        for m, k in adj.get(n, []):
            info = infos[k]
            dt = minutes(info, clock)
            if not math.isfinite(dt):
                continue
            step = info["len"] if cost == 0 else dt
            nd = d + step
            label = (nd, length + info["len"], clock + dt)
            if m not in best or nd < best[m][0] - 1e-9:
                best[m] = label
                count[m] = count[n]
                heapq.heappush(heap, (nd, m))
            elif abs(nd - best[m][0]) <= 1e-9:
                count[m] += count[n]
    if b not in best:
        return None, True
    c, length, clock = best[b]
    return (c, length, clock - t0), count[b] == 1


QUESTIONS = [
    # (name, day, time (minutes of the day), from, to, cost)
    ("saatsiz", "2026-11-03", None, (0, 0), (3, 0), 1),
    ("gunsuz", None, 480.0, (0, 0), (3, 0), 1),
    ("sabah-sure", "2026-11-03", 480.0, (0, 0), (3, 0), 1),
    ("sabah-uzunluk", "2026-11-03", 480.0, (0, 0), (3, 0), 0),
    ("sabah-oncesi", "2026-11-03", 419.95, (0, 0), (3, 0), 1),
    ("sabah-sonrasi", "2026-11-03", 540.0, (0, 0), (3, 0), 1),
    ("hafta-sonu", "2026-11-07", 480.0, (0, 0), (3, 0), 1),
    ("hafta-sonu-kuzey", "2026-11-07", 600.0, (0, 2), (3, 2), 0),
    ("gece", "2026-11-03", 330.0, (0, 1), (3, 1), 1),
    ("gece-uzunluk", "2026-11-03", 330.0, (0, 1), (3, 1), 0),
    ("gece-oncesi", "2026-11-03", 1319.9, (0, 1), (3, 1), 1),
    ("gece-yarisi", "2026-11-03", 1439.7, (0, 2), (3, 2), 1),
    ("carsamba-ogle", "2026-11-04", 630.0, (0, 2), (3, 2), 1),
    ("carsamba-ogle-uzunluk", "2026-11-04", 630.0, (0, 2), (3, 2), 0),
    ("tarih-disi", "2026-11-05", 630.0, (0, 2), (3, 2), 1),
    ("okunamayan", "2026-11-03", 720.0, (0, 0), (3, 2), 1),
    ("sabah-capraz", "2026-11-03", 470.0, (0, 0), (3, 2), 1),
    ("pazartesi-gece", "2026-11-02", 1380.0, (3, 1), (0, 1), 1),
]


def counts(day, time):
    by = {"date": 0, "hours": 0}
    unread = []
    for i, e in enumerate(EDGES):
        why, _, u = closing(e[2], day)
        if why:
            by[why] += 1
        if u:
            unread.append(i + 1)
    return by["date"], by["hours"], unread


def build():
    edges = [{"id": i + 1, "pts": [list(corner(*e[0])), list(corner(*e[1]))], "attrs": e[2]}
             for i, e in enumerate(EDGES)]
    cases = []
    for name, day_text, time, a, b, cost in QUESTIONS:
        day = day_of(day_text) if day_text else None
        found, only = route(a, b, cost, day, time)
        assert only, f"{name}: more than one cheapest way"
        by_date, by_hours, unread = counts(day, time)
        case = {"name": name, "day": day_text, "time": time, "from": list(corner(*a)), "to": list(corner(*b)),
                "cost": cost, "byDate": by_date, "byHours": by_hours,
                "hoursUnasked": by_hours > 0 and time is None, "unread": unread}
        if found is None:
            case["unreachable"] = True
        else:
            case["total"], length, mins = found
            case["totals"] = [length, mins]
        cases.append(case)
    return {"format": "kentos.network-timed-closures", "version": 1, "definition": DEFINITION, "edges": edges,
            "cases": cases}


def self_checks():
    assert day_of("2026-11-03") == 20760 and weekday(20760) == 1
    assert weekdays_of("Cum-Pzt") == {4, 5, 6, 0}
    assert weekdays_of("1-5; 7") == {0, 1, 2, 3, 4, 6}
    assert weekdays_of("Pzt,") is None
    assert hours_of("22:00-06:00") == [(1320, 360)]
    assert hours_of("00:00-24:00") == [(0, 1440)]
    assert hours_of("24:00-06:00") is None and hours_of("8-9") is None
    # The night window of the day before still closes the early morning; a rush window only on its weekdays.
    _, w, _ = closing({"kapanis_saat": "22:00-06:00"}, 20760)
    assert any(a <= 300 < b for a, b in w)
    # Saturday's week: Friday before it, then Monday to Thursday after it.
    assert closing({"kapanis_gun": "Pzt-Cum", "kapanis_saat": "07:00-09:00"}, day_of("2026-11-07"))[1] == [
        (k * DAY + 420, k * DAY + 540) for k in (-1, 2, 3, 4, 5)]
    assert closing({"kapanis_gun": "Cmt"}, day_of("2026-11-07"))[0] == "date"
    # Ten minutes' wait: 1000 m a minute from 21:59 into a window from 22:00 to 22:10.
    assert abs(minutes_forward(1319.0, 2000.0, 60.0, [(1320.0, 1330.0)]) - 12.0) < 1e-12


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    self_checks()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if args.check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            print(f"{OUT} güncel değil; betiği --check'siz çalıştırın.")
            return 1
        print(f"{OUT.relative_to(ROOT)} güncel: {len(QUESTIONS)} soru.")
        return 0
    OUT.write_text(text, "utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(QUESTIONS)} soru.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
