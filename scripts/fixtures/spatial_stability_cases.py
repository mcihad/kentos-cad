"""En yakın komşu'nun benzetimi ve kümelerin kararlılığı (docs/adr/0566): the cases that spatial_inference_cases.py adds
to fixtures/processing/v1/spatial-inference.json, written from the ADR without KentOS code.

- **Monte Carlo** (§1): the random places are the rule's numbers in Python's integers, placed by the same IEEE double
  operations the core does (x = x₀ + u·w, then y), so they are the core's places bit for bit; every simulation's mean
  nearest-neighbour distance is then exact (fractions and 60-digit roots). The places' box is the core's box of its
  worked places (points' coordinates less the first's, a double subtraction), so only point layers are simulated here.
- **Stability** (§2): the resamples are the rule's draws; DBSCAN and k-means are ADR 0238's rules in fractions (a distance
  within 10⁻⁹ of the radius and a centre's tie refused); the Jaccard similarities and their means are the ADR's float64
  operations in Python's doubles, the same IEEE operations as the core's.
"""

import math
from fractions import Fraction as F

import spatial_inference_cases as si

LAYER = "kaza"


# ── Places as the core works them ─────────────────────────────────────


def worked_places(objs):
    """The points' places as the core's doubles, each less the first (ADR 0238 §1): (input index, (x, y))."""
    out, origin = [], None
    for i, e in enumerate(objs):
        assert e["kind"] == "point" and not e.get("parts"), "yalnız tek noktalı nesneler"
        p = (float(e["p"]["x"]), float(e["p"]["y"]))
        if origin is None:
            origin = p
        out.append((i, (p[0] - origin[0], p[1] - origin[1])))
    return out


def box(pts):
    xs, ys = [p[0] for p in pts], [p[1] for p in pts]
    x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
    return (x0, y0), x1 - x0, y1 - y0


def region(pts, area):
    lo, w, h = box(pts)
    if area is None:
        return lo, w, h, "Yerlerin kutusu"
    cx, cy = lo[0] + w / 2.0, lo[1] + h / 2.0
    if w > 0.0 and h > 0.0:
        s = math.sqrt(area / (w * h))
        w2, h2 = w * s, h * s
    else:
        w2 = h2 = math.sqrt(area)
    return (cx - w2 / 2.0, cy - h2 / 2.0), w2, h2, "Alanla ölçeklenmiş kutu"


def unit(seq):
    return (seq.next() >> 11) / 9007199254740992.0


def mean_nearest(pts):
    """The exact mean nearest-neighbour distance of double places."""
    fp = [(F(x), F(y)) for x, y in pts]
    total = F(0)
    for i, p in enumerate(fp):
        total += si.sqrt(min(si.d2(p, q) for j, q in enumerate(fp) if j != i))
    return total / len(fp)


def place_lines(objs, index):
    text = ""
    for i in index:
        x, y = si.place(objs[i])
        text += f"{i}\t{si.fmt(x, 6)}\t{si.fmt(y, 6)}\n"
    return text


def experiment(rows, text, scope):
    digest = si.hashlib.sha256(text.encode("utf-8")).hexdigest()
    rows = rows + [("Kapsam", scope, "scope"), ("Girdi özeti (SHA-256)", digest, "inputs")]
    table = {"columns": ["Alan", "Değer"],
             "rows": [[label, value[:16] + "…" if key == "inputs" else value] for label, value, key in rows]}
    exp = "{" + ",".join(si.json.dumps(key) + ":" + si.json.dumps(value, ensure_ascii=False) for _, value, key in rows) + "}"
    return {"table": table, "experiment": exp}, digest


# ── En yakın komşu's Monte Carlo test (§1) ────────────────────────────


DONNELLY_INFO = ("Donnelly'nin kenar düzeltmesi: beklenen uzaklık 0,5·√(A/n) + (0,0514 + 0,041/√n)·P/n, standart hata "
                 "√(0,0703·A/n² + 0,037·P·√(A/n⁵)); P bölgenin çevresi. Dikdörtgene yakın bölgeler için türetildi.")


