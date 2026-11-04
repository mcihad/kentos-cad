#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.{templates.edit,sections.edit,sections.query}.json: the shared cases of KentOS Yol's
cross section commands (plan/road.md Y5, docs/adr/0903), their expectations built here without KentOS code: the drawing,
the alignments and the profiles are scripts/fixtures/road_profile_command_cases.py's (the alignment mpmath at 40 digits,
the vertical alignment exact fractions, the surface Shewchuk's Triangle's); a sample line's terrain is
scripts/fixtures/road_section_cases.py's (every triangle edge cut at 40 digits), a template's platform, slopes, daylight
and areas scripts/fixtures/road_template_cases.py's (exact fractions); where a line crosses the alignment's first
tangent is two lines' crossing in fractions. The desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_section_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_pavement_cases as pvref  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_cases as sref  # noqa: E402
import road_section_stations_cases as stref  # noqa: E402
import road_superelevation_cases as supref  # noqa: E402
import road_template_cases as tref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
SAME = 1e-3

completed, failed, step = pcmd.completed, pcmd.failed, pcmd.step
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Enkesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
U, fmt, case = pcmd.U, pcmd.fmt, pcmd.case
ALIGNED, REV, TIN = pcmd.ALIGNED, pcmd.REV, pcmd.TIN
PROFILES = [pcmd.MAIN, pcmd.SIDE, pcmd.PART]
BY_PROFILE = {p["id"]: p for p in PROFILES}


# ── The drawing: road_profile_command_cases.py's, with lines on `0` crossing the main alignment's first tangent ───────

def line(n, a, b):
    return {"kind": "line", "id": n, "layerId": "0", "attrs": {}, "a": {"x": a[0], "y": a[1]}, "b": {"x": b[0], "y": b[1]}}


def poly(n, pts):
    return {"kind": "polyline", "id": n, "layerId": "0", "attrs": {}, "pts": [{"x": x, "y": y} for x, y in pts]}


LINES = [line(121, (100, 30), (110, 90)), poly(122, [(60, 10), (70, 20), (80, 90)]), line(123, (100, 10), (200, 20))]
CROSSING_LINE, CROSSING_POLY, MISSING_LINE = U(121), U(122), U(123)
POINT_UID = U(5)


def comp(kind, w, s):
    return {"kind": kind, "width": w, "slope": s}


ROADWAY = [comp("lane", 3.5, -2.5), comp("shoulder", 1.0, -4.0)]
TK1 = {"id": "tk-1", "name": "Tip 1", "revision": 1,
       "left": {"components": ROADWAY, "cut": 1.0, "fill": 1.5, "ditch": {"fore": 2.0, "depth": 0.5, "bottom": 0.5}}}
TK2 = {"id": "tk-2", "name": "Tip 2", "revision": 2,
       "left": {"components": [comp("lane", 3.0, -2.0), comp("shoulder", 1.5, -4.0)], "cut": 1.5, "fill": 2.0},
       "right": {"components": [comp("lane", 3.5, -2.5), comp("lane", 3.5, -2.5), comp("shoulder", 1.0, -4.0)],
                 "cut": 1.0, "fill": 1.5}}
def layer(name, thickness, material=None):
    out = {"name": name, "thickness": thickness}
    if material:
        out["material"] = material
    return out


# KGM'nin bitümlü sıcak karışım üstyapısı gibi dört tabaka; kalınlıklar örnektir, standart değildir (docs/adr/0930).
PAVE = {"layers": [layer("Aşınma", 0.05, "KGM-a"), layer("Binder", 0.08), layer("Plentmiks temel", 0.2),
                   layer("Alttemel", 0.25)], "edge": {"kind": "slope", "h": 1.5}}
PAVE_V = {"layers": [layer("Beton", 0.25), layer("Alttemel", 0.3)], "edge": {"kind": "vertical"}}
S1 = {"id": "k-1", "name": "Enkesitler", "alignment": "g-1", "alignmentRevision": 1, "revision": 1, "profile": "b-1",
      "template": "tk-1", "surface": "t-1", "left": 25.0, "right": 25.0,
      "stations": {"method": "range", "from": 100.0, "to": 300.0, "every": 50.0, "geometry": True}}
S2 = {"id": "k-2", "name": "Yan kesitler", "alignment": "g-2", "alignmentRevision": 2, "revision": 3, "profile": "b-2",
      "template": "tk-2", "surface": "t-1", "left": 20.0, "right": 30.0,
      "stations": {"method": "stations", "stations": [50.0, 200.0, 400.0]}}


# The superelevation schedules (docs/adr/0904): the main alignment's from its curve, the side one's from rows.
D1 = {"id": "d-1", "name": "Ana aks deveri", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
      "normalLeft": -2.5, "normalRight": -2.5,
      "method": {"method": "curves", "curves": [{"pi": "p1", "rate": 6.0, "runoff": 40.0, "onTangent": 0.0,
                                                 "widening": 0.5}]}}
D2 = {"id": "d-2", "name": "Yan aks deveri", "alignment": "g-2", "alignmentRevision": 2, "revision": 2,
      "normalLeft": -2.0, "normalRight": -2.0,
      "method": {"method": "stations", "rows": [{"station": 100.0, "left": -2.0, "right": -2.0},
                                                {"station": 200.0, "left": 3.0, "right": -3.0},
                                                {"station": 400.0, "left": 3.0, "right": -3.0},
                                                {"station": 500.0, "left": -2.0, "right": -2.0}]}}
SUPERS = [D1, D2]
# A schedule on a PI without a curve: it does not build.
D_BAD = {"id": "d-3", "name": "Kurulamaz dever", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
         "normalLeft": -2.5, "normalRight": -2.5,
         "method": {"method": "curves", "curves": [{"pi": "p9", "rate": 6.0, "runoff": 40.0, "onTangent": 0.0}]}}


def road_of(templates, sets, surfaces=pcmd.SURFACES, supers=None):
    if templates is None:
        return None
    out = {"surfaces": copy.deepcopy(surfaces), "alignments": copy.deepcopy(pcmd.ALIGNMENTS),
           "profiles": copy.deepcopy(PROFILES)}
    if templates:
        out["templates"] = copy.deepcopy(templates)
    if sets:
        out["sections"] = copy.deepcopy(sets)
    supers = SUPERS if supers is None else supers
    if supers:
        out["superelevations"] = copy.deepcopy(supers)
    return out


def super_schedule(d):
    """A schedule by the superelevation reference: its sides' break points (exact fractions) and the normal slopes."""
    if d["method"]["method"] == "stations":
        rows = d["method"]["rows"]
        sch = {"left": [[F(r["station"]), F(r["left"])] for r in rows],
               "right": [[F(r["station"]), F(r["right"])] for r in rows], "leftWidening": [], "rightWidening": []}
        return sch
    pis = [pcmd.acmd.full(p) for p in next(a for a in pcmd.ALIGNMENTS if a["id"] == d["alignment"])["pis"]]
    got = supref.curves(pis, [d["normalLeft"], d["normalRight"]],
                        [{"widening": 0.0, **c} for c in d["method"]["curves"]])
    return got["schedule"]


def superelevated(template, d, s):
    sch = super_schedule(d)
    s = F(s)
    def side(sd, key, wide):
        slope = supref.along(sch[key], s)
        slope = F(d["normalLeft" if key == "left" else "normalRight"]) if slope is None else slope
        w = supref.along(sch[wide], s) or F(0)
        comps = [dict(c) for c in sd["components"]]
        last = max((i for i, c in enumerate(comps) if c["kind"] == "lane"), default=None)
        for i, c in enumerate(comps):
            if c["kind"] == "lane":
                c["slope"] = slope
                if i == last:
                    c["width"] = F(c["width"]) + w
        return {**sd, "components": comps}
    return {"left": side(template["left"], "left", "leftWidening"), "right": side(template["right"], "right", "rightWidening")}


