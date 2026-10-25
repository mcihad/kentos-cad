"""Çalışma alanı ve DBSCAN parametre taraması (docs/adr/0574): the cases that spatial_inference_cases.py adds to
fixtures/processing/v1/spatial-inference.json, written from the ADR without KentOS code.

- **Study area** (§1): the objects' areas are read from the drawing's JSON (outer rings, holes, parts); the region is their union,
  the net area (holes out, overlaps once) exact in fractions by coordinate compression (axis-parallel rectangles only), inside
  and outside by exact crossing numbers (a place or a candidate within 10⁻⁵ m of an edge is refused here: the core's boundary is
  1 µm). The simulation's candidates are the rule's numbers (docs/adr/0566 §1, one sequence, u then v) placed by the same IEEE
  double operations the core does; the thrown points' mean nearest distances are exact fractions.
- **Sweep** (§2): the k-distance graph, the knee, the radii, the aim and the suggestion are float64 operations the ADR spells out,
  done here in Python's doubles on the places as the core works them (each less the first); DBSCAN's neighbours are the module's
  own rule (ADR 0238: d = √(dx² + dy²) ≤ ε in doubles; the radii the sweep derives from the knee land on data distances, so the
  rule, not exact arithmetic, decides those cells); the clusters, the noise, the stability are ADR 0238's and 0566's.
"""

import math
from fractions import Fraction as F

import spatial_inference_cases as si
import spatial_stability_cases as st

LAYER = "kaza"
STUDY = "calisma"
STUDY_WORDS = "Çalışma alanı nesnesi"
DRAWS = 10_000
EDGE = F(1, 10 ** 10)  # (10⁻⁵ m)²: a place or candidate nearer an edge is refused


# ── The study area (§1) ───────────────────────────────────────────────


def xy(p):
    return (F(p["x"]), F(p["y"]))


def areas_of(entities):
    """Each object's areas in the core's order: (outer ring, holes), the first part then `parts`; rings as written."""
    out = []
    for e in entities:
        assert e["kind"] == "polygon"
        parts = [e] + list(e.get("parts") or [])
        for q in parts:
            out.append(([xy(p) for p in q["pts"]], [[xy(p) for p in h["pts"]] for h in q.get("holes") or []]))
    return out


def edges_of(ring):
    return [(ring[i], ring[(i + 1) % len(ring)]) for i in range(len(ring))]


def edge_dist2(p, a, b):
    dx, dy = b[0] - a[0], b[1] - a[1]
    t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)
    t = max(F(0), min(F(1), t))
    return (p[0] - a[0] - t * dx) ** 2 + (p[1] - a[1] - t * dy) ** 2


def in_ring(ring, p):
    """Crossing number, exact; a point off every edge."""
    inside = False
    for a, b in edges_of(ring):
        if (a[1] > p[1]) != (b[1] > p[1]) and p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]):
            inside = not inside
    return inside


def in_region(areas, p, what):
    for outer, holes in areas:
        for ring in [outer] + holes:
            for a, b in edges_of(ring):
                if edge_dist2(p, a, b) < EDGE:
                    raise si.Refused(f"{what}: bir kenara 10⁻⁵ m'den yakın")
    return any(in_ring(outer, p) and not any(in_ring(h, p) for h in holes) for outer, holes in areas)


def union_area(areas):
    """The region's net area, exact: cells of the rectangles' coordinates, each in or out by its centre."""
    for outer, holes in areas:
        for ring in [outer] + holes:
            assert all(a[0] == b[0] or a[1] == b[1] for a, b in edges_of(ring)), "eksenlere paralel olmayan kenar"
    xs = sorted({p[0] for outer, holes in areas for ring in [outer] + holes for p in ring})
    ys = sorted({p[1] for outer, holes in areas for ring in [outer] + holes for p in ring})
    total = F(0)
    for x0, x1 in zip(xs, xs[1:]):
        for y0, y1 in zip(ys, ys[1:]):
            centre = ((x0 + x1) / 2, (y0 + y1) / 2)
            if any(in_ring(outer, centre) and not any(in_ring(h, centre) for h in holes) for outer, holes in areas):
                total += (x1 - x0) * (y1 - y0)
    return total


