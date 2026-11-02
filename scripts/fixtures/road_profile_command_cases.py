#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.profiles.{create,edit,query}.json: the shared cases of KentOS Yol's vertical
alignment commands (plan/road.md Y4, docs/adr/0902), their expectations built here without KentOS code: the alignment
is scripts/fixtures/road_alignment_cases.py's (mpmath at 40 digits), the vertical alignment
scripts/fixtures/road_vertical_cases.py's (exact fractions, circles in mpmath), the surface Shewchuk's Triangle's and
the terrain profile's crossings scripts/fixtures/road_profile_cases.py's (a 5 cm walk refined by bisection at 40
digits). The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py)
run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_profile_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import math
import random
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_alignment_command_cases as acmd  # noqa: E402
import road_profile_cases as pref  # noqa: E402
import road_surface_cases as tri  # noqa: E402
import road_vertical_cases as vref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
ALGORITHM = "kentos.profile/1"
W = 1e-6
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}

completed, failed, step = acmd.completed, acmd.failed, acmd.step
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = acmd.UNCHANGED, acmd.STEPPED, acmd.CONFLICT, acmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Boykesit yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")


def U(n):
    return f"0192f5a0-7c3e-7d4a-9b1e-{n:012x}"


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(STYLE), "children": []}


# ── The drawing: 120 points with heights on `zemin` (road_profile_cases.py's second case), none on `bos` ─────────────

def points():
    rng = random.Random(2)
    out = []
    for _ in range(120):
        x = round(rng.random() * 600, 3)
        y = round(rng.random() * 400, 3)
        z = round(100 + 0.02 * x + 0.01 * y + 4 * math.sin(x / 37) + rng.random(), 3)
        out.append((x, y, z))
    return out


POINTS = points()
TIN = pref.Tin([(x, y) for x, y, _ in POINTS], [z for _, _, z in POINTS], tri.triangulate([(x, y) for x, y, _ in POINTS], []))
LAYERS = [node("0", "0"), node("zemin", "Zemin"), node("bos", "Boş")]
ENTITIES = [{"kind": "point", "id": i + 1, "layerId": "zemin", "attrs": {}, "p": {"x": x, "y": y}, "z": z}
            for i, (x, y, z) in enumerate(POINTS)]
UIDS = [U(i + 1) for i in range(len(ENTITIES))]

SURFACES = [{"id": "t-1", "name": "Mevcut zemin", "points": {"layers": ["zemin"]}},
            {"id": "t-2", "name": "Kurulamaz", "points": {"layers": ["bos"]}}]
