#!/usr/bin/env python3
"""The shared cases of cad.layers.crs and cad.layers.reproject (docs/adr/0583 §2, plan/gis.md CBS-07).

    python3 scripts/fixtures/layer_crs_cases.py           # writes fixtures/commands/v1/cad.layers.{crs,reproject}.json
    python3 scripts/fixtures/layer_crs_cases.py --check   # compares

No KentOS code. A layer's record is written here from the registry (fixtures/crs/v1/registry.json): its name, EPSG's
axis order, unit and area; its WKT2 from EPSG's definition (datum, ellipsoid, method and parameters with their EPSG
codes) and checked against PROJ (pyproj): the text read by PROJ moves points across Türkiye exactly as the EPSG code
does, and has the code's axis order. The moved coordinates are PROJ's, along the EPSG path ADR 0167 names for each
pair of datums (crs_transform_cases.py's `transformer`); the cases compare them within 1 mm.
"""

import copy
import json
import sys
from pathlib import Path

from pyproj import CRS, Transformer

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import crs_transform_cases as ct  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1"
REGISTRY = {s["srid"]: s for s in json.loads((ROOT / "fixtures/crs/v1/registry.json").read_text("utf-8"))["systems"]}
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}
PROJECT = 5256
WITHIN = 0.001

DEGREE = 'ANGLEUNIT["degree",0.0174532925199433]'
METRE = 'LENGTHUNIT["metre",1]'
DATUMS = {
    "TUREF": ("TUREF", "Turkish National Reference Frame", "GRS 1980", 6378137, 298.257222101),
    "ED50": ("ED50", "European Datum 1950", "International 1924", 6378388, 297),
    "WGS84": ("WGS 84", "World Geodetic System 1984", "WGS 84", 6378137, 298.257223563),
}


def num(v):
    return str(int(v)) if float(v) == int(v) else repr(float(v))


def wkt2(s):
    """EPSG's definition of a registry system as WKT2 (ISO 19162:2019)."""
    base, datum, ellipsoid, a, rf = DATUMS[s["datum"]]
    d = f'DATUM["{datum}",ELLIPSOID["{ellipsoid}",{num(a)},{num(rf)},{METRE}]],PRIMEM["Greenwich",0,{DEGREE}]'
    code = f'ID["EPSG",{s["srid"]}]'
    if s["kind"] == "geographic":
        return (f'GEOGCRS["{s["name"]}",{d},CS[ellipsoidal,2],AXIS["geodetic latitude (Lat)",north,ORDER[1],{DEGREE}],'
                f'AXIS["geodetic longitude (Lon)",east,ORDER[2],{DEGREE}],{code}]')
    if s["projection"] == "Pseudo-Mercator":
        method = 'METHOD["Popular Visualisation Pseudo Mercator",ID["EPSG",1024]]'
        params = (f'PARAMETER["Latitude of natural origin",0,{DEGREE},ID["EPSG",8801]],'
                  f'PARAMETER["Longitude of natural origin",0,{DEGREE},ID["EPSG",8802]],'
                  f'PARAMETER["False easting",0,{METRE},ID["EPSG",8806]],PARAMETER["False northing",0,{METRE},ID["EPSG",8807]]')
    else:
        method = 'METHOD["Transverse Mercator",ID["EPSG",9807]]'
        params = (f'PARAMETER["Latitude of natural origin",0,{DEGREE},ID["EPSG",8801]],'
                  f'PARAMETER["Longitude of natural origin",{num(s["centralMeridian"])},{DEGREE},ID["EPSG",8802]],'
                  f'PARAMETER["Scale factor at natural origin",{num(s["scaleFactor"])},SCALEUNIT["unity",1],ID["EPSG",8805]],'
                  f'PARAMETER["False easting",{num(s["falseEasting"])},{METRE},ID["EPSG",8806]],'
                  f'PARAMETER["False northing",{num(s["falseNorthing"])},{METRE},ID["EPSG",8807]]')
    if s["axisOrder"] == "ne":
        axes = f'AXIS["northing (X)",north,ORDER[1],{METRE}],AXIS["easting (Y)",east,ORDER[2],{METRE}]'
    else:
        axes = f'AXIS["easting (X)",east,ORDER[1],{METRE}],AXIS["northing (Y)",north,ORDER[2],{METRE}]'
    conversion = s["name"].rsplit(" / ", 1)[-1]
    return (f'PROJCRS["{s["name"]}",BASEGEOGCRS["{base}",{d}],CONVERSION["{conversion}",{method},{params}],'
            f'CS[Cartesian,2],{axes},{code}]')