def setup(templates=(TK1, TK2), sets=(S1, S2), work="road", surfaces=pcmd.SURFACES, layers=None, supers=None):
    base = pcmd.setup(PROFILES, work, surfaces, layers)
    base["name"] = "Yol enkesitleri"
    base["entities"] += copy.deepcopy(LINES)
    base["uids"] += [U(e["id"]) for e in LINES]
    if work == "road":
        base["settings"]["road"] = road_of(list(templates), list(sets), surfaces, supers)
    return base


# ── Stations, by the references ───────────────────────────────────────────────────────────────────────────────────

class Refused(Exception):
    def __init__(self, result):
        self.result = result


def length(alignment):
    return float(ALIGNED[alignment].length)


def tangent_crossing(a, b):
    """Where the segment a–b crosses the main alignment's first tangent (from its first PI toward its second): the
    station (the distance from the first PI) or None."""
    (px, py), (qx, qy) = (F(20), F(50)), (F(300), F(60))
    dx, dy = qx - px, qy - py
    ax, ay, bx, by = (F(v) for v in (*a, *b))
    side = lambda x, y: dx * (y - py) - dy * (x - px)  # noqa: E731
    sa, sb = side(ax, ay), side(bx, by)
    if sa == 0:
        t = F(0)
    elif (sa < 0) == (sb < 0) and sb != 0:
        return None
    else:
        t = sa / (sa - sb)
    x, y = ax + t * (bx - ax), ay + t * (by - ay)
    return float(mp.sqrt(mp.mpf((x - px) ** 2 + (y - py) ** 2)))


def path_of(uid):
    for e in LINES:
        if U(e["id"]) == uid:
            if e["kind"] == "line":
                return [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
            return [(p["x"], p["y"]) for p in e["pts"]]
    return None


def stations_of(d, profiles=BY_PROFILE):
    b = ALIGNED[d["alignment"]]
    L = length(d["alignment"])
    st = d["stations"]
    off = lambda path, words: Refused(failed("invalid_section_set", words, path))  # noqa: E731
    if st["method"] == "breaks":
        # Docs/adr/0933 §3: the centreline's ground breaks (the terrain profile's crossings), simplified on height.
        lo, hi = st["from"], st["to"]
        if lo < -1e-9 or hi > L + 1e-9:
            raise off("stations.from", f"Km aralığı aksın üstünde olmalı: aks 0 ile {L:.3f} m arası.")
        if d.get("surface") != "t-1":
            raise off("surface", "Arazi kırıklarında kesit için kesit takımının yüzeyi kurulabilmeli.")
        prof = [(g["station"], g["z"]) for g in pcmd.ground(d["alignment"])
                if g["crossing"] and "z" in g and lo - 1e-9 <= g["station"] <= hi + 1e-9]
        for at in (lo, hi):
            z = pcmd.black_at(b, at)
            if z is not None:
                prof.append((at, z))
        prof.sort()
        uniq = []
        for p_ in prof:
            if not uniq or p_[0] - uniq[-1][0] > 1e-9:
                uniq.append(p_)
        kept = stref.douglas([(F(a), F(z)) for a, z in uniq], st["tolerance"])
        if st.get("most"):
            kept = stref.gaps([(F(a), F(z)) for a, z in uniq], kept, st["most"])
        out = [uniq[k][0] for k in kept] + [lo, hi]
        if st.get("geometry"):
            out += [float(e.station) for e in b.els if lo < float(e.station) < hi]
            p = profiles.get(d.get("profile"))
            if p:
                out += [v["station"] for v in p["pvis"] if lo < v["station"] < hi]
    elif st["method"] == "range":
        lo, hi, every = st["from"], st["to"], st["every"]
        if lo < -1e-9:
            raise off("stations.from", f"Km aralığı aksın üstünde olmalı: aks 0 ile {L:.3f} m arası.")
        if hi > L + 1e-9:
            raise off("stations.to", f"Km aralığı aksın üstünde olmalı: aks 0 ile {L:.3f} m arası.")
        out = [s for s in pcmd.ticks(b, every) if lo - 1e-9 <= s <= hi + 1e-9] + [lo, hi]
        if st.get("geometry"):
            out += [float(e.station) for e in b.els if lo < float(e.station) < hi]
            p = profiles.get(d.get("profile"))
            if p:
                out += [v["station"] for v in p["pvis"] if lo < v["station"] < hi]
        if st.get("curve"):
            # On the curves (TS to ST) every round `curve` metres too.
            spans = [(float(r["stations"][0]), float(r["stations"][3])) for r in b.reports]
            out += [x for x in pcmd.ticks(b, st["curve"])
                    if lo - 1e-9 <= x <= hi + 1e-9 and any(a - 1e-9 <= x <= z + 1e-9 for a, z in spans)]
    elif st["method"] == "stations":
        for i, s in enumerate(st["stations"]):
            if s < -1e-9 or s > L + 1e-9:
                raise off(f"stations.stations[{i}]", f"{fmt(s)} m aksın üstünde değil: aks 0 ile {L:.3f} m arası.")
        out = list(st["stations"])
    else:
        out = []
        for i, uid in enumerate(st["uids"]):
            path = f"stations.uids[{i}]"
            if uid == POINT_UID:
                raise off(path, f"“{uid}” bir çizgi ya da çoklu çizgi değil.")
            pts = path_of(uid)
            if pts is None:
                raise off(path, f"“{uid}” kimlikli nesne çizimde yok.")
            s = next((c for c in (tangent_crossing(a, q) for a, q in zip(pts, pts[1:])) if c is not None), None)
            if s is None:
                raise off(path, f"“{uid}” aksı kesmiyor.")
            out.append(s)
    out.sort()
    kept = []
    for s in out:
        if not kept or s - kept[-1] >= SAME:
            kept.append(min(max(s, 0.0), L))
    if not kept:
        raise off("stations", "Kesit takımının hiç km'si yok.")
    return kept


def stale(d):
    return REV[d["alignment"]] != d.get("alignmentRevision", 0)


def widths_of(d, st):
    """Each station's widths (clipped where the set clips) and the pairs meeting on the full widths (docs/adr/0933)."""
    b = ALIGNED[d["alignment"]]
    full = [[d["left"], d["right"]]] * len(st)
    found = stref.meet(b.els, st, full) if len(st) > 1 else []
    widths = [[float(a), float(z)] for a, z in stref.clip(found, full)] if d.get("clip") else full
    return widths, found


def crossing_warning(d, st, found):
    if d.get("clip") or not found:
        return None
    c = found[0]
    return {"code": "section_lines_cross",
            "message": f"“{d['name']}” kesit takımında {len(found)} çift kesit kolu kurbun iç yanında kesişiyor (ilki "
                       f"{st[c['first']]:.2f} ve {st[c['second']]:.2f} m'de, {'sol' if c['left'] else 'sağ'} kolda "
                       f"{float(min(c['atFirst'], c['atSecond'])):.2f} m); kesitler aynı araziyi iki kez sayar. Kolları "
                       f"kırpın (clip) ya da genişliği azaltın.",
            "path": "set"}


def report(d):
    try:
        st = stations_of(d)
    except Refused:
        st = []
    out = {"id": d["id"], "name": d["name"], "revision": d.get("revision", 0), "alignment": d["alignment"],
           "stale": stale(d), "stations": st}
    if d.get("clip"):
        out["clip"] = True
    found = widths_of(d, st)[1] if st else []
    if found:
        out["crossings"] = [{"first": st[c["first"]], "second": st[c["second"]], "left": c["left"],
                             "atFirst": float(c["atFirst"]), "atSecond": float(c["atSecond"])} for c in found]
    for k in ("profile", "template", "surface", "superelevation"):
        if d.get(k):
            out[k] = d[k]
    return out


def listed_templates(templates, sets):
    return [{"id": t["id"], "name": t["name"], "revision": t.get("revision", 0),
             "usedBy": [s["id"] for s in sets if s.get("template") == t["id"]]} for t in templates]


# ── Sections, by the references ───────────────────────────────────────────────────────────────────────────────────

def side_row(s):
    out = {"kind": s["kind"], "hinge": [float(v) for v in s["hinge"]], "points": [[float(v) for v in p] for p in s["points"]]}
    if s["daylight"] is not None:
        out["daylight"] = [float(v) for v in s["daylight"]]
    return out


def sections(d, at, templates):
    """The rows at the stations `at` and the warnings they give."""
    b = ALIGNED[d["alignment"]]
    warnings = []
    surface = d.get("surface")
    surf_ok = surface == "t-1"
    if surface and not surf_ok:
        why = "“Kurulamaz” kurulamıyor" if surface == "t-2" else f"({surface}) projede yok"
        warnings.append({"code": "section_surface_gone",
                         "message": f"“{d['name']}” kesit takımının yüzeyi {why}; arazi kesiti verilmedi.", "path": "set"})
    prof = BY_PROFILE.get(d.get("profile")) if d.get("profile") else None
    ev = pcmd.solve(prof)[1] if prof else None
    t = next((x for x in templates if x["id"] == d.get("template")), None) if d.get("template") else None
    template = None if t is None else {"left": t["left"], "right": t.get("right") or t["left"]}
    sup = next((x for x in SUPERS + [D_BAD] if x["id"] == d.get("superelevation")), None) if d.get("superelevation") else None
    if sup is D_BAD:
        warnings.append({"code": "section_superelevation_gone",
                         "message": f"“{d['name']}” kesit takımının dever çizelgesi kurulamıyor (“p9” PI'sinde kurp yok); "
                                    "şeritler tip kesitin eğimiyle verildi.", "path": "set"})
        sup = None
    rows, open_, unpaved = [], [], []
    widths, found = widths_of(d, at)
    w = crossing_warning(d, at, found)
    if w:
        warnings.append(w)
    for k, s in enumerate(at):
        e, (x, y) = aref.place_at(b.els, mp.mpf(s))
        th = e.theta(mp.mpf(s) - e.station)
        rx, ry = mp.sin(th), -mp.cos(th)
        left, right = widths[k]
        ends = [[x - rx * left, y - ry * left], [x + rx * right, y + ry * right]]
        km = aref.km_of(b.regions, mp.mpf(s))
        row = {"station": float(s), "km": float(km), "kmText": pcmd.acmd.km_text(km),
               "ends": [[float(v) for v in p] for p in ends], "ground": []}
        ground = []
        if surf_ok:
            ground = sref.section(TIN, ends)
            for g in ground:
                g["offset"] -= left
            row["ground"] = [{"offset": g["offset"], **({"z": g["z"]} if g["z"] is not None else {}), "gap": g["gap"]}
                             for g in ground]
        red = ev(s) if ev else None
        if red is not None:
            row["axis"] = red[0]
        if template and red is not None and surf_ok:
            gr = [{"offset": g["offset"], "z": g["z"], "gap": g["gap"]} for g in ground]
            here = superelevated(template, sup, s) if sup else template
            # About an edge the axis rises over the profile grade (docs/adr/0931 §1).
            z = red[0]
            if sup and sup.get("pivot", "centerline") != "centerline":
                sch = super_schedule(sup)
                sl = supref.along(sch["left"], F(s)) if sch["left"] else F(sup["normalLeft"])
                sr = supref.along(sch["right"], F(s)) if sch["right"] else F(sup["normalRight"])
                lanes = lambda sd: sum(F(c["width"]) for c in sd["components"] if c["kind"] == "lane")  # noqa: E731
                held = {"leftEdge": True, "rightEdge": False, "insideEdge": sl <= sr, "outsideEdge": sl > sr}[sup["pivot"]]
                w, n, sl_now = (lanes(here["left"]), F(sup["normalLeft"]), sl) if held else (lanes(here["right"]), F(sup["normalRight"]), sr)
                z = float(F(red[0]) + w * (n - sl_now) / 100)
                row["axis"] = z
            lside = tref.build_side(here["left"], z, gr, False)
            rside = tref.build_side(here["right"], z, gr, True)
            cut, fill = tref.areas(list(reversed(lside["points"])) + rside["points"][1:], gr)
            row["left"], row["right"] = side_row(lside), side_row(rside)
            row["cut"], row["fill"] = float(cut), float(fill)
            if t.get("pavement"):
                pv = pvref.section_pavement(here, red[0], gr, t["pavement"], {"left": lside, "right": rside}, cut, fill)
                if pv is None:
                    unpaved.append(f"km {row['kmText']}")
                else:
                    row["pavement"] = pv
            for side, word in ((lside, "sol"), (rside, "sağ")):
                if side["kind"] == "open":
                    open_.append(f"km {row['kmText']} {word}")
        rows.append(row)
    if unpaved:
        warnings.append({"code": "pavement_unbuilt",
                         "message": "Şu kesitlerde üstyapı kurulamadı (kenar şevi son bileşenden yatık ya da bileşensiz "
                                    f"yan): {', '.join(unpaved)}.", "path": "set"})
    if open_:
        warnings.append({"code": "section_open",
                         "message": "Şu yanların şevi şerit genişliği içinde araziyle buluşmuyor: "
                                    f"{', '.join(open_)}. Şeridi genişletin.", "path": "set"})
    return rows, warnings


def stale_warning(d):
    return {"code": "section_set_stale",
            "message": f"“{d['name']}” kesit takımının aksı değişti (takım {d['alignmentRevision']}. revizyona göre "
                       f"kurulu, aks şimdi {REV[d['alignment']]}.); km'leri denetleyip aksa yeniden bağlayın.",
            "path": "set"}


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_section_command_cases.py yazar; aks, boykesit ve yüzey "
                    "road_profile_command_cases.py'nin, kesit hattının arazisi road_section_cases.py'nin (her üçgen "
                    "kenarı 40 basamakta), tip kesitin platformu, şevleri ve alanları road_template_cases.py'nin "
                    "(kesirler) başvurusundandır; KentOS kodu yok. `expect.road` projenin yol ayarlarıdır. Tip kesit ve "
                    "kesit takımı geri alma geçmişinde tutulan proje ayarlarıdır.",
            "setup": base, "cases": cases}


