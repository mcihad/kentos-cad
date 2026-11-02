#!/usr/bin/env python3
"""Writes fixtures/commands/v1/road.alignments.{create,edit,query}.json: the shared cases of KentOS Yol's horizontal
alignment commands (plan/road.md Y3, docs/adr/0901), their expectations built here without KentOS code: the geometry is
scripts/fixtures/road_alignment_cases.py's (the textbook PI formulas, mpmath at 40 digits), the ground length PROJ's
point scale and the Euler radius integrated along the alignment. The desktop
(crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    uv run --no-project --python 3.11 --with mpmath --with pyproj python3 scripts/fixtures/road_alignment_command_cases.py
    ... --check   # compares the files with the ones on disk
"""

import copy
import json
import math
import sys
from pathlib import Path

import mpmath as mp

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_alignment_cases as ref  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
ALGORITHM = "kentos.alignment/1"
W = 1e-6
NO_GROUND = "Zemin boyu için projenin ortalama elipsoit yüksekliği yazılmalı (Proje ayarları › Ölçme)."
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X0, Y0 = 485_000.0, 4_420_000.0


def pi(id, x, y, r=0, l1=0, l2=0):
    p = {"id": id, "x": float(x), "y": float(y)}
    if r:
        p["radius"] = float(r)
    if l1:
        p["spiralIn"] = float(l1)
    if l2:
        p["spiralOut"] = float(l2)
    return p


def full(p):
    return {"id": p["id"], "x": p["x"], "y": p["y"], "radius": p.get("radius", 0), "spiralIn": p.get("spiralIn", 0),
            "spiralOut": p.get("spiralOut", 0)}


MAIN_PIS = [pi("a", X0, Y0), pi("p1", X0 + 600, Y0, 400, 80, 80), pi("b", X0 + 1100, Y0 + 500)]
MAIN = {"id": "g-1", "name": "Ana aks", "revision": 1, "pis": MAIN_PIS, "stationing": {"start": 2000.0}}
LEFT = {"id": "g-2", "name": "Sol kenar", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": -7.5}}
LINK_PIS = [pi("a", X0 - 1000, Y0 - 1000), pi("p1", X0 - 900, Y0 - 1000), pi("b", X0 - 900, Y0 - 900)]
LINK = {"id": "g-3", "name": "Bağlantı", "revision": 1, "pis": LINK_PIS}
EQUATIONS = {"start": 2000.0, "equations": [{"back": 2200.0, "ahead": 2500.0},
                                            {"back": 2650.0, "ahead": 2700.0, "increasing": False}]}


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(STYLE), "children": []}


def setup(alignments=None, work="road", ground=None):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if alignments:
        settings["road"] = {"alignments": copy.deepcopy(alignments)}
    if ground is not None:
        settings["survey"] = {"groundHeight": ground}
    return {"format": "kentos.document", "version": 2, "name": "Yol aksları", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": [node("0", "0")], "activeLayer": "0", "entities": [],
            "uids": [], "styles": {"items": [], "categories": []}}


def road_of(alignments):
    return {"alignments": alignments} if alignments else None


# ── The geometry, by the reference ─────────────────────────────────────────────────────────────────────────────────

class Built:
    def __init__(self, els, length, reports, joins, regions, offset=0):
        self.els, self.length, self.reports, self.joins, self.regions = els, length, reports, joins, regions
        self.offset = mp.mpf(offset)


def stationing(d):
    st = d.get("stationing") or {}
    return {"start": st.get("start", 0.0),
            "equations": [{"back": q["back"], "ahead": q["ahead"], "increasing": q.get("increasing", True)}
                          for q in st.get("equations", [])]}


def build(alignments, d):
    if "offsetOf" in d:
        src = next(a for a in alignments if a["id"] == d["offsetOf"]["alignment"])
        b = build(alignments, src)
        b.offset = mp.mpf(d["offsetOf"]["offset"])
        return b
    els, length, reports, joins = ref.build([full(p) for p in d["pis"]])
    regions = ref.regions(stationing(d), length)
    return Built(els, length, reports, joins, regions)


def km_text(km, digits=3):
    scale = 10 ** digits
    rounded = math.floor(abs(float(km)) * scale + 0.5) / scale
    whole = math.floor(rounded / 1000.0)
    rest = rounded - whole * 1000.0
    sign = "-" if km < 0 and rounded > 0 else ""
    return f"{sign}{whole}+{rest:0{4 + digits}.{digits}f}"


def region_of(regions, s):
    return max(i for i, r in enumerate(regions) if r[0] <= s)


def element_at(b, s):
    e = b.els[0]
    for x in b.els:
        if x.station <= s:
            e = x
    return e


def place(b, s):
    s = mp.mpf(s)
    e = element_at(b, s)
    x, y = e.point(s - e.station)
    th = e.theta(s - e.station)
    k = e.curvature(s - e.station)
    d = b.offset
    km = ref.km_of(b.regions, s)
    return {"station": float(s), "km": float(km), "kmText": km_text(km), "region": region_of(b.regions, s),
            "x": float(x + d * mp.sin(th)), "y": float(y - d * mp.cos(th)), "direction": float(th),
            "curvature": float(k / (1 + k * d)), "element": e.id}


