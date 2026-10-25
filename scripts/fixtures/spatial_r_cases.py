#!/usr/bin/env python3
"""Mekânsal istatistik R'la (docs/adr/0639): the independent reference's values (spatial_stability_cases and
spatial_study_cases, the rules KentOS's core and the shared cases follow) against R's own packages.

    python3 scripts/fixtures/spatial_r_cases.py --check

R with fpc, dbscan, spatstat and jsonlite: `KENTOS_RSCRIPT`, else `.run/r/bin/Rscript` (micromamba: r-base, r-fpc,
r-dbscan, r-spatstat.explore, r-jsonlite into `.run/r`), else `Rscript` on the path; without R the check says so and stops
with 0 (it is a cross-check, the cases are checked by spatial_inference_cases.py).

**Kararlılık (§1)**, Hennig's own fpc::clusterboot on the same resamples:

The resamples are the rule's draws (SplitMix64 and xoshiro256**, each object once); spatial_fpc.R hands them to
clusterboot through its `sample`. R clusters every resample itself: DBSCAN with the dbscan package (standard DBSCAN,
border points to the first cluster that reaches them, or noise), k-means with stats::kmeans (Lloyd, one start) from the
farthest-first seeds. What is compared, for every run on the drawing's 27 accidents:

- the partition of the whole layer (clusters as sets, noise the same);
- every resample's γ of every cluster, to 10⁻¹² (fpc's bootresult or subsetresult);
- every cluster's mean, its recovered (γ > 0.75) and dissolved (γ ≤ 0.5) counts.

**Donnelly (§2)**, spatstat's clarkevans on the windows of the shared cases (the places' box, the box scaled to 40 000 m², the
first 16 and 5 accidents' boxes): its observed mean distance, area and perimeter are the reference's, and its expectation is
the same formula with its coefficient: spatstat writes 0.0412 where Donnelly's text, as Dixon gives it, has 0.041, and keeps
Clark and Evans's error where KentOS takes Donnelly's. Both differences are reported, not hidden.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import spatial_inference_cases as si  # noqa: E402
import spatial_stability_cases as st  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
R_SCRIPT = Path(__file__).resolve().parent / "spatial_r.R"

# (id, kind, parameters, scheme, resamples, seed)
RUNS = [
    ("db-12-4-boot", "dbscan", {"radius": 12, "minPoints": 4}, "bootstrap", 100, 1),
    ("db-12-3-subset", "dbscan", {"radius": 12, "minPoints": 3}, "subset", 60, 7),
    ("db-12-3-boot", "dbscan", {"radius": 12, "minPoints": 3}, "bootstrap", 100, 2),
    ("db-8-3-boot", "dbscan", {"radius": 8, "minPoints": 3}, "bootstrap", 100, 3),
    ("db-8-3-subset", "dbscan", {"radius": 8, "minPoints": 3}, "subset", 100, 4),
    ("db-15-5-boot", "dbscan", {"radius": 15, "minPoints": 5}, "bootstrap", 80, 5),
    ("db-20-4-boot", "dbscan", {"radius": 20, "minPoints": 4}, "bootstrap", 50, 6),
    ("db-10-2-boot", "dbscan", {"radius": 10, "minPoints": 2}, "bootstrap", 100, 8),
    ("db-6-2-boot", "dbscan", {"radius": 6, "minPoints": 2}, "bootstrap", 200, 9),
    ("db-6-2-subset", "dbscan", {"radius": 6, "minPoints": 2}, "subset", 200, 10),
    ("db-12-4-border-boot", "dbscan", {"radius": 12, "minPoints": 4, "borderNoise": True}, "bootstrap", 100, 11),
    ("db-8-3-border-subset", "dbscan", {"radius": 8, "minPoints": 3, "borderNoise": True}, "subset", 100, 12),
    ("km-2-boot", "kmeans", {"k": 2}, "bootstrap", 100, 1),
    ("km-3-boot", "kmeans", {"k": 3}, "bootstrap", 100, 1),
    ("km-4-boot", "kmeans", {"k": 4}, "bootstrap", 100, 3),
    ("km-5-boot", "kmeans", {"k": 5}, "bootstrap", 50, 2),
    ("km-6-boot", "kmeans", {"k": 6}, "bootstrap", 100, 13),
    ("km-3-subset", "kmeans", {"k": 3}, "subset", 100, 14),
    ("km-5-subset", "kmeans", {"k": 5}, "subset", 100, 15),
    ("km-8-subset", "kmeans", {"k": 8}, "subset", 60, 16),
]


def rscript():
    for c in (os.environ.get("KENTOS_RSCRIPT"), str(ROOT / ".run/r/bin/Rscript"), shutil.which("Rscript")):
        if c and Path(c).exists():
            return c
    return None


def ours(pts, kind, par, scheme, resamples, seed):
    if kind == "dbscan":
        border = par.get("borderNoise", False)
        labels = st.dbscan_labels(pts, par["radius"], par["minPoints"], border)
        cluster = lambda s: (st.dbscan_labels(s, par["radius"], par["minPoints"], border), 0, 0)  # noqa: E731
    else:
        labels = st.k_means_labels(pts, par["k"])[0]
        cluster = lambda s: (st.k_means_labels(s, min(par["k"], len(set(s))))[0], 0, 0)  # noqa: E731
    trace = []
    _, means, _, recovered, dissolved, _, _ = st.stability(pts, labels, scheme, resamples, seed, cluster, trace)
    seq = si.Sequence(seed)
    lists = [st.resample(len(pts), scheme, seq) for _ in range(resamples)]
    return labels, means, recovered, dissolved, trace, lists


def windows(pts):
    """The shared cases' windows: (id, places, x0, x1, y0, y1, fixture case)."""
    out = []
    for wid, sub, area, cid in (("box", pts, None, "nn-donnelly"), ("area", pts, 40000.0, "nn-donnelly-area"),
                                ("first-16", pts[:16], None, None), ("first-5", pts[:5], None, "nn-donnelly-few")):
        xs, ys = [float(x) for x, _ in sub], [float(y) for _, y in sub]
        x0, x1, y0, y1 = min(xs), max(xs), min(ys), max(ys)
        if area is not None:
            w, h = x1 - x0, y1 - y0
            k = (area / (w * h)) ** 0.5
            cx, cy = x0 + w / 2, y0 + h / 2
            x0, x1, y0, y1 = cx - w * k / 2, cx + w * k / 2, cy - h * k / 2, cy + h * k / 2
        out.append((wid, sub, x0, x1, y0, y1, cid))
    return out