def expectation(n, a, perimeter, edge):
    """The expected mean distance and its standard error: Clark and Evans's, or Donnelly's (1978) for the region's edge
    (docs/adr/0639 §2), exact."""
    if not edge:
        return F(1, 2) / si.sqrt(F(n) / a), F(26136, 100000) / si.sqrt(F(n * n) / a)
    e = F(1, 2) * si.sqrt(a / n) + (F(514, 10000) + F(41, 1000) / si.sqrt(F(n))) * perimeter / n
    var = F(703, 10000) * a / (n * n) + F(37, 1000) * perimeter * si.sqrt(a / F(n) ** 5)
    return e, si.sqrt(var)


def donnelly_rows(n, perimeter, edge):
    """The rows, infos and warnings the correction adds."""
    if not edge:
        return [], [], []
    warnings = [] if n >= 7 else [f"Donnelly'nin düzeltmesi en az 7 nesneyle türetildi; {n} nesnede beklenen uzaklık yaklaşıktır."]
    return [["Çevre (m)", si.fmt(perimeter, 2)], ["Kenar düzeltmesi", "Donnelly (1978)"]], [DONNELLY_INFO], warnings


def nearest_simulated(objs, area=None, count=999, seed=1, scope=("layer", LAYER), edge=False):
    placed = worked_places(objs)
    n = len(placed)
    pts = [p for _, p in placed]
    exact = [si.place(e) for e in objs]
    if area is None:
        lo, w, h = box(pts)
        a = F(w) * F(h)
    else:
        a = F(area)
    (x0, y0), w, h, words = region(pts, area)
    perimeter = 2 * (F(w) + F(h))
    observed = F(0)
    for i, p in enumerate(exact):
        observed += si.sqrt(min(si.d2(p, q) for j, q in enumerate(exact) if j != i))
    observed /= n
    expected, se = expectation(n, a, perimeter, edge)
    ratio = observed / expected
    z = (observed - expected) / se
    p = si.p_value(z)
    extra, extra_infos, extra_warnings = donnelly_rows(n, perimeter, edge)
    if count == 0:
        rows = [["Nesne sayısı", str(n)], ["Gözlenen ortalama uzaklık (m)", si.fmt(observed, 3)],
                ["Beklenen ortalama uzaklık (m)", si.fmt(expected, 3)], ["En yakın komşu oranı", si.fmt(ratio, 4)],
                ["z", si.fmt(z, 4)], ["p", si.fmt(p, 6)], ["Alan (m²)", si.fmt(a, 2)]] + extra
        pattern = ("Kümelenmiş" if z < 0 else "Dağınık") if p < 0.05 else "Rastgele"
        rows.append(["Desen", pattern])
        edged = "Donnelly kenar düzeltmesiyle " if edge else ""
        summary = f"En yakın komşu oranı {si.fmt(ratio, 4)} ({edged}z {si.fmt(z, 4)}, p {si.fmt(p, 6)}): {pattern.lower()}."
        return {"table": {"columns": ["Ölçü", "Değer"], "rows": rows}, "summary": summary, "infos": extra_infos,
                "warnings": extra_warnings}
    bound = si.sqrt(F(w) ** 2 + F(h) ** 2)
    seq = si.Sequence(seed)
    sims, larger = [], 0
    for _ in range(count):
        thrown = []
        for _ in range(n):
            x = x0 + unit(seq) * w
            y = y0 + unit(seq) * h
            thrown.append((x, y))
        m = mean_nearest(thrown)
        si.near(m, observed, bound, "benzetimin uzaklığı gözlenene")
        if m >= observed:
            larger += 1
        sims.append(m)
    mean, sd = si.moments(sims)
    ps, _ = si.folded(larger, count)
    zs = (observed - mean) / sd
    se_p = math.sqrt(ps * (1.0 - ps) / count)
    clustered = zs < 0
    pattern = ("Kümelenmiş" if clustered else "Dağınık") if ps < 0.05 else "Rastgele"
    rows = [["Nesne sayısı", str(n)], ["Gözlenen ortalama uzaklık (m)", si.fmt(observed, 3)],
            ["Beklenen ortalama uzaklık (m)", si.fmt(expected, 3)], ["En yakın komşu oranı", si.fmt(ratio, 4)],
            ["z", si.fmt(z, 4)], ["p", si.fmt(p, 6)], ["Alan (m²)", si.fmt(a, 2)]] + extra + [
            ["Benzetim sayısı", str(count)], ["Tohum", str(seed)], ["Benzetim alanı", words],
            ["Benzetimlerin ortalama uzaklığı (m)", si.fmt(mean, 3)], ["Benzetim oranı", si.fmt(observed / mean, 4)],
            ["Benzetim z", si.fmt(zs, 4)], ["Benzetim p", si.fmt(ps, 6)], ["p'nin standart hatası", si.fmt(se_p, 6)],
            ["Desen", pattern]]
    word = pattern.lower()
    summary = (f"En yakın komşu oranı {si.fmt(ratio, 4)} (benzetim z {si.fmt(zs, 4)}, p {si.fmt(ps, 6)}; "
               f"{count} rastgele desen, tohum {seed}): {word}.")
    prof_rows = [("Kural", si.RULE, "rule"), ("Araç", "En yakın komşu", "tool"),
                 ("Çıkarım", "Monte Carlo (rastgele desen)", "inference"), ("Benzetim sayısı", str(count), "simulations"),
                 ("Tohum", str(seed), "seed"), ("Sayı üreteci", si.GENERATOR, "generator"),
                 ("Benzetim alanı", f"{words}, {si.fmt(w * h, 2)} m²", "region"), ("Nesne sayısı", str(n), "objects")]
    if edge:
        prof_rows.append(("Kenar düzeltmesi", "Donnelly (1978)", "edge"))
    prof, digest = experiment(prof_rows, place_lines(objs, [i for i, _ in placed]), si.scope_label(scope[0], scope[1]))
    infos = extra_infos + [f"Deney profili ({si.RULE}): tohum {seed}, {count} rastgele desen; girdi özeti {digest[:12]}…."]
    return {"table": {"columns": ["Ölçü", "Değer"], "rows": rows}, "summary": summary, "infos": infos,
            "warnings": extra_warnings + si.subset_warning(scope[0], n), "profile": prof, "pSimulation": ps}