def record(srid):
    s = REGISTRY[srid]
    out = {"srid": srid, "name": s["name"], "wkt": wkt2(s), "axisOrder": s.get("axisOrder") or "en", "unit": s["unit"]}
    if s.get("area"):
        out["area"] = s["area"]
    return out


def check_wkt():
    """Every registry system's WKT2, read by PROJ, is its EPSG code: the same axis order, the same points."""
    problems = []
    for srid, s in REGISTRY.items():
        if s["kind"] == "local":
            continue
        text = wkt2(s)
        mine, theirs = CRS.from_wkt(text), CRS.from_epsg(srid)
        axes = [a.direction for a in mine.axis_info]
        if axes != [a.direction for a in theirs.axis_info]:
            problems.append(f"EPSG:{srid}: eksenler {axes}")
        a = Transformer.from_crs("EPSG:4326", mine, always_xy=True)
        b = Transformer.from_crs("EPSG:4326", theirs, always_xy=True)
        for _, lat, lon in ct.PLACES:
            if s.get("projection") in ("Transverse Mercator", "UTM") and abs(lon - s["centralMeridian"]) > 6:
                continue
            p, q = a.transform(lon, lat), b.transform(lon, lat)
            if max(abs(p[0] - q[0]), abs(p[1] - q[1])) > 1e-6:
                problems.append(f"EPSG:{srid}: {lat},{lon} → {p} ≠ {q}")
    if problems:
        raise SystemExit("; ".join(problems))


def moved(src, x, y):
    """(x, y) of the registry system `src` in the project's (TUREF / TM36), along ADR 0167's EPSG path."""
    t, _, _ = ct.transformer(REGISTRY[src], REGISTRY[PROJECT])
    p = t.transform(x, y)
    return {"x": p[0], "y": p[1]}


def layer(id_, name, **extra):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": [], **extra}


# Places: a point of the project's own; an old sheet in ED50 / TM36 (its numbers as ED50's); GPS fixes in
# longitude, latitude; a point whose numbers are no longitude.
ED50 = [(512345.678, 4420123.456), (512400.0, 4420150.0), (512380.0, 4420200.0)]
GPS = [(32.8597, 39.9334), (32.8640, 39.9370)]
# One point of Ankara in other systems' numbers (as their own): ED50 and WGS 84 UTM 36N, TUREF's latitude and
# longitude, Web Mercator.
OTHERS = {"utm": (23036, (488000.0, 4420000.0)), "wgsutm": (32636, (488000.0, 4420000.0)),
          "geo": (5252, (32.86, 39.93)), "merc": (3857, (3657946.0, 4855000.0)), "tm33": (2321, (757000.0, 4425000.0))}
RING = [(512000.0, 4421000.0), (512100.0, 4421000.0), (512100.0, 4421100.0), (512000.0, 4421100.0)]
HOLE = [(512030.0, 4421030.0), (512060.0, 4421030.0), (512060.0, 4421060.0), (512030.0, 4421060.0)]


