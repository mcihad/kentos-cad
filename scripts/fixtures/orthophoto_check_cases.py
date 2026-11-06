#!/usr/bin/env python3
"""The shared cases of `cad.orthophoto.check` (Ortofoto doğruluk denetimi; plan/map-cad.md MAP-12, docs/adr/1480),
without KentOS code.

    python3 scripts/fixtures/orthophoto_check_cases.py           # writes fixtures/commands/v1/cad.orthophoto.check.json
    python3 scripts/fixtures/orthophoto_check_cases.py --check   # writes nothing; compares

The rules, by hand (BÖHHBÜY, RG 26.06.2018/30460 Mükerrer, Articles 54, 67(5), 91(2); read from harita.gov.tr's text):
- the points of the ground's layer and of the image's layer matched by name (the first of a name on each side); a
  name the raster's georeference used as a control point (not a check point) is a fit point, the others check points;
- each point's error image − ground; KOH_X,Y = sqrt(Σ(dx² + dy²) / 2n), RMSE in X and Y, the radial RMSE, the means,
  the largest horizontal error; KOH_Z over the points with heights on both sides;
- KOH_X,Y under half a pixel is flagged: finer than reading the image, the result may flatter it;
- the limits: KOH_X,Y (cm) ≤ −1.665e-6·S² + 0.01745·S − 1.166, KOH_Z ≤ 1.33·KOH_X,Y; the ground sampling distance
  (the square root of a pixel's area) ≤ Article 54's for the scale, ±10 %.
"""

import copy
import json
import math
import sys
from pathlib import Path

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures" / "commands" / "v1" / "cad.orthophoto.check.json"
RULES = json.loads((ROOT / "catalog/v1/rules/ortofoto-bohhbuy-2018.json").read_text("utf-8"))
X0, Y0 = 487000.0, 4420000.0


def uid(n):
    return f"0192f5c0-1480-7000-8000-{n:012x}"


def layer(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}


GEOREF = {"method": "thinPlate",
          "points": [{"name": "Y1", "pixel": [0.0, 0.0], "target": [X0 + 10, Y0 + 290]},
                     {"name": "Y2", "pixel": [2900.0, 0.0], "target": [X0 + 290, Y0 + 290]},
                     {"name": "Y3", "pixel": [2900.0, 2900.0], "target": [X0 + 290, Y0 + 10]}],
          "source": {"width": 3000, "height": 3000, "asset": "raster-1be365b685ba77d1",
                     "digest": "1be365b685ba77d1d05f8c076a61e8f65572a688a0e6e8a78b563499da8b289b"},
          "output": {"affine": [X0, 0.1, 0.0, Y0 + 300, 0.0, -0.1], "width": 3000, "height": 3000, "pixel": 0.1,
                     "resampling": "nearest"}}


def raster(pixel=0.10, srid=5254, georef=True):
    r = {"kind": "raster", "layerId": "altlik", "attrs": {}, "affine": [X0, pixel, 0.0, Y0 + 300, 0.0, -pixel],
         "width": 3000, "height": 3000, "bands": 3, "sample": "u8", "srid": srid,
         "style": {"render": "rgb", "bands": [1, 2, 3]}, "file": "ortofoto-2026.tif"}
    if georef:
        r["georef"] = copy.deepcopy(GEOREF)
        # The fit's output grid is the raster's (a .kcad rule).
        r["georef"]["output"].update(affine=list(r["affine"]), pixel=pixel)
    return r


# Name, ground (x, y, z), image error (dx, dy, dz) in metres.
POINTS = [
    ("D1", (20.0, 30.0, 851.20), (0.040, -0.030, 0.06)),
    ("D2", (80.0, 40.0, 851.90), (-0.050, 0.020, -0.08)),
    ("D3", (150.0, 60.0, 852.40), (0.030, 0.060, 0.10)),
    ("D4", (220.0, 90.0, None), (-0.020, -0.070, None)),
    ("D5", (260.0, 150.0, 853.00), (0.080, 0.010, 0.05)),
    ("D6", (200.0, 220.0, 853.60), (-0.060, -0.040, -0.12)),
    ("D7", (120.0, 250.0, 852.80), (0.010, 0.050, 0.04)),
    ("D8", (40.0, 200.0, 852.10), (0.050, -0.050, 0.09)),
    ("Y1", (10.0, 290.0, 852.00), (0.004, -0.003, 0.01)),
    ("Y2", (290.0, 290.0, 854.00), (-0.002, 0.005, None)),
]


