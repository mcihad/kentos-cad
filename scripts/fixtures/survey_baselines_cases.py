#!/usr/bin/env python3
"""The shared cases of `cad.survey.baselines` (GNSS baz ağı; plan/map-cad.md MAP-05, docs/adr/1380), without KentOS
code: numpy's weighted least squares, scipy's χ² quantile and PROJ's (pyproj) ECEF → latitude, longitude.

    python3 scripts/fixtures/survey_baselines_cases.py           # writes fixtures/commands/v1/cad.survey.baselines.json
    python3 scripts/fixtures/survey_baselines_cases.py --check   # writes nothing; compares

Without numpy, scipy and pyproj here the script runs itself again under QGIS's Python on macOS. The rules (ADR 1380):

- unknowns: X, Y, Z of every station not fixed (a weighted one too); a baseline's equations X_to − X_from = d with the
  covariance Σ = scale·C; a weighted station's three pseudo-observations X = X₀ with its σ²;
- x̂ = (AᵀPA)⁻¹AᵀPl, P = Σ⁻¹; v = Ax̂ − l; vᵀPv; f = n − u; m0 = √(vᵀPv/f); χ²(0.95, f); passed: vᵀPv ≤ χ²;
- standard deviations √diag(Q)·m0 (m0 = 1 without degrees of freedom); east, north, up by the rotation at the
  station's GRS80 latitude and longitude;
- the tests on each baseline's components decorrelated by its covariance's Cholesky factor (lower, L·Lᵀ): Qvv̄ = I −
  ĀQĀᵀ, r = diag, w = |v̄|/√q̄vv (σ0 a priori 1), r < 1e-4 uncontrolled, w > 3.29 blunder; a baseline's w the largest
  of its components', `blunder` when one is, `uncontrolled` when all are;
- the worst: the baseline of the flagged observation with the largest w.
"""

import json
import math
import os
import sys
from pathlib import Path

try:
    import numpy as np  # noqa: F401
    import scipy  # noqa: F401
    import pyproj  # noqa: F401
except ImportError:
    qgis = sorted(Path("/Applications").glob("QGIS*.app/Contents"))
    if not qgis or os.environ.get("KENTOS_QGIS_PYTHON"):
        raise
    res = qgis[-1] / "Resources"
    env = dict(os.environ, KENTOS_QGIS_PYTHON="1", PYTHONHOME=str(res),
               PYTHONPATH=os.pathsep.join(str(res / p) for p in ("python3.12", "python3.12/site-packages",
                                                                 "python3.12/lib-dynload")))
    exe = qgis[-1] / "MacOS" / "python3.12"
    os.execve(str(exe), [str(exe), *sys.argv], env)

sys.dont_write_bytecode = True
import numpy as np  # noqa: E402
from pyproj import Transformer  # noqa: E402
from scipy.stats import chi2 as chi2_dist  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.survey.baselines.json"
TO_GEO = Transformer.from_crs(4978, 4979, always_xy=True)
TO_XYZ = Transformer.from_crs(4979, 4978, always_xy=True)
W_LIMIT, UNCONTROLLED = 3.29, 1e-4


def xyz(lon, lat, h):
    return list(TO_XYZ.transform(lon, lat, h))


def enu_rows(p):
    lon, lat, _ = TO_GEO.transform(*p)
    la, lo = math.radians(lat), math.radians(lon)
    return np.array([[-math.sin(lo), math.cos(lo), 0.0],
                     [-math.sin(la) * math.cos(lo), -math.sin(la) * math.sin(lo), math.cos(la)],
                     [math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la)]])


# A network near Ankara: A at 40°N 33°E, 1000 m; B, C, D a few km off (east, north, up from A).
A0 = xyz(33.0, 40.0, 1000.0)
R0 = enu_rows(A0)


def off(e, n, u):
    return list(np.array(A0) + R0.T @ np.array([e, n, u]))


TRUE = {"A": A0, "B": off(3000.0, 1000.0, 12.0), "C": off(1500.0, 4200.0, -25.0), "D": off(-2500.0, 2600.0, 40.0)}


