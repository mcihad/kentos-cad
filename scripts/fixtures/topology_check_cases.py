#!/usr/bin/env python3
"""The shared cases of `cad.topology.check` and `cad.topology.exceptions` (docs/adr/0589, plan/gis.md CBS-14), written
without KentOS code.

    /usr/bin/python3 scripts/fixtures/topology_check_cases.py --files   # GDAL writes fixtures/geopackage/v1/topoloji-parca*.gpkg
    python3 scripts/fixtures/topology_check_cases.py                    # writes the two case files
    python3 scripts/fixtures/topology_check_cases.py --check            # compares

What a rule finds (the problem, the objects, the place, the measure, the fixes) and what a fix makes come from the
topology rules' reference (topology_rules_cases.py, ADR 0202: cells of the coordinates, exact fractions). Here, from
ADR 0589:

- a finding's lasting key (ADR 0524 §1): FNV-1a 64 over the rule's id, each object's uid and the place in steps of the
  tolerance (JavaScript's rounding, each as eight little-endian bytes), every piece closed by 0xff;
- a bound layer's part takes the rows of its band (the area grown by 15 % of its shorter side) that the layer holds
  no object for as neighbours, after the drawing's objects; a finding of rows outside alone is left out; a part's
  object reaching out of the band, or a band that cannot be read, makes the check incomplete;
- an exception bound to its objects' geometry lapses once that geometry changes; one without a digest never does.

Two scenes. The drawing's: five parcels with fixed uids (A1 and A2 overlap in 2 × 6 m, A3 and A4 in 1 × 10 m, A5 has a
0.5 m edge), rules r1 (Çakışmamalı) and r2 (Kısa kenar, 1 m). The part's: GDAL's OGR writes G1…G4; the area holds G1,
G2 overlaps it from the band, G3 and G4 overlap each other in the band alone.
"""
import hashlib
import json
import math
import struct
import sys
import uuid
from fractions import Fraction as F
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import gpkg_import_cases as gi  # noqa: E402
import topology_rules_cases as tr  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/geopackage/v1"
PART = "topoloji-parca.gpkg"
PART_2 = "topoloji-parca-2.gpkg"
OUT_CHECK = ROOT / "fixtures/commands/v1/cad.topology.check.json"
OUT_EXC = ROOT / "fixtures/commands/v1/cad.topology.exceptions.json"
ANY = "$any"
SRID = 5256
E, N = tr.E, tr.N
T = F(1, 1000)
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

# ── keys ───────────────────────────────────────────────────────────────


def js_round(x):
    f = math.floor(x)
    return f + 1.0 if x - f >= 0.5 else f


def key(rule, uids, at, t=T):
    step = float(t) if t > 0 else 1e-6
    h = 0xCBF29CE484222325
    mask = (1 << 64) - 1

    def eat(bs):
        nonlocal h
        for b in bs:
            h ^= b
            h = (h * 0x100000001B3) & mask
        h ^= 0xFF
        h = (h * 0x100000001B3) & mask

    eat(rule.encode())
    for u in uids:
        eat(u.encode())
    eat(struct.pack("<q", int(js_round(at["x"] / step))))
    eat(struct.pack("<q", int(js_round(at["y"] / step))))
    return f"{h:016x}"


# ── the drawing's scene ────────────────────────────────────────────────


def uid(n):
    return f"0192f5a9-1414-7000-8000-{n:012x}"


def drawing_objs(stretched=False):
    a5 = [tr.P(50, 0), tr.P(60, 0), tr.P(60, 10), tr.P(F(101, 2), 10), tr.P(50, 10)]
    objs = [
        tr.area_obj("parsel", tr.rect(0, 0, 10, 10)),
        tr.area_obj("parsel", tr.rect(8, 2, 19 if stretched else 18, 8)),
        tr.area_obj("parsel", tr.rect(20, 0, 30, 10)),
        tr.area_obj("parsel", tr.rect(29, 0, 40, 10)),
        tr.area_obj("parsel", a5),
    ]
    for k, o in enumerate(objs):
        o.uid = uid(k + 1)
    return objs


R1 = tr.rule("r1", "mustNotOverlap", "parsel")
R2 = tr.rule("r2", "mustNotHaveShortEdges", "parsel", value=1)
RULES = [R1, R2]


def rules_json(rules):
    return [dict(r, value=float(r["value"])) if "value" in r else r for r in rules]