def setup():
    ents = [
        {"kind": "point", "id": 1, "layerId": "tum", "attrs": {}, "p": {"x": 512000.0, "y": 4420000.0}},
        {"kind": "point", "id": 2, "layerId": "eski", "attrs": {"ad": "S1"}, "p": {"x": ED50[0][0], "y": ED50[0][1]},
         "z": 875.25},
        {"kind": "polyline", "id": 3, "layerId": "eski", "attrs": {},
         "pts": [{"x": x, "y": y} for x, y in ED50]},
        {"kind": "point", "id": 4, "layerId": "gps", "attrs": {"ad": "G1"}, "p": {"x": GPS[0][0], "y": GPS[0][1]}},
        {"kind": "point", "id": 5, "layerId": "gps", "attrs": {"ad": "G2"}, "p": {"x": GPS[1][0], "y": GPS[1][1]}},
        {"kind": "point", "id": 6, "layerId": "bozuk", "attrs": {}, "p": {"x": 512000.0, "y": 4420000.0}},
        {"kind": "point", "id": 7, "layerId": "kilitli", "attrs": {}, "p": {"x": 32.86, "y": 39.93}},
        {"kind": "polygon", "id": 8, "layerId": "parsel", "attrs": {"ada": "101"},
         "pts": [{"x": x, "y": y} for x, y in RING], "holes": [{"pts": [{"x": x, "y": y} for x, y in HOLE]}]},
        {"kind": "polyline", "id": 9, "layerId": "kotlu", "attrs": {},
         "pts": [{"x": x, "y": y} for x, y in ED50[:2]], "zs": [870.5, 871.25]},
    ] + [{"kind": "point", "id": 10 + i, "layerId": lid, "attrs": {}, "p": {"x": xy[0], "y": xy[1]}}
         for i, (lid, (_, xy)) in enumerate(OTHERS.items())]
    layers = [layer("0", "0"), layer("tum", "Tüm"), layer("eski", "Eski pafta"), layer("gps", "GPS"),
              layer("bozuk", "Bozuk"), layer("kilitli", "Kilitli"), layer("parsel", "Parsel"), layer("kotlu", "Kotlu"),
              layer("bos", "Boş"), *[layer(lid, lid.upper()) for lid in OTHERS],
              {"id": "grup", "name": "Grup", "type": "group", "visible": True, "locked": False, "expanded": True,
               "style": STYLE, "children": []}]
    next(n for n in layers if n["id"] == "kilitli")["locked"] = True
    return {"format": "kentos.document", "version": 1, "name": "Kaynak sistemleri",
            "settings": {"srid": PROJECT, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000, "workspace": "gis"},
            "origin": {"x": 500000, "y": 4400000}, "layers": layers, "activeLayer": "0", "entities": ents,
            "styles": {"items": [], "categories": []}}


def done(output, warnings=()):
    return {"status": "completed", "output": output, "warnings": list(warnings)}


def failed(code, message, path, uids=None):
    e = {"code": code, "message": message, "path": path}
    if uids:
        e["uids"] = uids
    return {"status": "failed", "error": e}


SAME = {"revision": "same"}
CHANGED = {"revision": "changed"}


def step(inp, result, expect=None, command=None, within=None):
    s = {"op": "execute", "input": inp, "result": result, "expect": expect or SAME}
    if command:
        s["command"] = command
    if within:
        s["within"] = within
    return s


CONFLICT = ("Çizim bu komut hazırlandıktan sonra değişti; hiçbir şey yazılmadı. Komutu çizimin şimdiki hâline göre "
            "yeniden hazırlayın.")


def stale(inp):
    """A command prepared before the drawing changed: another layer's record is written in between."""
    return [{"op": "captureRevision", "as": "once"},
            step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": record(4326), "changed": True,
                                                        "revision": "$current"}), CHANGED, command="cad.layers.crs"),
            {"op": "execute", "input": {**inp, "expectedRevision": "$once"},
             "result": {"status": "conflict", "error": {"code": "revision_conflict", "message": CONFLICT,
                                                       "path": "expectedRevision", "revision": "$current"}},
             "expect": SAME}]