# ── Clusters (ADR 0238 §10) in fractions ──────────────────────────────


def far_from(a, b, bound, what):
    if a != b and abs(a - b) <= F(1, 10 ** 9) * max(bound, F(1)):
        raise si.Refused(f"{what}: {float(a)} ile {float(b)} float64'te ayırt edilemeyecek kadar yakın")


def dbscan_labels(pts, radius, min_points, border_noise):
    sq = F(radius) ** 2
    lists = []
    for p in pts:
        found = []
        for j, q in enumerate(pts):
            s = si.d2(p, q)
            far_from(s, sq, sq, "ε")
            if s <= sq:
                found.append(j)
        lists.append(found)
    core = [len(l) >= min_points for l in lists]
    labels = [None] * len(pts)
    nxt = 1
    for i in range(len(pts)):
        if labels[i] is not None or not core[i]:
            continue
        c, nxt = nxt, nxt + 1
        labels[i] = c
        queue = [i]
        while queue:
            p = queue.pop(0)
            for q in lists[p]:
                if labels[q] is not None or (border_noise and not core[q]):
                    continue
                labels[q] = c
                if core[q]:
                    queue.append(q)
    return [0 if l is None else l for l in labels]


def dyadic(v):
    return v.denominator & (v.denominator - 1) == 0 and v.denominator <= 2 ** 30