def setup(rules=RULES, exceptions=(), layers=("parsel",)):
    objs = drawing_objs()
    topology = {"tolerance": float(T), "rules": rules_json(rules)}
    if exceptions:
        topology["exceptions"] = list(exceptions)
    return {
        "format": "kentos.document", "version": 2, "name": "Topoloji denetimi",
        "settings": {"srid": SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000.0, "topology": topology},
        "origin": {"x": float(E), "y": float(N)},
        "layers": [{"id": l, "name": l.capitalize(), "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": STYLE, "children": []} for l in layers],
        "activeLayer": layers[0],
        "entities": [dict(o.shape, id=k + 1, layerId=o.layer, attrs={}) for k, o in enumerate(objs)],
        "uids": [o.uid for o in objs],
        "styles": {"items": [], "categories": []},
    }


def found(objs, rules, exceptions=(), lapsed=(), outside=()):
    """The findings as the command says them: the reference's, with key, uids, kind, exception and lapse. `lapsed`
    are exceptions (rule, uids, at) whose digest no longer holds: they do not make a finding an exception."""
    holding = [x for x in exceptions if x not in lapsed]
    out = []
    # The reference's grid wants a pair: a rule over fewer than two areas finds nothing.
    rules = [r for r in rules if r["kind"] != "mustNotOverlap" or len(tr.of_layer(objs, r["layer"], tr.AREA)) > 1]
    for f in tr.run(objs, rules, T, holding):
        uids = [objs[k].uid for k in f["objects"]]
        rule = next(r for r in rules if r["id"] == f["rule"])
        if all(objs[k].uid in outside for k in f["objects"]):
            continue
        g = {"key": key(f["rule"], uids, f["at"]), "rule": f["rule"], "kind": rule["kind"], "problem": f["problem"],
             "uids": uids}
        out_uids = [u for u in uids if u in outside]
        if out_uids:
            g["outside"] = out_uids
        g["at"] = f["at"]
        if f["measure"] is not None:
            g["measure"] = f["measure"]
        if f["measureKind"] is not None:
            g["measureKind"] = f["measureKind"]
        g["fixes"] = f["fixes"]
        g["exception"] = f["exception"]
        near = lambda x: x["rule"] == f["rule"] and x["objects"] == uids and (  # noqa: E731
            (F(x["at"]["x"]) - F(f["at"]["x"])) ** 2 + (F(x["at"]["y"]) - F(f["at"]["y"])) ** 2 <= T * T)
        if not f["exception"] and any(near(x) for x in lapsed):
            g["lapsed"] = True
        out.append(g)
    return out


def answer(findings, rules=2, objects=5, limit=None, missing=0, neighbours=0, incomplete=(), sources=(), preview=None):
    shown = findings if limit is None else findings[:limit]
    exc = sum(1 for f in findings if f["exception"])
    lapsed = sum(1 for f in findings if f.get("lapsed"))
    out = {"findings": shown, "total": len(findings), "open": len(findings) - exc, "exceptions": exc}
    if lapsed:
        out["lapsed"] = lapsed
    out["rules"] = rules
    if missing:
        out["missing"] = missing
    out["objects"] = objects
    if neighbours:
        out["neighbours"] = neighbours
    out["complete"] = not incomplete
    if incomplete:
        out["incomplete"] = list(incomplete)
    stamp = {"revision": "$current", "rules": ANY}
    if sources:
        stamp["sources"] = list(sources)
    out["stamp"] = stamp
    if preview is not None:
        out["preview"] = preview
    return out


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path):
    return {"status": "failed", "error": {"code": code, "message": message, "path": path}}


def step(inp, result, expect=None, op="execute", command=None):
    s = {"op": op, "input": inp, "result": result, "expect": expect or {"revision": "same"}}
    if command:
        s["command"] = command
    return s


def other(command, inp, expect=None):
    return step(inp, {"status": "completed", "output": ANY, "warnings": ANY}, expect or {"revision": "changed"},
                command=command)


def case(name, *steps, setup_=None):
    c = {"name": name, "steps": list(steps)}
    if setup_ is not None:
        c["setup"] = setup_
    return c


LAPSED_TEXT = ("{} istisnanın nesnelerinin geometrisi istisna yapıldıktan sonra değişti; bulguları yeniden açık. "
               "Gözden geçirip yeniden istisna yapın ya da düzeltin.")
