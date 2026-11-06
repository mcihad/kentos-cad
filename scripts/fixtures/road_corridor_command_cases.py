#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.corridors.{edit,query}.json: the shared cases of KentOS Yol's corridor commands
(plan/road.md Y7, docs/adr/0905), their expectations built here without KentOS code: the drawing, its alignments,
profiles, templates and superelevation schedules are scripts/fixtures/road_section_command_cases.py's; a corridor's
stations, feature lines and top surface scripts/fixtures/road_corridor_cases.py's; each station's section the section
reference's (the terrain along the sample line at 40 digits, the template in fractions). The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_corridor_command_cases.py
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
import road_corridor_cases as cref  # noqa: E402
import road_grades  # noqa: E402
import road_sampling_cases as sref  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_superelevation_command_cases as supcmd  # noqa: E402
import road_superelevation_cases as supref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Koridor yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
TEMPLATES = [scmd.TK1, scmd.TK2]
SUPERS = [scmd.D1, scmd.D2, scmd.D_BAD]
# Docs/adr/0972: a one-way crossfall on g-1 (the outside's normal rises outward: no level crown, so the widening's start
# is no critical station) and rows on g-2 whose sides cross (an inside-edge pivot changes sides between rows).
D_ONE = {"id": "d-4", "name": "Tek eğimli dever", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
         "normalLeft": -2.5, "normalRight": 2.5,
         "method": {"method": "curves", "curves": [{"pi": "p1", "rate": 6.0, "runoff": 50.0, "onTangent": 0.3,
                                                    "widening": 0.5}]}}
D_PIV = {"id": "d-5", "name": "Kesişen yanlar", "alignment": "g-2", "alignmentRevision": 2, "revision": 1,
         "normalLeft": -2.0, "normalRight": -2.0, "pivot": "insideEdge",
         "method": {"method": "stations", "rows": [{"station": 100.0, "left": -3.0, "right": -2.0},
                                                   {"station": 200.0, "left": 3.0, "right": -3.0},
                                                   {"station": 400.0, "left": 3.0, "right": -3.0},
                                                   {"station": 500.0, "left": -3.0, "right": -2.0}]}}
EXTRA_SUPERS = [D_ONE, D_PIV]

# A tight curve (R 4, inside the platform) for the folded strips: its alignment and a flat profile near the terrain.
G4 = {"id": "g-4", "name": "Dar kurp", "revision": 1,
      "pis": [pcmd.acmd.pi("a", 200, 180), pcmd.acmd.pi("p1", 240, 180, 4), pcmd.acmd.pi("b", 240, 240)]}
pcmd.ALIGNED["g-4"] = pcmd.acmd.build(pcmd.ALIGNMENTS + [G4], G4)
pcmd.REV["g-4"] = 1
G4_LENGTH = float(pcmd.ALIGNED["g-4"].length)
B4 = {"id": "b-4", "name": "Dar kurbun kotu", "alignment": "g-4", "alignmentRevision": 1, "revision": 1,
      "pvis": [pcmd.pv("a", 0, 106.0), pcmd.pv("b", round(G4_LENGTH - 0.5, 3), 106.0)]}
scmd.BY_PROFILE["b-4"] = B4
PROFILES = scmd.PROFILES + [B4]

C1 = {"id": "c-1", "name": "Ana koridor", "alignment": "g-1", "alignmentRevision": 1, "revision": 1, "profile": "b-1",
      "surface": "t-1", "left": 30.0, "right": 30.0, "tangent": 25.0, "curve": 10.0,
      "regions": [{"from": 0.0, "to": 300.0, "template": "tk-1"}, {"from": 300.0, "to": 600.0, "template": "tk-2"}]}
C2 = {"id": "c-2", "name": "Yan koridor", "alignment": "g-2", "alignmentRevision": 2, "revision": 2, "profile": "b-2",
      "surface": "t-1", "superelevation": "d-2", "left": 25.0, "right": 25.0, "tangent": 50.0, "curve": 20.0,
      "regions": [{"from": 50.0, "to": 400.0, "template": "tk-2"}]}
START = [C1, C2]


def road_of(corridors, supers=SUPERS, volumes=(), templates=None):
    out = scmd.road_of(templates or TEMPLATES, [scmd.S1, scmd.S2], supers=supers)
    out["alignments"].append(copy.deepcopy(G4))
    out["profiles"] = copy.deepcopy(PROFILES)
    if corridors:
        out["corridors"] = copy.deepcopy(corridors)
    if volumes:
        out["volumes"] = copy.deepcopy(list(volumes))
    return out


def setup(corridors=START, work="road", supers=SUPERS, layers=None, volumes=(), templates=None):
    base = scmd.setup(work=work, layers=layers)
    base["name"] = "Yol koridoru"
    if work == "road":
        base["settings"]["road"] = road_of(list(corridors), supers, volumes, templates)
    return base