def nearest_center(p, centers, bound):
    """The nearest centre, ties to the smaller (ADR 0238 §10); a tie only where float64 holds it exactly too."""
    ds = [si.d2(p, c) for c in centers]
    best = min(range(len(ds)), key=lambda c: (ds[c], c))
    for c, d in enumerate(ds):
        if c == best:
            continue
        if d == ds[best]:
            if centers[c] == centers[best]:
                continue  # copies of one place: the smaller is taken on both sides
            assert all(dyadic(v) for v in (*p, *centers[c], *centers[best])), "kesin olmayan eşitlik"
        else:
            far_from(d, ds[best], bound, "k-ortalamalar")
    return best


def k_means_labels(pts, k):
    n = len(pts)
    bound = max(max(si.d2(p, q) for q in pts) for p in pts)
    mean = (sum(p[0] for p in pts) / n, sum(p[1] for p in pts) / n)
    first = nearest_center(mean, pts, bound)
    centers = [pts[first]]
    least = [si.d2(p, pts[first]) for p in pts]
    while len(centers) < k:
        far = max(range(n), key=lambda i: (least[i], -i))
        for i in range(n):
            if i != far and least[i] != least[far]:
                far_from(least[i], least[far], bound, "en uzak")
        centers.append(pts[far])
        least = [min(least[i], si.d2(p, pts[far])) for i, p in enumerate(pts)]
    labels = [nearest_center(p, centers, bound) for p in pts]
    rounds, settled = 0, False
    while rounds < 500:
        for c in range(k):
            ms = [p for p, l in zip(pts, labels) if l == c]
            if ms:
                centers[c] = (sum(p[0] for p in ms) / len(ms), sum(p[1] for p in ms) / len(ms))
        rounds += 1
        new = [nearest_center(p, centers, bound) for p in pts]
        if new == labels:
            settled = True
            break
        labels = new
    return [l + 1 for l in labels], rounds, settled


# ── The stability (§2) ────────────────────────────────────────────────


def resample(n, scheme, seq):
    if scheme == "bootstrap":
        idx = [seq.below(n) for _ in range(n)]
    else:
        m = n // 2
        pool = list(range(n))
        for t in range(m):
            j = t + seq.below(n - t)
            pool[t], pool[j] = pool[j], pool[t]
        idx = pool[:m]
    # Each object once however often drawn: fpc's unique(bsamp) (docs/adr/0639 §1).
    return sorted(set(idx))


def class_of(g):
    if g is None:
        return "Sınanamadı"
    for limit, name in ((0.85, "Çok kararlı"), (0.75, "Kararlı"), (0.6, "Belirsiz")):
        if g >= limit:
            return name
    return "Güvenilmez"


def stability(pts, labels, scheme, resamples, seed, cluster, trace=None):
    """The clusters' stability; `trace` (a list) gets every resample's γ per cluster, 1…k."""
    n = len(pts)
    k = max(labels, default=0)
    size = [0] * (k + 1)
    for l in labels:
        size[l] += 1
    total, counted, recovered, dissolved = [0.0] * (k + 1), [0] * (k + 1), [0] * (k + 1), [0] * (k + 1)
    fewer = unsettled = 0
    seq = si.Sequence(seed)
    for _ in range(resamples):
        idx = resample(n, scheme, seq)
        boot, few, uns = cluster([pts[i] for i in idx])
        fewer += few
        unsettled += uns
        seen = {}
        for t, i in enumerate(idx):
            seen.setdefault(i, boot[t])
        part = [0] * (k + 1)
        dsize = {}
        joint = {}
        for i, d in seen.items():
            c = labels[i]
            if c > 0:
                part[c] += 1
            if d > 0:
                dsize[d] = dsize.get(d, 0) + 1
            if c > 0 and d > 0:
                joint[(c, d)] = joint.get((c, d), 0) + 1
        best = [0.0] * (k + 1)
        for (c, d), both in sorted(joint.items()):
            j = both / (part[c] + dsize[d] - both)
            if j > best[c]:
                best[c] = j
        gammas = []
        for c in range(1, k + 1):
            # A cluster none of whose objects is drawn counts with γ = 0 (fpc's bootmean).
            g = best[c] if part[c] else 0.0
            gammas.append(g)
            total[c] += g
            counted[c] += 1
            recovered[c] += g > 0.75
            dissolved[c] += g <= 0.5
        if trace is not None:
            trace.append(gammas)
    means = [None if counted[c] == 0 else total[c] / counted[c] for c in range(k + 1)]
    return size, means, counted, recovered, dissolved, fewer, unsettled


