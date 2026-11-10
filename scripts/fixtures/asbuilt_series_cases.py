#!/usr/bin/env python3
"""The shared cases of `cad.asbuilt.check`'s monitoring series (docs/adr/4420; plan/map-cad.md MAP-19), without KentOS
code.

    python3 scripts/fixtures/asbuilt_series_cases.py           # writes fixtures/commands/v1/cad.asbuilt.check.series.json
    python3 scripts/fixtures/asbuilt_series_cases.py --check   # writes nothing; compares

The drawing is asbuilt_cases.py's with a third epoch on its own layer after the rest (the earlier objects keep their
ids): the marks measured again three months after the second epoch. The answer is asbuilt_cases.py's monitoring answer
of this epoch against the one named `design`, and the tracks, by hand (Trimble Business Center's Monitor Point
Movement): the series is the earlier epochs, oldest first, then this one; a mark is matched by name; its reference is
the first epoch it was measured in; at each epoch from it on its movement from the reference (east, north, height,
horizontal), flagged beyond the job's limits; only a mark measured in two epochs or more is tracked; with the days of
its reference and of this epoch, its mean rates: its last movement over the years between (365.25 days a year).

Docs/adr/4840, by hand too: asked, a line through each mark's dated epochs (3 or more): its movement from its reference
against the years since its first dated epoch, east, north and up apart, by least squares (v = Sxy / Sxx, s² = Σr² /
(n − 2), σᵥ = s / √Sxx); a component moves when |v| > t·σᵥ, Student's t at (1 + c)/2 from its closed forms (1 degree
of freedom tan(π(p − ½)), 2 degrees (2p − 1) / √(2p(1 − p))), so no case has more than four epochs; the speed the
east and north ones' hypotenuse. Thresholds: a mark's last horizontal movement, its last height movement either way,
its horizontal and vertical speed (the line's, else the mean rate), each beyond its alarm limit puts it in alarm, else
beyond its warning limit in warning, else normal; the limits of that level it goes beyond listed. A fourth epoch
(2026-10-15) on its own layer gives the line 2 degrees of freedom.
"""

import json
import math
import sys
from datetime import date as Day
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import asbuilt_cases as ab  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures" / "commands" / "v1" / "cad.asbuilt.check.series.json"

# The third epoch (2026-07-15): M1 and M2 go on moving, M3 settles further, M5 is measured again (not at the second),
# M6 (new at the second) moves 4 mm, M7 is new; K1 and K2 as at the second.
EPOCH3 = [("M1", 10.006, 60.008, 99.995, None), ("M2", 20.03, 60.022, 100.498, None), ("M3", 30.0, 60.0, 100.97, None),
          ("M4", 40.001, 60.001, 101.2, None), ("M5", 50.002, 60.0, 101.39, None), ("K1", 0.0, 50.0, 99.0, None),
          ("K2", 60.006, 50.008, 99.5, None), ("M6", 70.004, 60.0, 101.598, None), ("M7", 80.0, 60.0, 101.8, None)]
# The fourth epoch (2026-10-15; docs/adr/4840, in its own setup): M1 and M2 go on, M3 settles further, the rest stay.
EPOCH4 = [("M1", 10.009, 60.012, 99.992, None), ("M2", 20.041, 60.03, 100.497, None), ("M3", 30.0, 60.0, 100.952, None),
          ("M4", 40.0, 60.002, 101.2, None), ("M5", 50.001, 60.001, 101.39, None), ("K1", 0.0, 50.0, 99.0, None),
          ("K2", 60.006, 50.008, 99.5, None), ("M6", 70.007, 60.0, 101.597, None), ("M7", 80.002, 60.0, 101.8, None)]
DAYS = {"donem1": "2026-01-15", "donem2": "2026-04-15", "donem3": "2026-07-15", "donem4": "2026-10-15"}
EPOCHS = {"donem1": [(n, x, y, z) for n, x, y, z in ab.EPOCH1],
          "donem2": [(n, x, y, z) for n, x, y, z, _ in ab.EPOCH2],
          "donem3": [(n, x, y, z) for n, x, y, z, _ in EPOCH3],
          "donem4": [(n, x, y, z) for n, x, y, z, _ in EPOCH4]}


