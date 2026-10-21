#!/usr/bin/env python3
"""Mekânsal istatistiğin PySAL ile çapraz denetimi (docs/adr/0566 §4): the values KentOS writes in the shared cases
(fixtures/processing/v1/spatial-inference.json, the core's own, checked against the independent reference) against
esda's, pointpats' and scikit-learn's own computation of the same statistics on the same drawing.

    uv run --no-project --python 3.12 --with esda --with libpysal --with pointpats --with scikit-learn \\
        python3 scripts/fixtures/spatial_esda_cases.py --check

The weights are the independent reference's neighbour lists (spatial_inference_cases.neighbours) made a libpysal W: the
packages compute, the rule of the neighbourhood is not theirs to test. What is compared:

- Moran I: I, E[I], the randomisation variance, z and p (esda.Moran);
- Gi*: every object's z (esda.G_Local, star, binary);
- Yerel Moran: every Iᵢ and quadrant (esda.Moran_Local); esda scales Iᵢ by n − 1 where Anselin (and KentOS) divide by n;
- the permutation p values: esda's 9 999 against KentOS's within five binomial errors (the packages draw with numpy);
- FDR: esda.fdr's threshold rejects what KentOS's adjusted p rejects;
- En yakın komşu: the observed mean distance (pointpats.PointPattern);
- DBSCAN and k-means: scikit-learn's partition (k-means from the same farthest-first seeds, Lloyd, one start) is KentOS's.
"""

import json
import math
import sys
from pathlib import Path

import esda
import libpysal
import numpy as np
from pointpats import PointPattern
from sklearn.cluster import DBSCAN, KMeans

sys.path.insert(0, str(Path(__file__).resolve().parent))
import spatial_inference_cases as si  # noqa: E402

CASES = si.OUT
PROBLEMS = []


def check(ok, what):
    if not ok:
        PROBLEMS.append(what)


def close(text, value, digits, what):
    """A written text of `digits` decimals against a package's double."""
    check(abs(float(text) - value) <= 0.5 * 10 ** -digits + 1e-9 * max(1.0, abs(value)), f"{what}: KentOS {text}, paket {value}")


def p_close(ours, theirs, r_ours, r_theirs, what):
    se = math.sqrt(max(theirs * (1 - theirs), 1e-6) * (1 / r_ours + 1 / r_theirs))
    check(abs(ours - theirs) <= 5 * se + 2 / r_ours, f"{what}: KentOS p {ours}, esda p {theirs}")


def weights(lists, own=False):
    neigh, w = {}, {}
    for i, lst in enumerate(lists):
        pairs = [(j, float(v)) for j, v in lst if own or j != i]
        neigh[i] = [j for j, _ in pairs]
        w[i] = [v for _, v in pairs]
    return libpysal.weights.W(neigh, w, silence_warnings=True)


def case(cases, cid):
    return next(c for c in cases if c["id"] == cid)["expect"]


def table(exp):
    return {r[0]: r[1] for r in exp["outputs"]["table"]["rows"]}


def values(objs, field):
    idx, pts, xs, _ = si.valued(objs, field)
    return idx, pts, np.array([float(x) for x in xs])


def moran_check(cases, cid, objs, field, concept, k, transform):
    idx, pts, y = values(objs, field)
    lists, _, _ = si.neighbours(pts, concept, None, k, False)
    w = weights(lists)
    w.transform = transform
    m = esda.Moran(y, w, transformation=transform, permutations=9999, two_tailed=True)
    t = table(case(cases, cid))
    close(t["Moran I"], m.I, 6, f"{cid} Moran I")
    close(t["Beklenen I"], m.EI, 6, f"{cid} beklenen I")
    close(t["Varyans"], m.VI_rand, 8, f"{cid} varyans")
    close(t["z"], m.z_rand, 4, f"{cid} z")
    close(t["p"], m.p_rand, 6, f"{cid} p")
    r = int(t["Permütasyon sayısı"])
    p_close(float(t["Permütasyon p"]), float(m.p_sim), r, 9999, f"{cid} permütasyon")


