#!/usr/bin/env python3
"""The shared cases of `cad.pointclouds.register` (Tarama kaydı denetimi; plan/map-cad.md MAP-13, docs/adr/1800),
without KentOS code.

    python3 scripts/fixtures/pointcloud_register_cases.py           # writes the clouds and the cases
    python3 scripts/fixtures/pointcloud_register_cases.py --check   # writes nothing; compares

The clouds are text clouds written here (fixtures/pointcloud/v1/files/kayit-*.xyz, 3 decimals) of one ground: a
slope, a mound, a ridge across it and a gable roof, sampled twice at random with ±1 cm of noise. The second sampling
is moved by a known rigid transform (ω 0.02°, φ −0.03°, κ 0.05° about the area's middle, then 0.30, −0.20, 0.06 m),
a scan registered badly. Two flat samplings elsewhere, the second moved 0.30, −0.20, 0.05 m, show a ground that fixes
the height only.

The rules, by hand (numpy for the eigenvalues; every nearest search a full one):
- the points whose plan place lies where the clouds' boxes meet, widened by the reach; one per voxel
  (floor(x/s), floor(y/s), floor(z/s)), its points' mean; voxels in key order;
- the centre c: the fixed voxel points' mean; a fixed point's normal: the eigenvector of the smallest eigenvalue of the
  covariance (each term over k) of its k nearest fixed points (squared distance, then index), turned upward (else
  north, else east), none unless the middle eigenvalue is at least four times the smallest;
- a step: each moving point (now R(p − c) + c + t) paired with the nearest fixed point within the reach, if that one
  has a normal; r = (p − q)·n, a = [(p − c) × n, n]; x = −N⁺ Σ a r with N = Σ a aᵀ over its eigenvalues above 1e-12
  of the largest; R ← Rz(x₂)·Ry(x₁)·Rx(x₀)·R, t ← Rz…·t + x[3:]; it stops when max(|x[3:]|, max|x[:3]|·L) is under
  the tolerance (L the moving points' RMS distance from c) or after the iterations;
- after: the pairs, RMS and mean of r, the share paired; strength √(λmin/λmax) of N with the angle rows and columns
  over L; horizontal √(λmin(N[3:5, 3:5]) / N[5, 5]); plan cells (floor(x/cell), floor(y/cell)) of the moving points
  now, those with `least` pairs covered, those with RMS ≤ the package's swath-to-swath value reliable; the 20 worst
  (RMS down, then the key); around a control point each set's plane z = c₀ + c₁u + c₂v within the radius (Cramer, as
  cad.pointclouds.overlap's reference); KOH_Z's limit from BÖHHBÜY m. 91(2) by the package.
"""

