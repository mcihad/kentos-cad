#!/usr/bin/env python3
"""The shared cases of `road.corridors.daylights` (plan/road.md Y45, docs/adr/0951; ROAD-COR-10).

The corridor's stations are road_corridor_command_cases.py's, each section's terrain, axis and superelevated template
road_section_command_cases.py's (the references of the corridor and the sections, no KentOS code); each side's
daylight is road_daylight_cases.py's, in exact fractions, cutting the slope's ray against every terrain stretch.

    uv run --no-project --python 3.11 --with mpmath --with triangle --with numpy --with pyproj python3 \\
        scripts/fixtures/road_corridor_daylights_command_cases.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_command_cases as ccmd  # noqa: E402
import road_daylight_cases as dref  # noqa: E402
import road_grades  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1/road.corridors.daylights.json"
completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
NOT_ROAD = ccmd.NOT_ROAD
W = 1e-6


def side_of(d, sign):
    if d["kind"] == "hit":
        out = {"kind": "hit", "offset": sign * d["offset"], "cut": d["cut"], "crossings": d["crossings"]}
        if "next" in d:
            out["next"] = d["next"]
        if d["sensitivity"] is not None:
            out["sensitivity"] = d["sensitivity"]
        return out
    if d["kind"] == "gap":
        return {"kind": "gap", "offset": sign * d["at"]}
    if d["kind"] == "swath":
        return {"kind": "swath", "offset": sign * d["end"]}
    return {"kind": "hinge"}


def answer(input_, corridors=ccmd.START, templates=ccmd.TEMPLATES):
    name = input_["corridor"]
    c = next((x for x in corridors if x["id"] == name), None) or next(
        (x for x in corridors if x["name"].lower() == name.lower()), None)
    if c is None:
        raise ccmd.Refused("unknown_corridor", f"“{name}” kimlikli ya da adlı koridor projede yok.", "corridor")
    sens = input_.get("sensitive")
    if sens is not None and not sens > 0:
        raise ccmd.Refused("invalid_daylight_check",
                           "Şev ucu denetimi sorusu geçerli değil: duyarlılık sınırı (sensitive) 0'dan büyük olmalı.",
                           "sensitive")
    report, warnings, _, _ = ccmd.make(c, templates=templates)
    sup = next((x for x in ccmd.SUPERS if x["id"] == c.get("superelevation")), None) if c.get("superelevation") else None
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    stations, counts, worst = [], {"hit": 0, "hinge": 0, "gap": 0, "swath": 0, "multiple": 0, "sensitive": 0,
                                   "unbuilt": 0}, None
    multiple, sensitive = [], []
    for st in report["stations"]:
        s, ri = st["station"], st["region"]
        r = c["regions"][ri]
        t = next(x for x in templates if x["id"] == r["template"])
        fake = {"name": c["name"], "alignment": c["alignment"], "surface": c["surface"], "profile": c["profile"],
                "template": r["template"], "superelevation": sup["id"] if sup else None,
                "left": c["left"], "right": c["right"]}
        row = scmd.sections(fake, [s], templates)[0][0]
        if "left" not in row:
            counts["unbuilt"] += 1
            continue
        tpl = {"left": t["left"], "right": t.get("right") or t["left"]}
        here = scmd.superelevated(tpl, sup, s) if sup else tpl
        ground = [{"offset": g["offset"], "z": g.get("z"), "gap": g["gap"]} for g in row["ground"]]
        both = [dref.diagnose(here["left"], row["axis"], ground, False),
                dref.diagnose(here["right"], row["axis"], ground, True)]
        for k, d in enumerate(both):
            counts[d["kind"]] += 1
            if d["kind"] == "hit":
                if d["crossings"] > 1:
                    counts["multiple"] += 1
                    multiple.append(s)
                if sens is not None and d["sensitivity"] is not None and d["sensitivity"] > sens:
                    counts["sensitive"] += 1
                    sensitive.append(s)
                if d["sensitivity"] is not None and (worst is None or d["sensitivity"] > worst["sensitivity"]):
                    worst = {"station": s, "kmText": text(s), "side": "left" if k == 0 else "right",
                             "sensitivity": d["sensitivity"]}
        stations.append({"station": s, "kmText": st["kmText"], "region": ri, "left": side_of(both[0], -1.0),
                         "right": side_of(both[1], 1.0)})

    def listed(xs):
        out = []
        for x in xs:
            if not out or out[-1] != x:
                out.append(x)
        words = ", ".join(text(x) for x in out[:8])
        return words + (f" ve {len(out) - 8} kesit daha" if len(out) > 8 else "")
    if multiple:
        warnings.append({"code": "corridor_daylight_crossings", "path": "corridor",
                         "message": f"“{c['name']}” koridorunda {len(multiple)} yanda şev araziyi birden çok kez "
                                    f"kesiyor (şev ucu ilk kesişme): {listed(multiple)}."})
    if sens is not None and sensitive:
        warnings.append({"code": "corridor_daylight_sensitive", "path": "sensitive",
                         "message": f"“{c['name']}” koridorunda {len(sensitive)} yanda şev ucu arazinin her metre kot "
                                    f"hatasıyla {ccmd.num(sens)} m'den çok kayıyor: {listed(sensitive)}."})
    out = {"corridor": c["id"]}
    if sens is not None:
        out["sensitive"] = sens
    out.update({"stations": stations, "counts": counts})
    if worst:
        out["worst"] = worst
    return out, warnings


class gentle_slopes:
    """The templates' cut and fill slopes `h` horizontal per one vertical while the block runs: a slope this gentle
    meets the undulating terrain far out, at a shallow angle, and crosses it again beyond."""

    def __init__(self, h):
        self.h = h

    def __enter__(self):
        self.saved = list(ccmd.TEMPLATES)

        def gentle(side):
            return {**side, "cut": self.h, "fill": self.h}
        ccmd.TEMPLATES[:] = [{**t, **{k: gentle(t[k]) for k in ("left", "right") if k in t}} for t in self.saved]
        return self

    def __exit__(self, *exc):
        ccmd.TEMPLATES[:] = self.saved
        return False


def cases():
    out = []

    def ok(name, input_, corridors=ccmd.START, op="execute", own=False):
        o, warns = answer(input_, corridors)
        result = completed(None, warns) if op == "validate" else completed(o, warns)
        s = step(op, input_, result, {**UNCHANGED, "road": ccmd.road_of(corridors)})
        s["within"] = W
        out.append(case(name, [s], None if corridors is ccmd.START and not own else ccmd.setup(corridors)))
        return o

    def no(name, input_, corridors=ccmd.START, result=None, setup_=None):
        if result is None:
            try:
                answer(input_, corridors)
            except ccmd.Refused as r:
                result = r.result
            else:
                raise AssertionError(name)
        road = None if setup_ is not None else ccmd.road_of(corridors)
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                        setup_ if setup_ is not None else (None if corridors is ccmd.START else ccmd.setup(corridors))))

    C1, C2 = ccmd.C1, ccmd.C2
    a = ok("Ana koridorun şev uçları", {"corridor": "c-1"})
    ok("Adıyla, devere bağlı yan koridor", {"corridor": "YAN koridor"})
    ok("Duyarlılık sınırıyla", {"corridor": "c-1", "sensitive": 1.5})
    ok("Gevşek sınır: uyarı yok", {"corridor": "c-1", "sensitive": 1000.0})
    ok("Dar şerit: şerit bitiyor", {"corridor": "c-1"}, [{**C1, "left": 6.0, "right": 6.0}, C2])
    ok("Çok dar şerit: menteşe dışarıda", {"corridor": "c-1"}, [{**C1, "left": 2.0, "right": 2.0}, C2])
    ok("Geniş şerit", {"corridor": "c-1", "sensitive": 3.0}, [{**C1, "left": 60.0, "right": 60.0}, C2])
    ok("Sık kesitler", {"corridor": "c-1"}, [{**C1, "tangent": 10.0}, C2])
    ok("Bir bölge", {"corridor": "c-1"}, [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-1"}]}, C2])
    ok("Arazi dışına taşan koridor", {"corridor": "c-1"},
       [{**C1, "left": 120.0, "right": 120.0, "regions": [{"from": 0.0, "to": 200.0, "template": "tk-1"}]}, C2])
    ok("Boykesitin dışı kesitsiz", {"corridor": "c-1"},
       [{**C1, "profile": "b-3", "regions": [{"from": 50.0, "to": 450.0, "template": "tk-1"}]}, C2])
    ok("Yan koridor, sık", {"corridor": "c-2", "sensitive": 2.5}, [C1, {**C2, "tangent": 20.0}])
    for h in (6.0, 12.0):
        with gentle_slopes(h):
            ok(f"Yatık şevler {ccmd.num(h)}/1: uzakta, yatık açıyla", {"corridor": "c-1", "sensitive": 10.0},
               [{**C1, "left": 60.0, "right": 60.0}, C2], own=True)
    ok("Planı", {"corridor": "c-1", "sensitive": 1.5}, op="plan")
    ok("Doğrulama", {"corridor": "c-1", "sensitive": 3.0}, op="validate")
    ok("Yüzeyi olmayan koridor", {"corridor": "c-1"}, [{**C1, "surface": "t-9"}, C2])
    ok("Ters bölge sırası", {"corridor": "c-1"},
       [{**C1, "regions": [{"from": 0.0, "to": 300.0, "template": "tk-2"}, {"from": 300.0, "to": 600.0, "template": "tk-1"}]}, C2])
    ok("Kıvrımda sık", {"corridor": "c-1"}, [{**C1, "curve": 5.0}, C2])
    no("Yol projesi değil", {"corridor": "c-1"}, result=NOT_ROAD, setup_=ccmd.setup(work="map"))
    no("Olmayan koridor", {"corridor": "yok"})
    no("Sıfır duyarlılık", {"corridor": "c-1", "sensitive": 0.0})
    no("Eksi duyarlılık", {"corridor": "c-1", "sensitive": -1.0})
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.corridors.daylights", "commandVersion": 1,
            "title": "Şev uçları",
            "note": "scripts/fixtures/road_corridor_daylights_command_cases.py yazar; istasyonlar koridorun, arazi, eksen "
                    "ve deverli tip kesit kesitlerin başvurusundan; şev ucu road_daylight_cases.py'nin tam kesirli "
                    "başvurusu. KentOS kodu yok.",
            "setup": ccmd.setup(), "cases": cases()}
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
            w = o.get("worst")
            print(f"  {c['name'][:38]:38} {o['counts']} en duyarlı {w and (w['kmText'], w['side'], round(w['sensitivity'], 2))} "
                  f"{[x['code'] for x in r.get('warnings', [])]}")
        else:
            print(f"  {c['name'][:38]:38} {r.get('status')} {r.get('error', {}).get('code', '')}")


if __name__ == "__main__":
    main()