def num(v):
    """A number as Rust's Display writes an f64."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def km_text(alignment, s):
    b = pcmd.ALIGNED[alignment]
    return pcmd.acmd.km_text(aref.km_of(b.regions, mp.mpf(s)))


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def warn(code, message):
    return {"code": code, "message": message, "path": "corridor"}


def bad(path, message):
    return Refused("invalid_road_corridor", message, path)


def template_codes(t):
    def one(side):
        d = side.get("ditch")
        # [components, ditch bottom, kinds (none: by the template's), benched cut and fill (docs/adr/0958)].
        return [len(side["components"]), d["bottom"] if d else None, None, (bool(side.get("bench")), bool(side.get("fillBench")))]
    return {"left": one(t["left"]), "right": one(t.get("right") or t["left"])}


def vertical_divisions(prof, t):
    """The profile's vertical curves divided by the chord height t (docs/adr/0939 §1): each parabola (an asymmetric
    one's two arms) or circle into the fewest equal steps of at most √(8t/k); k a parabola's |r|, a circle's
    (1 + g²)^(3/2)/R at its steeper end."""
    out = []
    pvis = {p["id"]: p for p in prof["pvis"]}
    for cv in pcmd.report(prof)["curves"]:
        s0, s1, g1, g2 = cv["start"][0], cv["end"][0], cv["gIn"], cv["gOut"]
        if cv["kind"] == "circle":
            k = max((1 + g * g) ** 1.5 for g in (g1, g2)) / cv["radius"]
            arms = [(s0, s1, k)]
        elif cv["kind"] == "parabola":
            arms = [(s0, s1, abs(g2 - g1) / (s1 - s0))]
        else:
            sp = pvis[cv["pvi"]]["station"]
            l1, l2 = sp - s0, s1 - sp
            r1 = (g2 - g1) * l2 / (l1 * (l1 + l2))
            arms = [(s0, sp, abs(r1)), (sp, s1, abs(r1 * l1 * l1 / (l2 * l2)))]
        for a, b, k in arms:
            if k == 0 or b <= a:
                continue
            step = math.sqrt(8 * t / k)
            n = max(1, math.ceil((b - a) / step - 1e-9))
            out += [a + (b - a) * i / n for i in range(1, n)]
    return out


def template_at(c, ri, s, templates):
    """Docs/adr/0960: corridor `c`'s region `ri`'s template at station `s` as (its id, the templates to find it in):
    within the region's slope transition, the region's template with its slopes, horizontal per vertical, a straight
    way from the region before's."""
    r = c["regions"][ri]
    t = next(x for x in templates if x["id"] == r["template"])
    lt = r.get("transition")
    if lt is None or ri == 0 or s >= r["from"] + lt:
        return r["template"], templates
    before = next(x for x in templates if x["id"] == c["regions"][ri - 1]["template"])
    k = min(max((s - r["from"]) / lt, 0.0), 1.0)
    both = lambda x: (x["left"], x.get("right") or x["left"])  # noqa: E731
    sides = [{**now, "cut": was["cut"] + (now["cut"] - was["cut"]) * k, "fill": was["fill"] + (now["fill"] - was["fill"]) * k}
             for was, now in zip(both(before), both(t))]
    return "__eased", list(templates) + [{**t, "id": "__eased", "left": sides[0], "right": sides[1]}]