import copy
import json
import math
import random
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.pointclouds.register.json"
FILES = ROOT / "fixtures" / "pointcloud" / "v1" / "files"
RULES = json.loads((ROOT / "catalog/v1/rules/lidar-uyum.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0


def uid(n):
    return f"0192f5c0-1800-7000-8000-{n:012x}"


def ground(x, y):
    z = 850.0 + 0.05 * x + 0.02 * y
    z += 1.5 * math.exp(-((x - 12.0) ** 2 + (y - 15.0) ** 2) / 30.0)
    d = (0.5 * x - y + 3.0) / math.sqrt(1.25)
    z += 0.8 * math.exp(-d * d / 2.0)
    if 20.0 < x < 26.0 and 6.0 < y < 14.0:
        z += 6.0 - 0.8 * abs(x - 23.0)
    return z


def rot(w):
    """Rz(κ)·Ry(φ)·Rx(ω)."""
    sa, ca = math.sin(w[0]), math.cos(w[0])
    sb, cb = math.sin(w[1]), math.cos(w[1])
    sg, cg = math.sin(w[2]), math.cos(w[2])
    return [[cg * cb, -sg * ca + cg * sb * sa, sg * sa + cg * sb * ca],
            [sg * cb, cg * ca + sg * sb * sa, -cg * sa + sg * sb * ca],
            [-sb, cb * sa, cb * ca]]


MIS_W = [math.radians(0.02), math.radians(-0.03), math.radians(0.05)]
MIS_T = [0.30, -0.20, 0.06]
MID = [X0 + 15.0, Y0 + 12.0, 852.0]


def scan(seed, moved, flat=False):
    rnd = random.Random(seed)
    r = rot(MIS_W)
    rows = []
    (x_from, x_to, y_to) = (100.0, 120.0, 15.0) if flat else (0.0, 30.0, 24.0)
    for _ in range(int((x_to - x_from) * y_to * 6)):
        x, y = rnd.uniform(x_from, x_to), rnd.uniform(0.0, y_to)
        z = (850.0 if flat else ground(x, y)) + rnd.uniform(-0.01, 0.01)
        p = [X0 + x, Y0 + y, z]
        if moved and not flat:
            d = [p[i] - MID[i] for i in range(3)]
            p = [sum(r[i][j] * d[j] for j in range(3)) + MID[i] + MIS_T[i] for i in range(3)]
        elif moved:
            p = [p[0] + 0.30, p[1] - 0.20, p[2] + 0.05]
        rows.append((round(p[0], 3), round(p[1], 3), round(p[2], 3)))
    return rows


CLOUDS = {"kayit-a.xyz": scan(1800, False), "kayit-b.xyz": scan(1801, True),
          "kayit-c.xyz": scan(1802, False, flat=True), "kayit-d.xyz": scan(1803, True, flat=True)}
IDS = {"kayit-a.xyz": 1, "kayit-b.xyz": 2, "kayit-c.xyz": 3, "kayit-d.xyz": 4}
# On the plain slope, away from the mound, the ridge and the roof: a plane over 2 m is the ground there.
CONTROL = [("K1", 27.2, 3.1), ("K2", 3.4, 21.7), ("K3", 28.1, 21.6), ("K4", 17.3, 2.2), ("K5", 2.3, 11.8)]


def text(rows):
    return "X Y Z\n" + "".join(f"{x:.3f} {y:.3f} {z:.3f}\n" for x, y, z in rows)


def bounds(rows):
    return [min(r[0] for r in rows), min(r[1] for r in rows), min(r[2] for r in rows),
            max(r[0] for r in rows), max(r[1] for r in rows), max(r[2] for r in rows)]


def cloud(id_, name, layer_="tarama"):
    rows = CLOUDS[name]
    b = bounds(rows)
    return {"kind": "pointcloud", "id": id_, "layerId": layer_, "attrs": {},
            "sources": [{"file": f"$fixture:pointcloud/v1/files/{name}", "format": "xyz", "count": len(rows),
                         "bounds": b}], "bounds": b, "count": len(rows), "srid": 5254,
            "style": {"render": "classification", "size": 2}}


def control_point(id_, name, x, y, z, layer_="gnss"):
    return {"kind": "point", "id": id_, "layerId": layer_, "attrs": {"Ad": name}, "p": {"x": X0 + x, "y": Y0 + y},
            "z": z}


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


def setup():
    ents = [cloud(IDS[n], n) for n in CLOUDS]
    for k, (n, x, y) in enumerate(CONTROL):
        ents.append(control_point(5 + k, n, x, y, round(ground(x, y), 3)))
    return {"format": "kentos.document", "version": 2, "name": "Tarama kaydı",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("tarama", "Taramalar"), layer("gnss", "GNSS kontrol noktaları")],
            "activeLayer": "tarama", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


class Sums:
    def __init__(self):
        self.n = self.u = self.v = self.uu = self.uv = self.vv = self.z = self.uz = self.vz = 0.0

    def add(self, u, v, z):
        self.n += 1
        self.u += u
        self.v += v
        self.uu += u * u
        self.uv += u * v
        self.vv += v * v
        self.z += z
        self.uz += u * z
        self.vz += v * z

    def height(self, least, h):
        if self.n < least:
            return None

        def det3(a):
            return (a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6])
                    + a[2] * (a[3] * a[7] - a[4] * a[6]))
        d = det3([self.n, self.u, self.v, self.u, self.uu, self.uv, self.v, self.uv, self.vv])
        if abs(d) <= 1e-9 * self.n ** 3 * h ** 4:
            return None
        return det3([self.z, self.u, self.v, self.uz, self.uu, self.uv, self.vz, self.uv, self.vv]) / d


def box_meets(a, b):
    return not (a[2] < b[0] or a[0] > b[2] or a[3] < b[1] or a[1] > b[3])


