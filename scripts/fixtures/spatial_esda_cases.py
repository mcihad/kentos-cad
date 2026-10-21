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

docs/adr/0574 adds shapely and kneed (`--with shapely --with kneed`):

- the study area: shapely's union has the net area KentOS writes, covers the places KentOS keeps and not the ones it leaves out,
  and a numpy Monte Carlo (uniform points in the union by rejection, 2 000 patterns) has a mean nearest-neighbour distance
  within three standard errors of KentOS's simulations';
- DBSCAN's sweep: every cell's clusters and noise are scikit-learn's DBSCAN's (DBSCAN\*: its core points; a cell with a pair
  distance within 10⁻⁹ of the radius is left out: float64's neighbour rule decides it), the k-distance graph is
  `NearestNeighbors`', the knee is `kneed.KneeLocator`'s (convex, increasing, S = 0, online = True: the global optimum; its default online = False reports the first knee it detects, which on a graph with a second bend of outliers is that later one).
"""

import json
import math
import sys
from pathlib import Path

import esda
import libpysal
import numpy as np
from pointpats import PointPattern
from kneed import KneeLocator
from shapely.geometry import Point, Polygon
from shapely.ops import unary_union
from sklearn.cluster import DBSCAN, KMeans
from sklearn.neighbors import NearestNeighbors

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


def region_of(entities):
    """shapely's union of the objects' areas, holes out."""
    polys = []
    for e in entities:
        for q in [e] + list(e.get("parts") or []):
            ring = lambda r: [(float(p["x"]), float(p["y"])) for p in r["pts"]]  # noqa: E731
            polys.append(Polygon(ring(q), [ring(h) for h in q.get("holes") or []]))
    return unary_union(polys)


def study_check(cases, cid, objs, study, simulated=False):
    region = region_of(study)
    t = table(case(cases, cid))
    close(t["Alan (m²)"], region.area, 2, f"{cid} bölgenin alanı")
    kept = [i for i, e in enumerate(objs) if region.covers(Point(*[float(c) for c in si.place(e)]))]
    check(int(t["Nesne sayısı"]) == len(kept), f"{cid}: KentOS {t['Nesne sayısı']} nesne aldı, shapely bölgede {len(kept)} buldu")
    check(int(t["Alanın dışında kalan nesne"]) == len(objs) - len(kept), f"{cid}: dışarıda kalanların sayısı")
    if not simulated:
        return
    pts = np.array([[float(c) for c in si.place(objs[i])] for i in kept])
    n, rng = len(kept), np.random.default_rng(7)
    minx, miny, maxx, maxy = region.bounds
    means = []
    for _ in range(2000):
        got = []
        while len(got) < n:
            x, y = rng.uniform(minx, maxx), rng.uniform(miny, maxy)
            if region.covers(Point(x, y)):
                got.append((x, y))
        means.append(PointPattern(np.array(got)).mean_nnd)
    mean, se = float(np.mean(means)), float(np.std(means, ddof=1)) / math.sqrt(len(means))
    theirs = float(t["Benzetimlerin ortalama uzaklığı (m)"])
    # KentOS's 199 patterns have their own error: the sd of one pattern over √199.
    sd = float(np.std(means, ddof=1))
    bound = 3 * math.sqrt(se ** 2 + sd ** 2 / int(t["Benzetim sayısı"]))
    check(abs(theirs - mean) <= bound, f"{cid}: KentOS'un benzetimlerinin ortalaması {theirs}, numpy'ınki {mean:.3f} ± {bound:.3f}")
    close(t["Gözlenen ortalama uzaklık (m)"], PointPattern(pts).mean_nnd, 3, f"{cid} gözlenen ortalama uzaklık")


def sweep_check(cases, cid, objs, first=3, last=8, step=1, radii=None, steps=7, border=False):
    """Every cell of a sweep against scikit-learn; the k-distance graph and the knee against NearestNeighbors and kneed."""
    exp = case(cases, cid)
    rows = exp["outputs"]["table"]["rows"]
    pts = np.array([[float(c) - float(o) for c, o in zip(si.place(e), si.place(objs[0]))] for e in objs])
    n = len(pts)
    mins = list(range(first, last + 1, step))
    aim = mins[0]
    for m in mins[1:]:
        if abs(m - 4) < abs(aim - 4) or (abs(m - 4) == abs(aim - 4) and m > aim):
            aim = m
    check(int(exp["outputs"]["suggestedMinPoints"]) == aim, f"{cid}: önerilen en az nokta")
    d = NearestNeighbors(n_neighbors=aim).fit(pts).kneighbors(pts)[0]
    kd = np.sort(d[:, aim - 1]) if aim > 1 else np.zeros(n)
    x = np.arange(n) / (n - 1)
    y = (kd - kd[0]) / (kd[-1] - kd[0])
    ours = float(kd[int(np.argmax(x - y))])
    check(abs(ours - exp["outputs"]["kneeRadius"]) <= 1e-9 * max(1.0, ours), f"{cid}: dirsek {exp['outputs']['kneeRadius']}, numpy {ours}")
    found = KneeLocator(list(range(n)), list(kd), curve="convex", direction="increasing", S=0, online=True).knee
    kneed_note = "kneed dirseği bulamadı" if found is None else f"kneed {kd[found]:.3f}"
    if found is not None:
        check(abs(float(kd[found]) - ours) <= 1e-9 * max(1.0, ours), f"{cid}: dirsek {ours}, kneed {kd[found]}")
    r0, r1 = radii if radii else (0.5 * ours, 2.0 * ours)
    rs = [r0 + (r1 - r0) * j / (steps - 1) for j in range(steps)]
    dist = np.sqrt(((pts[:, None, :] - pts[None, :, :]) ** 2).sum(axis=2))
    at, skipped = 0, 0
    for m in mins:
        for r in rs:
            row = rows[at]
            at += 1
            check(row[0] == str(m), f"{cid}: satırın en az noktası")
            off = dist[~np.eye(n, dtype=bool)]
            if np.any(np.abs(off - r) <= 1e-9 * max(r, 1.0)):
                skipped += 1
                continue
            db = DBSCAN(eps=r, min_samples=m).fit(pts)
            labels = db.labels_
            if border:
                core = db.core_sample_indices_
                clusters, noise = len(set(labels[core])), n - len(core)
            else:
                clusters, noise = len(set(labels) - {-1}), int((labels == -1).sum())
            check(int(row[2]) == clusters, f"{cid} (m {m}, r {r:.3f}): küme sayısı KentOS {row[2]}, scikit-learn {clusters}")
            check(int(row[3]) == noise, f"{cid} (m {m}, r {r:.3f}): gürültü KentOS {row[3]}, scikit-learn {noise}")
    return len(rows), skipped, kneed_note


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
    study = si.objects("calisma")
    study_check(cases, "nn-study", kaza, study)
    study_check(cases, "nn-study-sim", kaza, study, simulated=True)
    study_check(cases, "nn-study-area-ignored", kaza, study)
    study_check(cases, "nn-study-selection", kaza, study[:1])
    notes = []
    notes.append(("sweep-auto",) + sweep_check(cases, "sweep-auto", kaza))
    notes.append(("sweep-range",) + sweep_check(cases, "sweep-range", kaza, 2, 10, 4, (4.0, 14.0), 6, True))
    notes.append(("sweep-stability",) + sweep_check(cases, "sweep-stability", kaza, 4, 5, 1, None, 3))
    notes.append(("sweep-subset",) + sweep_check(cases, "sweep-subset", kaza, 3, 4, 1, None, 3))
    for cid, cells, skipped, note in notes:
        print(f"{cid}: {cells} hücre, scikit-learn ile {cells - skipped} karşılaştırıldı, {skipped} belirsiz hücre atlandı; {note}")
    if PROBLEMS:
        print("\n".join(PROBLEMS))
        return 1
    print(f"esda {esda.__version__}, libpysal {libpysal.__version__}, scikit-learn, shapely ve kneed ile 23 durum: KentOS'un değerleri paketlerinkiyle aynı.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