def union_perimeter(areas):
    """The length of the region's boundary, holes too, exact: the cells' edges between in and out (Donnelly's P)."""
    xs = sorted({p[0] for outer, holes in areas for ring in [outer] + holes for p in ring})
    ys = sorted({p[1] for outer, holes in areas for ring in [outer] + holes for p in ring})

    def inside(i, j):
        if i < 0 or j < 0 or i >= len(xs) - 1 or j >= len(ys) - 1:
            return False
        c = ((xs[i] + xs[i + 1]) / 2, (ys[j] + ys[j + 1]) / 2)
        return any(in_ring(outer, c) and not any(in_ring(h, c) for h in holes) for outer, holes in areas)

    cells = {(i, j): inside(i, j) for i in range(-1, len(xs)) for j in range(-1, len(ys))}
    total = F(0)
    for i in range(-1, len(xs) - 1):
        for j in range(len(ys) - 1):
            if cells[(i, j)] != cells[(i + 1, j)]:
                total += ys[j + 1] - ys[j]
    for j in range(-1, len(ys) - 1):
        for i in range(len(xs) - 1):
            if cells[(i, j)] != cells[(i, j + 1)]:
                total += xs[i + 1] - xs[i]
    return total


def study_lines(areas):
    text = ""
    for k, (outer, holes) in enumerate(areas):
        for r, ring in enumerate([outer] + holes):
            for x, y in ring:
                text += f"a{k}.{r}\t{si.fmt(x, 6)}\t{si.fmt(y, 6)}\n"
    return text


def bounds_of(areas):
    xs = [float(p[0]) for outer, _ in areas for p in outer]
    ys = [float(p[1]) for outer, _ in areas for p in outer]
    return min(xs), min(ys), max(xs) - min(xs), max(ys) - min(ys)


# ── En yakın komşu over a study area (§1) ─────────────────────────────


