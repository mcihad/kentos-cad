#!/usr/bin/env python3
"""cad.nurbs.create's shared cases (docs/adr/0259 §3), from the ADR's rules, without KentOS code.

Writes fixtures/commands/v1/cad.nurbs.create.json; `--check` compares and writes nothing.

The curve the command builds is computed three ways:
- an IEEE double twin of the ADR's construction, every operation in the ADR's order (chord lengths as
  sqrt(dx·dx + dy·dy), running sums from the start, The NURBS Book's A2.1 span and A2.2 basis, the open system
  eliminated in its band without pivoting, the closed one with partial pivoting, the first largest pivot): its
  numbers are the cases' expected numbers, bit for bit (as ADR 0158's sheet_f64.py does for the rubber sheet);
- the same construction in 50-digit arithmetic (mpmath): the twin's control points must lie within 1e-9 of it
  (relative to the coordinates), so the twin is the construction, not a copy of a mistake;
- OpenCASCADE (tools/occt/occt_ref.cpp, its own B-spline evaluator): the stored curve passes through every fit
  point at its parameter, a clamped curve starts and ends at its end control points, and a closed curve meets
  itself with the same first and second derivatives. Without the tool (no OpenCASCADE) that part is skipped and
  said.

The rules, as ADR 0259 §3 gives them:
- `by.kind == "control"`: the degree 1 to 11; weights one per control point; an open curve's knots as given or
  clamped and uniform (0 p+1 times, 1, …, n−p−1, n−p p+1 times); a closed curve takes no knots, at least 3
  control points and no fewer than its degree, its first p control points (and weights) wrapped after the last
  and the knots 0, 1, …, n+2p; it is stored closed and periodic;
- `by.kind == "fit"`: the degree (3 when absent) 1 to 11, lowered to one less than the points; an open curve
  through 2 points or more, a closed one through 3 or more (a last point at the first's place is the first), at
  most 1000; no two points in a row at one place (a closed curve's last and first too); chord-length
  parameters, an open curve's knots averaged (Piegl and Tiller 9.8), a closed curve's knots periodic with period
  1 at the parameters (odd degree) or at their midpoints (even), the data parameters before the first knot moved
  by 1; the points are stored as its fit points;
- the checks in order: coordinates, weights and knots finite (control points, weights, knots; fit points); two
  fit points in a row at one place; the degree and counts; the contract's NURBS rule (knots never decreasing, a
  span, an inner knot at most p times, weights above 0); the line weight; the revision; the layer.
"""

import argparse
import copy
import json
import math
import os
import shutil
import subprocess
import sys
from pathlib import Path

from mpmath import lu_solve, matrix, mp, mpf, sqrt

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.nurbs.create.json"
SETUP_FROM = ROOT / "fixtures/commands/v1/cad.circle.create.json"
OCCT = ROOT / ".run/occt_ref"
OCCT_SRC = ROOT / "tools/occt/occt_ref.cpp"

MAX_DEGREE = 11
MAX_CLOSED_FIT = 1000
X0, Y0 = 487000, 4420000


# ── The double twin ─────────────────────────────────────────────────────────


def find_span(last, p, t, knots):
    if t >= knots[last + 1]:
        return last
    low, high = p, last + 1
    mid = (low + high) // 2
    while t < knots[mid] or t >= knots[mid + 1]:
        if t < knots[mid]:
            high = mid
        else:
            low = mid
        mid = (low + high) // 2
    return mid


def basis(span, t, p, knots, zero=0.0, one=1.0):
    n = [zero] * (p + 1)
    left = [zero] * (p + 1)
    right = [zero] * (p + 1)
    n[0] = one
    for j in range(1, p + 1):
        left[j] = t - knots[span + 1 - j]
        right[j] = knots[span + j] - t
        saved = zero
        for r in range(j):
            temp = n[r] / (right[r + 1] + left[j - r])
            n[r] = saved + right[r + 1] * temp
            saved = left[j - r] * temp
        n[j] = saved
    return n


def chord(a, b, root=math.sqrt):
    dx, dy = b[0] - a[0], b[1] - a[1]
    return root(dx * dx + dy * dy)


def open_params(q, root=math.sqrt, zero=0.0, one=1.0):
    n = len(q)
    d = [chord(q[k - 1], q[k], root) for k in range(1, n)]
    total = zero
    for x in d:
        total += x
    t = [zero] * n
    for k in range(1, n - 1):
        t[k] = t[k - 1] + d[k - 1] / total
    t[n - 1] = one
    return t


def open_knots(t, p, zero=0.0, one=1.0):
    n = len(t)
    knots = [zero] * (p + 1)
    for j in range(1, n - p):
        s = zero
        for x in t[j : j + p]:
            s += x
        knots.append(s / p)
    knots += [one] * (p + 1)
    return knots