def setup(work="map", fourth=False):
    s = ab.setup(work)
    layers = [("donem3", "Üçüncü dönem", EPOCH3)] + ([("donem4", "Dördüncü dönem", EPOCH4)] if fourth else [])
    for layer, title, points in layers:
        s["layers"].append(ab.layer(layer, title))
        for name, x, y, z, _ in points:
            n = len(s["entities"]) + 1
            s["entities"].append({"kind": "point", "layerId": layer, "attrs": {"Ad": name}, "p": ab.P(x, y), "z": z, "id": n})
            s["uids"].append(ab.uid(n))
    return s


def third_ids():
    e1, e2 = ab.epoch_ids()
    n = int(e2[-1].rsplit("-", 1)[1], 16)
    return [ab.uid(n + 1 + i) for i in range(len(EPOCH3))]


def fourth_ids():
    n = int(third_ids()[-1].rsplit("-", 1)[1], 16)
    return [ab.uid(n + 1 + i) for i in range(len(EPOCH4))]


def layer_ids(layer):
    e1, e2 = ab.epoch_ids()
    return {"donem1": e1, "donem2": e2, "donem3": third_ids(), "donem4": fourth_ids()}[layer]


def t_two_sided(f, confidence):
    """Student's t at p = (1 + c)/2 with f degrees of freedom, from its closed forms (1 and 2 degrees)."""
    p = (1.0 + confidence) / 2.0
    if f == 1:
        return math.tan(math.pi * (p - 0.5))
    if f == 2:
        return (2.0 * p - 1.0) / math.sqrt(2.0 * p * (1.0 - p))
    raise ValueError(f"{f} degrees of freedom: no closed form here")


def velocity(t, d):
    """The least squares line's slope and its standard deviation (docs/adr/4840), as the module's docstring says."""
    n = len(t)
    tm = sum(t) / n
    dm = sum(d) / n
    sxx = sxy = 0.0
    for ti, di in zip(t, d):
        sxx += (ti - tm) * (ti - tm)
        sxy += (ti - tm) * (di - dm)
    rate = sxy / sxx
    a = dm - rate * tm
    ss = 0.0
    for ti, di in zip(t, d):
        r = di - (a + rate * ti)
        ss += r * r
    return rate, math.sqrt(ss / (n - 2) / sxx)


def fit(steps, days, confidence):
    dated = [(Day.fromisoformat(d), s) for s, d in zip(steps, days) if s is not None and d]
    if len(dated) < 3:
        return None
    first = dated[0][0]
    t = [(d - first).days / 365.25 for d, _ in dated]
    e = velocity(t, [s["de"] for _, s in dated])
    n = velocity(t, [s["dn"] for _, s in dated])
    zs = [(ti, s["dz"]) for ti, (_, s) in zip(t, dated) if s.get("dz") is not None]
    up = velocity([p[0] for p in zs], [p[1] for p in zs]) if len(zs) >= 3 else None
    moving = []
    for name, v, count in (("east", e, len(dated)), ("north", n, len(dated)), ("up", up, len(zs))):
        if v is None:
            continue
        tq = t_two_sided(count - 2, confidence)
        # No case lies on the test's edge: the rule's two sides are a percent apart at least.
        assert v[1] == 0.0 or abs(abs(v[0]) / v[1] - tq) > 0.01 * tq, (name, v, tq)
        if abs(v[0]) > tq * v[1]:
            moving.append(name)
    out = {"epochs": len(dated), "east": e[0], "north": n[0]}
    if up is not None:
        out["up"] = up[0]
    out.update({"sigmaEast": e[1], "sigmaNorth": n[1]})
    if up is not None:
        out["sigmaUp"] = up[1]
    out["speed"] = math.hypot(e[0], n[0])
    if moving:
        out["moving"] = moving
    return out


LIMIT_KEYS = ["horizontal", "vertical", "horizontalRate", "verticalRate"]


def state(row, thresholds):
    last = row["steps"][-1]
    f = row.get("fit")
    values = {
        "horizontal": last["horizontal"],
        "vertical": abs(last["dz"]) if last.get("dz") is not None else None,
        "horizontalRate": f["speed"] if f else row.get("rateHorizontal"),
        "verticalRate": abs(f["up"]) if f and "up" in f else (abs(row["rateVertical"]) if "rateVertical" in row else None),
    }
    for level in (thresholds["warning"], thresholds["alarm"]):
        for k in LIMIT_KEYS:
            if level.get(k) is not None and values[k] is not None:
                # No case lies on a limit: a value and a limit are a micrometre apart at least.
                assert abs(values[k] - level[k]) > 1e-6, (row["name"], k, values[k], level[k])
    beyond = lambda level: [k for k in LIMIT_KEYS if level.get(k) is not None and values[k] is not None and values[k] > level[k]]  # noqa: E731
    alarm = beyond(thresholds["alarm"])
    if alarm:
        return "alarm", alarm
    warning = beyond(thresholds["warning"])
    if warning:
        return "warning", warning
    return "normal", []