# ── road.templates.edit ────────────────────────────────────────────────────────────────────────────────────────────

def templates_cases():
    start, sets = [TK1, TK2], [S1, S2]
    cases = []

    def written(after, target):
        out = {"templates": listed_templates(after, sets), "revision": "$current"}
        if target:
            out["template"] = next(t for t in after if t["id"] == target)
        return out

    def ok(name, input_, after, target, op="execute", undo=False):
        writes = op == "execute" and after != start
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(after, sets)} if writes else {**UNCHANGED, "road": road_of(start, sets)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Tip kesit", "expect": {"road": road_of(start, sets), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None, before=start):
        road = None if before is None else road_of(before, sets)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def rule(words, path=None):
        return failed("invalid_road_template", f"Tip kesit kurallarına uymuyor: {words}.", path)

    def put(i, t):
        out = list(start)
        out[i] = t
        return out

    plain = {"components": [comp("lane", 3.25, -2.0)], "cut": 1.0, "fill": 1.5}
    ok("Yeni tip kesit: kimliği ilk boş tk-3, sağ yan solun aynısı",
       {"operation": "set", "name": "Tek şerit", "left": plain},
       start + [{"id": "tk-3", "name": "Tek şerit", "revision": 1, "left": plain}], "tk-3")
    ok("Kendi kimliğiyle", {"operation": "set", "template": "dar", "name": "Dar", "left": plain},
       start + [{"id": "dar", "name": "Dar", "revision": 1, "left": plain}], "dar")
    ok("Yeni tip kesit iki yanıyla", {"operation": "set", "name": "Asimetrik", "left": plain, "right": TK2["right"]},
       start + [{"id": "tk-3", "name": "Asimetrik", "revision": 1, "left": plain, "right": TK2["right"]}], "tk-3")
    ok("mirror verilen sağ yanı düşürür", {"operation": "set", "name": "Ayna", "left": plain, "right": TK2["right"],
                                            "mirror": True},
       start + [{"id": "tk-3", "name": "Ayna", "revision": 1, "left": plain}], "tk-3")
    vee = {"components": ROADWAY, "cut": 1.0, "fill": 1.5, "ditch": {"fore": 3.0, "depth": 0.6}}
    ok("V hendek: tabansız", {"operation": "set", "name": "V hendekli", "left": vee},
       start + [{"id": "tk-3", "name": "V hendekli", "revision": 1, "left": vee}], "tk-3")
    bare = {"cut": 1.0, "fill": 2.0}
    ok("Bileşensiz yan: şev eksenden başlar", {"operation": "set", "name": "Şevden", "left": bare},
       start + [{"id": "tk-3", "name": "Şevden", "revision": 1, "left": bare}], "tk-3")
    ok("Adı değişir", {"operation": "set", "template": "tk-1", "name": "Tip A"},
       put(0, {**TK1, "name": "Tip A", "revision": 2}), "tk-1")
    ok("Sol yan değişir", {"operation": "set", "template": "tk-1", "left": plain},
       put(0, {**TK1, "left": plain, "revision": 2}), "tk-1")
    ok("Sağ yan verilir", {"operation": "set", "template": "tk-1", "right": plain},
       put(0, {**TK1, "right": plain, "revision": 2}), "tk-1")
    ok("mirror sağ yanı kaldırır", {"operation": "set", "template": "tk-2", "mirror": True},
       put(1, {k: v for k, v in TK2.items() if k != "right"} | {"revision": 3}), "tk-2")
    ok("Adıyla, büyük küçük harf ayrımsız", {"operation": "set", "template": "TIP 2", "name": "Tip B"},
       put(1, {**TK2, "name": "Tip B", "revision": 3}), "tk-2")
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "template": "tk-1", "name": "Tip 1"}, start, "tk-1")
    # Removing tk-2 needs it unused: the side set without a template.
    sets_free = [S1, {k: v for k, v in S2.items() if k != "template"}]
    free = setup(sets=sets_free)
    cases.append(case("Kullanılmayan tip kesit silinir", [step(
        "execute", {"operation": "remove", "template": "tk-2"},
        completed({"templates": listed_templates([TK1], sets_free), "revision": "$current"}),
        {**STEPPED, "road": road_of([TK1], sets_free)}),
        {"op": "undo", "returns": "Tip kesit", "expect": {"road": road_of([TK1, TK2], sets_free), "canRedo": True}}],
        free))
    ok("Düzenleme tek geri alma adımı", {"operation": "set", "template": "tk-1", "name": "Tip A"},
       put(0, {**TK1, "name": "Tip A", "revision": 2}), "tk-1", undo=True)
    ok("Plan yazmaz", {"operation": "set", "name": "Tek şerit", "left": plain},
       start + [{"id": "tk-3", "name": "Tek şerit", "revision": 1, "left": plain}], "tk-3", op="plan")
    ok("Doğrulama yazmaz", {"operation": "set", "template": "tk-1", "name": "Tip A"},
       put(0, {**TK1, "name": "Tip A", "revision": 2}), "tk-1", op="validate")

    # The pavement (docs/adr/0930 §2): its layers from the top down and how they end; the thicknesses are examples.
    ok("Üstyapı yazılır: dört tabaka, kenar dolgu şevinde", {"operation": "set", "template": "tk-1", "pavement": PAVE},
       put(0, {**TK1, "pavement": PAVE, "revision": 2}), "tk-1")
    ok("Yeni tip kesit üstyapısıyla, dik kenar", {"operation": "set", "name": "Üstyapılı", "left": plain, "pavement": PAVE_V},
       start + [{"id": "tk-3", "name": "Üstyapılı", "revision": 1, "left": plain, "pavement": PAVE_V}], "tk-3")
    paved = [{**TK1, "pavement": PAVE}, TK2]
    cases.append(case("Üstyapı kaldırılır, tek geri alma adımı", [step(
        "execute", {"operation": "set", "template": "tk-1", "removePavement": True},
        completed({"templates": listed_templates([{**TK1, "revision": 2}, TK2], sets), "revision": "$current",
                   "template": {**TK1, "revision": 2}}),
        {**STEPPED, "road": road_of([{**TK1, "revision": 2}, TK2], sets)}),
        {"op": "undo", "returns": "Tip kesit", "expect": {"road": road_of(paved, sets), "canRedo": True}}],
        setup(templates=paved)))
    no("Üstyapı ile kaldırma birlikte", {"operation": "set", "template": "tk-1", "pavement": PAVE, "removePavement": True},
       failed("invalid_road_template", "pavement ile removePavement birlikte verilmez: üstyapı ya yazılır ya kaldırılır.",
              "removePavement"))
    no("Tabakasız üstyapı", {"operation": "set", "template": "tk-1", "pavement": {**PAVE, "layers": []}},
       rule("üstyapının 1 ile 12 arasında tabakası olur", "pavement.layers"))
    no("Sıfır kalınlıklı tabaka",
       {"operation": "set", "template": "tk-1", "pavement": {**PAVE, "layers": [layer("Aşınma", 0.05), layer("Binder", 0.0)]}},
       rule("tabakanın kalınlığı 0'dan büyük, en çok 2 m olmalı", "pavement.layers[1].thickness"))
    no("Adsız tabaka", {"operation": "set", "template": "tk-1", "pavement": {**PAVE, "layers": [layer("", 0.05)]}},
       rule("tabakanın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 60 karakter", "pavement.layers[0].name"))
    no("Üstyapı 5 m'den derin",
       {"operation": "set", "template": "tk-1", "pavement": {**PAVE, "layers": [layer("A", 2.0), layer("B", 2.0), layer("C", 1.5)]}},
       rule("üstyapının toplam kalınlığı en çok 5 m olur", "pavement.layers"))
    no("Sıfır kenar şevi", {"operation": "set", "template": "tk-1", "pavement": {**PAVE, "edge": {"kind": "slope", "h": 0.0}}},
       rule("kenar şevi 0'dan büyük, en çok 100 olmalı (yatay / 1 düşey)", "pavement.edge.h"))
    no("Yol projesi değil", {"operation": "remove", "template": "tk-1"}, NOT_ROAD, setup(work="map"), before=None)
    no("Olmayan tip kesit silinmez", {"operation": "remove", "template": "yok"},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "template"))
    no("Tip kesit yeniden bağlanmaz", {"operation": "rebind", "template": "tk-1"},
       failed("invalid_road_template", "Tip kesit yeniden bağlanmaz: rebind yalnız kesit takımınındır.", "operation"))
    no("Kullanılan tip kesit silinmez", {"operation": "remove", "template": "Tip 1"},
       failed("road_template_in_use", "“Tip 1” tip kesitini şu kesit takımları kullanıyor: Enkesitler. Önce "
                                      "takımlardan kaldırın.", "template"))
    no("Değiştirilecek bir şey yok", {"operation": "set", "template": "tk-1"},
       failed("invalid_road_template",
              "Değiştirilecek bir şey yok: name, left, right, mirror, pavement ya da removePavement verin."))
    no("Yeni tip kesitin adı yok", {"operation": "set", "left": plain},
       failed("invalid_road_template", "Yeni tip kesitin adı (name) ve sol yanı (left) verilmeli.", "name"))
    no("Yeni tip kesitin sol yanı yok", {"operation": "set", "name": "x"},
       failed("invalid_road_template", "Yeni tip kesitin adı (name) ve sol yanı (left) verilmeli.", "left"))
    no("Aynı ad", {"operation": "set", "name": "tip 2", "left": plain}, rule("bu adda başka bir tip kesit var", "name"))
    no("Boş ad", {"operation": "set", "template": "tk-1", "name": " Tip"},
       rule("tip kesitin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter", "name"))
    no("Sıfır genişlik", {"operation": "set", "name": "x", "left": {**plain, "components": [comp("lane", 0.0, -2.0)]}},
       rule("bileşenin genişliği 0'dan büyük olmalı", "left.components[0].width"))
    no("Yüzde 100'den dik enine eğim",
       {"operation": "set", "name": "x", "left": {**plain, "components": [comp("lane", 3.0, -2.0), comp("shoulder", 1.0, 150.0)]}},
       rule("enine eğim en çok %100 olur", "left.components[1].slope"))
    no("Sıfır yarma şevi", {"operation": "set", "name": "x", "left": {**plain, "cut": 0.0}},
       rule("yarma şevi 0'dan büyük olmalı (bir düşeye yatay)", "left.cut"))
    no("Sağ yanda eksi dolgu şevi", {"operation": "set", "name": "x", "left": plain, "right": {**plain, "fill": -1.5}},
       rule("dolgu şevi 0'dan büyük olmalı (bir düşeye yatay)", "right.fill"))
    no("Derinliksiz hendek",
       {"operation": "set", "name": "x", "left": {**plain, "ditch": {"fore": 2.0, "depth": 0.0}}},
       rule("hendeğin iç şevi ve derinliği 0'dan büyük, tabanı eksi olmamalı", "left.ditch"))
    no("Geçersiz beklenen sürüm", {"operation": "set", "template": "tk-1", "name": "Tip A", "expectedRevision": "on"},
       BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "set", "template": "tk-1", "name": "Tip A", "expectedRevision": "41"},
       CONFLICT)
    return file("road.templates.edit", "Tip kesiti düzenle", setup(), cases)