def sub(a, b):
    return [a[0] - b[0], a[1] - b[1], a[2] - b[2]]


def dot(a, b):
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2]


def cross(a, b):
    return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]


def seq_sum(values):
    s = 0.0
    for v in values:
        s += v
    return s


def num(v):
    t = repr(float(v))
    return t[:-2] if t.endswith(".0") else t


def apply(r, t, p, c):
    d = sub(p, c)
    return [r[0][0] * d[0] + r[0][1] * d[1] + r[0][2] * d[2] + c[0] + t[0],
            r[1][0] * d[0] + r[1][1] * d[1] + r[1][2] * d[2] + c[1] + t[1],
            r[2][0] * d[0] + r[2][1] * d[1] + r[2][2] * d[2] + c[2] + t[2]]


def nearest(fixed_np, q):
    """The nearest fixed point to q: (d², index), ties by index."""
    d = fixed_np - np.asarray(q)
    d2 = d[:, 0] * d[:, 0] + d[:, 1] * d[:, 1] + d[:, 2] * d[:, 2]
    i = int(np.argmin(d2))
    return float(d2[i]), i


def pairs_of(moving, r, t, c, fixed, fixed_np, normals, reach):
    out = []
    for m in moving:
        at = apply(r, t, m, c)
        if not len(fixed):
            continue
        d2, i = nearest(fixed_np, at)
        if d2 > reach * reach or normals[i] is None:
            continue
        out.append((at, i, dot(sub(at, fixed[i]), normals[i])))
    return out


def normal_matrix(ps, normals, c):
    m = [[0.0] * 6 for _ in range(6)]
    v = [0.0] * 6
    for at, i, r in ps:
        n = normals[i]
        w = cross(sub(at, c), n)
        a = [w[0], w[1], w[2], n[0], n[1], n[2]]
        for p in range(6):
            v[p] += a[p] * r
            for q in range(6):
                m[p][q] += a[p] * a[q]
    return m, v


def step(m, v):
    values, vectors = np.linalg.eigh(np.array(m))
    top = values[5]
    x = [0.0] * 6
    if top <= 0.0:
        return x
    for k in range(6):
        if values[k] <= 1e-12 * top:
            continue
        along = float(np.dot(vectors[:, k], v)) / values[k]
        for i in range(6):
            x[i] -= vectors[i, k] * along
    return x