P = aref.pi
MAIN_PIS = [acmd.pi("a", 20, 50), acmd.pi("p1", 300, 60, 150, 40, 40), acmd.pi("b", 560, 360)]
SIDE_PIS = [acmd.pi("a", 30, 380), acmd.pi("b", 580, 20)]
ALIGNMENTS = [{"id": "g-1", "name": "Ana aks", "revision": 1, "pis": MAIN_PIS, "stationing": {"start": 1000.0}},
              {"id": "g-2", "name": "Yan aks", "revision": 3, "pis": SIDE_PIS},
              {"id": "g-3", "name": "Sağ kenar", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": 5.0}}]


def pv(id_, s, z, **curve):
    out = {"id": id_, "station": float(s), "elevation": float(z)}
    out.update({k: float(v) for k, v in curve.items()})
    return out


MAIN_PVIS = [pv("a", 0, 102), pv("v1", 170, 101.5, length=80), pv("v2", 340, 111, radius=1200),
             pv("v3", 520, 106, lengthIn=50, lengthOut=35), pv("b", 669, 113)]
MAIN = {"id": "b-1", "name": "Proje kotu", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
        "pvis": MAIN_PVIS, "surface": "t-1"}
SIDE = {"id": "b-2", "name": "Yan kot", "alignment": "g-2", "alignmentRevision": 2, "revision": 4,
        "pvis": [pv("a", 0, 110), pv("v1", 300, 104, length=120), pv("b", 650, 112)], "surface": "t-1"}
PART = {"id": "b-3", "name": "Kısmi kot", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
        "pvis": [pv("a", 100, 104), pv("b", 400, 110)]}


def setup(profiles=(), work="road", surfaces=SURFACES):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if work == "road":
        settings["road"] = road_of(list(profiles), surfaces)
    return {"format": "kentos.document", "version": 2, "name": "Yol boykesitleri", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0",
            "entities": copy.deepcopy(ENTITIES), "uids": list(UIDS), "styles": {"items": [], "categories": []}}


def road_of(profiles, surfaces=SURFACES):
    if profiles is None:
        return None
    out = {"surfaces": copy.deepcopy(surfaces), "alignments": copy.deepcopy(ALIGNMENTS)}
    if profiles:
        out["profiles"] = copy.deepcopy(profiles)
    return out


# ── The geometry, by the references ───────────────────────────────────────────────────────────────────────────────

ALIGNED = {a["id"]: acmd.build(ALIGNMENTS, a) for a in ALIGNMENTS if "offsetOf" not in a}
REV = {a["id"]: a["revision"] for a in ALIGNMENTS}


def to_vref(pvis):
    out = []
    for p in pvis:
        c = None
        if p.get("radius"):
            c = vref.circ(p["radius"])
        elif p.get("length"):
            c = vref.par(p["length"])
        elif p.get("lengthIn") or p.get("lengthOut"):
            c = vref.asym(p.get("lengthIn", 0), p.get("lengthOut", 0))
        out.append(vref.pvi(p["id"], p["station"], p["elevation"], c))
    return out


def solve(d):
    return vref.solve(to_vref(d["pvis"]))


def report(d):
    b = ALIGNED[d["alignment"]]
    pvis = d["pvis"]
    curves, _, _, _ = solve(d)
    rows = vref.report(to_vref(pvis), curves)
    out_curves = []
    for r in rows:
        p = next(x for x in pvis if x["id"] == r["pvi"])
        kind = "circle" if p.get("radius") else ("parabola" if p.get("length") else "asymmetric")
        c = {"pvi": r["pvi"], "kind": kind, "gIn": r["gIn"], "gOut": r["gOut"], "start": r["start"], "end": r["end"],
             "length": r["end"][0] - r["start"][0], "a": r["a"], "k": r["k"], "crest": r["crest"]}
        if kind == "circle":
            c["radius"] = p["radius"]
        if r["extreme"] is not None:
            c["extreme"] = r["extreme"]
        out_curves.append(c)
    now = REV[d["alignment"]]
    first, last = pvis[0]["station"], pvis[-1]["station"]
    out = {"id": d["id"], "name": d["name"], "revision": d.get("revision", 0), "alignment": d["alignment"],
           "alignmentRevision": d.get("alignmentRevision", 0), "alignmentNow": now,
           "stale": now != d.get("alignmentRevision", 0),
           "stations": [first, last], "kms": [float(aref.km_of(b.regions, first)), float(aref.km_of(b.regions, last))],
           "grades": [{"between": [a["id"], c["id"]], "stations": [a["station"], c["station"]],
                       "grade": (c["elevation"] - a["elevation"]) / (c["station"] - a["station"])}
                      for a, c in zip(pvis, pvis[1:])],
           "curves": out_curves, "algorithm": ALGORITHM}
    if d.get("surface"):
        out["surface"] = d["surface"]
    return out


def listed(profiles):
    return [{"id": p["id"], "name": p["name"], "alignment": p["alignment"], "revision": p.get("revision", 0),
             "stale": REV[p["alignment"]] != p.get("alignmentRevision", 0)} for p in profiles]


def written(profiles, target):
    out = {"profiles": listed(profiles), "revision": "$current"}
    if target:
        out["profile"] = report(next(p for p in profiles if p["id"] == target))
    return out


def black_at(b, s):
    x, y = pref.point_at(b.els, mp.mpf(s))
    return TIN.height(float(x), float(y))


def at(d, s, surface=True):
    b = ALIGNED[d["alignment"]]
    km = aref.km_of(b.regions, mp.mpf(s))
    _, ev, _, _ = solve(d)
    red = ev(s)
    out = {"station": float(s), "km": float(km), "kmText": acmd.km_text(km)}
    if red is not None:
        out["red"], out["grade"] = red[0], red[1]
    black = black_at(b, s) if surface and d.get("surface") == "t-1" else None
    if black is not None:
        out["black"] = black
    if red is not None and black is not None:
        out["difference"] = red[0] - black
    return out


def ticks(b, step_):
    out = []
    for i, r in enumerate(b.regions):
        ka, kb = r[1], r[1] + r[2] * (r[3] - r[0])
        k = mp.ceil(min(ka, kb) / step_) * step_
        here = []
        while k <= max(ka, kb) + mp.mpf("1e-9"):
            s = r[0] + r[2] * (k - r[1])
            last = i == len(b.regions) - 1
            if s >= r[0] - mp.mpf("1e-9") and (s < r[3] - mp.mpf("1e-9") or (last and s <= r[3] + mp.mpf("1e-9"))):
                here.append(min(max(s, r[0]), r[3]))
            k += step_
        out += [float(s) for s in sorted(here)]
    return out


def ground(alignment):
    """The terrain profile's points: crossings (their edge's height), the elements' ends and the alignment's ends (the
    surface's height there); each says whether the stretch before it is off the surface."""
    b = ALIGNED[alignment]
    cross = pref.profile(TIN, b.els, b.length)
    marks = [(0.0, None)] + [(float(e.station), None) for e in b.els[1:]] + [(float(b.length), None)]
    allp = sorted(marks + [(s, z) for s, z in cross], key=lambda t: t[0])
    kept = []
    for s, z in allp:
        if kept and s - kept[-1][0] < 1e-9:
            if kept[-1][1] is None:
                kept[-1] = (kept[-1][0], z)
            continue
        kept.append((s, z))
    out = []
    for i, (s, z) in enumerate(kept):
        if i == 0:
            gap = False
        else:
            mid = (kept[i - 1][0] + s) / 2
            gap = black_at(b, mid) is None
        crossing = z is not None
        zz = z if z is not None else black_at(b, s)
        row = {"station": s, "gap": gap, "crossing": crossing}
        if zz is not None:
            row["z"] = zz
        out.append(row)
    return out


def fmt(v):
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def rule(words, path=None):
    return failed("invalid_road_profile", f"Boykesit kurallarına uymuyor: {words}.", path)


def vertical_refusal(d):
    pvis = d["pvis"]
    idx = {p["id"]: i for i, p in enumerate(pvis)}
    try:
        solve(d)
    except vref.Refused as r:
        e = r.kw
        k = e["kind"]
        if k == "NotIncreasing":
            return failed("invalid_road_profile", f"{e['pvi']} PVI'sinin km'si bir öncekinden büyük olmalı.",
                          f"pvis[{idx[e['pvi']]}].station")
        if k == "BadCurve":
            return failed("invalid_road_profile",
                          f"{e['pvi']} PVI'sinde kurp olamaz: başlangıçta ve bitişte kurp yok, eğimi değişmeyen PVI'de "
                          "kurp yok, boy ve yarıçap 0'dan büyük olur.", f"pvis[{idx[e['pvi']]}]")
        if k == "Overlap":
            carries = lambda id_: any(pvis[idx[id_]].get(f) for f in ("length", "lengthIn", "lengthOut", "radius"))
            who = e["b"] if carries(e["b"]) else e["a"]
            return failed("profile_curves_overlap",
                          f"{e['a']}–{e['b']} arasına düşey kurplar sığmıyor: kolları {e['excess']:.3f} m uzun. Kurp "
                          "boyunu ya da yarıçapı küçültün, PVI'ları uzaklaştırın.", f"pvis[{idx[who]}]")
        raise ValueError(k)
    length = float(ALIGNED[d["alignment"]].length)
    for i, p in enumerate(pvis):
        if p["station"] < -1e-9 or p["station"] > length + 1e-9:
            return failed("invalid_road_profile",
                          f"{p['id']} PVI'si aksın dışında: aksın km'si 0 ile {length:.3f} m arası.",
                          f"pvis[{i}].station")
    raise AssertionError("builds")


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_profile_command_cases.py yazar; aks road_alignment_cases.py'nin, düşey "
                    "güzergâh road_vertical_cases.py'nin (kesirler ve mpmath), yüzey Triangle'ın, arazi boykesiti "
                    "road_profile_cases.py'nin başvurusundandır; KentOS kodu yok. `expect.road` projenin yol "
                    "ayarlarıdır. Boykesit geri alma geçmişinde tutulan bir proje ayarıdır.",
            "setup": base, "cases": cases}


def case(name, steps, setup_=None):
    c = {"name": name, "steps": steps}
    if setup_ is not None:
        c["setup"] = setup_
    return c


# ── road.profiles.create ───────────────────────────────────────────────────────────────────────────────────────────

def create_cases():
    start = [PART]
    cases = []

    def ok(name, input_, d, op="execute", undo=False, before=start, setup_=None):
        after = before + [d]
        out = written(after, d["id"])
        if op == "validate":
            result, expect = completed(None), {**UNCHANGED, "road": road_of(before)}
        elif op == "plan":
            result, expect = completed(out), {**UNCHANGED, "road": road_of(before)}
        else:
            result, expect = completed(out), {**STEPPED, "road": road_of(after)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Boykesit oluştur", "expect": {"road": road_of(before), "canRedo": True}})
        cases.append(case(name, steps, setup_))

    def no(name, input_, result, before=start, setup_=None):
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})], setup_))

    def made(id_, name, alignment, pvis, surface=None, rev=None):
        d = {"id": id_, "name": name, "alignment": alignment, "alignmentRevision": rev or REV[alignment],
             "revision": 1, "pvis": pvis}
        if surface:
            d["surface"] = surface
        return d

    ok("Parabol, çember ve asimetrik kurp, yüzeyle: kimliği ilk boş b-1",
       {"name": "Proje kotu", "alignment": "g-1", "surface": "t-1", "pvis": MAIN_PVIS},
       made("b-1", "Proje kotu", "g-1", MAIN_PVIS, "t-1"))
    ok("Aksın ve yüzeyin adıyla, büyük küçük harf ayrımsız",
       {"name": "Proje kotu", "alignment": "ANA aks", "surface": "mevcut ZEMIN", "pvis": MAIN_PVIS},
       made("b-1", "Proje kotu", "g-1", MAIN_PVIS, "t-1"))
    ok("Kendi kimliğiyle", {"name": "Proje kotu", "id": "proje", "alignment": "g-1", "pvis": MAIN_PVIS},
       made("proje", "Proje kotu", "g-1", MAIN_PVIS))
    two = [pv("a", 0, 100), pv("b", 600, 106)]
    ok("Yalnız eğim: iki PVI", {"name": "Düz", "alignment": "g-1", "pvis": two}, made("b-1", "Düz", "g-1", two))
    kink = [pv("a", 0, 100), pv("v1", 250, 104), pv("b", 600, 99)]
    ok("Kurpsuz kırık PVI", {"name": "Kırık", "alignment": "g-1", "pvis": kink}, made("b-1", "Kırık", "g-1", kink))
    side = [pv("a", 0, 110), pv("v1", 300, 104, length=120), pv("b", 650, 112)]
    ok("Yan aks: aksın 3. revizyonunu tutar", {"name": "Yan kot", "alignment": "g-2", "surface": "t-1", "pvis": side},
       made("b-1", "Yan kot", "g-2", side, "t-1"))
    ok("Boş yüzey yazılmaz", {"name": "Düz", "alignment": "g-1", "surface": "", "pvis": two},
       made("b-1", "Düz", "g-1", two))
    ends = [pv("a", 0, 100), pv("v1", 334.693, 108, radius=3000), pv("b", float(ALIGNED["g-1"].length), 101)]
    ok("Aksın başından sonuna", {"name": "Uçtan uca", "alignment": "g-1", "pvis": ends},
       made("b-1", "Uçtan uca", "g-1", ends))
    sag = [pv("a", 50, 108), pv("v1", 300, 100, lengthIn=120, lengthOut=60), pv("b", 500, 104)]
    ok("Asimetrik çukur, en alçak nokta içinde", {"name": "Çukur", "alignment": "g-1", "pvis": sag},
       made("b-1", "Çukur", "g-1", sag))
    ok("Plan yazmaz", {"name": "Proje kotu", "alignment": "g-1", "pvis": MAIN_PVIS},
       made("b-1", "Proje kotu", "g-1", MAIN_PVIS), op="plan")
    ok("Doğrulama yazmaz", {"name": "Proje kotu", "alignment": "g-1", "pvis": MAIN_PVIS},
       made("b-1", "Proje kotu", "g-1", MAIN_PVIS), op="validate")
    ok("Tek geri alma adımı", {"name": "Proje kotu", "alignment": "g-1", "pvis": MAIN_PVIS},
       made("b-1", "Proje kotu", "g-1", MAIN_PVIS), undo=True)
    ok("İlk boşluk: b-1 varken b-2", {"name": "İkinci", "alignment": "g-1", "pvis": two},
       made("b-2", "İkinci", "g-1", two), before=[MAIN, PART], setup_=setup([MAIN, PART]))

    no("Yol projesi değil", {"name": "x", "alignment": "g-1", "pvis": two}, NOT_ROAD, before=None,
       setup_=setup(work="map"))
    no("Olmayan aks", {"name": "x", "alignment": "yok", "pvis": two},
       failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Olmayan yüzey", {"name": "x", "alignment": "g-1", "surface": "yok", "pvis": two},
       failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Paralel aksa bağlanmaz", {"name": "x", "alignment": "g-3", "pvis": two},
       rule("boykesit yalnız PI'larıyla kurulan aksa bağlanır", "alignment"))
    no("Boş ad", {"name": "", "alignment": "g-1", "pvis": two},
       rule("boykesitin adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter", "name"))
    no("Aynı ad", {"name": "kısmi KOT", "alignment": "g-1", "pvis": two}, rule("bu adda başka bir boykesit var", "name"))
    no("Aynı kimlik", {"name": "x", "id": "b-3", "alignment": "g-1", "pvis": two},
       rule("bu kimlikte başka bir boykesit var", "id"))
    no("Tek PVI", {"name": "x", "alignment": "g-1", "pvis": [pv("a", 0, 100)]},
       rule("boykesitin 2 ile 10000 arası PVI'si olur", "pvis"))
    no("İki PVI'nin kimliği aynı", {"name": "x", "alignment": "g-1", "pvis": [pv("a", 0, 100), pv("a", 100, 101)]},
       rule("PVI'nin kimliği boykesitte tek ve boşluksuz olmalı", "pvis[1].id"))
    no("Bir PVI'de iki kurp türü",
       {"name": "x", "alignment": "g-1", "pvis": [pv("a", 0, 100), pv("v1", 300, 94, length=100, radius=2000), pv("b", 600, 100)]},
       rule("bir PVI'de tek kurp olur: length, lengthIn ile lengthOut ya da radius", "pvis[1].length"))
    no("Asimetrik kurbun tek kolu",
       {"name": "x", "alignment": "g-1", "pvis": [pv("a", 0, 100), pv("v1", 300, 94, lengthIn=100), pv("b", 600, 100)]},
       rule("asimetrik kurbun iki kolu birlikte verilir", "pvis[1].lengthIn"))
    no("Eksi yarıçap",
       {"name": "x", "alignment": "g-1", "pvis": [pv("a", 0, 100), pv("v1", 300, 94, radius=-5), pv("b", 600, 100)]},
       rule("kurp boyu ve yarıçap eksi olamaz", "pvis[1].length"))
    for title, pvis in [
        ("Km geri gidiyor", [pv("a", 0, 100), pv("v1", 300, 94), pv("b", 250, 106)]),
        ("Başlangıçta kurp", [pv("a", 0, 100, length=50), pv("b", 300, 94)]),
        ("Eğimi değişmeyen PVI'de kurp", [pv("a", 0, 100), pv("v1", 300, 106, length=100), pv("b", 600, 112)]),
        ("İki kurp biniyor", [pv("a", 0, 100), pv("v1", 200, 104, length=160), pv("v2", 300, 102, length=100), pv("b", 500, 106)]),
        ("Kurp başlangıçtan taşıyor", [pv("a", 0, 100), pv("v1", 50, 101, length=160), pv("b", 500, 96)]),
        ("Kurp bitişten taşıyor", [pv("a", 0, 100), pv("v1", 450, 109, lengthIn=50, lengthOut=80), pv("b", 500, 106)]),
        ("PVI aksın dışında", [pv("a", 0, 100), pv("v1", 300, 104), pv("b", 700, 106)]),
    ]:
        d = {"alignment": "g-1", "pvis": pvis}
        no(title, {"name": "x", "alignment": "g-1", "pvis": pvis}, vertical_refusal(d))
    no("Geçersiz beklenen sürüm", {"name": "x", "alignment": "g-1", "pvis": two, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"name": "x", "alignment": "g-1", "pvis": two, "expectedRevision": "41"}, CONFLICT)
    return file("road.profiles.create", "Boykesit oluştur", setup(start), cases)