def nearest_study(objs, study_entities, count=0, seed=1, scope=("layer", LAYER), edge=False):
    areas = areas_of(study_entities)
    area = union_area(areas)
    perimeter = union_perimeter(areas)
    exact_all = [si.place(e) for e in objs]
    kept = [i for i, p in enumerate(exact_all) if in_region(areas, p, f"nesne {i}")]
    outside = len(objs) - len(kept)
    warnings = [f"{outside} nesnenin yeri çalışma alanının dışında; alınmadı."] if outside else []
    n = len(kept)
    if n < 2:
        return {"refused": f"En yakın komşu çalışma alanında en az iki nesne ister; {n} nesnenin yeri alanın içinde."}
    kobjs = [objs[i] for i in kept]
    placed = st.worked_places(kobjs)
    exact = [exact_all[i] for i in kept]
    observed = F(0)
    for i, p in enumerate(exact):
        observed += si.sqrt(min(si.d2(p, q) for j, q in enumerate(exact) if j != i))
    observed /= n
    expected, se = st.expectation(n, area, perimeter, edge)
    ratio = observed / expected
    z = (observed - expected) / se
    p = si.p_value(z)
    extra, extra_infos, extra_warnings = st.donnelly_rows(n, perimeter, edge)
    warnings = warnings + extra_warnings
    rows = [["Nesne sayısı", str(n)], ["Gözlenen ortalama uzaklık (m)", si.fmt(observed, 3)],
            ["Beklenen ortalama uzaklık (m)", si.fmt(expected, 3)], ["En yakın komşu oranı", si.fmt(ratio, 4)],
            ["z", si.fmt(z, 4)], ["p", si.fmt(p, 6)], ["Alan (m²)", si.fmt(area, 2)]] + extra + [
            ["Çalışma alanı", f"{len(areas)} alan, {sum(len(h) for _, h in areas)} delik"],
            ["Alanın dışında kalan nesne", str(outside)]]
    if count == 0:
        pattern = ("Kümelenmiş" if z < 0 else "Dağınık") if p < 0.05 else "Rastgele"
        rows.append(["Desen", pattern])
        edged = "Donnelly kenar düzeltmesiyle " if edge else ""
        summary = f"En yakın komşu oranı {si.fmt(ratio, 4)} ({edged}z {si.fmt(z, 4)}, p {si.fmt(p, 6)}): {pattern.lower()}."
        return {"table": {"columns": ["Ölçü", "Değer"], "rows": rows}, "summary": summary, "infos": extra_infos,
                "warnings": warnings}
    x0, y0, w, h = bounds_of(areas)
    share = float(area) / (w * h)
    assert count * n / share <= 5e8
    bound = si.sqrt(F(w) ** 2 + F(h) ** 2)
    first = kobjs[0]["p"]  # the first kept place: its own double is the working origin
    origin = (float(first["x"]), float(first["y"]))
    seq = si.Sequence(seed)
    sims, larger = [], 0
    for _ in range(count):
        thrown = []
        for _ in range(n):
            for _ in range(DRAWS):
                u, v = st.unit(seq), st.unit(seq)
                cand = (x0 + u * w, y0 + v * h)
                if in_region(areas, (F(cand[0]), F(cand[1])), "aday"):
                    thrown.append((cand[0] - origin[0], cand[1] - origin[1]))
                    break
            else:
                raise si.Refused("alanda aday bulunamadı")
        m = st.mean_nearest(thrown)
        si.near(m, observed, bound, "benzetimin uzaklığı gözlenene")
        if m >= observed:
            larger += 1
        sims.append(m)
    mean, sd = si.moments(sims)
    ps, _ = si.folded(larger, count)
    zs = (observed - mean) / sd
    se_p = math.sqrt(ps * (1.0 - ps) / count)
    pattern = ("Kümelenmiş" if zs < 0 else "Dağınık") if ps < 0.05 else "Rastgele"
    rows += [["Benzetim sayısı", str(count)], ["Tohum", str(seed)], ["Benzetim alanı", STUDY_WORDS],
             ["Benzetimlerin ortalama uzaklığı (m)", si.fmt(mean, 3)], ["Benzetim oranı", si.fmt(observed / mean, 4)],
             ["Benzetim z", si.fmt(zs, 4)], ["Benzetim p", si.fmt(ps, 6)], ["p'nin standart hatası", si.fmt(se_p, 6)],
             ["Desen", pattern]]
    summary = (f"En yakın komşu oranı {si.fmt(ratio, 4)} (benzetim z {si.fmt(zs, 4)}, p {si.fmt(ps, 6)}; "
               f"{count} rastgele desen, tohum {seed}): {pattern.lower()}.")
    prof_rows = [("Kural", si.RULE, "rule"), ("Araç", "En yakın komşu", "tool"),
                 ("Çıkarım", "Monte Carlo (rastgele desen)", "inference"), ("Benzetim sayısı", str(count), "simulations"),
                 ("Tohum", str(seed), "seed"), ("Sayı üreteci", si.GENERATOR, "generator"),
                 ("Benzetim alanı", f"{STUDY_WORDS}, {si.fmt(area, 2)} m²", "region"), ("Nesne sayısı", str(n), "objects")]
    if edge:
        prof_rows.append(("Kenar düzeltmesi", "Donnelly (1978)", "edge"))
    prof, digest = st.experiment(prof_rows, st.place_lines(objs, kept) + study_lines(areas), si.scope_label(scope[0], scope[1]))
    infos = extra_infos + [f"Deney profili ({si.RULE}): tohum {seed}, {count} rastgele desen; girdi özeti {digest[:12]}…."]
    return {"table": {"columns": ["Ölçü", "Değer"], "rows": rows}, "summary": summary, "infos": infos,
            "warnings": warnings + si.subset_warning(scope[0], len(objs)), "profile": prof, "pSimulation": ps, "area": float(area)}


# ── DBSCAN over a grid (§2) ───────────────────────────────────────────


def d2f(p, q):
    dx, dy = p[0] - q[0], p[1] - q[1]
    return dx * dx + dy * dy


def dbscan_float(pts, radius, min_points, border_noise):
    """ADR 0238 §10 with the module's neighbour rule: d = √(d²) ≤ ε in doubles, the place itself included."""
    lists = [[j for j, q in enumerate(pts) if math.sqrt(d2f(p, q)) <= radius] for p in pts]
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


