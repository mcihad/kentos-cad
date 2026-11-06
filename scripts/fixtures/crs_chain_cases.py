#!/usr/bin/env python3
"""Independent reference of the coordinate chain of `cad.crs.transform` (docs/adr/1230, plan/map-cad.md MAP-02).

    python3 scripts/fixtures/crs_chain_cases.py           # writes fixtures/commands/v1/cad.crs.transform.chain.json
    python3 scripts/fixtures/crs_chain_cases.py --check   # compares

Needs pyproj (PROJ) and, for the velocity and geoid cases, the grids of fixtures/geoid/v1 (geoid_cases.py). Without
pyproj here the script runs itself again under QGIS's Python on macOS, as crs_vertical_cases.py does; set PROJ_DATA to
the proj.db the other CRS scripts were written with (QGIS's bundled one) when another PROJ is also installed.

No KentOS code. The plan's acceptance, each against PROJ:

- millimetres at E = 500 000, N = 4 400 000 m (TUREF / TM30): three points a millimetre apart, to another zone, to
  WGS 84 / UTM, to ED50 and to latitude and longitude; PROJ's answers keep the millimetre (checked here);
- two distant regions: Edirne in TM27 and Hakkari in TM45, each to its UTM zone and to the other's zone, one call
  carrying both to TM30;
- local to real coordinates: a local system on TUREF / TM30 by an affine plane (the plane worked out here, the rest
  PROJ's), to its base, to latitude and longitude;
- epochs: a velocity grid between 2023.5 and 2005.0 (PROJ's `deformation` with `+dt`), an epoch only on one side, the
  project's own epoch;
- geoid: ellipsoidal heights to the project's Antalya heights through the synthetic TG geoid (PROJ's `vgridshift`),
  a point beyond the grid's nodes;
- what is not resolvable: a project whose stored EPSG code the registry does not hold, a side's unknown code.

The chain of every completed case is crs_chain_rules.py's (the ADR's rules, PROJ's database). Grid coordinates within
1e-6 m, degrees within 1e-10, heights within 1e-6 m: the engine's budget (docs/adr/1230 §4).
"""

import json
import os
import shutil
import sys
import tempfile
from pathlib import Path

try:
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
import crs_chain_rules as chain  # noqa: E402
import crs_vertical_cases as cv  # noqa: E402
import geoid_cases as gc  # noqa: E402
from crs_vertical_cases import n, pt, case, failed, SAME  # noqa: E402
import pyproj  # noqa: E402
from pyproj import Transformer  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.crs.transform.chain.json"

SETUP = json.loads(json.dumps(cv.SETUP))
SETUP["name"] = "Koordinat zinciri"
del SETUP["settings"]["heights"]
del SETUP["settings"]["datumTransforms"]
SETUP["origin"] = {"x": 500000, "y": 4400000}

ELL = {"kind": "ellipsoidal"}
EDIRNE, HAKKARI = (41.6772, 26.5557), (37.5744, 43.7408)


def mm_points():
    return [pt(500000.0, 4400000.0), pt(500000.001, 4400000.0), pt(500000.0, 4400000.001)]


def keeps_the_millimetre(c, unit_m):
    """PROJ's answers keep the millimetre: the first point's neighbours lie 1 mm × the scale between the zones away."""
    out = [p for p in c["result"]["output"]["points"]]
    for q in out[1:]:
        d = ((q["x"] - out[0]["x"]) ** 2 + (q["y"] - out[0]["y"]) ** 2) ** 0.5 * unit_m
        assert 0.0009 < d < 0.0011, d
    return c


def own_from(c):
    """The case with its source the project's own (TUREF / TM30, no heights, no epoch: the setup's)."""
    frm = c["input"].get("from", {})
    assert frm.get("srid") == 5254 and "heights" not in frm and "epoch" not in frm, frm
    del frm["srid"]
    if not frm:
        del c["input"]["from"]
    return c


# ── Local to real coordinates ───────────────────────────────────────────────────────────────────────────────────