def scheme_words(scheme):
    return {"bootstrap": ("Bootstrap (yerine koyarak)", "bootstrap tekrarı"),
            "subset": ("Alt örnek (yarısı, yerine koymadan)", "alt örnek tekrarı")}[scheme]


def clustered(objs, tool, labels, summary, warnings, cluster, parameters, scheme, resamples, seed, scope):
    exact = [si.place(e) for e in objs]
    size, means, counted, recovered, dissolved, fewer, unsettled = stability(exact, labels, scheme, resamples, seed, cluster)
    rows, clusters, stable = [], 0, 0
    for c in range(1, len(size)):
        if size[c] == 0:
            continue
        clusters += 1
        g = means[c]
        if g is not None:
            for limit in (0.85, 0.75, 0.6):
                assert g != limit, "kararlılık sınıf eşiğinde"
        stable += g is not None and g >= 0.75
        rows.append([str(c), str(size[c]), "—" if g is None else si.fmt(g, 3), class_of(g), str(recovered[c]),
                     str(dissolved[c]), str(counted[c])])
    words, noun = scheme_words(scheme)
    summary += f" Kararlı küme: {stable} / {clusters} (Jaccard ≥ 0.75; {resamples} {noun}, tohum {seed})."
    infos = ["Kararlılık: her kümenin tekrarlarda en çok benzediği yeni kümeyle Jaccard benzerliğinin ortalaması (Hennig "
             "2007); 0.85 ve üstü çok kararlı, 0.75 ve üstü kararlı, 0.6'nın altı güvenilmez."]
    warnings = list(warnings)
    if fewer:
        warnings.append(f"{fewer} tekrarda örneğin farklı yer sayısı küme sayısından az; o tekrarlar daha az kümeyle kümelendi.")
    if unsettled:
        warnings.append(f"{unsettled} tekrarda atamalar yinelemelerin sınırında durulmadı; son atamalarla sayıldı.")
    warnings += si.subset_warning(scope[0], len(objs))
    prof_rows = [("Kural", si.RULE, "rule"), ("Araç", tool, "tool"), ("Çıkarım", words, "inference"),
                 ("Tekrar sayısı", str(resamples), "resamples"), ("Tohum", str(seed), "seed"),
                 ("Sayı üreteci", si.GENERATOR, "generator"), ("Parametreler", parameters, "parameters"),
                 ("Nesne sayısı", str(len(objs)), "objects")]
    prof, digest = experiment(prof_rows, place_lines(objs, range(len(objs))), si.scope_label(scope[0], scope[1]))
    infos.append(f"Deney profili ({si.RULE}): tohum {seed}, {resamples} tekrar; girdi özeti {digest[:12]}….")
    copies = []
    for i, l in enumerate(labels):
        if l == 0:
            attrs = [["Küme", "0"], ["Küme boyu", ""], ["Kararlılık", ""], ["Kararlılık sınıfı", ""]]
            color = si.json.loads('"#BDBDBD"')
        else:
            g = means[l]
            attrs = [["Küme", str(l)], ["Küme boyu", str(size[l])], ["Kararlılık", "" if g is None else si.fmt(g, 3)],
                     ["Kararlılık sınıfı", class_of(g)]]
            color = CLUSTER_COLORS[(l - 1) % len(CLUSTER_COLORS)]
        copies.append({"index": i, "attrs": attrs, "color": color})
    table = {"columns": ["Küme", "Nesne sayısı", "Kararlılık", "Sınıf", "Yeniden bulunma", "Çözülme", "Sayılan tekrar"],
             "rows": rows}
    return {"summary": summary, "infos": infos, "warnings": warnings, "table": table, "profile": prof, "copies": copies,
            "clusters": clusters, "stable": stable}