# ── road.profiles.edit ─────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    start = [MAIN, SIDE, PART]
    cases = []

    def ok(name, input_, after, target, op="execute", undo=False):
        out = written(after, target)
        writes = op == "execute" and after != start
        result = completed(None) if op == "validate" else completed(out)
        expect = {**STEPPED, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(start)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Boykesiti düzenle", "expect": {"road": road_of(start), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None, before=start):
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})], setup_))

    def put(i, d):
        out = list(start)
        out[i] = d
        return out

    ok("Adı değişir", {"profile": "b-1", "operation": "set", "name": "Kırmızı kot"},
       put(0, {**MAIN, "name": "Kırmızı kot", "revision": 2}), "b-1")
    moved = [dict(p) for p in MAIN_PVIS]
    moved[2] = pv("v2", 340, 110, radius=1500)
    ok("PVI'ler değişir: çemberin yarıçapı ve kotu", {"profile": "b-1", "operation": "set", "pvis": moved},
       put(0, {**MAIN, "pvis": moved, "revision": 2}), "b-1")
    nosurf = {k: v for k, v in MAIN.items() if k != "surface"}
    ok("Boş yüzey yüzeyi kaldırır", {"profile": "b-1", "operation": "set", "surface": ""},
       put(0, {**nosurf, "revision": 2}), "b-1")
    ok("Yüzey adıyla verilir", {"profile": "b-3", "operation": "set", "surface": "Mevcut zemin"},
       put(2, {**PART, "surface": "t-1", "revision": 2}), "b-3")
    ok("Boykesit adıyla, büyük küçük harf ayrımsız", {"profile": "PROJE kotu", "operation": "set", "name": "Kot"},
       put(0, {**MAIN, "name": "Kot", "revision": 2}), "b-1")
    ok("Aynı değerler: değişiklik yok, adım yok", {"profile": "b-1", "operation": "set", "name": "Proje kotu"},
       start, "b-1")
    ok("Eskiyen boykesit aksa yeniden bağlanır", {"profile": "b-2", "operation": "rebind"},
       put(1, {**SIDE, "alignmentRevision": 3, "revision": 5}), "b-2")
    ok("Güncel boykesiti bağlamak bir şey değiştirmez", {"profile": "b-1", "operation": "rebind"}, start, "b-1")
    ok("Boykesit silinir", {"profile": "b-3", "operation": "remove"}, [MAIN, SIDE], None)
    ok("Silme tek geri alma adımı", {"profile": "b-1", "operation": "remove"}, [SIDE, PART], None, undo=True)
    ok("Düzenleme tek geri alma adımı", {"profile": "b-1", "operation": "set", "name": "Kot"},
       put(0, {**MAIN, "name": "Kot", "revision": 2}), "b-1", undo=True)
    ok("Plan yazmaz", {"profile": "b-1", "operation": "set", "name": "Kot"},
       put(0, {**MAIN, "name": "Kot", "revision": 2}), "b-1", op="plan")
    ok("Doğrulama yazmaz", {"profile": "b-2", "operation": "rebind"},
       put(1, {**SIDE, "alignmentRevision": 3, "revision": 5}), "b-2", op="validate")

    no("Yol projesi değil", {"profile": "b-1", "operation": "remove"}, NOT_ROAD, setup(work="map"), before=None)
    no("Olmayan boykesit", {"profile": "yok", "operation": "remove"},
       failed("unknown_road_profile", "“yok” kimlikli ya da adlı boykesit projede yok.", "profile"))
    no("Değiştirilecek bir şey yok", {"profile": "b-1", "operation": "set"},
       failed("invalid_road_profile", "Değiştirilecek bir şey yok: name, pvis ya da surface verin."))
    no("Olmayan yüzey", {"profile": "b-1", "operation": "set", "surface": "yok"},
       failed("unknown_terrain", "“yok” kimlikli ya da adlı yüzey projede yok.", "surface"))
    no("Başka boykesitin adı", {"profile": "b-1", "operation": "set", "name": "yan KOT"},
       rule("bu adda başka bir boykesit var", "name"))
    no("Tek PVI", {"profile": "b-1", "operation": "set", "pvis": [pv("a", 0, 100)]},
       rule("boykesitin 2 ile 10000 arası PVI'si olur", "pvis"))
    for title, pvis in [
        ("Kurplar biniyor", [pv("a", 0, 100), pv("v1", 200, 104, length=160), pv("v2", 300, 102, length=100), pv("b", 500, 106)]),
        ("PVI aksın dışında", [pv("a", -5, 100), pv("b", 300, 104)]),
        ("Bitişte kurp", [pv("a", 0, 100), pv("b", 300, 104, radius=500)]),
    ]:
        no(title, {"profile": "b-1", "operation": "set", "pvis": pvis}, vertical_refusal({"alignment": "g-1", "pvis": pvis}))
    no("Geçersiz beklenen sürüm", {"profile": "b-1", "operation": "remove", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"profile": "b-1", "operation": "remove", "expectedRevision": "41"}, CONFLICT)
    return file("road.profiles.edit", "Boykesiti düzenle", setup(start), cases)