# ── road.sections.edit ─────────────────────────────────────────────────────────────────────────────────────────────

def sections_cases():
    start, templates = [S1, S2], [TK1, TK2]
    cases = []

    def written(after, target):
        out = {"sets": [report(d) for d in after], "revision": "$current"}
        if target:
            out["set"] = report(next(d for d in after if d["id"] == target))
        return out

    def ok(name, input_, after, target, op="execute", undo=False):
        writes = op == "execute" and after != start
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(templates, after)} if writes else {**UNCHANGED, "road": road_of(templates, start)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Kesit takımı", "expect": {"road": road_of(templates, start), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None, before=start):
        road = None if before is None else road_of(templates, before)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def rule(words, path=None):
        return failed("invalid_section_set", f"Kesit takımı kurallarına uymuyor: {words}.", path)

    def put(i, d):
        out = list(start)
        out[i] = d
        return out

    def new(id_, name, alignment, left, right, stations, **refs):
        d = {"id": id_, "name": name, "alignment": alignment, "alignmentRevision": REV[alignment], "revision": 1}
        d.update({k: v for k, v in refs.items() if v})
        d.update({"left": left, "right": right, "stations": stations})
        return d

    rng = {"method": "range", "from": 0.0, "to": 400.0, "every": 100.0}
    ok("Yeni takım, km aralığıyla: kimliği ilk boş k-3",
       {"operation": "set", "name": "Her 100 m", "alignment": "g-1", "left": 30.0, "right": 30.0, "stations": rng},
       start + [new("k-3", "Her 100 m", "g-1", 30.0, 30.0, rng)], "k-3")
    geo = {"method": "range", "from": 50.0, "to": 600.0, "every": 25.0, "geometry": True}
    ok("Kurp noktaları ve PVI'lerle; boykesit, tip kesit ve yüzey adlarıyla",
       {"operation": "set", "name": "Sık", "alignment": "ANA aks", "profile": "proje KOTU", "template": "tip 1",
        "surface": "Mevcut zemin", "left": 25.0, "right": 25.0, "stations": geo},
       start + [new("k-3", "Sık", "g-1", 25.0, 25.0, geo, profile="b-1", template="tk-1", surface="t-1")], "k-3")
    lst = {"method": "stations", "stations": [400.0, 120.0, 120.0004, 0.0]}
    ok("Yazılan km'ler sıralanır, 1 mm içindekiler bir",
       {"operation": "set", "set": "secili", "name": "Seçili", "alignment": "g-1", "left": 10.0, "right": 15.0,
        "stations": lst}, start + [new("secili", "Seçili", "g-1", 10.0, 15.0, lst)], "secili")
    lines = {"method": "lines", "uids": [CROSSING_LINE, CROSSING_POLY]}
    ok("Çizgilerin aksı kestiği yerler",
       {"operation": "set", "name": "Çizgilerden", "alignment": "g-1", "left": 20.0, "right": 20.0, "stations": lines},
       start + [new("k-3", "Çizgilerden", "g-1", 20.0, 20.0, lines)], "k-3")
    ok("Adı değişir", {"operation": "set", "set": "k-1", "name": "Ana enkesitler"},
       put(0, {**S1, "name": "Ana enkesitler", "revision": 2}), "k-1")
    ok("Şerit genişliği değişir", {"operation": "set", "set": "Enkesitler", "left": 40.0},
       put(0, {**S1, "left": 40.0, "revision": 2}), "k-1")
    ok("Boş boykesit boykesiti kaldırır", {"operation": "set", "set": "k-1", "profile": ""},
       put(0, {k: v for k, v in S1.items() if k != "profile"} | {"revision": 2}), "k-1")
    ok("Km'ler yazılanlarla değişir", {"operation": "set", "set": "k-1", "stations": lst},
       put(0, {**S1, "stations": lst, "revision": 2}), "k-1")
    ok("Başka aksa taşınır, aksın revizyonunu alır",
       {"operation": "set", "set": "k-1", "alignment": "g-2", "profile": "b-2"},
       put(0, {**S1, "alignment": "g-2", "alignmentRevision": 3, "profile": "b-2", "revision": 2}), "k-1")
    ok("Eskiyen takım aksa yeniden bağlanır", {"operation": "rebind", "set": "k-2"},
       put(1, {**S2, "alignmentRevision": 3, "revision": 4}), "k-2")
    ok("Güncel takımı bağlamak bir şey değiştirmez", {"operation": "rebind", "set": "k-1"}, start, "k-1")
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "set": "k-1", "left": 25.0}, start, "k-1")
    ok("Takım silinir", {"operation": "remove", "set": "Yan kesitler"}, [S1], None)
    ok("Silme tek geri alma adımı", {"operation": "remove", "set": "k-1"}, [S2], None, undo=True)
    ok("Plan yazmaz", {"operation": "set", "set": "k-1", "name": "Ana enkesitler"},
       put(0, {**S1, "name": "Ana enkesitler", "revision": 2}), "k-1", op="plan")
    ok("Doğrulama yazmaz", {"operation": "rebind", "set": "k-2"},
       put(1, {**S2, "alignmentRevision": 3, "revision": 4}), "k-2", op="validate")

    # Docs/adr/0933 §3: the terrain's breaks, a range's curve step, the clip.
    brk = {"method": "breaks", "from": 0.0, "to": 600.0, "tolerance": 0.1, "most": 20.0}
    ok("Arazi kırıklarında kesit, 0,1 m toleransla",
       {"operation": "set", "name": "Kırıklar", "alignment": "g-1", "surface": "t-1", "left": 25.0, "right": 25.0,
        "stations": brk}, start + [new("k-3", "Kırıklar", "g-1", 25.0, 25.0, brk, surface="t-1")], "k-3")
    crv = {"method": "range", "from": 0.0, "to": 600.0, "every": 50.0, "curve": 10.0}
    ok("Kurpta her 10 m, düzlükte her 50 m",
       {"operation": "set", "name": "Kurpta sık", "alignment": "g-1", "left": 25.0, "right": 25.0, "stations": crv},
       start + [new("k-3", "Kurpta sık", "g-1", 25.0, 25.0, crv)], "k-3")
    ok("Kesişen kollar kırpılır", {"operation": "set", "set": "k-1", "clip": True},
       put(0, {**S1, "clip": True, "revision": 2}), "k-1")
    no("Yüzeysiz arazi kırıkları", {"operation": "set", "name": "x", "alignment": "g-1", "left": 10.0, "right": 10.0,
                                    "stations": brk}, rule("arazi kırıklarında kesit bir yüzey ister", "surface"))
    no("Eksi tolerans", {"operation": "set", "set": "k-1", "stations": {**brk, "tolerance": -0.1}},
       rule("kot toleransı eksi olmayan bir sayı olmalı", "stations.tolerance"))
    no("Sıfır en büyük aralık", {"operation": "set", "set": "k-1", "stations": {**brk, "most": 0.0}},
       rule("en büyük aralık 0'dan büyük olmalı", "stations.most"))
    no("Sıfır kurp aralığı", {"operation": "set", "set": "k-1", "stations": {**crv, "curve": 0.0}},
       rule("kurptaki aralık 0'dan büyük olmalı", "stations.curve"))

    ok("Takıma dever çizelgesi adıyla bağlanır", {"operation": "set", "set": "k-1", "superelevation": "ana AKS deveri"},
       put(0, {**S1, "superelevation": "d-1", "revision": 2}), "k-1")
    L = length("g-1")
    no("Olmayan dever çizelgesi", {"operation": "set", "set": "k-1", "superelevation": "yok"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Dever çizelgesi takımın aksında değil", {"operation": "set", "set": "k-1", "superelevation": "d-2"},
       rule("dever çizelgesi kesit takımının aksında değil", "superelevation"))
    no("Yol projesi değil", {"operation": "remove", "set": "k-1"}, NOT_ROAD, setup(work="map"), before=None)
    no("Olmayan takım silinmez", {"operation": "remove", "set": "yok"},
       failed("unknown_section_set", "“yok” kimlikli ya da adlı kesit takımı projede yok.", "set"))
    no("Olmayan aks", {"operation": "set", "name": "x", "alignment": "yok", "left": 10.0, "right": 10.0, "stations": rng},
       failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Olmayan boykesit", {"operation": "set", "set": "k-1", "profile": "yok"},
       failed("unknown_road_profile", "“yok” kimlikli ya da adlı boykesit projede yok.", "profile"))
    no("Olmayan tip kesit", {"operation": "set", "set": "k-1", "template": "yok"},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "template"))
    no("Olmayan yüzey", {"operation": "set", "set": "k-1", "surface": "yok"},
       failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Yeni takımın km'leri yok", {"operation": "set", "name": "x", "alignment": "g-1", "left": 10.0, "right": 10.0},
       failed("invalid_section_set", "Yeni kesit takımı için name, alignment, left, right ve stations verilmeli.",
              "stations"))
    no("Değiştirilecek bir şey yok", {"operation": "set", "set": "k-1"},
       failed("invalid_section_set", "Değiştirilecek bir şey yok: name, alignment, profile, template, surface, superelevation, "
                                     "left, right, stations ya da clip verin."))
    no("Paralel aksa bağlanmaz",
       {"operation": "set", "name": "x", "alignment": "g-3", "left": 10.0, "right": 10.0, "stations": rng},
       rule("kesit takımı yalnız PI'larıyla kurulan aksa bağlanır", "alignment"))
    no("Boykesit takımın aksında değil", {"operation": "set", "set": "k-1", "profile": "b-2"},
       rule("boykesit kesit takımının aksında değil", "profile"))
    no("Aynı ad", {"operation": "set", "set": "k-1", "name": "YAN kesitler"}, rule("bu adda başka bir kesit takımı var", "name"))
    no("Sıfır şerit genişliği", {"operation": "set", "set": "k-1", "right": 0.0},
       rule("şerit genişliği 0'dan büyük, en çok 1000 m olmalı", "right"))
    no("Km aralığının başı sonundan büyük",
       {"operation": "set", "set": "k-1", "stations": {"method": "range", "from": 300.0, "to": 100.0, "every": 50.0}},
       rule("km aralığının başı sonundan küçük olmalı", "stations.from"))
    no("Km aralığı aksın sonundan taşıyor",
       {"operation": "set", "set": "k-1", "stations": {"method": "range", "from": 100.0, "to": 900.0, "every": 50.0}},
       failed("invalid_section_set", f"Km aralığı aksın üstünde olmalı: aks 0 ile {L:.3f} m arası.", "stations.to"))
    no("Aksın dışındaki km",
       {"operation": "set", "set": "k-1", "stations": {"method": "stations", "stations": [100.0, 900.5]}},
       failed("invalid_section_set", f"900.5 m aksın üstünde değil: aks 0 ile {L:.3f} m arası.", "stations.stations[1]"))
    no("Aksı kesmeyen çizgi",
       {"operation": "set", "set": "k-1", "stations": {"method": "lines", "uids": [MISSING_LINE]}},
       failed("invalid_section_set", f"“{MISSING_LINE}” aksı kesmiyor.", "stations.uids[0]"))
    no("Nokta çizgi değil",
       {"operation": "set", "set": "k-1", "stations": {"method": "lines", "uids": [CROSSING_LINE, POINT_UID]}},
       failed("invalid_section_set", f"“{POINT_UID}” bir çizgi ya da çoklu çizgi değil.", "stations.uids[1]"))
    gone = "0192f5a0-7c3e-7d4a-9b1e-0000000fffff"
    no("Çizimde olmayan çizgi", {"operation": "set", "set": "k-1", "stations": {"method": "lines", "uids": [gone]}},
       failed("invalid_section_set", f"“{gone}” kimlikli nesne çizimde yok.", "stations.uids[0]"))
    no("Geçersiz beklenen sürüm", {"operation": "remove", "set": "k-1", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "remove", "set": "k-1", "expectedRevision": "41"}, CONFLICT)
    return file("road.sections.edit", "Kesit takımını düzenle", setup(), cases)