INCOMPLETE_TEXT = "“{}” katmanının denetimi eksik: {} Katman temiz sayılmaz."
BEYOND = ("parçanın {} nesnesi okunan bandın dışına uzanıyor; oradaki komşuları okunmadı. Alanı genişletin.")
UNREAD = "alanın dışındaki komşular okunamadı: {}."
NOT_FOUND = "Çizimin bugünkü denetiminde “{}” anahtarlı bulgu yok; önce cad.topology.check ile denetleyin."
NOT_OFFERED = "Bu bulgu “{}” düzeltmesini sunmuyor; sunduğu düzeltmeler: {}."
OUTSIDE_FIX = ("Bu düzeltme alanın dışındaki bir satırı değiştirir; alanı genişletip satırı çalışma kopyasına alın.")
LIMIT_TEXT = "Liste sınırı 1 ile 100000 arasında bir tam sayı olmalı."


def lapsed_warning(n):
    return {"code": "exception_lapsed", "message": LAPSED_TEXT.format(n)}


def stretch_a2():
    """A2's right edge from x = 18 to 19: its geometry changes, the overlap with A1 stays where it was."""
    return other("cad.entities.stretch", {
        "window": {"min": gi.P(E + 17.5, N + 1.5), "max": gi.P(E + 18.5, N + 8.5)}, "dx": 1, "dy": 0,
        "uids": [uid(2)]})


def exc_of(f):
    return {"rule": f["rule"], "objects": f["uids"], "at": f["at"]}


def fix_change(objs, k, before, after, layer="parsel"):
    return {"uid": objs[k].uid, "layer": layer, "areaBefore": float(before), "areaAfter": float(after)}


# ── the part's scene ───────────────────────────────────────────────────

# In metres from (E, N). G1 is in the area; G2's box starts past it but in the band and overlaps G1 by 0.25 × 10; G3
# and G4 lie under the area in the band and overlap each other by 1 × 1.
G_ROWS = [
    ("G1", (0, 0, 11, 10)),
    ("G2", (F(43, 4), 0, 20, 10)),
    ("G3", (0, F(-5, 2), 5, F(-3, 2))),
    ("G4", (4, F(-5, 2), 8, F(-3, 2))),
]
AREA = [E - 1.0, N - 1.0, E + 10.5, N + 11.0]
NARROW = [E - 1.0, N - 1.0, E + 2.0, N + 11.0]
LAYER = "import-parsel"


def write_files():
    from osgeo import ogr, osr
    for name, ad in ((PART, "G4"), (PART_2, "G4 (değişti)")):
        path = DIR / name
        if path.exists():
            path.unlink()
        srs = osr.SpatialReference()
        srs.ImportFromEPSG(SRID)
        ds = ogr.GetDriverByName("GPKG").CreateDataSource(str(path))
        lyr = ds.CreateLayer("parsel", srs=srs, geom_type=ogr.wkbPolygon)
        lyr.CreateField(ogr.FieldDefn("ad", ogr.OFTString))
        for name_, (x0, y0, x1, y1) in G_ROWS:
            f = ogr.Feature(lyr.GetLayerDefn())
            pts = [(E + x0, N + y0), (E + x1, N + y0), (E + x1, N + y1), (E + x0, N + y1)]
            f.SetGeometry(ogr.CreateGeometryFromWkt(
                "POLYGON ((" + ", ".join(f"{float(x)!r} {float(y)!r}" for x, y in [*pts, pts[0]]) + "))"))
            f.SetField("ad", ad if name_ == "G4" else name_)
            lyr.CreateFeature(f)
        ds = None


def g_uid(name):
    fid = next(i for i, (a, _) in enumerate(G_ROWS, 1) if a == name)
    return str(uuid.uuid5(gi.NAMESPACE, f"gpkg:parsel:{fid}"))


def g_objs(names):
    out = []
    for n in names:
        x0, y0, x1, y1 = dict(G_ROWS)[n]
        o = tr.area_obj(LAYER, tr.rect(x0, y0, x1, y1))
        o.uid = g_uid(n)
        out.append(o)
    return out


def sha(name):
    return hashlib.sha256((DIR / name).read_bytes()).hexdigest()


def fx(name):
    return f"$fixture:geopackage/v1/{name}"


