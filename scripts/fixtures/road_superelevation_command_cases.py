#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.superelevations.{edit,query}.json: the shared cases of KentOS Yol's superelevation
commands (plan/road.md Y6, docs/adr/0904), their expectations built here without KentOS code: the drawing is
scripts/fixtures/road_section_command_cases.py's (its alignments mpmath at 40 digits), a schedule
scripts/fixtures/road_superelevation_cases.py's (exact fractions of the reference alignment's curve stations). The
desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_superelevation_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import sys
from fractions import Fraction as F
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as aref  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_superelevation_cases as supref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"

completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED, STEPPED, CONFLICT, BAD_REVISION = pcmd.UNCHANGED, pcmd.STEPPED, pcmd.CONFLICT, pcmd.BAD_REVISION
NOT_ROAD = failed("not_a_road_project", "Dever yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
D1, D2 = scmd.D1, scmd.D2
TEMPLATES = [scmd.TK1, scmd.TK2]
SETS = [scmd.S1, scmd.S2]
REV = pcmd.REV
MARKS = {"normalCrown", "levelCrown", "reverseCrown", "full"}


def road_of(supers, sets=SETS):
    return scmd.road_of(TEMPLATES, sets, supers=supers)


def setup(supers=(D1, D2), sets=SETS, work="road"):
    base = scmd.setup(sets=sets, work=work)
    if work == "road":
        base["settings"]["road"] = road_of(list(supers), list(sets))
    return base


def pis_of(alignment):
    return [pcmd.acmd.full(p) for p in next(a for a in pcmd.ALIGNMENTS if a["id"] == alignment)["pis"]]


def build(d):
    """The schedule and critical stations, or the refusal (path, message), by the reference."""
    length = pcmd.ALIGNED[d["alignment"]].length
    m = d["method"]
    if m["method"] == "stations":
        rows = m["rows"]
        for i, r in enumerate(rows):
            if r["station"] < 0 or r["station"] > float(length):
                return None, (f"method.rows[{i}].station",
                              f"{i + 1}. satır aksın dışında: aksın km'si 0 ile {float(length):.3f} m arası.")
        for i, r in enumerate(rows):
            if i and r["station"] <= rows[i - 1]["station"]:
                return None, (f"method.rows[{i}].station", f"{i + 1}. satırın km'si bir öncekinden büyük olmalı.")
        return ({"left": [[F(r["station"]), F(r["left"])] for r in rows],
                 "right": [[F(r["station"]), F(r["right"])] for r in rows], "leftWidening": [], "rightWidening": []},
                []), None
    curves = [{"widening": 0.0, **c} for c in m["curves"]]
    got = supref.curves(pis_of(d["alignment"]), [d["normalLeft"], d["normalRight"]], curves)
    if "error" in got:
        e = got["error"]
        at = lambda pi: next((i for i, c in enumerate(curves) if c["pi"] == pi), 0)  # noqa: E731
        k = e["kind"]
        if k == "NoCurve":
            return None, (f"method.curves[{at(e['pi'])}].pi", f"“{e['pi']}” PI'sinde kurp yok.")
        if k == "Bad":
            return None, (f"method.curves[{at(e['pi'])}]",
                          f"“{e['pi']}” kurbunda oran ve geçiş boyu 0'dan büyük, teğet payı 0 ile 1 arası, genişleme eksi olmamalı.")
        if k == "RateBelowNormal":
            return None, (f"method.curves[{at(e['pi'])}].rate", f"“{e['pi']}” kurbunun dever oranı normal enine eğimden küçük.")
        if k == "TooShort":
            return None, (f"method.curves[{at(e['pi'])}].runoff",
                          f"“{e['pi']}” kurbunda tam dever kurba sığmıyor: geçiş boyunu kısaltın ya da teğet payını büyütün.")
        if k == "OffAlignment":
            return None, (f"method.curves[{at(e['pi'])}]", f"“{e['pi']}” kurbunun geçişi aksın dışına taşıyor.")
        if k == "Twice":
            return None, (f"method.curves[{at(e['pi'])}]", f"“{e['pi']}” PI'si iki kez yazılmış.")
        raise ValueError(e)
    return (got["schedule"], got["critical"]), None


def fl(v):
    return [[float(a), float(b)] for a, b in v]


def report(d):
    b = pcmd.ALIGNED[d["alignment"]]
    out = {"id": d["id"], "name": d["name"], "revision": d.get("revision", 0), "alignment": d["alignment"],
           "stale": REV[d["alignment"]] != d.get("alignmentRevision", 0), "normalLeft": d["normalLeft"],
           "normalRight": d["normalRight"], "left": [], "right": [], "leftWidening": [], "rightWidening": [],
           "critical": []}
    got, why = build(d)
    if why:
        out["problem"] = why[1]
        return out
    sch, crit = got
    out.update({k: fl(sch[k]) for k in ("left", "right", "leftWidening", "rightWidening")})
    for c in crit:
        km = aref.km_of(b.regions, mp.mpf(float(c["station"])))
        out["critical"].append({"pi": c["pi"], "mark": c["mark"], "station": float(c["station"]), "km": float(km),
                                "kmText": pcmd.acmd.km_text(km), "exit": c["exit"]})
    return out


def at(d, s):
    (sch, _), _ = build(d)
    b = pcmd.ALIGNED[d["alignment"]]
    km = aref.km_of(b.regions, mp.mpf(s))
    v = lambda k, n: supref.along(sch[k], F(s)) if sch[k] else n  # noqa: E731
    return {"station": float(s), "km": float(km), "kmText": pcmd.acmd.km_text(km),
            "left": float(v("left", F(d["normalLeft"]))), "right": float(v("right", F(d["normalRight"]))),
            "leftWidening": float(v("leftWidening", F(0))), "rightWidening": float(v("rightWidening", F(0)))}


def stale_warning(d):
    return {"code": "superelevation_stale",
            "message": f"“{d['name']}” dever çizelgesinin aksı değişti (çizelge {d['alignmentRevision']}. revizyona göre "
                       f"kurulu, aks şimdi {REV[d['alignment']]}.); km'leri denetleyip aksa yeniden bağlayın.",
            "path": "superelevation"}


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_superelevation_command_cases.py yazar; çizim "
                    "road_section_command_cases.py'nin, dever çizelgesi road_superelevation_cases.py'nin (başvuru "
                    "aksının kurp km'lerinin kesirleriyle) başvurusundandır; KentOS kodu yok. `expect.road` projenin "
                    "yol ayarlarıdır. Dever çizelgesi geri alma geçmişinde tutulan bir proje ayarıdır.",
            "setup": base, "cases": cases}