def case(name, *steps):
    return {"name": name, "steps": list(steps)}


def crs_cases():
    gps = record(4326)
    out = [
        case("GPS katmanının kaynağı WGS 84: kayıt adı, WKT2'si, eksen sırası ve kullanım alanıyla",
             step({"layer": "gps", "srid": 4326},
                  done({"layer": "gps", "crs": gps, "changed": True, "revision": "$current"}), CHANGED)),
        case("aynı kayıt ikinci kez yazılmaz",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": True,
                                                         "revision": "$current"}), CHANGED),
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": False,
                                                         "revision": "$current"}))),
        case("kayıt silinir",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": gps, "changed": True,
                                                         "revision": "$current"}), CHANGED),
             step({"layer": "gps"}, done({"layer": "gps", "changed": True, "revision": "$current"}), CHANGED)),
        case("kaydı olmayanın silinmesi bir şey yazmaz",
             step({"layer": "tum"}, done({"layer": "tum", "changed": False, "revision": "$current"}))),
        case("kilitli katmana da atanır: veri değişmez",
             step({"layer": "kilitli", "srid": 4326}, done({"layer": "kilitli", "crs": gps, "changed": True,
                                                             "revision": "$current"}), CHANGED)),
    ]
    for srid in (5256, 2322, 23036, 32636, 3857, 5252, 2319, 5259, 23038, 32635, 2325):
        out.append(case(f"{REGISTRY[srid]['name']} kaydı (EPSG:{srid})",
                        step({"layer": "tum", "srid": srid},
                             done({"layer": "tum", "crs": record(srid), "changed": True, "revision": "$current"}),
                             CHANGED)))
    out += [
        case("ret: kayıtta olmayan kod",
             step({"layer": "gps", "srid": 9999},
                  failed("unknown_srid", "EPSG:9999 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin.",
                         "srid"))),
        case("ret: yerel sistem kaynak sistemi değildir",
             step({"layer": "gps", "srid": 0},
                  failed("unknown_srid", "Yerel sistem bir kaynak sistemi değildir; kaydı silmek için srid vermeyin.",
                         "srid"))),
        case("ret: grup",
             step({"layer": "grup", "srid": 4326},
                  failed("not_a_layer",
                         "“Grup” bir katman grubu; kaynak sistemi yalnız katmanın olur. Grubun bir katmanını verin.",
                         "layer"))),
        case("sürüm çakışması: hazırlandıktan sonra çizim değişmişse hiçbir şey yazılmaz",
             *stale({"layer": "tum", "srid": 4326})),
        case("ret: sürüm metni değil",
             step({"layer": "gps", "srid": 4326, "expectedRevision": "x"}, failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))),
        case("ret: olmayan katman",
             step({"layer": "yok", "srid": 4326},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                         "layer"))),
    ]
    return out


def ent(id_, kind, layer_id, attrs, **geom):
    return {"kind": kind, "id": id_, "layerId": layer_id, "attrs": attrs, **geom}