# ── road.sections.query ────────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    templates, start = [TK1, TK2], [S1, S2]
    cases = []

    def q(name, input_, out, warns=(), setup_=None, sets=start, op="execute", supers=None):
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        road = road_of(templates, sets, supers=supers)
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road})], setup_))

    def no(name, input_, result, setup_=None, sets=start):
        road = None if sets is None else road_of(templates, sets)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def one(d, at=None):
        st = stations_of(d)
        rows, warns = sections(d, st if at is None else at, templates)
        pre = [stale_warning(d)] if stale(d) else []
        return {"sets": [report(d)], "sections": rows}, pre + warns

    def ask(name, d, input_, at=None, sets=None):
        out, warns = one(d, at)
        sets = sets or start
        q(name, input_, out, warns, None if sets is start else setup(sets=sets), sets)

    q("Bütün takımlar; eskiyen uyarılır", {}, {"sets": [report(d) for d in start], "sections": []}, [stale_warning(S2)])
    ask("Bir takımın bütün kesitleri: arazi, platform, şevler, alanlar", S1, {"set": "k-1"})
    ask("Takım adıyla, büyük küçük harf ayrımsız", S1, {"set": "ENKESITLER"})
    ask("Yalnız sorulan km'ler", S1, {"set": "k-1", "stations": [150.0, 250.0]}, at=[150.0, 250.0])
    ask("Eskiyen takım uyarılır, yine sorulur; asimetrik tip kesit", S2, {"set": "k-2"})
    nt = {k: v for k, v in S1.items() if k != "template"}
    ask("Tip kesitsiz takım: arazi ve eksen kotu", nt, {"set": "k-1", "stations": [200.0]}, at=[200.0], sets=[nt, S2])
    npf = {k: v for k, v in S1.items() if k != "profile"}
    ask("Boykesitsiz takım: yalnız arazi", npf, {"set": "k-1", "stations": [200.0]}, at=[200.0], sets=[npf, S2])
    broken = {**S1, "surface": "t-2"}
    ask("Kurulamayan yüzey: arazi yok, uyarı", broken, {"set": "k-1", "stations": [100.0]}, at=[100.0],
        sets=[broken, S2])
    missing = {**S1, "surface": "t-yok"}
    ask("Projede olmayan yüzey: arazi yok, uyarı", missing, {"set": "k-1", "stations": [100.0]}, at=[100.0],
        sets=[missing, S2])
    narrow = {**S1, "left": 4.0, "right": 6.0}
    ask("Dar şerit: şev araziyle buluşmuyor, yan açık", narrow, {"set": "k-1", "stations": [100.0, 200.0]},
        at=[100.0, 200.0], sets=[narrow, S2])
    lined = {**S1, "stations": {"method": "lines", "uids": [CROSSING_LINE, CROSSING_POLY]}}
    ask("Çizgilerin kestiği km'lerde", lined, {"set": "k-1"}, sets=[lined, S2])
    wide = {**S1, "left": 60.0, "right": 60.0, "stations": {"method": "stations", "stations": [30.0, 400.0, 600.0]}}
    ask("Geniş şerit yüzeyin dışına taşar", wide, {"set": "k-1"}, sets=[wide, S2])
    ends = {**S1, "stations": {"method": "stations", "stations": [0.0, length("g-1")]}}
    ask("Aksın başı ve sonu: boykesitin dışında platform yok", ends, {"set": "k-1"}, sets=[ends, S2])
    sup = {**S1, "superelevation": "d-1", "stations": {"method": "stations", "stations": [150.0, 194.0, 220.0, 260.0, 330.0]}}
    ask("Dever çizelgeli takım: şeritler o km'nin eğimini alır, iç yan genişler", sup, {"set": "k-1"}, sets=[sup, S2])
    # About the inside edge the axis rises (docs/adr/0931 §1): the schedule's pivot for this one case.
    SUPERS[0] = {**D1, "pivot": "insideEdge"}
    try:
        out, warns = one(sup)
        q("Dever iç kenar etrafında: eksen kalkar", {"set": "k-1"}, out, warns, setup(sets=[sup, S2]), [sup, S2])
    finally:
        SUPERS[0] = D1
    gone = {**S1, "superelevation": "d-3", "stations": {"method": "stations", "stations": [200.0]}}
    out, warns = one(gone)
    q("Kurulamayan dever çizelgesi: şeritler tip kesitin eğimiyle, uyarı", {"set": "k-1"}, out, warns,
      setup(sets=[gone, S2], supers=SUPERS + [D_BAD]), [gone, S2], supers=SUPERS + [D_BAD])
    rows = {**S2, "superelevation": "d-2"}
    ask("Km satırlarından dever, eskiyen takım", rows, {"set": "k-2"}, sets=[S1, rows])
    flat = {**S1, "template": "tk-2", "stations": {"method": "range", "from": 300.0, "to": 600.0, "every": 100.0}}
    ask("Hendeksiz tip kesit, aralığın her 100 m'si", flat, {"set": "k-1"}, sets=[flat, S2])
    # The template's pavement on the sections (docs/adr/0930 §2): the layers, the formation, Formasyon–Zemin.
    for name, pave, stations_, sup_ in (
        ("Üstyapılı tip kesit: tabakalar, formasyon ve Formasyon–Zemin alanları", PAVE, None, None),
        ("Dik kenarlı üstyapı", PAVE_V, [150.0, 250.0], None),
        ("Üstyapı deverle döner", PAVE, [150.0, 194.0, 220.0, 260.0], "d-1"),
        ("Banketten yatık kenar: üstyapı kurulmaz, uyarı", {**PAVE, "edge": {"kind": "slope", "h": 30.0}}, [200.0], None),
    ):
        tpls = [{**TK1, "pavement": pave}, TK2]
        d = {**S1, **({"superelevation": sup_, "stations": {"method": "stations", "stations": stations_}} if sup_ else {})}
        at = stations_of(d) if stations_ is None or sup_ else stations_
        rows_, warns_ = sections(d, at, tpls)
        input_ = {"set": "k-1", **({"stations": stations_} if stations_ and not sup_ else {})}
        cases.append(case(name, [step("execute", input_, completed({"sets": [report(d)], "sections": rows_}, warns_),
                                      {**UNCHANGED, "road": road_of(tpls, [d, S2], supers=SUPERS if sup_ else None)})],
                          setup(templates=tpls, sets=[d, S2], supers=SUPERS if sup_ else None)))
    out, warns = one(S1, [200.0])
    q("Plan cevabı verir", {"set": "k-1", "stations": [200.0]}, out, warns, op="plan")
    q("Doğrulama uyarıları verir", {"set": "k-2"}, None, one(S2)[1], op="validate")
    q("Takım yok: boş liste", {}, {"sets": [], "sections": []}, setup_=setup(sets=[]), sets=[])
    dead = {**S1, "id": "k-3", "name": "Kopuk", "stations": {"method": "lines", "uids": [MISSING_LINE]}}
    q("Km'si çıkmayan takım raporda km'siz, öbürleri yine", {}, {"sets": [report(d) for d in (S1, S2, dead)], "sections": []},
      [stale_warning(S2)], setup(sets=[S1, S2, dead]), [S1, S2, dead])

    no("Yol projesi değil", {}, NOT_ROAD, setup(work="map"), sets=None)
    no("Olmayan takım", {"set": "yok"}, failed("unknown_section_set", "“yok” kimlikli ya da adlı kesit takımı projede yok.", "set"))
    no("Takımın km'si olmayan soru", {"set": "k-1", "stations": [100.0, 120.0]},
       failed("invalid_places", "120 m “Enkesitler” kesit takımının km'lerinden biri değil.", "stations[1]"))
    no("Km'si çıkmayan takımın kesitleri sorulmaz", {"set": "Kopuk"},
       failed("invalid_section_set", f"“{MISSING_LINE}” aksı kesmiyor.", "stations.uids[0]"), setup(sets=[S1, S2, dead]),
       [S1, S2, dead])
    no("Takımsız km sorusu", {"stations": [100.0]},
       failed("invalid_places", "Km bir kesit takımında sorulur: takımın kimliğini ya da adını (set) verin.", "set"))
    # Docs/adr/0933: lines meeting on the curve's inside (p1 turns left, R 150 m), said or clipped; the terrain's
    # breaks; the curve step.
    inside = {**S1, "left": 180.0, "right": 20.0, "stations": {"method": "range", "from": 200.0, "to": 420.0, "every": 20.0}}
    ask("İç kurpta kollar kesişir, uyarılır", inside, {"set": "k-1"}, sets=[inside, S2])
    clipped = {**inside, "clip": True}
    ask("Kesişen kollar buluşma noktasında kırpılır", clipped, {"set": "k-1"}, sets=[clipped, S2])
    breaks = {**S1, "stations": {"method": "breaks", "from": 0.0, "to": 600.0, "tolerance": 0.1, "most": 20.0}}
    ask("Arazi kırıklarında kesit", breaks, {"set": "k-1"}, sets=[breaks, S2])
    exact = {**S1, "stations": {"method": "breaks", "from": 100.0, "to": 300.0, "tolerance": 0.0, "geometry": True}}
    ask("Sıfır toleransla her kırıkta, kurp noktaları ve PVI'lerle", exact, {"set": "k-1"}, sets=[exact, S2])
    curvy = {**S1, "stations": {"method": "range", "from": 0.0, "to": 600.0, "every": 100.0, "curve": 20.0}}
    ask("Kurpta sıklaştırma", curvy, {"set": "k-1"}, sets=[curvy, S2])
    nosurf = {**S1, "surface": "t-2", "stations": {"method": "breaks", "from": 0.0, "to": 600.0, "tolerance": 0.1}}
    no("Kurulamayan yüzeyde arazi kırıkları", {"set": "k-1"},
       failed("invalid_section_set", "Arazi kırıklarında kesit için kesit takımının yüzeyi kurulabilmeli.", "surface"),
       setup(sets=[nosurf, S2]), sets=[nosurf, S2])
    return file("road.sections.query", "Enkesitleri sorgula", setup(), cases)


