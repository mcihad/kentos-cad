#!/usr/bin/env python3
"""The shared cases of `map.survey.network` and of `map.survey.edit`'s network operations (plan/map-cad.md MAP-03 and
MAP-05, docs/adr/1740), written from the ADR without KentOS code, and the files they read (fixtures/map-survey/v1:
ag.gvx, ag-v2.gvx, ag.txt).

    python3 scripts/fixtures/map_survey_network_cases.py           # writes map.survey.network.json and the files
    python3 scripts/fixtures/map_survey_network_cases.py --check   # writes nothing; compares

map_survey_cases.py appends `edit_cases()` to map.survey.edit.json. Needs numpy, scipy and pyproj: without them here
the script runs itself again under QGIS's Python on macOS. The rules (ADR 1740):

- the network: the archive's baselines with `adjust` and not excluded (of one campaign when asked), its marks in the
  order the baselines name them (start, then end), its known points those not excluded on those marks, all in one
  geographic frame and epoch; a known point's X, Y, Z on the frame's ellipsoid (PROJ, TUREF 3D → geocentric);
- a stage: x̂ = (AᵀPA)⁻¹AᵀPl with P = Σ⁻¹, Σ block diagonal: each baseline's covariance times the variance factor, a
  weighted known point's north, east, up deviations turned into X, Y, Z at its own latitude and longitude
  (Rᵀ·diag(σe², σn², σu²)·R); vᵀPv, f = n − u, m0 = √(vᵀPv/f), χ²(0.95, f) (scipy), passed vᵀPv ≤ χ²;
- the free stage holds the first known point (or the one asked) fixed, the others are unknowns; the stage held by the
  known points holds those without σ fixed and weights the others; a free part of the network no known point holds is
  refused at the first unknown whose Cholesky pivot is not above 1e-10 of its diagonal (ADR 0203's rule);
- the tests: each block's components decorrelated by its covariance's lower Cholesky factor, r = 1 − (ĀQĀᵀ)ᵢᵢ, w =
  |v̄|/√r (r < 1e-4: uncontrolled), a baseline's w the largest of its components'; a blunder above the limit: the class's
  two-sided normal quantile at its confidence (scipy's norm.ppf((1 + p)/2)), else Baarda's 3.29; a weighted known
  point's pseudo-observations too;
- a baseline's residual in the east, north, up of its start's adjusted place; the adjusted vector's standard
  deviations √diag(Qtt + Qff − Qtf − Qft)·m0 (m0 = 1 without degrees of freedom); a station's north, east, up
  deviations √diag(R·Q·Rᵀ)·m0;
- the marks that are not known points moved into the project by the coordinate chain (crs_vertical_cases.py, as
  map_survey_cases.py does) and compared with their accepted points (dx, dy, dz);
- a class (catalog/v1/rules/gnss-ag-bohhbuy-2018.json, BÖHHBÜY 2018 Madde 15, 20, 22): a baseline's σX, σY, σZ (the
  processor's by the variance factor, or C1's free stage's) ≤ a + b·10⁻⁶·length; a new mark's σN, σE ≤ horizontal, σU ≤
  height.

ADR 2120 adds the measurement epoch: the input's `epoch`, else the mean of the baselines' observation mid-times (each
GPS time a decimal year: the year and the share of its seconds gone) when every one has start and end, else none. With
one every known point says its epoch (one frame) and one at another epoch is moved to it along the campaign's velocity
grid by PROJ (geoid_cases.transformed, the deformation step); the shift is east, north, up at its given place. The
network's epoch is the measurement epoch; without one it is the known points' one epoch, as before.

ADR 2560 adds each mark's confidence region from the stage held by the known points (a mark not held fixed): its east,
north, up covariance R·Q·Rᵀ·m0²; numpy's eigen decomposition of the north, east 2×2 gives the error ellipse's semi-axes
(√χ²₂(p)·√λ, scipy's χ²) and the major axis's direction from north clockwise in [0°, 180°), the height's half-interval
is √χ²₁(p)·σu; p is the class's confidence, else 0.95 (Madde 15(1)(d)). A circle's direction (the axes within a
millionth of the major one) is not given. A case gives the baselines unequal east and north deviations so the ellipses
have a direction.

ADR 2520 adds a known point's own velocity (a station's published one: the frame's Earth-centred X, Y, Z, m/yr): with
it the point goes to the measurement epoch linearly, X(t) = X(t₀) + V·(t − t₀) (IERS Conventions 2010 §4.1), its X, Y,
Z by PROJ (TUREF 3D → geocentric and back), before the campaign's velocity grid is looked at; `moved` says `by`
(`velocity` or `grid`). The cases give BN1 and BN2 the grid's own velocity at their places (PROJ's deformation over a
year, as X, Y, Z, rounded to 0.01 mm/yr as a table would print it), so the network fits as it does with the grid.

ADR 2160 adds BÖHHBÜY Madde 15(1)(c): with three known points or more, the 3D similarity given = T + λ·R·free between
their given coordinates (at the measurement epoch) and the free stage's, by Horn's closed-form absolute orientation
(unit quaternions; Horn 1987), the scale λ = Σ g'·Rf' / Σ|f'|²; the rotations from R's antisymmetric part (arc-seconds);
σ of the scale from the linearised normal equations at the solution with σ0² = vᵀv/(3n − 7); t = |λ − 1|/σ against
scipy's Student t at (1 + confidence)/2 (confidence: the class's, else 0.95, Madde 15(1)(d)); the residuals east, north,
up at the given places; a class with `similarity` (C1: 3 ppm) holds when |λ − 1| ≤ it.
"""

import copy
import hashlib
import json
import math
import os
import sys
from pathlib import Path
from xml.etree import ElementTree as ET

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
sys.path.insert(0, str(Path(__file__).resolve().parent))
import numpy as np  # noqa: E402
import pyproj  # noqa: E402
from scipy.stats import chi2 as chi2_dist  # noqa: E402
from scipy.stats import norm  # noqa: E402
from scipy.stats import t as student  # noqa: E402

import map_survey_cases as ms  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/map.survey.network.json"
FILES = ROOT / "fixtures/map-survey/v1"
RULES = json.loads((ROOT / "catalog/v1/rules/gnss-ag-bohhbuy-2018.json").read_text("utf-8"))
TO_XYZ = pyproj.Transformer.from_crs(5251, 5250, always_xy=True)
TO_GEO = pyproj.Transformer.from_crs(5250, 5251, always_xy=True)
BAARDA, UNCONTROLLED, PIVOT = 3.29, 1e-4, 1e-10
GEOGRAPHIC = {4326, 5252, 4230, 4258}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def fx(name):
    return f"$fixture:map-survey/v1/{name}"


# ── The geometry: two known points and three marks around N1's accepted place ─────────────────────────────────────


def ecef(lat, lon, h):
    x, y, z = TO_XYZ.transform(lon, lat, h)
    return np.array([x, y, z])


def geodetic(p):
    lon, lat, h = TO_GEO.transform(float(p[0]), float(p[1]), float(p[2]))
    return lat, lon, h


def rows(lat, lon):
    la, lo = math.radians(lat), math.radians(lon)
    return np.array([[-math.sin(lo), math.cos(lo), 0.0],
                     [-math.sin(la) * math.cos(lo), -math.sin(la) * math.sin(lo), math.cos(la)],
                     [math.cos(la) * math.cos(lo), math.cos(la) * math.sin(lo), math.sin(la)]])


ms.ORIGIN = ms.origin()
BASE = ms.setup()
N1R = BASE.survey["points"][0]["result"]
R1 = rows(N1R["lat"], N1R["lon"])
P1 = ecef(N1R["lat"], N1R["lon"], N1R["height"])


def at(e, n, u):
    return P1 + R1.T @ np.array([e, n, u])


def given(p):
    """A known point's published coordinates: latitude and longitude to 1e-9°, height to 0.1 mm."""
    lat, lon, h = geodetic(p)
    return round(lat, 9), round(lon, 9), round(h, 4)


KNOWN_AT = {"BN1": given(at(-4100.0, -2600.0, 32.5)), "BN2": given(at(3600.0, -2900.0, -18.25))}
TRUE = {"N1": P1, "BN1": ecef(*KNOWN_AT["BN1"]), "BN2": ecef(*KNOWN_AT["BN2"]), "N4": at(1550.0, 2800.0, 14.75),
        "N5": at(-2250.0, 3150.0, -36.5)}
PAIRS = [("BN1", "N1"), ("BN2", "N1"), ("BN1", "N4"), ("BN2", "N4"), ("N1", "N4"), ("N1", "N5"), ("BN1", "N5"),
         ("N4", "N5")]
ERRS = {("BN1", "N1"): (0.0012, -0.0008, 0.0021), ("BN2", "N1"): (-0.0009, 0.0011, -0.0017),
        ("BN1", "N4"): (0.0006, 0.0009, -0.0013), ("BN2", "N4"): (-0.0011, -0.0005, 0.0024),
        ("N1", "N4"): (0.0008, -0.0012, 0.0009), ("N1", "N5"): (-0.0007, 0.0006, -0.0022),
        ("BN1", "N5"): (0.0010, -0.0004, 0.0015), ("N4", "N5"): (-0.0005, 0.0010, -0.0011)}
SIG = (0.0025, 0.0025, 0.005)


def vector(f, t, err, sig=SIG, turn=0.0):
    """f → t: the true difference with an error east, north, up (m) at f, its covariance from deviations east, north,
    up at f (their axes turned `turn` degrees clockwise about up) as GVX keeps it: standard deviations and
    correlations, the covariance made of them again."""
    lat, lon, _ = geodetic(TRUE[f])
    r = rows(lat, lon)
    d = TRUE[t] - TRUE[f] + r.T @ np.array(err)
    a = math.radians(turn)
    z = np.array([[math.cos(a), math.sin(a), 0.0], [-math.sin(a), math.cos(a), 0.0], [0.0, 0.0, 1.0]])
    c = r.T @ z @ np.diag([s * s for s in sig]) @ z.T @ r
    sd = [math.sqrt(float(c[k, k])) for k in range(3)]
    p = [float(c[0, 1]) / (sd[0] * sd[1]), float(c[0, 2]) / (sd[0] * sd[2]), float(c[1, 2]) / (sd[1] * sd[2])]
    cov = [sd[0] * sd[0], p[0] * sd[0] * sd[1], p[1] * sd[0] * sd[2], sd[1] * sd[1], p[2] * sd[1] * sd[2], sd[2] * sd[2]]
    return {"from": f, "to": t, "d": [float(x) for x in d], "covariance": cov, "sd": sd, "p": p}


VECTORS = [vector(f, t, ERRS[(f, t)]) for f, t in PAIRS]


def record(k, v, **extra):
    r = {"id": f"bz-{k}", "campaign": "kp-1", "from": v["from"], "to": v["to"], "d": list(v["d"]),
         "covariance": list(v["covariance"])}
    r.update(extra)
    r.setdefault("adjust", True)
    return r


def known(k, mark, sigma=None, epoch=2005.0, srid=5252, **extra):
    lat, lon, h = KNOWN_AT[mark] if mark in KNOWN_AT else geodetic(TRUE[mark])
    r = {"id": f"bn-{k}", "mark": mark, "lat": lat, "lon": lon, "height": h, "srid": srid}
    if epoch is not None:
        r["epoch"] = epoch
    if sigma is not None:
        r["sigma"] = list(sigma)
    r.update(extra)
    return r


KNOWN = [known(1, "BN1", source="TUSAGA-Aktif koordinatı (örnek), TUREF, 2005,0"),
         known(2, "BN2", source="C1 noktası (örnek), TUREF, 2005,0")]


def networked(baselines=None, known_points=None, survey=None):
    m = (survey or BASE).clone()
    m.survey["baselines"] = copy.deepcopy(baselines if baselines is not None else
                                          [record(k + 1, v) for k, v in enumerate(VECTORS)])
    m.survey["known"] = copy.deepcopy(known_points if known_points is not None else KNOWN)
    m.survey.setdefault("adjustments", [])
    return m


# ── The files ────────────────────────────────────────────────────────────────────────────────────────────────────