# The image's errors as read on a 10 cm image: about 0.7 of a pixel.
POINTS = [(n, g, (e[0] * 1.6, e[1] * 1.6, e[2]) if not n.startswith("Y") else e) for n, g, e in POINTS]


def point(n, layer_, x, y, z, name, field="Ad"):
    e = {"kind": "point", "id": n, "layerId": layer_, "attrs": {field: name}, "p": {"x": X0 + x, "y": Y0 + y}}
    if z is not None:
        e["z"] = z
    return e


def setup(points=POINTS, ras=None, field="Ad", extra=()):
    ents = [dict(ras or raster(), id=1)]
    n = 2
    for name, (x, y, z), _ in points:
        ents.append(point(n, "gnss", x, y, z, name, field))
        n += 1
    for name, (x, y, z), (dx, dy, dz) in points:
        ents.append(point(n, "okuma", x + dx, y + dy, None if z is None or dz is None else z + dz, name, field))
        n += 1
    for e in extra:
        ents.append(dict(e, id=n))
        n += 1
    return {"format": "kentos.document", "version": 2, "name": "Ortofoto doğruluk denetimi",
            "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0, "workspace": "cad", "cadWork": "map"},
            "origin": {"x": X0, "y": Y0},
            "layers": [layer("0", "0"), layer("altlik", "Altlık"), layer("gnss", "GNSS denetim noktaları"),
                       layer("okuma", "Ortofotoda okunan")],
            "activeLayer": "okuma", "entities": ents, "uids": [uid(i + 1) for i in range(len(ents))],
            "styles": {"items": [], "categories": []}}


def named(s, layer_, field):
    out = []
    for e in s["entities"]:
        if e["layerId"] != layer_ or e["kind"] != "point":
            continue
        name = e["attrs"].get(field, "").strip()
        if name and name not in [o[0] for o in out]:
            out.append((name, e["p"]["x"], e["p"]["y"], e.get("z")))
    return out


def stats(ps):
    n = len(ps)
    if n == 0:
        return {"n": 0, "rmseX": 0.0, "rmseY": 0.0, "kohXy": 0.0, "rmseR": 0.0, "meanX": 0.0, "meanY": 0.0,
                "maxD": 0.0, "nZ": 0}
    sx, sy = sum(p["dx"] ** 2 for p in ps), sum(p["dy"] ** 2 for p in ps)
    zs = [p["dz"] for p in ps if "dz" in p]
    out = {"n": n, "rmseX": math.sqrt(sx / n), "rmseY": math.sqrt(sy / n), "kohXy": math.sqrt((sx + sy) / (2 * n)),
           "rmseR": math.sqrt((sx + sy) / n), "meanX": sum(p["dx"] for p in ps) / n,
           "meanY": sum(p["dy"] for p in ps) / n, "maxD": max(p["d"] for p in ps), "nZ": len(zs)}
    if zs:
        out["kohZ"] = math.sqrt(sum(z * z for z in zs) / len(zs))
    return out