def reproject_cases():
    eski = record(2322)
    p0 = moved(2322, *ED50[0])
    line = [moved(2322, x, y) for x, y in ED50]
    g = [moved(4326, x, y) for x, y in GPS]
    old_entities = {"2": ent(2, "point", "eski", {"ad": "S1"}, p={"x": ED50[0][0], "y": ED50[0][1]}, z=875.25),
                    "3": ent(3, "polyline", "eski", {}, pts=[{"x": x, "y": y} for x, y in ED50])}
    new_entities = {"2": ent(2, "point", "eski", {"ad": "S1"}, p=p0, z=875.25),
                    "3": ent(3, "polyline", "eski", {}, pts=line)}
    out = [
        case("ED50 / TM36'da sanılan eski pafta projenin sistemine: noktalar EPSG yoluyla, kot kalır, kayıt ED50",
             step({"layer": "eski", "from": 2322},
                  done({"layer": "eski", "from": 2322, "to": PROJECT, "moved": 2, "via": "$any", "crs": eski,
                        "revision": "$current"}),
                  {"revision": "changed", "entities": new_entities}, within=WITHIN)),
        case("geri alma eski koordinatları ve kaydın yokluğunu geri getirir",
             step({"layer": "eski", "from": 2322},
                  done({"layer": "eski", "from": 2322, "to": PROJECT, "moved": 2, "via": "$any", "crs": eski,
                        "revision": "$current"}), CHANGED, within=WITHIN),
             {"op": "undo", "returns": "Yeniden izdüşür", "expect": {"entities": old_entities}}),
        case("plan hiçbir şey yazmaz",
             {"op": "plan", "input": {"layer": "eski", "from": 2322},
              "result": done({"layer": "eski", "from": 2322, "to": PROJECT, "moved": 2, "via": "$any", "crs": eski,
                              "revision": "$current"}), "expect": SAME, "within": WITHIN}),
        case("kaydı olan katman from'suz: önce Sistem ata, sonra Yeniden izdüşür",
             step({"layer": "gps", "srid": 4326}, done({"layer": "gps", "crs": record(4326), "changed": True,
                                                         "revision": "$current"}), CHANGED, command="cad.layers.crs"),
             step({"layer": "gps"},
                  done({"layer": "gps", "from": 4326, "to": PROJECT, "moved": 2, "via": "$any", "crs": record(4326),
                        "revision": "$current"}),
                  {"revision": "changed", "entities": {
                      "4": ent(4, "point", "gps", {"ad": "G1"}, p=g[0]),
                      "5": ent(5, "point", "gps", {"ad": "G2"}, p=g[1])}}, within=WITHIN)),
        *[case(f"{REGISTRY[srid]['name']} numaraları projenin sistemine (EPSG:{srid} → {PROJECT})",
               step({"layer": lid, "from": srid},
                    done({"layer": lid, "from": srid, "to": PROJECT, "moved": 1, "via": "$any", "crs": record(srid),
                          "revision": "$current"}),
                    {"revision": "changed", "entities": {str(10 + i): ent(10 + i, "point", lid, {}, p=moved(srid, *xy))}},
                    within=WITHIN))
          for i, (lid, (srid, xy)) in enumerate(OTHERS.items())],
        case("delikli alan: dış halka ve delik birlikte",
             step({"layer": "parsel", "from": 2322},
                  done({"layer": "parsel", "from": 2322, "to": PROJECT, "moved": 1, "via": "$any", "crs": eski,
                        "revision": "$current"}),
                  {"revision": "changed", "entities": {"8": ent(8, "polygon", "parsel", {"ada": "101"},
                                                              pts=[moved(2322, x, y) for x, y in RING],
                                                              holes=[{"pts": [moved(2322, x, y) for x, y in HOLE]}])}},
                  within=WITHIN)),
        case("kotlu çizgi: köşelerin kotları kalır",
             step({"layer": "kotlu", "from": 2322},
                  done({"layer": "kotlu", "from": 2322, "to": PROJECT, "moved": 1, "via": "$any", "crs": eski,
                        "revision": "$current"}),
                  {"revision": "changed", "entities": {"9": ent(9, "polyline", "kotlu", {},
                                                              pts=[moved(2322, x, y) for x, y in ED50[:2]],
                                                              zs=[870.5, 871.25])}},
                  within=WITHIN)),
        case("boş katman: hiçbir nesne çevrilmez, kayıt yazılır",
             step({"layer": "bos", "from": 2322},
                  done({"layer": "bos", "from": 2322, "to": PROJECT, "moved": 0, "via": "", "crs": eski,
                        "revision": "$current"}), CHANGED)),
        case("sürüm çakışması: hazırlandıktan sonra çizim değişmişse hiçbir şey yazılmaz",
             *stale({"layer": "eski", "from": 2322})),
        case("ret: sürüm metni değil",
             step({"layer": "eski", "from": 2322, "expectedRevision": "x"}, failed("invalid_revision", "Beklenen sürüm “x” geçerli bir sürüm değil; sürüm “12” gibi bir tamsayı yazısıdır. Sürümü belgeden ya da komutun planından alın.", "expectedRevision"))),
        case("ret: olmayan katman",
             step({"layer": "yok", "from": 2322},
                  failed("layer_not_found", "“yok” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.",
                         "layer"))),
        case("ret: kaydı olmayan katman from'suz",
             step({"layer": "eski"},
                  failed("invalid_input",
                         "“Eski pafta” katmanının kaynak sistemi kaydı yok; nesnelerin şimdiki sistemini from ile verin.",
                         "from"))),
        case("ret: nesneler zaten projenin sisteminde",
             step({"layer": "tum", "from": PROJECT},
                  failed("crs_mismatch",
                         f"Nesneler zaten projenin sisteminde (EPSG:{PROJECT}); çevrilecek bir şey yok. Yalnız kayıt için cad.layers.crs.",
                         "from"))),
        case("ret: kayıtta olmayan kod",
             step({"layer": "eski", "from": 9999},
                  failed("unknown_srid", "EPSG:9999 koordinat sistemleri kaydında yok. Kayıttaki bir sistemin kodunu verin.",
                         "from"))),
        case("ret: kilitli katman",
             step({"layer": "kilitli", "from": 4326},
                  failed("layer_locked", "“Kilitli” katmanı kilitli; kilidi Katmanlar panelinden açın.", "layer"))),
        case("ret: grup",
             step({"layer": "grup", "from": 4326},
                  failed("not_a_layer",
                         "“Grup” bir katman grubu; kaynak sistemi yalnız katmanın olur. Grubun bir katmanını verin.",
                         "layer"))),
        case("ret: boylam olamayacak sayılar WGS 84'ten çevrilemez, hiçbir şey yazılmaz",
             step({"layer": "bozuk", "from": 4326},
                  failed("crs_unreached",
                         "“Bozuk” katmanının 1 nesnesi EPSG:4326'den projenin sistemine çevrilemiyor (sistemin dışında); hiçbir şey yazılmadı. Doğru sistemi from ile verin.",
                         "from", ["$uidOf:6"]))),
    ]
    return out