def band(extent):
    w = 0.15 * min(extent[2] - extent[0], extent[3] - extent[1])
    return [extent[0] - w, extent[1] - w, extent[2] + w, extent[3] + w]


def meets(box, area):
    x0, y0, x1, y1 = (E + float(v) if i % 2 == 0 else N + float(v) for i, v in enumerate(box))
    return not (x1 < area[0] or x0 > area[2] or y1 < area[1] or y0 > area[3])


def within(box, area):
    x0, y0, x1, y1 = (E + float(v) if i % 2 == 0 else N + float(v) for i, v in enumerate(box))
    return x0 >= area[0] and y0 >= area[1] and x1 <= area[2] and y1 <= area[3]


def part_scene(extent):
    """(held, outside) row names: the rows whose box meets the area, then those whose box meets the band alone."""
    held = [n for n, b in G_ROWS if meets(b, extent)]
    b = band(extent)
    outside = [n for n, box in G_ROWS if n not in held and meets(box, b)]
    return held, outside


P_RULES = [tr.rule("p1", "mustNotOverlap", LAYER)]


def part_setup(exceptions=()):
    topology = {"tolerance": float(T), "rules": rules_json(P_RULES)}
    if exceptions:
        topology["exceptions"] = list(exceptions)
    return {
        "format": "kentos.document", "version": 1, "name": "Topoloji: bağlı parça",
        "settings": {"srid": SRID, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "topology": topology},
        "origin": gi.P(0, 0),
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False,
                    "expanded": True, "style": STYLE, "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []},
    }


def link(path, extent):
    return other("cad.sources.edit", {"edit": {"kind": "link", "path": path, "table": "parsel", "extent": extent}})


def part_answer(extent, name=PART, preview=None, unread=None):
    held, outside = part_scene(extent)
    objs = g_objs(held + outside)
    outs = {g_uid(n) for n in outside}
    gaps = []
    beyond = [n for n in held if not within(dict(G_ROWS)[n], band(extent))]
    if beyond:
        gaps.append({"layer": LAYER, "reason": BEYOND.format(len(beyond))})
    if unread:
        gaps.append({"layer": LAYER, "reason": UNREAD.format(unread)})
        objs = g_objs(held)
        outside = []
    fs = found(objs, P_RULES, outside=outs)
    src = {"layer": LAYER, "revision": sha(name)}
    if not unread:
        src["band"] = ANY
    out = answer(fs, rules=1, objects=len(held), neighbours=len(outside), incomplete=gaps, sources=[src],
                 preview=preview)
    # The bound layer takes the table's name (docs/adr/0503).
    warnings = [{"code": "topology_incomplete", "message": INCOMPLETE_TEXT.format("parsel", g["reason"])}
                for g in gaps]
    return fs, objs, out, warnings


# ── cad.topology.check ─────────────────────────────────────────────────