def k_distances(pts, m):
    if m <= 1:
        return [0.0] * len(pts)
    out = []
    for i, p in enumerate(pts):
        ds = sorted(d2f(p, q) for j, q in enumerate(pts) if j != i)
        out.append(math.sqrt(ds[m - 2]))
    return sorted(out)


def knee(sorted_d, strict=True):
    n = len(sorted_d)
    d0, last = sorted_d[0], sorted_d[-1]
    if n < 3 or last == d0:
        return d0
    span = last - d0
    best, most, gs = 0, 0.0, []
    for i, d in enumerate(sorted_d):
        g = i / (n - 1) - (d - d0) / span
        gs.append(g)
        if g > most:
            best, most = i, g
    top = sorted(gs, reverse=True)
    assert not strict or top[0] - top[1] > 1e-9, "dirsek iki yerde aynı"
    return sorted_d[best]


def sweep(objs, first=3, last=8, step=1, radii=None, steps=7, border_noise=False, scheme=None, resamples=100, seed=1,
          scope=("layer", LAYER)):
    pts = [p for _, p in st.worked_places(objs)]
    n = len(pts)
    mins = list(range(first, last + 1, step))
    if len(mins) > 20:
        return {"refused": "En az nokta için en çok 20 değer denenir; Adım'ı büyütün ya da Son'u küçültün."}
    if mins[-1] > n:
        return {"refused": f"En az nokta ({mins[-1]}) nesne sayısından ({n}) büyük olamaz."}
    cells = len(mins) * steps
    assert cells <= 400
    aim = mins[0]
    for m in mins[1:]:
        a, b = abs(m - 4), abs(aim - 4)
        if a < b or (a == b and m > aim):
            aim = m
    graph = k_distances(pts, aim)
    eps = knee(graph)
    if radii is None:
        assert eps > 0
        r0, r1 = 0.5 * eps, 2.0 * eps
    else:
        r0, r1 = radii
    rs = [r0 + (r1 - r0) * j / (steps - 1) for j in range(steps)]
    suggested = 0
    for j, r in enumerate(rs):
        if abs(r - eps) < abs(rs[suggested] - eps):
            suggested = j
    rows, chosen = [], None
    for m in mins:
        for j, r in enumerate(rs):
            labels = dbscan_float(pts, r, m, border_noise)
            clusters = max(labels, default=0)
            noise = labels.count(0)
            share = 100.0 * noise / n
            row = [str(m), si.fmt(r, 3), str(clusters), str(noise), si.fmt(share, 1)]
            if scheme:
                size, means, *_ = st.stability(pts, labels, scheme, resamples, seed,
                                               lambda s, r=r, m=m: (dbscan_float(s, r, m, border_noise), 0, 0))
                got = [means[c] for c in range(1, len(size)) if size[c] > 0 and means[c] is not None]
                row.append("—" if not got else si.fmt(sum(got) / len(got), 3))
                row.append(str(sum(1 for g in got if g >= 0.75)))
            is_choice = m == aim and j == suggested
            row.append("◀ önerilen" if is_choice else "")
            if is_choice:
                chosen = (clusters, share)
            rows.append(row)
    clusters, share = chosen
    columns = ["En az nokta", "Yarıçap (m)", "Küme sayısı", "Gürültü", "Gürültü payı (%)"]
    if scheme:
        columns += ["Ortalama kararlılık", "Kararlı küme"]
    columns.append("Önerilen")
    summary = (f"{cells} çift denendi; önerilen: yarıçap {si.fmt(rs[suggested], 3)} m, en az {aim} nokta "
               f"({clusters} küme, %{si.fmt(share, 1)} gürültü); k-uzaklık dirseği {si.fmt(eps, 3)} m.")
    infos = ["Öneri bir sezgidir: en az nokta ızgaradaki 4'e en yakın değer (Schubert ve ark. 2017), yarıçap o en az nokta için sıralı "
             "k-uzaklık grafiğinin dirseğine (Ester ve ark. 1996; Satopää ve ark. 2011) en yakın ızgara değeridir."]
    out = {"table": {"columns": columns, "rows": rows}, "summary": summary, "infos": infos, "warnings": [], "cells": cells,
           "suggestedRadius": rs[suggested], "suggestedMinPoints": aim, "kneeRadius": eps, "clusters": clusters, "noiseShare": share,
           "chart": {"columns": ["Sıra", f"{aim - 1}. en yakın uzaklık (m)", "Dirsek"],
                     "rows": [[str(i + 1), si.fmt(d, 3), "◀ dirsek" if i == graph.index(eps) else ""]
                              for i, d in enumerate(graph)]}}
    if scheme:
        words, noun = st.scheme_words(scheme)
        parameters = (f"Yarıçap {si.fmt(rs[0], 3)} – {si.fmt(rs[-1], 3)} m ({steps} adım), en az nokta {mins[0]} – {mins[-1]} "
                      f"(adım {step}){', sınır noktaları gürültü' if border_noise else ''}")
        prof_rows = [("Kural", si.RULE, "rule"), ("Araç", "DBSCAN parametre taraması", "tool"), ("Çıkarım", words, "inference"),
                     ("Tekrar sayısı", str(resamples), "resamples"), ("Tohum", str(seed), "seed"),
                     ("Sayı üreteci", si.GENERATOR, "generator"), ("Parametreler", parameters, "parameters"),
                     ("Nesne sayısı", str(n), "objects")]
        prof, digest = st.experiment(prof_rows, st.place_lines(objs, range(n)), si.scope_label(scope[0], scope[1]))
        infos.append(f"Deney profili ({si.RULE}): tohum {seed}, {resamples} tekrar; girdi özeti {digest[:12]}….")
        out["profile"] = prof
        out["warnings"] += si.subset_warning(scope[0], n)
    return out