def build(name, title, note, cases):
    return {"format": "kentos.command-cases", "version": 1, "command": name, "commandVersion": 1, "title": title,
            "note": note, "setup": setup(), "cases": cases}


def main():
    check_wkt()
    note = ("ADR 0583. Kayıt koordinat sistemleri kaydından; WKT2 EPSG'nin tanımından yazılır ve PROJ'da (pyproj) kodun "
            "kendisiyle aynı eksen sırası ve aynı noktalar olarak okunur. Çevrilen koordinatlar PROJ'un, ADR 0167'nin "
            "her datum çifti için adını verdiği EPSG yoluyla; 1 mm içinde. Bağımsız başvuru: "
            "scripts/fixtures/layer_crs_cases.py.")
    files = {
        "cad.layers.crs.json": build("cad.layers.crs", "Sistem ata: katmanın kaynak sistemi kaydı", note, crs_cases()),
        "cad.layers.reproject.json": build("cad.layers.reproject",
                                           "Yeniden izdüşür: katmanın nesneleri bir sistemden projeninkine", note,
                                           reproject_cases()),
    }
    bad = False
    for name, data in files.items():
        text = json.dumps(data, ensure_ascii=False, indent=1) + "\n"
        path = OUT / name
        if "--check" in sys.argv:
            if path.read_text() != text:
                print(f"{name} güncel değil", file=sys.stderr)
                bad = True
            else:
                print(f"{name}: {len(data['cases'])} durum")
        else:
            path.write_text(text)
            print(f"{name}: {len(data['cases'])} durum yazıldı")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