def length_of(b):
    if b.offset == 0:
        return float(b.length)
    return float(sum(e.length + b.offset * (e.theta(e.length) - e.theta0) for e in b.els))


def ground_of(b, height):
    """∫ ds_grid / (k(s) · R_e/(R_e + h)), k PROJ's point scale, R_e the Euler radius in the line's azimuth."""
    if height is None:
        return None, NO_GROUND
    from pyproj import CRS, Proj, Transformer
    proj = Proj(CRS.from_epsg(5254))
    to_geo = Transformer.from_crs(5254, 4326, always_xy=True)
    a, f = 6378137.0, 1 / 298.257222101
    e2 = f * (2 - f)
    total = 0.0
    for e in b.els:
        n = max(8, math.ceil(float(e.length) / 5))
        h = float(e.length) / n
        for i in range(n):
            # Simpson on each piece.
            vals = []
            for t in (h * i, h * (i + 0.5), h * (i + 1)):
                th = e.theta(t)
                x, y = e.point(t)
                x, y = float(x + b.offset * mp.sin(th)), float(y - b.offset * mp.cos(th))
                lon, lat = to_geo.transform(x, y)
                fac = proj.get_factors(lon, lat)
                grid_az = 90.0 - math.degrees(float(th))
                az = math.radians(grid_az + fac.meridian_convergence)
                phi = math.radians(lat)
                w = math.sqrt(1 - e2 * math.sin(phi) ** 2)
                m_, n_ = a * (1 - e2) / w ** 3, a / w
                r = m_ * n_ / (n_ * math.cos(az) ** 2 + m_ * math.sin(az) ** 2)
                grid_ds = 1.0 + float(e.curvature(t)) * float(b.offset)
                vals.append(grid_ds / (fac.meridional_scale * r / (r + height)))
            total += h / 6 * (vals[0] + 4 * vals[1] + vals[2])
    height_text = f"{height:g}"
    return total, f"Zemin boyu: her parçanın ızgara boyu, ölçek ve {height_text} m ortalama elipsoit yüksekliğinin çarpanına bölünerek."


def report(alignments, d, height=None):
    b = build(alignments, d)
    els = []
    for e in b.els:
        row = {"id": e.id, "kind": e.kind, "station": float(e.station), "length": float(e.length),
               "km": float(ref.km_of(b.regions, e.station)), "start": [float(e.start[0]), float(e.start[1])],
               "direction": float(e.theta0), "k0": float(e.k0), "k1": float(e.k1)}
        if e.kind == "arc":
            row["radius"] = float(1 / abs(e.k0))
        if e.kind == "spiral":
            row["a"] = float(mp.sqrt(e.length / abs(e.k1 - e.k0)))
        els.append(row)
    curves = [{"pi": r["pi"], "delta": r["delta"], "radius": r["radius"], "spiralIn": r["spiralIn"],
               "spiralOut": r["spiralOut"], "tIn": r["tIn"], "tOut": r["tOut"], "arc": r["arc"],
               "stations": r["stations"], "centre": r["centre"]} for r in b.reports]
    gl, gw = ground_of(b, height)
    out = {"id": d["id"], "name": d["name"], "revision": d.get("revision", 0), "length": length_of(b),
           "ground": gw, "kmStart": float(ref.km_of(b.regions, 0)), "kmEnd": float(ref.km_of(b.regions, b.length)),
           "elements": els, "curves": curves, "joins": b.joins, "residual": 0.0, "residualDirection": 0.0,
           "algorithm": ALGORITHM}
    if "offsetOf" in d:
        out["offsetOf"] = d["offsetOf"]
    if gl is not None:
        out["groundLength"] = gl
    return out


def listed(alignments):
    return [{"id": a["id"], "name": a["name"], "revision": a.get("revision", 0),
             **({"offsetOf": a["offsetOf"]["alignment"]} if "offsetOf" in a else {})} for a in alignments]


def same_shape(a, b):
    tol = mp.mpf("1e-9")
    return (a.kind == b.kind and abs(a.length - b.length) <= tol and abs(a.k0 - b.k0) <= tol
            and abs(a.k1 - b.k1) <= tol and abs(a.start[0] - b.start[0]) <= tol
            and abs(a.start[1] - b.start[1]) <= tol and abs(ref.wrap(a.theta0 - b.theta0)) <= tol)


def mapping(before, after):
    rng = lambda e: [float(e.station), float(e.station + e.length)]
    rows = []
    for n in (after.els if after else []):
        o = next((x for x in (before.els if before else []) if x.id == n.id), None)
        if o is None:
            rows.append({"element": n.id, "change": "added", "after": rng(n)})
            continue
        change = "changed" if not same_shape(o, n) else ("kept" if abs(o.station - n.station) <= 1e-9 else "moved")
        rows.append({"element": n.id, "change": change, "before": rng(o), "after": rng(n)})
    for o in (before.els if before else []):
        if not after or all(n.id != o.id for n in after.els):
            rows.append({"element": o.id, "change": "removed", "before": rng(o)})
    return rows


