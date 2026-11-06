#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.volumes.compare.json: the shared cases of `road.volumes.compare` (plan/road.md
Y37.2, docs/adr/0943), their expectations built without KentOS code: the drawing, the corridors and the volumes are
scripts/fixtures/road_volume_command_cases.py's, with two alternatives of the main road added: “Geniş kurp” (its
curve's radius 150 → 220, on a new alignment and profile) and “Yüksek kot” (the same alignment, its
crest PVI 1.5 m higher); the volumes by that reference, the departures by scripts/fixtures/road_alternatives_cases.py
(dense sampling and exact refinement), the surfaces' digests by the terrain commands' reference.

    uv run --no-project --python 3.11 --with triangle --with numpy --with mpmath --with pyproj \\
        python3 scripts/fixtures/road_volume_compare_command_cases.py
    ... --check   # compares the file with the one on disk
"""

import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alternatives_cases as alt  # noqa: E402
import road_corridor_command_cases as ccmd  # noqa: E402
import road_grades  # noqa: E402
import road_profile_command_cases as pcmd  # noqa: E402
import road_section_command_cases as scmd  # noqa: E402
import road_terrain_command_cases as tcmd  # noqa: E402
import road_volume_command_cases as vcmd  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/road.volumes.compare.json"
completed, failed, step, case = pcmd.completed, pcmd.failed, pcmd.step, pcmd.case
UNCHANGED = pcmd.UNCHANGED
NOT_ROAD = failed("not_a_road_project", "Kübaj yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
TOLERANCE, HEIGHT = 0.01, 0.01

# The alternatives: a wider curve on its own alignment and profile, and a higher crest on the main alignment.
G5 = {"id": "g-5", "name": "Geniş kurp aksı", "revision": 1,
      "pis": [pcmd.acmd.pi("a", 20, 50), pcmd.acmd.pi("p1", 300, 60, 220, 40, 40), pcmd.acmd.pi("b", 560, 360)],
      "stationing": {"start": 1000.0}}
pcmd.ALIGNED["g-5"] = pcmd.acmd.build(pcmd.ALIGNMENTS + [G5], G5)
pcmd.REV["g-5"] = 1
B5 = {"id": "b-5", "name": "Geniş kurbun kotu", "alignment": "g-5", "alignmentRevision": 1, "revision": 1,
      "pvis": pcmd.MAIN_PVIS[:-1] + [pcmd.pv("b", 665, 113)], "surface": "t-1"}
B6 = {"id": "b-6", "name": "Yüksek kot", "alignment": "g-1", "alignmentRevision": 1, "revision": 2,
      "pvis": [dict(p, elevation=112.5) if p["id"] == "v2" else p for p in pcmd.MAIN_PVIS], "surface": "t-1"}
for b in (B5, B6):
    scmd.BY_PROFILE[b["id"]] = b
C5 = {**ccmd.C1, "id": "c-5", "name": "Geniş kurp koridoru", "alignment": "g-5", "profile": "b-5"}
C6 = {**ccmd.C1, "id": "c-6", "name": "Yüksek kot koridoru", "profile": "b-6"}
V5 = {**vcmd.V1, "id": "v-5", "name": "Geniş kurp", "corridor": "c-5"}
V6 = {**vcmd.V1, "id": "v-6", "name": "Yüksek kot", "corridor": "c-6"}
CORRIDORS = vcmd.CORRIDORS + [C5, C6]
VOLUMES = vcmd.START + [V5, V6]


def setup(volumes=VOLUMES, corridors=CORRIDORS, work="road"):
    base = vcmd.setup(volumes, corridors, work=work)
    base["name"] = "Yol alternatifleri"
    if work == "road":
        road = base["settings"]["road"]
        road["alignments"].append(copy.deepcopy(G5))
        road["profiles"] += copy.deepcopy([B5, B6])
    return base


def digest(doc, sid):
    s = next((x for x in doc["settings"]["road"]["surfaces"] if x["id"] == sid), None)
    if s is None:
        return ""
    layers = s["points"]["layers"]
    points = [(e["p"]["x"], e["p"]["y"], e["z"], uid) for e, uid in zip(doc["entities"], doc["uids"])
              if e["kind"] == "point" and e["layerId"] in layers and e.get("z") is not None]
    return tcmd.digest({"points": points, "breaklines": [], "voids": [], "boundary": None, "tolerance": None,
                        "maxEdge": None})


def zref_pvis(pvis):
    out = []
    for p in pvis:
        curve = None
        if "length" in p:
            curve = alt.par(p["length"])
        elif "radius" in p:
            curve = alt.zref.circ(p["radius"])
        elif "lengthIn" in p:
            curve = alt.zref.asym(p["lengthIn"], p["lengthOut"])
        out.append(alt.pvi(p["id"], p["station"], p["elevation"], curve))
    return out


ALIGNMENT_DEFS = {a["id"]: a for a in pcmd.ALIGNMENTS + [G5]}
ROADS = {}


def road_of(c):
    key = (c["alignment"], c["profile"])
    if key not in ROADS:
        prof = scmd.BY_PROFILE[c["profile"]]
        pis = [alt.pi(q["id"], q["x"], q["y"], q.get("radius", 0), q.get("spiralIn", 0), q.get("spiralOut", 0))
               for q in ALIGNMENT_DEFS[c["alignment"]]["pis"]]
        ROADS[key] = alt.Road(pis, zref_pvis(prof["pvis"]))
    return ROADS[key]


def report(doc, v, corridors):
    c = next((x for x in corridors if x["id"] == v["corridor"]), None)
    profile = scmd.BY_PROFILE.get(c["profile"]) if c else None
    out = {"volume": v["id"], "name": v["name"], "corridor": v["corridor"],
           "alignment": c["alignment"] if c else "", "alignmentRevision": pcmd.REV.get(c["alignment"], 0) if c else 0,
           "profile": c["profile"] if c else "", "profileRevision": profile["revision"] if profile else 0,
           "surface": c["surface"] if c else "", "surfaceDigest": digest(doc, c["surface"]) if c else ""}
    # The corridor builds without its surface too (no sections then): the alignment and profile are there.
    built = c is not None
    if built:
        out["length"] = float(pcmd.ALIGNED[c["alignment"]].length)
    warn = None
    try:
        rep, _ = vcmd.compute(v, corridors)
        out["figures"] = {"cut": rep["cut"], "fill": rep["fill"], "factoredCut": rep["factoredCut"],
                          "factoredFill": rep["factoredFill"]}
        if "approval" in rep:
            a = rep["approval"]
            out["approved"] = {"at": a["approved"]["at"], "changed": a["changed"], "cutChange": a["cutChange"],
                               "fillChange": a["fillChange"]}
    except vcmd.Refused as r:
        why = r.result["error"]["message"]
        out["problem"] = why
        warn = {"code": "volume_compare_broken",
                "message": f"“{v['name']}” kübajı hesaplanamıyor; karşılaştırmada hacmi yok: {why}", "path": "volumes"}
    return out, warn, (c if built else None)


def rows(deps, other_c, base_c):
    def text(c, s):
        return ccmd.km_text(c["alignment"], s)
    return [{"from": d["from"], "to": d["to"], "fromText": text(other_c, d["from"]), "toText": text(other_c, d["to"]),
             **({"baseFromText": text(base_c, d["baseFrom"])} if d["baseFrom"] is not None else {}),
             **({"baseToText": text(base_c, d["baseTo"])} if d["baseTo"] is not None else {}),
             "largest": d["largest"]} for d in deps]


def answer(doc, q, volumes, corridors):
    tol, h = q.get("tolerance", TOLERANCE), q.get("height", HEIGHT)
    defs = []
    for i, name in enumerate(q["volumes"]):
        v = next((x for x in volumes if x["id"] == name), None) or \
            next((x for x in volumes if x["name"].lower() == name.lower()), None)
        defs.append(v)
    built = [report(doc, v, corridors) for v in defs]
    warns = [w for _, w, _ in built if w]
    base, _, base_c = built[0]
    out = [base]
    for rep, _, c in built[1:]:
        rep = dict(rep)
        against = {}
        if "length" in rep and "length" in base:
            against["lengthChange"] = rep["length"] - base["length"]
        if "figures" in rep and "figures" in base:
            against["figuresChange"] = {k: rep["figures"][k] - base["figures"][k] for k in rep["figures"]}
        if c is not None and base_c is not None:
            o, b = road_of(c), road_of(base_c)
            against["plan"] = rows(alt.plan(b, o, tol), c, base_c)
            against["height"] = rows(alt.height(b, o, tol, h), c, base_c)
        else:
            against["plan"], against["height"] = [], []
        rep["against"] = against
        out.append(rep)
    return {"tolerance": tol, "height": h, "alternatives": out}, warns


def cases():
    out = []
    doc0 = setup()

    def q(name, input_, volumes=VOLUMES, corridors=CORRIDORS, op="execute"):
        default = volumes is VOLUMES and corridors is CORRIDORS
        doc = doc0 if default else setup(volumes, corridors)
        result, warns = answer(doc, input_, volumes, corridors)
        res = completed(None, warns) if op == "validate" else completed(result, warns)
        out.append(case(name, [step(op, input_, res, {**UNCHANGED, "road": doc["settings"]["road"]})],
                        None if default else doc))

    def no(name, input_, result, doc=None):
        d = doc if doc is not None else doc0
        out.append(case(name, [step("execute", input_, result, {**UNCHANGED, "road": d["settings"].get("road")})],
                        doc))

    bad = lambda path, words: failed("invalid_volume_compare", words, path)  # noqa: E731
    q("Geniş kurp: aks kurpta ayrılıyor", {"volumes": ["v-1", "v-5"]})
    q("Yüksek kot: aks aynı, kot tepede ayrılıyor", {"volumes": ["Ana kübaj", "yüksek kot"]})
    q("Üç alternatif, toleranslarla", {"volumes": ["v-1", "v-5", "v-6"], "tolerance": 0.5, "height": 0.25})
    q("Taban alternatif olunca işaretler döner", {"volumes": ["v-5", "v-1"]})
    q("Bambaşka koridor: yan kübaj", {"volumes": ["v-1", "v-2"]})
    q("Onaylı taban", {"volumes": ["v-1", "v-6"]}, [vcmd.approved(vcmd.V1), vcmd.V2, V5, V6])
    q("Onaylı alternatif", {"volumes": ["v-1", "v-6"]}, [vcmd.V1, vcmd.V2, V5, vcmd.approved(V6, corridors=CORRIDORS)])
    q("Geniş tolerans kurbu da yutar", {"volumes": ["v-1", "v-5"], "tolerance": 10.0, "height": 10.0})
    q("Kot toleransı 2 m: yüksek kot ayrılmaz", {"volumes": ["v-1", "v-6"], "height": 2.0})
    q("Dört kübaj birden", {"volumes": ["v-1", "v-2", "v-5", "v-6"]})
    broken = [dict(x, surface="t-2") if x["id"] == "c-2" else x for x in CORRIDORS]
    q("Hesaplanamayan alternatif", {"volumes": ["v-1", "v-2"]}, VOLUMES, broken)
    q("Plan cevabı verir", {"volumes": ["v-1", "v-6"]}, op="plan")
    q("Doğrulama uyarıları verir", {"volumes": ["v-1", "v-2"]}, VOLUMES, broken, op="validate")
    no("Yol projesi değil", {"volumes": ["v-1", "v-5"]}, NOT_ROAD, setup(work="map"))
    no("Olmayan kübaj", {"volumes": ["v-1", "yok"]},
       failed("unknown_volume", "“yok” kimlikli ya da adlı kübaj hesabı projede yok.", "volumes[1]"))
    no("Tek kübaj", {"volumes": ["v-1"]}, bad("volumes", "İkiden altıya kadar kübaj karşılaştırılır; ilki taban."))
    no("Yedi kübaj", {"volumes": ["v-1", "v-2", "v-5", "v-6", "a", "b", "c"]},
       bad("volumes", "İkiden altıya kadar kübaj karşılaştırılır; ilki taban."))
    no("Aynı kübaj iki kez", {"volumes": ["v-1", "Ana kübaj"]}, bad("volumes[1]", "“Ana kübaj” kübajı iki kez seçilmiş."))
    no("Sıfır tolerans", {"volumes": ["v-1", "v-5"], "tolerance": 0.0},
       bad("tolerance", "Plan toleransı 0'dan büyük, en çok 10 m olmalı."))
    no("Çok büyük kot toleransı", {"volumes": ["v-1", "v-5"], "height": 11.0},
       bad("height", "Kot toleransı 0'dan büyük, en çok 10 m olmalı."))
    return out, doc0


def main():
    got, doc0 = cases()
    data = {"format": "kentos.command-cases", "version": 1, "command": "road.volumes.compare", "commandVersion": 1,
            "title": "Alternatifleri karşılaştır",
            "note": "scripts/fixtures/road_volume_compare_command_cases.py yazar; çizim, koridorlar ve kübajlar "
                    "road_volume_command_cases.py'nin, iki alternatifle (Geniş kurp, Yüksek kot); ayrılmalar "
                    "road_alternatives_cases.py'nin (sık örnekleme ve kesin inceltme); KentOS kodu yok.",
            "setup": doc0, "cases": got}
    data = road_grades.fixture(data)
    text = json.dumps(data, ensure_ascii=False, indent=2) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text("utf-8") != text:
            sys.exit(f"güncel değil: {OUT.relative_to(ROOT)}")
        return
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(got)} durum)")
    for c in got:
        r = c["steps"][0]["result"]
        o = r.get("output") or {}
        line = []
        for a in o.get("alternatives", [])[1:]:
            ag = a["against"]
            line.append(f"{a['name']}: plan {[(round(d['from'], 2), round(d['to'], 2)) for d in ag['plan']]} "
                        f"kot {[(round(d['from'], 2), round(d['to'], 2)) for d in ag['height']]}")
        print(f"  {c['name'][:40]:40} {r['status']} {'; '.join(line)} {[w['code'] for w in r.get('warnings', [])]}")


if __name__ == "__main__":
    main()