def permutations_of(exp):
    rows = {r[0]: r[1] for r in exp["outputs"]["profile"]["rows"]}
    return int(rows["Permütasyon sayısı"])


def attrs_of(exp, name):
    return [a.get(name) for a in (e["attrs"] for e in exp["added"])]


def hot_check(cases, cid, objs, field, concept, k):
    idx, pts, y = values(objs, field)
    lists, _, _ = si.neighbours(pts, concept, None, k, False)
    w = weights(lists)
    w.transform = "B"
    g = esda.G_Local(y, w, transform="B", star=True, permutations=9999, seed=1)
    exp = case(cases, cid)
    zs = attrs_of(exp, "z puanı")
    ps = attrs_of(exp, "Permütasyon p")
    r = permutations_of(exp)
    for i, z in enumerate(zs):
        close(z, g.Zs[i], 4, f"{cid} {i}. nesnenin Gi* z'si")
        p_close(float(ps[i]), float(g.p_sim[i]), r, 9999, f"{cid} {i}. nesnenin permütasyonu")
    if "Düzeltilmiş p" in exp["added"][0]["attrs"]:
        raw = np.array([float(p) for p in ps])
        adj = [float(q) for q in attrs_of(exp, "Düzeltilmiş p")]
        for alpha in (0.01, 0.05, 0.10):
            threshold = esda.fdr(raw, alpha)
            rejected = {i for i, p in enumerate(raw) if p <= threshold}
            ours = {i for i, q in enumerate(adj) if q <= alpha}
            check(rejected == ours, f"{cid} FDR %{alpha * 100:g}: esda {sorted(rejected)}, KentOS {sorted(ours)}")


QUADRANTS = {1: "YY", 2: "DY", 3: "DD", 4: "YD"}


def brute_p(z, lists, i, factor, draws, rng):
    """Object i's folded conditional permutation p by numpy's own draws: its neighbours' values drawn from the others."""
    nb = [j for j, _ in lists[i]]
    ws = np.array([float(v) for _, v in lists[i]])
    observed = factor * float((ws * z[nb]).sum())
    others = np.array([j for j in range(len(z)) if j != i])
    larger = 0
    for _ in range(draws):
        pick = rng.choice(others, size=len(nb), replace=False)
        larger += factor * float((ws * z[pick]).sum()) >= observed - 1e-12
    low = min(larger, draws - larger)
    return (low + 1) / (draws + 1)


def lisa_check(cases, cid, objs, field, concept, k, transform):
    idx, pts, y = values(objs, field)
    lists, _, _ = si.neighbours(pts, concept, None, k, False)
    w = weights(lists)
    w.transform = transform
    lm = esda.Moran_Local(y, w, transformation=transform, permutations=9999, seed=1)
    exp = case(cases, cid)
    n = len(y)
    r = permutations_of(exp)
    z = y - y.mean()
    m2 = float((z * z).sum()) / n
    rng = np.random.default_rng(5)
    for i, (value, quadrant, p) in enumerate(zip(attrs_of(exp, "Yerel Moran I"), attrs_of(exp, "Çeyrek"),
                                                attrs_of(exp, "Permütasyon p"))):
        if not value:
            continue
        # esda's Iᵢ is (n − 1) Σ… / Σz²; Anselin's, KentOS's, n Σ… / Σz².
        close(value, lm.Is[i] * n / (n - 1), 6, f"{cid} {i}. nesnenin Iᵢ'si")
        check(QUADRANTS[int(lm.q[i])] == quadrant, f"{cid} {i}. nesnenin çeyreği: esda {QUADRANTS[int(lm.q[i])]}, KentOS {quadrant}")
        if transform == "r":
            p_close(float(p), float(lm.p_sim[i]), r, 9999, f"{cid} {i}. nesnenin permütasyonu")
        else:
            # esda's conditional permutation of binary weights is not the definition's (docs/adr/0566 §4): the p is
            # numpy's own draws of the neighbours' values.
            lists_i = [(j, w_) for j, w_ in zip(w.neighbors[i], w.weights[i])]
            theirs = brute_p(z, {i: lists_i}, i, float(z[i]) / m2, 20000, rng)
            p_close(float(p), theirs, r, 20000, f"{cid} {i}. nesnenin permütasyonu (numpy)")