def closed_params(q, root=math.sqrt, zero=0.0, one=1.0):
    n = len(q)
    d = [chord(q[k], q[(k + 1) % n], root) for k in range(n)]
    total = zero
    for x in d:
        total += x
    t = [zero] * (n + 1)
    for k in range(1, n):
        t[k] = t[k - 1] + d[k - 1] / total
    t[n] = one
    return t


def closed_knots(t, n, p):
    base = [t[i] if p % 2 == 1 else (t[i] + t[i + 1]) / 2 for i in range(n)]

    def knot(m):
        q, r = divmod(m, n)
        return base[r] + q

    return base, [knot(j - p) for j in range(n + 2 * p + 1)]


def twin_open(q, p):
    n = len(q)
    t = open_params(q)
    knots = open_knots(t, p)
    a = [[0.0] * n for _ in range(n)]
    for k in range(n):
        span = find_span(n - 1, p, t[k], knots)
        for r, v in enumerate(basis(span, t[k], p, knots)):
            a[k][span - p + r] = v
    bx = [v[0] for v in q]
    by = [v[1] for v in q]
    for k in range(n):
        piv = a[k][k]
        for i in range(k + 1, min(k + p + 1, n)):
            f = a[i][k] / piv
            for j in range(k, min(k + p + 1, n)):
                a[i][j] -= f * a[k][j]
            bx[i] -= f * bx[k]
            by[i] -= f * by[k]
    x, y = [0.0] * n, [0.0] * n
    for i in reversed(range(n)):
        sx, sy = bx[i], by[i]
        for j in range(i + 1, min(i + p + 1, n)):
            sx -= a[i][j] * x[j]
            sy -= a[i][j] * y[j]
        x[i] = sx / a[i][i]
        y[i] = sy / a[i][i]
    return [(x[i], y[i]) for i in range(n)], knots, t


def twin_closed(q, p):
    n = len(q)
    t = closed_params(q)
    base, knots = closed_knots(t, n, p)
    a = [[0.0] * n for _ in range(n)]
    params = []
    for k in range(n):
        tk = t[k] if t[k] >= base[0] else t[k] + 1.0
        params.append(tk)
        span = find_span(n + p - 1, p, tk, knots)
        for r, v in enumerate(basis(span, tk, p, knots)):
            a[k][(span - p + r) % n] += v
    bx = [v[0] for v in q]
    by = [v[1] for v in q]
    for k in range(n):
        best = k
        for i in range(k + 1, n):
            if abs(a[i][k]) > abs(a[best][k]):
                best = i
        if best != k:
            a[k], a[best] = a[best], a[k]
            bx[k], bx[best] = bx[best], bx[k]
            by[k], by[best] = by[best], by[k]
        piv = a[k][k]
        for i in range(k + 1, n):
            f = a[i][k] / piv
            for j in range(k, n):
                a[i][j] -= f * a[k][j]
            bx[i] -= f * bx[k]
            by[i] -= f * by[k]
    x, y = [0.0] * n, [0.0] * n
    for i in reversed(range(n)):
        sx, sy = bx[i], by[i]
        for j in range(i + 1, n):
            sx -= a[i][j] * x[j]
            sy -= a[i][j] * y[j]
        x[i] = sx / a[i][i]
        y[i] = sy / a[i][i]
    ctrl = [(x[i % n], y[i % n]) for i in range(n + p)]
    return ctrl, knots, params


# ── The same construction exactly (50 digits) ───────────────────────────────


def exact(q, p, closed):
    """The control points of the curve through `q` in 50-digit arithmetic."""
    mp.dps = 50
    qm = [(mpf(a), mpf(b)) for a, b in q]
    n = len(q)
    zero, one = mpf(0), mpf(1)
    if closed:
        t = closed_params(qm, sqrt, zero, one)
        base, knots = closed_knots(t, n, p)
        a = matrix(n, n)
        for k in range(n):
            tk = t[k] if t[k] >= base[0] else t[k] + 1
            span = find_span(n + p - 1, p, tk, knots)
            for r, v in enumerate(basis(span, tk, p, knots, zero, one)):
                a[k, (span - p + r) % n] += v
    else:
        t = open_params(qm, sqrt, zero, one)
        knots = open_knots(t, p, zero, one)
        a = matrix(n, n)
        for k in range(n):
            span = find_span(n - 1, p, t[k], knots)
            for r, v in enumerate(basis(span, t[k], p, knots, zero, one)):
                a[k, span - p + r] = v
    xs = lu_solve(a, matrix([v[0] for v in qm]))
    ys = lu_solve(a, matrix([v[1] for v in qm]))
    return [(xs[i], ys[i]) for i in range(n)], knots


