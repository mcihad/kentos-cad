#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.volumes.progress.json: the shared cases of `road.volumes.progress` (plan/road.md
Y36.2, docs/adr/0942), their expectations built without KentOS code: the drawing, the corridors and the volumes are
scripts/fixtures/road_volume_command_cases.py's; the corridor's top surface scripts/fixtures/road_corridor_command_cases.py's;
the original ground's TIN the profile reference's; the surveys' TINs the surface reference's Delaunay over their points;
the work scripts/fixtures/road_volume_progress_cases.py's (exact fractions, checked there against closed forms and
Y8's pairwise volumes); the sources' digests scripts/fixtures/road_terrain_command_cases.py's.

Three surveys are added to the drawing, each the original ground's 120 points and points along the main corridor: a
September one where 40 % of the work is done, an October one at 80 % with a stretch dug and filled 15 % past the design,
and one of the corridor's western half only.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_progress_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_corridor_cases as cref  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_surface_cases as sref  # noqa: E402
import road_terrain_command_cases as tcmd  # noqa: E402
import road_volume_cases as vref  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402
import road_volume_progress_cases as pref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.volumes.progress.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
KEYS = ("cutDone", "cutLeft", "cutOver", "fillDone", "fillLeft", "fillOver")
BACKWARD = -1e-6
NOT_ROAD = failed("not_a_road_project", "Kübaj yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
ORIGINAL = [((x, y), z) for (x, y), z in zip(pcmd.TIN.xy, pcmd.TIN.z)]


def tin_tris(points):
    """The Delaunay triangles over (x, y), z points, as corner triples."""
    xy = [p for p, _ in points]
    tris = sref.triangulate(xy, [])
    return [[(points[i][0][0], points[i][0][1], points[i][1]) for i in t] for t in tris]


ORIGINAL_TRIS = tin_tris(ORIGINAL)
_, _, MAIN_TRIS, MAIN_REGS = ccmd.make(next(c for c in vcmd.CORRIDORS if c["id"] == "c-1"))


def along():
    """Points along the main corridor: every fourth corner of its top surface, apart from each other and the ground's."""
    seen = {(round(x, 3), round(y, 3)) for (x, y), _ in ORIGINAL}
    out = []
    corners = []
    for t in MAIN_TRIS:
        for p in t:
            corners.append((float(p[0]), float(p[1]), float(p[2])))
    corners = sorted(set(corners))
    for x, y, z in corners[::4]:
        k = (round(x, 3), round(y, 3))
        if pcmd.TIN.height(*k) is None:
            continue
        if k in seen or any((k[0] - a) ** 2 + (k[1] - b) ** 2 < 0.25 for a, b in seen):
            continue
        seen.add(k)
        out.append((k, z))
    return out


ALONG = along()


def survey(done, past=None, west=None):
    """A survey: the original ground everywhere, and along the corridor `done` of the way from the ground to the design
    (`past`: (from x, to x, share) a stretch taken further); `west`: only the points west of this x."""
    out = []
    for (x, y), z in ORIGINAL:
        if west is None or x < west:
            out.append(((x, y), z))
    for (x, y), zd in ALONG:
        if west is not None and x >= west:
            continue
        zo = pcmd.TIN.height(x, y)  # inside the ground: along() keeps no other
        t = past[2] if past and past[0] <= x < past[1] else done
        out.append(((x, y), round(float(zo) + t * (zd - float(zo)), 3)))
    return out


SURVEYS = {
    "t-3": ("Eylül ölçüsü", "olcu1", survey(0.4)),
    "t-4": ("Ekim ölçüsü", "olcu2", survey(0.8, past=(150.0, 330.0, 1.15))),
    "t-5": ("Batı yarısı", "olcu3", survey(0.8, west=300.0)),
    "t-6": ("Eylül kopyası", "olcu4", survey(0.4)),
}
SURVEY_TRIS = {k: tin_tris(pts) for k, (_, _, pts) in SURVEYS.items()}


def setup(volumes=vcmd.START, corridors=vcmd.CORRIDORS, work="road"):
    base = vcmd.setup(volumes, corridors, work=work)
    base["name"] = "Yol dönem metrajı"
    next_id = max(e["id"] for e in base["entities"]) + 1
    for sid, (name, layer, pts) in SURVEYS.items():
        base["layers"].append(pcmd.node(layer, name))
        for (x, y), z in pts:
            base["entities"].append({"kind": "point", "id": next_id, "layerId": layer, "attrs": {}, "p": {"x": x, "y": y},
                                     "z": z})
            base["uids"].append(pcmd.U(next_id))
            next_id += 1
        if work == "road":
            base["settings"]["road"]["surfaces"].append({"id": sid, "name": name, "points": {"layers": [layer]}})
    return base


def only(s, keep):
    """`s` with only the surfaces `keep` and the points of their layers, the objects numbered in their order again:
    docs/adr/1029's cases compare those alone, and the others' points would only make the file larger."""
    road = s["settings"]["road"]
    layers = lambda ts: {l for t in ts for l in t.get("points", {}).get("layers", [])}  # noqa: E731
    drop = layers(t for t in road["surfaces"] if t["id"] not in keep) - layers(t for t in road["surfaces"] if t["id"] in keep)
    pairs = [(e, u) for e, u in zip(s["entities"], s["uids"]) if e["layerId"] not in drop]
    s["entities"], s["uids"] = [dict(e, id=k + 1) for k, (e, _) in enumerate(pairs)], [u for _, u in pairs]
    road["surfaces"] = [t for t in road["surfaces"] if t["id"] in keep]
    return s


def road_of(volumes=vcmd.START, corridors=vcmd.CORRIDORS):
    return setup(volumes, corridors)["settings"]["road"]


def digest(doc, sid):
    s = next(x for x in doc["settings"]["road"]["surfaces"] if x["id"] == sid)
    layers = s["points"]["layers"]
    points = [(e["p"]["x"], e["p"]["y"], e["z"], uid) for e, uid in zip(doc["entities"], doc["uids"])
              if e["kind"] == "point" and e["layerId"] in layers and e.get("z") is not None]
    return tcmd.digest({"points": points, "breaklines": [], "voids": [], "boundary": None, "tolerance": None,
                        "maxEdge": None})


def amounts(p):
    return {k: float(p[k]) for k in KEYS}


def decimal(v):
    return f"{v:.1f}".replace(".", ",")


def surface_of(doc, name, path):
    surfaces = doc["settings"]["road"]["surfaces"]
    s = next((x for x in surfaces if x["id"] == name), None) or \
        next((x for x in surfaces if x["name"].lower() == name.lower()), None)
    if s is None:
        raise vcmd.Refused("unknown_terrain", f"“{name}” kimlikli ya da adlı yüzey projede yok.", path)
    return s


def answer(doc, v, q, corridors=vcmd.CORRIDORS):
    c = next(x for x in corridors if x["id"] == v["corridor"])
    now_s = surface_of(doc, q["survey"], "survey")
    before_s = surface_of(doc, q["previous"], "previous") if q.get("previous") is not None else None
    if now_s["id"] == c["surface"]:
        raise vcmd.Refused("invalid_volume_progress",
                           f"“{now_s['name']}” yüzeyi “{c['name']}” koridorunun ilk arazisi; ara ölçü başka bir yüzey olmalı.",
                           "survey")
    if before_s is not None and before_s["id"] == now_s["id"]:
        raise vcmd.Refused("invalid_volume_progress",
                           f"Önceki ölçü ara ölçünün kendisi (“{now_s['name']}”); daha önceki bir yüzey seçin.", "previous")
    if c["surface"] != "t-1":
        raise vcmd.Refused("invalid_volume", f"“{c['name']}” koridorunun yüzeyi kurulamıyor; ilk arazi okunamaz.", "corridor")

    def built(s, path):
        if s["id"] == "t-1":
            return ORIGINAL_TRIS
        if s["id"] not in SURVEY_TRIS:
            raise vcmd.Refused("invalid_volume_progress", f"“{s['name']}” yüzeyi kaynaklarından kurulamıyor.", path)
        return SURVEY_TRIS[s["id"]]

    now_tris = built(now_s, "survey")
    before_tris = built(before_s, "previous") if before_s is not None else None
    _, _, tris, regs = ccmd.make(c)
    base = vref.between(tris, ORIGINAL_TRIS)["uncovered"]
    now = pref.progress(tris, ORIGINAL_TRIS, now_tris)
    before = pref.progress(tris, ORIGINAL_TRIS, before_tris) if before_tris is not None else None
    warns = []
    whole = float(now["covered"] + now["uncovered"])
    tol = 1e-6 * max(whole, 1.0)
    for s, p, path in ((now_s, now, "survey"), (before_s, before, "previous")):
        if s is not None and p is not None and float(p["uncovered"]) > base + tol:
            warns.append({"code": "volume_progress_uncovered",
                          "message": f"“{s['name']}” yüzeyi tasarımın {decimal(float(p['uncovered']) - base)} m²'sini "
                                     "örtmüyor; orada yapılan iş sayılmadı.", "path": path})
    na = amounts(now)
    ba = amounts(before) if before is not None else None
    period = {k: na[k] - ba[k] for k in KEYS} if ba is not None else None
    if period is not None:
        back = [f"{w} {decimal(-period[k])} m³" for w, k in (("kazı", "cutDone"), ("dolgu", "fillDone"))
                if period[k] < BACKWARD]
        if back:
            warns.append({"code": "volume_progress_backward",
                          "message": f"“{v['name']}” kübajında önceki ölçüden bu yana yapılan iş geri gitmiş "
                                     f"({', '.join(back)}): ölçü hatası ya da geri doldurma.", "path": "previous"})
    stretches = []
    for reg in regs:
        t = reg["template"]
        for x, y in zip(reg["crosses"], reg["crosses"][1:]):
            if x["section"] is None or y["section"] is None:
                continue
            got = cref.strip(cref.top(x, t), cref.top(y, t))
            if any(cref.area(g) <= 0 for g in got):
                continue
            p = pref.progress(got, ORIGINAL_TRIS, now_tris)
            stretches.append({"from": x["station"], "to": y["station"],
                              "fromText": ccmd.km_text(c["alignment"], x["station"]),
                              "toText": ccmd.km_text(c["alignment"], y["station"]),
                              "amounts": amounts(p), "uncovered": float(p["uncovered"])})

    def share(done, left):
        return 100 * done / (done + left) if done + left > 1e-9 else None

    out = {"volume": v["id"], "original": c["surface"], "originalDigest": digest(doc, c["surface"]),
           "survey": now_s["id"], "surveyDigest": digest(doc, now_s["id"])}
    if before_s is not None:
        out.update({"previous": before_s["id"], "previousDigest": digest(doc, before_s["id"])})
    out["now"] = na
    if ba is not None:
        out.update({"before": ba, "period": period})
    for key, d, l in (("cutShare", "cutDone", "cutLeft"), ("fillShare", "fillDone", "fillLeft")):
        s = share(na[d], na[l])
        if s is not None:
            out[key] = s
    out.update({"covered": float(now["covered"]), "uncovered": float(now["uncovered"]), "stretches": stretches})
    return out, warns


def cases():
    out = []
    start = vcmd.START
    doc0 = setup()

    def q(name, input_, op="execute", corridors=vcmd.CORRIDORS):
        default = corridors is vcmd.CORRIDORS
        doc = doc0 if default else setup(start, corridors)
        v = next(x for x in start if x["id"] == input_["volume"] or x["name"].lower() == input_["volume"].lower())
        result, warns = answer(doc, v, input_, corridors)
        res = completed(None, warns) if op == "validate" else completed(result, warns)
        out.append(case(name, [step(op, input_, res, {**UNCHANGED, "road": doc["settings"]["road"]})],
                        None if default else doc))

    def no(name, input_, result, doc=None):
        d = doc if doc is not None else doc0
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": d["settings"].get("road")})],
                        None if doc is None else doc))

    def refused(name, input_, doc=None, corridors=vcmd.CORRIDORS):
        d = doc if doc is not None else doc0
        v = next(x for x in start if x["id"] == input_["volume"])
        try:
            answer(d, v, input_, corridors)
        except vcmd.Refused as r:
            no(name, input_, r.result, doc)
            return
        raise AssertionError(f"{name}: not refused")

    q("Eylül ölçüsü: işin %40'ı", {"volume": "v-1", "survey": "t-3"})
    q("Adlarıyla: Ekim ölçüsü, Eylül'den bu yana", {"volume": "ANA kübaj", "survey": "ekim ölçüsü", "previous": "Eylül ölçüsü"})
    q("Ekim ölçüsü: bir aralıkta fazla kazı ve dolgu", {"volume": "v-1", "survey": "t-4"})
    q("Önceki ölçü ilk arazi: dönem = birikmiş", {"volume": "v-1", "survey": "t-3", "previous": "t-1"})
    q("Geri gitmiş: önceki ölçü daha ileride", {"volume": "v-1", "survey": "t-3", "previous": "t-4"})
    q("Aynı noktalar iki ad: dönemde iş yok", {"volume": "v-1", "survey": "t-6", "previous": "t-3"})
    q("Ölçü yalnız batı yarısında", {"volume": "v-1", "survey": "t-5"})
    q("Önceki ölçü yalnız batı yarısında", {"volume": "v-1", "survey": "t-4", "previous": "t-5"})
    q("Yan koridorun kübajı: ölçü ona dokunmuyor", {"volume": "v-2", "survey": "t-4"})
    q("Plan cevabı verir", {"volume": "v-1", "survey": "t-3", "previous": "t-4"}, op="plan")
    q("Doğrulama uyarıları verir", {"volume": "v-1", "survey": "t-3", "previous": "t-4"}, op="validate")
    # Docs/adr/1029: the September survey made of a cloud's ground on its own layer, its record's vertical system as
    # given; the design and the original ground the drawing's, the project's heights as given.
    def heights_doc(vertical=None, heights=None):
        d = copy.deepcopy(doc0)
        record = {"layer": SURVEYS["t-3"][1], "clouds": [pcmd.U(1)], "classes": [2], "returns": "all", "cell": 1.0, "read": 9,
                  "kept": 9, "coverage": 1.0, "density": 1.0}
        if vertical is not None:
            record["vertical"] = vertical
        next(t for t in d["settings"]["road"]["surfaces"] if t["id"] == "t-3")["clouds"] = [record]
        if heights is not None:
            d["settings"]["heights"] = heights
        return only(d, {"t-1", "t-3"})

    drawing = "çizimin kendi kotları (“Ana koridor”, “Mevcut zemin”; projenin kotlarının düşey sistemi söylenmemiş)"
    cloud = (f"“Eylül ölçüsü” yüzeyinin “{SURVEYS['t-3'][1]}” katmanındaki bulut noktaları (dosya düşey sistemini "
             "söylemiyor ya da alındığında kaydedilmedi)")
    unshown = ("Karşılaştırılan kotların aynı düşey sistemde olduğu gösterilemiyor: {}. Sıfır ayrım ya da geoit tahmini "
               "yapılmaz. Projenin kotlarını Proje ayarlarında söyleyin, ya da kotları aynı saymak için assumeHeights "
               "verin (sonuç taslak olur).")
    sept = {"volume": "v-1", "survey": "t-3"}
    unknown_doc = heights_doc()
    no("Ölçünün düşey sistemi bilinmiyor: ret", sept,
       failed("unproven_heights", unshown.format(f"{drawing}; {cloud}"), "assumeHeights"), unknown_doc)
    v1 = next(x for x in start if x["id"] == "v-1")
    result, warns = answer(unknown_doc, v1, sept)
    assumed = {"code": "heights_assumed", "message": f"Kotlar aynı düşey sistemde sayıldı (assumeHeights): {drawing}; "
               f"{cloud}. Sonuç bu varsayıma dayanan bir taslaktır.", "path": "assumeHeights"}
    out.append(case("Varsayımla taslak dönem metrajı", [step("execute", {**sept, "assumeHeights": True},
                    completed(result, [assumed] + warns), {**UNCHANGED, "road": unknown_doc["settings"]["road"]})],
                    unknown_doc))
    same = heights_doc("EPSG:5775", {"kind": "orthometric", "verticalSrid": 5775})
    result, warns = answer(same, v1, sept)
    out.append(case("Ölçü ile proje aynı düşey sistemde", [step("execute", sept, completed(result, warns),
                    {**UNCHANGED, "road": same["settings"]["road"]})], same))
    other = heights_doc("EGM2008 height", {"kind": "orthometric", "verticalSrid": 5775})
    no("Ölçü başka düşey sistemde: varsayımla da ret", {**sept, "assumeHeights": True},
       failed("unproven_heights", "Karşılaştırılan kotlar iki ayrı düşey sistemde: çizimin kendi kotları (“Ana "
              "koridor”, “Mevcut zemin”) Antalya height (EPSG:5775), “Eylül ölçüsü” yüzeyinin "
              f"“{SURVEYS['t-3'][1]}” katmanındaki bulut noktaları EGM2008 height (EPSG:3855). Aradaki ayrım sıfır "
              "sayılmaz; kotları önce aynı sisteme çevirin (Koordinat dönüştür, geoit modeliyle).", "survey"), other)
    broken = [dict(c, surface="t-2") if c["id"] == "c-1" else c for c in vcmd.CORRIDORS]
    refused("Koridorun yüzeyi kurulamıyor", {"volume": "v-1", "survey": "t-3"}, setup(start, broken), broken)
    no("Yol projesi değil", {"volume": "v-1", "survey": "t-3"}, NOT_ROAD, setup(work="map"))
    no("Olmayan kübaj", {"volume": "yok", "survey": "t-3"},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volume"))
    refused("Olmayan ölçü", {"volume": "v-1", "survey": "Kasım ölçüsü"})
    refused("Olmayan önceki ölçü", {"volume": "v-1", "survey": "t-3", "previous": "t-9"})
    refused("Ölçü ilk arazinin kendisi", {"volume": "v-1", "survey": "t-1"})
    refused("Önceki ölçü ara ölçünün kendisi", {"volume": "v-1", "survey": "t-3", "previous": "Eylül ölçüsü"})
    refused("Ölçü kurulamıyor", {"volume": "v-1", "survey": "t-2"})
    refused("Önceki ölçü kurulamıyor", {"volume": "v-1", "survey": "t-3", "previous": "Kurulamaz"})
    return out, doc0


def main():
    got, doc0 = cases()
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.progress", "commandVersion": 1,
            "title": "Dönem metrajı",
            "note": "scripts/fixtures/road_volume_progress_command_cases.py yazar; çizim, koridorlar ve kübajlar "
                    "road_volume_command_cases.py'nin, ölçüler bu betiğin (ilk arazinin noktaları ve koridor boyunca "
                    "işin %40'ı, %80'i), iş road_volume_progress_cases.py'nin (kesirli üç yüzey bindirmesi); KentOS kodu yok.",
            "setup": doc0, "cases": got}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text("utf-8") != text:
            sys.exit(f"güncel değil: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(got)} durum, {len(ALONG)} koridor noktası)")
    for c in got:
        r = c["steps"][0]["result"]
        o = r.get("output") or {}
        n = o.get("now")
        w = [x["code"] for x in r.get("warnings", [])]
        print(f"  {c['name'][:44]:44} " + (" ".join(f"{k} {n[k]:.0f}" for k in KEYS) if n else r.get("status", "")) + f" {w}")


if __name__ == "__main__":
    main()