# ── road.sections.draw ─────────────────────────────────────────────────────────────────────────────────────────────

def step_of(span, steps=(1, 2, 5, 10, 20, 50, 100, 200, 500, 1000)):
    return float(next((g for g in steps if span / g <= 10), 1000))


def design_line(r):
    if "left" not in r:
        return []
    return list(reversed(r["left"]["points"])) + r["right"]["points"][1:]


def drawing(d, at, stations=None, columns=5, ex=1.0, height=2.5):
    """What the drawing says (each view's frame, datum, top, areas; the grid steps) and how many objects it writes."""
    rows, _ = sections(d, stations or stations_of(d), [TK1, TK2])
    h = height * 1000.0 / 1000.0
    row = 2.4 * h
    ranges = []
    for r in rows:
        zs = [g["z"] for g in r["ground"] if "z" in g] + [p[1] for p in design_line(r)] + ([r["axis"]] if "axis" in r else [])
        ranges.append((float(math.floor(min(zs))) - 1.0, float(math.ceil(max(zs))) + 1.0) if zs else None)
    lowest = min(x[0] for x in ranges if x)
    span = max(x[1] - x[0] for x in ranges if x)
    datums = [x[0] if x else lowest for x in ranges]
    width = d["left"] + d["right"]
    tall = span * ex
    px, py = 8 * h + width + 4 * h, row + tall + 3 * row + 4 * h
    grid, step = step_of(span), step_of(width)
    views, n = [], 0
    for i, (r, datum) in enumerate(zip(rows, datums)):
        col, line = i % columns, i // columns
        x0 = at[0] + col * px + 8 * h
        y1 = at[1] - line * py - row
        top = datum + span
        levels = [k * grid for k in range(math.ceil(datum / grid), math.floor(top / grid + 1e-9) + 1)]
        offs = [k * step for k in range(math.ceil(-d["left"] / step), math.floor(d["right"] / step + 1e-9) + 1)]
        runs, drawn = 0, False
        for a, b in zip(r["ground"], r["ground"][1:]):
            if "z" in a and "z" in b and not b["gap"]:
                runs += 0 if drawn else 1
                drawn = True
            else:
                drawn = False
        n += (1 + len(levels) + sum(datum + 1e-9 < v < top - 1e-9 for v in levels)
              + len(offs) + sum(-d["left"] + 1e-9 < o < d["right"] - 1e-9 for o in offs) + 1 + runs
              + (1 if "left" in r else 0) + 1 + ("axis" in r) + ("cut" in r))
        v = {"station": r["station"], "km": r["km"], "kmText": r["kmText"],
             "frame": [[x0, y1 - tall], [x0 + width, y1]], "datum": datum, "top": top}
        if "cut" in r:
            v["cut"], v["fill"] = r["cut"], r["fill"]
        views.append(v)
    return {"set": d["id"], "views": views, "grid": grid, "offsets": step, "exaggeration": ex}, n