def check_cases():
    out = []
    objs = drawing_objs()
    fs = found(objs, RULES)
    over = fs[0]
    assert over["problem"] == "overlap" and over["uids"] == [uid(1), uid(2)], over
    short = next(f for f in fs if f["rule"] == "r2")
    c = lambda name, *steps, setup_=None: out.append(case(name, *steps, setup_=setup_))  # noqa: E731

    c("bütün kurallar: iki çakışma ve kısa kenar, anahtarları, ölçüleri ve düzeltmeleriyle", step({}, done(answer(fs))))
    c("plan da aynı cevabı verir, çizim değişmez", step({}, done(answer(fs)), op="plan"))
    c("yalnız r2: kısa kenar", step({"rules": ["r2"]}, done(answer(found(objs, [R2]), rules=1))))
    c("yalnız r1: iki çakışma", step({"rules": ["r1"]}, done(answer(found(objs, [R1]), rules=1))))
    c("bilinmeyen kural reddedilir", step({"rules": ["r1", "r9"]},
                                          failed("topology_rule_not_found", "Projede “r9” kimlikli topoloji kuralı yok.",
                                                 "rules[1]")))
    c("liste sınırı: ilk bulgu verilir, sayılar hepsinin", step({"limit": 1}, done(answer(fs, limit=1))))
    c("liste sınırı sıfır olamaz", step({"limit": 0}, failed("invalid_limit", LIMIT_TEXT, "limit")))
    c("liste sınırı 100000'i aşamaz", step({"limit": 100001}, failed("invalid_limit", LIMIT_TEXT, "limit")))

    def preview(k, fix, before, after):
        return {"key": over["key"], "fix": fix, "changes": [fix_change(objs, k, before, after)]}

    first = tr.overlap_fix(objs, 0, 1, "subtractFirst")[0]["area"]
    second = tr.overlap_fix(objs, 0, 1, "subtractSecond")[0]["area"]
    c("önizleme: ilkinden çıkar, A1'in alanı önce ve sonra",
      step({"preview": {"key": over["key"], "fix": "subtractFirst"}},
           done(answer(fs, preview=preview(0, "subtractFirst", 100, first)))))
    c("önizleme: ikincisinden çıkar, A2'nin alanı önce ve sonra",
      step({"preview": {"key": over["key"], "fix": "subtractSecond"}},
           done(answer(fs, preview=preview(1, "subtractSecond", 60, second)))))
    c("önizleme: bilinmeyen anahtar", step({"preview": {"key": "0000000000000000", "fix": "subtractFirst"}},
                                         failed("finding_not_found", NOT_FOUND.format("0000000000000000"),
                                                "preview.key")))
    c("önizleme: bulgunun sunmadığı düzeltme",
      step({"preview": {"key": over["key"], "fix": "snapEnd"}},
           failed("fix_unavailable", NOT_OFFERED.format("snapEnd", ", ".join(over["fixes"])), "preview.fix")))
    if not short["fixes"]:
        c("önizleme: düzeltmesi olmayan bulgu",
          step({"preview": {"key": short["key"], "fix": "subtractFirst"}},
               failed("fix_unavailable", NOT_OFFERED.format("subtractFirst", "yok"), "preview.fix")))
    x = exc_of(over)
    c("istisna (özetsiz): bulgu istisna, açık sayısı bir eksik",
      step({}, done(answer(found(objs, RULES, exceptions=[x])))), setup_=setup(exceptions=[x]))
    xd = dict(x, digest="0000000000000000")
    c("geometrisi değişmiş istisna düşer: bulgu yeniden açık, uyarılır",
      step({}, done(answer(found(objs, RULES, exceptions=[x], lapsed=[x])), [lapsed_warning(1)])),
      setup_=setup(exceptions=[xd]))
    c("olmayan katmanın kuralı sayılır, öbürleri denetlenir",
      step({}, done(answer(fs, rules=3, missing=1))),
      setup_=setup(rules=[*RULES, tr.rule("r3", "mustNotOverlap", "yok")]))
    c("kural yoksa bulgu da yok", step({}, done(answer([], rules=0, objects=0))), setup_=setup(rules=[]))
    moved = drawing_objs()
    moved[1] = tr.area_obj("parsel", tr.rect(108, 2, 118, 8))
    moved[1].uid = uid(2)
    c("A2 uzağa taşınınca çakışması kalkar",
      other("cad.entities.transform", {"uids": [uid(2)], "transform": {"kind": "move", "dx": 100, "dy": 0}}),
      step({}, done(answer(found(moved, RULES)))))
    c("esnetilen A2: çakışma yerinde, ölçüsü aynı, anahtarı aynı",
      stretch_a2(), step({"rules": ["r1"]}, done(answer(found(drawing_objs(True), [R1]), rules=1))))

    # The part.
    fs_p, objs_p, ans, warn = part_answer(AREA)
    assert [f["uids"] for f in fs_p] == [[g_uid("G1"), g_uid("G2")]], fs_p
    c("bağlı parça: alanın dışındaki komşuyla çakışma bulunur, yalnız dışarıdakilerinki bırakılır",
      link(fx(PART), AREA), step({}, done(ans, warn)), setup_=part_setup())
    pk = fs_p[0]["key"]
    g1 = next(k for k, o in enumerate(objs_p) if o.uid == g_uid("G1"))
    fixed = tr.overlap_fix(objs_p, 0, 1, "subtractFirst")[0]["area"]
    pv = {"key": pk, "fix": "subtractFirst", "changes": [fix_change(objs_p, g1, 110, fixed, LAYER)],
          "sources": [{"layer": LAYER, "table": "parsel", "rows": 1}]}
    _, _, ans_pv, _ = part_answer(AREA, preview=pv)
    c("bağlı parça: parçanın satırını değiştiren düzeltmenin önizlemesi kaynağın tablosunu söyler",
      link(fx(PART), AREA), step({"preview": {"key": pk, "fix": "subtractFirst"}}, done(ans_pv, warn)),
      setup_=part_setup())
    c("bağlı parça: dışarıdaki satırı değiştirecek düzeltme önizlenmez",
      link(fx(PART), AREA), step({"preview": {"key": pk, "fix": "subtractSecond"}},
                                 failed("fix_unavailable", OUTSIDE_FIX, "preview.fix")),
      setup_=part_setup())
    _, _, ans_n, warn_n = part_answer(NARROW)
    c("bağlı parça: bandın dışına uzanan nesne denetimi eksik bırakır",
      link(fx(PART), NARROW), step({}, done(ans_n, warn_n)), setup_=part_setup())
    tmp = "$tmp:topoloji-parca.gpkg"
    _, _, ans_u, warn_u = part_answer(AREA, unread="dosya katmanın son eşleştiği sürümden farklı")
    c("bağlı parça: dosya değişince band okunamaz, denetim eksik",
      {"op": "copyFile", "from": fx(PART), "to": tmp}, link(tmp, AREA),
      {"op": "copyFile", "from": fx(PART_2), "to": tmp}, step({}, done(ans_u, warn_u)), setup_=part_setup())
    return out


