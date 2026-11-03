#!/usr/bin/env python3
"""The shared cases of `road.terrains.quality` (plan/road.md Y18.4, docs/adr/0916 §4; a surface's survey quality).

The drawing is road_terrain_compare_cases.py's with two more surfaces and check points of this script:

- a sparse lattice (columns 0, 1, 2, 10, 11, 12 apart): the wide squares' triangles have an edge over any limit
  between √2 and 8, whichever diagonal they take (both triangles of a square hold its diagonal);
- a spiky lattice: one corner raised, one lowered, a second shot 0.038 m off a corner;
- check points over the terrace (each square planar: the surface's height is the lattice's function), two classes and
  one off the surface; a point with no height and a line for the refusals.

The rules are the ADR's: a near pair is two corners closer in plan than the limit (each pair once, the smaller place
first, sorted by the two places); a spike a corner more than the limit off the mean of the other corners within the
radius (their heights summed from the lowest), the largest off first, then by place; a gap a triangle with an edge over the
limit; a check point's difference is its height less the surface's, a class's statistics the mean, the standard
deviation (n − 1), the RMSE, the least and the largest over its points on the surface. No KentOS code.

    python3 scripts/fixtures/road_terrain_quality_cases.py [--check]
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import road_drainage_cases as drainage  # noqa: E402
import road_terrain_analysis_cases as analysis  # noqa: E402
import road_terrain_compare_cases as compare  # noqa: E402

OUT = Path(__file__).resolve().parents[2] / "fixtures/commands/v1"
W = 1e-6
ANY = "$any"
UID = "0192f600-7c3e-7d4a-9b1e-{:012d}"
UNCHANGED = analysis.UNCHANGED
SX, KX = 10000.0, 11000.0


def sparse_z(x, y):
    return 70.0 + 0.1 * (x - SX) + 0.05 * y


def spiky_z(x, y):
    xr = x - KX
    base = 60.0 + 0.02 * xr
    return base + (2.0 if (xr, y) == (3.0, 3.0) else -1.5 if (xr, y) == (5.0, 1.0) else 0.0)


SPARSE = [(SX + x, float(y), sparse_z(SX + x, float(y))) for y in range(4) for x in (0, 1, 2, 10, 11, 12)]
SPIKY = [(KX + x, float(y), spiky_z(KX + x, float(y))) for y in range(7) for x in range(7)] + [(KX + 1.03, 1.024, 60.4)]
TX = analysis.TX
CHECKS = [  # (x, y, z offset from the terrace, class)
    (TX + 0.5, 0.5, 0.03, "sert"), (TX + 2.3, 1.7, -0.02, "sert"), (TX + 5.5, 2.2, 0.01, "sert"),
    (TX + 8.4, 6.6, 0.05, "sert"), (TX + 1.1, 8.8, 0.12, "bitkili"), (TX + 3.7, 4.4, 0.20, "bitkili"),
    (TX + 6.2, 9.1, -0.06, "bitkili"), (TX + 9.6, 3.3, 0.15, "bitkili"), (TX + 4.9, 7.25, 0.0, None),
    (TX + 20.0, 5.0, 0.0, "sert")]
UIDS = {}


def setup(work="road"):
    s = compare.setup(work)
    base = {"type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    for lid, name in (("seyrek", "Seyrek"), ("sivri", "Sivri"), ("kontrol", "Kontrol"), ("kotsuz", "Kotsuz")):
        s["layers"].append({"id": lid, "name": name, **copy.deepcopy(base)})
    k = max(e["id"] for e in s["entities"])

    def add(e, name=None):
        nonlocal k
        k += 1
        s["entities"].append({**e, "id": k})
        s["uids"].append(UID.format(k))
        if name:
            UIDS[name] = UID.format(k)
        return UID.format(k)

    for layer, pts in (("seyrek", SPARSE), ("sivri", SPIKY)):
        for x, y, z in pts:
            add({"kind": "point", "layerId": layer, "attrs": {}, "p": {"x": x, "y": y}, "z": z})
    for n, (x, y, dz, cls) in enumerate(CHECKS):
        attrs = {"Sınıf": cls} if cls else {}
        add({"kind": "point", "layerId": "kontrol", "attrs": attrs, "p": {"x": x, "y": y},
             "z": analysis.terrace_z(x, y) + dz}, f"k{n}")
    add({"kind": "point", "layerId": "kotsuz", "attrs": {}, "p": {"x": TX + 1.0, "y": 1.0}}, "kotsuz")
    add({"kind": "line", "layerId": "kotsuz", "attrs": {}, "a": {"x": TX, "y": 0.0}, "b": {"x": TX + 1.0, "y": 1.0}},
        "cizgi")
    if work == "road":
        s["settings"]["road"]["surfaces"] += [{"id": "t-10", "name": "Seyrek", "points": {"layers": ["seyrek"]}},
                                              {"id": "t-11", "name": "Sivri", "points": {"layers": ["sivri"]}}]
    return s


# The surfaces' corners in their records' order, and their lattices for the squares.
CORNERS = {"t-10": SPARSE, "t-11": SPIKY,
           "t-6": [(x, y, analysis.terrace_z(x, y)) for y in analysis.EXTRA[0][3] for x in analysis.EXTRA[0][2]],
           "t-5": [(x, y, drainage.plane_z(x, y)) for y in drainage.LATTICES[2][3] for x in drainage.LATTICES[2][2]]}
NAMES = {**compare.NAMES, "t-10": "Seyrek", "t-11": "Sivri"}
LATTICE_TRIANGLES = {"t-10": 2 * 5 * 3, "t-11": None, "t-6": 200, "t-5": 200}


def near_pairs(sid, near):
    v = CORNERS[sid]
    out = []
    for i, p in enumerate(v):
        for j, q in enumerate(v):
            if i != j and tuple(p) < tuple(q) and math.hypot(q[0] - p[0], q[1] - p[1]) < near:
                out.append({"a": list(p), "b": list(q), "distance": math.hypot(q[0] - p[0], q[1] - p[1]),
                            "dz": q[2] - p[2]})
    out.sort(key=lambda r: (r["a"][0], r["a"][1], r["b"][0], r["b"][1]))
    return out


def spikes(sid, radius, limit):
    v = CORNERS[sid]
    out = []
    for i, p in enumerate(v):
        zs = [q[2] for j, q in enumerate(v) if j != i and math.hypot(q[0] - p[0], q[1] - p[1]) <= radius]
        if not zs:
            continue
        total = 0.0  # a plain left fold: Python's sum() compensates since 3.12
        for z in sorted(zs):
            total += z
        mean = total / len(zs)
        d = p[2] - mean
        if abs(d) > limit:
            out.append({"at": list(p), "mean": mean, "difference": d, "neighbours": len(zs)})
    out.sort(key=lambda r: (-abs(r["difference"]), r["at"][0], r["at"][1]))
    return out


def gaps(sid, limit):
    xs = sorted({p[0] for p in CORNERS[sid]})
    ys = sorted({p[1] for p in CORNERS[sid]})
    tri, area, longest = 0, 0.0, 0.0
    for j in range(len(ys) - 1):
        for i in range(len(xs) - 1):
            dx, dy = xs[i + 1] - xs[i], ys[j + 1] - ys[j]
            diag = math.hypot(dx, dy)
            longest = max(longest, diag)
            if diag > limit:
                tri += 2
                area += dx * dy
    return {"triangles": tri, "area": area, "longest": longest}


def stats(cls, ds, off):
    n = len(ds)
    mean = sum(ds) / n if n else 0.0
    std = math.sqrt(sum((d - mean) ** 2 for d in ds) / (n - 1)) if n > 1 else 0.0
    rmse = math.sqrt(sum(d * d for d in ds) / n) if n else 0.0
    return {"class": cls, "count": n, "outside": off, "mean": mean, "std": std, "rmse": rmse,
            "min": min(ds) if n else 0.0, "max": max(ds) if n else 0.0}


def checkpoints(ask):
    taken = []
    if "layer" in ask:
        for n, c in enumerate(CHECKS):
            taken.append((UIDS[f"k{n}"], c))
    for uid in ask.get("uids", []):
        if any(u == uid for u, _ in taken):
            continue
        n = next(i for i in range(len(CHECKS)) if UIDS[f"k{i}"] == uid)
        taken.append((uid, CHECKS[n]))
    field = ask.get("classField")
    points, by, every, off = [], {}, [], 0
    xs, ys = analysis.EXTRA[0][2], analysis.EXTRA[0][3]
    for uid, (x, y, dz, cls) in taken:
        z = analysis.terrace_z(x, y) + dz
        c = (cls or "") if field == "Sınıf" else ""
        on = xs[0] <= x <= xs[-1] and ys[0] <= y <= ys[-1]
        e = by.setdefault(c, ([], [0]))
        p = {"uid": uid, "at": [x, y], "z": z, "class": c}
        if on:
            sz = analysis.terrace_z(x, y)
            p["surfaceZ"] = sz
            p["difference"] = z - sz
            e[0].append(z - sz)
            every.append(z - sz)
        else:
            e[1][0] += 1
            off += 1
        points.append(p)
    return {"points": points, "classes": [stats(c, by[c][0], by[c][1][0]) for c in sorted(by)],
            "all": stats("", every, off)}


def answer(inp):
    sid = next(i for i, n in NAMES.items() if inp["surface"] in (i, n))
    out = {"surface": sid, "corners": len(CORNERS[sid]), "triangles": ANY}
    if "near" in inp:
        pairs = near_pairs(sid, inp["near"])
        out["nearCount"] = len(pairs)
        if pairs:
            out["near"] = pairs[:1000]
    if "spikes" in inp:
        sp = spikes(sid, inp["spikes"]["radius"], inp["spikes"]["limit"])
        out["spikeCount"] = len(sp)
        if sp:
            out["spikes"] = sp[:1000]
    if "gaps" in inp:
        out["gaps"] = gaps(sid, inp["gaps"])
    if "checkpoints" in inp:
        out["checkpoints"] = checkpoints(inp["checkpoints"])
    tri = LATTICE_TRIANGLES.get(sid)
    if tri is not None:
        out["triangles"] = tri
    return out


def cases():
    setup()
    out = []

    def q(name, inp, result=None, setup_=None):
        if result is None:
            result = {"status": "completed", "output": answer(inp), "warnings": []}
        c = {"name": name, "steps": [{"op": "execute", "input": inp, "result": result, "expect": UNCHANGED,
                                      "within": W}]}
        if setup_:
            c["setup"] = setup_
        out.append(c)

    fail = analysis.failed
    q("İki kez ölçülen nokta", {"surface": "Sivri", "near": 0.05})
    q("Yakın nokta yok", {"surface": "Sivri", "near": 0.03})
    q("Sivri ve çukur noktalar", {"surface": "Sivri", "spikes": {"radius": 1.0, "limit": 1.0}})
    q("Geniş erimle sivri noktalar", {"surface": "Sivri", "spikes": {"radius": 2.0, "limit": 0.3}})
    q("Sınırı sıfır: her köşe", {"surface": "Seyrek", "spikes": {"radius": 1.5, "limit": 0.0}})
    q("Düzlemde sivri yok", {"surface": "Düz", "spikes": {"radius": 1.5, "limit": 0.01}})
    q("Seyrek ölçünün boşlukları", {"surface": "Seyrek", "gaps": 2.0})
    q("Boşluk sınırı uzun", {"surface": "Seyrek", "gaps": 9.0})
    q("Düzlemde boşluk yok", {"surface": "Düz", "gaps": 1.5})
    q("Kontrol noktaları sınıflarıyla", {"surface": "Teras", "checkpoints": {"layer": "kontrol", "classField": "Sınıf"}})
    q("Kontrol noktaları tek sınıf", {"surface": "Teras", "checkpoints": {"layer": "kontrol"}})
    q("Seçilen kontrol noktaları", {"surface": "Teras", "checkpoints": {"uids": [UIDS["k5"], UIDS["k1"], UIDS["k9"]],
                                                                         "classField": "Sınıf"}})
    q("Katman ve seçim birlikte, yinelenen bir kez", {"surface": "t-6", "checkpoints": {
        "layer": "kontrol", "uids": [UIDS["k2"]], "classField": "Sınıf"}})
    q("Olmayan alan sınıf vermez", {"surface": "Teras", "checkpoints": {"layer": "kontrol", "classField": "Yok"}})
    q("Hepsi birden", {"surface": "Seyrek", "near": 1.5, "spikes": {"radius": 1.0, "limit": 0.02}, "gaps": 1.2})
    # Refusals.
    q("Bir şey sorulmadı", {"surface": "Düz"}, fail("invalid_analysis", "Bir şey sorun: near, spikes, gaps ya da checkpoints."))
    q("Yakınlık sıfır", {"surface": "Düz", "near": 0.0},
      fail("invalid_analysis", "Yakın noktaların uzaklığı sıfırdan büyük olmalı.", "near"))
    q("Erim sıfır", {"surface": "Düz", "spikes": {"radius": 0.0, "limit": 1.0}},
      fail("invalid_analysis", "Sivri noktanın komşu erimi sıfırdan büyük olmalı.", "spikes.radius"))
    q("Eksi sınır", {"surface": "Düz", "spikes": {"radius": 1.0, "limit": -1.0}},
      fail("invalid_analysis", "Sivri noktanın sınırı eksi olmayan bir sayı olmalı.", "spikes.limit"))
    q("Boşluk sınırı eksi", {"surface": "Düz", "gaps": -2.0},
      fail("invalid_analysis", "Boşluk sayılan kenar boyu sıfırdan büyük olmalı.", "gaps"))
    q("Kontrol noktası verilmedi", {"surface": "Düz", "checkpoints": {}},
      fail("invalid_analysis", "Kontrol noktalarını verin: bir katman (layer) ya da nesneler (uids).", "checkpoints"))
    q("Olmayan katman", {"surface": "Teras", "checkpoints": {"layer": "yok"}},
      fail("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
           "checkpoints.layer"))
    q("Olmayan nokta", {"surface": "Teras", "checkpoints": {"uids": ["0192f600-7c3e-7d4a-9b1e-999999999999"]}},
      fail("entity_not_found", "“0192f600-7c3e-7d4a-9b1e-999999999999” kimlikli nesne çizimde yok.",
           "checkpoints.uids[0]"))
    q("Kotsuz nokta", {"surface": "Teras", "checkpoints": {"uids": [UIDS["kotsuz"]]}},
      fail("invalid_analysis", f"“{UIDS['kotsuz']}” kontrol noktasının kotu yok.", "checkpoints.uids[0]"))
    q("Çizgi kontrol noktası değil", {"surface": "Teras", "checkpoints": {"uids": [UIDS["cizgi"]]}},
      fail("invalid_analysis", f"“{UIDS['cizgi']}” bir nokta değil; kontrol noktası kotlu bir nokta olmalı.",
           "checkpoints.uids[0]"))
    q("Kotsuz noktası olan katman", {"surface": "Teras", "checkpoints": {"layer": "kotsuz"}},
      fail("invalid_analysis", f"“{UIDS['kotsuz']}” kontrol noktasının kotu yok.", "checkpoints.layer"))
    q("Olmayan yüzey", {"surface": "Tasarım", "near": 1.0},
      fail("unknown_terrain", "“Tasarım” kimlikli ya da adlı yüzey projede yok.", "surface"))
    q("Kurulamayan yüzey", {"surface": "Kurulamaz", "near": 1.0},
      {"status": "failed", "error": {"code": "terrain_unbuildable", "message": ANY}})
    q("Yol projesi değil", {"surface": "Düz", "near": 1.0}, drainage.NOT_ROAD, setup_="$map")
    return out


def main():
    found = cases()
    for c in found:
        if c.get("setup") == "$map":
            c["setup"] = setup(work="map")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "road.terrains.quality", "commandVersion": 1,
           "title": "Ölçme kalitesini denetle: yakın, sivri, boşluk, kontrol noktaları",
           "note": "scripts/fixtures/road_terrain_quality_cases.py'den; köşeler kaydedildikleri sırayla, kontrol "
                   "noktaları teras kafesinde (her kare düzlem), ADR 0916'nın kuralıyla, KentOS kodu olmadan.",
           "setup": setup(), "cases": found}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    path = OUT / "road.terrains.quality.json"
    if "--check" in sys.argv:
        if not path.exists() or path.read_text() != text:
            sys.exit("güncel değil: road.terrains.quality.json")
    else:
        path.write_text(text)
    print(f"road.terrains.quality.json: {len(found)} durum")


if __name__ == "__main__":
    main()