def make(c, points=False, templates=TEMPLATES):
    """The corridor's report, its warnings and its triangles by the references; Refused where it does not build."""
    b = pcmd.ALIGNED[c["alignment"]]
    length = float(b.length)
    regions = cref.regions([[r["from"], r["to"]] for r in c["regions"]], length)
    if "error" in regions:
        e = regions["error"]
        if e["kind"] == "EmptyRegion":
            raise bad(f"regions[{e['index']}]", f"{e['index'] + 1}. bölgenin sonu başından en az 1 mm sonra olmalı.")
        if e["kind"] == "Outside":
            raise bad(f"regions[{e['index']}]",
                      f"{e['index'] + 1}. bölge aksın dışına taşıyor: aks 0 ile {length:.3f} m arası.")
        raise bad(f"regions[{e['second']}]",
                  f"{e['first'] + 1}. ve {e['second'] + 1}. bölge kesişiyor; bölgeler en çok uç uca gelir.")
    for i, s in enumerate(c.get("stations", [])):
        if s < -1e-9 or s > length + 1e-9:
            raise bad(f"stations[{i}]", f"{num(s)} m aksın üstünde değil: aks 0 ile {length:.3f} m arası.")
    prof = scmd.BY_PROFILE[c["profile"]]
    warnings = []
    sup = None
    crit = []
    if c.get("superelevation"):
        d = next(x for x in SUPERS + EXTRA_SUPERS if x["id"] == c["superelevation"])
        got, why = supcmd.build(d)
        if why:
            warnings.append(warn("corridor_superelevation_gone",
                                 f"“{c['name']}” koridorunun dever çizelgesi kurulamıyor; şeritler tip kesitin eğimiyle verildi."))
        else:
            sup = d
            crit = [float(x["station"]) for x in got[1]]
            sched = got[0]
    events = [[s, "user"] for s in c.get("stations", [])]
    if c.get("profilePoints"):
        events += [[p["station"], "profile"] for p in prof["pvis"]]
        for cv in pcmd.report(prof)["curves"]:
            events += [[cv["start"][0], "profile"], [cv["end"][0], "profile"]]
            if "extreme" in cv:
                events.append([cv["extreme"][0], "profile"])
    if c.get("verticalSag"):
        events += [[s, "vertical"] for s in vertical_divisions(prof, c["verticalSag"])]
    if c.get("critical"):
        events += [[s, "critical"] for s in crit]
    if c.get("critical") and sup is not None:
        # Docs/adr/0972: the widening's break points; where an inside- or outside-edge pivot changes sides, the
        # held side being the left where left ≤ right (insideEdge) or left > right (outsideEdge): the same set of
        # changes, found exactly in fractions between the rows' and the rules' break points.
        events += [[float(s), "widening"] for s in sorted({p[0] for p in sched["leftWidening"] + sched["rightWidening"]})]
        if sup.get("pivot") in ("insideEdge", "outsideEdge"):
            nl, nr = F(sup["normalLeft"]), F(sup["normalRight"])
            val = lambda pts, s, n: n if not pts else supref.along(pts, s)  # noqa: E731
            d_ = lambda s: val(sched["left"], s, nl) - val(sched["right"], s, nr)  # noqa: E731
            stops = sorted({p[0] for p in sched["left"] + sched["right"]})
            for p_, q_ in zip(stops, stops[1:]):
                dp, dq = d_(p_), d_(q_)
                if (dp > 0) != (dq > 0):
                    events.append([float(p_ + (q_ - p_) * dp / (dp - dq)), "pivot"])
    if c["surface"] != "t-1":
        warnings.append(warn("corridor_surface_gone",
                             f"“{c['name']}” koridorunun yüzeyi ({c['surface']}) projede yok ya da kurulamıyor; "
                             "hiçbir kesit çözülmedi."))
    stations, regs, checks = [], [], []
    for ri, r in enumerate(c["regions"]):
        t = next((x for x in templates if x["id"] == r["template"]), None)
        if t is None:
            raise Refused("unknown_road_template", f"“{r['template']}” kimlikli tip kesit projede yok.",
                          f"regions[{ri}].template")
        # Docs/adr/0960: the slope transition from the region before, its end sampled.
        span = None
        if "transition" in r:
            lt = r["transition"]
            prev = c["regions"][ri - 1] if ri > 0 else None
            if prev is None or abs(prev["to"] - r["from"]) > 1e-3 or not 0 < lt <= r["to"] - r["from"]:
                raise bad(f"regions[{ri}].transition", f"{ri + 1}. bölgenin şev geçişi 0'dan uzun, bölgeden kısa olur "
                                                         "ve önceki bölgenin bittiği yerde başlar.")
            span = (r["from"], r["from"] + lt)
        region_events = events + ([[span[1], "transition"]] if span else [])
        got = cref.stations(b.els, [r["from"], r["to"]], c["tangent"], c["curve"], c.get("sag"), region_events)
        if "error" in got:
            if got["error"]["kind"] == "TooMany":
                raise bad("tangent", f"Bu aralıklarla {cref.MAX_STATIONS}'den çok km çıkar; tangent ya da curve'ü büyütün.")
            raise bad("tangent", "Aralıklar ve sapma toleransı 0'dan büyük olmalı.")
        fake = {"name": c["name"], "alignment": c["alignment"], "surface": c["surface"], "profile": c["profile"],
                "template": r["template"], "superelevation": sup["id"] if sup else None,
                "left": c["left"], "right": c["right"]}
        built_at = {}

        def cross_at(s):
            if s in built_at:
                return built_at[s]
            tid, tl = template_at(c, ri, s, templates)
            row = scmd.sections({**fake, "template": tid}, [s], tl, EXTRA_SUPERS)[0][0]
            e, (x, y) = aref.place_at(b.els, mp.mpf(s))
            th = e.theta(mp.mpf(s) - e.station)
            sec = None
            if "left" in row:
                def side(sd):
                    return {"points": sd["points"], "hinge": sd["hinge"], "daylight": sd.get("daylight"),
                            "kind": sd["kind"]}
                sec = {"axis": row["axis"], "left": side(row["left"]), "right": side(row["right"]),
                       "cut": row["cut"], "fill": row["fill"]}
            built_at[s] = {"station": s, "at": [float(x), float(y)], "direction": float(th), "section": sec}
            return built_at[s]

        codes_of_t = template_codes(t)

        def coded_at(s):
            # The cross's coded points as its feature lines take them (docs/adr/0939 §2).
            cr = cross_at(s)
            sec = cr["section"]
            if sec is None:
                return None
            xy = cref.place(cr, 0)
            out = [("axis", (xy[0], xy[1], sec["axis"]))]
            for word in ("left", "right"):
                for code, o, z in cref.coded(codes_of_t, sec[word], word):
                    q = cref.place(cr, o)
                    out.append((code, (q[0], q[1], z)))
            return out

        here = [(s, why) for s, why in got["stations"]]
        if c.get("refine"):
            rf = c["refine"]
            tol = {"plan": rf.get("plan"), "height": rf.get("height"), "least": rf["least"]}
            budget = max(0, cref.MAX_STATIONS - (len(stations) + len(here)))
            done = sref.refine_with(coded_at, [s for s, _ in here], tol, budget)
            checks.append((ri, done))
            here = sorted(here + [(s, "refine") for s in done["added"]], key=lambda x: x[0])
        crosses = []
        for s, why in here:
            stations.append({"station": s, "kmText": km_text(c["alignment"], s), "why": why, "region": ri})
            crosses.append(cross_at(s))
        regs.append({"template": codes_of_t, "crosses": crosses})
    built = cref.assemble(regs)
    text = lambda s: km_text(c["alignment"], s)  # noqa: E731
    unsolved = [{"region": u["region"], "side": "right" if u["right"] else "left", "why": u["why"], "from": u["from"],
                 "to": u["to"], "fromText": text(u["from"]), "toText": text(u["to"])} for u in built["unsolved"]]
    features = []
    for f in built["features"]:
        p = f["points"]
        length3 = 0.0
        for a, q in zip(p, p[1:]):
            length3 += ((q[1] - a[1]) ** 2 + (q[2] - a[2]) ** 2 + (q[3] - a[3]) ** 2) ** 0.5
        out = {"code": f["code"], "region": f["region"], "from": p[0][0], "to": p[-1][0], "length": length3}
        if points:
            out["points"] = p
        features.append(out)
    area = 0.0
    for t in built["triangles"]:
        area += cref.area(t)
    now = pcmd.REV[c["alignment"]]
    stale = now != c.get("alignmentRevision", 0)
    if stale:
        warnings.append(warn("corridor_stale",
                             f"“{c['name']}” koridorunun aksı değişti (koridor {c.get('alignmentRevision', 0)}. revizyona "
                             f"göre kurulu, aks şimdi {now}.); bölgeleri denetleyip aksa yeniden bağlayın."))
    if regions["gaps"]:
        words = ", ".join(f"{text(g[0])} – {text(g[1])}" for g in regions["gaps"])
        warnings.append(warn("corridor_gap", f"“{c['name']}” koridorunun bölgeleri arasında kesitsiz aralık var: {words}."))
    if unsolved:
        def word(u):
            side = "sağ" if u["side"] == "right" else "sol"
            why = "şev buluşmuyor" if u["why"] == "open" else "kesit yok"
            if u["from"] == u["to"]:
                return f"km {u['fromText']} {side} ({why})"
            return f"km {u['fromText']} – {u['toText']} {side} ({why})"
        more = f" ve {len(unsolved) - 8} aralık daha" if len(unsolved) > 8 else ""
        warnings.append(warn("corridor_unsolved",
                             f"“{c['name']}” koridorunun çözülmeyen aralıkları: {', '.join(word(u) for u in unsolved[:8])}{more}."))
    if built["folded"]:
        words = ", ".join(f"km {text(f[1])} – {text(f[2])}" for f in built["folded"])
        warnings.append(warn("corridor_folded",
                             f"“{c['name']}” koridorunda ardışık kesitler planda kesişiyor: {words}; orada üst yüzey kurulmadı."))
    sampling = None
    if c.get("refine"):
        sampling = sampling_of(checks, text)
        left = [x for x in sampling["left"] if x["why"] != "change"]
        if left:
            def lw(x):
                why = "km sınırı" if x["why"] == "budget" else "en kısa adımda"
                return f"km {x['fromText']} – {x['toText']} ({why})"
            more = f" ve {len(left) - 8} aralık daha" if len(left) > 8 else ""
            warnings.append(warn("corridor_sampling_left",
                                 f"“{c['name']}” koridorunda kesitler şu aralıklarda toleransa inmedi: "
                                 f"{', '.join(lw(x) for x in left[:8])}{more}."))
    report = {"id": c["id"], "name": c["name"], "revision": c.get("revision", 0), "alignment": c["alignment"],
              "profile": c["profile"], "surface": c["surface"], "stale": stale, "stations": stations,
              "gaps": regions["gaps"], "unsolved": unsolved, "folded": [[f[1], f[2]] for f in built["folded"]],
              "features": features, "triangles": len(built["triangles"]), "area": area}
    if c.get("superelevation"):
        report["superelevation"] = c["superelevation"]
    if sampling is not None:
        report["sampling"] = sampling
    return report, warnings, built["triangles"], regs


