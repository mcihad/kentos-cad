#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.volumes.{edit,query}.json: the shared cases of KentOS Yol's earthwork volume commands
(plan/road.md Y8, docs/adr/0906), their expectations built here without KentOS code: the drawing and its corridors are
scripts/fixtures/road_corridor_command_cases.py's (each station's section, its cut and fill areas, the section
reference's); the volumes between sections by the average end area or the prismoidal formula with a middle section built
by the same reference; between the corridor's top surface and the terrain scripts/fixtures/road_volume_cases.py's
overlay in fractions, strip by strip. The desktop (crates/native/application/tests/all/fixtures.rs) and Python
(python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_template_cases as tref  # noqa: E402
import road_volume_cases as vref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

completed, failed, step, case = ccmd.completed, ccmd.failed, ccmd.step, ccmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = ccmd.UNCHANGED, ccmd.STEPPED, ccmd.CONFLICT, ccmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Kübaj yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
CORRIDORS = ccmd.START

V1 = {"id": "v-1", "name": "Ana kübaj", "corridor": "c-1", "revision": 1, "method": "endArea",
      "factors": [{"from": 0.0, "to": 300.0, "cut": 1.25, "fill": 0.9}, {"from": 300.0, "to": 600.0, "cut": 1.2}]}
V2 = {"id": "v-2", "name": "Yan kübaj", "corridor": "c-2", "revision": 1, "method": "surfaces"}
START = [V1, V2]

TERRAIN = [[(pcmd.TIN.xy[k][0], pcmd.TIN.xy[k][1], pcmd.TIN.z[k]) for k in t] for t in pcmd.TIN.tris]


def setup(volumes=START, corridors=CORRIDORS, work="road"):
    base = ccmd.setup(corridors, work=work, volumes=volumes)
    base["name"] = "Yol kübajı"
    return base


def road_of(volumes, corridors=CORRIDORS):
    return ccmd.road_of(corridors, volumes=volumes)


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(message)
        self.result = failed(code, message, path)


def warn(code, message):
    return {"code": code, "message": message, "path": "volume"}


def factors_at(v, m):
    rows = v.get("factors", [])
    for i, f in enumerate(rows):
        if f["from"] <= m and (m < f["to"] or (i + 1 == len(rows) and m <= f["to"])):
            return f.get("cut"), f.get("fill")
    return None, None


def section_row(c, r, s):
    """The section reference's row at station `s` of corridor `c`'s region `r`."""
    sup = c.get("superelevation")
    if sup:
        d = next(x for x in ccmd.SUPERS if x["id"] == sup)
        if ccmd.supcmd.build(d)[1]:
            sup = None
    fake = {"name": c["name"], "alignment": c["alignment"], "surface": c["surface"], "profile": c["profile"],
            "template": r["template"], "superelevation": sup, "left": c["left"], "right": c["right"]}
    return scmd.sections(fake, [s], ccmd.TEMPLATES)[0][0]


def mid_areas(c, r, s):
    """The cut and fill areas of the section at station `s` of corridor `c`'s region `r`; none where none is built."""
    row = section_row(c, r, s)
    return (row["cut"], row["fill"]) if "left" in row else None


def depth_at(v, s):
    rows = v.get("strip", [])
    for i, f in enumerate(rows):
        if f["from"] <= s and (s < f["to"] or (i + 1 == len(rows) and s <= f["to"])):
            return f["depth"]
    return 0.0


def earthwork(c, r, s, v):
    """A section's cut, fill, topsoil and benches (docs/adr/0906 §6) in fractions: the design line against the terrain
    lowered by the topsoil depth; the topsoil the depth times the footprint where the terrain is; the benches half their
    height times the width where the stripped terrain under a fill falls steeper than the bench's slope. None where no
    section is built."""
    row = section_row(c, r, s)
    if "left" not in row:
        return None
    depth = F(depth_at(v, s))
    design = [(F(o), F(z)) for o, z in reversed(row["left"]["points"])] + [(F(o), F(z)) for o, z in row["right"]["points"][1:]]
    ground = [{"offset": F(g["offset"]), "z": None if g.get("z") is None else F(g["z"]) - depth, "gap": g["gap"]}
              for g in row["ground"]]
    cut, fill = tref.areas(design, ground)
    t = [(g["offset"], g["z"], g["gap"]) for g in ground]
    lo, hi = design[0][0], design[-1][0]
    stops = sorted({p[0] for p in design} | {g["offset"] for g in ground if lo <= g["offset"] <= hi})

    def line(o):
        for a, b in zip(design, design[1:]):
            if a[0] <= o <= b[0]:
                return b[1] if a[0] == b[0] else a[1] + (b[1] - a[1]) * (o - a[0]) / (b[0] - a[0])

    covered = benched = F(0)
    bench = v.get("bench")
    for a, b in zip(stops, stops[1:]):
        if tref.height(t, (a + b) / 2) is None:
            continue
        ga, gb = tref.height(t, a), tref.height(t, b)
        if ga is None or gb is None:
            continue
        covered += b - a
        if bench is None or abs((gb - ga) / (b - a)) * 100 < F(bench["slope"]):
            continue
        fa, fb = line(a) - ga, line(b) - gb
        if fa > 0 and fb > 0:
            benched += b - a
        elif fa > 0:
            benched += (b - a) * fa / (fa - fb)
        elif fb > 0:
            benched += (b - a) * fb / (fb - fa)
    out = {"cut": float(cut), "fill": float(fill), "strip": float(depth * covered), "bench": 0.0}
    if bench is not None:
        out["bench"] = float(benched * F(bench["height"]) / 2)
    return out


def bbox(t):
    return min(p[0] for p in t), min(p[1] for p in t), max(p[0] for p in t), max(p[1] for p in t)


def surfaces(design):
    """The volumes between a strip's triangles and the terrain, by the fraction reference (ground triangles whose box
    misses the strip's box left out: they meet nothing of it)."""
    if not design:
        return vref.between([], [])
    x0 = min(bbox(t)[0] for t in design)
    y0 = min(bbox(t)[1] for t in design)
    x1 = max(bbox(t)[2] for t in design)
    y1 = max(bbox(t)[3] for t in design)
    near = [g for g in TERRAIN if not (bbox(g)[2] < x0 or bbox(g)[0] > x1 or bbox(g)[3] < y0 or bbox(g)[1] > y1)]
    return vref.between(design, near)


def compute(v, corridors=CORRIDORS):
    """The volume's report and warnings; Refused where it does not compute."""
    c = next((x for x in corridors if x["id"] == v["corridor"]), None)
    if c is None:
        raise Refused("unknown_corridor", f"“{v['corridor']}” kimlikli koridor projede yok.", "corridor")
    try:
        cr, cwarns, _, regs = ccmd.make(c)
    except ccmd.Refused as r:
        raise Refused("invalid_volume", f"“{c['name']}” koridoru kurulamıyor: {r.result['error']['message']}",
                      "corridor") from None
    warnings = [w for w in cwarns if w["code"] == "corridor_stale"]
    raws = []
    areas = {"cut": 0.0, "fill": 0.0, "uncovered": 0.0}
    method = v["method"]
    if method == "surfaces" and c["surface"] != "t-1":
        raise Refused("invalid_volume", f"“{c['name']}” koridorunun yüzeyi kurulamıyor; yüzeyler arası hacim hesaplanamaz.",
                      "corridor")
    worked = bool(v.get("strip")) or v.get("bench") is not None
    for ri, reg in enumerate(regs):
        region = c["regions"][ri]
        for x, y in zip(reg["crosses"], reg["crosses"][1:]):
            s0, s1 = x["station"], y["station"]
            length = s1 - s0
            a, b = x["section"], y["section"]
            vol = extra = None
            if a is not None and b is not None and worked:
                ea, eb = earthwork(c, region, s0, v), earthwork(c, region, s1, v)
                if method == "endArea":
                    q = {k: length * (ea[k] + eb[k]) / 2 for k in ea}
                else:
                    em = earthwork(c, region, (s0 + s1) / 2, v)
                    q = None if em is None else {k: length * (ea[k] + 4 * em[k] + eb[k]) / 6 for k in ea}
                if q is not None:
                    vol, extra = (q["cut"], q["fill"]), (q["strip"], q["bench"])
            elif a is not None and b is not None:
                if method == "endArea":
                    vol = (length * (a["cut"] + b["cut"]) / 2, length * (a["fill"] + b["fill"]) / 2)
                elif method == "prismoidal":
                    m = mid_areas(c, region, (s0 + s1) / 2)
                    if m is not None:
                        vol = (length * (a["cut"] + 4 * m[0] + b["cut"]) / 6,
                               length * (a["fill"] + 4 * m[1] + b["fill"]) / 6)
                else:
                    t = reg["template"]
                    got = cref.strip(cref.top(x, t), cref.top(y, t))
                    if not any(cref.area(g) <= 0 for g in got):
                        r = surfaces(got)
                        areas["cut"] += r["cutArea"]
                        areas["fill"] += r["fillArea"]
                        areas["uncovered"] += r["uncovered"]
                        vol = (r["cut"], r["fill"])
            raws.append((s0, s1, vol, extra))
    text = lambda s: ccmd.km_text(c["alignment"], s)  # noqa: E731
    cut = fill = fcut = ffill = strip = benched = 0.0
    unfactored = 0
    rows = []
    for s0, s1, vol, extra in raws:
        cf, ff = factors_at(v, (s0 + s1) / 2)
        row = {"from": s0, "to": s1, "fromText": text(s0), "toText": text(s1)}
        if vol is not None:
            cut += vol[0]
            fill += vol[1]
            fcut += vol[0] * (cf if cf is not None else 1.0)
            ffill += vol[1] * (ff if ff is not None else 1.0)
            if cf is None or ff is None:
                unfactored += 1
            row["cut"], row["fill"] = vol
        if extra is not None:
            strip += extra[0]
            benched += extra[1]
            if v.get("strip"):
                row["strip"] = extra[0]
            if v.get("bench") is not None:
                row["bench"] = extra[1]
        if cf is not None:
            row["cutFactor"] = cf
        if ff is not None:
            row["fillFactor"] = ff
        row.update({"totalCut": fcut, "totalFill": ffill, "mass": fcut - ffill})
        rows.append(row)
    gaps = sorted([list(g) for g in cr["gaps"]] + [[s0, s1] for s0, s1, vol, _ in raws if vol is None], key=lambda g: g[0])
    joined = []
    for g in gaps:
        if joined and g[0] <= joined[-1][1] + 1e-9:
            joined[-1][1] = max(joined[-1][1], g[1])
        else:
            joined.append(list(g))
    if joined:
        words = ", ".join(f"{text(g[0])} – {text(g[1])}" for g in joined[:8])
        more = f" ve {len(joined) - 8} aralık daha" if len(joined) > 8 else ""
        warnings.append(warn("volume_gap", f"“{v['name']}” kübajında hacmi olmayan aralıklar: {words}{more}."))
    if method == "surfaces" and areas["uncovered"] > 1e-6:
        warnings.append(warn("volume_uncovered", f"“{v['name']}” kübajında tasarım yüzeyinin {areas['uncovered']:.3f} m²'sini "
                                                 "arazi örtmüyor; o alanın hacmi yok."))
    if unfactored:
        warnings.append(warn("volume_unfactored", f"“{v['name']}” kübajının {unfactored} aralığına kabarma ya da sıkışma "
                                                  "katsayısı verilmedi; orada düzeltilmiş hacim ham hacimdir."))
    report = {"id": v["id"], "name": v["name"], "revision": v.get("revision", 0), "corridor": v["corridor"],
              "method": method, "stale": cr["stale"], "rows": rows, "cut": cut, "fill": fill,
              "factoredCut": fcut, "factoredFill": ffill, "gaps": joined}
    if method == "surfaces":
        report.update({"cutArea": areas["cut"], "fillArea": areas["fill"], "uncovered": areas["uncovered"]})
    if v.get("strip"):
        report["strip"] = strip
    if v.get("bench") is not None:
        report["bench"] = benched
    return report, warnings


def broken(v, why):
    return {"id": v["id"], "name": v["name"], "revision": v.get("revision", 0), "corridor": v["corridor"],
            "method": v["method"], "stale": False, "rows": [], "cut": 0.0, "fill": 0.0, "factoredCut": 0.0,
            "factoredFill": 0.0, "gaps": [], "problem": why}


def report(v, corridors=CORRIDORS):
    try:
        return compute(v, corridors)[0]
    except Refused as r:
        return broken(v, r.result["error"]["message"])


def listed(volumes):
    return [{"id": v["id"], "name": v["name"], "corridor": v["corridor"], "method": v["method"],
             "revision": v.get("revision", 0)} for v in volumes]


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_volume_command_cases.py yazar; çizim ve koridorlar "
                    "road_corridor_command_cases.py'nin, kesitlerin alanları enkesit başvurusunun, yüzeyler arası hacim "
                    "road_volume_cases.py'nin kesirli bindirmesidir; KentOS kodu yok. `expect.road` projenin yol "
                    "ayarlarıdır. Kübaj geri alma geçmişinde tutulan bir proje ayarıdır; sonucu her okumada hesaplanır.",
            "setup": base, "cases": cases}