# The site's plane: base x = a·x + b·y + c, base y = d·x + e·y + f (a turn of about 12.5° and a scale of 1.000012).
PLANE = {"kind": "affine", "a": 0.976316, "b": -0.216347, "c": 500000.0, "d": 0.216347, "e": 0.976316,
         "f": 4400000.0}
LOCAL = {"name": "Şantiye", "system": {"kind": "local", "base": {"srid": 5254}, "plane": PLANE}}
LOCAL_SETUP = json.loads(json.dumps(SETUP))
LOCAL_SETUP["name"] = "Şantiye (yerel)"
LOCAL_SETUP["settings"]["srid"] = 0
LOCAL_SETUP["settings"]["customCrs"] = LOCAL


def affine_step(p):
    return (f"+proj=affine +xoff={n(p['c'])} +yoff={n(p['f'])} +s11={n(p['a'])} +s12={n(p['b'])} "
            f"+s21={n(p['d'])} +s22={n(p['e'])}")


def local_case(points, dst):
    """From the local project to registry system `dst`: the plane here, the rest PROJ's through crs_vertical_cases."""
    base = cv.transformed([pt(PLANE["a"] * q["x"] + PLANE["b"] * q["y"] + PLANE["c"],
                              PLANE["d"] * q["x"] + PLANE["e"] * q["y"] + PLANE["f"]) for q in points],
                          frm={"srid": 5254}, to={"srid": dst})
    rec = base["result"]["output"]["operation"]
    rec["steps"].insert(0, {"kind": "conversion", "name": "Yerel düzlem", "reverse": False, "accuracy": 0.0})
    rec["pipeline"] = rec["pipeline"].replace("+proj=pipeline", f"+proj=pipeline +step {affine_step(PLANE)}", 1)
    rec["from"] = "Şantiye (özel sistem)"
    pipeline = Transformer.from_pipeline(rec["pipeline"])
    tol = base["within"]
    for q, want in zip(points, base["result"]["output"]["points"]):
        x, y = pipeline.transform(q["x"], q["y"])
        assert abs(x - want["x"]) < tol and abs(y - want["y"]) < tol, (x - want["x"], y - want["y"])
    base["input"] = {"points": points, "to": {"srid": dst}}
    out = base["result"]["output"]
    out["chain"]["source"] = chain.side(None, None, None, "none", definition=("Şantiye", "local", "TUREF"))
    out["chain"]["bases"].insert(0, "exact")
    return base


# ── Epochs ──────────────────────────────────────────────────────────────────────────────────────────────────────

HIZ = gc.model("hiz.tif", "Hız alanı (sentetik)")
TG = gc.TG


def project_epoch(c, epoch):
    """The case with its source's epoch the project's (a setup of its own with `coordinateEpoch`)."""
    frm = c["input"]["from"]
    assert frm.pop("epoch") == epoch and frm.pop("srid") == 5254
    if not frm:
        del c["input"]["from"]
    c["result"]["output"]["chain"]["source"]["epochSource"] = "project"
    setup = json.loads(json.dumps(SETUP))
    setup["settings"]["coordinateEpoch"] = epoch
    return setup, c


