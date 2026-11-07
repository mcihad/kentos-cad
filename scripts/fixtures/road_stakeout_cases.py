#!/usr/bin/env python3
"""The shared cases of `road.stakeout.query` (plan/road.md Y10, docs/adr/0908; ROAD-SEC-09).

A corridor's coded section points by the references and no KentOS code: the crosses and their sections are
road_corridor_command_cases.py's (the alignment road_alignment_cases.py's at 40 digits, the template and the terrain
road_section_command_cases.py's, the codes road_corridor_cases.py's), a ground length the grid offset over the line's
scale (PROJ's point scales by Simpson) and the mean height's factor (the Euler radius in the line's azimuth), as
ground_survey_cases.py reads them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_stakeout_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import ground_cases as ground  # noqa: E402
import ground_survey_cases as gsurvey  # noqa: E402
import road_alignment_cases as aref  # noqa: E402
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.stakeout.query.json"

completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED = ccmd.UNCHANGED
MOST_STATIONS, MOST_POINTS = 20000, 200000
TM30 = ground.entry(5254)

# A corridor on a surface the project does not have: no section anywhere.
C3 = {**copy.deepcopy(ccmd.C1), "id": "c-3", "name": "Yüzeysiz koridor", "surface": "t-9",
      "regions": [{"from": 100.0, "to": 200.0, "template": "tk-2"}]}
CORRIDORS = ccmd.START + [C3]


def setup(work="road", height=None, srid=5254):
    base = ccmd.setup(CORRIDORS, work=work)
    if height is not None:
        base["settings"]["survey"] = {"groundHeight": height}
    if srid != 5254:
        base["settings"]["srid"] = srid
    return base


def num(v):
    return ccmd.num(v)


class Refused(Exception):
    def __init__(self, code, message, path):
        super().__init__(message)
        self.result = failed(code, message, path)


def bad(path, message):
    return Refused("invalid_stakeout", message, path)


def template_of(tid):
    t = next(x for x in ccmd.TEMPLATES if x["id"] == tid)
    return {"left": t["left"], "right": t.get("right") or t["left"]}


def side_words(templates, word):
    sides = [t[word] for t in templates]
    n = max((len(s["components"]) for s in sides), default=0)
    out = [f"c{i}" for i in range(1, n + 1)]
    if any(s.get("ditch") for s in sides):
        out.append("ditchToe")
    if any(s.get("ditch") and s["ditch"]["bottom"] > 0 for s in sides):
        out.append("ditchBottom")
    return out + ["hinge", "daylight"]


def cross_at(c, r, s):
    """The cross at station `s` of corridor `c`'s region `r`, as road_corridor_command_cases.make builds one."""
    b = ccmd.pcmd.ALIGNED[c["alignment"]]
    sup = c.get("superelevation")
    if sup:
        d = next(x for x in ccmd.SUPERS if x["id"] == sup)
        if ccmd.supcmd.build(d)[1]:
            sup = None
    fake = {"name": c["name"], "alignment": c["alignment"], "surface": c["surface"], "profile": c["profile"],
            "template": r["template"], "superelevation": sup, "left": c["left"], "right": c["right"]}
    row = ccmd.scmd.sections(fake, [s], ccmd.TEMPLATES)[0][0]
    e, (x, y) = aref.place_at(b.els, mp.mpf(s))
    th = e.theta(mp.mpf(s) - e.station)
    sec = None
    if "left" in row:
        def side(sd):
            return {"points": sd["points"], "hinge": sd["hinge"], "daylight": sd.get("daylight"), "kind": sd["kind"]}
        sec = {"axis": row["axis"], "left": side(row["left"]), "right": side(row["right"])}
    return {"station": s, "at": [float(x), float(y)], "direction": float(th), "section": sec}


def stepped(lo, hi, every):
    out = [lo]
    k = math.floor(lo / every)
    while True:
        s = k * every
        if s >= hi - 1e-9:
            break
        if s > lo + 1e-9:
            out.append(s)
        k += 1.0
    if hi > lo + 1e-9:
        out.append(hi)
    return out