def gvx(version="1.0", crossed=True):
    out = ['<?xml version="1.0" encoding="utf-8"?>', f'<GVX VERSION="{version}">',
           "  <SOURCE_DATA><NAME>ag-2026-11.job</NAME><CREATED_DATE>2026-11-03T12:00:00.00</CREATED_DATE>"
           "<APPLICATION><NAME>KentOS örnek</NAME><VERSION>1</VERSION></APPLICATION>"
           "<CONVERTED_BY><SOFTWARE_NAME>map_survey_network_cases.py</SOFTWARE_NAME>"
           "<CONVERTED_DATE>2026-11-07T00:00:00.00</CONVERTED_DATE></CONVERTED_BY></SOURCE_DATA>",
           "  <REFERENCE_SYSTEM><ID>153</ID><NAME>IGS14</NAME><LINEAR_UNIT><NAME>meters</NAME></LINEAR_UNIT>"
           "<ANGULAR_UNIT><NAME>decimal degrees</NAME></ANGULAR_UNIT></REFERENCE_SYSTEM>",
           "  <EQUIPMENT><ID>E1</ID><RECEIVER><TYPE>TRIMBLE R10</TYPE><SERIAL_NUMBER>5123K40001</SERIAL_NUMBER>"
           "<FIRMWARE_VERSION>5.52</FIRMWARE_VERSION></RECEIVER><ANTENNA><TYPE>TRM57971.00     NONE</TYPE>"
           "<SERIAL_NUMBER>1441112222</SERIAL_NUMBER></ANTENNA></EQUIPMENT>",
           "  <SURVEY_SETUP><ID>S1</ID><SOLUTION_TYPE>Post-processed</SOLUTION_TYPE><OPERATOR>Ayşe Demir</OPERATOR>"
           "<PROCESSING_SOFTWARE><NAME>TBC</NAME><VERSION>5.90</VERSION></PROCESSING_SOFTWARE></SURVEY_SETUP>"]
    for mark in ("BN1", "BN2", "N1", "N4", "N5"):
        lat, lon, h = geodetic(TRUE[mark])
        out.append(f"  <POINT><ID>P-{mark}</ID><NAME>{mark}</NAME><EQUIPMENT_ID>E1</EQUIPMENT_ID>"
                   f"<ARP_HEIGHT>1.5000</ARP_HEIGHT><POINT_TYPE>{'Keyed-in' if mark.startswith('BN') else 'Fixed'}"
                   f"</POINT_TYPE><COORDINATES><REFERENCE_SYSTEM_ID>153</REFERENCE_SYSTEM_ID><EPOCH>2026.8400</EPOCH>"
                   f"<GEODETIC_COORDINATES><LATITUDE>{lat:.10f}</LATITUDE><LONGITUDE>{lon:.10f}</LONGITUDE>"
                   f"<ELLIPSOIDAL_HEIGHT>{h:.4f}</ELLIPSOIDAL_HEIGHT></GEODETIC_COORDINATES></COORDINATES></POINT>")
    for k, v in enumerate(VECTORS):
        hour = 8 + 2 * (k // 4)
        out.append(f"  <GNSS_VECTOR><ID>V{k + 1}</ID><INITIAL_POINT_ID>P-{v['from']}</INITIAL_POINT_ID>"
                   f"<TERMINAL_POINT_ID>P-{v['to']}</TERMINAL_POINT_ID><SURVEY_SETUP_ID>S1</SURVEY_SETUP_ID>"
                   f"<OBSERVATION_TIME><START>2026-11-03T{hour:02d}:00:00.00</START>"
                   f"<END>2026-11-03T{hour + 2:02d}:00:00.00</END></OBSERVATION_TIME>"
                   f"<QUALITY_CONTROL><EPOCHS_USED>{481 + k}</EPOCHS_USED><SATELLITE_USED><TOTAL>{14 + k % 4}</TOTAL>"
                   f"</SATELLITE_USED></QUALITY_CONTROL>"
                   f"<ECEF_DELTAS><DX>{v['d'][0]!r}</DX><DY>{v['d'][1]!r}</DY><DZ>{v['d'][2]!r}</DZ></ECEF_DELTAS>"
                   f"<CORRELATION_MATRIX><SDX>{v['sd'][0]!r}</SDX><SDY>{v['sd'][1]!r}</SDY><SDZ>{v['sd'][2]!r}</SDZ>"
                   f"<PXY>{v['p'][0]!r}</PXY><PXZ>{v['p'][1]!r}</PXZ><PYZ>{v['p'][2]!r}</PYZ></CORRELATION_MATRIX>"
                   f"</GNSS_VECTOR>")
    if crossed:
        out.append('  <SESSION ID="O1" TOTAL_VECTORS="2"><SESSION_TIME><START>2026-11-03T08:00:00.00</START>'
                   '<END>2026-11-03T10:00:00.00</END></SESSION_TIME><CROSS_CORRELATION_MATRIX ORDER="XYZ">'
                   '<CCM_BLOCK VECTOR_ID_ROW="V1" VECTOR_ID_COL="V2"><CORRELATIONS>0.010000,0.000000,0.000000,'
                   '0.000000,0.010000,0.000000,0.000000,0.000000,0.010000</CORRELATIONS></CCM_BLOCK>'
                   '</CROSS_CORRELATION_MATRIX></SESSION>')
    out.append("</GVX>")
    return ("\n".join(out) + "\n").encode()


def text_file():
    lines = ["# GNSS bazları (KentOS baz metni)"]
    lat, lon, h = KNOWN_AT["BN1"]
    x = [float(c) for c in ecef(lat, lon, h)]
    lines.append(f"K;BN1;{x[0]!r};{x[1]!r};{x[2]!r}")
    for v in VECTORS[:4]:
        lines.append(";".join(["B", v["from"], v["to"], *(repr(c) for c in v["d"]), *(repr(c) for c in v["covariance"])]))
    return ("\n".join(lines) + "\n").encode()


WRITTEN = {"ag.gvx": gvx(), "ag-v2.gvx": gvx(version="2.0"), "ag.txt": text_file()}


def read_gvx(data):
    """The vectors of a GVX 1.0 file as archive records (Python's ElementTree, not KentOS's reader)."""
    root = ET.fromstring(data)
    names = {p.findtext("ID"): p.findtext("NAME") for p in root.findall("POINT")}
    setups = {}
    for s in root.findall("SURVEY_SETUP"):
        kind = {"Post-processed": "postProcessed", "Single-baseRTK": "singleBaseRtk",
                "NetworkRTK": "networkRtk"}.get(s.findtext("SOLUTION_TYPE"))
        name, ver = s.findtext("PROCESSING_SOFTWARE/NAME"), s.findtext("PROCESSING_SOFTWARE/VERSION")
        setups[s.findtext("ID")] = (kind, f"{name} {ver}" if name and ver else name)
    out = []
    for v in root.findall("GNSS_VECTOR"):
        sd = [float(v.findtext(f"CORRELATION_MATRIX/SD{c}")) for c in "XYZ"]
        p = [float(v.findtext(f"CORRELATION_MATRIX/P{c}")) for c in ("XY", "XZ", "YZ")]
        rec = {"from": names[v.findtext("INITIAL_POINT_ID")], "to": names[v.findtext("TERMINAL_POINT_ID")],
               "d": [float(v.findtext(f"ECEF_DELTAS/D{c}")) for c in "XYZ"],
               "covariance": [sd[0] * sd[0], p[0] * sd[0] * sd[1], p[1] * sd[0] * sd[2], sd[1] * sd[1],
                              p[2] * sd[1] * sd[2], sd[2] * sd[2]],
               "start": v.findtext("OBSERVATION_TIME/START") + "Z", "end": v.findtext("OBSERVATION_TIME/END") + "Z"}
        kind, software = setups.get(v.findtext("SURVEY_SETUP_ID"), (None, None))
        if kind:
            rec["solution"] = kind
        if software:
            rec["software"] = software
        rec["epochs"] = int(v.findtext("QUALITY_CONTROL/EPOCHS_USED"))
        rec["satellites"] = int(v.findtext("QUALITY_CONTROL/SATELLITE_USED/TOTAL"))
        rec["source"] = v.findtext("ID")
        out.append(rec)
    crossed = sum(1 for s in root.findall("SESSION") if s.find("CROSS_CORRELATION_MATRIX") is not None)
    return out, crossed


def read_text(data):
    stations, vectors = 0, []
    for line in data.decode().splitlines():
        t = line.strip()
        if not t or t.startswith("#"):
            continue
        parts = [x.strip() for x in t.replace(",", ";").replace("\t", ";").split(";")]
        if parts[0].upper() in ("K", "N"):
            stations += 1
        elif parts[0].upper() == "B":
            v = [float(x) for x in parts[3:12]]
            vectors.append({"from": parts[1], "to": parts[2], "d": v[:3], "covariance": v[3:]})
    return vectors, stations


# ── The adjustment (ADR 1740) ────────────────────────────────────────────────────────────────────────────────────


class Refused(Exception):
    def __init__(self, code, message, path=None):
        super().__init__(code)
        self.code, self.message, self.path = code, message, path


def full(c):
    return np.array([[c[0], c[1], c[2]], [c[1], c[3], c[4]], [c[2], c[4], c[5]]])


def first_defect(n):
    """ADR 0203's Cholesky: the first unknown whose pivot is not above 1e-10 of its diagonal, or None."""
    m = n.shape[0]
    low = np.zeros((m, m))
    for k in range(m):
        s = n[k, k] - sum(low[k, j] * low[k, j] for j in range(k))
        if not s > PIVOT * n[k, k]:
            return k
        d = math.sqrt(s)
        low[k, k] = d
        for i in range(k + 1, m):
            low[i, k] = (n[i, k] - sum(low[i, j] * low[k, j] for j in range(k))) / d
    return None


def stage(names, xyz, fixed, weights, baselines, scale):
    """One stage: names in order; xyz the known X, Y, Z (or None); fixed; weights a station's X, Y, Z covariance (or
    None); the baselines (from, to, d, covariance)."""
    slot, u = [], 0
    for f in fixed:
        slot.append(None if f else u)
        u += 0 if f else 3
    # Approximations: known coordinates, else carried along the baselines (the unknowns are corrections).
    approx = {i: np.array(x) for i, x in enumerate(xyz) if x is not None}
    grew = True
    while grew:
        grew = False
        for b in baselines:
            i, j = names.index(b["from"]), names.index(b["to"])
            if i in approx and j not in approx:
                approx[j] = approx[i] + np.array(b["d"])
                grew = True
            elif j in approx and i not in approx:
                approx[i] = approx[j] - np.array(b["d"])
                grew = True
    x0 = [approx.get(i, np.zeros(3)) for i in range(len(names))]
    blocks, rows_a, rows_l = [], [], []
    for b in baselines:
        i, j = names.index(b["from"]), names.index(b["to"])
        a = np.zeros((3, u))
        for k in range(3):
            if slot[j] is not None:
                a[k, slot[j] + k] = 1.0
            if slot[i] is not None:
                a[k, slot[i] + k] = -1.0
        rows_a.append(a)
        rows_l.append(np.array(b["d"]) - (x0[j] - x0[i]))
        blocks.append(full(b["covariance"]) * scale)
    weighted = []
    for i, w in enumerate(weights):
        if w is not None and slot[i] is not None:
            a = np.zeros((3, u))
            for k in range(3):
                a[k, slot[i] + k] = 1.0
            rows_a.append(a)
            rows_l.append(np.array(xyz[i]) - x0[i])
            blocks.append(w)
            weighted.append(i)
    a = np.vstack(rows_a) if u else np.zeros((3 * len(blocks), 0))
    l = np.concatenate(rows_l)
    n_obs = a.shape[0]
    sigma = np.zeros((n_obs, n_obs))
    for k, blk in enumerate(blocks):
        sigma[3 * k:3 * k + 3, 3 * k:3 * k + 3] = blk
    p = np.linalg.inv(sigma)
    nmat = a.T @ p @ a
    bad = first_defect(nmat) if u else None
    if bad is not None:
        station = next(i for i in range(len(names)) if slot[i] is not None and slot[i] <= bad < slot[i] + 3)
        raise Refused("network_disconnected",
                      f"“{names[station]}” istasyonu datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.")
    q = np.linalg.inv(nmat) if u else np.zeros((0, 0))
    x = q @ (a.T @ p @ l) if u else np.zeros(0)
    v = a @ x - l
    omega = float(v @ p @ v)
    f = n_obs - u
    m0 = math.sqrt(omega / f) if f > 0 else None
    s0 = m0 if m0 is not None else 1.0
    comps = []
    for k, blk in enumerate(blocks):
        li = np.linalg.inv(np.linalg.cholesky(blk))
        ab = li @ a[3 * k:3 * k + 3]
        vb = li @ v[3 * k:3 * k + 3]
        qvv = np.eye(3) - ab @ q @ ab.T
        for c in range(3):
            r = qvv[c, c]
            comps.append(None if not r >= UNCONTROLLED else abs(vb[c]) / math.sqrt(r))
    coords = []
    for i in range(len(names)):
        coords.append(x0[i] + (x[slot[i]:slot[i] + 3] if slot[i] is not None else 0.0))
    return {"slot": slot, "q": q, "x": coords, "v": v, "omega": omega, "f": f, "m0": m0, "s0": s0, "comps": comps,
            "weighted": weighted, "chi2": float(chi2_dist.ppf(0.95, f)) if f > 0 else None}


def ellipse(ce, p):
    """ADR 2560: a mark's confidence region at `p` from its east, north, up covariance — numpy's eigen decomposition
    of the north, east 2×2 (semi-axes √χ²₂(p)·√λ, scipy's χ²), the major axis's direction from its eigenvector (degrees
    clockwise from north, in [0, 180)), the height's half-interval √χ²₁(p)·σu."""
    ne = np.array([[ce[1, 1], ce[1, 0]], [ce[0, 1], ce[0, 0]]])
    values, vectors = np.linalg.eigh(ne)
    k2, k1 = math.sqrt(chi2_dist.ppf(p, 2)), math.sqrt(chi2_dist.ppf(p, 1))
    axis = vectors[:, 1]
    major, minor = k2 * math.sqrt(max(values[1], 0.0)), k2 * math.sqrt(max(values[0], 0.0))
    out = {"confidence": p, "major": major, "minor": minor, "up": k1 * math.sqrt(max(ce[2, 2], 0.0))}
    # A circle's direction is rounding: given only when the axes differ by more than a millionth of the major one.
    if major - minor > 1e-6 * major:
        out["azimuth"] = math.degrees(math.atan2(axis[1], axis[0])) % 180.0
    return out


def tests(st, ids, known_ids, limit, held, names, nb):
    out, worst, blunders = [], None, 0
    for k in range(nb):
        part = st["comps"][3 * k:3 * k + 3]
        ws = [w for w in part if w is not None]
        w = max(ws) if ws else None
        flag = "uncontrolled" if not ws else ("blunder" if w > limit else "ok")
        if flag == "blunder":
            blunders += 1
            if worst is None or w > worst[0]:
                worst = (w, ids[k])
        out.append((w, flag))
    for j, i in enumerate(st["weighted"]):
        ws = [w for w in st["comps"][3 * (nb + j):3 * (nb + j) + 3] if w is not None]
        if ws and max(ws) > limit:
            blunders += 1
            if worst is None or max(ws) > worst[0]:
                worst = (max(ws), known_ids[i] or "")
    s = {"held": held, "dof": st["f"], "omega": st["omega"], "limit": limit, "blunders": blunders}
    if st["m0"] is not None:
        s.update(m0=st["m0"], chi2=st["chi2"], passed=st["omega"] <= st["chi2"])
    if worst:
        s["worst"] = worst[1]
    return s, out


def enu(p, v):
    lat, lon, _ = geodetic(p)
    return [float(x) for x in rows(lat, lon) @ np.array(v)]


def place(m, lat, lon, h, srid, epoch, campaign):
    """map_survey_cases.Model.compute's chain: the mark into the project, or the chain's refusal."""
    target = m.decision(campaign)
    with_z = "heights" in m.d["settings"]
    frm = {"srid": srid, "epoch": epoch} if epoch is not None else {"srid": srid}
    to = {"srid": 5254, "epoch": target} if target is not None else {"srid": 5254}
    if with_z:
        frm["heights"], to["heights"] = ms.ELL, ms.ELL
    point = ms.cv.pt(lon, lat, h) if with_z else ms.cv.pt(lon, lat)
    vel = campaign.get("velocity") if campaign else None
    if epoch is not None and target is not None and epoch != target:
        if vel is None:
            raise Refused("epoch_unsupported", f"Koordinatlar {ms.cv.n(epoch)} epokundan {ms.cv.n(target)} epokuna "
                                               "taşınmalı: bunun için hız modeli gerekir. Hız ızgarasını (doğu, "
                                               "kuzey, yukarı mm/yıl; GeoTIFF) Proje ayarları › Izgaralar'a ekleyip "
                                               "velocity ile verin; koordinatlar epok farkı yok sayılarak "
                                               "çevrilmez.", "operation")
        s = ms.gc.transformed([point], frm=frm, to=to, vel=vel)
    else:
        s = ms.cv.transformed([point], frm=frm, to=to)
    out = s["result"]["output"]
    p = out["points"][0]
    return {"x": p["x"], "y": p["y"], "z": p.get("z"), "chain": out["chain"]["status"], "target": target}


def rules_problem(r):
    if not r.get("id", "").strip() or not r.get("source", "").strip() or not r.get("classes"):
        return "Kural paketinin kimliği, kaynağı ve en az bir sınıfı olmalı."
    seen = set()
    for c in r["classes"]:
        if not c["id"].strip() or c["id"] in seen:
            return "Kural paketinin sınıflarının kimlikleri boş olmayan ve tek olmalı."
        seen.add(c["id"])
        if not all(math.isfinite(x) and x >= 0 for x in (c["constant"], c["ppm"])) or c["constant"] == c["ppm"] == 0:
            return "Sınıfın baz sınırı eksi olmayan sonlu iki sayıdır (m ve ppm), ikisi birden sıfır olamaz."
        if not (c["horizontal"] > 0 and c["height"] > 0):
            return "Sınıfın nokta sınırları 0'dan büyük sonlu sayılar olmalı (m)."
        if "confidence" in c and not 0 < c["confidence"] < 1:
            return "Sınıfın güveni 0 ile 1 arasında olmalı."
    return None


def similarity(common, conf):
    """ADR 2160: Madde 15(1)(c)'s 3D similarity given = T + λ·R·free of three known points or more, by Horn's
    closed-form absolute orientation; σ of the scale from the linearised normals at the solution."""
    n = len(common)
    if n < 3:
        return None
    g = np.array([c[2] for c in common], dtype=float)
    f = np.array([c[3] for c in common], dtype=float)
    gm, fm = g.mean(axis=0), f.mean(axis=0)
    gp, fp = g - gm, f - fm
    m = fp.T @ gp
    sxx, sxy, sxz = m[0]
    syx, syy, syz = m[1]
    szx, szy, szz = m[2]
    nq = np.array([[sxx + syy + szz, syz - szy, szx - sxz, sxy - syx],
                   [syz - szy, sxx - syy - szz, sxy + syx, szx + sxz],
                   [szx - sxz, sxy + syx, -sxx + syy - szz, syz + szy],
                   [sxy - syx, szx + sxz, syz + szy, -sxx - syy + szz]])
    w, v = np.linalg.eigh(nq)
    q0, qx, qy, qz = v[:, np.argmax(w)]
    r = np.array([[q0 * q0 + qx * qx - qy * qy - qz * qz, 2 * (qx * qy - q0 * qz), 2 * (qx * qz + q0 * qy)],
                  [2 * (qy * qx + q0 * qz), q0 * q0 - qx * qx + qy * qy - qz * qz, 2 * (qy * qz - q0 * qx)],
                  [2 * (qz * qx - q0 * qy), 2 * (qz * qy + q0 * qx), q0 * q0 - qx * qx - qy * qy + qz * qz]])
    rf = fp @ r.T
    lam = float((gp * rf).sum() / (fp * fp).sum())
    tr = gm - lam * (r @ fm)
    res = g - (tr + lam * (f @ r.T))
    vv = float((res * res).sum())
    dof = 3 * n - 7
    a = []
    for x in fp:
        a += [[1, 0, 0, x[0], 0, x[2], -x[1]], [0, 1, 0, x[1], -x[2], 0, x[0]], [0, 0, 1, x[2], x[1], -x[0], 0]]
    a = np.array(a, dtype=float)
    q = np.linalg.inv(a.T @ a)
    sigma = math.sqrt(vv / dof) * math.sqrt(q[3, 3])
    d = lam - 1.0
    tval = abs(d) / sigma if sigma > 0 else 0.0
    quant = float(student.ppf((1.0 + conf) / 2.0, dof))
    rad = 180.0 * 3600.0 / math.pi
    rot = [(r[2, 1] - r[1, 2]) / 2.0 * rad, (r[0, 2] - r[2, 0]) / 2.0 * rad, (r[1, 0] - r[0, 1]) / 2.0 * rad]
    residuals = []
    for (kid, mark, _, _, (lat, lon, _)), vec in zip(common, res):
        residuals.append({"id": kid, "mark": mark, "enu": [float(x) for x in rows(lat, lon) @ vec]})
    return {"known": [c[0] for c in common], "translation": [float(x) for x in tr], "rotation": rot,
            "scale": d * 1e6, "sigma": sigma * 1e6, "dof": dof, "t": tval, "quantile": quant, "agrees": tval <= quant,
            "residuals": residuals}


def scaled_known(points, ppm):
    """Known points whose coordinates are their own scaled about their mean by `ppm`, published as given() rounds."""
    xs = [ecef(p["lat"], p["lon"], p["height"]) for p in points]
    c = sum(xs) / len(xs)
    out = []
    for p, x in zip(points, xs):
        lat, lon, h = given(c + (1.0 + ppm * 1e-6) * (x - c))
        out.append(dict(p, lat=lat, lon=lon, height=h))
    return out


def decimal_year(t):
    """A GPS time YYYY-MM-DDTHH:MM:SS(.f)(Z) as a decimal year (ADR 2120 §1)."""
    t = t[:-1] if t.endswith("Z") else t
    date, time = t.split("T")
    y, mo, d = (int(x) for x in date.split("-"))
    hh, mi, ss = time.split(":")
    leap = (y % 4 == 0 and y % 100 != 0) or y % 400 == 0
    days = [31, 29 if leap else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    before = sum(days[:mo - 1]) + d - 1
    seconds = float(before * 86400 + int(hh) * 3600 + int(mi) * 60) + float(ss)
    return y + seconds / ((366.0 if leap else 365.0) * 86400.0)


def rs(v):
    """A float as Rust's Display writes it (1800.0 → 1800)."""
    s = repr(float(v))
    return s[:-2] if s.endswith(".0") else s


def epoch_text(e):
    """An epoch as a sentence writes it: two decimals, a decimal comma."""
    return f"{e:.2f}".replace(".", ",")


def analyse(m, inp, prefix=""):
    """ADR 1740's network of the model's archive: (output, the facts a write needs), or Refused."""
    path = lambda f: prefix + f  # noqa: E731
    held_wrong = "operation" if prefix else None
    if m.d["settings"].get("cadWork") != "map":
        raise Refused("not_a_map_project", "GNSS arşivi yalnız harita projesinde (KentOS Harita) olur; bu çizim harita "
                                           "projesi değil.")
    sv = m.survey
    if "campaign" in inp and m.get("campaigns", inp["campaign"]) is None:
        raise Refused("unknown_survey_record", f"GNSS arşivinde “{inp['campaign']}” kimlikli arazi ölçümü yok.",
                      path("campaign"))
    scale = inp.get("scale", 1.0)
    if not (math.isfinite(scale) and scale > 0):
        raise Refused("invalid_survey_value", "Varyans çarpanı 0'dan büyük sonlu bir sayı olmalı.", path("scale"))
    package = inp.get("rules", RULES)
    why = rules_problem(package)
    if why:
        raise Refused("invalid_survey_value", why, path("rules"))
    klass = None
    if "class" in inp:
        klass = next((c for c in package["classes"] if c["id"] == inp["class"]), None)
        if klass is None:
            raise Refused("unknown_rule_class", f"“{package['id']}” kural paketinde “{inp['class']}” sınıfı yok; "
                                                f"sınıflar: {', '.join(c['id'] for c in package['classes'])}.",
                          path("class"))
    if "epoch" in inp and not (math.isfinite(inp["epoch"]) and 1900.0 <= inp["epoch"] <= 2100.0):
        raise Refused("invalid_epoch", "Ölçme epoku 1900 ile 2100 arasında ondalık bir yıl olmalı (2026.84 gibi); "
                                       f"yazılan {rs(inp['epoch'])}.", path("epoch"))
    used = [b for b in sv.get("baselines", []) if b["adjust"] and "excluded" not in b
            and ("campaign" not in inp or b["campaign"] == inp["campaign"])]
    if not used:
        raise Refused("no_baselines", "Dengelemeye girecek baz yok: bazları içe aktarın, dışlananları geri alın ya da "
                                      "dengeleme seçimini açın.", held_wrong)
    camp = m.get("campaigns", used[0]["campaign"])
    key = lambda c: (c.get("epoch"), c.get("velocity")) if c else None  # noqa: E731
    for b in used[1:]:
        if key(m.get("campaigns", b["campaign"])) != key(camp):
            raise Refused("solutions_differ", f"“{used[0]['id']}” ile “{b['id']}” bazlarının arazi ölçümlerinin epok "
                                              "kararı ya da hız ızgarası farklı; ağ tek epok kararıyla projeye taşınır.",
                          held_wrong)
    marks = []
    for b in used:
        for x in (b["from"], b["to"]):
            if x not in marks:
                marks.append(x)
    kn = [k for k in sv.get("known", []) if "excluded" not in k and k["mark"] in marks]
    if not kn:
        raise Refused("datum_missing", "Ağın bilinen noktası yok: bazların uçlarından en az birine bilinen nokta "
                                       "(TUSAGA-Aktif, C1 …) ekleyin.", held_wrong)
    srid, epoch = kn[0]["srid"], kn[0].get("epoch")
    # ADR 2120 §1: the measurement epoch.
    measured = None
    if "epoch" in inp:
        measured = (inp["epoch"], "given")
    elif all("start" in b and "end" in b for b in used):
        total = 0.0
        for b in used:
            total += (decimal_year(b["start"]) + decimal_year(b["end"])) / 2.0
        measured = (total / len(used), "baselines")
    if measured is None:
        for k in kn:
            if k["srid"] != srid or k.get("epoch") != epoch:
                raise Refused("known_differ", f"“{kn[0]['id']}” ile “{k['id']}” bilinen noktaları aynı çerçeve ve "
                                              "epokta değil; hepsini ölçme epokunda aynı çerçevede girin ya da "
                                              "bazların gözlem zamanlarını verin.", held_wrong)
    else:
        for k in kn:
            if k["srid"] != srid:
                raise Refused("known_differ", f"“{kn[0]['id']}” ile “{k['id']}” bilinen noktaları aynı çerçevede "
                                              "değil; hepsini aynı çerçevede girin.", held_wrong)
        for k in kn:
            if "epoch" not in k:
                raise Refused("known_differ", f"“{k['id']}” bilinen noktasının epoku yok; ölçme epoku "
                                              f"{epoch_text(measured[0])} olan ağda her bilinen noktanın epoku "
                                              "verilmeli.", held_wrong)
    if srid not in GEOGRAPHIC:
        raise Refused("unknown_frame", f"EPSG:{srid} kayıttaki coğrafi bir sistem değil; bilinen noktaların "
                                       "çerçevesini (4326 WGS 84, 5252 TUREF) verin.", held_wrong)
    # ADR 2120 §2: each known point at the measurement epoch, moved along the campaign's velocity grid by PROJ.
    at, moved = {}, []
    vel = camp.get("velocity") if camp else None
    for k in kn:
        given_at = (k["lat"], k["lon"], k["height"])
        if measured is not None and k["epoch"] != measured[0]:
            years = measured[0] - k["epoch"]
            if "velocity" in k:
                # ADR 2520: the point's own velocity, linearly in the frame's Earth-centred X, Y, Z.
                now = geodetic(ecef(*given_at) + np.array(k["velocity"]) * years)
                by = "velocity"
            else:
                if vel is None:
                    raise Refused("epoch_unsupported", f"“{k['id']}” bilinen noktası {epoch_text(k['epoch'])} "
                                                       f"epokunda, ağın ölçme epoku {epoch_text(measured[0])}: Madde "
                                                       "15(1)(a) uyarınca ölçme epokuna taşınmalı. Bilinen noktaya "
                                                       "kendi hızını (yer merkezli VX, VY, VZ) verin, arazi ölçümüne "
                                                       "hız ızgarasını verin (Proje ayarları › Izgaralar) ya da "
                                                       "bilinen noktayı ölçme epokunda girin.", held_wrong)
                s = ms.gc.transformed([ms.cv.pt(k["lon"], k["lat"], k["height"])],
                                      frm={"srid": srid, "epoch": k["epoch"], "heights": ms.ELL},
                                      to={"srid": srid, "epoch": measured[0], "heights": ms.ELL}, vel=vel)
                q = s["result"]["output"]["points"][0]
                now = (q["y"], q["x"], q["z"])
                by = "grid"
            d = ecef(*now) - ecef(*given_at)
            shift = [float(v) for v in rows(k["lat"], k["lon"]) @ d]
            moved.append({"id": k["id"], "mark": k["mark"], "by": by, "from": k["epoch"], "years": years,
                          "shift": shift})
            at[k["id"]] = now
        else:
            at[k["id"]] = given_at
    epoch = measured[0] if measured is not None else epoch
    epoch_from = measured[1] if measured is not None else "known"
    hold = kn[0]
    if "hold" in inp:
        hold = next((k for k in kn if k["id"] == inp["hold"]), None)
        if hold is None:
            raise Refused("unknown_survey_record", f"“{inp['hold']}” ağın bilinen noktalarından biri değil.",
                          path("hold"))
    of = {k["mark"]: k for k in kn}
    xyz = [ecef(*at[of[x]["id"]]) if x in of else None for x in marks]
    ids = [b["id"] for b in used]
    known_ids = [of[x]["id"] if x in of else None for x in marks]
    limit = float(norm.ppf((1.0 + klass["confidence"]) / 2.0)) if klass and "confidence" in klass else BAARDA
    try:
        free = stage(marks, xyz, [x in of and of[x]["id"] == hold["id"] for x in marks], [None] * len(marks), used,
                     scale)
        weights = []
        for x in marks:
            k = of.get(x)
            if k is not None and "sigma" in k:
                r = rows(at[k["id"]][0], at[k["id"]][1])
                g = k["sigma"]
                weights.append(r.T @ np.diag([g[1] * g[1], g[0] * g[0], g[2] * g[2]]) @ r)
            else:
                weights.append(None)
        held = stage(marks, xyz, [x in of and "sigma" not in of[x] for x in marks], weights, used, scale)
    except Refused as e:
        raise Refused(e.code, e.message, held_wrong)
    free_s, free_t = tests(free, ids, known_ids, limit, [hold["mark"]], marks, len(used))
    # ADR 2160: Madde 15(1)(c)'s similarity of the known points' given coordinates and the free stage's.
    common = [(k["id"], k["mark"], ecef(*at[k["id"]]), np.array(free["x"][marks.index(k["mark"])]), at[k["id"]])
              for k in kn]
    conf = klass["confidence"] if klass and "confidence" in klass else 0.95
    similar = similarity(common, conf)
    held_s, held_t = tests(held, ids, known_ids, limit, [k["mark"] for k in kn if "sigma" not in k], marks, len(used))
    failed_b = failed_s = 0
    brows = []
    for k, b in enumerate(used):
        i, j = marks.index(b["from"]), marks.index(b["to"])
        c = b["covariance"]
        length = math.sqrt(b["d"][0] * b["d"][0] + b["d"][1] * b["d"][1] + b["d"][2] * b["d"][2])
        sig = [math.sqrt(c[0] * scale), math.sqrt(c[3] * scale), math.sqrt(c[5] * scale)]
        q, sl = free["q"], free["slot"]

        def blk(a, bb, r):
            return q[a + r, bb + r] if a is not None and bb is not None else 0.0
        adj = [math.sqrt(max((blk(sl[j], sl[j], r) + blk(sl[i], sl[i], r) - blk(sl[j], sl[i], r) - blk(sl[i], sl[j], r))
                             * free["s0"] ** 2, 0.0)) for r in range(3)]
        row = {"id": b["id"], "from": b["from"], "to": b["to"], "length": length, "sigma": sig, "adjusted": adj}
        for name, st, t in (("free", free, free_t), ("constrained", held, held_t)):
            vb = st["v"][3 * k:3 * k + 3]
            test = {"v": enu(st["x"][i], vb), "flag": t[k][1]}
            if t[k][0] is not None:
                test["w"] = t[k][0]
            row[name] = test
        if klass:
            lim = klass["constant"] + klass["ppm"] * 1e-6 * length
            ok = all(x <= lim for x in (adj if klass["baselineOf"] == "adjusted" else sig))
            row["limit"], row["rules"] = lim, ok
            failed_b += not ok
        brows.append(row)
    srows, new = [], []
    for i, x in enumerate(marks):
        lat, lon, h = geodetic(held["x"][i])
        sl = held["slot"][i]
        ce = None
        if sl is None:
            sne = [0.0, 0.0, 0.0]
        else:
            c = held["q"][sl:sl + 3, sl:sl + 3] * held["s0"] ** 2
            plat, plon, _ = geodetic(held["x"][i])
            ce = rows(plat, plon) @ c @ rows(plat, plon).T
            se = [math.sqrt(max(ce[k, k], 0.0)) for k in range(3)]
            sne = [se[1], se[0], se[2]]
        row = {"mark": x}
        if x in of:
            row["known"] = of[x]["id"]
        row.update(held=sl is None, lat=lat, lon=lon, height=h, sigma=sne)
        point = next((p for p in sv["points"] if p["mark"] == x), None)
        if point:
            row["point"] = point["id"]
        if x not in of:
            try:
                pl = place(m, lat, lon, h, srid, epoch, camp)
                row["x"], row["y"] = pl["x"], pl["y"]
                if pl["z"] is not None:
                    row["z"] = pl["z"]
                if point:
                    row["dx"], row["dy"] = pl["x"] - point["result"]["x"], pl["y"] - point["result"]["y"]
                    if pl["z"] is not None and "z" in point["result"]:
                        row["dz"] = pl["z"] - point["result"]["z"]
            except Refused as e:
                pl = e
            if klass:
                ok = sne[0] <= klass["horizontal"] and sne[1] <= klass["horizontal"] and sne[2] <= klass["height"]
                row["rules"] = ok
                failed_s += not ok
            new.append({"mark": x, "lat": lat, "lon": lon, "height": h, "sigma": sne, "placed": pl})
        if ce is not None:
            row["ellipse"] = ellipse(ce, klass.get("confidence", 0.95) if klass else 0.95)
        srows.append(row)
    # The same network accepted already: its baselines, known points, held point, factor and class, and every new
    # mark's point carrying that adjustment now.
    class_of = (klass["id"], f"{package['id']}@{package['release']}") if klass else None
    current = None
    for a in reversed(sv.get("adjustments", [])):
        theirs = (a["class"], a["rules"]) if "class" in a and "rules" in a else None
        if (a["baselines"] == ids and a["known"] == [k["id"] for k in kn] and a["hold"] == hold["id"]
                and a["scale"] == scale and a.get("epoch") == epoch and theirs == class_of and new
                and all(any(p["mark"] == n["mark"] and p["result"].get("adjustment") == a["id"] for p in sv["points"])
                        for n in new)):
            current = a["id"]
            break
    why = []
    if current:
        why.append(f"Bu ağ “{current}” dengelemesiyle kabul edildi; noktaları güncel.")
    for name, st in (("Serbest dengelemede", free_s), ("Bilinen noktalara dayalı dengelemede", held_s)):
        if st["blunders"]:
            why.append(f"{name} {st['blunders']} uyuşumsuz ölçü var (test değeri {st['limit']:.2f}'yi aşıyor); en "
                       f"kötüsü “{st.get('worst', '')}”.")
        if st.get("passed") is False:
            why.append(f"{name} model testi geçmedi: vᵀPv = {st['omega']:.2f}, χ² sınırı {st['chi2']:.2f} "
                       f"(m0 = {st['m0']:.2f}).")
    for n in new:
        if isinstance(n["placed"], Refused):
            why.append(f"“{n['mark']}” projeye taşınamadı: {n['placed'].message}")
    if not new:
        why.append("Ağda bilinen nokta olmayan işaret yok; dengelenecek nokta yok.")
    out = {"srid": srid}
    if epoch is not None:
        out["epoch"] = epoch
    out["epochFrom"] = epoch_from
    if moved:
        out["moved"] = moved
    out.update(known=[k["id"] for k in kn], baselines=brows, stations=srows, free=free_s, constrained=held_s)
    if similar:
        out["similarity"] = similar
    if klass:
        r = {"package": package["id"], "release": package["release"], "class": klass["id"],
             "articles": klass["articles"]}
        if "confidence" in klass:
            r["confidence"] = klass["confidence"]
        holds = abs(similar["scale"]) <= klass["similarity"] if similar and "similarity" in klass else None
        r.update(baselineOf=klass["baselineOf"], passed=not failed_b and not failed_s and holds is not False,
                 baselines=failed_b, stations=failed_s)
        if holds is not None:
            r["similarity"] = holds
        out["rules"] = r
    if current:
        out["current"] = current
    out["acceptable"] = not why
    if why:
        out["why"] = why
    out["revision"] = "$current"
    facts = {"ids": ids, "known": [k["id"] for k in kn], "hold": hold["id"], "scale": scale, "class": klass,
             "package": package, "new": new, "camp": camp}
    return out, facts


def warnings_of(out, prefix=""):
    w = []
    r = out.get("rules")
    if r and not r["passed"]:
        scale = ""
        if r.get("similarity") is False and "similarity" in out:
            scale = f"; benzerlik dönüşümünün ölçeği sınırı aşıyor (|1 − λ| = {abs(out['similarity']['scale']):.2f} ppm)"
        w.append({"code": "network_rules_failed", "message": f"{r['class']} sınıfı ({r['package']} {r['articles']}) "
                                                             f"tutmuyor: {r['baselines']} baz ve {r['stations']} "
                                                             f"nokta sınırı aşıyor{scale}.", "path": prefix + "class"})
    if out["free"]["dof"] == 0:
        w.append({"code": "network_unchecked", "message": "Ağda fazla ölçü yok; uyuşumsuz baz ve model sınanamaz."})
    return w


# ── Cases ────────────────────────────────────────────────────────────────────────────────────────────────────────

SAME = {"revision": "same", "canUndo": False}
WITHIN = 1e-6


def failed(e):
    err = {"code": e.code, "message": e.message}
    if e.path is not None:
        err["path"] = e.path
    return {"status": "failed", "error": err}


def ask(m, inp, op="execute", within=WITHIN):
    try:
        out, _ = analyse(m, inp)
    except Refused as e:
        return {"op": op, "input": inp, "result": failed(e), "expect": SAME}
    result = {"status": "completed", "output": None if op == "validate" else out, "warnings": warnings_of(out)}
    return {"op": op, "input": inp, "result": result, "expect": SAME, "within": within}


def case(name, m, steps, note=None):
    c = {"name": name}
    if note:
        c["note"] = note
    c["setup"] = m.document()
    c["steps"] = steps
    return c


def with_errors(errs):
    return [record(k + 1, vector(f, t, errs.get((f, t), ERRS[(f, t)]))) for k, (f, t) in enumerate(PAIRS)]


def network_cases():
    base = networked()
    out = [case("İki bilinen nokta, üç işaret, sekiz baz: serbest ve bilinen noktalara dayalı dengeleme; N1'in kabul "
                "edilen noktasından farkı", base, [ask(base, {})],
                "Serbest dengelemede BN1 tutulur, BN2 bilinmeyendir; dayalı dengelemede ikisi de sabittir.")]
    # ADR 2560: baselines whose east and north deviations differ give the marks' ellipses a direction.
    oblong = networked(baselines=[record(k + 1, vector(f, t, ERRS[(f, t)], sig=(0.0015, 0.004, 0.005), turn=30.0))
                                  for k, (f, t) in enumerate(PAIRS)])
    out.append(case("Bazların sapmaları 1,5 ve 4 mm, eksenleri 30° dönük: noktaların hata elipsi o yöne uzanır, yönü "
                    "verilir", oblong, [ask(oblong, {})]))
    for klass, note in (("C1", "Güven 0,95: w > 1,96 uyuşumsuz; baz σ'sı serbest dengelemenin (Madde 15)."),
                        ("C2", "Baz σ'sı işlenmiş σ, ≤ 10 mm + 1,5 ppm; nokta σφ, σλ ≤ 4 cm, σh ≤ 5 cm (Madde 20)."),
                        ("C3", "Nokta σφ, σλ ≤ 5 cm, σh ≤ 6 cm (Madde 22).")):
        out.append(case(f"BÖHHBÜY {klass} sınıfı", base, [ask(base, {"class": klass})], note))
    out.append(case("Varyans çarpanı 100: işlenmiş σ'lar C2'nin baz sınırını aşar; ağ yine kabul edilebilir, uyarı "
                    "söyler", base, [ask(base, {"class": "C2", "scale": 100.0})]))
    out.append(case("Varyans çarpanı 0,07: m0 büyür, model testi geçmez, uyuşumsuz baz yok; kabul edilemez", base,
                    [ask(base, {"scale": 0.07})]))
    blunder = networked(baselines=with_errors({("N1", "N4"): (0.0008, -0.0012, 0.0809)}))
    out.append(case("N1 → N4 bazında 8 cm yükseklik hatası: serbest dengeleme uyuşumsuz bazı bulur; kabul edilemez",
                    blunder, [ask(blunder, {})]))
    weighted = networked(known_points=[KNOWN[0], known(2, "BN2", sigma=(0.01, 0.01, 0.02),
                                                       source="C1 noktası (örnek), σ 1/1/2 cm")])
    out.append(case("BN2 ağırlıklı bilinen nokta (σ kuzey/doğu 1 cm, yukarı 2 cm): ECEF'e döndürülmüş tam kovaryans",
                    weighted, [ask(weighted, {})]))
    out.append(case("Serbest dengelemede BN2 tutulur (hold)", base, [ask(base, {"hold": "bn-2"})]))
    ex = networked()
    ex.survey["baselines"][7]["excluded"] = "Ağaç altında; float'a düştü"
    ex.survey["baselines"][6]["adjust"] = False
    out.append(case("Dışlanan ve dengelemeden çıkarılan baz girmez", ex, [ask(ex, {})]))
    two = networked()
    two.survey["campaigns"].append({"id": "kp-2", "name": "GNSS ölçümü 2026-12", "epoch": 2005.0})
    two.survey["baselines"].append(record(9, vector("N4", "N5", (0.0004, -0.0003, 0.0010)), campaign="kp-2"))
    out.append(case("İki arazi ölçümü: yalnız kp-1'in bazları (campaign)", two, [ask(two, {"campaign": "kp-1"}),
                                                                                  ask(two, {})]))
    tree = networked(baselines=[record(1, VECTORS[0]), record(2, VECTORS[2])], known_points=[KNOWN[0]])
    out.append(case("Fazla ölçü yok (BN1'den iki baz): m0 ve χ² yok, bazlar denetlenemez; uyarı", tree,
                    [ask(tree, {})]))
    measured = networked(known_points=[known(1, "BN1", epoch=2026.84), known(2, "BN2", epoch=2026.84)])
    out.append(case("Bilinen noktalar ölçme epokunda (2026,84), karar 2005,0, hız ızgarası yok: işaretler projeye "
                    "taşınamaz, kabul edilemez", measured, [ask(measured, {})]))
    accepted, _, _, _, _ = accept(base, {})
    accepted = materialized(accepted)
    out.append(case("Kabul edilmiş ağ yeniden kabul edilemez: noktaları onun dengelemesini taşıyor", accepted,
                    [ask(accepted, {})]))
    later = accepted.clone()
    later.survey["baselines"][7]["excluded"] = "Yeniden işlenecek"
    out.append(case("Kabulden sonra bir baz dışlandı: ağ değişti, yeniden kabul edilebilir", later, [ask(later, {})]))
    # ADR 2160: Madde 15(1)(c)'s similarity with three known points; and the same points scaled by 5 ppm.
    three_known = KNOWN + [known(3, "N5", source="TUSAGA-Aktif koordinatı (örnek), TUREF, 2005,0")]
    three = networked(known_points=three_known)
    out.append(case("Üç bilinen nokta: 15(1)(c) benzerlik dönüşümü; ölçek C1'in ±3 ppm'i içinde, istatistikle uyumlu",
                    three, [ask(three, {"class": "C1"}, within=1e-5)],
                    "t, ölçek farkının (10⁻⁷) kendi σ'sına (10⁻⁸) bölümüdür: serbest aşamanın koordinatlarındaki 10⁻⁹ m'lik "
                    "çözücü farkı onda 10⁻⁶'yı aşar; sayılar 1e-5 içinde karşılaştırılır."))
    stretched = networked(known_points=scaled_known(three_known, 5.0))
    out.append(case("Bilinen noktalar 5 ppm ölçekli: benzerlik dönüşümünün ölçeği C1'in sınırını aşar", stretched,
                    [ask(stretched, {"class": "C1"}, within=1e-4)],
                    "Uyuşmayan ağda vᵀPv yüzlerce olur; çözücülerin yuvarlaması orada 1e-6'yı aşabildiği için sayılar "
                    "1e-4 içinde karşılaştırılır."))
    # ADR 2120: the measurement epoch from the baselines' observation times (or given) and the known points moved to
    # it along the campaign's velocity grid (BÖHHBÜY Madde 15(1)(a)); the new marks go back to the decision, 2005.0.
    for name, m, inp in timed_cases():
        out.append(case(name, m, [ask(m, inp)]))
    out.append(case("Plan aynı cevabı verir", base, [ask(base, {"class": "C2"}, op="plan")]))
    out.append(case("Doğrulama", base, [ask(base, {}, op="validate")]))
    # Refusals, in their order.
    arch = ms.setup(work="architecture", survey=None)
    out.append(case("Harita projesi değil", arch, [ask(arch, {})]))
    out.append(case("Olmayan arazi ölçümü", base, [ask(base, {"campaign": "kp-9"})]))
    out.append(case("Varyans çarpanı sıfır", base, [ask(base, {"scale": 0.0})]))
    bad = copy.deepcopy(RULES)
    bad["classes"] = []
    out.append(case("Kural paketinin sınıfı yok", base, [ask(base, {"rules": bad, "class": "C1"})]))
    out.append(case("Pakette olmayan sınıf", base, [ask(base, {"class": "C4"})]))
    out.append(case("Ölçme epoku 1900–2100 dışında", base, [ask(base, {"epoch": 1800.0})]))
    none = networked()
    for b in none.survey["baselines"]:
        b["adjust"] = False
    out.append(case("Dengelemeye girecek baz yok", none, [ask(none, {})]))
    differ = networked()
    differ.survey["campaigns"].append({"id": "kp-2", "name": "GNSS ölçümü 2027", "epoch": 2027.0})
    differ.survey["baselines"][7]["campaign"] = "kp-2"
    out.append(case("Bazların arazi ölçümlerinin epok kararı farklı", differ, [ask(differ, {})]))
    nok = networked(known_points=[])
    out.append(case("Bilinen nokta yok", nok, [ask(nok, {})]))
    mixed = networked(known_points=[KNOWN[0], known(2, "BN2", epoch=2026.84)])
    out.append(case("Bilinen noktaların epokları farklı, bazların gözlem zamanı yok", mixed, [ask(mixed, {})]))
    unmoved = timed(velocity=False)
    out.append(case("Bilinen noktalar 2005,0'da, ölçme epoku bazlardan, arazi ölçümünün hız ızgarası yok", unmoved,
                    [ask(unmoved, {})]))
    no_epoch = timed(known_points=[KNOWN[0], known(2, "BN2", epoch=None)])
    out.append(case("Ölçme epoku olan ağda epoku olmayan bilinen nokta", no_epoch, [ask(no_epoch, {})]))
    proj = networked(known_points=[known(1, "BN1", srid=5254), known(2, "BN2", srid=5254)])
    out.append(case("Bilinen noktaların çerçevesi coğrafi değil (TM30)", proj, [ask(proj, {})]))
    out.append(case("Tutulacak nokta ağın bilinen noktası değil", base, [ask(base, {"hold": "bn-9"})]))
    apart = networked()
    apart.survey["baselines"].append(record(9, {"from": "N6", "to": "N7", "d": [1200.5, -300.25, 410.0],
                                                "covariance": VECTORS[0]["covariance"]}))
    out.append(case("N6 → N7 ağa bağlı değil", apart, [ask(apart, {})]))
    twice = networked()
    twice.survey["baselines"].append(record(9, {"from": "N4", "to": "n4", "d": [0.5, 0.25, 0.125],
                                                "covariance": VECTORS[0]["covariance"]}))
    twice_out = [ask(twice, {})]
    twice_out[0]["result"] = {"status": "failed", "error": {"code": "duplicate_station",
                                                            "message": "“n4” istasyonu iki kez verilmiş."}}
    out.append(case("Büyük küçük harfle ayrılan iki işaret aynı istasyon sayılır", twice, twice_out,
                    "KentOS adları Türkçe katlayarak karşılaştırır (ADR 0203 §2); n4 ile N4 bir istasyondur."))
    return out


# ── ADR 2120: the measurement epoch ──────────────────────────────────────────────────────────────────────────────

VEL = {"name": ms.HIZ["name"], "sha256": ms.HIZ["sha256"]}


def times(k):
    """The k-th baseline's observation (GPS time): on 2 November 2026, an hour and a half each, one an hour apart."""
    return {"start": f"2026-11-02T{8 + k:02d}:00:00Z", "end": f"2026-11-02T{9 + k:02d}:30:00Z"}


def timed(velocity=True, known_points=None):
    """The network's baselines with their observation times; the campaign with the velocity grid when asked."""
    m = networked(baselines=[record(k + 1, v, **times(k)) for k, v in enumerate(VECTORS)], known_points=known_points)
    if velocity:
        m.survey["campaigns"][0]["velocity"] = dict(VEL)
    return m


def at_epoch(mark, epoch):
    """A known point's published coordinates moved from 2005.0 to `epoch` along the velocity grid by PROJ, rounded as
    published (a CORS coordinate at the observation time)."""
    lat, lon, h = KNOWN_AT[mark]
    s = ms.gc.transformed([ms.cv.pt(lon, lat, h)], frm={"srid": 5252, "epoch": 2005.0, "heights": ms.ELL},
                          to={"srid": 5252, "epoch": epoch, "heights": ms.ELL}, vel=VEL)
    q = s["result"]["output"]["points"][0]
    return {"lat": round(q["y"], 9), "lon": round(q["x"], 9), "height": round(q["z"], 4)}


def grid_velocity(mark):
    """The campaign's velocity grid at a known point's place as its own Earth-centred velocity (m/yr): PROJ's move over
    one year (its deformation step is linear in time), rounded to 0.01 mm/yr as a station table prints it."""
    lat, lon, h = KNOWN_AT[mark]
    s = ms.gc.transformed([ms.cv.pt(lon, lat, h)], frm={"srid": 5252, "epoch": 2005.0, "heights": ms.ELL},
                          to={"srid": 5252, "epoch": 2006.0, "heights": ms.ELL}, vel=VEL)
    q = s["result"]["output"]["points"][0]
    return [round(float(v), 5) for v in ecef(q["y"], q["x"], q["z"]) - ecef(lat, lon, h)]


OWN = {}


def own(k, mark, **extra):
    """A known point at 2005.0 with its own velocity, as a station's published one (worked out once the grid is
    written)."""
    if mark not in OWN:
        OWN[mark] = grid_velocity(mark)
    return known(k, mark, velocity=OWN[mark], source="TUSAGA-Aktif koordinatı ve hızı (örnek), TUREF, 2005,0", **extra)


def timed_cases():
    moving = timed()
    two_epochs = timed(known_points=[KNOWN[0], known(2, "BN2", epoch=2026.84, **at_epoch("BN2", 2026.84),
                                                     source="TUSAGA-Aktif günlük çözümü (örnek), 2026,84")])
    own_only = timed(velocity=False, known_points=[own(1, "BN1"), own(2, "BN2")])
    own_undecided = copy.deepcopy(own_only)
    own_undecided.survey["campaigns"][0].pop("epoch")
    own_and_grid = timed(known_points=[own(1, "BN1"), KNOWN[1]])
    return [
        ("Bilinen noktaların kendi hızları (yer merkezli, m/yıl), hız ızgarası yok: ikisi de hızıyla ölçme epokuna "
         "taşınır, ağ uyuşur; yeni noktalar arazi ölçümünün kararına (2005,0) hız modelsiz taşınamaz", own_only,
         {"class": "C2"}),
        ("Bilinen noktaların kendi hızları, arazi ölçümünün epok kararı yok: yeni noktalar ölçme epokunda projeye "
         "taşınır, ağ kabul edilebilir", own_undecided, {"class": "C2"}),
        ("BN1 kendi hızıyla, BN2 arazi ölçümünün hız ızgarasıyla taşınır", own_and_grid, {}),
        ("Ölçme epoku bazların gözlem zamanlarından (2 Kasım 2026); 2005,0'daki bilinen noktalar hız ızgarasıyla oraya "
         "taşınır, yeni noktalar karara (2005,0) döner", moving, {}),
        ("Ölçme epoku verilir (2026,5): bazların zamanından önce gelir", moving, {"epoch": 2026.5}),
        ("Bilinen noktaların epokları farklı (BN1 2005,0'da, BN2 2026,84'te): ikisi de ölçme epokuna taşınır, ağ "
         "uyuşur", two_epochs, {"class": "C2"}),
    ]


# ── map.survey.edit's network operations ─────────────────────────────────────────────────────────────────────────


def any_numbers(survey):
    """The archive as a case expects it: the projected x, y, z, a network result's latitude, longitude, height and σ
    and an adjustment's m0 anything (they are compared in map.survey.network's answers within 1e-6)."""
    s = ms.with_any_xy(survey)
    for p in s.get("points", []):
        for r in [p["result"], *p.get("history", [])]:
            if r.get("method") == "network":
                for k in ("lat", "lon", "height", "sigma"):
                    if k in r:
                        r[k] = "$any"
    for a in s.get("adjustments", []):
        for k in ("freeM0", "m0"):
            if k in a:
                a[k] = "$any"
    return s


def edit(input_, result, expect, within=None):
    s = {"op": "execute", "input": input_, "result": result, "command": "map.survey.edit", "expect": expect}
    if within is not None:
        s["within"] = within
    return s


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def edited(m, **kw):
    out = {k: v for k, v in kw.items() if v}
    out["stale"] = m.stale()
    out["revision"] = "$current"
    return out


def write(before, after, operation, fields, label, warnings=(), extra=None, then_undo=True):
    expect = {"revision": "changed", "canUndo": True, "settings": {"mapSurvey": any_numbers(after.survey)}}
    expect.update(extra or {})
    steps = [edit({"operation": operation}, done(edited(after, **fields), warnings), expect, within=WITHIN)]
    if then_undo:
        steps.append({"op": "undo", "returns": label,
                      "expect": {"canRedo": True, "settings": {"mapSurvey": any_numbers(before.survey)},
                                 "ids": sorted(before.entities)}})
    return steps


def refusal(operation, code, message, path, after_write=False):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    return edit({"operation": operation}, {"status": "failed", "error": err}, {"revision": "same"} if after_write else SAME)


def rust_number(v):
    """A number as Rust's `{}` writes it: the shortest that reads back, no `.0` on a whole number."""
    return str(int(v)) if float(v).is_integer() else repr(float(v))


def convergence(x, y):
    """ADR 2960: the angle (radians, counter-clockwise) from TM30's up to the meridian through (x, y): the meridian's
    direction on the map from two points 0.0001° of latitude either side, PROJ projecting them."""
    to_geo = pyproj.Transformer.from_crs(5254, 5252, always_xy=True)
    to_map = pyproj.Transformer.from_crs(5252, 5254, always_xy=True)
    lon, lat = to_geo.transform(x, y)
    (x1, y1), (x2, y2) = to_map.transform(lon, lat - 1e-4), to_map.transform(lon, lat + 1e-4)
    return math.atan2(-(x2 - x1), y2 - y1)


def ellipse_entity(mark, da, at, e, with_):
    """ADR 2960: the error ellipse of a mark's region `e` at its object's place, magnified, its major axis turned from
    its azimuth (true north) to the map (the convergence taken off); a circle's drawn up the map."""
    bearing = math.radians(e["azimuth"]) - convergence(*at) if "azimuth" in e else 0.0
    a = e["major"] * with_["scale"]
    attrs = {"İşaret": mark, "Dengeleme": da, "Güven": rust_number(e["confidence"]),
             "Büyük yarı eksen (m)": f"{e['major']:.4f}", "Küçük yarı eksen (m)": f"{e['minor']:.4f}",
             "Yön (°)": f"{e['azimuth']:.4f}" if "azimuth" in e else "", "Büyütme": rust_number(with_["scale"])}
    return {"kind": "ellipse", "layerId": with_["layer"], "attrs": attrs, "c": {"x": at[0], "y": at[1]},
            "major": {"x": a * math.sin(bearing), "y": a * math.cos(bearing)},
            "ratio": e["minor"] / e["major"] if e["major"] > 0 else 1.0, "t0": 0.0, "t1": 2 * math.pi}


def height_bar(mark, da, at, e, with_):
    """ADR 3360: a point's height half-interval u (magnified as its ellipse) as a bar through the point: a polyline from
    (x − u/5, y + u) to (x + u/5, y + u), back to (x, y + u), down to (x, y − u), and the bottom tick (x − u/5, y − u) to
    (x + u/5, y − u)."""
    u = e["up"] * with_["scale"]
    t = u / 5.0
    x, y = at
    attrs = {"İşaret": mark, "Dengeleme": da, "Güven": rust_number(e["confidence"]),
             "Kot yarı aralığı (m)": f"{e['up']:.4f}", "Büyütme": rust_number(with_["scale"])}
    pts = [(x - t, y + u), (x + t, y + u), (x, y + u), (x, y - u), (x - t, y - u), (x + t, y - u)]
    return {"kind": "polyline", "layerId": with_["layer"], "attrs": attrs, "pts": [{"x": a, "y": b} for a, b in pts]}


def ellipse_label(mark, da, at, e, with_, plot_scale):
    """ADR 3760: an ellipse's label: its semi-axes in millimetres to a tenth (with the bars the half-interval too),
    the label height on paper (2.5 mm when not given) at the plot scale, its middle left at the point's place plus the
    magnified semi-major axis and the height."""
    h = with_.get("labelHeight", 2.5) * plot_scale / 1000.0
    text = f"a {e['major'] * 1000.0:.1f} · b {e['minor'] * 1000.0:.1f} mm"
    if with_.get("heights"):
        text += f" · h ±{e['up'] * 1000.0:.1f} mm"
    return {"kind": "text", "layerId": with_["layer"], "attrs": {"İşaret": mark, "Dengeleme": da},
            "p": {"x": at[0] + e["major"] * with_["scale"] + h, "y": at[1]}, "text": text, "height": h, "rotation": 0.0,
            "align": "middleLeft"}


def accept(m, inp, layer="gnss", ellipses=None):
    """map.survey.edit network: the archive after (da-N, the points), the new and moved objects, the records; with
    `ellipses` ({layer, scale}) each point's error ellipse and the use that records them (ADR 2960)."""
    out, facts = analyse(m, inp, "operation.")
    after = m.clone()
    sv = after.survey
    da = f"da-{1 + max([int(a['id'][3:]) for a in sv.get('adjustments', [])] or [0])}"
    records, fields, slot = [], {}, max(after.entities) + 1
    for n in facts["new"]:
        pl = n["placed"]
        r = {"revision": 1, "solutions": [], "method": "network", "lat": n["lat"], "lon": n["lon"],
             "height": n["height"], "srid": out["srid"]}
        if "epoch" in out:
            r["epoch"] = out["epoch"]
        if pl["target"] is not None:
            r["targetEpoch"] = pl["target"]
        r.update(antenna=[], sigma=n["sigma"], x=pl["x"], y=pl["y"])
        if pl["z"] is not None:
            r["z"] = pl["z"]
        r.update(chain=pl["chain"], adjustment=da)
        old = next((p for p in sv["points"] if p["mark"] == n["mark"]), None)
        if old:
            r["revision"] = old["result"]["revision"] + 1
            r["reasons"] = [f"“{da}” ağ dengelemesiyle üretildi."]
            old.setdefault("history", []).append(old["result"])
            old["result"] = r
            s = next(k for k, u in after.uids.items() if u == old["uid"])
            e = after.entities[s]
            e["p"] = {"x": r["x"], "y": r["y"]}
            if "z" in r:
                e["z"] = r["z"]
            e["attrs"].update({"Revizyon": str(r["revision"]), "Yöntem": "Ağ dengelemesi", "Çözüm": da})
            fields[str(s)] = {"layerId": e["layerId"], "p": e["p"], "attrs": e["attrs"], **({"z": e["z"]} if "z" in e else {})}
            records.append(old["id"])
        else:
            pid = f"kn-{1 + max([int(p['id'][3:]) for p in sv['points']] or [0])}"
            sv["points"].append({"id": pid, "mark": n["mark"], "uid": f"$uidOf:{slot}", "result": r})
            slot += 1
            records.append(pid)
    a = {"id": da, "baselines": facts["ids"], "known": facts["known"], "hold": facts["hold"], "scale": facts["scale"]}
    if facts["class"]:
        a.update({"class": facts["class"]["id"], "rules": f"{facts['package']['id']}@{facts['package']['release']}",
                  "rulesPassed": out["rules"]["passed"]})
    if "m0" in out["free"]:
        a["freeM0"] = out["free"]["m0"]
    a["freeDof"] = out["free"]["dof"]
    if "m0" in out["constrained"]:
        a["m0"] = out["constrained"]["m0"]
    a.update(dof=out["constrained"]["dof"], srid=out["srid"])
    if "epoch" in out:
        a["epoch"] = out["epoch"]
    sv.setdefault("adjustments", []).append(a)
    for p in sv["points"]:
        if str(p["uid"]).startswith("$uidOf:"):
            at = int(p["uid"].split(":")[1])
            e = ms.point_entity(layer, p["result"], p["mark"], p["id"])
            e["id"] = at
            after.entities[at] = e
            # The object's id is the drawing's; the archive names it by the same placeholder, so the model finds it.
            after.uids[at] = p["uid"]
            fields[str(at)] = {"layerId": layer, "p": e["p"], "attrs": e["attrs"], **({"z": e["z"]} if "z" in e else {})}
    if ellipses:
        rows = {r["mark"]: r for r in out["stations"]}
        uids, points = [], []
        for n in facts["new"]:
            e = rows[n["mark"]].get("ellipse")
            if e is None:
                continue
            at = max(after.entities) + 1
            ent = ellipse_entity(n["mark"], da, (n["placed"]["x"], n["placed"]["y"]), e, ellipses)
            ent["id"] = at
            after.entities[at] = ent
            after.uids[at] = f"$uidOf:{at}"
            uids.append(f"$uidOf:{at}")
            fields[str(at)] = {k: ent[k] for k in ("layerId", "attrs", "c", "major", "ratio", "t0", "t1")}
            # ADR 3360: the height's half-interval as a bar beside it, recorded with it.
            if ellipses.get("heights"):
                bt = max(after.entities) + 1
                bar = height_bar(n["mark"], da, (n["placed"]["x"], n["placed"]["y"]), e, ellipses)
                bar["id"] = bt
                after.entities[bt] = bar
                after.uids[bt] = f"$uidOf:{bt}"
                uids.append(f"$uidOf:{bt}")
                fields[str(bt)] = {k: bar[k] for k in ("layerId", "attrs", "pts")}
            # ADR 3760: its label beside it, recorded with it.
            if ellipses.get("labels"):
                lt = max(after.entities) + 1
                lab = ellipse_label(n["mark"], da, (n["placed"]["x"], n["placed"]["y"]), e, ellipses,
                                    after.d["settings"]["plotScale"])
                lab["id"] = lt
                after.entities[lt] = lab
                after.uids[lt] = f"$uidOf:{lt}"
                uids.append(f"$uidOf:{lt}")
                fields[str(lt)] = {k: lab[k] for k in ("layerId", "attrs", "p", "text", "height", "rotation", "align")}
            p = next(p for p in sv["points"] if p["mark"] == n["mark"])
            points.append({"point": p["id"], "revision": p["result"]["revision"]})
        kb = f"kb-{1 + max([int(u['id'][3:]) for u in sv['uses']] or [0])}"
        sv["uses"].append({"id": kb, "operation": f"Hata elipsleri ({da})", "uids": uids, "points": points})
    return after, out, da, records, fields


def materialized(m):
    """A model after a write as a setup can hold it: each object's placeholder id (`$uidOf:N`) a real one."""
    m = m.clone()
    real = {}
    for slot, u in list(m.uids.items()):
        if str(u).startswith("$uidOf:"):
            real[u] = ms.uid(0x1740 + slot)
            m.uids[slot] = real[u]
    for p in m.survey["points"]:
        if p["uid"] in real:
            p["uid"] = real[p["uid"]]
    return m


def edit_cases():
    out = []
    base = BASE
    # baselines from a GVX file and from KentOS's text
    vecs, crossed = read_gvx(WRITTEN["ag.gvx"])
    after = base.clone()
    after.survey["baselines"] = [dict(v, id=f"bz-{k + 1}", campaign="kp-1",
                                      file={"path": fx("ag.gvx"), "revision": sha(WRITTEN["ag.gvx"]), "format": "gvx"},
                                      adjust=True) for k, v in enumerate(vecs)]
    ids = [b["id"] for b in after.survey["baselines"]]
    out.append(ms.case("GVX 1.0 dosyasından sekiz baz: işaretten işarete ECEF bileşenleri, korelasyon matrisinden "
                       "kovaryans, GPS zamanı, çözüm türü ve yazılım; oturumun çapraz korelasyonları okunmaz",
                       write(base, after, {"op": "baselines", "campaign": "kp-1", "file": fx("ag.gvx")},
                             {"id": "bz-1", "records": ids}, "Baz ekle",
                             warnings=[{"code": "gvx_correlations_ignored",
                                        "message": f"{crossed} oturumun vektörler arası çapraz korelasyonları okunmadı; "
                                                   "vektörler birbirinden bağımsız dengelenir.",
                                        "path": "operation.file"}])))
    vecs, stations = read_text(WRITTEN["ag.txt"])
    after = base.clone()
    after.survey["baselines"] = [dict(v, id=f"bz-{k + 1}", campaign="kp-1", source=f"B{k + 1}",
                                      file={"path": fx("ag.txt"), "revision": sha(WRITTEN["ag.txt"]), "format": "text"},
                                      adjust=True) for k, v in enumerate(vecs)]
    out.append(ms.case("Baz metninden dört baz; K satırı arşive alınmaz, uyarı söyler",
                       write(base, after, {"op": "baselines", "campaign": "kp-1", "file": fx("ag.txt")},
                             {"id": "bz-1", "records": [b["id"] for b in after.survey["baselines"]]}, "Baz ekle",
                             warnings=[{"code": "baseline_text_stations",
                                        "message": f"Baz metninin {stations} K/N satırı arşive alınmadı; bilinen noktaları "
                                                   "çerçevesi ve epokuyla Bilinen nokta ekle ile girin.",
                                        "path": "operation.file"}])))
    out.append(ms.case("GVX 2.0 okunmaz", [refusal({"op": "baselines", "campaign": "kp-1", "file": fx("ag-v2.gvx")},
                                                    "unreadable_gnss_file",
                                                    "GVX 2.0 sürümü okunmaz; KentOS GVX 1.0'ı (NGS, 4 Şubat 2021) okur.",
                                                    "operation.file")]))
    out.append(ms.case("Bazlar: olmayan arazi ölçümü", [refusal(
        {"op": "baselines", "campaign": "kp-9", "file": fx("ag.gvx")}, "unknown_survey_record",
        "GNSS arşivinde “kp-9” kimlikli arazi ölçümü yok.", "operation.campaign")]))
    one = VECTORS[0]
    manual = {"op": "baseline", "campaign": "kp-1", "from": "BN1", "to": "N1", "d": one["d"],
              "covariance": one["covariance"], "start": "2026-11-03T08:00:00Z", "end": "2026-11-03T10:00:00Z",
              "solution": "postProcessed", "recipe": "TBC 5.90, kesin yörünge, %10 maske"}
    after = base.clone()
    after.survey["baselines"] = [record(1, one, start=manual["start"], end=manual["end"], solution="postProcessed",
                                        recipe=manual["recipe"])]
    out.append(ms.case("Elle bir baz", write(base, after, manual, {"id": "bz-1"}, "Baz ekle")))
    out.append(ms.case("Elle baz: iki ucu aynı işaret", [refusal(dict(manual, to="BN1"), "invalid_survey_value",
                                                                 "Bazın iki ucu aynı işaret olamaz.", "operation.to")]))
    out.append(ms.case("Elle baz: kovaryans pozitif tanımlı değil", [refusal(
        dict(manual, covariance=[1e-05, 2e-05, 0.0, 1e-05, 0.0, 4e-05]), "invalid_survey_value",
        "Baz: bazın kovaryansı pozitif tanımlı olmalı (köşegen, 2×2 ve 3×3 öncü küçükler 0'dan büyük).",
        "operation.covariance")]))
    # known points
    k1 = KNOWN[0]
    op_known = {"op": "known", "mark": "BN1", "lat": k1["lat"], "lon": k1["lon"], "height": k1["height"],
                "srid": 5252, "epoch": 2005.0, "source": k1["source"]}
    after = base.clone()
    after.survey["known"] = [copy.deepcopy(k1)]
    out.append(ms.case("Bilinen nokta: TUREF'te, ölçme epokunda; sabit", write(base, after, op_known, {"id": "bn-1"},
                                                                          "Bilinen nokta ekle")))
    out.append(ms.case("Bilinen nokta: projeksiyonlu çerçeve", [refusal(
        dict(op_known, srid=5254), "unknown_frame",
        "EPSG:5254 kayıttaki coğrafi bir sistem değil; çözümün çerçevesini (4326 WGS 84, 5252 TUREF) verin.",
        "operation.srid")]))
    out.append(ms.case("Bilinen nokta: enlem aralık dışında", [refusal(
        dict(op_known, lat=95.0), "invalid_survey_value", "Bilinen nokta: enlem −90 ile 90 derece arasında olmalı.",
        "operation.lat")]))
    # ADR 2520: a known point's own velocity (Earth-centred, m/yr), from its epoch.
    moving = own(1, "BN1")
    after = base.clone()
    after.survey["known"] = [copy.deepcopy(moving)]
    op_moving = dict(op_known, velocity=moving["velocity"], source=moving["source"])
    out.append(ms.case("Bilinen nokta: kendi hızıyla (yer merkezli VX, VY, VZ, m/yıl), 2005,0'da",
                       write(base, after, op_moving, {"id": "bn-1"}, "Bilinen nokta ekle")))
    no_epoch = {k: v for k, v in op_moving.items() if k != "epoch"}
    out.append(ms.case("Bilinen nokta: hızı var, epoku yok", [refusal(
        no_epoch, "invalid_survey_value",
        "Bilinen nokta: hızı olan noktanın epoku verilmeli: hız noktayı o epoktan taşır.", "operation.velocity")]))
    out.append(ms.case("Bilinen nokta: hız 1 m/yıldan büyük (mm/yıl m/yıl diye yazılmış)", [refusal(
        dict(op_moving, velocity=[-10.5, 9.8, 11.2]), "invalid_survey_value",
        "Bilinen nokta: hız yer merkezli VX, VY, VZ olarak yılda metredir; her bileşen en çok 1 m/yıl olan sonlu bir "
        "sayı olmalı.", "operation.velocity")]))
    with_known = networked()
    out.append(ms.case("Bilinen nokta: işaretin dışlanmamış bilinen noktası var", [refusal(
        op_known, "invalid_survey_value",
        "“BN1” işaretinin dışlanmamış bilinen noktası var (“bn-1”); önce onu dışlayın.", "operation.mark")],
        d=with_known))
    # exclude, include, adjust on baselines and known points
    after = with_known.clone()
    after.survey["baselines"][4]["excluded"] = "Çok yollu yansıma; w = 6,2"
    out.append(ms.case("Bazı dışla: kayıtta kalır", write(with_known, after, {
        "op": "exclude", "solution": "bz-5", "reason": "Çok yollu yansıma; w = 6,2"}, {"id": "bz-5"}, "Bazı dışla"),
        d=with_known))
    after = with_known.clone()
    after.survey["baselines"][4]["adjust"] = False
    out.append(ms.case("Bazı dengelemeden çıkar", write(with_known, after, {
        "op": "adjust", "solution": "bz-5", "adjust": False}, {"id": "bz-5"}, "Dengeleme seçimi"), d=with_known))
    out.append(ms.case("Bilinen nokta dengelemeden çıkarılmaz, dışlanır", [refusal(
        {"op": "adjust", "solution": "bn-1", "adjust": False}, "invalid_survey_value",
        "“bn-1” bilinen nokta; bilinen nokta hep dengelemeye girer, çıkarmak için dışlayın.", "operation.solution")],
        d=with_known))
    after = with_known.clone()
    after.survey["known"][1]["excluded"] = "Eski C1 koordinatı"
    out.append(ms.case("Bilinen noktayı dışla", write(with_known, after, {
        "op": "exclude", "solution": "bn-2", "reason": "Eski C1 koordinatı"}, {"id": "bn-2"}, "Bilinen noktayı dışla"),
        d=with_known))
    # the network accepted
    after, res, da, records, fields = accept(with_known, {})
    out.append(ms.case("Ağı kabul et: da-1; N1'in noktası yeni revizyonla taşınır (eski sonuç kalır), N4 ve N5'in "
                       "noktaları çizilir; N1'e dayanan yüzey güncel değil; tek adımda geri alınır",
                       write(with_known, after, {"op": "network"}, {"id": da, "records": records},
                             "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after.entities), "fields": fields}), d=with_known))
    # ADR 2120: accepted at the measurement epoch; the known points moved there, the new marks back to 2005.0.
    moving = timed()
    after_t, _, da, records, fields = accept(moving, {})
    out.append(ms.case("Ağı ölçme epokunda kabul et: bilinen noktalar hız ızgarasıyla 2 Kasım 2026'ya taşınır, "
                       "dengeleme kaydının epoku ölçme epoku, noktalar karara (2005,0) döner",
                       write(moving, after_t, {"op": "network"}, {"id": da, "records": records},
                             "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_t.entities), "fields": fields}), d=moving))
    # ADR 2960: the error ellipses drawn with the points, magnified, on the map's directions, recorded as a use.
    with_e = {"layer": "gnss", "scale": 1000.0}
    after_e, _, da, records, fields = accept(with_known, {}, ellipses=with_e)
    out.append(ms.case("Ağı kabul et, hata elipsleriyle: N1, N4 ve N5'in elipsleri 1000 kat büyütülmüş, büyük eksen "
                       "meridyen yakınsaması çıkarılmış yönde; dayanan kaydı; tek adımda geri alınır",
                       write(with_known, after_e, {"op": "network", "ellipses": with_e},
                             {"id": da, "records": records, "ellipses": 3}, "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_e.entities), "fields": fields}), d=with_known))
    # ADR 3360: with the height bars, each beside its ellipse, recorded with them.
    with_h = {"layer": "gnss", "scale": 1000.0, "heights": True}
    after_h, _, da, records, fields = accept(with_known, {}, ellipses=with_h)
    out.append(ms.case("Ağı kabul et, hata elipsleri ve kot çubuklarıyla: her noktanın kot yarı aralığı 1000 kat "
                       "büyütülmüş düşey çubuk, uçlarında çentik; elipslerle aynı dayanan kaydında",
                       write(with_known, after_h, {"op": "network", "ellipses": with_h},
                             {"id": da, "records": records, "ellipses": 3, "bars": 3}, "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_h.entities), "fields": fields}), d=with_known))
    # ADR 3760: with the labels, each ellipse's semi-axes (and the half-interval) beside it, recorded with them.
    with_l = {"layer": "gnss", "scale": 1000.0, "heights": True, "labels": True}
    after_l, _, da, records, fields = accept(with_known, {}, ellipses=with_l)
    out.append(ms.case("Ağı kabul et, hata elipsleri, kot çubukları ve etiketleriyle: her elipsin yanında yarı eksenleri "
                       "ve kot yarı aralığı mm olarak, kâğıtta 2,5 mm yüksekliğinde (1/1000'de 2,5 m)",
                       write(with_known, after_l, {"op": "network", "ellipses": with_l},
                             {"id": da, "records": records, "ellipses": 3, "bars": 3, "labels": 3},
                             "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_l.entities), "fields": fields}), d=with_known))
    with_l5 = {"layer": "gnss", "scale": 2000.0, "labels": True, "labelHeight": 5.0}
    after_l5, _, da, records, fields = accept(with_known, {}, ellipses=with_l5)
    out.append(ms.case("Ağı kabul et, hata elipsleri ve 5 mm etiketleriyle, 2000 kat: etiket yalnız yarı eksenleri",
                       write(with_known, after_l5, {"op": "network", "ellipses": with_l5},
                             {"id": da, "records": records, "ellipses": 3, "labels": 3}, "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_l5.entities), "fields": fields}, then_undo=False),
                       d=with_known))
    out.append(ms.case("Ağı kabul et, hata elipsleri: etiket yüksekliği 0", [refusal(
        {"op": "network", "ellipses": {"layer": "gnss", "scale": 1000.0, "labels": True, "labelHeight": 0.0}},
        "invalid_survey_value", "Elips etiketinin kâğıttaki yüksekliği 0'dan büyük olmalı (mm); 0 verildi.",
        "operation.ellipses.labelHeight")], d=with_known))
    # The oblong network's ellipses have directions: the major axis turned by the convergence (ADR 2960).
    oblong = networked(baselines=[record(k + 1, vector(f, t, ERRS[(f, t)], sig=(0.0015, 0.004, 0.005), turn=30.0))
                                  for k, (f, t) in enumerate(PAIRS)])
    with_o = {"layer": "gnss", "scale": 2000.0}
    after_o, _, da, records, fields = accept(oblong, {}, ellipses=with_o)
    out.append(ms.case("Ağı kabul et, hata elipsleriyle, sapmalar 30° dönük: elipslerin büyük ekseni semtinden meridyen "
                       "yakınsaması çıkarılarak haritanın yönüne çevrilir, 2000 kat",
                       write(oblong, after_o, {"op": "network", "ellipses": with_o},
                             {"id": da, "records": records, "ellipses": 3}, "Ağ dengelemesini kabul et",
                             extra={"ids": sorted(after_o.entities), "fields": fields}, then_undo=False), d=oblong))
    out.append(ms.case("Ağı kabul et, hata elipsleri: büyütme 1'den küçük", [refusal(
        {"op": "network", "ellipses": {"layer": "gnss", "scale": 0.5}}, "invalid_survey_value",
        "Elipslerin büyütmesi 1 ile 100 000 arasında olmalı; 0.5 verildi.", "operation.ellipses.scale")],
        d=with_known))
    out.append(ms.case("Ağı kabul et, hata elipsleri: katman yok", [refusal(
        {"op": "network", "ellipses": {"layer": "elips", "scale": 1000.0}}, "layer_not_found",
        "“elips” kimlikli katman çizimde yok.", "operation.ellipses.layer")], d=with_known))
    after_c2, res_c2, da, records, fields = accept(with_known, {"class": "C2"})
    out.append(ms.case("Ağı C2 sınıfıyla kabul et: dengeleme kaydı sınıfı, paketi ve sonucunu tutar",
                       write(with_known, after_c2, {"op": "network", "class": "C2"}, {"id": da, "records": records},
                             "Ağ dengelemesini kabul et", extra={"ids": sorted(after_c2.entities), "fields": fields},
                             then_undo=False), d=with_known))
    # after it: a baseline excluded puts the network's points out of date
    later = after.clone()
    later.survey["baselines"][2]["excluded"] = "Yeniden işlenecek"
    steps = write(with_known, after, {"op": "network"}, {"id": "da-1", "records": ["kn-1", "kn-2", "kn-3"]},
                  "Ağ dengelemesini kabul et", then_undo=False)
    steps += [edit({"operation": {"op": "exclude", "solution": "bz-3", "reason": "Yeniden işlenecek"}},
                   done(edited(later, id="bz-3")), {"revision": "changed", "canUndo": True,
                                                    "settings": {"mapSurvey": any_numbers(later.survey)}},
                   within=WITHIN),
              {"op": "execute", "command": "map.survey.query", "input": {},
               "result": done(later.tree()), "expect": {"revision": "same"}, "within": WITHIN}]
    out.append(ms.case("Kabulden sonra bir bazı dışlandı: ağın üç noktası güncel değil, nedeni yazılı", steps,
                       d=with_known))
    steps = write(with_known, after, {"op": "network"}, {"id": "da-1", "records": ["kn-1", "kn-2", "kn-3"]},
                  "Ağ dengelemesini kabul et", then_undo=False)
    steps += [refusal({"op": "remove", "id": "bz-3"}, "survey_in_use",
                      "“bz-3” kaydı “da-1” ağ dengelemesinde; önce dengelemeyi çıkarın ya da kaydı dışlayın.",
                      "operation.id", after_write=True),
              refusal({"op": "remove", "id": "da-1"}, "survey_in_use",
                      "“kn-1” noktası “da-1” ağ dengelemesiyle üretildi; önce noktayı yeniden üretin ya da çıkarın.",
                      "operation.id", after_write=True),
              refusal({"op": "regenerate", "point": "kn-2"}, "invalid_survey_value",
                      "“kn-2” ağ dengelemesiyle üretildi; ağı yeniden dengeleyip kabul edin ya da çözümlerini verin.",
                      "operation.point", after_write=True)]
    out.append(ms.case("Ağın kullandığı baz ve dengeleme çıkmaz; ağ noktası çözümsüz yeniden üretilmez", steps,
                       d=with_known))
    steps = write(with_known, after, {"op": "network"}, {"id": "da-1", "records": ["kn-1", "kn-2", "kn-3"]},
                  "Ağ dengelemesini kabul et", then_undo=False)
    steps.append(refusal({"op": "network"}, "nothing_to_set",
                         "Ağ “da-1” dengelemesinden beri değişmedi; noktaları güncel.", "operation", after_write=True))
    out.append(ms.case("Aynı ağ ikinci kez kabul edilmez", steps, d=with_known))
    # refusals of the network
    blunder = networked(baselines=with_errors({("N1", "N4"): (0.0008, -0.0012, 0.0809)}))
    res, _ = analyse(blunder, {}, "operation.")
    st = res["free"]
    out.append(ms.case("Uyuşumsuz baz varken ağ kabul edilmez", [refusal(
        {"op": "network"}, "network_blunder",
        f"Serbest dengelemede {st['blunders']} uyuşumsuz ölçü var; en kötüsü “{st['worst']}”. Dışlayıp nedenini "
        "yazın ya da yeniden işleyin, sonra yeniden dengeleyin.", "operation")], d=blunder))
    res, _ = analyse(with_known, {"scale": 0.07}, "operation.")
    m0 = res["free"]["m0"]
    out.append(ms.case("Model testi geçmeyen ağ kabul edilmez; ileti varyans çarpanını söyler", [refusal(
        {"op": "network", "scale": 0.07}, "network_test_failed",
        f"Ağın model testi (χ²) geçmedi: m0 = {m0:.2f}. Varyans çarpanını yaklaşık {m0 * m0 * 0.07:.2f} (m0²·çarpan) "
        "yapıp yeniden dengeleyin.", "operation.scale")], d=with_known))
    out.append(ms.case("Yeni noktalar kilitli katmana çizilmez", [refusal(
        {"op": "network", "layer": "kilitli"}, "layer_locked",
        "“Kilitli” katmanı kilitli: kabul edilen nokta kilitli katmana çizilmez. Kilidi açın.", "operation.layer")],
        d=with_known))
    measured = networked(known_points=[known(1, "BN1", epoch=2026.84), known(2, "BN2", epoch=2026.84)])
    _, facts = analyse(measured, {}, "operation.")
    e = facts["new"][0]["placed"]
    out.append(ms.case("İşaret projeye taşınamazsa (ızgarasız iki epok) zincirin reddi aynen döner", [refusal(
        {"op": "network"}, e.code, e.message, e.path)], d=measured))
    only = networked(baselines=[record(1, vector("BN1", "BN2", (0.0005, -0.0004, 0.0008)))])
    out.append(ms.case("Bilinen noktalar arasında tek baz: dengelenecek nokta yok", [refusal(
        {"op": "network"}, "nothing_to_set", "Ağda bilinen nokta olmayan işaret yok; dengelenecek nokta yok.",
        "operation")], d=only))
    none = networked()
    for b in none.survey["baselines"]:
        b["adjust"] = False
    out.append(ms.case("Ağı kabul et: dengelemeye girecek baz yok", [refusal(
        {"op": "network"}, "no_baselines", "Dengelemeye girecek baz yok: bazları içe aktarın, dışlananları geri alın ya "
                                           "da dengeleme seçimini açın.", "operation")], d=none))
    mixed = networked(known_points=[KNOWN[0], known(2, "BN2", epoch=2026.84)])
    out.append(ms.case("Ağı kabul et: bilinen noktaların epokları farklı, bazların gözlem zamanı yok", [refusal(
        {"op": "network"}, "known_differ", "“bn-1” ile “bn-2” bilinen noktaları aynı çerçeve ve epokta değil; hepsini "
                                           "ölçme epokunda aynı çerçevede girin ya da bazların gözlem zamanlarını "
                                           "verin.", "operation")], d=mixed))
    out.append(ms.case("Ağı kabul et: pakette olmayan sınıf", [refusal(
        {"op": "network", "class": "C4"}, "unknown_rule_class",
        "“bohhbuy-2018-gnss” kural paketinde “C4” sınıfı yok; sınıflar: C1, C2, C3.", "operation.class")],
        d=with_known))
    apart = networked()
    apart.survey["baselines"].append(record(9, {"from": "N6", "to": "N7", "d": [1200.5, -300.25, 410.0],
                                                "covariance": VECTORS[0]["covariance"]}))
    out.append(ms.case("Ağı kabul et: ağa bağlanmayan işaretler", [refusal(
        {"op": "network"}, "network_disconnected",
        "“N7” istasyonu datuma bağlı değil: ona ulaşan baz yok ya da ağ parçalı.", "operation")], d=apart))
    twice = networked()
    twice.survey["baselines"].append(record(9, {"from": "N4", "to": "n4", "d": [0.5, 0.25, 0.125],
                                                "covariance": VECTORS[0]["covariance"]}))
    out.append(ms.case("Ağı kabul et: büyük küçük harfle ayrılan iki işaret", [refusal(
        {"op": "network"}, "duplicate_station", "“n4” istasyonu iki kez verilmiş.", "operation")], d=twice))
    after_w, res_w, da, records, fields = accept(with_known, {"class": "C2", "scale": 100.0})
    out.append(ms.case("Ağı C2 ile, varyans çarpanı 100 ile kabul et: sınıf tutmaz, uyarı söyler, dengeleme kaydı sonucu "
                       "tutar", write(with_known, after_w, {"op": "network", "class": "C2", "scale": 100.0},
                                      {"id": da, "records": records}, "Ağ dengelemesini kabul et",
                                      warnings=warnings_of(res_w, "operation."),
                                      extra={"ids": sorted(after_w.entities), "fields": fields}, then_undo=False),
                       d=with_known))
    tree = networked(baselines=[record(1, VECTORS[0]), record(2, VECTORS[2])], known_points=[KNOWN[0]])
    after_t, res_t, da, records, fields = accept(tree, {})
    out.append(ms.case("Fazla ölçüsü olmayan ağ da kabul edilir; uyarı sınanamadığını söyler",
                       write(tree, after_t, {"op": "network"}, {"id": da, "records": records},
                             "Ağ dengelemesini kabul et", warnings=warnings_of(res_t, "operation."),
                             extra={"ids": sorted(after_t.entities), "fields": fields}, then_undo=False), d=tree))
    nok = networked(known_points=[])
    out.append(ms.case("Ağı kabul et: bilinen nokta yok", [refusal(
        {"op": "network"}, "datum_missing",
        "Ağın bilinen noktası yok: bazların uçlarından en az birine bilinen nokta (TUSAGA-Aktif, C1 …) ekleyin.",
        "operation")], d=nok))
    return out


def assert_cases():
    """What the cases claim, checked once: margins away from every limit they name."""
    base = networked()
    out, _ = analyse(base, {})
    assert out["acceptable"] and out["free"]["blunders"] == 0 and out["free"]["passed"], out["free"]
    ws = [b[s]["w"] for b in out["baselines"] for s in ("free", "constrained") if "w" in b[s]]
    assert max(ws) < 1.9, max(ws)
    c1, _ = analyse(base, {"class": "C1"})
    assert c1["rules"]["passed"] and c1["acceptable"]
    c2, _ = analyse(base, {"class": "C2", "scale": 100.0})
    assert not c2["rules"]["passed"] and c2["acceptable"] and c2["rules"]["baselines"] > 0
    low, _ = analyse(base, {"scale": 0.07})
    assert low["free"]["passed"] is False and low["free"]["blunders"] == 0, low["free"]
    blunder = networked(baselines=with_errors({("N1", "N4"): (0.0008, -0.0012, 0.0809)}))
    b, _ = analyse(blunder, {})
    assert b["free"]["blunders"] > 0 and not b["acceptable"]


def with_grids(fn):
    """`fn()` with the shared cases' geoid and velocity grids where PROJ finds them, by their SHA-256 (as
    map_survey_cases.build does): the known points' move to the measurement epoch reads the velocity grid (ADR 2120)."""
    import shutil
    import tempfile
    tmp = Path(tempfile.mkdtemp(prefix="kentos-network-"))
    for name in ms.gc.FILES:
        shutil.copy(ms.gc.GRIDS / name, tmp / f"{ms.gc.sha(ms.gc.GRIDS / name)}.{ms.gc.ext(name)}")
    pyproj.datadir.append_data_dir(str(tmp))
    try:
        return fn()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def main():
    check = "--check" in sys.argv[1:]
    assert_cases()
    cases = with_grids(network_cases)
    doc = {"format": "kentos.command-cases", "version": 1, "command": "map.survey.network", "commandVersion": 1,
           "title": "GNSS ağını dengele: serbest ve bilinen noktalara dayalı aşamalar, BÖHHBÜY sınıfları, projeye taşıma",
           "note": "ADR 1740 (MAP-03, MAP-05). Bağımsız başvuru scripts/fixtures/map_survey_network_cases.py: numpy'nin "
                   "tam kovaryanslı en küçük kareleri, scipy'nin χ² ve normal nicemleri, PROJ'un TUREF ECEF dönüşümü ve "
                   "TM30'u (crs_vertical_cases.py); kural paketi catalog/v1/rules/gnss-ag-bohhbuy-2018.json; KentOS kodu "
                   "yok. Sayılar 1e-6 içinde.",
           "setup": networked().document(), "cases": cases}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    stale = []
    for name, data in WRITTEN.items():
        path = FILES / name
        if check:
            if not path.exists() or path.read_bytes() != data:
                stale.append(path)
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
    if check:
        if not OUT.exists() or OUT.read_text("utf-8") != text:
            stale.append(OUT)
    else:
        OUT.write_text(text, "utf-8")
    if stale:
        for p in stale:
            print(f"{p.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
        sys.exit(1)
    print(f"map.survey.network: {len(doc['cases'])} durum" + (" (denetlendi)" if check else "") +
          f"; map.survey.edit'e {len(with_grids(edit_cases))} ağ durumu (map_survey_cases.py yazar)")


if __name__ == "__main__":
    main()