def tracks(i):
    """Docs/adr/4420: each mark measured now through the series (the module's rules, by hand)."""
    series = [e["layer"] for e in i.get("epochs", [])] + [i["measured"]]
    days = [e.get("date") for e in i.get("epochs", [])] + [i.get("date")]
    h_limit, v_limit = i.get("horizontal"), i.get("vertical")
    out = []
    for name, x, y, z in EPOCHS[i["measured"]]:
        at = []
        for layer in series:
            hits = [p for p in EPOCHS[layer] if p[0] == name]
            at.append(hits[0] if len(hits) == 1 else None)
        if sum(1 for a in at if a is not None) < 2:
            continue
        frm = next(k for k, a in enumerate(at) if a is not None)
        _, rx, ry, rz = at[frm]
        steps = []
        for a in at:
            if a is None:
                steps.append(None)
                continue
            de, dn, dz = a[1] - rx, a[2] - ry, a[3] - rz
            horizontal = math.hypot(de, dn)
            st = {"de": de, "dn": dn, "dz": dz, "horizontal": horizontal}
            if (h_limit is not None and horizontal > h_limit) or (v_limit is not None and abs(dz) > v_limit):
                st["outside"] = True
            steps.append(st)
        row = {"name": name, "from": frm, "steps": steps}
        if days[frm] and days[-1]:
            years = (Day.fromisoformat(days[-1]) - Day.fromisoformat(days[frm])).days / 365.25
            if years > 0:
                last = steps[-1]
                row["rateHorizontal"] = last["horizontal"] / years
                row["rateVertical"] = last["dz"] / years
        if i.get("trend"):
            f = fit(steps, days, i["trend"]["confidence"])
            if f is not None:
                row["fit"] = f
        if i.get("thresholds"):
            st, beyond = state(row, i["thresholds"])
            row["state"] = st
            if beyond:
                row["exceeded"] = beyond
        out.append(row)
    return out


def answer(i):
    """asbuilt_cases.py's monitoring answer of this epoch against `design`, with the tracks."""
    plan = (EPOCHS[i["design"]], [], (layer_ids(i["design"]), []))
    measured = {"donem3": (list(EPOCH3), third_ids()), "donem4": (list(EPOCH4), fourth_ids())}.get(i["measured"])
    out = ab.report(design=True, surface=False, lines=False, search=None, horizontal=i.get("horizontal"),
                    vertical=i.get("vertical"), quality=i.get("quality"), expected=None,
                    quality_field="qualityField" in i, plan=plan, measured=measured)
    out["kind"] = "monitoring"
    if i.get("control"):
        c = {"checked": [], "moved": [], "missing": []}
        for name in i["control"]:
            p = next((p for p in out["points"] if p["name"] == name and p.get("by") == "name"), None)
            if p is None:
                c["missing"].append(name)
                continue
            c["checked"].append(name)
            if "outsideHorizontal" in p["flags"] or "outsideVertical" in p["flags"]:
                c["moved"].append(name)
        out["control"] = c
    out = ab.order(out)
    t = tracks(i)
    if t:
        out["tracks"] = t
    return out


SERIES = {"kind": "monitoring", "measured": "donem3", "design": "donem2", "horizontal": 0.008, "vertical": 0.01,
          "control": ["K1", "K2"],
          "epochs": [{"layer": "donem1", "date": DAYS["donem1"]}, {"layer": "donem2", "date": DAYS["donem2"]}],
          "date": DAYS["donem3"]}


def ok(name, i, op="plan", setup_=None):
    case = {"name": name, "steps": [{
        "op": op, "input": i,
        "result": {"status": "completed", "output": None if op == "validate" else answer(i), "warnings": []},
        "expect": ab.SAME, "within": ab.W}]}
    if setup_ is not None:
        case["setup"] = setup_
    return case


def refused(name, i, code, message, path_):
    return ab.refused(name, i, code, message, path_)