def ground_offset(a, p, offset, height):
    if offset == 0.0:
        return 0.0
    k, hf = gsurvey.factors(TM30, height, a, p)
    return offset / (k * hf)


def sources(c, road):
    """Docs/adr/1038: what the corridor's stakeout is computed from, each as `road` has it now: the corridor's chain
    (scripts/fixtures/road_corridor_command_cases.py's `corridor_sources`, the surface's digest of now with docs/adr/1040)
    and its earthwork volumes."""
    out = ccmd.corridor_sources(c, road)
    volumes = [{"id": v["id"], "revision": v.get("revision", 0), "approved": "approved" in v}
               for v in road.get("volumes", []) if v["corridor"] == c["id"]]
    if volumes:
        out["volumes"] = volumes
    return out


def table(c, input_, height=None, road=None):
    """The answer and the warnings, or Refused."""
    _report, warnings, _tri, regs = ccmd.make(c)
    first = min(r["from"] for r in c["regions"])
    last = max(r["to"] for r in c["regions"])
    lo_all = input_.get("from", first)
    hi_all = input_.get("to", last)
    span = f"koridor {num(first)} ile {num(last)} m arası"
    if lo_all < first - 1e-9 or lo_all > last + 1e-9:
        raise bad("from", f"{num(lo_all)} m koridorun üstünde değil: {span}.")
    if hi_all < first - 1e-9 or hi_all > last + 1e-9:
        raise bad("to", f"{num(hi_all)} m koridorun üstünde değil: {span}.")
    if hi_all < lo_all:
        raise bad("to", "Aralığın sonu başından önce olamaz.")
    every = input_.get("step")
    if every is not None and not 0.1 <= every <= 10000.0:
        raise bad("step", "Aralık 0,1 ile 10 000 m arası olmalı.")
    templates = [template_of(r["template"]) for r in c["regions"]]
    left, right = side_words(templates, "left"), side_words(templates, "right")
    wanted = input_.get("codes", [])
    for i, q in enumerate(wanted):
        known = q == "axis" or q in left or q in right \
            or (q.startswith("left.") and q[5:] in left) or (q.startswith("right.") and q[6:] in right)
        if not known:
            allc = ["axis"] + [f"left.{w}" for w in left] + [f"right.{w}" for w in right]
            raise bad(f"codes[{i}]", f"“{q}” kodunu koridorun hiçbir tip kesiti vermez; kodlar: {', '.join(allc)}.")

    def keep(code):
        return not wanted or any(code == q or (code.endswith(q) and code[: -len(q)].endswith(".")) for q in wanted)

    crosses = []
    for ri, (reg, r) in enumerate(zip(regs, c["regions"])):
        if every is None:
            crosses += [(ri, x) for x in reg["crosses"] if lo_all - 1e-9 <= x["station"] <= hi_all + 1e-9]
        else:
            lo, hi = max(lo_all, r["from"]), min(hi_all, r["to"])
            if lo > hi + 1e-9:
                continue
            got = stepped(lo, hi, every)
            if len(crosses) + len(got) > MOST_STATIONS:
                raise bad("step", f"Bu aralıkla {MOST_STATIONS}'den çok km çıkar; step'i büyütün.")
            crosses += [(ri, cross_at(c, r, s)) for s in got]
    if input_.get("distances") == "ground":
        if height is None:
            raise Refused("stakeout_no_ground",
                          "Zemin boyu için projenin ortalama elipsoit yüksekliği yazılmalı (Proje ayarları › Ölçme).",
                          "distances")
    rows, empty = [], []
    for ri, x in crosses:
        s = x["station"]
        sec = x["section"]
        if sec is None:
            empty.append(s)
            continue
        codes = ccmd.template_codes(templates[ri])
        pts = [("axis", 0.0, sec["axis"])]
        for word in ("left", "right"):
            pts += cref.coded(codes, sec[word], word)
        pts.sort(key=lambda p: p[1])
        for code, o, z in pts:
            if not keep(code):
                continue
            at = cref.place(x, o)
            side = code.split(".")[0] if "." in code else "axis"
            row = {"station": s, "kmText": ccmd.km_text(c["alignment"], s), "region": ri, "code": code,
                   "side": side, "offset": o, "x": at[0], "y": at[1], "z": z}
            if input_.get("distances") == "ground":
                row["groundOffset"] = ground_offset(x["at"], at, o, height)
            rows.append(row)
    if empty:
        shown = ", ".join(f"km {ccmd.km_text(c['alignment'], s)}" for s in empty[:8])
        more = f" ve {len(empty) - 8} km daha" if len(empty) > 8 else ""
        warnings.append({"code": "stakeout_unsolved",
                         "message": f"“{c['name']}” koridorunun aplikasyon cetvelinde kesitsiz km'ler: {shown}{more}; "
                                    "oralarda nokta yok.", "path": "corridor"})
    if input_.get("distances") == "ground":
        method = (f"Ötelemeler plan boyu; zemin boyu her ötelemenin eksenden noktaya çizginin ölçeği ve {num(height)} m "
                  "ortalama elipsoit yüksekliğinin çarpanına bölünmesiyle.")
    else:
        method = "Ötelemeler plan (ızgara) boyu."
    out = {"corridor": c["id"], "name": c["name"], "distances": input_.get("distances", "grid"), "method": method,
           "stations": len(crosses), "rows": rows,
           "sources": sources(c, road if road is not None else setup()["settings"]["road"])}
    return out, warnings