def sampling_of(checks, text):
    """The sampling as it was checked, all regions together (docs/adr/0939 §2): middles measured (each within, or
    strayed and added), stations added, the largest strays of the first intervals and of the final ones (within, or
    left at the least step without a change), the changes found and the intervals left."""
    def worst(items):
        out = {"plan": 0.0, "height": 0.0}
        for m in items:
            at = (m["from"] + m["to"]) / 2
            if m["plan"] > out["plan"]:
                out["plan"], out["planAt"] = m["plan"], at
                if m["planCode"] is not None:
                    out["planCode"] = m["planCode"]
                else:
                    out.pop("planCode", None)
            if m["height"] > out["height"]:
                out["height"], out["heightAt"] = m["height"], at
                if m["heightCode"] is not None:
                    out["heightCode"] = m["heightCode"]
                else:
                    out.pop("heightCode", None)
        return out
    before = worst(m for _, d in checks for m in d["before"])
    after = worst([m for _, d in checks
                   for m in d["after"] + [x for x in d["left"] if x["why"] == "least" and not x["changed"]]])
    left, changes = [], 0
    for ri, d in checks:
        for x in d["left"]:
            why = x["why"]
            if why == "least" and x["changed"]:
                why, changes = "change", changes + 1
            left.append({"region": ri, "from": x["from"], "to": x["to"], "fromText": text(x["from"]),
                         "toText": text(x["to"]), "why": why, "plan": x.get("plan", 0.0),
                         "height": x.get("height", 0.0)})
    return {"measured": sum(len(d["after"]) + len(d["added"]) for _, d in checks),
            "added": sum(len(d["added"]) for _, d in checks), "before": before, "after": after,
            "changes": changes, "left": left}


def report(c):
    try:
        return make(c)[0]
    except Refused as r:
        out = {"id": c["id"], "name": c["name"], "revision": c.get("revision", 0), "alignment": c["alignment"],
               "profile": c["profile"], "surface": c["surface"], "stale": pcmd.REV[c["alignment"]] != c.get("alignmentRevision", 0),
               "stations": [], "gaps": [], "unsolved": [], "folded": [], "features": [], "triangles": 0, "area": 0.0,
               "problem": r.result["error"]["message"]}
        if c.get("superelevation"):
            out["superelevation"] = c["superelevation"]
        return out


def listed(corridors):
    return [{"id": c["id"], "name": c["name"], "alignment": c["alignment"], "revision": c.get("revision", 0),
             "stale": pcmd.REV[c["alignment"]] != c.get("alignmentRevision", 0)} for c in corridors]


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_corridor_command_cases.py yazar; çizim road_section_command_cases.py'nin, "
                    "km örneklemesi, öznitelik çizgileri ve üst yüzey road_corridor_cases.py'nin, her km'nin kesiti "
                    "enkesit başvurusunundur; KentOS kodu yok. `expect.road` projenin yol ayarlarıdır. Koridor geri alma "
                    "geçmişinde tutulan bir proje ayarıdır; sonucu her okumada kurulur.",
            "setup": base, "cases": cases}