def check(s, known="gnss", observed="okuma", field="Ad", scale=None, rules=None):
    ras = s["entities"][0]
    rules = rules or RULES
    scale = scale or round(s["settings"]["plotScale"])
    k, o = named(s, known, field), named(s, observed, field)
    fitted = [p["name"].strip() for p in (ras.get("georef") or {}).get("points", []) if not p.get("check") and
              p.get("name")]
    points, unmatched = [], []
    for name, x, y, z in k:
        m = [q for q in o if q[0] == name]
        if not m:
            unmatched.append(name)
            continue
        _, ox, oy, oz = m[0]
        p = {"name": name, "role": "fit" if name in fitted else "check", "dx": ox - x, "dy": oy - y}
        if oz is not None and z is not None:
            p["dz"] = oz - z
        p["d"] = math.hypot(p["dx"], p["dy"])
        points.append(p)
    for name, *_ in o:
        if name not in [q[0] for q in k]:
            unmatched.append(name)
    chk = stats([p for p in points if p["role"] == "check"])
    fits = [p for p in points if p["role"] == "fit"]
    S = float(scale)
    a, b, c = rules["xy"]
    limit_xy = (a * S * S + b * S + c) / 100.0
    limit_z = rules["zFactor"] * limit_xy
    af = ras["affine"]
    gsd = math.sqrt(abs(af[1] * af[5] - af[2] * af[4]))
    gl = [g["gsd"] for g in rules["gsd"] if g["scale"] == scale]
    notes = [f"KOH_X,Y = √(Σ(dx² + dy²) / 2n): X ve Y'nin ortak karesel ortalama hatası; sınırı {rules['id']} (S = {scale}).",
             "Uyum noktaları (görüntünün oturtulduğu kontrol noktaları) doğruluk göstermez; yalnız bağımsız denetim "
             "noktaları sayılır.",
             "Denetim noktalarının sayısı ve dağılımı (BÖHHBÜY m. 91(1): detay noktalarının en az %5'i) bu denetimde "
             "bilinmez; elle gözden geçirin."]
    if not gl:
        notes.append(f"Kural paketi 1/{scale} ölçeği için yer örnekleme aralığı vermiyor.")
    if unmatched:
        notes.append(f"Eşi bulunmayan noktalar: {', '.join(unmatched)}.")
    out = {"rules": rules["id"], "source": rules["source"], "scale": scale, "gsd": gsd}
    if gl:
        out["gsdLimit"] = gl[0]
        out["gsdOk"] = gsd <= gl[0] * (1 + rules["gsdTolerance"]) + 1e-12
    out.update({"points": points, "unmatched": unmatched, "check": chk})
    if fits:
        out["fit"] = stats(fits)
    out.update({"limitXy": limit_xy, "limitZ": limit_z})
    if chk["n"] > 0:
        out["passedXy"] = chk["kohXy"] <= limit_xy
    if "kohZ" in chk:
        out["passedZ"] = chk["kohZ"] <= limit_z
    out["belowPixel"] = chk["n"] > 0 and chk["kohXy"] < gsd / 2
    out["notes"] = notes
    return out


SAME = {"revision": "same", "canUndo": False}
BASE = setup()


def ok(name, s=BASE, op="plan", **kw):
    inp = {"raster": uid(1), "known": kw.pop("known", "gnss"), "observed": kw.pop("observed", "okuma")}
    inp.update(kw)
    args = {k: v for k, v in kw.items() if k in ("field", "scale", "rules")}
    out = check(s, inp["known"], inp["observed"], **args)
    case = {"name": name, "steps": [{"op": op, "input": inp, "result": {
        "status": "completed", "output": None if op == "validate" else out, "warnings": []}, "expect": SAME,
        "within": 1e-9}]}
    if s is not BASE:
        case["setup"] = s
    return case


def refused(name, inp, code, message, path_, s=None):
    case = {"name": name, "steps": [{"op": "execute", "input": dict({"raster": uid(1), "known": "gnss",
                                                                    "observed": "okuma"}, **inp),
                                     "result": {"status": "failed", "error": {"code": code, "message": message,
                                                                              "path": path_}}, "expect": SAME}]}
    if s is not None:
        case["setup"] = s
    return case