def cases():
    full = answer(SERIES)
    t = {r["name"]: r for r in full["tracks"]}
    assert set(t) == {"M1", "M2", "M3", "M4", "M5", "K1", "K2", "M6"}, set(t)
    assert t["M5"]["steps"][1] is None and t["M6"]["from"] == 1 and "M7" not in t
    assert t["M2"]["steps"][2].get("outside") and "rateHorizontal" in t["M1"]
    s = lambda **over: {**{k: v for k, v in SERIES.items() if k not in over}, **{k: v for k, v in over.items() if v is not None}}  # noqa: E731
    bad = "invalid_asbuilt_check"
    no_day = "“{}” bir gün değil; günü YYYY-AA-GG olarak yazın (ör. 2026-04-15)."
    return [
        ok("dizi: üç dönem günleriyle; her nokta ilk ölçüldüğü döneme göre, hızıyla", SERIES),
        ok("dizi: günsüz: iz var, hız yok", s(epochs=[{"layer": "donem1"}, {"layer": "donem2"}], date=None)),
        ok("dizi: sınırsız: hiçbir adım işaretli değil", s(horizontal=None, vertical=None)),
        ok("dizi: yalnız yatay sınır", s(vertical=None)),
        ok("dizi: yalnız düşey sınır", s(horizontal=None)),
        ok("dizi: yalnız birinci dönem dizide; ikinci dönem karşılaştırılan", s(epochs=[{"layer": "donem1", "date": DAYS["donem1"]}])),
        ok("dizi: yalnız ikinci dönem dizide", s(epochs=[{"layer": "donem2", "date": DAYS["donem2"]}])),
        ok("dizi: birinci dönemin günü yok: yalnız ikinciden başlayanın hızı",
           s(epochs=[{"layer": "donem1"}, {"layer": "donem2", "date": DAYS["donem2"]}])),
        ok("dizi: aynı dönem iki kez", s(epochs=[{"layer": "donem1", "date": DAYS["donem1"]},
                                                 {"layer": "donem1", "date": DAYS["donem1"]}])),
        ok("dizi: bu dönem dizide de", s(epochs=[{"layer": "donem1", "date": DAYS["donem1"]},
                                                 {"layer": "donem2", "date": DAYS["donem2"]},
                                                 {"layer": "donem3", "date": DAYS["donem3"]}])),
        ok("dizi: önceki dönem birinci (TBC'nin başvuru dönemi)", s(design="donem1")),
        ok("dizi: kalite sınırıyla", s(qualityField="Hassasiyet", quality=0.01)),
        ok("dizi: uygula salt okunur", SERIES, op="execute"),
        ok("dizi: doğrulama", SERIES, op="validate"),
        ok("dizi: Yol projesinde", SERIES, setup_=setup("road")),
        ok("dizi: CBS projesinde", SERIES, setup_=setup("gis")),
        refused("dizi izleme dışında", {**ab.FULL, "epochs": [{"layer": "donem1"}]}, bad,
                "Dönem dizisi ve dönemin günü izleme işinindir (kind: monitoring).", "epochs"),
        refused("dönemin günü izleme dışında", {**ab.FULL, "date": "2026-07-15"}, bad,
                "Dönem dizisi ve dönemin günü izleme işinindir (kind: monitoring).", "date"),
        refused("dönemin günü gün değil", s(epochs=[{"layer": "donem1"}, {"layer": "donem2", "date": "2026-02-30"}]), bad,
                no_day.format("2026-02-30"), "epochs[1].date"),
        refused("bu dönemin günü biçimsiz", s(date="15.07.2026"), bad, no_day.format("15.07.2026"), "date"),
        refused("dönemin katmanı yok", s(epochs=[{"layer": "yok"}]), "layer_not_found",
                "“yok” kimlikli katman çizimde yok.", "epochs[0].layer"),
        refused("dizide önceki dönem yine gerekir", s(design=None), bad,
                "İzleme işinde önceki dönemin noktalarının katmanını (design) verin; noktalar adla karşılaştırılır.",
                "design"),
    ] + trend_cases(s)