# ── road.volumes.edit ──────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    cases = []

    def written(after, target):
        out = {"volumes": listed(after), "revision": "$current"}
        if target:
            out["volume"] = report(next(v for v in after if v["id"] == target))
        return out

    def ok(name, input_, after, target, op="execute", undo=False):
        writes = op == "execute" and after != START
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(START)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Kübaj", "expect": {"road": road_of(START), "canRedo": True}})
        cases.append(case(name, steps))

    def no(name, input_, result, setup_=None):
        road = setup_["settings"].get("road") if setup_ is not None else road_of(START)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})], setup_))

    def rulefail(words, path=None):
        return failed("invalid_volume", f"Kübaj hesabı kurallarına uymuyor: {words}.", path)

    def put(i, v):
        out = list(START)
        out[i] = v
        return out

    def new(id_, **kw):
        v = {"id": id_, "name": "Yeni kübaj", "corridor": "c-1", "revision": 1, "method": "endArea"}
        v.update(kw)
        return v

    base = {"operation": "set", "name": "Yeni kübaj", "corridor": "c-1", "method": "endArea"}
    ok("Yeni kübaj: kimliği ilk boş v-3", base, START + [new("v-3")], "v-3")
    ok("Koridor adıyla, kendi kimliğiyle", {**base, "volume": "kendi", "corridor": "YAN koridor"},
       START + [new("kendi", corridor="c-2")], "kendi")
    ok("Prizmoidal yöntem", {**base, "method": "prismoidal"}, START + [new("v-3", method="prismoidal")], "v-3")
    ok("Yüzeyler arası yöntem", {**base, "method": "surfaces"}, START + [new("v-3", method="surfaces")], "v-3")
    f2 = [{"from": 0.0, "to": 150.0, "cut": 1.3, "fill": 0.85}, {"from": 150.0, "to": 600.0, "fill": 0.95}]
    ok("Katsayılar km aralıklarına", {**base, "factors": f2}, START + [new("v-3", factors=f2)], "v-3")
    ok("Adı değişir", {"operation": "set", "volume": "v-1", "name": "Kübaj A"}, put(0, {**V1, "name": "Kübaj A", "revision": 2}), "v-1")
    ok("Yöntem değişir", {"operation": "set", "volume": "Ana kübaj", "method": "prismoidal"},
       put(0, {**V1, "method": "prismoidal", "revision": 2}), "v-1")
    ok("Katsayılar kaldırılır", {"operation": "set", "volume": "v-1", "factors": []},
       put(0, {k: val for k, val in V1.items() if k != "factors"} | {"revision": 2}), "v-1")
    ok("Aynı koridor yeniden verilir", {"operation": "set", "volume": "v-2", "corridor": "c-2", "name": "Yan kübaj"}, START, "v-2")
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "volume": "v-1", "name": "Ana kübaj"}, START, "v-1")
    ok("Kübaj silinir", {"operation": "remove", "volume": "v-2"}, [V1], None)
    ok("Silme tek geri alma adımı", {"operation": "remove", "volume": "Ana kübaj"}, [V2], None, undo=True)
    ok("Plan yazmaz", {"operation": "set", "volume": "v-1", "name": "Kübaj A"},
       put(0, {**V1, "name": "Kübaj A", "revision": 2}), "v-1", op="plan")
    ok("Doğrulama yazmaz", {"operation": "set", "volume": "v-2", "method": "endArea"},
       put(1, {**V2, "method": "endArea", "revision": 2}), "v-2", op="validate")

    strips = [{"from": 0.0, "to": 300.0, "depth": 0.3}, {"from": 300.0, "to": 600.0, "depth": 0.2}]
    bench = {"slope": 12.0, "height": 0.5}
    ok("Sıyırma ve diş kazısıyla yeni kübaj", {**base, "strip": strips, "bench": bench},
       START + [new("v-3", strip=strips, bench=bench)], "v-3")
    ok("Sıyırma eklenir", {"operation": "set", "volume": "v-1", "strip": strips[:1]},
       put(0, {**V1, "strip": strips[:1], "revision": 2}), "v-1")
    vb = {**V1, "bench": bench, "revision": 1}
    cases.append(case("Diş kazısı null ile kaldırılır", [step(
        "execute", {"operation": "set", "volume": "v-1", "bench": None},
        completed({"volumes": listed([{**V1, "revision": 2}, V2]), "revision": "$current", "volume": report({**V1, "revision": 2})}),
        {**STEPPED, "road": road_of([{**V1, "revision": 2}, V2])})], setup([vb, V2])))
    no("Yol projesi değil", {"operation": "remove", "volume": "v-1"}, NOT_ROAD, setup(work="map"))
    no("Yeniden bağlanmaz", {"operation": "rebind", "volume": "v-1"},
       failed("invalid_volume", "Kübaj hesabı koridorunu izler, yeniden bağlanmaz; koridoru aksa yeniden bağlayın.", "operation"))
    no("Olmayan kübaj silinmez", {"operation": "remove", "volume": "yok"},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    no("Olmayan koridor", {**base, "corridor": "yok"},
       failed("unknown_corridor", "“yok” kimlikli ya da adlı koridor projede yok.", "corridor"))
    no("Başka koridora bağlanmaz", {"operation": "set", "volume": "v-1", "corridor": "c-2"},
       failed("invalid_volume", "“Ana kübaj” kübaj hesabının koridoru değiştirilmez; başka koridor için yeni bir kübaj "
                                "hesabı yazın.", "corridor"))
    no("Değiştirilecek bir şey yok", {"operation": "set", "volume": "v-1"},
       failed("invalid_volume", "Değiştirilecek bir şey yok: name, method, factors, strip ya da bench verin."))
    no("Yeni kübajın yöntemi yok", {k: val for k, val in base.items() if k != "method"},
       failed("invalid_volume", "Yeni kübaj hesabı için name, corridor ve method verilmeli.", "method"))
    no("Aynı ad", {**base, "name": "YAN kübaj"}, rulefail("bu adda başka bir kübaj hesabı var", "name"))
    no("Sıfır katsayı", {**base, "factors": [{"from": 0.0, "to": 100.0, "cut": 0.0}]},
       rulefail("katsayı 0'dan büyük, en çok 10 olur", "factors[0]"))
    no("On'dan büyük katsayı", {**base, "factors": [{"from": 0.0, "to": 100.0, "fill": 12.0}]},
       rulefail("katsayı 0'dan büyük, en çok 10 olur", "factors[0]"))
    no("Binen katsayı satırları", {**base, "factors": [{"from": 0.0, "to": 200.0, "cut": 1.2},
                                                      {"from": 150.0, "to": 300.0, "cut": 1.1}]},
       rulefail("katsayı satırları artan km'lerle, birbirine binmeden yazılır", "factors[1]"))
    no("Boş katsayı satırı", {**base, "factors": [{"from": 100.0, "to": 100.0, "cut": 1.2}]},
       rulefail("katsayı satırının sonu başından sonra olmalı", "factors[0]"))
    broken_corr = dict(ccmd.C1)
    broken_corr["regions"] = [{"from": 0.0, "to": 100.0, "template": "tk-1"}, {"from": 50.0, "to": 200.0, "template": "tk-1"}]
    no("Kurulamayan koridorun kübajı", base,
       failed("invalid_volume", "“Ana koridor” koridoru kurulamıyor: 1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.",
              "corridor"), setup(START, [broken_corr, ccmd.C2]))
    no("Kurulamayan yüzeyle yüzeyler arası", {**base, "method": "surfaces"},
       failed("invalid_volume", "“Ana koridor” koridorunun yüzeyi kurulamıyor; yüzeyler arası hacim hesaplanamaz.", "corridor"),
       setup(START, [{**ccmd.C1, "surface": "t-2"}, ccmd.C2]))
    no("Yüzeyler arasında sıyırma olmaz", {"operation": "set", "volume": "v-2", "strip": strips},
       rulefail("sıyırma ve diş kazısı yalnız kesit yöntemleriyle (ortalama alan, prizmoidal) hesaplanır", "method"))
    no("Binen sıyırma satırları", {**base, "strip": [{"from": 0.0, "to": 200.0, "depth": 0.3},
                                                    {"from": 100.0, "to": 300.0, "depth": 0.3}]},
       rulefail("sıyırma satırları artan km'lerle, birbirine binmeden yazılır", "strip[1]"))
    no("Çok kalın bitkisel toprak", {**base, "strip": [{"from": 0.0, "to": 200.0, "depth": 6.0}]},
       rulefail("bitkisel toprak kalınlığı 0'dan büyük, en çok 5 m olur", "strip[0]"))
    no("Sıfır eğimli diş kazısı", {**base, "bench": {"slope": 0.0, "height": 0.5}},
       rulefail("diş kazısının eğimi 0'dan büyük, en çok %1000; basamak yüksekliği 0'dan büyük, en çok 10 m olur", "bench"))
    no("Geçersiz beklenen sürüm", {"operation": "remove", "volume": "v-2", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "remove", "volume": "v-2", "expectedRevision": "41"}, CONFLICT)
    return file("road.volumes.edit", "Kübaj hesabını düzenle", setup(), cases)


# ── road.volumes.query ─────────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    cases = []

    def q(name, input_, volumes=START, corridors=CORRIDORS, op="execute"):
        one = input_.get("volume")
        if one is None:
            reports, warns = [], []
            for v in volumes:
                try:
                    r, w = compute(v, corridors)
                    reports.append(r)
                    warns += w
                except Refused as e:
                    reports.append(broken(v, e.result["error"]["message"]))
            out = {"volumes": reports}
        else:
            v = next(x for x in volumes if x["id"] == one or x["name"].lower() == one.lower())
            r, warns = compute(v, corridors)
            out = {"volumes": [r]}
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        default = volumes is START and corridors is CORRIDORS
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road_of(volumes, corridors)})],
                          None if default else setup(volumes, corridors)))

    def no(name, input_, result, volumes=START, corridors=CORRIDORS, setup_=None):
        road = None if setup_ is not None else road_of(volumes, corridors)
        default = volumes is START and corridors is CORRIDORS
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if default else setup(volumes, corridors))))

    def v(**kw):
        out = copy.deepcopy(V1)
        for k, val in kw.items():
            if val is None:
                out.pop(k, None)
            else:
                out[k] = val
        return out

    q("Bütün kübajlar; eskiyen koridor uyarılır", {})
    q("Bir kübaj adıyla", {"volume": "ANA kübaj"})
    q("Yüzeyler arası kübaj", {"volume": "v-2"})
    q("Prizmoidal", {"volume": "v-1"}, [v(method="prismoidal"), V2])
    q("Ana koridorda yüzeyler arası", {"volume": "v-1"}, [v(method="surfaces"), V2])
    q("Katsayısız: düzeltilmiş ham", {"volume": "v-1"}, [v(factors=None), V2])
    q("Katsayı yalnız bir aralıkta", {"volume": "v-1"}, [v(factors=[{"from": 100.0, "to": 200.0, "cut": 1.3, "fill": 0.9}]), V2])
    q("Son katsayı satırının sonu dahil", {"volume": "v-1"}, [v(factors=[{"from": 0.0, "to": 600.0, "cut": 1.1, "fill": 1.05}]), V2])
    c_gap = {**ccmd.C1, "regions": [{"from": 0.0, "to": 150.0, "template": "tk-1"}, {"from": 250.0, "to": 400.0, "template": "tk-2"}]}
    q("Koridorun boşluğu hacimsiz", {"volume": "v-1"}, START, [c_gap, ccmd.C2])
    c_narrow = {**ccmd.C1, "left": 6.0, "right": 6.0}
    q("Dar şerit: açık yanlarda da kesit alanı", {"volume": "v-1"}, START, [c_narrow, ccmd.C2])
    c_out = {**ccmd.C1, "profile": "b-3", "regions": [{"from": 50.0, "to": 450.0, "template": "tk-1"}]}
    q("Boykesitin dışı hacimsiz", {"volume": "v-1"}, START, [c_out, ccmd.C2])
    q("Boykesitin dışı, yüzeyler arası", {"volume": "v-1"}, [v(method="surfaces"), V2], [c_out, ccmd.C2])
    c_wide = {**ccmd.C1, "left": 120.0, "right": 120.0, "regions": [{"from": 0.0, "to": 200.0, "template": "tk-1"}]}
    q("Arazinin örtmediği tasarım", {"volume": "v-1"}, [v(method="surfaces"), V2], [c_wide, ccmd.C2])
    q("Devere bağlı koridor, prizmoidal", {"volume": "v-2"}, [V1, {**V2, "method": "prismoidal"}])
    strips = [{"from": 0.0, "to": 300.0, "depth": 0.3}, {"from": 300.0, "to": 600.0, "depth": 0.2}]
    q("Sıyırma: bitkisel toprak ve indirilmiş zemine göre kübaj", {"volume": "v-1"}, [v(strip=strips), V2])
    q("Diş kazısı", {"volume": "v-1"}, [v(bench={"slope": 3.0, "height": 0.5}), V2])
    q("Prizmoidal, sıyırma ve diş kazısı", {"volume": "v-1"},
      [v(method="prismoidal", strip=strips[:1], bench={"slope": 2.0, "height": 0.6}), V2])
    q("Sıyırma yalnız bir aralıkta", {"volume": "v-1"}, [v(strip=[{"from": 150.0, "to": 250.0, "depth": 0.25}]), V2])
    q("Dik eşikte diş kazısı yok", {"volume": "v-1"}, [v(bench={"slope": 900.0, "height": 0.5}), V2])
    q("Devere bağlı koridorda sıyırma", {"volume": "v-2"}, [V1, {**V2, "method": "endArea", "strip": strips}])
    q("Plan cevabı verir", {"volume": "v-1"}, op="plan")
    q("Doğrulama uyarıları verir", {"volume": "v-2"}, op="validate")
    q("Kübaj yok: boş liste", {}, [])
    broken_corr = {**ccmd.C1, "regions": [{"from": 0.0, "to": 100.0, "template": "tk-1"},
                                          {"from": 50.0, "to": 200.0, "template": "tk-1"}]}
    q("Hesaplanamayan kübaj raporda nedeniyle", {}, START, [broken_corr, ccmd.C2])

    no("Yol projesi değil", {}, NOT_ROAD, setup_=setup(work="map"))
    no("Olmayan kübaj", {"volume": "yok"}, failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    no("Hesaplanamayan kübajın sorusu", {"volume": "v-1"},
       failed("invalid_volume", "“Ana koridor” koridoru kurulamıyor: 1. ve 2. bölge kesişiyor; bölgeler en çok uç uca gelir.",
              "corridor"), START, [broken_corr, ccmd.C2])
    no("Kurulamayan yüzeyle yüzeyler arası", {"volume": "v-2"},
       failed("invalid_volume", "“Yan koridor” koridorunun yüzeyi kurulamıyor; yüzeyler arası hacim hesaplanamaz.", "corridor"),
       START, [ccmd.C1, {**ccmd.C2, "surface": "t-2"}])
    return file("road.volumes.query", "Kübajı sorgula", setup(), cases)


def main():
    files = {"road.volumes.edit.json": edit_cases(), "road.volumes.query.json": query_cases()}
    check = "--check" in sys.argv[1:]
    bad = []
    for name, data in files.items():
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
        print("road volume cases are current")


if __name__ == "__main__":
    main()