# ── Cases ─────────────────────────────────────────────────────────────


def self_checks():
    # A square with a square hole, no edge effect to speak of at a large count: the simulations' mean meets Donnelly's edge
    # corrected expectation with the perimeter of the outer ring and the hole's (area 100² − 40²; 50 places).
    outer = [(F(0), F(0)), (F(100), F(0)), (F(100), F(100)), (F(0), F(100))]
    hole = [(F(30), F(30)), (F(30), F(70)), (F(70), F(70)), (F(70), F(30))]
    areas = [(outer, [hole])]
    assert union_area(areas) == 100 * 100 - 40 * 40
    assert union_perimeter(areas) == 4 * 100 + 4 * 40
    seq = si.Sequence(5)
    n, means = 50, []
    for _ in range(200):
        pts = []
        while len(pts) < n:
            c = (st.unit(seq) * 100.0, st.unit(seq) * 100.0)
            if in_region(areas, (F(c[0]), F(c[1])), "aday"):
                pts.append(c)
        means.append(float(st.mean_nearest(pts)))
    mean = sum(means) / len(means)
    area, perimeter = 100 * 100 - 40 * 40, 4 * 100 + 4 * 40
    donnelly = 0.5 * math.sqrt(area / n) + (0.0514 + 0.041 / math.sqrt(n)) * perimeter / n
    assert abs(mean - donnelly) < 0.03 * donnelly, (mean, donnelly)
    # Overlapping rectangles count once.
    a = [(F(0), F(0)), (F(10), F(0)), (F(10), F(10)), (F(0), F(10))]
    b = [(F(5), F(5)), (F(15), F(5)), (F(15), F(15)), (F(5), F(15))]
    assert union_area([(a, []), (b, [])]) == 100 + 100 - 25
    assert union_perimeter([(a, []), (b, [])]) == 60
    # The knee of a flat-then-steep graph is the last flat distance; of a straight ramp, the first.
    assert knee([1.0] * 8 + [5.0, 9.0]) == 1.0
    assert knee([float(i) for i in range(10)], strict=False) == 0.0
    # Two far groups of five: DBSCAN finds both at a radius that holds each; the suggestion's knee is within a group.
    group = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0), (0.5, 0.5)]
    two = group + [(100.0 + x, 100.0 + y) for x, y in group]
    assert max(dbscan_float(two, 2.0, 3, False)) == 2
    assert dbscan_float(two, 0.4, 3, False) == [0] * 10