def check_exact(q, p, closed, ctrl, knots):
    want, wknots = exact(q, p, closed)
    scale = max(1.0, max(max(abs(a), abs(b)) for a, b in q))
    for i, (a, b) in enumerate(want):
        got = ctrl[i]
        err = max(abs(mpf(got[0]) - a), abs(mpf(got[1]) - b))
        assert err <= 1e-12 * scale, f"denetim noktası {i}: kesin çözümden {float(err)} uzak"
    for i, k in enumerate(wknots):
        assert abs(mpf(knots[i]) - k) <= 1e-15, f"düğüm {i}"


# ── OpenCASCADE ─────────────────────────────────────────────────────────────


class Occt:
    def __init__(self):
        self.proc = None
        if not OCCT.exists() and shutil.which("clang++") and Path("/opt/homebrew/include/opencascade").is_dir():
            OCCT.parent.mkdir(exist_ok=True)
            subprocess.run(
                ["clang++", "-std=c++17", "-O2", str(OCCT_SRC), "-o", str(OCCT), "-I/opt/homebrew/include/opencascade", "-L/opt/homebrew/lib"]
                + [f"-l{x}" for x in ("TKernel", "TKMath", "TKG2d", "TKG3d", "TKGeomBase", "TKGeomAlgo")]
                + ["-Wl,-rpath,/opt/homebrew/lib"],
                check=True,
            )
        if OCCT.exists():
            self.proc = subprocess.Popen([str(OCCT)], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.checked = 0

    def ask(self, line):
        self.proc.stdin.write(line + "\n")
        self.proc.stdin.flush()
        answer = self.proc.stdout.readline().strip()
        assert not answer.startswith("error"), f"OpenCASCADE: {answer} ({line[:80]})"
        return [float(x) for x in answer.split()] if answer else []

    def curve(self, e):
        ctrl, knots = e["ctrl"], e["knots"]
        w = e.get("weights") or [1.0] * len(ctrl)
        nums = [f"{c['x']!r} {c['y']!r} {wi!r}" for c, wi in zip(ctrl, w)]
        self.ask(f"nurbs {e['degree']} {len(ctrl)} {' '.join(nums)} {len(knots)} {' '.join(map(repr, knots))}")

    def check(self, e, params=None, fit=None):
        """The stored curve as OpenCASCADE evaluates it: through its fit points, its ends, a closed curve's seam."""
        if self.proc is None:
            return
        self.curve(e)
        lo, hi = self.ask("domain")
        scale = max(1.0, max(max(abs(c["x"]), abs(c["y"])) for c in e["ctrl"]))
        tol = 1e-12 * scale + 1e-12
        if fit is not None:
            for t, q in zip(params, fit):
                x, y = self.ask(f"point {t!r}")
                assert max(abs(x - q["x"]), abs(y - q["y"])) <= 1e-9 * scale, f"OpenCASCADE: eğri {q} noktasından geçmiyor ({x}, {y})"
        if e.get("closed"):
            # A curve of degree p is C^(p−1) at a knot, the seam too: the point, then the derivatives up to p − 1.
            a = self.ask(f"derivs {lo!r}")
            b = self.ask(f"derivs {hi!r}")
            orders = 2 * min(3, e["degree"])
            span = max(1.0, max(abs(v) for v in a + b))
            assert all(abs(u - v) <= 1e-9 * span for u, v in zip(a[:orders], b[:orders])), f"OpenCASCADE: kapalı eğri dikişte aynı değil {a} {b}"
        elif e["knots"][0] == e["knots"][e["degree"]] and e["knots"][-1] == e["knots"][-1 - e["degree"]]:
            first, last = e["ctrl"][0], e["ctrl"][-1]
            x, y = self.ask(f"point {lo!r}")
            assert abs(x - first["x"]) <= tol and abs(y - first["y"]) <= tol, "OpenCASCADE: sıkıştırılmış eğri ilk denetim noktasında başlamıyor"
            x, y = self.ask(f"point {hi!r}")
            assert abs(x - last["x"]) <= tol and abs(y - last["y"]) <= tol, "OpenCASCADE: sıkıştırılmış eğri son denetim noktasında bitmiyor"
        self.checked += 1


# ── The command's rules ─────────────────────────────────────────────────────


def refusal(code, message, path, status="failed"):
    return {"status": status, "error": {"code": code, "message": message, "path": path}}


def pt(p):
    return {"x": p[0], "y": p[1]}


def nurbs_problem(p, ctrl, knots, weights):
    """The contract's NURBS rule (docs/adr/0259 §1), its messages."""
    if not 1 <= p <= MAX_DEGREE:
        return f"Derece {p}; 1 ile {MAX_DEGREE} arasında olmalı."
    n = len(ctrl)
    if n < p + 1:
        return f"{p}. dereceden eğrinin en az {p + 1} denetim noktası olur; {n} verildi."
    want = n + p + 1
    if len(knots) != want:
        return f"Düğüm sayısı denetim noktası sayısı + derece + 1 = {want} olmalı; {len(knots)} verildi."
    if any(b < a for a, b in zip(knots, knots[1:])):
        return "Düğümler azalmamalı: her düğüm bir öncekinden küçük olmaz."
    if knots[n] <= knots[p]:
        return "Eğrinin aralığı boş: düğümlerin ortası (derece. ile denetim noktası sayısı. arası) artmalı."
    run, inner = 1, knots[1:-1]
    for a, b in zip(inner, inner[1:]):
        run = run + 1 if a == b else 1
        if run > p:
            return f"İç düğüm {run} kez yinelenmiş; en çok derece ({p}) kez yinelenir."
    if weights is not None:
        if len(weights) != n:
            return f"Ağırlık sayısı ({len(weights)}) denetim noktası sayısından ({n}) farklı."
        if any(not w > 0 for w in weights):
            return "Ağırlıklar sonlu ve sıfırdan büyük olmalı."
    return None


AXIS = {"x": "doğu (Y)", "y": "kuzey (X)"}


def non_finite_point(whose, path, axis):
    return refusal("not_finite", f"{whose} {AXIS[axis]} değeri sonlu bir sayı değil (NaN ya da sonsuz). Koordinatı sonlu bir sayıyla verin.", f"{path}.{axis}")


def non_finite_number(what, path):
    return refusal("not_finite", f"{what} sonlu bir sayı değil (NaN ya da sonsuz). Sonlu bir sayı verin.", path)


def is_finite(v):
    return isinstance(v, (int, float)) and math.isfinite(v)


def closed_fit(pts):
    return pts[:-1] if len(pts) > 1 and pts[0] == pts[-1] else pts


def build(inp):
    """The command's answer for `inp` (its geometry checks), or the curve: (degree, ctrl, knots, weights, fit, params)."""
    by, closed = inp["by"], inp.get("closed", False)
    if by["kind"] == "control":
        p, ctrl = by["degree"], [(c["x"], c["y"]) for c in by["ctrl"]]
        for i, c in enumerate(by["ctrl"]):
            for axis in ("x", "y"):
                if not is_finite(c[axis]):
                    return non_finite_point(f"{i + 1}. denetim noktasının", f"by.ctrl[{i}]", axis)
        weights, knots = by.get("weights"), by.get("knots")
        for name, what, lst in (("weights", "ağırlık", weights), ("knots", "düğüm", knots)):
            for i, v in enumerate(lst or []):
                if not is_finite(v):
                    return non_finite_number(f"{i + 1}. {what}", f"by.{name}[{i}]")
        n = len(ctrl)
        if not 1 <= p <= MAX_DEGREE:
            return refusal("invalid_nurbs", f"Derece {p}; 1 ile {MAX_DEGREE} arasında olmalı.", "by.degree")
        if weights is not None and len(weights) != n:
            return refusal("invalid_nurbs", f"Ağırlık sayısı ({len(weights)}) denetim noktası sayısından ({n}) farklı.", "by.weights")
        if closed:
            if knots is not None:
                return refusal("invalid_nurbs", "Kapalı eğrinin düğümlerini komut kurar; düğüm vermeyin.", "by.knots")
            if n < 3 or n < p:
                return refusal("invalid_nurbs", f"{p}. dereceden kapalı eğrinin en az {max(p, 3)} denetim noktası olur; {n} verildi.", "by.ctrl")
            out_ctrl = [ctrl[i % n] for i in range(n + p)]
            out_knots = [float(i) for i in range(n + 2 * p + 1)]
            out_w = [weights[i % n] for i in range(n + p)] if weights is not None else None
        else:
            if n < p + 1:
                return refusal("invalid_nurbs", f"{p}. dereceden eğrinin en az {p + 1} denetim noktası olur; {n} verildi.", "by.ctrl")
            out_ctrl = ctrl
            if knots is None:
                spans = n - p
                knots = [0.0] * (p + 1) + [float(i) for i in range(1, spans)] + [float(spans)] * (p + 1)
            out_knots = [float(k) for k in knots]
            out_w = [float(w) for w in weights] if weights is not None else None
        why = nurbs_problem(p, out_ctrl, out_knots, out_w)
        if why:
            path = "by.weights" if out_w is not None and any(not w > 0 for w in out_w) else ("by.knots" if by.get("knots") is not None else "by")
            return refusal("invalid_nurbs", why, path)
        return p, out_ctrl, out_knots, out_w, None, None
    pts = [(c["x"], c["y"]) for c in by["pts"]]
    for i, c in enumerate(by["pts"]):
        for axis in ("x", "y"):
            if not is_finite(c[axis]):
                return non_finite_point(f"{i + 1}. geçiş noktasının", f"by.pts[{i}]", axis)
    q = closed_fit(pts) if closed else pts
    n = len(q)
    edges = (n if closed else n - 1) if n >= 2 else 0
    for i in range(edges):
        if q[i] == q[(i + 1) % n]:
            return refusal(
                "coincident_points",
                f"{i + 1}. ve {(i + 1) % n + 1}. geçiş noktası aynı yerde; eğri her noktadan bir kez geçer. Yinelenen noktayı çıkarın.",
                f"by.pts[{(i + 1) % n}]",
            )
    asked = by.get("degree", 3)
    if not 1 <= asked <= MAX_DEGREE:
        return refusal("invalid_nurbs", f"Derece {asked}; 1 ile {MAX_DEGREE} arasında olmalı.", "by.degree")
    least = 3 if closed else 2
    if n < least:
        return refusal("invalid_nurbs", f"{'Kapalı' if closed else 'Açık'} eğri en az {least} geçiş noktasından geçer; {n} verildi.", "by.pts")
    if closed and n > MAX_CLOSED_FIT:
        return refusal("invalid_nurbs", f"Kapalı eğri en çok {MAX_CLOSED_FIT} geçiş noktasından kurulur; {n} verildi.", "by.pts")
    p = max(1, min(asked, n - 1))
    ctrl, knots, params = twin_closed(q, p) if closed else twin_open(q, p)
    check_exact(q, p, closed, ctrl, knots)
    return p, ctrl, knots, None, q, params


LOCKED = "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
HIDDEN = {"code": "layer_hidden", "message": "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}
WEIGHT = "Çizgi kalınlığı 0 ile 100 mm arasında bir sayı olmalı (0 en ince çizgidir). Bir kalınlık ya da “Katmana göre” seçin."
CONFLICT = "Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre yeniden hazırlayın."


def answer(inp, occt, rev_conflict=False):
    """The command's result for `inp` and the entity it writes (none when refused)."""
    built = build(inp)
    if isinstance(built, dict):
        return built, None
    p, ctrl, knots, weights, fit, params = built
    if "lineWeight" in inp and not (is_finite(inp["lineWeight"]) and 0 <= inp["lineWeight"] <= 100):
        return refusal("invalid_line_weight", WEIGHT, "lineWeight"), None
    rev = inp.get("expectedRevision")
    # `$prepared` is a revision the runner captured: a decimal text.
    if rev is not None and not rev.startswith("$") and not (rev.isdigit() and rev.isascii() and (rev == "0" or not rev.startswith("0"))):
        return refusal(
            "invalid_revision",
            f"Beklenen sürüm “{rev}” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.",
            "expectedRevision",
        ), None
    if rev is not None and rev_conflict:
        out = refusal("revision_conflict", CONFLICT, "expectedRevision", "conflict")
        out["error"]["revision"] = "$current"
        return out, None
    layer = inp["layerId"]
    if layer == "kilitli":
        return refusal("layer_locked", LOCKED, "layerId"), None
    if layer == "eski":
        return refusal("layer_locked", "“Eski” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.", "layerId"), None
    if layer in ("pafta", "arsiv", "taslak"):
        name = {"pafta": "Pafta", "arsiv": "Arşiv", "taslak": "Taslak"}[layer]
        return refusal("not_a_layer", f"“{name}” bir katman grubu; nesne yalnız katmana eklenir. Grubun içinden bir katman seçin.", "layerId"), None
    if layer not in ("yapi", "gizli", "sinir", "notlar"):
        return refusal("layer_not_found", f"“{layer}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.", "layerId"), None
    warnings = [HIDDEN] if layer == "gizli" else []
    if layer == "notlar":
        warnings = [{"code": "layer_hidden", "message": "“Notlar” katmanı gizli; çizilen nesne görünmeyecek.", "path": "layerId"}]
    e = {"kind": "nurbs", "id": 0, "layerId": layer}
    for key in ("color",):
        if key in inp:
            e[key] = inp[key]
    e["attrs"] = inp.get("attrs", {})
    for key in ("label", "symbol", "lineWeight"):
        if key in inp:
            e[key] = inp[key]
    e["degree"] = p
    e["ctrl"] = [pt(c) for c in ctrl]
    e["knots"] = knots
    if weights is not None:
        e["weights"] = weights
    if inp.get("closed"):
        e["closed"] = True
        e["periodic"] = True
    if fit is not None:
        e["fit"] = [pt(c) for c in fit]
    occt.check(e, params, e.get("fit"))
    return {"status": "completed", "warnings": warnings}, e


# ── The cases ───────────────────────────────────────────────────────────────

BEFORE = {"ids": [4, 9], "canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def at(dx, dy):
    return {"x": X0 + dx, "y": Y0 + dy}


RIVER = [at(0, 0), at(12, 7.5), at(25, 3), at(38, 11), at(52, 6.25)]
POND = [at(10, 10), at(30, 8), at(36, 22), at(20, 30), at(6, 22)]
COLUMNS = [at(0, 40), at(8, 46), at(16, 38), at(24, 47), at(32, 41), at(40, 44)]


class Writer:
    def __init__(self, occt):
        self.occt = occt
        self.cases = []
        self.next = 10

    def step(self, op, inp, non_finite=None, note=None, conflict=False):
        result, e = answer(with_non_finite(inp, non_finite or {}), self.occt, conflict)
        s = {"op": op}
        if note:
            s["note"] = note
        s["input"] = inp
        if non_finite:
            s["nonFinite"] = non_finite
        if e is None:
            s["result"] = result
            s["expect"] = dict(BEFORE)
            return s
        if op == "validate":
            s["result"] = {"status": "completed", "output": None, "warnings": result["warnings"]}
            s["expect"] = dict(BEFORE)
        elif op == "plan":
            s["result"] = {"status": "completed", "output": {"entity": e, "revision": "$current"}, "warnings": result["warnings"]}
            s["expect"] = dict(BEFORE)
        else:
            e = dict(e, id=self.next)
            s["result"] = {"status": "completed", "output": {"uid": "$uid", "id": self.next, "revision": "$current"}, "warnings": result["warnings"]}
            s["expect"] = {"ids": [4, 9, self.next], "entities": {str(self.next): e}, "canUndo": True, "dirty": True, "revision": "changed"}
            self.next += 1
        return s

    def case(self, name, steps):
        self.cases.append({"name": name, "steps": steps})
        self.next = 10

    def one(self, name, inp, **kw):
        self.case(name, [self.step("execute", inp, **kw)])


def with_non_finite(inp, non_finite):
    """`inp` as the runner gives it to the command: the numbers `nonFinite` names made NaN or ±∞."""
    inp = copy.deepcopy(inp)
    for path, word in non_finite.items():
        value = {"NaN": math.nan, "Infinity": math.inf, "-Infinity": -math.inf}[word]
        parts = path.replace("]", "").replace("[", ".").split(".")
        node = inp
        for key in parts[:-1]:
            node = node[int(key)] if key.isdigit() else node[key]
        last = parts[-1]
        node[int(last) if last.isdigit() else last] = value
    return inp


def cases(occt):
    w = Writer(occt)
    fit = lambda pts, **kw: {"kind": "fit", "pts": pts, **kw}  # noqa: E731
    ctl = lambda degree, ctrl, **kw: {"kind": "control", "degree": degree, "ctrl": ctrl, **kw}  # noqa: E731

    first = w.step("execute", {"layerId": "yapi", "by": fit(RIVER)})
    w.case(
        "geçiş noktalarından kübik eğri yazılır: her noktadan geçer, noktalar eğrinin geçiş noktalarıdır; tek adımda geri alınır, aynı kimlikle yinelenir",
        [
            first,
            {"op": "captureUid", "id": 10, "as": "created"},
            {"op": "undo", "returns": "Ekle", "expect": {"ids": [4, 9], "canUndo": False, "canRedo": True, "dirty": True, "revision": "changed"}},
            {"op": "redo", "returns": "Ekle", "expect": {"ids": [4, 9, 10], "entities": first["expect"]["entities"], "uids": {"10": "created"}, "canUndo": True, "canRedo": False, "revision": "changed"}},
        ],
    )
    w.one("iki geçiş noktası: derece 1'e iner, düz bir parçadır", {"layerId": "yapi", "by": fit(RIVER[:2])})
    w.one("üç geçiş noktasıyla kübik istenirse derece 2'ye iner", {"layerId": "yapi", "by": fit(RIVER[:3], degree=3)})
    w.one("ikinci dereceden eğri altı noktadan geçer", {"layerId": "yapi", "by": fit(COLUMNS, degree=2)})
    w.one("beşinci dereceden eğri altı noktadan geçer", {"layerId": "yapi", "by": fit(COLUMNS, degree=5)})
    w.one("birinci dereceden eğri noktaları düz parçalarla birleştirir", {"layerId": "yapi", "by": fit(COLUMNS, degree=1)})
    w.one(
        "kapalı kübik eğri beş noktadan geçer ve dikişte düzgündür (periyodik); düğümler parametrelerdedir",
        {"layerId": "yapi", "by": fit(POND), "closed": True},
    )
    w.one(
        "kapalı ikinci dereceden eğri: çift derecede düğümler parametrelerin ortalarındadır",
        {"layerId": "yapi", "by": fit(POND, degree=2), "closed": True},
    )
    w.one(
        "kapalı eğrinin ilkiyle aynı yerdeki son noktası ilk noktadır: aynı eğri, geçiş noktaları yinelenmeden saklanır",
        {"layerId": "yapi", "by": fit(POND + [POND[0]]), "closed": True},
    )
    w.one("kapalı dördüncü dereceden eğri beş noktadan geçer", {"layerId": "yapi", "by": fit(POND, degree=4), "closed": True})
    sq = [at(0, 0), at(6, 0), at(6, 6), at(0, 6)]
    w.one(
        "denetim noktalı kübik eğri: düğümler uçlara sıkıştırılmış ve eşit aralıklıdır, eğri ilk ve son noktada başlar ve biter",
        {"layerId": "yapi", "by": ctl(3, COLUMNS)},
    )
    w.one(
        "ağırlıklı ikinci dereceden eğri olduğu gibi saklanır: tam bir çeyrek daire",
        {"layerId": "yapi", "by": ctl(2, [at(10, 0), at(10, 10), at(0, 10)], weights=[1, 0.7071067811865476, 1])},
    )
    w.one(
        "verilen düğümler olduğu gibi saklanır (eşit aralıklı olmayan)",
        {"layerId": "yapi", "by": ctl(3, COLUMNS, knots=[0, 0, 0, 0, 0.25, 0.75, 2, 2, 2, 2])},
    )
    w.one(
        "kapalı denetim noktalı eğri: ilk derece kadar nokta sona sarılır, düğümler 0, 1, …; kapalı ve periyodik saklanır",
        {"layerId": "yapi", "by": ctl(3, sq), "closed": True},
    )
    w.one(
        "kapalı ağırlıklı eğride ağırlıklar da sarılır",
        {"layerId": "yapi", "by": ctl(2, sq, weights=[1, 2, 1, 0.5]), "closed": True},
    )
    w.case(
        "plan yazılacak eğriyi gösterir ve hiçbir şey yazmaz; doğrulama yazmanın cevabını verir",
        [
            w.step("plan", {"layerId": "gizli", "by": fit(RIVER), "color": "#3B82F6"}),
            w.step("validate", {"layerId": "yapi", "by": fit(POND), "closed": True}),
            w.step("validate", {"layerId": "kilitli", "by": fit(POND)}),
        ],
    )
    w.one("gizli katmana uyarıyla yazılır", {"layerId": "gizli", "by": fit(RIVER[:4])})
    w.one(
        "renk, çizgi kalınlığı, öznitelikler, etiket ve sembol verildiği gibi yazılır; grubun içindeki katmana da",
        {"layerId": "sinir", "by": fit(RIVER, degree=2), "color": "#E5484D", "lineWeight": 0.5, "attrs": {"Ad": "Dere"}, "label": "Dere", "symbol": "sys.dere"},
    )
    # Refusals.
    w.case(
        "NaN ya da sonsuz değer yazılmaz; yol hangi değer olduğunu söyler: denetim noktası, ağırlık, düğüm, geçiş noktası",
        [
            w.step("execute", {"layerId": "yapi", "by": ctl(3, COLUMNS)}, non_finite={"by.ctrl[1].y": "NaN"}),
            w.step("execute", {"layerId": "yapi", "by": ctl(2, sq[:3], weights=[1, 1, 1])}, non_finite={"by.weights[2]": "Infinity"}),
            w.step("execute", {"layerId": "yapi", "by": ctl(1, sq[:2], knots=[0, 0, 1, 1])}, non_finite={"by.knots[3]": "-Infinity"}),
            w.step("execute", {"layerId": "yapi", "by": fit(RIVER)}, non_finite={"by.pts[4].x": "NaN"}),
        ],
    )
    w.case(
        "ardışık iki geçiş noktası aynı yerde olamaz; kapalı eğride ilk ikisi de",
        [
            w.step("execute", {"layerId": "yapi", "by": fit([RIVER[0], RIVER[1], RIVER[1], RIVER[2]])}),
            w.step("execute", {"layerId": "yapi", "by": fit([POND[0], POND[0], POND[1], POND[2]]), "closed": True}),
        ],
    )
    w.case(
        "derece 1 ile 11 arasında: sıfır ve on iki reddedilir",
        [
            w.step("execute", {"layerId": "yapi", "by": ctl(0, COLUMNS)}),
            w.step("execute", {"layerId": "yapi", "by": fit(RIVER, degree=12)}),
        ],
    )
    w.case(
        "denetim ve geçiş noktası sayısı: açık eğride en az derece + 1 denetim noktası, kapalıda en az üç; açık eğri en az iki, kapalı en az üç noktadan geçer",
        [
            w.step("execute", {"layerId": "yapi", "by": ctl(3, COLUMNS[:3])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(2, sq[:2]), "closed": True}),
            w.step("execute", {"layerId": "yapi", "by": fit(RIVER[:1])}),
            w.step("execute", {"layerId": "yapi", "by": fit([POND[0], POND[1], POND[0]]), "closed": True}),
        ],
    )
    w.case(
        "düğümler ve ağırlıklar kurala uymalı: sayı, azalmama, aralık, iç düğüm yinelenmesi, pozitif ağırlık; kapalı eğriye düğüm verilmez",
        [
            w.step("execute", {"layerId": "yapi", "by": ctl(3, COLUMNS, knots=[0, 0, 0, 0, 1, 2, 2, 2, 2])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(3, COLUMNS, knots=[0, 0, 0, 0, 2, 1, 2, 2, 2, 2])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(1, sq[:2], knots=[0, 1, 1, 2])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(1, sq[:3], knots=[0, 0, 1, 1, 1])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(2, sq[:3], weights=[1, 0, 1])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(2, sq[:3], weights=[1, 1])}),
            w.step("execute", {"layerId": "yapi", "by": ctl(2, sq, knots=[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]), "closed": True}),
        ],
    )
    w.case(
        "girdinin hatası sırayla söylenir: önce sonlu olmayan değer, sonra yinelenen nokta, sonra eğrinin kuralı, sonra kalınlık, sürüm ve katman",
        [
            w.step("execute", {"layerId": "yok", "by": fit([RIVER[0], RIVER[0], RIVER[1]], degree=12), "lineWeight": 200, "expectedRevision": "99"}, non_finite={"by.pts[2].y": "NaN"}),
            w.step("execute", {"layerId": "yok", "by": fit([RIVER[0], RIVER[0], RIVER[1]], degree=12), "lineWeight": 200, "expectedRevision": "99"}),
            w.step("execute", {"layerId": "yok", "by": fit(RIVER, degree=12), "lineWeight": 200, "expectedRevision": "99"}),
            w.step("execute", {"layerId": "yok", "by": fit(RIVER), "lineWeight": 200, "expectedRevision": "99"}),
        ],
    )
    w.case(
        "beklenen sürüm ondalık bir tamsayı yazısıdır",
        [
            w.step("execute", {"layerId": "yapi", "by": fit(RIVER), "expectedRevision": "12.0"}),
            w.step("execute", {"layerId": "yapi", "by": fit(RIVER), "expectedRevision": "01"}),
        ],
    )
    w.case(
        "kilitli katmana, kilitli grubun katmanına, gruba ve olmayan katmana yazılmaz",
        [
            w.step("execute", {"layerId": "kilitli", "by": fit(RIVER)}),
            w.step("execute", {"layerId": "eski", "by": fit(RIVER)}),
            w.step("execute", {"layerId": "pafta", "by": fit(RIVER)}),
            w.step("execute", {"layerId": "yok", "by": fit(RIVER)}),
        ],
    )
    conflict = w.step("execute", {"layerId": "yapi", "by": fit(RIVER), "expectedRevision": "$prepared"}, conflict=True)
    conflict["expect"] = {"ids": [4, 9, 10], "canUndo": True, "canRedo": False, "revision": "same"}
    w.next = 10
    changed = w.step("execute", {"layerId": "yapi", "by": fit(RIVER[:3])}, note="Girdi hazırlandıktan sonra çizim başka bir düzenlemeyle değişir.")
    changed["expect"] = {"revision": "changed"}
    w.case("çizim beklenen sürümde değilse hiçbir şey yazılmaz: conflict", [{"op": "captureRevision", "as": "prepared"}, changed, conflict])
    return w.cases


def fix_refusal_expectations(all_cases):
    """A refused step changes nothing: the drawing as before the case's steps (no earlier write in these cases)."""
    for c in all_cases:
        wrote = False
        for s in c["steps"]:
            r = s.get("result")
            if r is None:
                continue
            if r["status"] in ("failed", "conflict") and s["op"] == "execute" and not wrote:
                s["expect"] = dict(BEFORE)
            if r["status"] == "completed" and s["op"] == "execute":
                wrote = True


def document(occt):
    src = json.loads(SETUP_FROM.read_text())
    all_cases = cases(occt)
    fix_refusal_expectations(all_cases)
    return {
        "format": "kentos.command-cases",
        "version": 1,
        "command": "cad.nurbs.create",
        "commandVersion": 1,
        "title": "NURBS eğrisi oluştur: denetim noktalarından ve geçiş noktalarından, açık ve kapalı; doğrulama, plan, yazma, geri alma",
        "note": "ADR 0259 §3. Beklenen eğriler scripts/fixtures/nurbs_cases.py'nin IEEE çift duyarlıklı ikizinden (ADR'nin işlem sırasıyla, bit bit), 50 basamaklı kesin çözümle ve OpenCASCADE'le denetlenmiş. Kurulumdaki çizimde iki nesne var (4 ve 9); yeni nesne 10'dan başlar.",
        "setup": src["setup"],
        "cases": all_cases,
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--check", action="store_true")
    args = ap.parse_args()
    occt = Occt()
    doc = document(occt)
    text = json.dumps(doc, ensure_ascii=False, indent=2) + "\n"
    where = "OpenCASCADE ile denetlendi" if occt.proc else "OpenCASCADE yok: o denetim atlandı"
    if args.check:
        if not OUT.exists() or OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} başvurudan farklı; --check'siz yeniden yazın ve farkı okuyun.")
            return 1
        print(f"cad.nurbs.create: {len(doc['cases'])} durum başvuruyla aynı ({occt.checked} eğri {where}).")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(doc['cases'])} durum yazıldı ({occt.checked} eğri {where}).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