def by(name):
    return next(c for c in CORRIDORS if c["id"] == name or c["name"] == name)


def ok(name, input_, height=None, op="execute", within=None):
    out, warnings = table(by(input_["corridor"]), input_, height)
    result = completed(None if op == "validate" else out)
    result["warnings"] = warnings
    s = step(op, input_, result, UNCHANGED)
    if within is not None:
        s["within"] = within
    return case(name, [s], None if height is None else setup(height=height))


def no(name, input_, height=None, setup_=None):
    try:
        table(by(input_["corridor"]), input_, height)
    except Refused as r:
        return case(name, [step("execute", input_, r.result, UNCHANGED)], setup_)
    raise AssertionError(f"{name}: refuses")


def with_volume():
    """Docs/adr/1038: the drawing with an earthwork volume of the main corridor at its second revision."""
    d = setup()
    d["settings"]["road"]["volumes"] = [{"id": "v-1", "name": "Ana kübaj", "corridor": "c-1", "method": "endArea",
                                         "revision": 2}]
    return d


def cases():
    vol = with_volume()
    q = {"corridor": "c-1", "from": 100.0, "to": 120.0}
    out_v, warns_v = table(by("c-1"), q, road=vol["settings"]["road"])
    res_v = completed(out_v)
    res_v["warnings"] = warns_v
    out = [case("Kaynakları: koridorun kübajı ikinci sürümünde", [step("execute", q, res_v, UNCHANGED)], vol),

        ok("Ana koridorun km 100–160'ı, koridorun kendi km'leri, bütün kodlar",
           {"corridor": "Ana koridor", "from": 100.0, "to": 160.0}),
        ok("Kimlikle ve yalnız doğrulama", {"corridor": "c-1", "from": 100.0, "to": 120.0}, op="validate"),
        ok("Bölge sınırında her 10 m: km 300 iki bölgede de",
           {"corridor": "Ana koridor", "from": 280.0, "to": 320.0, "step": 10.0}),
        ok("Bütün koridor her 100 m, yalnız şev ayağı (iki yan)", {"corridor": "Ana koridor", "step": 100.0,
                                                                 "codes": ["daylight"]}),
        ok("Eksen ve sol ilk bileşen her 50 m", {"corridor": "Ana koridor", "step": 50.0, "codes": ["axis", "left.c1"]}),
        ok("Hendek tabanı ve şev başı, tip 1'in bölgesinde", {"corridor": "Ana koridor", "from": 0.0, "to": 300.0,
                                                            "step": 75.0, "codes": ["ditchBottom", "ditchToe", "hinge"]}),
        ok("Sağ yanın bütün noktaları kesirli aralıkla", {"corridor": "Ana koridor", "from": 410.0, "to": 430.0,
                                                         "step": 7.5, "codes": ["right.c1", "right.c2", "right.c3",
                                                                                "right.hinge", "right.daylight"]}),
        ok("Deverli yan koridor her 25 m", {"corridor": "Yan koridor", "from": 100.0, "to": 200.0, "step": 25.0}),
        ok("Yan koridorun kendi km'leri, şev noktaları", {"corridor": "c-2", "codes": ["hinge", "daylight"]}),
        ok("Tek km: aralığın başı ve sonu aynı", {"corridor": "Ana koridor", "from": 250.0, "to": 250.0, "step": 10.0}),
        ok("Zemin boyu: 1250 m yükseklikle her 100 m, şev noktaları",
           {"corridor": "Ana koridor", "step": 100.0, "codes": ["axis", "hinge", "daylight"], "distances": "ground"},
           height=1250.0, within=1e-6),
        ok("Zemin boyu: deniz düzeyinde, yan koridor", {"corridor": "Yan koridor", "from": 300.0, "to": 400.0,
                                                       "step": 50.0, "distances": "ground"}, height=0.0, within=1e-6),
        ok("Yüzeysiz koridor: kesitsiz km'ler uyarıyla, nokta yok", {"corridor": "Yüzeysiz koridor", "step": 10.0}),
        ok("Kesitsiz km'ler sekizi aşınca", {"corridor": "c-3", "step": 5.0}),
        case("Yol projesi değil", [step("execute", {"corridor": "Ana koridor"}, ccmd.NOT_ROAD, UNCHANGED)],
             setup(work="map")),
        case("Olmayan koridor", [step("execute", {"corridor": "yok"},
                                      failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.",
                                             "corridor"), UNCHANGED)]),
        no("Aralığın başı koridordan önce", {"corridor": "Yan koridor", "from": 10.0}),
        no("Aralığın sonu koridorun ötesinde", {"corridor": "Ana koridor", "to": 700.0}),
        no("Aralık ters", {"corridor": "Ana koridor", "from": 200.0, "to": 100.0}),
        no("Çok küçük aralık", {"corridor": "Ana koridor", "step": 0.05}),
        no("Çok büyük aralık", {"corridor": "Ana koridor", "step": 20000.0}),
        no("Tip kesitlerin vermediği kod", {"corridor": "Ana koridor", "codes": ["axis", "kerb"]}),
        no("Yan koridorda hendek yok", {"corridor": "Yan koridor", "codes": ["left.ditchToe"]}),
        no("Sağda dördüncü bileşen yok", {"corridor": "Ana koridor", "codes": ["right.c4"]}),
        no("Zemin boyu yüksekliksiz", {"corridor": "Ana koridor", "distances": "ground"}),
    ]
    if any(x["name"] == "Sağda dördüncü bileşen yok" for x in out):
        right = side_words([template_of(r["template"]) for r in ccmd.C1["regions"]], "right")
        if "c4" in right:
            out = [x for x in out if x["name"] != "Sağda dördüncü bileşen yok"]
    return out


def main():
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.stakeout.query", "commandVersion": 1,
            "title": "Aplikasyon cetveli",
            "note": "scripts/fixtures/road_stakeout_cases.py yazar; kesitler road_corridor_command_cases.py'nin "
                    "başvurusundan (aks road_alignment_cases.py, tip kesit ve arazi road_section_command_cases.py, "
                    "kodlar road_corridor_cases.py), zemin boyu PROJ'un nokta ölçekleri ve Euler yarıçapıyla "
                    "(ground_survey_cases.py); KentOS kodu yok. Okur, yazmaz.",
            "setup": setup(), "cases": cases()}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if OUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUT} güncel değil: python3 {Path(__file__).name}")
        print(f"{OUT.name}: {len(data['cases'])} durum güncel")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.name}: {len(data['cases'])} durum")


if __name__ == "__main__":
    main()
