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
import road_grades  # noqa: E402

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
            "spiralOut": p.get("spiralOut", 0), "ove": p.get("ove", False)}


MAIN_PIS = [pi("a", X0, Y0), pi("p1", X0 + 600, Y0, 400, 80, 80), pi("b", X0 + 1100, Y0 + 500)]
MAIN = {"id": "g-1", "name": "Ana aks", "revision": 1, "pis": MAIN_PIS, "stationing": {"start": 2000.0}}
LEFT = {"id": "g-2", "name": "Sol kenar", "revision": 1, "offsetOf": {"alignment": "g-1", "offset": -7.5}}
LINK_PIS = [pi("a", X0 - 1000, Y0 - 1000), pi("p1", X0 - 900, Y0 - 1000), pi("b", X0 - 900, Y0 - 900)]
LINK = {"id": "g-3", "name": "Bağlantı", "revision": 1, "pis": LINK_PIS}
EQUATIONS = {"start": 2000.0, "equations": [{"back": 2200.0, "ahead": 2500.0},
                                            {"back": 2650.0, "ahead": 2700.0, "increasing": False}]}


def node(id_, name, children=None, **flags):
    return {"id": id_, "name": name, "type": "group" if children is not None else "layer", "visible": True,
            "locked": False, "expanded": True, "style": dict(STYLE), "children": children or [], **flags}


LAYERS = [node("0", "0"), node("km", "Km"), node("kilitli", "Kilitli", locked=True),
          node("gizli", "Gizli", visible=False), node("grup", "Yol", [node("yol-km", "Yol km")])]