# ── cad.topology.exceptions ────────────────────────────────────────────


def exc_cases():
    out = []
    objs = drawing_objs()
    fs = found(objs, RULES)
    over, over2 = fs[0], fs[1]
    short = next(f for f in fs if f["rule"] == "r2")
    c = lambda name, *steps, setup_=None: out.append(case(name, *steps, setup_=setup_))  # noqa: E731
    check = lambda answer_, warnings=(): step({}, done(answer_, warnings), command="cad.topology.check")  # noqa: E731

    def edited(changed, exceptions):
        return done({"changed": changed, "exceptions": exceptions, "revision": "$current"})

    # The exceptions are the project's settings: a write makes the drawing dirty and is no undo step.
    def mark(keys, changed, exceptions, op="execute", expect=None):
        return step({"operation": "mark", "keys": keys}, edited(changed, exceptions),
                    expect or {"revision": "changed" if changed and op != "plan" else "same"}, op=op)

    def unmark(keys, changed, exceptions):
        return step({"operation": "unmark", "keys": keys}, edited(changed, exceptions),
                    {"revision": "changed" if changed else "same"})

    x1, x2 = exc_of(over), exc_of(over2)

    def held(xs):
        return {"settings": {"topology": {"tolerance": float(T), "rules": rules_json(RULES),
                                          "exceptions": [dict(x, digest=ANY) for x in xs]}}}

    c("istisna yap: bulgu istisna olur, ayarlarda geometrisinin özetiyle",
      mark([over["key"]], 1, 1, expect={"revision": "changed", "canUndo": False, "dirty": True, **held([x1])}),
      check(answer(found(objs, RULES, exceptions=[x1]))))
    c("plan yazmaz", mark([over["key"]], 1, 1, op="plan"), check(answer(fs)))
    c("iki bulgu birlikte", mark([over["key"], over2["key"]], 2, 2),
      check(answer(found(objs, RULES, exceptions=[x1, x2]))))
    c("istisna olan bulgu yeniden işaretlenmez", mark([over["key"]], 1, 1), mark([over["key"]], 0, 1))
    c("anahtar yok", step({"operation": "mark", "keys": []},
                          failed("invalid_input", "Bulgu anahtarı verilmedi; cad.topology.check'in verdiği anahtarlardan en az birini verin.", "keys")))
    c("aynı anahtar iki kez", step({"operation": "mark", "keys": [over["key"], over["key"]]},
                                   failed("invalid_input", f"“{over['key']}” anahtarı birden çok kez verildi.",
                                          "keys[1]")))
    c("bilinmeyen anahtar", step({"operation": "mark", "keys": ["0000000000000000"]},
                                 failed("finding_not_found", NOT_FOUND.format("0000000000000000"), "keys[0]")))
    c("biri bilinmeyince hiçbiri yazılmaz",
      step({"operation": "mark", "keys": [over["key"], "ffffffffffffffff"]},
           failed("finding_not_found", NOT_FOUND.format("ffffffffffffffff"), "keys[1]")),
      check(answer(fs)))
    c("istisnayı kaldır", mark([over["key"]], 1, 1), unmark([over["key"]], 1, 0), check(answer(fs)))
    c("açık bulgunun istisnası kaldırılmaz", unmark([over["key"]], 0, 0))
    c("özetsiz istisna kaldırılır", unmark([over["key"]], 1, 0), setup_=setup(exceptions=[x1]))
    c("geometrisi değişince istisna düşer, uyarılır",
      mark([over["key"]], 1, 1), stretch_a2(),
      check(answer(found(drawing_objs(True), RULES, exceptions=[x1], lapsed=[x1])), [lapsed_warning(1)]))
    c("düşen istisna yeniden yapılır, bugünkü geometriyle",
      mark([over["key"]], 1, 1), stretch_a2(), mark([over["key"]], 1, 1),
      check(answer(found(drawing_objs(True), RULES, exceptions=[x1]))))
    c("düşen istisna kaldırılır",
      mark([over["key"]], 1, 1), stretch_a2(), unmark([over["key"]], 1, 0),
      check(answer(found(drawing_objs(True), RULES))))
    xd = dict(x1, digest="0000000000000000")
    c("kurulumda düşmüş istisna yeniden yapılınca tutar", mark([over["key"]], 1, 1),
      check(answer(found(objs, RULES, exceptions=[x1]))), setup_=setup(exceptions=[xd]))
    c("başka istisnalar korunur", mark([over["key"]], 1, 2),
      check(answer(found(objs, RULES, exceptions=[x1, x2]))), setup_=setup(exceptions=[x2]))
    c("kısa kenar da istisna olur", mark([short["key"]], 1, 1),
      check(answer(found(objs, RULES, exceptions=[exc_of(short)]))))
    c("beklenen sürüm tutarsa yazılır",
      step({"operation": "mark", "keys": [over["key"]], "expectedRevision": "$current"}, edited(1, 1),
           {"revision": "changed", "canUndo": False}))
    c("beklenen sürüm tutmazsa çakışma",
      step({"operation": "mark", "keys": [over["key"]], "expectedRevision": "999"},
           {"status": "conflict", "error": {
               "code": "revision_conflict",
               "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki "
                          "hâline göre yeniden hazırlayın.",
               "path": "expectedRevision", "revision": "$current"}}))
    c("beklenen sürüm yazı değilse ret",
      step({"operation": "mark", "keys": [over["key"]], "expectedRevision": "x"},
           failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı "
                  "yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")))
    # The part: an exception of a finding with a row outside.
    fs_p, objs_p, _, warn = part_answer(AREA)
    xp = exc_of(fs_p[0])
    fs_px = found(objs_p, P_RULES, exceptions=[xp], outside={g_uid(n) for n in part_scene(AREA)[1]})
    held_n, out_n = part_scene(AREA)
    ans = answer(fs_px, rules=1, objects=len(held_n), neighbours=len(out_n),
                 sources=[{"layer": LAYER, "revision": sha(PART), "band": ANY}])
    c("bağlı parça: dışarıdaki satırla çakışma istisna yapılır",
      link(fx(PART), AREA), mark([fs_p[0]["key"]], 1, 1), check(ans, warn), setup_=part_setup())
    return out