# ── road.corridors.edit ────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    cases = []

    def written(after, target):
        out = {"corridors": listed(after), "revision": "$current"}
        if target:
            out["corridor"] = report(next(c for c in after if c["id"] == target))
        return out

    def ok(name, input_, after, target, op="execute", undo=False):
        writes = op == "execute" and after != START
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(START)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Koridor", "expect": {"road": road_of(START), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None):
        road = None if setup_ is not None else road_of(START)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def rulefail(words, path=None):
        return failed("invalid_road_corridor", f"Koridor kurallarına uymuyor: {words}.", path)

    def put(i, c):
        out = list(START)
        out[i] = c
        return out

    base = {"operation": "set", "name": "Yeni koridor", "alignment": "g-1", "profile": "b-1", "surface": "t-1",
            "left": 30.0, "right": 30.0, "tangent": 50.0, "curve": 20.0}
    one = [{"from": 100.0, "to": 250.0, "template": "tk-1"}]

    def new(id_, **kw):
        c = {"id": id_, "name": "Yeni koridor", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
             "profile": "b-1", "surface": "t-1", "left": 30.0, "right": 30.0, "tangent": 50.0, "curve": 20.0,
             "regions": one}
        c.update(kw)
        return c

    ok("Yeni koridor: kimliği ilk boş c-3", {**base, "regions": one}, START + [new("c-3")], "c-3")
    ok("Adlarla: aks, boykesit, yüzey ve tip kesit",
       {**base, "corridor": "kendi", "alignment": "ANA aks", "profile": "proje kotu", "surface": "Mevcut zemin",
        "regions": [{"from": 100.0, "to": 250.0, "template": "Tip 1"}]}, START + [new("kendi")], "kendi")
    ok("Devere ve kritik km'lere bağlı", {**base, "superelevation": "Ana aks deveri", "critical": True,
                                         "regions": [{"from": 120.0, "to": 260.0, "template": "tk-1"}]},
       START + [new("c-3", superelevation="d-1", critical=True, regions=[{"from": 120.0, "to": 260.0, "template": "tk-1"}])],
       "c-3")
    ok("Sapma toleransı, profil noktaları ve kullanıcı km'leri",
       {**base, "sag": 0.05, "profilePoints": True, "stations": [133.3], "regions": one},
       START + [new("c-3", sag=0.05, profilePoints=True, stations=[133.3])], "c-3")
    ok("Düşey kiriş yüksekliği ve örnekleme denetimi (ADR 0939)",
       {**base, "verticalSag": 0.05, "refine": {"plan": 0.05, "height": 0.05, "least": 0.5}, "regions": one},
       START + [new("c-3", verticalSag=0.05, refine={"plan": 0.05, "height": 0.05, "least": 0.5})], "c-3")
    ok("Var olan koridora yalnız kot denetimi", {"operation": "set", "corridor": "c-1", "refine": {"height": 0.02, "least": 1.0}},
       put(0, {**C1, "refine": {"height": 0.02, "least": 1.0}, "revision": 2}), "c-1")
    ok("En kısa adım 0: denetim yoksa değişiklik yok", {"operation": "set", "corridor": "c-1", "refine": {"least": 0.0}},
       START, "c-1")
    ok("Düşey kiriş yüksekliği 0: değişiklik yok", {"operation": "set", "corridor": "c-1", "verticalSag": 0.0}, START, "c-1")
    ok("Adı değişir", {"operation": "set", "corridor": "c-1", "name": "Koridor A"},
       put(0, {**C1, "name": "Koridor A", "revision": 2}), "c-1")
    ok("Bölgeler değişir", {"operation": "set", "corridor": "Ana koridor",
                            "regions": [{"from": 0.0, "to": 200.0, "template": "tk-2"}]},
       put(0, {**C1, "regions": [{"from": 0.0, "to": 200.0, "template": "tk-2"}], "revision": 2}), "c-1")
    ok("Dever kaldırılır", {"operation": "set", "corridor": "c-2", "superelevation": ""},
       put(1, {k: v for k, v in C2.items() if k != "superelevation"} | {"revision": 3}), "c-2")
    ok("Sapma toleransı kaldırılır", {"operation": "set", "corridor": "c-1", "sag": 0.0}, START, "c-1")
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "corridor": "c-1", "name": "Ana koridor"}, START, "c-1")
    ok("Eskiyen koridor aksa yeniden bağlanır", {"operation": "rebind", "corridor": "c-2"},
       put(1, {**C2, "alignmentRevision": 3, "revision": 3}), "c-2")
    ok("Koridor silinir", {"operation": "remove", "corridor": "c-2"}, [C1], None)
    ok("Silme tek geri alma adımı", {"operation": "remove", "corridor": "c-1"}, [C2], None, undo=True)
    ok("Plan yazmaz", {"operation": "set", "corridor": "c-1", "name": "Koridor A"},
       put(0, {**C1, "name": "Koridor A", "revision": 2}), "c-1", op="plan")
    ok("Doğrulama yazmaz", {"operation": "rebind", "corridor": "c-2"},
       put(1, {**C2, "alignmentRevision": 3, "revision": 3}), "c-2", op="validate")

    L1 = float(pcmd.ALIGNED["g-1"].length)
    no("Yol projesi değil", {"operation": "remove", "corridor": "c-1"}, NOT_ROAD, setup(work="map"))
    no("Olmayan koridor silinmez", {"operation": "remove", "corridor": "yok"},
       failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.", "corridor"))
    no("Olmayan aks", {**base, "alignment": "yok", "regions": one},
       failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Olmayan boykesit", {**base, "profile": "yok", "regions": one},
       failed("unknown_road_profile", "“yok” kimlikli ya da adlı boykesit projede yok.", "profile"))
    no("Olmayan yüzey", {**base, "surface": "yok", "regions": one},
       failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Olmayan dever", {**base, "superelevation": "yok", "regions": one},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Olmayan tip kesit", {**base, "regions": [{"from": 0.0, "to": 100.0, "template": "yok"}]},
       failed("unknown_road_template", "“yok” kimlikli ya da adlı tip kesit projede yok.", "regions[0].template"))
    no("Değiştirilecek bir şey yok", {"operation": "set", "corridor": "c-1"},
       failed("invalid_road_corridor", "Değiştirilecek bir şey yok: name, alignment, profile, surface, superelevation, left, "
                                       "right, tangent, curve, sag, verticalSag, refine, profilePoints, critical, stations ya da "
                                       "regions verin."))
    no("Yeni koridorun bölgesi yok", base,
       failed("invalid_road_corridor", "Yeni koridor için name, alignment, profile, surface, left, right, tangent, curve ve "
                                       "regions verilmeli.", "regions"))
    no("Boykesit başka aksta", {**base, "profile": "b-2", "regions": one},
       rulefail("boykesit koridorun aksında değil", "profile"))
    no("Dever başka aksta", {**base, "superelevation": "d-2", "regions": one},
       rulefail("dever çizelgesi koridorun aksında değil", "superelevation"))
    no("Paralel aksa bağlanmaz", {**base, "alignment": "g-3", "regions": one},
       rulefail("koridor yalnız PI'larıyla kurulan aksa bağlanır", "alignment"))
    no("Aynı ad", {**base, "name": "YAN koridor", "regions": one}, rulefail("bu adda başka bir koridor var", "name"))
    no("Sıfır aralık", {**base, "tangent": 0.0, "regions": one}, rulefail("aralık 0'dan büyük, en çok 1000 m olur", "tangent"))
    no("Düşey kiriş yüksekliği 10 m'den büyük", {**base, "verticalSag": 20.0, "regions": one},
       rulefail("düşey kiriş yüksekliği 0'dan büyük, en çok 10 m olur", "verticalSag"))
    no("Eksi plan sapması", {**base, "refine": {"plan": -0.1, "least": 1.0}, "regions": one},
       rulefail("sapma toleransı 0'dan büyük, en çok 10 m olur", "refine.plan"))
    no("En kısa adım 1 cm'den kısa", {**base, "refine": {"height": 0.05, "least": 0.001}, "regions": one},
       rulefail("en kısa adım 0,01 ile 100 m arası olur", "refine.least"))
    no("Kesişen bölgeler", {**base, "regions": [{"from": 0.0, "to": 200.0, "template": "tk-1"},
                                                 {"from": 150.0, "to": 300.0, "template": "tk-2"}]},
       failed("invalid_road_corridor", "1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.", "regions[1]"))
    no("Boş bölge", {**base, "regions": [{"from": 100.0, "to": 100.0, "template": "tk-1"}]},
       failed("invalid_road_corridor", "1. bölgenin sonu başından en az 1 mm sonra olmalı.", "regions[0]"))
    no("Aksın dışına taşan bölge", {**base, "regions": [{"from": 500.0, "to": 900.0, "template": "tk-1"}]},
       failed("invalid_road_corridor", f"1. bölge aksın dışına taşıyor: aks 0 ile {L1:.3f} m arası.", "regions[0]"))
    # Docs/adr/0960: a region's slope transition from the region before.
    eased = [{"from": 0.0, "to": 200.0, "template": "tk-1"}, {"from": 200.0, "to": 400.0, "template": "tk-2", "transition": 40.0}]
    ok("Şev geçişli bölge", {**base, "regions": eased}, START + [new("c-3", regions=eased)], "c-3")
    no("İlk bölgede şev geçişi", {**base, "regions": [{**one[0], "transition": 20.0}]},
       rulefail("şev geçişi ancak önceki bölgenin bittiği yerde başlayan bölgede olur", "regions[0].transition"))
    no("Bölgeden uzun şev geçişi", {**base, "regions": [eased[0], {**eased[1], "transition": 250.0}]},
       rulefail("şev geçişi 0'dan uzun, bölgeden kısa olur", "regions[1].transition"))
    no("Boşluktan sonra şev geçişi", {**base, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, eased[1]]},
       rulefail("şev geçişi ancak önceki bölgenin bittiği yerde başlayan bölgede olur", "regions[1].transition"))
    no("Aksın dışındaki km", {**base, "stations": [900.0], "regions": one},
       failed("invalid_road_corridor", f"900 m aksın üstünde değil: aks 0 ile {L1:.3f} m arası.", "stations[0]"))
    no("Çok sık örnekleme", {**base, "tangent": 0.02, "regions": [{"from": 0.0, "to": 600.0, "template": "tk-1"}]},
       failed("invalid_road_corridor", f"Bu aralıklarla {cref.MAX_STATIONS}'den çok km çıkar; tangent ya da curve'ü büyütün.",
              "tangent"))
    used = setup(volumes=[{"id": "v-1", "name": "Ana kübaj", "corridor": "c-1", "revision": 1, "method": "endArea"}])
    cases.append(case("Kübajın kullandığı koridor silinmez", [step(
        "execute", {"operation": "remove", "corridor": "c-1"},
        failed("corridor_in_use", "“Ana koridor” koridorunu şu kübaj hesapları kullanıyor: Ana kübaj; önce onları silin.",
               "corridor"), {**UNCHANGED, "road": used["settings"]["road"]})], used))
    no("Geçersiz beklenen sürüm", {"operation": "remove", "corridor": "c-2", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "remove", "corridor": "c-2", "expectedRevision": "41"}, CONFLICT)
    return file("road.corridors.edit", "Koridoru düzenle", setup(), cases)