def cases():
    out = []
    # Millimetres at large coordinates.
    out.append(case("E=500 000 / N=4 400 000'de milimetre: TM30'dan TM33'e, yalnız izdüşüm, zincir tam",
                    keeps_the_millimetre(own_from(cv.transformed(mm_points(), frm={"srid": 5254},
                                                                 to={"srid": 5255})), 1.0)))
    out.append(case("Aynısının planı: aynı cevap, çizim değişmez",
                    own_from(cv.transformed(mm_points(), frm={"srid": 5254}, to={"srid": 5255}, op="plan"))))
    out.append(case("Milimetre WGS 84 / UTM 36N'e: boş dönüşüm, WGS 84 datum topluluğu (2 m)",
                    keeps_the_millimetre(own_from(cv.transformed(mm_points(), frm={"srid": 5254},
                                                                 to={"srid": 32636})), 1.0)))
    out.append(case("Milimetre ED50 / TM30'a: iki Helmert, resmî olmayan işlem",
                    keeps_the_millimetre(own_from(cv.transformed(mm_points(), frm={"srid": 5254},
                                                                 to={"srid": 2320})), 1.0)))
    out.append(case("Milimetre TUREF enlem ve boylamına: derece, bütçe 1e-9°",
                    own_from(cv.transformed(mm_points(), frm={"srid": 5254}, to={"srid": 5252}))))
    lon, lat = 30.0, 39.74
    out.append(case("TUREF coğrafiden TM30'a ve milimetrede geri: derece girişi, metre çıkışı",
                    cv.transformed([pt(lon, lat), pt(lon + 1e-8, lat)], frm={"srid": 5252}, to={"srid": 5254})))
    # Two distant regions.
    ed, hk = cv.place(5253, *EDIRNE), cv.place(5259, *HAKKARI)
    out.append(case("Edirne TM27'den WGS 84 / UTM 35N'e",
                    cv.transformed([pt(*ed)], frm={"srid": 5253}, to={"srid": 32635})))
    out.append(case("Hakkari TM45'ten WGS 84 / UTM 38N'e",
                    cv.transformed([pt(*hk)], frm={"srid": 5259}, to={"srid": 32638})))
    out.append(case("Edirne ve Hakkari TUREF coğrafiden tek çağrıda TM30'a: merkez boylamdan 13,7° uzak",
                    cv.transformed([pt(EDIRNE[1], EDIRNE[0]), pt(HAKKARI[1], HAKKARI[0])], frm={"srid": 5252},
                                   to={"srid": 5254})))
    out.append(case("Hakkari TM45'ten ED50 / TM45'e: doğu uçta iki Helmert",
                    cv.transformed([pt(*hk)], frm={"srid": 5259}, to={"srid": 2325})))
    # Local to real coordinates.
    local = [pt(12.345, 67.891), pt(250.0, -120.5), pt(0.001, 0.0)]
    c = local_case(local, 5254)
    out.append({**case("Yerelden gerçek koordinata: şantiye düzlemi TM30'a, milimetre korunur", c),
                "setup": LOCAL_SETUP})
    c = local_case(local, 4326)
    out.append({**case("Yerelden WGS 84 coğrafiye: düzlem, izdüşüm, boş dönüşüm", c), "setup": LOCAL_SETUP})
    # Epochs.
    p0 = (500000.0, 4400000.0)
    out.append(case("Hız ızgarasıyla 2023,5'ten 2005,0'a, elipsoit yüksekliğiyle: hızın doğruluğu bilinmiyor",
                    gc.transformed([pt(*p0, 1000.0)], frm={"srid": 5254, "heights": ELL, "epoch": 2023.5},
                                   to={"srid": 5254, "heights": ELL, "epoch": 2005.0}, vel=HIZ)))
    out.append(case("Yalnız kaynağın epoku, hedef WGS 84 / UTM 36N: epok belirsiz, sınırlı",
                    cv.transformed([pt(*p0)], frm={"srid": 5254, "epoch": 2023.5}, to={"srid": 32636})))
    out.append(case("Yalnız hedefin epoku: kaynağınki söylenmedi, sınırlı",
                    own_from(cv.transformed([pt(*p0)], frm={"srid": 5254}, to={"srid": 5255, "epoch": 2005.0}))))
    setup, c = project_epoch(cv.transformed([pt(*p0)], frm={"srid": 5254, "epoch": 2005.0},
                                            to={"srid": 5255, "epoch": 2005.0}), 2005.0)
    out.append({**case("Projenin epoku 2005,0, hedefinki de: epok yerleşik, zincir tam", c), "setup": setup})
    # Geoid.
    out.append(case("Elipsoit yüksekliğinden Antalya kotuna TG geoidiyle, büyük koordinatta milimetre",
                    gc.transformed([pt(500000.0, 4400000.0, 1000.0), pt(500000.001, 4400000.0, 1000.0)],
                                   frm={"srid": 5254, "heights": ELL},
                                   to={"srid": 5254, "heights": gc.ortho(5775, TG)})))
    west = cv.place(5254, 39.0, 24.5)
    out.append(case("Geoidin düğümlerinin batısında bir nokta: değeri yok, kapsam dışı, sınırlı",
                    gc.transformed([pt(*p0, 1000.0), pt(*west, 1000.0)], frm={"srid": 5254, "heights": ELL},
                                   to={"srid": 5254, "heights": gc.ortho(5775, TG)}, unreached_at=(1,))))
    out.append(case("Kotlar verildi, iki yanda da ne oldukları söylenmedi: kotlar kalır, sınırlı",
                    own_from(cv.transformed([pt(*p0, 120.75)], frm={"srid": 5254}, to={"srid": 5255}))))
    # What is not resolvable.
    nz = json.loads(json.dumps(SETUP))
    nz["settings"]["srid"] = 2193
    out.append({**case("Projenin kayıtlı kodu EPSG:2193 KentOS'un çözdüğü sistemlerden değil: ret",
                       {"op": "execute", "input": {"points": [pt(*p0)], "to": {"srid": 5254}},
                        "result": failed("unsupported_srid",
                                         "from: projenin kayıtlı sistemi EPSG:2193 KentOS'un çözdüğü sistemler "
                                         "arasında yok (TUREF, ED50 ve WGS 84'ün coğrafi, TM ve UTM sistemleri, WGS 84 / "
                                         "Pseudo-Mercator, projenin yerel tanımı). Koordinatlar bu kodla dönüştürülmez: "
                                         "projeye kayıttaki bir sistemi atayın ya da tarafın sistemini EPSG koduyla "
                                         "verin (srid).", "from"),
                        "expect": SAME}), "setup": nz})
    out.append(case("Tarafın kodu kayıtta yok (EPSG:2193): ret",
                    {"op": "execute", "input": {"points": [pt(*p0)], "to": {"srid": 2193}},
                     "result": failed("unknown_srid",
                                      "EPSG:2193 koordinat sistemleri kaydında yok ya da bir yere bağlı değil. Kayıttaki "
                                      "bir sistemin kodunu verin ya da kodu bırakın (projenin sistemi).", "to.srid"),
                     "expect": SAME}))
    return out