def register(names, spacing=0.5, reach=1.0, iterations=30, tolerance=0.0001, neighbours=8, cell=2.0, least=4,
             classes=None, control=None, radius=None, scale=1000, rules=None):
    rules = rules or RULES
    radius = radius if radius is not None else cell
    rows = [CLOUDS[n] for n in names]
    b = [bounds(r) for r in rows]
    boxes = [[x[0], x[1], x[3], x[4]] for x in b]
    meet = [max(boxes[0][0], boxes[1][0]) - reach, max(boxes[0][1], boxes[1][1]) - reach,
            min(boxes[0][2], boxes[1][2]) + reach, min(boxes[0][3], boxes[1][3]) + reach]
    read = [0, 0]
    sums = [{}, {}]
    for k in range(2):
        if not box_meets(meet, boxes[k]):
            continue
        read[k] = len(rows[k])
        if classes is not None and 0 not in classes:  # a text cloud's points are class 0
            continue
        for x, y, z in rows[k]:
            if x < meet[0] or x > meet[2] or y < meet[1] or y > meet[3]:
                continue
            key = (math.floor(x / spacing), math.floor(y / spacing), math.floor(z / spacing))
            e = sums[k].setdefault(key, [0.0, 0.0, 0.0, 0.0])
            e[0] += x
            e[1] += y
            e[2] += z
            e[3] += 1.0
    vox = [[[e[0] / e[3], e[1] / e[3], e[2] / e[3]] for _, e in sorted(s.items())] for s in sums]
    fixed, moving = vox
    nf = float(max(len(fixed), 1))
    c = [seq_sum(p[0] for p in fixed) / nf, seq_sum(p[1] for p in fixed) / nf, seq_sum(p[2] for p in fixed) / nf]
    fixed_np = np.array(fixed) if fixed else np.zeros((0, 3))
    k = min(neighbours, len(fixed))
    normals = []
    for p in fixed:
        if k < 3:
            normals.append(None)
            continue
        d = fixed_np - np.asarray(p)
        d2 = d[:, 0] * d[:, 0] + d[:, 1] * d[:, 1] + d[:, 2] * d[:, 2]
        near = np.lexsort((np.arange(len(fixed)), d2))[:k]
        kk = float(len(near))
        mean = [0.0, 0.0, 0.0]
        for i in near:
            q = fixed[i]
            mean = [mean[0] + q[0], mean[1] + q[1], mean[2] + q[2]]
        mean = [mean[0] / kk, mean[1] / kk, mean[2] / kk]
        cov = [[0.0] * 3 for _ in range(3)]
        for i in near:
            dd = sub(fixed[i], mean)
            for a in range(3):
                for bb in range(3):
                    cov[a][bb] += dd[a] * dd[bb] / kk
        values, vectors = np.linalg.eigh(np.array(cov))
        if values[1] > 0.0 and values[1] >= 4.0 * max(values[0], 0.0):
            n = [float(vectors[0, 0]), float(vectors[1, 0]), float(vectors[2, 0])]
            if n[2] < 0.0 or (n[2] == 0.0 and (n[1] < 0.0 or (n[1] == 0.0 and n[0] < 0.0))):
                n = [-n[0], -n[1], -n[2]]
            normals.append(n)
        else:
            normals.append(None)
    if moving:
        rr = math.sqrt(seq_sum(dot(sub(m, c), sub(m, c)) for m in moving) / len(moving))
        L = rr if rr > 0.0 else 1.0
    else:
        L = 1.0
    r = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    t = [0.0, 0.0, 0.0]
    done, converged, rms_before = 0, False, 0.0
    for it in range(iterations):
        ps = pairs_of(moving, r, t, c, fixed, fixed_np, normals, reach)
        if len(ps) < 6:
            break
        if it == 0:
            rms_before = math.sqrt(seq_sum(p[2] * p[2] for p in ps) / len(ps))
        m, v = normal_matrix(ps, normals, c)
        x = step(m, v)
        s = rot(x[:3])
        r = [[s[i][0] * r[0][j] + s[i][1] * r[1][j] + s[i][2] * r[2][j] for j in range(3)] for i in range(3)]
        st = [dot(s[0], t), dot(s[1], t), dot(s[2], t)]
        t = [st[0] + x[3], st[1] + x[4], st[2] + x[5]]
        done += 1
        moved = max(abs(x[3]), abs(x[4]), abs(x[5]), max(abs(x[0]), abs(x[1]), abs(x[2])) * L)
        if moved < tolerance:
            converged = True
            break
    last = pairs_of(moving, r, t, c, fixed, fixed_np, normals, reach)
    np_ = float(len(last))
    if last:
        rms_after = math.sqrt(seq_sum(p[2] * p[2] for p in last) / np_)
        mean_after = seq_sum(p[2] for p in last) / np_
    else:
        rms_after = mean_after = 0.0
    m, _ = normal_matrix(last, normals, c)
    f = [1.0 / L] * 3 + [1.0] * 3
    scaled = [[m[i][j] * (f[i] * f[j]) for j in range(6)] for i in range(6)]
    values = np.linalg.eigvalsh(np.array(scaled))
    strength = math.sqrt(max(values[0], 0.0) / values[5]) if values[5] > 0.0 else 0.0
    a, bb, d = m[3][3], m[3][4], m[4][4]
    plan_min = (a + d) / 2.0 - math.sqrt(((a - d) / 2.0) ** 2 + bb * bb)
    horizontal = math.sqrt(max(plan_min, 0.0) / m[5][5]) if m[5][5] > 0.0 else 0.0
    cells = {}
    for at, _, rr_ in last:
        e = cells.setdefault((math.floor(at[0] / cell), math.floor(at[1] / cell)), [0.0, 0])
        e[0] += rr_ * rr_
        e[1] += 1
    held = [(key, math.sqrt(e[0] / e[1]), e[1]) for key, e in sorted(cells.items()) if e[1] >= least]
    covered = len(held) * cell * cell
    reliable = sum(1 for h in held if h[1] <= rules["interswath"]) * cell * cell
    worst = sorted(held, key=lambda h: (-h[1], h[0]))[:20]
    controls = []
    if control is not None:
        for n_, x_, y_ in CONTROL:
            controls.append((n_, X0 + x_, Y0 + y_, round(ground(x_, y_), 3)))

    def plane(points, cx, cy):
        s_ = Sums()
        for p in points:
            u, v = p[0] - cx, p[1] - cy
            if abs(u) <= radius and abs(v) <= radius and math.hypot(u, v) <= radius:
                s_.add(u, v, p[2])
        return s_.height(max(least, 3), radius)
    ctrl = []
    now_moving = [apply(r, t, mm, c) for mm in moving]
    for n_, cx, cy, z in controls:
        o = {"name": n_, "z": z}
        for key, pts in (("dzFixed", fixed), ("dzBefore", moving), ("dzAfter", now_moving)):
            h = plane(pts, cx, cy)
            if h is not None:
                o[key] = h - z
        ctrl.append(o)
    S = float(scale)
    limit_z = rules["zFactor"] * (rules["xy"][0] * S * S + rules["xy"][1] * S + rules["xy"][2]) / 100.0

    def stats(key):
        if not controls:
            return None
        v_ = [o[key] for o in ctrl if key in o]
        koh = math.sqrt(sum(dz * dz for dz in v_) / len(v_)) if v_ else 0.0
        out_ = {"n": len(v_), "kohZ": koh, "mean": sum(v_) / len(v_) if v_ else 0.0}
        if v_:
            out_["passed"] = koh <= limit_z
        return out_
    rotation = [math.atan2(r[2][1], r[2][2]), math.asin(min(1.0, max(-1.0, -r[2][0]))), math.atan2(r[1][0], r[0][0])]
    rc = [dot(r[0], c), dot(r[1], c), dot(r[2], c)]
    t0 = [c[0] + t[0] - rc[0], c[1] + t[1] - rc[1], c[2] + t[2] - rc[2]]
    matrix = [r[0][0], r[0][1], r[0][2], t0[0], r[1][0], r[1][1], r[1][2], t0[1], r[2][0], r[2][1], r[2][2], t0[2],
              0.0, 0.0, 0.0, 1.0]
    notes = ["Dönüşüm hareketli buluta uygulanmadı; bulut yerinde durur. Tarif: p ↦ R(p − merkez) + merkez + öteleme, "
             "R = Rz(κ)·Ry(φ)·Rx(ω).",
             "RMS nokta-düzlem uzaklığıdır ve yalnız erişim içinde eşleşen noktalardan hesaplanır; eşleşmeyen kısım "
             "örtüşme payında görünür.",
             f"Güvenilir kapsamın sınırı {rules['id']} paketinin şeritler arası örnek değeridir; kurum kendi değerini "
             "verir."]
    if not last:
        notes.append("Eşleşen nokta yok: bulutlar erişim içinde örtüşmüyor; erişimi büyütün ya da bulutları "
                     "yaklaştırın.")
    elif horizontal < 0.1:
        notes.append("Geometri yatay ötelemeyi zayıf tutuyor (düz zemin): kayıt düşeyde anlamlı, yatayda değil; yatay "
                     "için eğimli yüzeyler, yapılar ya da kontrol noktaları gerekir.")
    if not converged and done == iterations:
        notes.append("Yineleme sınırına varıldı; son adım toleransın üstünde kaldı.")
    if not controls:
        notes.append("Kontrol noktası yok: kaydın mutlak doğruluğu bilinmiyor; bulut ölçüye kesin hazır sayılmaz.")
    out = {"rules": rules["id"], "source": rules["source"], "read": read, "used": [len(fixed), len(moving)],
           "iterations": done, "converged": converged, "centre": c, "rotation": rotation, "translation": t,
           "matrix": matrix, "pairs": len(last), "overlap": (np_ / len(moving)) if moving else 0.0,
           "rmsBefore": rms_before, "rmsAfter": rms_after, "meanAfter": mean_after, "strength": strength,
           "horizontal": horizontal, "cell": cell, "covered": covered, "reliable": reliable,
           "limit": rules["interswath"],
           "worst": [{"x": (i + 0.5) * cell, "y": (j + 0.5) * cell, "n": n_, "rms": rms} for (i, j), rms, n_ in worst],
           "control": ctrl}
    for key, name in (("dzFixed", "controlFixed"), ("dzBefore", "controlBefore"), ("dzAfter", "controlAfter")):
        s_ = stats(key)
        if s_ is not None:
            out[name] = s_
    out["limitZ"] = limit_z
    out["notes"] = notes
    return out