def donnelly(n, a, p, c):
    return 0.5 * (a / n) ** 0.5 + (0.0514 + c / n ** 0.5) * p / n


def sets(labels):
    k = max(labels, default=0)
    return [frozenset(i for i, l in enumerate(labels) if l == c) for c in range(1, k + 1)]


def main():
    check = "--check" in sys.argv
    r = rscript()
    if r is None:
        print("spatial_r_cases: R bulunamadı (KENTOS_RSCRIPT, .run/r/bin/Rscript ya da Rscript); atlandı.")
        return 0
    pts = [si.place(e) for e in si.objects(st.LAYER)]
    mine, runs = {}, []
    for rid, kind, par, scheme, resamples, seed in RUNS:
        labels, means, recovered, dissolved, trace, lists = ours(pts, kind, par, scheme, resamples, seed)
        mine[rid] = (labels, means, recovered, dissolved, trace)
        runs.append({"id": rid, "kind": kind, **par, "scheme": "boot" if scheme == "bootstrap" else "subset",
                     "lists": lists})
    with tempfile.TemporaryDirectory() as d:
        src, dst = Path(d) / "in.json", Path(d) / "out.json"
        wins = windows(pts)
        src.write_text(json.dumps({"points": [[float(x), float(y)] for x, y in pts], "runs": runs,
                                   "donnelly": [{"id": wid, "points": [[float(x), float(y)] for x, y in sub],
                                                 "x0": x0, "x1": x1, "y0": y0, "y1": y1}
                                                for wid, sub, x0, x1, y0, y1, _ in wins]}))
        done = subprocess.run([r, str(R_SCRIPT), str(src), str(dst)], capture_output=True, text=True)
        if done.returncode != 0:
            print(done.stderr, file=sys.stderr)
            raise SystemExit("spatial_r_cases: R çalışmadı")
        theirs = json.loads(dst.read_text())
    problems, compared = [], 0
    for rid, *_ in RUNS:
        labels, means, recovered, dissolved, trace = mine[rid]
        got = theirs[rid]
        a, b = sets(labels), sets([int(v) for v in got["partition"]])
        if set(a) != set(b):
            problems.append(f"{rid}: bölüntü farklı")
            continue
        for c, members in enumerate(a, start=1):
            rc = b.index(members)  # R's number of the same cluster
            rg = got["gammas"][rc]
            ours_g = [row[c - 1] for row in trace]
            worst = max(abs(x - y) for x, y in zip(ours_g, rg))
            if len(rg) != len(ours_g) or worst > 1e-12:
                problems.append(f"{rid}: küme {c} γ farkı {worst}")
            if abs(means[c] - got["means"][rc]) > 1e-12:
                problems.append(f"{rid}: küme {c} ortalama {means[c]} ≠ {got['means'][rc]}")
            if recovered[c] != got["recovered"][rc] or dissolved[c] != got["dissolved"][rc]:
                problems.append(f"{rid}: küme {c} sayılar {recovered[c]}/{dissolved[c]} ≠ "
                                f"{got['recovered'][rc]}/{got['dissolved'][rc]}")
            compared += len(rg)
        if not check:
            print(rid, " ".join(f"{m:.4f}" for m in means[1:]))
    cases = {c["id"]: c for c in json.loads(si.OUT.read_text("utf-8"))["cases"]}
    spread = []
    for wid, sub, x0, x1, y0, y1, cid in wins:
        got = {k: v[0] for k, v in theirs["donnelly"][wid].items()}
        n = len(sub)
        observed = float(sum(si.sqrt(min(si.d2(p, q) for j, q in enumerate(sub) if j != i)) for i, p in enumerate(sub)) / n)
        a, per = (x1 - x0) * (y1 - y0), 2 * ((x1 - x0) + (y1 - y0))
        for what, ours_v, theirs_v in (("gözlenen", observed, got["observed"]), ("alan", a, got["area"]),
                                       ("çevre", per, got["perimeter"]),
                                       ("beklenen (0.0412)", donnelly(n, a, per, 0.0412), got["expected"])):
            if abs(ours_v - theirs_v) > 1e-9 * max(1.0, abs(ours_v)):
                problems.append(f"{wid}: {what} {ours_v} ≠ spatstat {theirs_v}")
        ours_e = donnelly(n, a, per, 0.041)
        if cid is not None:
            rows = dict(map(tuple, cases[cid]["expect"]["outputs"]["table"]["rows"]))
            if rows["Beklenen ortalama uzaklık (m)"] != si.fmt(ours_e, 3) or rows["Çevre (m)"] != si.fmt(per, 2):
                problems.append(f"{wid}: ortak durumun satırları ({cid}) bu pencereninkiler değil")
        spread.append(abs(observed / ours_e - got["ratio"]) / got["ratio"])
    if problems:
        print("\n".join(problems))
        return 1
    print(f"spatial_r_cases: {len(RUNS)} çalıştırma, {compared} γ fpc {version(r, 'fpc')} ile aynı; Donnelly {len(wins)} pencerede "
          f"spatstat {version(r, 'spatstat.explore')} ile aynı (katsayı 0.041 ile 0.0412: oranlar en çok "
          f"%{100 * max(spread):.3f} ayrı).")
    return 0


def version(r, package):
    done = subprocess.run([r, "-e", f'cat(format(packageVersion("{package}")))'], capture_output=True, text=True)
    return done.stdout.strip() or "?"


if __name__ == "__main__":
    sys.exit(main())