def cases(case):
    """The ADR's cases on the drawing; `case` the caller's writer for the plain cases (id, title, tool, values, run, selection)."""
    kaza = si.objects(LAYER)
    study = si.objects(STUDY)
    parsel = si.objects("parsel")
    lay = {"scope": "layer", "layerId": LAYER}
    region = {"region": "areas", "studyArea": {"scope": "layer", "layerId": STUDY}}
    out = []
    out.append(case("nn-study", "En yakın komşu, çalışma alanı iki alan nesnesi: delik ve alanın dışındakiler söylenir, A birleşimin net alanı",
                    "stats.nearestNeighbor", {"input": lay, **region}, nearest_study(kaza, study)))
    out.append(case("nn-study-sim", "En yakın komşu, çalışma alanında 199 rastgele desen, tohum 3: noktalar bölgede atılır, profilde bölgenin köşeleri",
                    "stats.nearestNeighbor", {"input": lay, **region, "inference": "simulation", "simulations": 199, "seed": 3},
                    nearest_study(kaza, study, 199, 3)))
    out.append(case("nn-study-area-ignored", "En yakın komşu, çalışma alanı seçiliyken yazılan Alan yok sayılır",
                    "stats.nearestNeighbor", {"input": lay, **region, "area": 5000}, nearest_study(kaza, study)))
    first = [study[0]]
    out.append(case("nn-study-selection", "En yakın komşu, çalışma alanı seçili tek nesne (delikli iki parça): daha küçük bölge, daha çok dışarıda",
                    "stats.nearestNeighbor", {"input": lay, "region": "areas", "studyArea": {"scope": "selection"}},
                    nearest_study(kaza, first), [first[0]["id"]]))
    out.append(case("nn-study-none", "En yakın komşu, çalışma alanı Parseller: kazaların hiçbiri alanda değil, ret",
                    "stats.nearestNeighbor", {"input": lay, "region": "areas", "studyArea": {"scope": "layer", "layerId": "parsel"}},
                    nearest_study(kaza, parsel)))
    out.append(case("nn-study-points", "En yakın komşu, çalışma alanı olarak nokta katmanı: alan yok",
                    "stats.nearestNeighbor", {"input": lay, "region": "areas", "studyArea": {"scope": "layer", "layerId": LAYER}},
                    {"invalid": [{"param": "studyArea",
                                  "message": "“Çalışma alanı nesneleri”: bu katmanda uygun nesne yok. Başka bir katman seçin."}]}))
    # Donnelly's edge correction (docs/adr/0639 §2): the box, the box scaled to an area, the study area.
    don = {"edge": "donnelly"}
    out.append(case("nn-donnelly", "En yakın komşu, Donnelly kenar düzeltmesi: beklenen uzaklık kutunun çevresiyle büyür, oran küçülür",
                    "stats.nearestNeighbor", {"input": lay, **don}, st.nearest_simulated(kaza, count=0, edge=True)))
    out.append(case("nn-donnelly-area", "En yakın komşu, Donnelly, alan 40 000 m²: çevre alanla ölçeklenmiş kutunun",
                    "stats.nearestNeighbor", {"input": lay, **don, "area": 40000},
                    st.nearest_simulated(kaza, area=40000.0, count=0, edge=True)))
    out.append(case("nn-donnelly-sim", "En yakın komşu, Donnelly ve 99 rastgele desen, tohum 3: benzetim değişmez, profilde kenar düzeltmesi",
                    "stats.nearestNeighbor", {"input": lay, **don, "inference": "simulation", "simulations": 99, "seed": 3},
                    st.nearest_simulated(kaza, count=99, seed=3, edge=True)))
    sel5 = [e["id"] for e in kaza[:5]]
    out.append(case("nn-donnelly-few", "En yakın komşu, Donnelly, seçili 5 kaza: 7'den az nesne söylenir",
                    "stats.nearestNeighbor", {"input": {"scope": "selection"}, **don},
                    st.nearest_simulated(kaza[:5], count=0, scope=("selection", None), edge=True), sel5))
    sel16 = [e["id"] for e in kaza[:16]]
    out.append(case("nn-donnelly-subset", "En yakın komşu, Donnelly, seçili 16 kaza ve 49 rastgele desen: alt küme diye söylenir",
                    "stats.nearestNeighbor", {"input": {"scope": "selection"}, **don, "inference": "simulation", "simulations": 49},
                    st.nearest_simulated(kaza[:16], count=49, scope=("selection", None), edge=True), sel16))
    out.append(case("nn-donnelly-study", "En yakın komşu, Donnelly, çalışma alanı: P birleşimin bütün halkaları, deliğinki de",
                    "stats.nearestNeighbor", {"input": lay, **region, **don}, nearest_study(kaza, study, edge=True)))
    out.append(case("nn-donnelly-study-one", "En yakın komşu, Donnelly, çalışma alanı seçili tek nesne (delikli iki parça)",
                    "stats.nearestNeighbor", {"input": lay, "region": "areas", "studyArea": {"scope": "selection"}, **don},
                    nearest_study(kaza, first, edge=True), [first[0]["id"]]))
    out.append(case("nn-donnelly-study-sim", "En yakın komşu, Donnelly, çalışma alanında 49 rastgele desen, tohum 4",
                    "stats.nearestNeighbor", {"input": lay, **region, **don, "inference": "simulation", "simulations": 49, "seed": 4},
                    nearest_study(kaza, study, 49, 4, edge=True)))
    out.append(case("sweep-auto", "DBSCAN taraması, kendiliğinden aralık: en az nokta 3 … 8, yarıçap önerilenin yarısından iki katına 7 adım",
                    "stats.dbscanSweep", {"input": lay}, sweep(kaza)))
    out.append(case("sweep-range", "DBSCAN taraması, yarıçap 4 … 14 m 6 adım, en az nokta 2, 6, 10 (4'e eşit uzaklıkta: büyük olan), DBSCAN*",
                    "stats.dbscanSweep", {"input": lay, "minFirst": 2, "minLast": 10, "minStep": 4, "radiusFirst": 4, "radiusLast": 14,
                                          "radiusSteps": 6, "borderNoise": True},
                    sweep(kaza, 2, 10, 4, (4.0, 14.0), 6, True)))
    out.append(case("sweep-stability", "DBSCAN taraması, en az nokta 4 ve 5, 3 yarıçap, 20 bootstrap tekrarı, tohum 2: kararlılık sütunları ve deney profili",
                    "stats.dbscanSweep", {"input": lay, "minFirst": 4, "minLast": 5, "radiusSteps": 3, "stability": "bootstrap",
                                          "resamples": 20, "seed": 2},
                    sweep(kaza, 4, 5, 1, None, 3, False, "bootstrap", 20, 2)))
    out.append(case("sweep-subset", "DBSCAN taraması, en az nokta 3 ve 4, 3 yarıçap, 30 alt örnek tekrarı, tohum 5: yarısı yerine koymadan",
                    "stats.dbscanSweep", {"input": lay, "minFirst": 3, "minLast": 4, "radiusSteps": 3, "stability": "subset",
                                          "resamples": 30, "seed": 5},
                    sweep(kaza, 3, 4, 1, None, 3, False, "subset", 30, 5)))
    out.append(case("sweep-many", "DBSCAN taraması, en az nokta 3 … 30: en çok 20 değer, ret", "stats.dbscanSweep",
                    {"input": lay, "minLast": 30}, sweep(kaza, 3, 30)))
    out.append(case("sweep-too-big", "DBSCAN taraması, en az nokta 30 > 27 nesne: ret", "stats.dbscanSweep",
                    {"input": lay, "minLast": 30, "minStep": 9}, sweep(kaza, 3, 30, 9)))
    out.append(case("sweep-half-range", "DBSCAN taraması, yarıçapın yalnız İlk'i yazılmış: ret", "stats.dbscanSweep",
                    {"input": lay, "radiusFirst": 5},
                    {"refused": "Yarıçap: İlk ve Son birlikte yazılır ya da ikisi de boş bırakılır (otomatik aralık)."}))
    out.append(case("sweep-steps", "DBSCAN taraması, 1 yarıçap adımı: alan en az 2 ister", "stats.dbscanSweep",
                    {"input": lay, "radiusSteps": 1},
                    {"invalid": [{"param": "radiusSteps", "message": "“Yarıçap adım sayısı” en az 2 olmalı."}]}))
    return out