def trend_cases(s):
    """Docs/adr/4840: the line through the dated epochs and the two thresholds."""
    bad = "invalid_asbuilt_check"
    three = s(trend={"confidence": 0.95})
    t3 = {r["name"]: r for r in answer(three)["tracks"]}
    assert "fit" in t3["M1"] and "fit" not in t3["M5"] and "fit" not in t3["M6"], t3
    four = {**SERIES, "measured": "donem4", "design": "donem3", "date": DAYS["donem4"], "trend": {"confidence": 0.95},
            "epochs": [{"layer": k, "date": DAYS[k]} for k in ("donem1", "donem2", "donem3")]}
    four_setup = setup(fourth=True)
    t4 = {r["name"]: r for r in answer(four)["tracks"]}
    assert t4["M1"]["fit"]["epochs"] == 4 and "moving" in t4["M3"]["fit"], t4["M3"]
    assert "moving" not in t4["K1"]["fit"], t4["K1"]
    levels = {"warning": {"horizontal": 0.0075, "vertical": 0.012}, "alarm": {"horizontal": 0.03, "vertical": 0.04}}
    limits = s(thresholds=levels)
    states = {r["name"]: r.get("state") for r in answer(limits)["tracks"]}
    assert {"alarm", "warning", "normal"} <= set(states.values()), states
    rates = {"warning": {"horizontalRate": 0.015, "verticalRate": 0.05},
             "alarm": {"horizontalRate": 0.06, "verticalRate": 0.08}}
    speed = s(trend={"confidence": 0.95}, thresholds=rates)
    mean = s(thresholds=rates)
    by = lambda i: {r["name"]: r.get("state") for r in answer(i)["tracks"]}  # noqa: E731
    assert by(speed) != {k: None for k in by(speed)}, by(speed)
    lone = {k: v for k, v in s().items() if k not in ("epochs", "date")}
    return [
        ok("doğru: üç günlü dönem, güven %95; iki dönemli noktada doğru yok", three),
        ok("doğru: dört dönem, serbestlik iki; oturan M3 anlamlı, K1 değil", four, setup_=four_setup),
        ok("doğru: güven %50, aynı noktalar daha kolay anlamlı", s(trend={"confidence": 0.5})),
        ok("eşikler: yatay ve düşey hareketle uyarı ve alarm", limits),
        ok("eşikler: hızla, doğrunun hızı", speed),
        ok("eşikler: hızla, doğru istenmeden ortalama hız", mean),
        ok("doğru ve eşikler birlikte, dört dönem", {**four, "thresholds": {**levels, "alarm": {**levels["alarm"], **rates["alarm"]},
                                                                             "warning": {**levels["warning"], **rates["warning"]}}},
           setup_=four_setup),
        refused("doğru dizisiz", {**lone, "trend": {"confidence": 0.95}}, bad,
                "Doğru uydurma ve eşikler izleme işinin dönem dizisinindir (kind: monitoring ve önceki dönemler, epochs).",
                "trend"),
        refused("eşikler dizisiz", {**lone, "thresholds": levels}, bad,
                "Doğru uydurma ve eşikler izleme işinin dönem dizisinindir (kind: monitoring ve önceki dönemler, epochs).",
                "thresholds"),
        refused("güven düzeyi 1'den büyük", s(trend={"confidence": 95.0}), bad,
                "Güven düzeyi 0 ile 1 arasında bir sayı olmalı (ör. 0.95).", "trend.confidence"),
        refused("eksi eşik", s(thresholds={"warning": {"vertical": -0.01}, "alarm": {}}), bad,
                "Düşey hareketin uyarı eşiği 0'dan büyük bir sayı olmalı; denetlenmeyecekse boş bırakın.",
                "thresholds.warning.vertical"),
        refused("uyarı alarmdan büyük", s(thresholds={"warning": {"horizontalRate": 0.05}, "alarm": {"horizontalRate": 0.02}}),
                bad, "Yatay hızın uyarı eşiği alarm eşiğinden küçük olmalı.", "thresholds.warning.horizontalRate"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.asbuilt.check", "commandVersion": 1,
           "title": "Uygulama kontrolü: izlemenin dönem dizisi",
           "note": "ADR 4420. scripts/fixtures/asbuilt_series_cases.py: asbuilt_cases.py'nin çizimi ve üçüncü dönem; izler "
                   "elle (TBC'nin başvuru dönemi kuralı); KentOS kodu yok. ADR 4840: günlü dönemlerden en küçük kareler "
                   "doğrusu ve Student t'nin kapalı biçimleriyle anlamlılık, uyarı ve alarm eşikleri elle; dördüncü dönem "
                   "kendi kurulumunda.",
           "setup": setup(), "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        good = OUT.exists() and OUT.read_text("utf-8") == text
        print("Dönem dizisi ortak durumları güncel." if good else f"{OUT.name} güncel değil.")
        return 0 if good else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.name} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