# ── road.corridors.query ───────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    cases = []

    def q(name, input_, corridors=START, op="execute", templates=None, supers=None):
        one = input_.get("corridor")
        tpls = templates or TEMPLATES
        if one is None:
            reports, warns = [], []
            for c in corridors:
                try:
                    r, w, _, _ = make(c, templates=tpls)
                    reports.append(r)
                    warns += w
                except Refused:
                    reports.append(report(c))
            out = {"corridors": reports}
        else:
            c = next(x for x in corridors if x["id"] == one or x["name"].lower() == one.lower())
            r, warns, tris, _ = make(c, input_.get("points", False), templates=tpls)
            out = {"corridors": [r]}
            if input_.get("triangles"):
                out["triangles"] = tris
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        sp = supers or SUPERS
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road_of(corridors, sp, templates=templates)})],
                          None if corridors is START and templates is None and supers is None
                          else setup(corridors, supers=sp, templates=templates)))

    def no(name, input_, result, corridors=START, setup_=None):
        road = None if setup_ is not None else road_of(corridors)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if corridors is START else setup(corridors))))

    def with_(c, **kw):
        out = dict(c)
        for k, v in kw.items():
            if v is None:
                out.pop(k, None)
            else:
                out[k] = v
        return out

    q("Bütün koridorlar; eskiyen uyarılır", {})
    q("Bir koridor adıyla", {"corridor": "ANA koridor"})
    q("Öznitelik çizgileri noktalarıyla", {"corridor": "c-1", "points": True})
    q("Üst yüzeyin üçgenleri", {"corridor": "c-2", "triangles": True})
    q("Devere bağlı, kritik km'lerle", {"corridor": "c-1"},
      [with_(C1, superelevation="d-1", critical=True, regions=[{"from": 100.0, "to": 300.0, "template": "tk-1"}]), C2])
    # Docs/adr/0972: the widening's start where no level crown is (one-way crossfall), the inside-edge pivot's
    # change between rows.
    q("Tek eğimli devere bağlı: genişlemenin başı km olur", {"corridor": "c-1"},
      [with_(C1, superelevation="d-4", critical=True, regions=[{"from": 100.0, "to": 300.0, "template": "tk-1"}]), C2],
      supers=SUPERS + EXTRA_SUPERS)
    q("İç kenar dönme ekseni yan değiştirir: km olur", {"corridor": "c-2"},
      [C1, with_(C2, superelevation="d-5", critical=True, regions=[{"from": 50.0, "to": 550.0, "template": "tk-2"}])],
      supers=SUPERS + EXTRA_SUPERS)
    q("Profil noktaları ve kullanıcı km'leri", {"corridor": "c-1"},
      [with_(C1, profilePoints=True, stations=[77.7, 412.5]), C2])
    q("Kurpta sapma toleransı", {"corridor": "c-1"},
      [with_(C1, sag=0.02, regions=[{"from": 150.0, "to": 300.0, "template": "tk-1"}]), C2])
    q("Bölgeler arasında boşluk", {"corridor": "c-1"},
      [with_(C1, regions=[{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 400.0, "template": "tk-2"}]), C2])
    q("Dar şerit: açık yanlar", {"corridor": "c-1"}, [with_(C1, left=6.0, right=6.0), C2])
    q("Boykesitin dışı kesitsiz", {"corridor": "c-1"},
      [with_(C1, profile="b-3", regions=[{"from": 50.0, "to": 450.0, "template": "tk-1"}]), C2])
    q("Kurulamayan yüzey", {"corridor": "c-1"}, [with_(C1, surface="t-2"), C2])
    q("Projede olmayan yüzey", {"corridor": "c-1"}, [with_(C1, surface="t-9"), C2])
    q("Kurulamayan dever", {"corridor": "c-1"}, [with_(C1, superelevation="d-3"), C2])
    folded = {"id": "c-3", "name": "Dar kurp koridoru", "alignment": "g-4", "alignmentRevision": 1, "revision": 1,
              "profile": "b-4", "surface": "t-1", "left": 40.0, "right": 40.0, "tangent": 10.0, "curve": 2.0,
              "regions": [{"from": 0.0, "to": round(G4_LENGTH - 1.0, 3), "template": "tk-1"}]}
    # Docs/adr/0958: the main corridor with its first template's cut slopes benched every 2 m; then its second
    # template's fill slopes (up to 2.7 m high, km 1+347 – 1+425) every metre.
    benched = [{**scmd.TK1, "left": {**scmd.TK1["left"], "bench": {"height": 2.0, "width": 2.5, "fall": 5.0}}}, scmd.TK2]
    q("Basamaklı yarma şevli koridor", {"corridor": "c-1", "points": True}, START, templates=benched)
    fb = {"height": 1.0, "width": 1.5, "fall": 4.0}
    filled = [scmd.TK1, {**scmd.TK2, "left": {**scmd.TK2["left"], "fillBench": fb}, "right": {**scmd.TK2["right"], "fillBench": fb}}]
    q("Basamaklı dolgu şevli koridor", {"corridor": "c-1", "points": True}, START, templates=filled)
    # Docs/adr/0960: at km 1+300 tk-1 (cut 1, fill 1.5) gives way to its twin with flatter slopes (cut 2, fill 3):
    # without a transition the slopes jump there; over 40 m they go from the one to the other.
    flatter = [scmd.TK1, scmd.TK2, {**scmd.TK1, "id": "tk-3", "name": "Tip 1 yatık şev",
                                    "left": {**scmd.TK1["left"], "cut": 2.0, "fill": 3.0}}]
    twin = [{"from": 0.0, "to": 300.0, "template": "tk-1"}, {"from": 300.0, "to": 600.0, "template": "tk-3"}]
    q("Yatık şevli ikiz tip kesit: geçişsiz", {"corridor": "c-1", "points": True}, [with_(C1, regions=twin), C2],
      templates=flatter)
    q("Şev geçişli bölge", {"corridor": "c-1", "points": True},
      [with_(C1, regions=[twin[0], {**twin[1], "transition": 40.0}]), C2], templates=flatter)
    q("Uzun şev geçişi", {"corridor": "c-1"}, [with_(C1, regions=[twin[0], {**twin[1], "transition": 120.0}]), C2],
      templates=flatter)
    q("Farklı bileşenli tip kesite şev geçişi", {"corridor": "c-1"},
      [with_(C1, regions=[C1["regions"][0], {**C1["regions"][1], "transition": 40.0}]), C2])
    q("Dar kurpta kesitler kesişir", {"corridor": "c-3"}, START + [folded])
    # docs/adr/0939: the vertical curves divided by their chord height, the sections between measured and refined.
    q("Düşey kurplar kiriş yüksekliğiyle bölünür", {"corridor": "c-1"},
      [with_(C1, verticalSag=0.02, profilePoints=True), C2])
    q("Denetim: kot sapması 5 cm", {"corridor": "c-1"},
      [with_(C1, refine={"height": 0.05, "least": 1.0}), C2])
    q("Denetim: kurpta plan sapması 2 cm", {"corridor": "c-1"},
      [with_(C1, refine={"plan": 0.02, "least": 1.0}, regions=[{"from": 150.0, "to": 300.0, "template": "tk-1"}]), C2])
    q("Denetim: plan ve kot birlikte, sık aralıkta", {"corridor": "c-1"},
      [with_(C1, refine={"plan": 0.05, "height": 0.05, "least": 0.5},
             regions=[{"from": 100.0, "to": 260.0, "template": "tk-1"}]), C2])
    q("Toleranssız denetim: yalnız kesit değişimlerinin yeri", {"corridor": "c-1"},
      [with_(C1, refine={"least": 0.5}), C2])
    q("En kısa adımda bırakılan aralıklar uyarılır", {"corridor": "c-1"},
      [with_(C1, refine={"height": 0.001, "least": 5.0}, regions=[{"from": 0.0, "to": 200.0, "template": "tk-1"}]), C2])
    q("Dar şerit: açık yanların sınırları bulunur", {"corridor": "c-1"},
      [with_(C1, left=6.0, right=6.0, refine={"least": 1.0}), C2])
    q("Devere bağlı, kritik km'siz: enine eğim ölçülür", {"corridor": "c-1"},
      [with_(C1, superelevation="d-1", refine={"height": 0.01, "least": 1.0},
             regions=[{"from": 100.0, "to": 300.0, "template": "tk-1"}]), C2])
    q("Düşey bölme ve denetim birlikte", {"corridor": "c-1"},
      [with_(C1, verticalSag=0.05, refine={"plan": 0.1, "height": 0.1, "least": 1.0}), C2])
    q("Yan koridorun denetimi", {"corridor": "c-2"}, [C1, with_(C2, refine={"height": 0.05, "least": 1.0})])
    q("Bütün koridorlar denetimleriyle", {},
      [with_(C1, refine={"height": 0.1, "least": 2.0}), with_(C2, verticalSag=0.05)])
    q("Plan cevabı verir", {"corridor": "c-2"}, op="plan")
    q("Doğrulama uyarıları verir", {"corridor": "c-2"}, op="validate")
    q("Koridor yok: boş liste", {}, [])
    broken = with_(C1, regions=[{"from": 0.0, "to": 100.0, "template": "tk-1"}, {"from": 50.0, "to": 200.0, "template": "tk-1"}])
    q("Kurulamayan koridor raporda nedeniyle", {}, [broken, C2])

    no("Yol projesi değil", {}, NOT_ROAD, setup_=setup(work="map"))
    no("Olmayan koridor", {"corridor": "yok"},
       failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.", "corridor"))
    no("Koridorsuz nokta sorusu", {"points": True},
       failed("invalid_road_corridor", "Noktalar ve üçgenler bir koridorda sorulur: koridorun kimliğini ya da adını "
                                       "(corridor) verin.", "corridor"))
    no("Kurulamayan koridorun sorusu", {"corridor": "c-1"},
       failed("invalid_road_corridor", "1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.", "regions[1]"),
       [broken, C2])
    return file("road.corridors.query", "Koridoru sorgula", setup(), cases)


# ── road.corridors.draw ────────────────────────────────────────────────────────────────────────────────────────────

def drawing(c, tick=5.0):
    """What the slope plan says and how many objects it writes: the feature lines but the axis, the ticks, the
    elevations; and the warnings it gives (the corridor's but the surface's)."""
    report, warns, _, regs = make(c, points=True)
    lines = sum(1 for f in report["features"] if f["code"] != "axis" and len(f["points"]) >= 2)
    ticks = 0
    for reg in regs:
        for a, b in zip(reg["crosses"], reg["crosses"][1:]):
            if a["section"] is None or b["section"] is None:
                continue
            s0, s1 = a["station"], b["station"]
            for side in ("left", "right"):
                sa, sb = a["section"][side], b["section"][side]
                if sa["daylight"] is None or sb["daylight"] is None or "open" in (sa["kind"], sb["kind"]):
                    continue
                k = math.ceil(s0 / tick - 1e-9)
                while k * tick < s1 - 1e-9:
                    ticks += 1
                    k += 1
    elevations = sum(1 for reg in regs for x in reg["crosses"] if x["section"] is not None)
    out = {"corridor": c["id"], "lines": lines, "ticks": ticks, "elevations": elevations, "tick": tick}
    return out, lines + ticks + elevations, [w for w in warns if w["code"] != "corridor_surface_gone"], regs


def draw_cases():
    layers = pcmd.DRAW_LAYERS
    cases = []
    first = len(pcmd.ENTITIES) + len(scmd.LINES) + 1

    def ok(name, input_, c, op="execute", extra=(), undo=False, corridors=START, tick=5.0):
        out, n, warns, _ = drawing(c, tick)
        warns = list(extra) + warns
        out = {**out, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        road = road_of(corridors)
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road, "ids": list(range(1, first + n))}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Şevli plan", "expect": {"ids": list(range(1, first)), "canRedo": True}})
        cases.append(case(name, steps, None if corridors is START else setup(corridors, layers=layers)))

    def no(name, input_, result, corridors=START, setup_=None):
        road = None if setup_ is not None else road_of(corridors)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if corridors is START else setup(corridors, layers=layers))))

    hidden = {"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
    base = {"corridor": "c-1", "layerId": "0"}
    ok("Ana koridorun şevli planı", base, C1)
    ok("Şev taraması 2 m'de", {**base, "tick": 2.0}, C1, tick=2.0)
    ok("Seyrek tarama, büyük yazı", {**base, "tick": 20.0, "height": 3.5}, C1, tick=20.0)
    ok("Koridor adıyla", {**base, "corridor": "ana KORIDOR"}, C1)
    ok("Eskiyen yan koridor, devere bağlı", {**base, "corridor": "c-2"}, C2)
    narrow = {**C1, "left": 6.0, "right": 6.0}
    ok("Dar şerit: açık yanlarda tarama yok", base, narrow, corridors=[narrow, C2])
    gap = {**C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    ok("Bölgeler arasında boşluk", base, gap, corridors=[gap, C2])
    part = {**C1, "profile": "b-3", "regions": [{"from": 50.0, "to": 450.0, "template": "tk-1"}]}
    ok("Boykesitin dışı çizilmez", base, part, corridors=[part, C2])
    sup = {**C1, "superelevation": "d-1", "critical": True, "regions": [{"from": 100.0, "to": 300.0, "template": "tk-1"}]}
    ok("Dever ve kritik km'ler", base, sup, corridors=[sup, C2])
    bad = {**C1, "superelevation": "d-3"}
    ok("Kurulamayan dever uyarıyla", base, bad, corridors=[bad, C2])
    ok("Grubun içindeki katmana", {**base, "layerId": "yol-boykesit"}, C1)
    ok("Gizli katmana uyarıyla", {**base, "layerId": "gizli"}, C1, extra=[hidden])
    ok("Plan yazmaz", base, C1, op="plan")
    ok("Doğrulama yazmaz", {**base, "corridor": "c-2"}, C2, op="validate")
    ok("Tek geri alma adımı", {**base, "tick": 10.0}, C1, undo=True, tick=10.0)

    no("Yol projesi değil", base, NOT_ROAD, setup_=setup(work="map", layers=layers))
    no("Olmayan koridor", {**base, "corridor": "yok"},
       failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.", "corridor"))
    no("Sıfır tarama aralığı", {**base, "tick": 0.0},
       failed("invalid_label", "Şev taramasının aralığı (tick) 0'dan büyük olmalı.", "tick"))
    no("Eksi yazı yüksekliği", {**base, "height": -1.0},
       failed("invalid_label", "Yazı yüksekliği (height) 0'dan büyük olmalı.", "height"))
    broken = {**C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"}, {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    no("Kurulamayan koridor", base,
       failed("invalid_road_corridor", "1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.", "regions[1]"), [broken, C2])
    nosurf = {**C1, "surface": "t-2"}
    no("Hiçbir kesit çözülmedi", base,
       failed("no_terrain", "“Ana koridor” koridorunda hiçbir kesit çözülmedi: yüzeyi ve boykesiti denetleyin.", "corridor"),
       [nosurf, C2])
    no("Olmayan katman", {**base, "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {**base, "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"))
    no("Kilitli katman", {**base, "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {**base, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {**base, "expectedRevision": "41"}, CONFLICT)
    return file("road.corridors.draw", "Şevli plan çiz", setup(layers=layers), cases)


def main():
    files = {"road.corridors.edit.json": edit_cases(), "road.corridors.query.json": query_cases(),
             "road.corridors.draw.json": draw_cases()}
    check = "--check" in sys.argv[1:]
    bad = []
    for name, data in files.items():
        data = road_grades.fixture(data)
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if check:
            if not path.exists() or path.read_text() != text:
                bad.append(name)
        else:
            path.write_text(text)
            print(f"written {path} ({len(data['cases'])} cases)")
    if bad:
        sys.exit(f"not current: {', '.join(bad)}")
    if check:
        print("road corridor cases are current")


if __name__ == "__main__":
    main()