def locate(b, p):
    """Feet as the reference finds them, a parallel axis's read off its source."""
    got = ref.locate(b.els, b.length, p)
    feet = []
    for f in got["feet"]:
        s = mp.mpf(f["station"])
        corner = False
        for e in b.els[1:]:
            if abs(e.station - s) < mp.mpf("1e-6"):
                prev = b.els[b.els.index(e) - 1]
                corner = abs(ref.wrap(e.theta0 - prev.theta(prev.length))) > mp.mpf("1e-9")
        at = place(b, s)
        feet.append({"at": at, "offset": f["offset"] - float(b.offset), "corner": corner})
    out = {"point": [float(p[0]), float(p[1])], "feet": feet}
    if feet:
        out["nearest"] = feet[0]
    if got["beyond"]:
        out["beyond"] = got["beyond"]["end"]
    return out


# ── Cases ─────────────────────────────────────────────────────────────────────────────────────────────────────────────

def completed(output, warns=()):
    return {"status": "completed", "output": output, "warnings": list(warns)}


def failed(code, message, path=None):
    e = {"code": code, "message": message}
    if path is not None:
        e["path"] = path
    return {"status": "failed", "error": e}


def step(op, input_, result, expect=None):
    s = {"op": op, "input": input_, "result": result}
    if expect is not None:
        s["expect"] = expect
    s["within"] = W
    return s


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
STEPPED = {"canUndo": True, "canRedo": False, "dirty": True, "revision": "changed"}
NOT_ROAD = failed("not_a_road_project", "Aks yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.")
CONFLICT = {"status": "conflict", "error": {
    "code": "revision_conflict", "message": "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın.",
    "path": "expectedRevision", "revision": "$current"}}
BAD_REVISION = failed("invalid_revision", "Beklenen sürüm “on” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision")


def rule(words, path):
    return failed("invalid_alignment", f"Aks kurallarına uymuyor: {words}.", path)


def file(command, title, base, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": command, "commandVersion": 1, "title": title,
            "note": "scripts/fixtures/road_alignment_command_cases.py yazar; geometri road_alignment_cases.py'nin "
                    "mpmath başvurusundandır, zemin boyu PROJ'un ölçeğinden; KentOS kodu yok. `expect.road` projenin "
                    "yol ayarlarıdır. Aks geri alma geçmişinde tutulan bir proje ayarıdır.",
            "setup": base, "cases": cases}


def chain_message(e, pis):
    k = e["kind"]
    at = lambda id_: f"pis[{[p['id'] for p in pis].index(id_)}]"
    if k == "SpiralsLong":
        return failed("alignment_spirals_long",
                      f"{e['pi']} noktasında klotoidler {math.degrees(e['turn']):.4f}° dönüyor, sapma "
                      f"{math.degrees(e['delta']):.4f}°; klotoidleri kısaltın ya da yarıçapı büyütün.", at(e["pi"]))
    if k == "CurvesOverlap":
        return failed("alignment_curves_overlap",
                      f"{e['a']}–{e['b']} aliymanına kurplar sığmıyor: teğet boyları aliymandan {e['excess']:.3f} m "
                      "uzun. Yarıçapı ya da klotoidleri küçültün, PI'ları uzaklaştırın.", at(e["b"]))
    if k == "SamePlace":
        return failed("invalid_alignment",
                      f"{e['pi']} noktası bir öncekiyle aynı yerde; PI'lar ayrı yerlerde olmalı.", at(e["pi"]))
    if k == "TurnTooSharp":
        return failed("invalid_alignment",
                      f"{e['pi']} noktasında doğrultu geri dönüyor (180°); aks kendi üstüne dönemez.", at(e["pi"]))
    if k == "BadCurve":
        return failed("invalid_alignment",
                      f"{e['pi']} noktasının kurpu olamaz: başlangıç ve bitişte kurp yok, klotoid yarıçapla verilir, "
                      "değerler eksi olmaz.", at(e["pi"]))
    raise ValueError(k)


def refusal_of(pis):
    try:
        ref.build([full(p) for p in pis])
    except ref.Refused as r:
        return chain_message(r.kw, pis)
    raise AssertionError("builds")


def cusp(name, b, d, path):
    for e in b.els:
        if e.kind == "tangent":
            continue
        fa, fb = 1 + e.k0 * d <= 0, 1 + e.k1 * d <= 0
        if fa or fb:
            rate = e.rate()
            sf = min(max((-1 / mp.mpf(d) - e.k0) / rate if rate else 0, 0), e.length)
            frm, to = (0, e.length) if fa and fb else ((0, sf) if fa else (sf, e.length))
            return failed("offset_cusp", f"“{name}” paralel aksı km {float(e.station + frm):.3f}–"
                                         f"{float(e.station + to):.3f} arasında kıvrılıyor: orada kurpun yarıçapı "
                                         "ötelemeden küçük. Ötelemeyi küçültün ya da kurpları genişletin.", path)
    raise AssertionError("no cusp")


def written(alignments, target, mapping_rows, height=None):
    out = {"mapping": mapping_rows, "dependents": [a["id"] for a in alignments if a.get("offsetOf", {}).get("alignment") == target] if target else [],
           "alignments": listed(alignments), "revision": "$current"}
    if target:
        d = next(a for a in alignments if a["id"] == target)
        out["alignment"] = report(alignments, d, height)
    return out


# ── road.alignments.create ──────────────────────────────────────────────────────────────────────────────────────────