# ── road.profiles.query ────────────────────────────────────────────────────────────────────────────────────────────

def stale_warning(d):
    return {"code": "profile_stale",
            "message": f"“{d['name']}” boykesitinin aksı değişti (boykesit {d['alignmentRevision']}. revizyona göre "
                       f"kurulu, aks şimdi {REV[d['alignment']]}.); PVI'ları denetleyip aksa yeniden bağlayın.",
            "path": "profile"}


def gone_warning(d, why):
    return {"code": "profile_surface_gone", "message": f"“{d['name']}” boykesitinin yüzeyi {why}; siyah kot verilmedi.",
            "path": "profile"}


def query_cases():
    start = [MAIN, SIDE, PART]
    cases = []

    def q(name, input_, out, warns=(), setup_=None, before=start, op="execute"):
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road_of(before)})], setup_))

    def no(name, input_, result, setup_=None, before=start):
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})], setup_))

    def one(d, **extra):
        return {"profiles": [report(d)], "at": extra.get("at", []), "ticks": extra.get("ticks", []),
                "terrain": extra.get("terrain", [])}

    q("Bütün boykesitler; eskiyen uyarılır", {}, {"profiles": [report(d) for d in start], "at": [], "ticks": [],
                                                  "terrain": []}, [stale_warning(SIDE)])
    q("Bir boykesit adıyla", {"profile": "proje KOTU"}, one(MAIN))
    curves, _, _, _ = solve(MAIN)
    marks = [0.0, 100.0, 130.0, 170.0, 210.0, 250.0, 340.0, 470.0, 520.0, 555.0, 600.0, 669.0,
             float(ALIGNED["g-1"].length)]
    q("Kırmızı ve siyah kot: eğimde, kurpların başında, ortasında ve sonunda, aksın sonunda",
      {"profile": "b-1", "stations": marks}, one(MAIN, at=[at(MAIN, s) for s in marks]))
    q("Her 50 m'nin kotu", {"profile": "b-1", "every": 50.0},
      one(MAIN, ticks=[at(MAIN, s) for s in ticks(ALIGNED["g-1"], 50)]))
    q("Arazi boykesiti: kesişimler, eleman uçları, yüzeyin dışı", {"profile": "b-1", "terrain": True},
      one(MAIN, terrain=ground("g-1")))
    q("Arazi boykesiti ve km'ler birlikte", {"profile": "b-1", "terrain": True, "every": 100.0},
      one(MAIN, terrain=ground("g-1"), ticks=[at(MAIN, s) for s in ticks(ALIGNED["g-1"], 100)]))
    part = [50.0, 100.0, 250.0, 400.0, 500.0]
    q("Kısmi boykesit: ilk ve son PVI'nin dışında kırmızı kot yok, yüzeysiz siyah kot yok",
      {"profile": "b-3", "stations": part}, one(PART, at=[at(PART, s) for s in part]))
    q("Eskiyen boykesit uyarılır, yine sorulur", {"profile": "b-2", "stations": [0.0, 300.0, 650.0]},
      one(SIDE, at=[at(SIDE, s) for s in (0.0, 300.0, 650.0)]), [stale_warning(SIDE)])
    q("Yan aksın arazi boykesiti", {"profile": "Yan kot", "terrain": True}, one(SIDE, terrain=ground("g-2")),
      [stale_warning(SIDE)])
    elems = [float(e.station) for e in ALIGNED["g-1"].els]
    q("Elemanların başında", {"profile": "b-1", "stations": elems}, one(MAIN, at=[at(MAIN, s) for s in elems]))
    broken = {**MAIN, "surface": "t-2"}
    q("Kurulamayan yüzey: siyah kot yok, uyarı", {"profile": "b-1", "stations": [100.0, 300.0]},
      {"profiles": [report(broken)], "at": [at(broken, s, False) for s in (100.0, 300.0)], "ticks": [], "terrain": []},
      [gone_warning(broken, "“Kurulamaz” kurulamıyor")], setup([broken, SIDE, PART]), [broken, SIDE, PART])
    missing = {**MAIN, "surface": "t-yok"}
    q("Projede olmayan yüzey: siyah kot yok, uyarı", {"profile": "b-1", "stations": [100.0]},
      {"profiles": [report(missing)], "at": [at(missing, 100.0, False)], "ticks": [], "terrain": []},
      [gone_warning(missing, "(t-yok) projede yok")], setup([missing, SIDE, PART]), [missing, SIDE, PART])
    q("Kurulamayan yüzeyde arazi boykesiti boş", {"profile": "b-1", "terrain": True},
      {"profiles": [report(broken)], "at": [], "ticks": [], "terrain": []},
      [gone_warning(broken, "“Kurulamaz” kurulamıyor")], setup([broken, SIDE, PART]), [broken, SIDE, PART])
    q("Doğrulama uyarıları verir", {"profile": "b-2"}, None, [stale_warning(SIDE)], op="validate")
    q("Plan cevabı verir", {"profile": "b-1", "stations": [200.0]}, one(MAIN, at=[at(MAIN, 200.0)]), op="plan")
    q("Boykesit yok: boş liste", {}, {"profiles": [], "at": [], "ticks": [], "terrain": []}, setup_=setup([]), before=[])

    length = float(ALIGNED["g-1"].length)
    no("Yol projesi değil", {}, NOT_ROAD, setup(work="map"), before=None)
    no("Olmayan boykesit", {"profile": "yok"}, failed("unknown_road_profile", "“yok” kimlikli ya da adlı boykesit projede yok.", "profile"))
    no("Aksın dışındaki km", {"profile": "b-1", "stations": [10.0, 700.0]},
       failed("station_outside", f"{fmt(700.0)} m aksın üstünde değil (aks {length:.3f} m).", "stations[1]"))
    no("Eksi km", {"profile": "b-1", "stations": [-1.0]},
       failed("station_outside", f"{fmt(-1.0)} m aksın üstünde değil (aks {length:.3f} m).", "stations[0]"))
    no("Sıfır aralık", {"profile": "b-1", "every": 0.0},
       failed("invalid_places", "Km'ler sonlu sayılar, every 0'dan büyük olmalı.", "stations"))
    no("Boykesitsiz soru", {"stations": [10.0]},
       failed("invalid_places", "Kot bir boykesitte sorulur: boykesitin kimliğini ya da adını (profile) verin.", "profile"))
    no("Yüzeysiz boykesitin arazisi", {"profile": "b-3", "terrain": True},
       failed("no_terrain", "“Kısmi kot” boykesitinin yüzeyi yok: arazi boykesiti için önce bir yüzey verin.", "terrain"))
    return file("road.profiles.query", "Boykesiti sorgula", setup(start), cases)


def main():
    files = {"road.profiles.create.json": create_cases(), "road.profiles.edit.json": edit_cases(),
             "road.profiles.query.json": query_cases()}
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
