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
DAYS = {"donem1": "2026-01-15", "donem2": "2026-04-15", "donem3": "2026-07-15"}
EPOCHS = {"donem1": [(n, x, y, z) for n, x, y, z in ab.EPOCH1],
          "donem2": [(n, x, y, z) for n, x, y, z, _ in ab.EPOCH2],
          "donem3": [(n, x, y, z) for n, x, y, z, _ in EPOCH3]}


def setup(work="map"):
    s = ab.setup(work)
    s["layers"].append(ab.layer("donem3", "Üçüncü dönem"))
    for name, x, y, z, _ in EPOCH3:
        n = len(s["entities"]) + 1
        s["entities"].append({"kind": "point", "layerId": "donem3", "attrs": {"Ad": name}, "p": ab.P(x, y), "z": z, "id": n})
        s["uids"].append(ab.uid(n))
    return s


def third_ids():
    e1, e2 = ab.epoch_ids()
    n = int(e2[-1].rsplit("-", 1)[1], 16)
    return [ab.uid(n + 1 + i) for i in range(len(EPOCH3))]


def layer_ids(layer):
    e1, e2 = ab.epoch_ids()
    return {"donem1": e1, "donem2": e2, "donem3": third_ids()}[layer]


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
        out.append(row)
    return out


def answer(i):
    """asbuilt_cases.py's monitoring answer of this epoch against `design`, with the tracks."""
    plan = (EPOCHS[i["design"]], [], (layer_ids(i["design"]), []))
    measured = ([(n, x, y, z, s) for n, x, y, z, s in EPOCH3], third_ids()) if i["measured"] == "donem3" else None
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
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.asbuilt.check", "commandVersion": 1,
           "title": "Uygulama kontrolü: izlemenin dönem dizisi",
           "note": "ADR 4420. scripts/fixtures/asbuilt_series_cases.py: asbuilt_cases.py'nin çizimi ve üçüncü dönem; izler "
                   "elle (TBC'nin başvuru dönemi kuralı); KentOS kodu yok.",
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