def create_cases():
    start = [MAIN]
    cases = []

    def ok(name, input_, d, op="execute", undo=False):
        after = start + [d]
        b = build(after, d)
        rows = [] if "offsetOf" in d else mapping(None, b)
        out = written(after, d["id"], rows)
        if op == "validate":
            result, expect = completed(None), {**UNCHANGED, "road": road_of(start)}
        elif op == "plan":
            result, expect = completed(out), {**UNCHANGED, "road": road_of(start)}
        else:
            result, expect = completed(out), {**STEPPED, "road": road_of(after)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Aks oluştur", "expect": {"road": road_of(start), "canRedo": True}})
        cases.append({"name": name, "steps": steps})

    def no(name, input_, result, setup_=None, before=start):
        c = {"name": name, "steps": [step("execute", input_, result,
                                          {**UNCHANGED, "road": road_of(before)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    sym = [pi("a", X0, Y0 + 2000), pi("p1", X0 + 600, Y0 + 2000, 400, 80, 80), pi("b", X0 + 1100, Y0 + 2500)]
    ok("Simetrik klotoidli kurp, km 2+000'den: kimliği ilk boş g-2",
       {"name": "Yan yol", "pis": sym, "stationing": {"start": 2000.0}},
       {"id": "g-2", "name": "Yan yol", "revision": 1, "pis": sym, "stationing": {"start": 2000.0}})
    ok("Kendi kimliğiyle", {"name": "Yan yol", "id": "yan", "pis": sym},
       {"id": "yan", "name": "Yan yol", "revision": 1, "pis": sym})
    kink = [pi("a", X0, Y0 + 3000), pi("p1", X0 + 200, Y0 + 3000), pi("b", X0 + 200, Y0 + 3200)]
    ok("Kurpsuz kırık nokta", {"name": "Kırık", "pis": kink}, {"id": "g-2", "name": "Kırık", "revision": 1, "pis": kink})
    une = [pi("a", X0, Y0 + 4000), pi("p1", X0 + 600, Y0 + 4100, 350, 60, 120), pi("b", X0 + 900, Y0 + 4700)]
    ok("Eşit olmayan klotoidler", {"name": "Eşitsiz", "pis": une}, {"id": "g-2", "name": "Eşitsiz", "revision": 1, "pis": une})
    only = [pi("a", X0, Y0 + 5000), pi("p1", X0 + 500, Y0 + 5000, 300, 90, 0), pi("b", X0 + 500, Y0 + 5600)]
    ok("Yalnız giriş klotoidi", {"name": "Tek klotoid", "pis": only},
       {"id": "g-2", "name": "Tek klotoid", "revision": 1, "pis": only})
    five = [pi("a", X0, Y0 - 3000), pi("p1", X0 + 800, Y0 - 2880, 500, 100, 100), pi("p2", X0 + 1500, Y0 - 3300, 350, 70, 90),
            pi("p3", X0 + 2400, Y0 - 3250), pi("p4", X0 + 3100, Y0 - 2600, 600), pi("b", X0 + 3900, Y0 - 2500)]
    ok("Beş PI, kırık nokta ve klotoidsiz kurp", {"name": "Uzun aks", "pis": five, "stationing": {"start": 12000.0}},
       {"id": "g-2", "name": "Uzun aks", "revision": 1, "pis": five, "stationing": {"start": 12000.0}})
    eqs = {"start": 2000.0, "equations": [{"back": 2200.0, "ahead": 2500.0}]}
    ok("Km eşitliğiyle", {"name": "Eşitlikli", "pis": sym, "stationing": eqs},
       {"id": "g-2", "name": "Eşitlikli", "revision": 1, "pis": sym, "stationing": eqs})
    rev = [{**p, "x": p["x"], "y": p["y"] + 6000} for p in ref.meeting((X0, 0), [0, 40, -10], [(300, 60, 60), (250, 50, 50)], 400, 300)]
    rev = [pi(p["id"], p["x"], p["y"], p["radius"], p["spiralIn"], p["spiralOut"]) for p in rev]
    ok("Ters kurp: araya aliyman girmez", {"name": "Ters kurp", "pis": rev},
       {"id": "g-2", "name": "Ters kurp", "revision": 1, "pis": rev})
    ok("Paralel aks, sağda 7,5 m", {"name": "Sağ kenar", "offsetOf": {"alignment": "g-1", "offset": 7.5}},
       {"id": "g-2", "name": "Sağ kenar", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": 7.5}})
    ok("Paralel aks, solda 12 m, kaynağın adıyla değil kimliğiyle",
       {"name": "Sol kenar", "offsetOf": {"alignment": "g-1", "offset": -12.0}},
       {"id": "g-2", "name": "Sol kenar", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": -12.0}})
    ok("Boş km yazılmaz", {"name": "Yan yol", "pis": sym, "stationing": {}},
       {"id": "g-2", "name": "Yan yol", "revision": 1, "pis": sym})
    ok("Plan yazmaz", {"name": "Yan yol", "pis": sym}, {"id": "g-2", "name": "Yan yol", "revision": 1, "pis": sym}, op="plan")
    ok("Doğrulama yazmaz", {"name": "Yan yol", "pis": sym}, {"id": "g-2", "name": "Yan yol", "revision": 1, "pis": sym},
       op="validate")
    ok("Geri alınır", {"name": "Yan yol", "pis": sym}, {"id": "g-2", "name": "Yan yol", "revision": 1, "pis": sym},
       undo=True)
    no("Yol projesi değil", {"name": "Yan yol", "pis": sym}, NOT_ROAD, setup(work="map"), [])
    no("Boş ad", {"name": "", "pis": sym},
       rule("aksın adı boş olmamalı, başında ve sonunda boşluk olmamalı, en çok 120 karakter", "name"))
    no("Aynı ad, büyük küçük harf ayrımsız", {"name": "ANA aks", "pis": sym}, rule("bu adda başka bir aks var", "name"))
    no("Aynı kimlik", {"name": "Yan yol", "id": "g-1", "pis": sym}, rule("bu kimlikte başka bir aks var", "id"))
    no("Tek nokta", {"name": "Yan yol", "pis": sym[:1]}, rule("aksın 2 ile 10000 arası noktası olur", "pis"))
    no("Ne PI ne paralel", {"name": "Yan yol"},
       rule("aks ya PI'larıyla ya bir aksa paralel kurulur, ikisi birden değil", "pis"))
    no("Eksi yarıçap", {"name": "Yan yol", "pis": [sym[0], {**sym[1], "radius": -5.0}, sym[2]]},
       rule("yarıçap ve klotoid boyları eksi olamaz", "pis[1].radius"))
    long_ = [pi("a", X0, Y0 + 7000), pi("p1", X0 + 400, Y0 + 7000, 200, 150, 150), pi("b", X0 + 800, Y0 + 7100)]
    no("Klotoidler sapmadan uzun", {"name": "Yan yol", "pis": long_}, refusal_of(long_))
    over = [pi("a", X0, Y0 + 8000), pi("p1", X0 + 300, Y0 + 8000, 300, 50, 50), pi("p2", X0 + 450, Y0 + 8150, 300, 50, 50),
            pi("b", X0 + 450, Y0 + 8500)]
    no("Kurplar biniyor", {"name": "Yan yol", "pis": over}, refusal_of(over))
    same = [pi("a", X0, Y0), pi("p1", X0 + 100, Y0), pi("p2", X0 + 100, Y0), pi("b", X0 + 200, Y0 + 50)]
    no("Aynı yerde iki PI", {"name": "Yan yol", "pis": same}, refusal_of(same))
    back = [pi("a", X0, Y0), pi("p1", X0 + 100, Y0), pi("b", X0 + 50, Y0)]
    no("Geri dönen doğrultu", {"name": "Yan yol", "pis": back}, refusal_of(back))
    starting = [{**sym[0], "radius": 100.0}, sym[1], sym[2]]
    no("Başlangıçta kurp", {"name": "Yan yol", "pis": starting}, refusal_of(starting))
    b_sym = build([], {"pis": sym})
    no("Km eşitliği aksın dışında", {"name": "Yan yol", "pis": sym,
                                    "stationing": {"start": 0.0, "equations": [{"back": 5000.0, "ahead": 6000.0}]}},
       failed("invalid_stationing", f"1. km eşitliğinin geri km'si aksın üstünde, bir önceki eşitlikten sonra değil "
                                    f"(aks {float(b_sym.length):.3f} m).", "stationing.equations[0]"))
    main_b = build(start, MAIN)
    no("Paralel aks kurpun merkezini geçiyor", {"name": "İç kenar", "offsetOf": {"alignment": "g-1", "offset": -450.0}},
       cusp("İç kenar", main_b, -450.0, "offsetOf.offset"))
    no("Paralel aksın kaynağı yok", {"name": "Kenar", "offsetOf": {"alignment": "g-9", "offset": 5.0}},
       rule("kaynak aks projede yok", "offsetOf.alignment"))
    no("Paralel aksa paralel", {"name": "Kenar 2", "offsetOf": {"alignment": "g-2", "offset": 5.0}},
       rule("paralel aks yalnız PI'larıyla kurulan aksa bağlanır", "offsetOf.alignment"), setup([MAIN, LEFT]),
       [MAIN, LEFT])
    no("Öteleme sıfır", {"name": "Kenar", "offsetOf": {"alignment": "g-1", "offset": 0.0}},
       rule("öteleme 0 olmayan, en çok 1000 m bir sayı olmalı", "offsetOf.offset"))
    no("Geçersiz beklenen sürüm", {"name": "Yan yol", "pis": sym, "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"name": "Yan yol", "pis": sym, "expectedRevision": "41"}, CONFLICT)
    return file("road.alignments.create", "Aks oluştur (plan/road.md Y3, ADR 0901)", setup(start), cases)


# ── road.alignments.edit ────────────────────────────────────────────────────────────────────────────────────────────

def edit_cases():
    start = [MAIN, LEFT, LINK]
    cases = []

    def ok(name, input_, after, target, rows, op="execute", warns=(), undo=False, setup_=None, before=start):
        out = written(after, target, rows)
        writes = op == "execute" and after != before
        if op == "validate":
            result = completed(None, warns)
        else:
            result = completed(out, warns)
        expect = {**STEPPED, "road": road_of(after)} if writes else {**UNCHANGED, "road": road_of(before)}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Aksı düzenle", "expect": {"road": road_of(before), "canRedo": True}})
        c = {"name": name, "steps": steps}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    def no(name, input_, result, setup_=None, before=start):
        c = {"name": name, "steps": [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    def put(i, d, base=start):
        out = list(base)
        out[i] = d
        return out

    main_b = build(start, MAIN)
    renamed = {**MAIN, "name": "Gövde", "revision": 2}
    ok("Adı değişir: elemanlar yerinde", {"alignment": "g-1", "operation": "set", "name": "Gövde"}, put(0, renamed),
       "g-1", mapping(main_b, build(start, renamed)))
    moved = {**MAIN, "revision": 2, "pis": [MAIN_PIS[0], {**MAIN_PIS[1], "x": X0 + 650.0}, MAIN_PIS[2]]}
    ok("PI kayar: elemanlar değişir", {"alignment": "Ana aks", "operation": "set", "pis": moved["pis"]}, put(0, moved),
       "g-1", mapping(main_b, build(put(0, moved), moved)))
    wider = {**MAIN, "revision": 2, "pis": [MAIN_PIS[0], {**MAIN_PIS[1], "radius": 300.0}, MAIN_PIS[2]]}
    ok("Yarıçap küçülür, sol kenar izler", {"alignment": "g-1", "operation": "set", "pis": wider["pis"]}, put(0, wider),
       "g-1", mapping(main_b, build(put(0, wider), wider)))
    eq = {**MAIN, "revision": 2, "stationing": {"start": 2000.0, "equations": [{"back": 2200.0, "ahead": 2500.0}]}}
    ok("Km eşitliği eklenir: geometri yerinde", {"alignment": "g-1", "operation": "set", "stationing": eq["stationing"]},
       put(0, eq), "g-1", mapping(main_b, build(put(0, eq), eq)))
    cleared = {k: v for k, v in MAIN.items() if k != "stationing"}
    cleared["revision"] = 2
    ok("Boş km başlangıcı sıfırlar", {"alignment": "g-1", "operation": "set", "stationing": {}}, put(0, cleared), "g-1",
       mapping(main_b, build(put(0, cleared), cleared)))
    off = {**LEFT, "revision": 2, "offsetOf": {"alignment": "g-1", "offset": -10.0}}
    ok("Paralel aksın ötelemesi değişir", {"alignment": "g-2", "operation": "set", "offset": -10.0}, put(1, off),
       "g-2", [])
    same_ = {"alignment": "g-1", "operation": "set", "name": "Ana aks"}
    ok("Aynı değer bir şey değiştirmez", same_, start, "g-1", mapping(main_b, main_b))
    # Reverse: the PIs the other way, the clothoids swapped; the parallel axis changes sides.
    r_pis = [dict(p) for p in reversed(MAIN_PIS)]
    for p in r_pis:
        a_, b_ = p.pop("spiralIn", None), p.pop("spiralOut", None)
        if b_ is not None:
            p["spiralIn"] = b_
        if a_ is not None:
            p["spiralOut"] = a_
    rmain = {**MAIN, "revision": 2, "pis": r_pis}
    rleft = {**LEFT, "revision": 2, "offsetOf": {"alignment": "g-1", "offset": 7.5}}
    rb = build([rmain], rmain)
    rows = [{"element": e.id, "change": "reversed",
             "before": [float(rb.length - e.station - e.length), float(rb.length - e.station)],
             "after": [float(e.station), float(e.station + e.length)]} for e in rb.els]
    ok("Ters çevrilir: sol kenar sağa geçer", {"alignment": "g-1", "operation": "reverse"}, [rmain, rleft, LINK], "g-1",
       rows, undo=True)
    link_eq = {**LINK, "stationing": {"start": 500.0, "equations": [{"back": 550.0, "ahead": 600.0}]}}
    lr_pis = list(reversed(LINK_PIS))
    rlink = {**LINK, "revision": 2, "pis": lr_pis, "stationing": {"start": 500.0}}
    lb = build([rlink], rlink)
    rows = [{"element": e.id, "change": "reversed",
             "before": [float(lb.length - e.station - e.length), float(lb.length - e.station)],
             "after": [float(e.station), float(e.station + e.length)]} for e in lb.els]
    ok("Eşitlikli aks ters çevrilir: eşitlikler gider", {"alignment": "g-3", "operation": "reverse"},
       [MAIN, LEFT, rlink], "g-3", rows,
       warns=[{"code": "equations_dropped", "message": "Aks ters çevrildi: km eşitlikleri kaldırıldı, başlangıç km'si kaldı.",
               "path": "alignment"}], setup_=setup([MAIN, LEFT, link_eq]), before=[MAIN, LEFT, link_eq])
    link_b = build(start, LINK)
    ok("Kaldır", {"alignment": "g-3", "operation": "remove"}, [MAIN, LEFT], None, mapping(link_b, None), undo=True)
    ok("Paralel aks kaldırılır", {"alignment": "Sol kenar", "operation": "remove"}, [MAIN, LINK], None, [])
    ok("Plan yazmaz", {"alignment": "g-1", "operation": "set", "name": "Gövde"}, put(0, renamed), "g-1",
       mapping(main_b, build(start, renamed)), op="plan")
    ok("Doğrulama yazmaz", {"alignment": "g-3", "operation": "remove"}, [MAIN, LEFT], None, [], op="validate")
    no("Yol projesi değil", {"alignment": "g-1", "operation": "remove"}, NOT_ROAD, setup(work="map"), [])
    no("Olmayan aks", {"alignment": "g-9", "operation": "remove"},
       failed("unknown_alignment", "“g-9” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Değiştirilecek bir şey yok", {"alignment": "g-1", "operation": "set"},
       failed("invalid_alignment", "Değiştirilecek bir şey yok: name, pis, stationing ya da offset verin."))
    no("Paralel aksın PI'ı olmaz", {"alignment": "g-2", "operation": "set", "pis": MAIN_PIS},
       failed("invalid_alignment", "Paralel aksın PI'ı olmaz: kaynağını izler; kaynağı düzenleyin.", "pis"))
    no("Öteleme yalnız paralel aksın", {"alignment": "g-1", "operation": "set", "offset": 3.0},
       failed("invalid_alignment", "Öteleme yalnız paralel aksın olur.", "offset"))
    no("İzlenen aks silinmez", {"alignment": "g-1", "operation": "remove"},
       failed("alignment_in_use", "“Ana aks” aksını şu paralel akslar izliyor: g-2. Önce onları silin.", "alignment"))
    no("Paralel aks ters çevrilmez", {"alignment": "g-2", "operation": "reverse"},
       failed("invalid_alignment", "Paralel aks ters çevrilmez: kaynağını izler; kaynağı ters çevirin.", "alignment"))
    tight = {**MAIN, "pis": [MAIN_PIS[0], {"id": "p1", "x": X0 + 600, "y": Y0, "radius": 7.0}, MAIN_PIS[2]]}
    no("Sol kenarı kıvıran yarıçap", {"alignment": "g-1", "operation": "set", "pis": tight["pis"]},
       cusp("Sol kenar", build([tight], tight), -7.5, "pis"))
    over = [MAIN_PIS[0], {**MAIN_PIS[1], "radius": 4000.0, "spiralIn": 0.0, "spiralOut": 0.0}, MAIN_PIS[2]]
    over = [{k: v for k, v in p.items() if v != 0.0 or k in ("x", "y")} for p in over]
    no("Kurp aksa sığmıyor", {"alignment": "g-1", "operation": "set", "pis": over}, refusal_of(over))
    longer = [MAIN_PIS[0], {**MAIN_PIS[1], "spiralIn": 400.0, "spiralOut": 400.0}, MAIN_PIS[2]]
    no("Klotoidler sapmadan uzun", {"alignment": "g-1", "operation": "set", "pis": longer}, refusal_of(longer))
    no("Km eşitliği aksın dışında", {"alignment": "g-1", "operation": "set",
                                    "stationing": {"start": 2000.0, "equations": [{"back": 1000.0, "ahead": 3000.0}]}},
       failed("invalid_stationing", f"1. km eşitliğinin geri km'si aksın üstünde, bir önceki eşitlikten sonra değil "
                                    f"(aks {float(main_b.length):.3f} m).", "stationing.equations[0]"))
    no("Aynı ad", {"alignment": "g-3", "operation": "set", "name": "sol KENAR"},
       rule("bu adda başka bir aks var", "name"))
    no("Geçersiz beklenen sürüm", {"alignment": "g-3", "operation": "remove", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"alignment": "g-3", "operation": "remove", "expectedRevision": "41"}, CONFLICT)
    return file("road.alignments.edit", "Aksı düzenle (plan/road.md Y3, ADR 0901)", setup(start), cases)


# ── road.alignments.query ───────────────────────────────────────────────────────────────────────────────────────────

def query_cases():
    main = {**MAIN, "stationing": EQUATIONS}
    start = [main, LEFT, LINK]
    cases = []
    bm, bl, bk = build(start, main), build(start, LEFT), build(start, LINK)

    def q(name, input_, out, setup_=None, height=None):
        c = {"name": name, "steps": [step("execute", input_, completed(out), {**UNCHANGED, "road": road_of(start)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    def no(name, input_, result, setup_=None, before=start):
        c = {"name": name, "steps": [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    def one(d, height=None, **extra):
        return {"alignments": [report(start, d, height)], "at": extra.get("at", []), "kms": extra.get("kms", []),
                "ticks": extra.get("ticks", []), "points": extra.get("points", [])}

    q("Bütün akslar", {}, {"alignments": [report(start, d) for d in start], "at": [], "kms": [], "ticks": [],
                          "points": []})
    q("Bir aks adıyla, büyük küçük harf ayrımsız", {"alignment": "ana AKS"}, one(main))
    stations = [0.0, 100.0, 450.0, 500.0, float(bm.length)]
    q("Km'lerdeki yerler, bölgeleriyle", {"alignment": "g-1", "stations": stations},
      one(main, at=[place(bm, s) for s in stations]))
    q("Paralel aksın yerleri: kaynağın km'si, ötelenmiş yer ve eğrilik",
      {"alignment": "g-2", "stations": [100.0, 450.0, 500.0]}, one(LEFT, at=[place(bl, s) for s in (100.0, 450.0, 500.0)]))

    def kms(asks):
        out = []
        for a in asks:
            got = ref.stations_of(bm.regions, mp.mpf(a["km"]))
            if "region" in a:
                got = [g for g in got if g[0] == a["region"]]
            out.append({"km": a["km"], "places": [place(bm, s) for _, s in got]})
        return out

    asks = [{"km": 2100.0, "region": 0}, {"km": 2550.0, "region": 1}, {"km": 2550.0, "region": 2}, {"km": 2690.0}]
    q("Km yazısından yer; iki yere düşen km bölgesiyle", {"alignment": "g-1", "kms": asks}, one(main, kms=kms(asks)))
    ticks = []
    for i, r in enumerate(bm.regions):
        ka, kb = r[1], r[1] + r[2] * (r[3] - r[0])
        k = mp.ceil(min(ka, kb) / 100) * 100
        here = []
        while k <= max(ka, kb) + mp.mpf("1e-9"):
            s = r[0] + r[2] * (k - r[1])
            last = i == len(bm.regions) - 1
            if s >= r[0] - mp.mpf("1e-9") and (s < r[3] - mp.mpf("1e-9") or (last and s <= r[3] + mp.mpf("1e-9"))):
                here.append(min(max(s, r[0]), r[3]))
            k += 100
        ticks += [place(bm, s) for s in sorted(here)]
    q("Her 100 m'nin km işareti, eşitliklerle", {"alignment": "g-1", "every": 100.0}, one(main, ticks=ticks))
    pts = [[X0 + 520, Y0 + 30], [X0 + 650, Y0 + 100], [X0 + 700, Y0 - 100], [X0 - 50, Y0 + 10], [X0 + 1200, Y0 + 650]]
    q("Noktaların km ve ötelemesi, uçların ötesi", {"alignment": "g-1", "points": pts},
      one(main, points=[locate(bm, p) for p in pts]))
    lp = [[X0 + 520, Y0 + 30], [X0 + 650, Y0 + 100]]
    q("Paralel aksa göre öteleme", {"alignment": "g-2", "points": lp}, one(LEFT, points=[locate(bl, p) for p in lp]))
    kp = [[X0 - 920, Y0 - 980], [X0 - 950, Y0 - 950], [X0 - 850, Y0 - 1050]]
    q("Kırık noktada eşit iki ayak, küçük km önce; köşenin ayağı",
      {"alignment": "g-3", "points": kp}, one(LINK, points=[locate(bk, p) for p in kp]))
    ground_setup = setup(start, ground=1250.0)
    cases.append({"name": "Zemin boyu: ölçek ve 1250 m yüksekliğin çarpanıyla", "setup": ground_setup,
                  "steps": [{**step("execute", {"alignment": "g-1"},
                                    completed({"alignments": [report(start, main, 1250.0)], "at": [], "kms": [],
                                               "ticks": [], "points": []}),
                                    {**UNCHANGED, "road": road_of(start)}), "within": 1e-4}]})
    cases.append({"name": "Paralel aksın zemin boyu", "setup": ground_setup,
                  "steps": [{**step("execute", {"alignment": "g-2"},
                                    completed({"alignments": [report(start, LEFT, 1250.0)], "at": [], "kms": [],
                                               "ticks": [], "points": []}),
                                    {**UNCHANGED, "road": road_of(start)}), "within": 1e-4}]})
    no("Yol projesi değil", {}, NOT_ROAD, setup(work="map"), [])
    no("Olmayan aks", {"alignment": "g-9"}, failed("unknown_alignment", "“g-9” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Aks verilmeden km sorulmaz", {"stations": [10.0]},
       failed("invalid_places", "Km ve yer bir aksta sorulur: aksın kimliğini ya da adını (alignment) verin.", "alignment"))
    no("Aralık sıfır", {"alignment": "g-1", "every": 0.0},
       failed("invalid_places", "Km'ler ve yerler sonlu sayılar, every 0'dan büyük olmalı.", "stations"))
    no("Aksın dışında km", {"alignment": "g-1", "stations": [10.0, 5000.0]},
       failed("station_outside", f"5000 m aksın üstünde değil (aks {float(bm.length):.3f} m).", "stations[1]"))
    no("Eksi km", {"alignment": "g-1", "stations": [-1.0]},
       failed("station_outside", f"-1 m aksın üstünde değil (aks {float(bm.length):.3f} m).", "stations[0]"))
    no("İki yere düşen km bölgesiz", {"alignment": "g-1", "kms": [{"km": 2550.0}]},
       failed("station_ambiguous", "2+550.000 km'si aksın 2 yerine düşüyor (bölgeler 1, 2); hangisi olduğunu region ile verin.",
              "kms[0]"))
    no("Aksta olmayan km", {"alignment": "g-1", "kms": [{"km": 9000.0}]},
       failed("station_outside", "9+000.000 km'si aksın üstünde değil.", "kms[0]"))
    no("Bölgesinde olmayan km", {"alignment": "g-1", "kms": [{"km": 2100.0, "region": 1}]},
       failed("station_outside", "2+100.000 km'si aksın üstünde değil.", "kms[0]"))
    return file("road.alignments.query", "Aksı sorgula (plan/road.md Y3, ADR 0901)", setup(start), cases)


def main():
    files = {"road.alignments.create.json": create_cases(), "road.alignments.edit.json": edit_cases(),
             "road.alignments.query.json": query_cases()}
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