def setup(alignments=None, work="road", ground=None):
    settings = {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
                "plotScale": 1000.0, "workspace": "cad", "cadWork": work, "drawingFont": "barlow"}
    if alignments:
        settings["road"] = {"alignments": copy.deepcopy(alignments)}
    if ground is not None:
        settings["survey"] = {"groundHeight": ground}
    return {"format": "kentos.document", "version": 2, "name": "Yol aksları", "settings": settings,
            "origin": {"x": 0.0, "y": 0.0}, "layers": copy.deepcopy(LAYERS), "activeLayer": "0", "entities": [],
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
    width = 4 + digits if digits else 3
    return f"{sign}{whole}+{rest:0{width}.{digits}f}"


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
    # Docs/adr/0967: where two elements meet, how far they part in place, direction and curvature (G0, G1, G2).
    seams = []
    for e0, e1 in zip(b.els, b.els[1:]):
        end = e0.end()
        seams.append({"station": float(e1.station), "before": e0.id, "after": e1.id,
                      "place": float(mp.hypot(end[0] - e1.start[0], end[1] - e1.start[1])),
                      "direction": float(abs(ref.wrap(e0.theta(e0.length) - e1.theta0))),
                      "curvature": float(abs(e0.curvature(e0.length) - e1.k0))})
    if seams:
        out["seams"] = seams
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
    # Docs/adr/0979: the ove's refusals.
    if k == "BadOve":
        why = {"NoNextCurve": "kendisinin ve sonraki PI'nın kurpu olmalı, sonraki bitiş olamaz",
               "Reverse": "iki kurp aynı yöne dönmeli (ters kurpta ove olmaz)",
               "EqualRadii": "iki kurbun yarıçapı farklı olmalı",
               "SpiralOnSharedSide": "ilk kurbun çıkış ve ikincinin giriş klotoidi olmamalı, ove onların yerine geçer",
               "Chained": "art arda iki ove henüz kurulmuyor"}[e["why"]]
        return failed("invalid_alignment", f"{e['pi']} noktasında ove olamaz: {why}.", at(e["pi"]))
    if k == "OveTooLong":
        return failed("alignment_curves_overlap",
                      f"{e['a']}–{e['b']} arasına ove sığmıyor: en uzun ovede bile (ikinci kurbun yayı biter) ortadaki "
                      f"aliyman {e['excess']:.3f} m uzun kalıyor. PI'ları yaklaştırın ya da yarıçapları değiştirin.",
                      at(e["b"]))
    if k == "OveAmbiguous":
        return failed("invalid_alignment",
                      f"{e['a']}–{e['b']} arasında birden çok ove var (A = "
                      f"{', '.join(f'{v:.3f}' for v in e['parameters'])}); hangisi istendiği PI'lar oynatılarak "
                      "seçilmeli.", at(e["b"]))
    raise ValueError(k)


def ove_pis(*args, **kw):
    """The reference's PIs of a known ove (road_alignment_cases.ove_pis), as a command writes them."""
    out = []
    for p in ref.ove_pis(*args, **kw):
        q = {"id": p["id"], "x": float(p["x"]), "y": float(p["y"])}
        for k in ("radius", "spiralIn", "spiralOut"):
            if p[k]:
                q[k] = float(p[k])
        if p.get("ove"):
            q["ove"] = True
        out.append(q)
    return out


def refusal_of(pis):
    try:
        ref.build([full(p) for p in pis])
    except ref.Refused as r:
        return chain_message(r.kw, pis)
    raise AssertionError("builds")


def offset_place(els, s, d):
    e, (x, y) = ref.place_at(els, s)
    th = e.theta(s - e.station)
    return x + d * mp.sin(th), y - d * mp.cos(th)


def first_crossing(b, d):
    """Docs/adr/0970: the first pair of stations a < b whose offset places are one point, or none. The offset is
    sampled every 0.25 m by float Simpson steps, its segments' crossings found by shapely's STRtree, each refined by
    mpmath's findroot at 40 digits; the least (a, b) is kept."""
    from shapely import STRtree
    from shapely.geometry import LineString

    d = mp.mpf(d)
    st, pts = [], []
    for e in b.els:
        n = max(2, math.ceil(float(e.length) / 0.25))
        th0, k0, c = float(e.theta0), float(e.k0), float(e.rate())
        th = lambda t: th0 + k0 * t + c * t * t / 2
        x, y, h = float(e.start[0]), float(e.start[1]), float(e.length) / n
        for i in range(n):
            t = h * i
            st.append(float(e.station) + t)
            pts.append((x + float(d) * math.sin(th(t)), y - float(d) * math.cos(th(t))))
            m, u = t + h / 2, t + h
            x += h / 6 * (math.cos(th(t)) + 4 * math.cos(th(m)) + math.cos(th(u)))
            y += h / 6 * (math.sin(th(t)) + 4 * math.sin(th(m)) + math.sin(th(u)))
    end = float(b.length)
    st.append(end)
    pts.append(tuple(float(v) for v in offset_place(b.els, mp.mpf(end), d)))
    segs = [LineString([pts[i], pts[i + 1]]) for i in range(len(pts) - 1)]
    tree = STRtree(segs)
    best = None
    for i, g in enumerate(segs):
        for j in tree.query(g, predicate="intersects"):
            j = int(j)
            if j <= i + 1:
                continue
            q = g.intersection(segs[j])
            if q.geom_type != "Point":
                continue
            ga = st[i] + g.project(q) / g.length * (st[i + 1] - st[i])
            gb = st[j] + segs[j].project(q) / segs[j].length * (st[j + 1] - st[j])
            f = lambda a_, b_: [offset_place(b.els, a_, d)[0] - offset_place(b.els, b_, d)[0],
                                offset_place(b.els, a_, d)[1] - offset_place(b.els, b_, d)[1]]
            ra, rb = mp.findroot(f, (mp.mpf(ga), mp.mpf(gb)))
            ra, rb = min(ra, rb), max(ra, rb)
            if rb - ra > mp.mpf("1e-6") and (best is None or (ra, rb) < best):
                best = (ra, rb)
    return best


def kink_between(b, a, z):
    """The PI of a kink (a direction jump) between two stations, by the later element's id."""
    for e0, e1 in zip(b.els, b.els[1:]):
        if abs(ref.wrap(e1.theta0 - e0.theta(e0.length))) > mp.mpf("1e-9") and a < e1.station < z:
            return e1.id.split("/")[0]
    return None


def cusp(name, b, d, path):
    """Docs/adr/0970: the cusp's stations, the element's tightest radius on the offset's side and the fix in numbers;
    else the first crossing, at a kink or not."""
    dd = abs(d)
    k = lambda st: km_text(ref.km_of(b.regions, st))
    for e in b.els:
        if e.kind == "tangent":
            continue
        fa, fb = 1 + e.k0 * d <= 0, 1 + e.k1 * d <= 0
        if fa or fb:
            rate = e.rate()
            sf = min(max((-1 / mp.mpf(d) - e.k0) / rate if rate else 0, 0), e.length)
            frm, to = (0, e.length) if fa and fb else ((0, sf) if fa else (sf, e.length))
            radius = 1 / max(abs(k) for k, f in ((e.k0, fa), (e.k1, fb)) if f)
            p = e.id.split("/")[0]
            return failed("offset_cusp", f"“{name}” paralel aksı km {k(e.station + frm)}–"
                                         f"{k(e.station + to)} arasında kıvrılıyor: {p} kurbunun yarıçapı "
                                         f"orada {float(radius):.3f} m'ye iniyor, öteleme {dd:.3f} m. {p} kurbunun "
                                         f"yarıçapını {dd:.3f} m'nin üstüne çıkarın ya da ötelemeyi "
                                         f"{float(radius):.3f} m'nin altına indirin.", path)
    x = first_crossing(b, d)
    assert x is not None, "no cusp, no crossing"
    a, z = x
    p = kink_between(b, a, z)
    if p is not None:
        return failed("offset_cusp", f"“{name}” paralel aksı km {k(a)} ile km {k(z)} arasında kendini "
                                     f"kesiyor: {p} kurpsuz bir kırık ve iç yanında her öteleme kendini keser. {p} "
                                     f"noktasına yarıçapı {dd:.3f} m'nin üstünde bir kurp koyun.", path)
    return failed("offset_cusp", f"“{name}” paralel aksının km {k(a)} ve km {k(z)} noktaları aynı yere "
                                 f"düşüyor: aksın bu iki yeri {dd:.3f} m'lik ötelemeye göre birbirine çok yakın. "
                                 "Ötelemeyi küçültün ya da aksın bu iki yerini birbirinden uzaklaştırın.", path)


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
    # Docs/adr/0979: an ove between two curves turning the same way, its A solved on the PIs.
    egg = ove_pis(500, 200, 250, 0.6, 1.0, at=(X0, Y0 + 3000))
    ok("Ove: R 500'den R 200'e", {"name": "Oveli", "pis": egg}, {"id": "g-2", "name": "Oveli", "revision": 1, "pis": egg})
    egg2 = ove_pis(500, 200, 250, 0.6, 1.1, 80, 60, at=(X0, Y0 + 4000), right=True, opening=True)
    ok("Ove: açılan, sağa, klotoidli", {"name": "Açılan ove", "pis": egg2},
       {"id": "g-2", "name": "Açılan ove", "revision": 1, "pis": egg2})
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
    far = ove_pis(500, 200, 250, 0.6, 1.0, at=(X0, Y0 + 9000))
    far = [far[0], far[1], {**far[2], "x": far[2]["x"] + 400}, {**far[3], "x": far[3]["x"] + 400}]
    no("Ove sığmıyor: ortadaki aliyman uzun", {"name": "Yan yol", "pis": far}, refusal_of(far))
    twice = ove_pis(1000, 900, 120, 0.3, 0.3, at=(X0, Y0 + 9500))
    no("İki ove çözümü", {"name": "Yan yol", "pis": twice}, refusal_of(twice))
    rov = [{**p, **({"ove": True} if p["id"] == "p1" else {})} for p in rev]
    no("Ters kurpta ove olmaz", {"name": "Yan yol", "pis": rov}, refusal_of(rov))
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
    # Docs/adr/0970: a hairpin whose arms close in, R 60 over the 50 m offset (no cusp): its inner offset's arms
    # cross between the curves.
    hair = {"id": "g-2", "name": "Dönüş", "revision": 1,
            "pis": [pi("a", X0, Y0), pi("p1", X0 + 300, Y0, 60), pi("p2", X0 + 300, Y0 + 160, 60), pi("b", X0, Y0 + 60)]}
    no("Kolları yaklaşan dönüşün iç kenarı kendini keser", {"name": "İç kenar", "offsetOf": {"alignment": "g-2",
                                                                                            "offset": -50.0}},
       cusp("İç kenar", build([hair], hair), -50.0, "offsetOf.offset"), setup([MAIN, hair]), [MAIN, hair])
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
    # Docs/adr/0967: without clothoids the circular curve meets its tangents with a curvature jump of 1/R (G2 broken,
    # G0 and G1 kept). A PI without a curve is a kink, a direction jump of its deflection: g-3's p1 in the queries.
    bare = {**MAIN, "revision": 2, "pis": [MAIN_PIS[0], {k: v for k, v in MAIN_PIS[1].items() if k not in ("spiralIn", "spiralOut")},
                                         MAIN_PIS[2]]}
    ok("Klotoidler kaldırılır: eğrilik 1/R sıçrar", {"alignment": "g-1", "operation": "set", "pis": bare["pis"]},
       put(0, bare), "g-1", mapping(main_b, build(put(0, bare), bare)))
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
    # Docs/adr/0964 (ROAD-DEVER-03): the superelevation schedules on the alignment turn round with it, one step. A
    # station s becomes L − s and the left side the old right; each side's slope is written outward, so it moves to the
    # other column unchanged. A rule on a curve stays: checked here against the superelevation reference, the reversed
    # alignment with the swapped normals gives every physical edge the same slope at the same place.
    length = float(rb.length)
    d_rows = {"id": "d-1", "name": "Satırlı dever", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
              "normalLeft": -2.5, "normalRight": -3.0, "pivot": "leftEdge",
              "method": {"method": "stations", "rows": [{"station": 100.0, "left": -2.5, "right": -3.0},
                                                        {"station": 400.0, "left": 4.0, "right": -4.0},
                                                        {"station": 900.0, "left": 6.0, "right": -6.0}]}}
    d_curve = {"id": "d-2", "name": "Kurplu dever", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
               "normalLeft": -2.5, "normalRight": -3.0, "pivot": "insideEdge",
               "method": {"method": "curves", "curves": [{"pi": "p1", "rate": 6.0, "runoff": 100.0, "onTangent": 0.6,
                                                          "widening": 0.8}]}}
    d_link = {"id": "d-3", "name": "Bağlantı deveri", "alignment": "g-3", "alignmentRevision": 1, "revision": 1,
              "normalLeft": -2.0, "normalRight": -2.0,
              "method": {"method": "stations", "rows": [{"station": 10.0, "left": -2.0, "right": -2.0}]}}

    def turned(d):
        out = {**d, "normalLeft": d["normalRight"], "normalRight": d["normalLeft"], "alignmentRevision": 2,
               "revision": d["revision"] + 1}
        if "pivot" in d:
            out["pivot"] = {"leftEdge": "rightEdge", "rightEdge": "leftEdge"}.get(d["pivot"], d["pivot"])
        if d["method"]["method"] == "stations":
            out["method"] = {"method": "stations", "rows": [
                {"station": length - r["station"], "left": r["right"], "right": r["left"]}
                for r in reversed(d["method"]["rows"])]}
        return out

    import road_superelevation_cases as supref
    from fractions import Fraction as Fr
    curve_rules = d_curve["method"]["curves"]
    was = supref.curves([full(p) for p in MAIN_PIS], [-2.5, -3.0], curve_rules)["schedule"]
    now = supref.curves([full(p) for p in rmain["pis"]], [-3.0, -2.5], curve_rules)["schedule"]
    for k in range(401):
        s_ = Fr(k, 400) * Fr(str(length))
        for ka, kr, n in [("left", "right", -2.5), ("right", "left", -3.0), ("leftWidening", "rightWidening", 0),
                          ("rightWidening", "leftWidening", 0)]:
            va = supref.along(was[ka], s_) if was[ka] else n
            vr = supref.along(now[kr], Fr(str(length)) - s_) if now[kr] else n
            assert abs(float(va) - float(vr)) < 1e-9, ("a curve's rule does not hold when reversed", float(s_), ka)
    supers = [d_rows, d_curve, d_link]
    supers_after = [turned(d_rows), turned(d_curve), d_link]
    with_supers = setup(start)
    with_supers["settings"]["road"]["superelevations"] = copy.deepcopy(supers)
    out = written([rmain, rleft, LINK], "g-1", rows)
    out["superelevations"] = ["d-1", "d-2"]
    road_before = {**road_of(start), "superelevations": supers}
    road_after = {**road_of([rmain, rleft, LINK]), "superelevations": supers_after}
    cases.append({"name": "Ters çevrilir: dever çizelgeleri de döner", "setup": with_supers, "steps": [
        step("execute", {"alignment": "g-1", "operation": "reverse"}, completed(out), {**STEPPED, "road": road_after}),
        {"op": "undo", "returns": "Aksı düzenle", "expect": {"road": road_before, "canRedo": True}}]})
    plan_out = dict(out)
    cases.append({"name": "Ters çevirmenin planı devere de bakar", "setup": with_supers, "steps": [
        step("plan", {"alignment": "g-1", "operation": "reverse"}, completed(plan_out),
             {**UNCHANGED, "road": road_before})]})
    # Docs/adr/0965: its profiles turn round with it too. A PVI at s goes to L − s with its elevation, in the reversed
    # order; a parabola and a circle are symmetric about their PVI, an asymmetric parabola's arms swap; a crossing goes
    # to L − s. Checked against the vertical reference: the grade line at L − s is the old one at s.
    import road_vertical_cases as vref

    def pv(id_, s_, z, **c):
        return {"id": id_, "station": float(s_), "elevation": float(z), **{k: float(v) for k, v in c.items()}}

    prof = {"id": "b-1", "name": "Proje kotu", "alignment": "g-1", "alignmentRevision": 1, "revision": 1,
            "pvis": [pv("a", 0, 100), pv("v1", 250, 108, length=120), pv("v2", 560, 101, lengthIn=60, lengthOut=90),
                     pv("v3", 900, 112, radius=3000), pv("b", length, 106)],
            "crossings": [{"name": "Köprü", "station": 300.0, "side": "above", "elevation": 120.0, "clearance": 5.5}]}

    def turned_profile(d):
        out = {**d, "alignmentRevision": 2, "revision": d["revision"] + 1, "pvis": [], "crossings": [
            {**c, "station": length - c["station"]} for c in d["crossings"]]}
        for p_ in reversed(d["pvis"]):
            q = {**p_, "station": length - p_["station"]}
            if "lengthIn" in p_:
                q["lengthIn"], q["lengthOut"] = p_["lengthOut"], p_["lengthIn"]
            out["pvis"].append(q)
        return out

    def to_v(pvis):
        out = []
        for p_ in pvis:
            c = None
            if p_.get("radius"):
                c = vref.circ(p_["radius"])
            elif p_.get("length"):
                c = vref.par(p_["length"])
            elif p_.get("lengthIn"):
                c = vref.asym(p_["lengthIn"], p_["lengthOut"])
            out.append(vref.pvi(p_["id"], p_["station"], p_["elevation"], c))
        return out

    turned_prof = turned_profile(prof)
    z_was = vref.solve(to_v(prof["pvis"]))[1]
    z_now = vref.solve(to_v(turned_prof["pvis"]))[1]
    for k in range(401):
        s_ = length * k / 400
        za, zb = z_was(s_), z_now(length - s_)
        assert za is not None and zb is not None and abs(za[0] - zb[0]) < 1e-9, ("the grade line moved", s_)
    with_profile = setup(start)
    with_profile["settings"]["road"]["superelevations"] = copy.deepcopy(supers)
    with_profile["settings"]["road"]["profiles"] = [copy.deepcopy(prof)]
    out_p = written([rmain, rleft, LINK], "g-1", rows)
    out_p["superelevations"] = ["d-1", "d-2"]
    out_p["profiles"] = ["b-1"]
    cases.append({"name": "Ters çevrilir: boykesit de döner", "setup": with_profile, "steps": [
        step("execute", {"alignment": "g-1", "operation": "reverse"}, completed(out_p),
             {**STEPPED, "road": {**road_after, "profiles": [turned_prof]}}),
        {"op": "undo", "returns": "Aksı düzenle",
         "expect": {"road": {**road_before, "profiles": [prof]}, "canRedo": True}}]})
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
    # A profile lies on the alignment (docs/adr/0902 §6): it goes first.
    prof = {"id": "b-1", "name": "Bağlantı kotu", "alignment": "g-3", "alignmentRevision": 1, "revision": 1,
            "pvis": [{"id": "a", "station": 0.0, "elevation": 100.0}, {"id": "b", "station": 150.0, "elevation": 103.0}]}
    with_profile = setup(start)
    with_profile["settings"]["road"]["profiles"] = [prof]
    cases.append({"name": "Boykesiti olan aks silinmez", "setup": with_profile, "steps": [step(
        "execute", {"alignment": "g-3", "operation": "remove"},
        failed("alignment_in_use", "“Bağlantı” aksına şu boykesitler bağlı: Bağlantı kotu. Önce onları silin.", "alignment"),
        {**UNCHANGED, "road": {"alignments": copy.deepcopy(start), "profiles": [prof]}})]})
    # A sample line set lies on the alignment (docs/adr/0903 §6): it goes first.
    kset = {"id": "k-1", "name": "Bağlantı kesitleri", "alignment": "g-3", "alignmentRevision": 1, "revision": 1,
            "left": 10.0, "right": 10.0, "stations": {"method": "stations", "stations": [50.0]}}
    with_set = setup(start)
    with_set["settings"]["road"]["sections"] = [kset]
    cases.append({"name": "Kesit takımı olan aks silinmez", "setup": with_set, "steps": [step(
        "execute", {"alignment": "g-3", "operation": "remove"},
        failed("alignment_in_use", "“Bağlantı” aksına şu kesit takımları bağlı: Bağlantı kesitleri. Önce onları silin.",
               "alignment"),
        {**UNCHANGED, "road": {"alignments": copy.deepcopy(start), "sections": [kset]}})]})
    # A superelevation schedule lies on the alignment (docs/adr/0904 §5): it goes first.
    dsup = {"id": "d-1", "name": "Bağlantı deveri", "alignment": "g-3", "alignmentRevision": 1, "revision": 1,
            "normalLeft": -2.5, "normalRight": -2.5,
            "method": {"method": "stations", "rows": [{"station": 10.0, "left": -2.5, "right": -2.5}]}}
    with_super = setup(start)
    with_super["settings"]["road"]["superelevations"] = [dsup]
    cases.append({"name": "Dever çizelgesi olan aks silinmez", "setup": with_super, "steps": [step(
        "execute", {"alignment": "g-3", "operation": "remove"},
        failed("alignment_in_use", "“Bağlantı” aksına şu dever çizelgeleri bağlı: Bağlantı deveri. Önce onları silin.",
               "alignment"),
        {**UNCHANGED, "road": {"alignments": copy.deepcopy(start), "superelevations": [dsup]}})]})
    no("Paralel aks ters çevrilmez", {"alignment": "g-2", "operation": "reverse"},
       failed("invalid_alignment", "Paralel aks ters çevrilmez: kaynağını izler; kaynağı ters çevirin.", "alignment"))
    tight = {**MAIN, "pis": [MAIN_PIS[0], {"id": "p1", "x": X0 + 600, "y": Y0, "radius": 7.0}, MAIN_PIS[2]]}
    no("Sol kenarı kıvıran yarıçap", {"alignment": "g-1", "operation": "set", "pis": tight["pis"]},
       cusp("Sol kenar", build([tight], tight), -7.5, "pis"))
    # Docs/adr/0970: p1 without its curve is a kink; Sol kenar (7.5 m on the inner side) crosses itself
    # 7.5·tan(δ/2) before and after it, δ = 45°: the textbook's answer, checked against the found one.
    kinked = {**MAIN, "pis": [MAIN_PIS[0], {"id": "p1", "x": X0 + 600, "y": Y0}, MAIN_PIS[2]]}
    kb = build([kinked], kinked)
    ka, kz = first_crossing(kb, -7.5)
    reach = mp.mpf("7.5") * mp.tan(mp.pi / 8)
    assert abs(ka - (600 - reach)) < mp.mpf("1e-9") and abs(kz - (600 + reach)) < mp.mpf("1e-9"), (ka, kz)
    no("Kurp kaldırılır: sol kenar kırıkta kendini keser", {"alignment": "g-1", "operation": "set",
                                                           "pis": kinked["pis"]},
       cusp("Sol kenar", kb, -7.5, "pis"))
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


# ── road.alignments.label ───────────────────────────────────────────────────────────────────────────────────────────

def label_rows(b, every, major, curves):
    near = lambda a, z: abs(a - z) <= mp.mpf("1e-6")
    rows = [{"kind": "start", "station": 0.0, "km": float(ref.km_of(b.regions, 0)),
             "text": km_text(ref.km_of(b.regions, 0), 3)}]
    for i, r in enumerate(b.regions):
        a, z = r[1], r[1] + r[2] * (r[3] - r[0])
        lo, hi = min(a, z), max(a, z)
        n = mp.ceil(lo / every)
        here = []
        while n * every <= hi + mp.mpf("1e-9"):
            k = n * every
            st = r[0] + r[2] * (k - r[1])
            last = i == len(b.regions) - 1
            if st >= r[0] - mp.mpf("1e-9") and (st < r[3] - mp.mpf("1e-9") or (last and st <= r[3] + mp.mpf("1e-9"))):
                here.append((min(max(st, r[0]), r[3]), k))
            n += 1
        for st, k in sorted(here):
            if near(st, 0) or near(st, b.length):
                continue
            major_ = abs(k / major - mp.nint(k / major)) <= mp.mpf("1e-9")
            row = {"kind": "major" if major_ else "tick", "station": float(st), "km": float(k)}
            if major_:
                row["text"] = km_text(k, 0)
            rows.append(row)
    if curves:
        for c in b.reports:
            names = ["TS", "SC", "CS", "ST"] if c["spiralIn"] or c["spiralOut"] else ["TC", "TC", "CT", "CT"]
            seen = []
            for name, st in zip(names, c["stations"]):
                if any(abs(t - st) <= 1e-6 for t in seen):
                    continue
                seen.append(st)
                km = ref.km_of(b.regions, mp.mpf(st))
                rows.append({"kind": name, "station": st, "km": float(km), "text": f"{name} {km_text(km, 3)}"})
    end_km = ref.km_of(b.regions, b.length)
    rows.append({"kind": "end", "station": float(b.length), "km": float(end_km), "text": km_text(end_km, 3)})
    rows.sort(key=lambda r: r["station"])
    return rows


def label_cases():
    main = {**MAIN, "stationing": EQUATIONS}
    start = [main, LEFT, LINK]
    cases = []
    bm, bl, bk = build(start, main), build(start, LEFT), build(start, LINK)

    def ok(name, input_, b, alignment, every=20, major=100, curves=True, op="execute", warns=(), undo=False):
        rows = label_rows(b, mp.mpf(every), mp.mpf(major), curves)
        n = len(rows) + sum(1 for r in rows if "text" in r)
        out = {"alignment": alignment, "labels": rows, "uids": "$any" if op == "execute" else [], "revision": "$current"}
        if op == "validate":
            result, expect = completed(None, warns), {**UNCHANGED, "road": road_of(start)}
        elif op == "plan":
            result, expect = completed(out, warns), {**UNCHANGED, "road": road_of(start)}
        else:
            result = completed(out, warns)
            expect = {**STEPPED, "road": road_of(start), "ids": list(range(1, n + 1))}
        steps = [step(op, input_, result, expect)]
        if undo:
            steps.append({"op": "undo", "returns": "Aks km yazısı", "expect": {"ids": [], "canRedo": True}})
        cases.append({"name": name, "steps": steps})

    def no(name, input_, result, setup_=None, before=start):
        c = {"name": name, "steps": [step("execute", input_, result, {**UNCHANGED, "road": road_of(before)})]}
        if setup_ is not None:
            c["setup"] = setup_
        cases.append(c)

    ok("Her 20 m'de işaret, her 100 m'de km; eşitlikler ve kurp noktaları", {"alignment": "g-1", "layerId": "km"}, bm, "g-1")
    ok("Her 25 m'de işaret", {"alignment": "g-1", "layerId": "km", "every": 25.0}, bm, "g-1", every=25)
    ok("Her 50 m'de km yazısı", {"alignment": "g-1", "layerId": "km", "every": 50.0, "major": 50.0}, bm, "g-1", 50, 50)
    ok("Her 500 m'de km, 100 m'de işaret", {"alignment": "g-1", "layerId": "km", "every": 100.0, "major": 500.0}, bm,
       "g-1", 100, 500)
    ok("Kurp noktaları yazılmadan", {"alignment": "g-1", "layerId": "km", "curves": False}, bm, "g-1", curves=False)
    ok("Büyük yazıyla", {"alignment": "g-1", "layerId": "km", "height": 3.5}, bm, "g-1")
    ok("Adıyla, büyük küçük harf ayrımsız", {"alignment": "ANA aks", "layerId": "km"}, bm, "g-1")
    ok("Paralel aksın km'si kaynağınınki", {"alignment": "g-2", "layerId": "km"}, bl, "g-2")
    ok("Kırık noktalı aks, km 0'dan", {"alignment": "g-3", "layerId": "km", "every": 10.0, "major": 50.0}, bk, "g-3", 10, 50)
    ok("Grubun içindeki katmana", {"alignment": "g-3", "layerId": "yol-km"}, bk, "g-3")
    ok("Gizli katmana uyarıyla", {"alignment": "g-3", "layerId": "gizli"}, bk, "g-3",
       warns=[{"code": "layer_hidden", "message": "“Gizli” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}])
    ok("Plan yazmaz", {"alignment": "g-1", "layerId": "km"}, bm, "g-1", op="plan")
    ok("Doğrulama yazmaz", {"alignment": "g-1", "layerId": "km"}, bm, "g-1", op="validate")
    ok("Geri alınır", {"alignment": "g-3", "layerId": "km"}, bk, "g-3", undo=True)
    no("Yol projesi değil", {"alignment": "g-1", "layerId": "km"}, NOT_ROAD, setup(work="map"), [])
    no("Olmayan aks", {"alignment": "g-9", "layerId": "km"},
       failed("unknown_alignment", "“g-9” kimlikli ya da adlı aks projede yok.", "alignment"))
    no("Aralık sıfır", {"alignment": "g-1", "layerId": "km", "every": 0.0},
       failed("invalid_label", "İşaret aralığı (every) 0'dan büyük olmalı.", "every"))
    no("Eksi aralık", {"alignment": "g-1", "layerId": "km", "every": -5.0},
       failed("invalid_label", "İşaret aralığı (every) 0'dan büyük olmalı.", "every"))
    no("Km aralığı işaretin katı değil", {"alignment": "g-1", "layerId": "km", "major": 30.0},
       failed("invalid_label", "Km yazılan aralık (major) işaret aralığının tam katı olmalı.", "major"))
    no("Km aralığı işaretten küçük", {"alignment": "g-1", "layerId": "km", "major": 10.0},
       failed("invalid_label", "Km yazılan aralık (major) işaret aralığının tam katı olmalı.", "major"))
    no("Yazı yüksekliği sıfır", {"alignment": "g-1", "layerId": "km", "height": 0.0},
       failed("invalid_label", "Yazı yüksekliği (height) 0'dan büyük olmalı.", "height"))
    no("Çok sık işaret", {"alignment": "g-1", "layerId": "km", "every": 0.001, "major": 0.001},
       failed("invalid_label", "Bu aralıkla 100000'den çok işaret çıkar; every'yi büyütün.", "every"))
    no("Olmayan katman", {"alignment": "g-1", "layerId": "yok"},
       failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"))
    no("Katman grubu", {"alignment": "g-1", "layerId": "grup"},
       failed("not_a_layer", "“Yol” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"))
    no("Kilitli katman", {"alignment": "g-1", "layerId": "kilitli"},
       failed("layer_locked", "“Kilitli” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"))
    no("Geçersiz beklenen sürüm", {"alignment": "g-1", "layerId": "km", "expectedRevision": "on"}, BAD_REVISION)
    no("Eski sürüme göre hazırlanmış", {"alignment": "g-1", "layerId": "km", "expectedRevision": "41"}, CONFLICT)
    return file("road.alignments.label", "Aks km yazısı (plan/road.md Y3, ADR 0901)", setup(start), cases)


def main():
    files = {"road.alignments.create.json": create_cases(), "road.alignments.edit.json": edit_cases(),
             "road.alignments.query.json": query_cases(), "road.alignments.label.json": label_cases()}
    check = "--check" in sys.argv[1:]
    bad = False
    for name, data in files.items():
        data = road_grades.fixture(data)
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