def cov_enu(se, sn, su, at="A"):
    """A covariance (xx, xy, xz, yy, yz, zz) of standard deviations east, north, up at a station: correlated in X, Y, Z."""
    r = enu_rows(TRUE[at])
    c = r.T @ np.diag([se * se, sn * sn, su * su]) @ r
    return [c[0, 0], c[0, 1], c[0, 2], c[1, 1], c[1, 2], c[2, 2]]


def base(f, t, err=(0.0, 0.0, 0.0), sig=(0.003, 0.003, 0.006)):
    """The baseline f → t: the true difference plus an error given in east, north, up (m)."""
    e = R0.T @ np.array(err)
    d = [TRUE[t][k] - TRUE[f][k] + e[k] for k in range(3)]
    return {"from": f, "to": t, "d": d, "cov": cov_enu(*sig, at=f)}


ERRS = {("A", "B"): (0.002, -0.001, 0.003), ("B", "C"): (-0.002, 0.002, -0.004), ("A", "C"): (0.001, 0.001, 0.002),
        ("C", "D"): (0.0, -0.002, 0.003), ("A", "D"): (-0.001, 0.002, -0.002), ("B", "D"): (0.002, 0.0, 0.004)}


def net(*pairs, blunder=None):
    out = []
    for p in pairs:
        err = list(ERRS[p])
        if blunder == p:
            err[2] += 0.060
        out.append(base(*p, err=err))
    return out


ALL = [("A", "B"), ("B", "C"), ("A", "C"), ("C", "D"), ("A", "D"), ("B", "D")]


def full(c):
    return np.array([[c[0], c[1], c[2]], [c[1], c[3], c[4]], [c[2], c[4], c[5]]])