CLUSTER_COLORS = ["#1F77B4", "#FF7F0E", "#2CA02C", "#D62728", "#9467BD", "#8C564B", "#E377C2", "#17BECF", "#BCBD22", "#7F7F7F"]


def dbscan(objs, radius, min_points, border_noise=False, scheme="bootstrap", resamples=100, seed=1, scope=("layer", LAYER)):
    pts = [si.place(e) for e in objs]
    labels = dbscan_labels(pts, radius, min_points, border_noise)
    clusters = max(labels, default=0)
    noise = labels.count(0)
    summary = (f"Küme bulunamadı; {len(pts)} nesnenin hepsi gürültü." if clusters == 0
               else f"{len(pts)} nesne {clusters} kümeye ayrıldı; {noise} nesne gürültü.")
    parameters = f"Yarıçap {si.fmt(F(radius), 3)} m, en az {min_points} nokta" + (", sınır noktaları gürültü" if border_noise else "")
    return clustered(objs, "DBSCAN kümeleme", labels, summary, [], lambda s: (dbscan_labels(s, radius, min_points, border_noise), 0, 0),
                     parameters, scheme, resamples, seed, scope)


def k_means(objs, k, scheme="bootstrap", resamples=100, seed=1, scope=("layer", LAYER)):
    pts = [si.place(e) for e in objs]
    labels, rounds, settled = k_means_labels(pts, k)
    warnings = [] if settled else ["Atamalar 500 yinelemede durulmadı; son atamalar yazıldı."]
    clusters = len(set(labels))
    summary = f"{len(pts)} nesne {clusters} kümeye ayrıldı ({rounds} yineleme)."

    def again(sample):
        d = len(set(sample))
        lab, _, held = k_means_labels(sample, min(k, d))
        return lab, int(d < k), int(not held)

    return clustered(objs, "k-ortalamalar kümeleme", labels, summary, warnings, again, f"k = {k}", scheme, resamples, seed, scope)


# ── Self-checks against the theory ────────────────────────────────────


def self_checks():
    # A square without edge effect is not to be had; a large count on a torus-free square: the simulations' mean meets
    # Donnelly's edge-corrected expectation 0.5·√(A/n) + (0.0514 + 0.041/√n)·P/n within 2 %.
    seq = si.Sequence(9)
    n, side = 50, 100.0
    means = []
    for _ in range(200):
        pts = [(unit(seq) * side, unit(seq) * side) for _ in range(n)]
        means.append(float(mean_nearest(pts)))
    m = sum(means) / len(means)
    donnelly = 0.5 * math.sqrt(side * side / n) + (0.0514 + 0.041 / math.sqrt(n)) * 4 * side / n
    assert abs(m - donnelly) < 0.02 * donnelly, (m, donnelly)
    # Two far clusters of four (every place a core) are found again in every bootstrap: stability 1. With two points a
    # core, a cluster's one drawn place is noise and its J is 0 (0.98 here): the rule counts it.
    pts = [(F(x), F(y)) for x, y in [(0, 0), (1, 0), (0, 1), (1, 1), (100, 100), (101, 100), (100, 101), (101, 101)]]
    labels = dbscan_labels(pts, 3, 1, False)
    _, means, *_ = stability(pts, labels, "bootstrap", 50, 3, lambda s: (dbscan_labels(s, 3, 1, False), 0, 0))
    assert means[1] == 1.0 and means[2] == 1.0, means
    # k = 2 on two clusters finds them in every bootstrap; k = 3 splits one, and a split cluster is less stable.
    pts = [(F(x), F(y)) for x, y in [(0, 0), (1.25, 0.5), (0.25, 1.75), (1.5, 1.25), (0.75, 2.5),
                                      (100, 100), (101.5, 100.25), (100.75, 101.5), (101.25, 101.75), (102.5, 100.5)]]

    def again(k):
        return lambda s: (k_means_labels(s, min(k, len(set(s))))[0], 0, 0)

    _, two, *_ = stability(pts, k_means_labels(pts, 2)[0], "bootstrap", 50, 5, again(2))
    _, three, *_ = stability(pts, k_means_labels(pts, 3)[0], "bootstrap", 50, 5, again(3))
    assert two[1:] == [1.0, 1.0], two
    assert min(three[1:]) < 1.0, three