def curves(*rules):
    return {"method": "curves", "curves": list(rules)}


def rule(pi, rate, runoff, on_tangent, widening=None):
    out = {"pi": pi, "rate": rate, "runoff": runoff, "onTangent": on_tangent}
    if widening:
        out["widening"] = widening
    return out


# ── road.superelevations.edit ──────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    start = [D1, D2]
    cases = []

    def written(after, target):
        out = {"superelevations": [report(d) for d in after], "revision": "$current"}
        if target:
            out["superelevation"] = report(next(d for d in after if d["id"] == target))
        return out

    def ok(name, input_, after, target, op="execute", undo=False, sets=SETS):
        writes = op == "execute" and after != start
        result = completed(None) if op == "validate" else completed(written(after, target))
        expect = {**STEPPED, "road": road_of(after, sets)} if writes else {**UNCHANGED, "road": road_of(start, sets)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Dever", "expect": {"road": road_of(start, sets), "canRedo": True}})
        cases.append(case(name, steps, None if sets is SETS else setup(sets=sets)))

    def no(name, input_, result, sets=SETS, setup_=None):
        road = None if sets is None else road_of(start, sets)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if sets is SETS else setup(sets=sets))))

    def rulefail(words, path=None):
        return failed("invalid_superelevation", f"Dever çizelgesi kurallarına uymuyor: {words}.", path)

    def new(id_, name, alignment, nl, nr, method):
        return {"id": id_, "name": name, "alignment": alignment, "alignmentRevision": REV[alignment], "revision": 1,
                "normalLeft": nl, "normalRight": nr, "method": method}

    def put(i, d):
        out = list(start)
        out[i] = d
        return out

    m1 = curves(rule("p1", 7.0, 50.0, 0.3))
    ok("Yeni çizelge kurplardan: kimliği ilk boş d-3",
       {"operation": "set", "name": "Yeni dever", "alignment": "g-1", "normalLeft": -2.5, "normalRight": -2.5, "method": m1},
       start + [new("d-3", "Yeni dever", "g-1", -2.5, -2.5, m1)], "d-3")
    rows = {"method": "stations", "rows": [{"station": 50.0, "left": -2.0, "right": -2.0},
                                           {"station": 300.0, "left": 4.0, "right": -4.0}]}
    ok("Kendi kimliğiyle, km satırlarından, aksın adıyla",
       {"operation": "set", "superelevation": "satir", "name": "Satırlar", "alignment": "ANA aks", "normalLeft": -2.0,
        "normalRight": -2.0, "method": rows}, start + [new("satir", "Satırlar", "g-1", -2.0, -2.0, rows)], "satir")
    m2 = curves(rule("p1", 6.0, 40.0, 0.0, 0.8))
    ok("Genişlemeli kurp", {"operation": "set", "name": "Geniş", "alignment": "g-1", "normalLeft": -2.5,
                            "normalRight": -2.5, "method": m2}, start + [new("d-3", "Geniş", "g-1", -2.5, -2.5, m2)], "d-3")
    one_way = curves(rule("p1", 4.0, 40.0, 0.5))
    ok("Tek yöne eğimli normal", {"operation": "set", "name": "Tek yön", "alignment": "g-1", "normalLeft": 2.0,
                                  "normalRight": -2.0, "method": one_way},
       start + [new("d-3", "Tek yön", "g-1", 2.0, -2.0, one_way)], "d-3")
    ok("Adı değişir", {"operation": "set", "superelevation": "d-1", "name": "Dever A"},
       put(0, {**D1, "name": "Dever A", "revision": 2}), "d-1")
    ok("Normal eğimler değişir", {"operation": "set", "superelevation": "d-1", "normalLeft": -3.0, "normalRight": -3.0},
       put(0, {**D1, "normalLeft": -3.0, "normalRight": -3.0, "revision": 2}), "d-1")
    ok("Kurbun oranı değişir", {"operation": "set", "superelevation": "Ana aks deveri", "method": m1},
       put(0, {**D1, "method": m1, "revision": 2}), "d-1")
    ok("Aynı değerler: değişiklik yok, adım yok", {"operation": "set", "superelevation": "d-1", "name": "Ana aks deveri"},
       start, "d-1")
    ok("Eskiyen çizelge aksa yeniden bağlanır", {"operation": "rebind", "superelevation": "d-2"},
       put(1, {**D2, "alignmentRevision": 3, "revision": 3}), "d-2")
    ok("Güncel çizelgeyi bağlamak bir şey değiştirmez", {"operation": "rebind", "superelevation": "d-1"}, start, "d-1")
    ok("Çizelge silinir", {"operation": "remove", "superelevation": "d-2"}, [D1], None)
    ok("Silme tek geri alma adımı", {"operation": "remove", "superelevation": "d-1"}, [D2], None, undo=True)
    ok("Plan yazmaz", {"operation": "set", "superelevation": "d-1", "name": "Dever A"},
       put(0, {**D1, "name": "Dever A", "revision": 2}), "d-1", op="plan")
    ok("Doğrulama yazmaz", {"operation": "rebind", "superelevation": "d-2"},
       put(1, {**D2, "alignmentRevision": 3, "revision": 3}), "d-2", op="validate")

    base = {"operation": "set", "name": "x", "alignment": "g-1", "normalLeft": -2.5, "normalRight": -2.5}
    no("Yol projesi değil", {"operation": "remove", "superelevation": "d-1"}, NOT_ROAD, None, setup(work="map"))
    no("Olmayan çizelge silinmez", {"operation": "remove", "superelevation": "yok"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Olmayan aks", {**base, "alignment": "yok", "method": m1},
       failed("unknown_alignment", "“yok” kimlikli ya da adlı aks projede yok.", "alignment"))
    # The axis of rotation (docs/adr/0931 §2): an edge is kept, the centreline is no field.
    ok("Dönme ekseni iç kenar", {"operation": "set", "superelevation": "d-1", "pivot": "insideEdge"},
       [{**D1, "pivot": "insideEdge", "revision": 2}, D2], "d-1")
    ok("Eksen etrafında: alan yazılmaz, değişiklik yok", {"operation": "set", "superelevation": "d-1", "pivot": "centerline"},
       [D1, D2], "d-1")
    no("Değiştirilecek bir şey yok", {"operation": "set", "superelevation": "d-1"},
       failed("invalid_superelevation", "Değiştirilecek bir şey yok: name, alignment, normalLeft, normalRight, method ya da pivot verin."))
    no("Yeni çizelgenin yöntemi yok", base,
       failed("invalid_superelevation", "Yeni dever çizelgesi için name, alignment, normalLeft, normalRight ve method verilmeli.",
              "method"))
    no("Paralel aksa bağlanmaz", {**base, "alignment": "g-3", "method": m1},
       rulefail("dever çizelgesi yalnız PI'larıyla kurulan aksa bağlanır", "alignment"))
    no("Aynı ad", {**base, "name": "yan AKS deveri", "method": m1}, rulefail("bu adda başka bir dever çizelgesi var", "name"))
    no("Normal eğim çok dik", {**base, "normalLeft": -120.0, "method": m1}, rulefail("normal enine eğim en çok %100 olur", "normalLeft"))
    for title, method, d_ in [
        ("Kurbu olmayan PI", curves(rule("b", 6.0, 40.0, 0.0)), None),
        ("Sıfır oran", curves(rule("p1", 0.0, 40.0, 0.0)), None),
        ("Oran normal eğimden küçük", curves(rule("p1", 2.0, 40.0, 0.0)), None),
        ("Tam dever kurba sığmıyor", curves(rule("p1", 6.0, 400.0, 0.0)), None),
        ("Geçiş aksın başından taşıyor", curves(rule("p1", 6.0, 400.0, 1.0)), None),
        ("Aynı PI iki kez", curves(rule("p1", 6.0, 40.0, 0.0), rule("p1", 5.0, 40.0, 0.0)), None),
        ("Km satırları artmıyor", {"method": "stations", "rows": [{"station": 50.0, "left": -2.0, "right": -2.0},
                                                                  {"station": 50.0, "left": 0.0, "right": -2.0}]}, None),
        ("Satır aksın dışında", {"method": "stations", "rows": [{"station": 50.0, "left": -2.0, "right": -2.0},
                                                               {"station": 900.0, "left": 0.0, "right": -2.0}]}, None),
    ]:
        _, why = build({"alignment": "g-1", "normalLeft": -2.5, "normalRight": -2.5, "method": method})
        no(title, {**base, "method": method}, failed("invalid_superelevation", why[1], why[0]))
    used = [{**scmd.S1, "superelevation": "d-1"}, scmd.S2]
    no("Kesit takımının kullandığı çizelge silinmez", {"operation": "remove", "superelevation": "d-1"},
       failed("superelevation_in_use", "“Ana aks deveri” dever çizelgesini şu kesit takımları kullanıyor: Enkesitler. "
                                       "Önce takımlardan kaldırın.", "superelevation"), used)
    no("Geçersiz beklenen sürüm", {"operation": "remove", "superelevation": "d-2", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"operation": "remove", "superelevation": "d-2", "expectedRevision": "41"}, CONFLICT)
    return file("road.superelevations.edit", "Deveri düzenle", setup(), cases)


# ── road.superelevations.query ─────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    start = [D1, D2]
    cases = []

    def q(name, input_, out, warns=(), supers=start, op="execute"):
        result = completed(None, warns) if op == "validate" else completed(out, warns)
        cases.append(case(name, [step(op, input_, result, {**UNCHANGED, "road": road_of(supers)})],
                          None if supers is start else setup(supers=supers)))

    def no(name, input_, result, supers=start, setup_=None):
        road = None if supers is None else road_of(supers)
        cases.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": road})],
                          setup_ or (None if supers is start else setup(supers=supers))))

    def one(d, stations=()):
        return {"superelevations": [report(d)], "at": [at(d, s) for s in stations]}

    q("Bütün çizelgeler; eskiyen uyarılır", {}, {"superelevations": [report(d) for d in start], "at": []},
      [stale_warning(D2)])
    q("Bir çizelge adıyla", {"superelevation": "ana AKS deveri"}, one(D1))
    crit = [c["station"] for c in report(D1)["critical"]]
    q("Kritik km'lerde eğimler", {"superelevation": "d-1", "stations": crit}, one(D1, crit))
    mid = [0.0, 150.0, 200.0, 230.0, 260.0, 400.0, 600.0]
    q("Teğette, geçişte, kurpta ve sonra", {"superelevation": "d-1", "stations": mid}, one(D1, mid))
    q("Km satırlarından, eskiyen", {"superelevation": "d-2", "stations": [0.0, 100.0, 150.0, 300.0, 450.0, 600.0]},
      one(D2, [0.0, 100.0, 150.0, 300.0, 450.0, 600.0]), [stale_warning(D2)])
    wide = {**D1, "method": curves(rule("p1", 6.0, 40.0, 0.0, 0.5))}
    q("Genişleme iç yanda", {"superelevation": "d-1", "stations": [180.0, 200.0, 240.0]}, one(wide, [180.0, 200.0, 240.0]),
      supers=[wide, D2])
    lean = {**D1, "method": curves(rule("p1", 8.0, 30.0, 0.6))}
    q("Teğet payı 0,6", {"superelevation": "d-1", "stations": [170.0, 190.0, 210.0]}, one(lean, [170.0, 190.0, 210.0]),
      supers=[lean, D2])
    one_way = {**D1, "normalLeft": 2.0, "normalRight": -2.0, "method": curves(rule("p1", 4.0, 40.0, 0.5))}
    q("Tek yöne eğimli normal: dış yan zaten yükselir", {"superelevation": "d-1", "stations": [150.0, 180.0, 220.0, 260.0]},
      one(one_way, [150.0, 180.0, 220.0, 260.0]), supers=[one_way, D2])
    q("Satırların arasında doğrusal", {"superelevation": "Yan aks deveri", "stations": [120.0, 175.0, 250.0, 480.0]},
      one(D2, [120.0, 175.0, 250.0, 480.0]), [stale_warning(D2)])
    broken = {**D1, "method": curves(rule("p9", 6.0, 40.0, 0.0))}
    q("Kurulamayan çizelge raporda nedeniyle", {}, {"superelevations": [report(broken), report(D2)], "at": []},
      [stale_warning(D2)], supers=[broken, D2])
    q("Plan cevabı verir", {"superelevation": "d-1", "stations": [200.0]}, one(D1, [200.0]), op="plan")
    q("Doğrulama uyarıları verir", {"superelevation": "d-2"}, None, [stale_warning(D2)], op="validate")
    q("Çizelge yok: boş liste", {}, {"superelevations": [], "at": []}, supers=[])
    q("Yalnız rapor, km sorusuz", {"superelevation": "d-2"}, one(D2), [stale_warning(D2)])

    L = float(pcmd.ALIGNED["g-1"].length)
    no("Yol projesi değil", {}, NOT_ROAD, None, setup(work="map"))
    no("Olmayan çizelge", {"superelevation": "yok"},
       failed("unknown_superelevation", "“yok” kimlikli ya da adlı dever çizelgesi projede yok.", "superelevation"))
    no("Çizelgesiz km sorusu", {"stations": [100.0]},
       failed("invalid_places", "Eğim bir dever çizelgesinde sorulur: çizelgenin kimliğini ya da adını (superelevation) verin.",
              "superelevation"))
    no("Aksın dışındaki km", {"superelevation": "d-1", "stations": [10.0, 900.0]},
       failed("station_outside", f"900 m aksın üstünde değil (aks {L:.3f} m).", "stations[1]"))
    no("Eksi km", {"superelevation": "d-1", "stations": [-1.0]},
       failed("station_outside", f"-1 m aksın üstünde değil (aks {L:.3f} m).", "stations[0]"))
    no("Kurulamayan çizelgenin km sorusu", {"superelevation": "d-1", "stations": [100.0]},
       failed("invalid_superelevation", "“p9” PI'sinde kurp yok.", "method.curves[0].pi"), [broken, D2])
    return file("road.superelevations.query", "Deveri sorgula", setup(), cases)


def main():
    files = {"road.superelevations.edit.json": edit_cases(), "road.superelevations.query.json": query_cases()}
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