def adjust(stations, baselines, scale=1.0, minimal=False):
    """The answer of ADR 1380's rules, or (code, message, path)."""
    names = [s["name"] for s in stations]
    fixed = [bool(s.get("fixed")) for s in stations]
    held = None
    if not any(s.get("fixed") or s.get("sigma") for s in stations):
        first = next((i for i, s in enumerate(stations) if s.get("xyz")), None)
        if not minimal or first is None:
            return ("datum_missing", "Ağın datumu yok: bir istasyonu sabit ya da ağırlıklı verin, ya da datum minimal ile "
                                     "koordinatı olan ilk istasyonu tutun.", "datum")
        fixed[first] = True
        held = names[first]
    # Approximations (the reference's own): a station's coordinates, else the baselines carried from one that has
    # them; the unknowns are corrections, so the numbers stay small.
    approx = {i: np.array(s["xyz"], dtype=float) for i, s in enumerate(stations) if s.get("xyz")}
    grew = True
    while grew:
        grew = False
        for b in baselines:
            i, j = names.index(b["from"]), names.index(b["to"])
            if i in approx and j not in approx:
                approx[j] = approx[i] + np.array(b["d"]); grew = True
            elif j in approx and i not in approx:
                approx[i] = approx[j] - np.array(b["d"]); grew = True
    x0 = [approx.get(i, np.zeros(3)) for i in range(len(stations))]
    slot, u = [], 0
    for f in fixed:
        slot.append(None if f else u)
        u += 0 if f else 3
    rows_a, rows_l, blocks = [], [], []
    for b in baselines:
        i, j = names.index(b["from"]), names.index(b["to"])
        a = np.zeros((3, u))
        l = np.array(b["d"], dtype=float) - (x0[j] - x0[i])
        for k in range(3):
            if slot[j] is not None:
                a[k, slot[j] + k] = 1.0
            if slot[i] is not None:
                a[k, slot[i] + k] = -1.0
        rows_a.append(a)
        rows_l.append(l)
        blocks.append(full(b["cov"]) * scale)
    for i, s in enumerate(stations):
        if s.get("sigma") and slot[i] is not None:
            a = np.zeros((3, u))
            for k in range(3):
                a[k, slot[i] + k] = 1.0
            rows_a.append(a)
            rows_l.append(np.array(s["xyz"], dtype=float) - x0[i])
            blocks.append(np.diag([x * x for x in s["sigma"]]))
    A = np.vstack(rows_a)
    l = np.concatenate(rows_l)
    n = A.shape[0]
    sigma = np.zeros((n, n))
    for k, blk in enumerate(blocks):
        sigma[3 * k:3 * k + 3, 3 * k:3 * k + 3] = blk
    P = np.linalg.inv(sigma)
    N = A.T @ P @ A
    if np.linalg.matrix_rank(N) < u:
        bad = next(i for i in range(len(stations)) if slot[i] is not None and N[slot[i], slot[i]] == 0.0)
        return ("network_disconnected", f"“{names[bad]}” istasyonu datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.",
                "baselines")
    Q = np.linalg.inv(N)
    x = Q @ (A.T @ P @ l)
    v = A @ x - l
    omega = float(v @ P @ v)
    f = n - u
    m0 = math.sqrt(omega / f) if f > 0 else None
    q2 = float(chi2_dist.ppf(0.95, f)) if f > 0 else None
    s0 = m0 if m0 is not None else 1.0
    # The decorrelated tests.
    ws, flags = [], []
    for k, blk in enumerate(blocks):
        L = np.linalg.cholesky(blk)
        Li = np.linalg.inv(L)
        ab = Li @ A[3 * k:3 * k + 3]
        vb = Li @ v[3 * k:3 * k + 3]
        qvv = np.eye(3) - ab @ Q @ ab.T
        for c in range(3):
            r = qvv[c, c]
            if not r >= UNCONTROLLED:
                ws.append(None)
                flags.append("uncontrolled")
            else:
                w = abs(vb[c]) / math.sqrt(r)
                ws.append(w)
                flags.append("blunder" if w > W_LIMIT else "ok")
    worst = None
    for i, (w, fl) in enumerate(zip(ws, flags)):
        if fl == "blunder" and (worst is None or w > ws[worst]):
            worst = i
    out_st = []
    for i, s in enumerate(stations):
        if slot[i] is None:
            p = list(s["xyz"])
            sx = se = [0.0, 0.0, 0.0]
        else:
            t = slot[i]
            p = [float(x0[i][k] + x[t + k]) for k in range(3)]
            c = Q[t:t + 3, t:t + 3] * s0 * s0
            sx = [math.sqrt(c[k, k]) for k in range(3)]
            r = enu_rows(p)
            ce = r @ c @ r.T
            se = [math.sqrt(max(ce[k, k], 0.0)) for k in range(3)]
        row = {"name": s["name"], "xyz": p, "sxyz": sx, "senu": se}
        if fixed[i]:
            row["held"] = True
        out_st.append(row)
    out_b = []
    for k, b in enumerate(baselines):
        part_w = [w for w in ws[3 * k:3 * k + 3] if w is not None]
        part_f = flags[3 * k:3 * k + 3]
        row = {"from": b["from"], "to": b["to"], "v": [float(v[3 * k + c]) for c in range(3)]}
        if part_w:
            row["w"] = max(part_w)
        row["flag"] = "blunder" if "blunder" in part_f else ("uncontrolled" if all(fl == "uncontrolled" for fl in part_f) else "ok")
        out_b.append(row)
    out = {"stations": out_st, "baselines": out_b, "n": n, "u": u, "f": f, "omega": omega, "revision": "$current"}
    if held:
        out["held"] = held
    if m0 is not None:
        out.update(m0=m0, chi2=q2, passed=omega <= q2)
    if worst is not None and worst < 3 * len(baselines):
        out["worst"] = worst // 3
    return out


SAME = {"revision": "same", "canUndo": False}


def ok_case(name, stations, baselines, scale=None, minimal=False, op="execute", note=None, text=None):
    input_ = {}
    if text is not None:
        input_["text"] = text
    else:
        input_["stations"] = stations
        input_["baselines"] = baselines
    if scale is not None:
        input_["scale"] = scale
    if minimal:
        input_["datum"] = "minimal"
    out = adjust(stations, baselines, scale or 1.0, minimal)
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": []}
    c = {"name": name}
    if note:
        c["note"] = note
    c["steps"] = [{"op": op, "input": input_, "result": result, "expect": SAME, "within": 1e-6}]
    return c