def cases(case):
    """The ADR's cases on the drawing's accidents, with `case` the caller's writer."""
    kaza = si.objects(LAYER)
    lay = {"scope": "layer", "layerId": LAYER}
    out = []
    out.append(case("nn-sim", "En yakın komşu, 999 rastgele desen, tohum 1: kümelenmiş, kenar etkisinden arınmış p",
                    "stats.nearestNeighbor", {"input": lay, "inference": "simulation"}, nearest_simulated(kaza)))
    out.append(case("nn-sim-area", "En yakın komşu, 199 rastgele desen, tohum 4, alan 40 000 m²: kutu alanla ölçeklenir",
                    "stats.nearestNeighbor", {"input": lay, "inference": "simulation", "simulations": 199, "seed": 4,
                                              "area": 40000},
                    nearest_simulated(kaza, area=40000.0, count=199, seed=4)))
    sel = [e["id"] for e in kaza[:16]]
    out.append(case("nn-sim-subset", "En yakın komşu, seçili 16 kaza, 99 rastgele desen: alt küme diye söylenir",
                    "stats.nearestNeighbor", {"input": {"scope": "selection"}, "inference": "simulation", "simulations": 99},
                    nearest_simulated(kaza[:16], count=99, scope=("selection", None)), sel))
    out.append(case("nn-sim-few", "En yakın komşu, 5 benzetim: alan en az 9 ister", "stats.nearestNeighbor",
                    {"input": lay, "inference": "simulation", "simulations": 5},
                    {"invalid": [{"param": "simulations", "message": "“Benzetim sayısı” en az 9 olmalı."}]}))
    out.append(case("dbscan-boot", "DBSCAN, yarıçap 12 m, en az 4 nokta, 100 bootstrap tekrarı: üç küme kararlı",
                    "stats.dbscan", {"input": lay, "radius": 12, "minPoints": 4, "stability": "bootstrap"},
                    dbscan(kaza, 12, 4)))
    out.append(case("dbscan-subset", "DBSCAN, yarıçap 12 m, en az 3 nokta, 60 alt örnek tekrarı, tohum 7",
                    "stats.dbscan", {"input": lay, "radius": 12, "minPoints": 3, "stability": "subset", "resamples": 60,
                                     "seed": 7},
                    dbscan(kaza, 12, 3, scheme="subset", resamples=60, seed=7)))
    out.append(case("kmeans-boot-3", "k-ortalamalar, k = 3, 100 bootstrap tekrarı: iki kaza kümesi birleşmiş ve belirsiz, aykırı K25 tek başına küme",
                    "stats.kMeans", {"input": lay, "clusters": 3, "stability": "bootstrap"}, k_means(kaza, 3)))
    out.append(case("kmeans-boot-5", "k-ortalamalar, k = 5, 50 bootstrap tekrarı, tohum 2: üç küme ve iki aykırı, hepsi çok kararlı",
                    "stats.kMeans", {"input": lay, "clusters": 5, "stability": "bootstrap", "resamples": 50, "seed": 2},
                    k_means(kaza, 5, resamples=50, seed=2)))
    out.append(case("kmeans-subset", "k-ortalamalar, k = 4, 40 alt örnek tekrarı, tohum 9 (docs/adr/0741)",
                    "stats.kMeans", {"input": lay, "clusters": 4, "stability": "subset", "resamples": 40, "seed": 9},
                    k_means(kaza, 4, scheme="subset", resamples=40, seed=9)))
    out.append(case("kmeans-few", "k-ortalamalar, 5 tekrar: alan en az 10 ister", "stats.kMeans",
                    {"input": lay, "clusters": 3, "stability": "bootstrap", "resamples": 5},
                    {"invalid": [{"param": "resamples", "message": "“Tekrar sayısı” en az 10 olmalı."}]}))
    return out