def build():
    # PROJ reads the grids by the names the record gives them: their SHA-256 (as geoid_cases.py does).
    tmp = Path(tempfile.mkdtemp(prefix="kentos-chain-"))
    for name in gc.FILES:
        shutil.copy(gc.GRIDS / name, tmp / f"{gc.sha(gc.GRIDS / name)}.{gc.ext(name)}")
    pyproj.datadir.append_data_dir(str(tmp))
    try:
        return built()
    finally:
        shutil.rmtree(tmp, ignore_errors=True)


def built():
    return {
        "format": "kentos.command-cases", "version": 1, "command": "cad.crs.transform", "commandVersion": 1,
        "title": "Koordinat dönüştür: koordinat zinciri, büyük koordinatta milimetre, uzak bölgeler, yerel → gerçek",
        "note": "ADR 1230 (MAP-02). Noktalar PROJ'dan (pyproj); yerel düzlem betikte, geoit ve hız ızgaraları "
                "fixtures/geoid/v1'den (geoid_cases.py). Zincir crs_chain_rules.py'nin: tarafların ailesi, birimi ve "
                "EPSG eksen sırası PROJ'un veritabanından, kararın nedenleri ADR'nin sırasıyla, sayısal bütçe 1e-6 m, "
                "1e-9°. Proje: TUREF / TM30, kotsuz, epoksuz. Bağımsız başvuru: scripts/fixtures/crs_chain_cases.py.",
        "setup": SETUP, "cases": cases(),
    }


def main():
    built = build()
    text = json.dumps(built, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        if OUT.read_text() != text:
            print(f"{OUT.relative_to(ROOT)} eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"koordinat zincirinin durumları tutarlı: {len(built['cases'])} durum (PROJ "
              f"{__import__('pyproj').proj_version_str})")
        return 0
    OUT.write_text(text)
    print(f"{OUT.relative_to(ROOT)}: {len(built['cases'])} durum")
    return 0


if __name__ == "__main__":
    sys.exit(main())