def build(command, cases, title, setup_=None):
    return {
        "format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
        "note": "Bulgular topology_rules_cases.py'nin başvurusundan (ADR 0202), anahtarlar ADR 0524'ün FNV-1a kuralından, "
                "bant ve istisnanın düşmesi ADR 0589'dan; parçanın satırlarını GDAL'ın OGR'ı yazar. "
                "scripts/fixtures/topology_check_cases.py yazar.",
        "setup": setup_ or setup(),
        "cases": cases,
    }


def main():
    if "--files" in sys.argv:
        write_files()
        print(f"yazıldı: {PART}, {PART_2}")
        return
    files = [
        (OUT_CHECK, build("cad.topology.check", check_cases(), "Topolojiyi denetle (ADR 0589)")),
        (OUT_EXC, build("cad.topology.exceptions", exc_cases(), "Topoloji istisnaları (ADR 0589)")),
    ]
    bad = []
    for path, data in files:
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        if "--check" in sys.argv:
            if not path.exists() or path.read_text() != text:
                bad.append(path.name)
            else:
                print(f"{path.name}: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"{path.name}: {len(data['cases'])} durum yazıldı")
    if bad:
        sys.exit(f"güncel değil: {', '.join(bad)}; betiği --check'siz çalıştırın")


if __name__ == "__main__":
    main()