def nearest_check(cases, cid, objs):
    pts = np.array([[float(c) for c in si.place(e)] for e in objs])
    t = table(case(cases, cid))
    close(t["Gözlenen ortalama uzaklık (m)"], PointPattern(pts).mean_nnd, 3, f"{cid} ortalama en yakın komşu uzaklığı")


def partition(labels, noise=None):
    """The groups of a labelling as sets of indices (noise apart)."""
    groups = {}
    for i, l in enumerate(labels):
        if l == noise:
            continue
        groups.setdefault(l, set()).add(i)
    return sorted(sorted(g) for g in groups.values())


def seeds(pts, k):
    """k-means' farthest-first seeds (ADR 0238 §10): the place nearest the mean, then the farthest from those taken."""
    mean = pts.mean(axis=0)
    first = int(np.argmin(((pts - mean) ** 2).sum(axis=1)))
    chosen = [first]
    least = ((pts - pts[first]) ** 2).sum(axis=1)
    while len(chosen) < k:
        far = int(np.argmax(least))
        chosen.append(far)
        least = np.minimum(least, ((pts - pts[far]) ** 2).sum(axis=1))
    return pts[chosen]


def clusters_check(cases, cid, objs, tool, **params):
    pts = np.array([[float(c) - float(o) for c, o in zip(si.place(e), si.place(objs[0]))] for e in objs])
    exp = case(cases, cid)
    ours = [int(a) for a in attrs_of(exp, "Küme")]
    if tool == "dbscan":
        theirs = DBSCAN(eps=params["radius"], min_samples=params["min_points"]).fit(pts).labels_
        check(partition(ours, 0) == partition(list(theirs), -1), f"{cid}: DBSCAN'ın kümeleri scikit-learn'ünkiyle aynı değil")
    else:
        k = params["k"]
        km = KMeans(n_clusters=k, init=seeds(pts, k), n_init=1, algorithm="lloyd", tol=0.0, max_iter=500).fit(pts)
        check(partition(ours) == partition(list(km.labels_)), f"{cid}: k-ortalamaların kümeleri scikit-learn'ünkiyle aynı değil")


def main():
    cases = json.loads(CASES.read_text("utf-8"))["cases"]
    parsel, kaza = si.objects("parsel"), si.objects("kaza")
    moran_check(cases, "moran-perm", parsel, "Değer", "band", 8, "r")
    moran_check(cases, "moran-perm-seed", parsel, "Değer", "nearest", 4, "B")
    moran_check(cases, "moran-perm-kaza", kaza, "Ağırlık", "inverse", 8, "r")
    hot_check(cases, "hot-perm", parsel, "Değer", "band", 8)
    hot_check(cases, "hot-fdr", parsel, "Değer", "band", 8)
    hot_check(cases, "hot-kaza-fdr", kaza, "Ağırlık", "nearest", 5)
    lisa_check(cases, "lisa-band", parsel, "Değer", "band", 8, "r")
    lisa_check(cases, "lisa-fdr", parsel, "Değer", "nearest", 4, "r")
    lisa_check(cases, "lisa-kaza", kaza, "Ağırlık", "band", 8, "B")
    nearest_check(cases, "nn-sim", kaza)
    nearest_check(cases, "nn-sim-subset", kaza[:16])
    clusters_check(cases, "dbscan-boot", kaza, "dbscan", radius=12, min_points=4)
    clusters_check(cases, "dbscan-subset", kaza, "dbscan", radius=12, min_points=3)
    clusters_check(cases, "kmeans-boot-3", kaza, "kmeans", k=3)
    clusters_check(cases, "kmeans-boot-5", kaza, "kmeans", k=5)
    if PROBLEMS:
        print("\n".join(PROBLEMS))
        return 1
    print(f"esda {esda.__version__}, libpysal {libpysal.__version__}, scikit-learn ile 15 durum: KentOS'un değerleri paketlerinkiyle aynı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