def draw_cases():
    templates, start = [TK1, TK2], [S1, S2]
    layers = pcmd.DRAW_LAYERS
    cases = []
    first = len(pcmd.ENTITIES) + len(LINES) + 1

    def ok(name, input_, d, op="execute", warns=(), undo=False, sets=start, **kw):
        out, n = drawing(d, input_["at"], **kw)
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        road = road_of(templates, sets)
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road, "ids": list(range(1, first + n))}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Enkesit çiz", "expect": {"ids": list(range(1, first)), "canRedo": True}})
        cases.append(case(name, steps, None if sets is start else setup(sets=sets, layers=layers)))

    def no(name, input_, result, sets=start, setup_=None):
        road = None if sets is None else road_of(templates, sets)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if sets is start else setup(sets=sets, layers=layers))))

    def opens(d, at):
        return [w for w in sections(d, at, templates)[1] if w["code"] == "section_open"]

    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    base = {"set": "k-1", "layerId": "0", "at": [0.0, -100.0]}
    ok("Takımın bütün enkesitleri, satırda beş görünüş", base, S1)
    ok("Satırda üç görünüş", {**base, "columns": 3}, S1, columns=3)
    ok("Yalnız iki km", {**base, "stations": [250.0, 150.0]}, S1, stations=[250.0, 150.0])
    ok("İki kat abartma, satırda dört", {**base, "exaggeration": 2.0, "columns": 4}, S1, ex=2.0, columns=4)
    ok("Büyük yazı: aynı nesneler, geniş aralık", {**base, "height": 3.5}, S1, height=3.5)
    ok("Takım adıyla", {**base, "set": "enkesitler"}, S1)
    ok("Eskiyen takım uyarıyla, asimetrik tip kesit", {**base, "set": "k-2"}, S2,
       warns=[stale_warning(S2)] + opens(S2, stations_of(S2)))
    narrow = {**S1, "left": 4.0, "right": 6.0}
    ok("Dar şerit: açık yanlar uyarılır", base, narrow, warns=opens(narrow, stations_of(narrow)), sets=[narrow, S2])
    nt = {k: v for k, v in S1.items() if k != "template"}
    ok("Tip kesitsiz: arazi ve eksen kotu", base, nt, sets=[nt, S2])
    npf = {k: v for k, v in S1.items() if k != "profile"}
    ok("Boykesitsiz: yalnız arazi", base, npf, sets=[npf, S2])
    wide = {**S1, "left": 60.0, "right": 60.0, "stations": {"method": "stations", "stations": [30.0, 400.0, 600.0]}}
    ok("Yüzeyin dışına taşan şerit", base, wide, sets=[wide, S2])
    ok("Grubun içindeki katmana", {**base, "layerId": "yol-boykesit"}, S1)
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, S1, warns=[hidden])
    ok("Plan yazmaz", base, S1, op="plan")
    ok("Doğrulama yazmaz", {**base, "set": "k-2"}, S2, op="validate",
       warns=[stale_warning(S2)] + opens(S2, stations_of(S2)))
    ok("Tek geri alma adımı", {**base, "stations": [200.0]}, S1, undo=True, stations=[200.0])

    no("Yol projesi değil", base, NOT_ROAD, None, setup(work="map", layers=layers))
    no("Olmayan takım", {**base, "set": "yok"},
       failed("unknown_section_set", "“yok” kimlikli ya da adlı kesit takımı projede yok.", "set"))
    no("Sıfır görünüş", {**base, "columns": 0},
       failed("invalid_label", "Bir satırda 1 ile 100 arası görünüş (columns) olur.", "columns"))
    no("Çok görünüş", {**base, "columns": 101},
       failed("invalid_label", "Bir satırda 1 ile 100 arası görünüş (columns) olur.", "columns"))
    no("Eksi yazı yüksekliği", {**base, "height": -1.0},
       failed("invalid_label", "Yazı yüksekliği (height) 0'dan büyük olmalı.", "height"))
    no("Sıfır abartma", {**base, "exaggeration": 0.0},
       failed("invalid_label", "Düşey abartma (exaggeration) 0'dan büyük olmalı.", "exaggeration"))
    dead = {**S1, "stations": {"method": "lines", "uids": [MISSING_LINE]}}
    no("Km'si çıkmayan takım", base,
       failed("invalid_section_set", f"“{MISSING_LINE}” aksı kesmiyor.", "stations.uids[0]"), [dead, S2])
    no("Takımın km'si olmayan", {**base, "stations": [150.0, 160.0]},
       failed("invalid_places", "160 m “Enkesitler” kesit takımının km'lerinden biri değil.", "stations[1]"))
    bare = {k: v for k, v in S1.items() if k != "surface"}
    no("Yüzeysiz takım", base, failed("no_terrain", "“Enkesitler” kesit takımının yüzeyi yok: enkesit çizmek için önce "
                                                    "bir yüzey verin.", "set"), [bare, S2])
    broken = {**S1, "surface": "t-2"}
    no("Kurulamayan yüzey", base, failed("no_terrain", "“Enkesitler” kesit takımının yüzeyi projede yok ya da "
                                                       "kurulamıyor: önce yüzeyi düzeltin.", "set"), [broken, S2])
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return file("road.sections.draw", "Enkesit çiz", setup(layers=layers), cases)


def main():
    files = {"road.templates.edit.json": templates_cases(), "road.sections.edit.json": sections_cases(),
             "road.sections.query.json": query_cases(), "road.sections.draw.json": draw_cases()}
    check = "--check" in sys.argv[1:]
    bad = False
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text("utf-8") != text:
                print(f"güncel değil: {path.relative_to(ROOT)}")
                bad = True
        else:
            path.write_text(text, "utf-8")
            print(f"yazıldı: {path.relative_to(ROOT)} ({len(data['cases'])} durum)")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