def refused(name, input_, code, message, path):
    return {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": {
        "code": code, "message": message, "path": path}}, "expect": SAME}]}


def station(name, fixed=False, sigma=None, known=True):
    s = {"name": name}
    if known:
        s["xyz"] = TRUE[name]
    if fixed:
        s["fixed"] = True
    if sigma is not None:
        s["sigma"] = sigma
    return s


def text_of(stations, baselines, sep=";"):
    lines = ["# GNSS baz ağı"]
    for s in stations:
        if s.get("fixed"):
            lines.append(sep.join(["K", s["name"], *(repr(float(v)) for v in s["xyz"])]))
        else:
            lines.append(sep.join(["N", s["name"]]))
    for b in baselines:
        lines.append(sep.join(["B", b["from"], b["to"], *(repr(float(v)) for v in b["d"]), *(repr(float(v)) for v in b["cov"])]))
    return "\n".join(lines) + "\n"


def cases():
    one = [station("A", fixed=True), station("B", known=False), station("C", known=False), station("D", known=False)]
    out = [
        ok_case("Bir sabit, üç yeni istasyon, altı baz: korelasyonlu kovaryans, χ² geçer", one, net(*ALL)),
        ok_case("Bir baz 6 cm yukarıda kaba hatalı: Baarda testi onu gösterir, model testi düşer", one,
                net(*ALL, blunder=("B", "C"))),
        ok_case("İki sabit istasyon", [station("A", fixed=True), station("B", known=False), station("C", known=False),
                                       station("D", fixed=True)], net(*ALL)),
        ok_case("Ağırlıklı bilinen istasyon (σ 1 cm)", [station("A", sigma=[0.01, 0.01, 0.01]), station("B", known=False),
                                                       station("C", known=False), station("D", known=False)], net(*ALL)),
        ok_case("Minimal datum: sabit yokken koordinatı olan ilk istasyon tutulur", [station("A"), station("B", known=False),
                                                                                    station("C", known=False),
                                                                                    station("D", known=False)],
                net(*ALL), minimal=True),
        ok_case("Varyans çarpanı 4: m0 yarıya iner, standart sapmalar değişmez", one, net(*ALL), scale=4.0),
        ok_case("Fazlalık yok (ağaç): m0 ve χ² yok, bazlar denetlenemez", [station("A", fixed=True), station("B", known=False),
                                                                          station("C", known=False)],
                net(("A", "B"), ("B", "C"))),
        ok_case("Tek baz, iki sabit uç: bilinmeyen yok", [station("A", fixed=True), station("B", fixed=True)],
                net(("A", "B"))),
        ok_case("Ters yönde verilen baz da aynı ağı kurar", one,
                net(("A", "B"), ("B", "C"), ("A", "C"), ("C", "D"), ("A", "D")) + [base("D", "B", err=(0.0, 0.001, 0.0))]),
        ok_case("Plan aynı cevabı verir", one, net(*ALL), op="plan"),
        ok_case("Doğrulama", one, net(*ALL), op="validate"),
        ok_case("Metinle: noktalı virgül", one, net(*ALL), text=text_of(one, net(*ALL))),
        ok_case("Metinle: virgül ve sekme ayraç, açıklama satırı", one, net(*ALL),
                text=text_of(one, net(*ALL), sep=",").replace("N,B", "N\tB")),
    ]
    two = [station("A", fixed=True), station("B", known=False)]
    ab = net(("A", "B"))
    out += [
        refused("Baz yok", {"stations": two}, "no_baselines", "Baz vektörü yok; en az bir baz verin.", "baselines"),
        refused("Aynı ad iki kez (büyük küçük harf ayırmadan)", {"stations": two + [{"name": "b"}], "baselines": ab},
                "duplicate_station", "“b” istasyonu iki kez verilmiş.", "stations[2].name"),
        refused("Bazın ucu istasyon değil", {"stations": two, "baselines": [dict(ab[0], to="X")]}, "unknown_station",
                "“X” istasyonu verilmedi.", "baselines[0].to"),
        refused("Kovaryans pozitif tanımlı değil", {"stations": two, "baselines": [dict(ab[0], cov=[1e-6, 2e-6, 0, 1e-6, 0, 1e-6])]},
                "invalid_covariance", "1. bazın kovaryansı pozitif tanımlı değil.", "baselines[0].cov"),
        refused("Datum yok", {"stations": [station("A"), station("B", known=False)], "baselines": ab}, "datum_missing",
                "Ağın datumu yok: bir istasyonu sabit ya da ağırlıklı verin, ya da datum minimal ile koordinatı olan ilk "
                "istasyonu tutun.", "datum"),
        refused("Minimal datumda koordinatı olan istasyon yok", {"stations": [station("A", known=False),
                                                                              station("B", known=False)],
                                                                 "baselines": ab, "datum": "minimal"}, "datum_missing",
                "Ağın datumu yok: bir istasyonu sabit ya da ağırlıklı verin, ya da datum minimal ile koordinatı olan ilk "
                "istasyonu tutun.", "datum"),
        refused("Baza bağlanmayan istasyon", {"stations": two + [station("C", known=False)], "baselines": ab},
                "network_disconnected", "“C” istasyonu datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.", "baselines"),
        refused("Hem sabit hem ağırlıklı", {"stations": [dict(station("A", fixed=True), sigma=[0.01, 0.01, 0.01]),
                                                         station("B", known=False)], "baselines": ab},
                "invalid_input", "“A” hem sabit hem ağırlıklı olamaz.", "stations[0]"),
        refused("Varyans çarpanı sıfır", {"stations": two, "baselines": ab, "scale": 0.0}, "invalid_input",
                "Varyans çarpanı sıfırdan büyük bir sayı olmalı.", "scale"),
        refused("Metin ve liste birlikte", {"stations": two, "text": "N;C\n"}, "invalid_input",
                "Ağı ya metinle (text) ya da istasyon ve baz listeleriyle verin, ikisiyle birden değil.", "text"),
        refused("Metinde sayı olmayan değer", {"text": "K;A;1;2;x\n"}, "invalid_input",
                "1. satırda “x” bir sayı değil (ondalıktan önce nokta).", "text"),
        refused("Metinde eksik satır", {"text": "# ağ\nB;A;B;1;2;3\n"}, "invalid_input",
                "2. satır okunamadı: K, N ya da B ile başlayıp değerleri tam olmalı.", "text"),
    ]
    return out


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.survey.baselines", "commandVersion": 1,
           "title": "GNSS baz ağı dengelemesi",
           "note": "ADR 1380. Bağımsız başvuru scripts/fixtures/survey_baselines_cases.py: numpy'nin ağırlıklı en küçük "
                   "kareleri, scipy'nin χ² nicemi, PROJ'un (pyproj) ECEF → enlem, boylam dönüşümü; KentOS kodu yok.",
           "setup": {"format": "kentos.document", "version": 1, "name": "GNSS ağı",
                     "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
                                  "angleUnit": "grad", "plotScale": 1000, "workspace": "cad", "cadWork": "map"},
                     "origin": {"x": 500000, "y": 4400000},
                     "layers": [{"id": "0", "name": "0", "type": "layer", "visible": True, "locked": False,
                                 "expanded": True, "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25},
                                 "children": []}],
                     "activeLayer": "0", "entities": [], "styles": {"items": [], "categories": []}},
           "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        old = json.loads(OUT.read_text("utf-8")) if OUT.exists() else None
        new = json.loads(text)
        same = old is not None and len(old["cases"]) == len(new["cases"]) and all(
            a["name"] == b["name"] for a, b in zip(old["cases"], new["cases"]))
        print("GNSS baz ağı ortak durumları güncel." if same else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if same else 1
    OUT.write_text(text, "utf-8")
    # A sample file for the window: the network with B → C's blunder (fixtures/gnss-network/v1/ankara-ag.txt).
    one = [station("A", fixed=True), station("B", known=False), station("C", known=False), station("D", known=False)]
    sample = ROOT / "fixtures/gnss-network/v1/ankara-ag.txt"
    sample.parent.mkdir(parents=True, exist_ok=True)
    sample.write_text(text_of(one, net(*ALL, blunder=("B", "C"))), "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum), {sample.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