SAME = {"revision": "same", "canUndo": False}
ARGS = ("spacing", "reach", "iterations", "tolerance", "neighbours", "cell", "least", "classes", "control", "radius",
        "scale", "rules")


def ok(name, clouds=("kayit-a.xyz", "kayit-b.xyz"), op="plan", within=1e-7, **kw):
    inp = {"uids": [uid(IDS[c]) for c in clouds], **kw}
    out = register(list(clouds), **{k: v for k, v in kw.items() if k in ARGS})
    return {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": within}]}


def refused(name, inp, code, message, path_):
    return {"name": name, "steps": [{"op": "execute", "input": inp, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path_}}, "expect": SAME}]}


def cases():
    base = register(["kayit-a.xyz", "kayit-b.xyz"], control="gnss")
    inv_t = base["translation"]
    assert base["converged"] and base["rmsAfter"] < base["rmsBefore"] / 3, (base["rmsBefore"], base["rmsAfter"])
    assert abs(inv_t[0] + 0.30) < 0.05 and abs(inv_t[1] - 0.20) < 0.05 and abs(inv_t[2] + 0.06) < 0.02, inv_t
    assert base["horizontal"] >= 0.1, base["horizontal"]
    flat = register(["kayit-c.xyz", "kayit-d.xyz"])
    assert flat["horizontal"] < 0.1 and abs(flat["translation"][2] + 0.05) < 0.01, (flat["horizontal"], flat["translation"])
    apart = register(["kayit-a.xyz", "kayit-c.xyz"])
    assert apart["pairs"] == 0 and apart["read"] == [0, 0], apart
    other = dict(copy.deepcopy(RULES), id="deneme-3cm", source="Deneme paketi (sentetik): 3 cm", interswath=0.03)
    two = [uid(1), uid(2)]
    return [
        ok("A'ya B: kaydırılmış tarama sabitine oturur; GNSS kontrolü önce ve sonra", control="gnss"),
        ok("uygula: salt okunur", op="execute", control="gnss"),
        ok("doğrulama", op="validate", control="gnss"),
        ok("kontrolsüz: mutlak doğruluk bilinmez notu"),
        # A flat ground leaves the plan parameters to the noise: 30 steps of an ill-conditioned solve, the matrix's
        # translation (R·c with c 4.4 million metres from the origin) agrees to 1e-5 m between implementations.
        ok("düz zemin: yükseklik bulunur, yatay öteleme zayıf", ("kayit-c.xyz", "kayit-d.xyz"), within=1e-5),
        ok("örtüşmeyen iki bulut: eşleşme yok", ("kayit-a.xyz", "kayit-c.xyz")),
        ok("tek yineleme: tolerans altına inmez", iterations=1),
        ok("1 m voksel", spacing=1.0),
        ok("dar erişim 0,2 m", reach=0.2),
        ok("yalnız sınıf 2: metin bulutu sınıf 0, nokta kalmaz", classes=[2]),
        ok("B'ye A: sıra ters dönüşümü verir", ("kayit-b.xyz", "kayit-a.xyz")),
        ok("başka kural paketi: güvenilir sınır 3 cm", rules=other),
        ok("16 komşulu normaller, 4 m kapsam hücresi", neighbours=16, cell=4.0),
        ok("1/500: KOH_Z sınırı daha sıkı, kontrol çevresi 1,5 m", control="gnss", scale=500, radius=1.5),
        refused("tek bulut", {"uids": [uid(1)]}, "invalid_input",
                "İki ayrı nokta bulutu verin: sabit tarama ve ona oturtulacak tarama.", "uids"),
        refused("aynı bulut iki kez", {"uids": [uid(1), uid(1)]}, "invalid_input",
                "İki ayrı nokta bulutu verin: sabit tarama ve ona oturtulacak tarama.", "uids"),
        refused("bulut değil", {"uids": [uid(1), uid(5)]}, "not_a_pointcloud",
                "Verilen nesne bir nokta bulutu değil; bir nokta bulutunun kimliğini verin.", "uids[1]"),
        refused("kimlik UUID değil", {"uids": [uid(1), "x"]}, "invalid_uid",
                "“x” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir "
                "(01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da "
                "çizimden alın.", "uids[1]"),
        refused("çizimde olmayan bulut", {"uids": [uid(1), uid(99)]}, "entity_not_found",
                f"“{uid(99)}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir "
                "nesnenin kimliğini verin.", "uids[1]"),
        refused("voksel 0", {"uids": two, "spacing": 0.0}, "invalid_input",
                f"Voksel kenarı sonlu ve sıfırdan büyük olmalı (yazılan {num(0.0)}).", "spacing"),
        refused("erişim eksi", {"uids": two, "reach": -1.0}, "invalid_input",
                f"Eşleşme erişimi sonlu ve sıfırdan büyük olmalı (yazılan {num(-1.0)}).", "reach"),
        refused("yineleme 0", {"uids": two, "iterations": 0}, "invalid_input",
                "Yineleme sayısı 1 ile 200 arasında olmalı.", "iterations"),
        refused("yineleme 201", {"uids": two, "iterations": 201}, "invalid_input",
                "Yineleme sayısı 1 ile 200 arasında olmalı.", "iterations"),
        refused("tolerans 0", {"uids": two, "tolerance": 0.0}, "invalid_input",
                f"Durma toleransı sonlu ve sıfırdan büyük olmalı (yazılan {num(0.0)}).", "tolerance"),
        refused("2 komşu", {"uids": two, "neighbours": 2}, "invalid_input",
                "Normal için komşu sayısı 3 ile 32 arasında olmalı.", "neighbours"),
        refused("kapsam hücresi 0", {"uids": two, "cell": 0.0}, "invalid_input",
                f"Kapsam hücresi sonlu ve sıfırdan büyük olmalı (yazılan {num(0.0)}).", "cell"),
        refused("hücrede 0 eşleşme", {"uids": two, "least": 0}, "invalid_input",
                "Bir kapsam hücresi için en az 1 eşleşme gerekir.", "least"),
        refused("kontrol çevresi 0", {"uids": two, "radius": 0.0}, "invalid_input",
                f"Kontrol noktasının çevresi sonlu ve sıfırdan büyük olmalı (yazılan {num(0.0)}).", "radius"),
        refused("ölçek 0", {"uids": two, "scale": 0}, "invalid_input", "Ölçek paydası 0 olamaz.", "scale"),
        refused("kural paketinin kaynağı boş", {"uids": two, "rules": dict(copy.deepcopy(RULES), source="")},
                "invalid_input", "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim "
                "yapılmaz.", "rules"),
        refused("kontrol katmanı yok", {"uids": two, "control": "yok"}, "layer_not_found",
                "“yok” katmanı çizimde yok.", "control"),
    ]


def main():
    check_only = "--check" in sys.argv[1:]
    stale = []
    for name, rows in CLOUDS.items():
        path = FILES / name
        t = text(rows)
        if check_only:
            if not path.exists() or path.read_text("utf-8") != t:
                stale.append(name)
        else:
            path.write_text(t, "utf-8")
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.pointclouds.register", "commandVersion": 1,
           "title": "Tarama kaydı denetimi",
           "note": "ADR 1800. Bağımsız başvuru scripts/fixtures/pointcloud_register_cases.py: voksel, normal, nokta-düzlem "
                   "ICP (Low 2004), güç, kapsam ve kontrol elle (numpy yalnız öz değerler için); bulutlar sentetik metin "
                   "bulutlarıdır; KentOS kodu yok.",
           "setup": setup(), "cases": cases()}
    out = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if check_only:
        ok_ = not stale and OUT.exists() and OUT.read_text("utf-8") == out
        print("Tarama kaydı ortak durumları güncel." if ok_ else f"güncel değil: {stale or OUT.relative_to(ROOT)}")
        return 0 if ok_ else 1
    OUT.write_text(out, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum), {len(CLOUDS)} bulut")
    return 0


if __name__ == "__main__":
    sys.exit(main())