def cases():
    first = check(BASE)
    assert first["passedXy"] and first["check"]["n"] == 8 and first["fit"]["n"] == 2, first
    blunder = [(n, g, (0.60, -0.40, e[2]) if n == "D3" else e) for n, g, e in POINTS]
    tiny = [(n, g, (e[0] / 10, e[1] / 10, e[2])) for n, g, e in POINTS]
    asprs_like = dict(copy.deepcopy(RULES), id="deneme-sabit-5cm", source="Deneme paketi (sentetik): sabit 5 cm",
                      xy=[0.0, 0.0, 5.0], zFactor=1.0, gsd=[])
    return [
        ok("1/1000: denetim noktaları sınırın içinde; uyum noktaları ayrı"),
        ok("uygula: salt okunur", op="execute"),
        ok("doğrulama", op="validate"),
        ok("1/500: sınır 7,14 cm, aynı noktalar dışında", scale=500),
        ok("1/2000", scale=2000),
        ok("1/5000", scale=5000),
        ok("1/1500: kural paketi bu ölçeğe YÖA vermez", scale=1500),
        ok("başka kural paketi: sabit 5 cm", rules=asprs_like),
        ok("kaba hatalı nokta KOH'u sınırın dışına taşır", setup(blunder)),
        ok("yarım piksel altı KOH: okuma inceliğinin altında, iyimser olabilir", setup(tiny)),
        ok("YÖA 15 cm: 1/1000 için %10'dan fazla kaba", setup(ras=raster(pixel=0.15))),
        ok("YÖA 11 cm: %10 içinde", setup(ras=raster(pixel=0.11))),
        ok("oturtma bilgisi yoksa bütün noktalar denetim noktası", setup(ras=raster(georef=False))),
        ok("raster sistemi 0: projeninki sayılır", setup(ras=raster(srid=0))),
        ok("başka ad alanı", setup(field="No"), field="No"),
        ok("eşi olmayan noktalar söylenir", setup(extra=[point(0, "gnss", 5, 5, None, "Yalniz-arazi"),
                                                        point(0, "okuma", 6, 6, None, "Yalniz-goruntu")])),
        ok("eşleşen nokta yok: sonuç yok", known="okuma", observed="altlik"),
        ok("yalnız uyum noktaları: denetim sayısı 0", setup([p for p in POINTS if p[0].startswith("Y")])),
        ok("kotsuz noktalar: KOH_Z yok", setup([(n, (g[0], g[1], None), (e[0], e[1], None)) for n, g, e in POINTS])),
        refused("raster projenin sisteminde değil", {}, "crs_mismatch",
                "Ortofotonun koordinat sistemi EPSG:5256, projeninki EPSG:5254; farklı sistemdeki görüntü denetlenmez. "
                "Önce projeye çevirin (Raster › Yeniden izdüşür).", "raster", setup(ras=raster(srid=5256))),
        refused("raster değil", {"raster": uid(2)}, "not_a_raster", f"“{uid(2)}” bir raster değil; ortofotoyu seçin.",
                "raster"),
        refused("kimlik UUID değil", {"raster": "x"}, "invalid_uid", "“x” geçerli bir nesne kimliği değil.", "raster"),
        refused("çizimde olmayan raster", {"raster": uid(999)}, "entity_not_found",
                f"“{uid(999)}” kimlikli nesne çizimde yok.", "raster"),
        refused("arazi katmanı yok", {"known": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.", "known"),
        refused("görüntü katmanı yok", {"observed": "yok"}, "layer_not_found", "“yok” katmanı çizimde yok.",
                "observed"),
        refused("ölçek 0", {"scale": 0}, "invalid_input", "Ölçek paydası 0 olamaz.", "scale"),
        refused("kural paketinin kaynağı boş", {"rules": dict(copy.deepcopy(RULES), source=" ")}, "invalid_input",
                "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim yapılmaz.", "rules"),
    ]


def main():
    doc = {"format": "kentos.command-cases", "version": 1, "command": "cad.orthophoto.check", "commandVersion": 1,
           "title": "Ortofoto doğruluk denetimi",
           "note": "ADR 1480. Bağımsız başvuru scripts/fixtures/orthophoto_check_cases.py: BÖHHBÜY m. 54, 91(2) "
                   "kuralları elle; KentOS kodu yok. Noktalar sentetiktir.",
           "setup": BASE, "cases": cases()}
    text = json.dumps(doc, ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv[1:]:
        ok_ = OUT.exists() and OUT.read_text("utf-8") == text
        print("Ortofoto doğruluk ortak durumları güncel." if ok_ else f"{OUT.relative_to(ROOT)} güncel değil.")
        return 0 if ok_ else 1
    OUT.write_text(text, "utf-8")
    print(f"yazıldı: {OUT.relative_to(ROOT)} ({len(doc['cases'])} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
