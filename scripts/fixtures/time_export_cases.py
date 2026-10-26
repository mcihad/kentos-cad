#!/usr/bin/env python3
"""The shared cases of the time filter in export (docs/adr/0646 §2; plan/gis.md CBS-33).

    python3 scripts/fixtures/time_export_cases.py             # writes fixtures/time-export/v1/{drawing.kcad,cases.json}
    python3 scripts/fixtures/time_export_cases.py --check     # compares
    .run/geo-py/bin/python scripts/fixtures/time_export_cases.py --written
                                                              # reads the files KentOS wrote (fixtures/time-export/v1/out,
                                                              # KENTOS_WRITE_TIME_EXPORT=1 cargo test -p kentos-headless
                                                              # --test headless time_exports) with OGR and pyarrow

No KentOS code is used. A moment is read and a valid time and a window's rule applied by temporal_cases.py (ADR 0210
§3–§4); a version's recorded interval is ADR 0537's rule. ADR 0646 §2: each written layer with time settings (or every
written layer when the question names the fields) keeps the objects the question shows; a layer without them is
written whole; the day known applies to the layers that record their versions. The drawing is written by the
independent KCAD v2 writer (kcad_v2_reference.py). Every object carries a number `no`; a file's objects are checked by
the numbers it holds (XYZ, which holds no attributes, by its vertices' eastings).
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
import kcad_v2_reference as kr  # noqa: E402
import temporal_cases as tc  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
DIR = ROOT / "fixtures/time-export/v1"
OUT = DIR / "out"
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

PARSEL_TIME = {"start": "gecerlilik_baslangic", "end": "gecerlilik_bitis", "key": "parsel_no",
               "recordedStart": "kayit_baslangic", "recordedEnd": "kayit_bitis"}
ARIZA_TIME = {"start": "an"}
TIMES = {"parsel": PARSEL_TIME, "ariza": ARIZA_TIME}
NAMES = {"parsel": "Parsel", "ariza": "Arıza", "zamansiz": "Zamansız"}

OBJECTS = {
    "parsel": [
        (1, {"parsel_no": "101", "gecerlilik_baslangic": "2020-01-01", "gecerlilik_bitis": "2024-03-01",
             "kayit_baslangic": "2020-01-05", "kayit_bitis": "2024-03-02"}),
        (2, {"parsel_no": "101", "gecerlilik_baslangic": "2024-03-01", "kayit_baslangic": "2024-03-02"}),
        (3, {"parsel_no": "102", "gecerlilik_baslangic": "2022-06-01", "kayit_baslangic": "2025-01-10T09:00:00+03:00"}),
        (4, {"parsel_no": "103", "gecerlilik_baslangic": "2021-01-01", "gecerlilik_bitis": "2023-01-01",
             "kayit_baslangic": "2021-01-02", "kayit_bitis": "2023-01-01"}),
        (5, {"parsel_no": "104", "kayit_baslangic": "2022-01-01"}),
        (6, {"parsel_no": "105", "gecerlilik_baslangic": "2024-06-01T00:30:00+03:00", "kayit_baslangic": "2024-06-01"}),
    ],
    "ariza": [
        (7, {"an": "2024-06-01T10:00:00Z"}),
        (8, {"an": "2024-06-02"}),
        (9, {"an": "2024-05-20"}),
    ],
    "zamansiz": [
        (10, {"baslangic": "2019-01-01"}),
        (11, {"baslangic": "2025-01-01"}),
        (12, {}),
    ],
}


def uid(n):
    return f"0192f5a3-5555-7000-8000-{n:012d}"


def square(n, layer, attrs):
    x, y = 500000.0 + 30 * n, 4400000.0
    pts = [{"x": x, "y": y}, {"x": x + 20.0, "y": y}, {"x": x + 20.0, "y": y + 20.0}, {"x": x, "y": y + 20.0}]
    return {"kind": "polygon", "id": n, "layerId": layer, "attrs": {**attrs, "no": str(n)}, "pts": pts}


def node(id_, **extra):
    return {"id": id_, "name": NAMES[id_], "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": STYLE, "children": [], **extra}


def drawing():
    layers = [node("parsel", time=PARSEL_TIME), node("ariza", time=ARIZA_TIME), node("zamansiz")]
    ents = [square(n, layer, a) for layer, objs in OBJECTS.items() for n, a in objs]
    return {"format": "kentos.document", "version": 2, "name": "Zamanla dışa aktarma",
            "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                         "plotScale": 1000.0},
            "origin": {"x": 500000.0, "y": 4400000.0}, "layers": layers, "activeLayer": "parsel", "entities": ents,
            "uids": [uid(e["id"]) for e in ents], "styles": {"items": [], "categories": []}}


# ── The rule ──────────────────────────────────────────────────────────


def recorded(start, end):
    s = tc.read(start) if start is not None else "empty"
    e = tc.read(end) if end is not None else "empty"
    if not isinstance(s, int) or e == "unreadable":
        return None
    return (s, tc.INF if e == "empty" else e)


def kept(layer, time):
    """The numbers of `layer`'s objects the export writes for the question `time`."""
    fields = time.get("fields") or TIMES.get(layer)
    nos = [n for n, _ in OBJECTS[layer]]
    if fields is None:
        return nos
    window = None
    if "at" in time:
        window = {"instant": tc.read(time["at"])}
    elif "from" in time:
        window = {"range": (tc.read(time["from"]), tc.read(time["to"]))}
    known = tc.read(time["knownAt"]) if "knownAt" in time and "recordedStart" in fields else None
    rule = {"ranged": "end" in fields, "cumulative": fields.get("cumulative", False)}
    out = []
    for n, attrs in OBJECTS[layer]:
        start = attrs.get(fields["start"])
        end = attrs.get(fields["end"]) if rule["ranged"] else None
        valid = tc.object_time(rule, start, end)
        if valid is not None and window is not None and not tc.shows(valid, window):
            continue
        if known is not None:
            iv = recorded(attrs.get(fields["recordedStart"]), attrs.get(fields.get("recordedEnd")))
            if not (iv is not None and iv[0] <= known < iv[1]):
                continue
        out.append(n)
    return out


# ── The cases ─────────────────────────────────────────────────────────


def written(command, name, layers, time, extra=None):
    expect = {layer: kept(layer, time) for layer in layers}
    left = sum(len(OBJECTS[layer]) - len(expect[layer]) for layer in layers)
    warnings = [{"code": "time_left_out", "message": f"Zamanın dışında kalan {left} nesne yazılmadı.",
                 "path": "time"}] if left else []
    return {"name": name, "command": command,
            "input": {"path": f"$out:{name}", "layers": layers, "time": time, **(extra or {})},
            "status": "completed", "warnings": warnings, "kept": expect}


def refused(name, command, layers, time, code, message, path):
    return {"name": name, "command": command, "input": {"path": f"$out:{name}", "layers": layers, "time": time},
            "status": "failed", "error": {"code": code, "message": message, "path": path}}


def moment_message(text):
    return (f"“{text}” bir an değil: bir tarih (2024-06-01 ya da 01.06.2024) ya da tarih ve saat (2024-06-01T14:30, "
            "bölgesiyle 2024-06-01T14:30+03:00) yazın.")


ALL = ["parsel", "ariza", "zamansiz"]


def cases():
    return [
        written("file.export.gpkg", "an.gpkg", ALL, {"at": "2024-06-01T10:00:00Z"}),
        written("file.export.gpkg", "aralik.gpkg", ALL, {"from": "2024-05-15", "to": "2024-06-02"}),
        written("file.export.gpkg", "bilinen.gpkg", ALL, {"at": "2024-06-01", "knownAt": "2025-01-01"}),
        written("file.export.gpkg", "bilinen-sonra.gpkg", ["parsel"], {"at": "2024-06-01", "knownAt": "2025-02-01"}),
        written("file.export.gpkg", "alanlarla.gpkg", ["zamansiz"],
                {"at": "2020-01-01", "fields": {"start": "baslangic"}}),
        written("file.export.csv", "an.csv", ["parsel"], {"at": "2022-07-01"}),
        written("file.export.fgb", "aralik.fgb", ["parsel"], {"from": "2023-01-01", "to": "2024-03-01"}),
        written("file.export.parquet", "bilinen.parquet", ["parsel"], {"at": "2024-06-01", "knownAt": "2024-03-01"}),
        written("file.export.gml", "an.gml", ["parsel"], {"at": "2024-02-01"}),
        written("file.export.shp", "an-shp.zip", ["parsel"], {"at": "2024-05-31T21:00:00Z"}),
        written("file.export.xyz", "an.xyz", ["parsel"], {"at": "2021-06-01"}, {"vertices": "all"}),
        written("file.export.gpkg", "hepsi.gpkg", ["parsel"], {"knownAt": "2030-01-01"}),
        refused("zamansiz.gpkg", "file.export.gpkg", ["zamansiz"], {"at": "2024-06-01"}, "not_temporal",
                "Yazılan katmanların hiçbirinin zaman ayarı yok; zaman alanlarını (time.fields) verin.", "time"),
        refused("kayitsiz.gpkg", "file.export.gpkg", ["ariza", "zamansiz"], {"knownAt": "2024-06-01"}, "not_recorded",
                "Yazılan katmanların hiçbiri sürümlerinin kayıt zamanını tutmuyor; bilinen günle yazmak için kayıt "
                "başlangıcı alanını (time.fields.recordedStart) verin.", "time.knownAt"),
        refused("okunmayan.gpkg", "file.export.gpkg", ["parsel"], {"at": "dün"}, "invalid_moment",
                moment_message("dün"), "time.at"),
        refused("bos.gpkg", "file.export.gpkg", ["parsel"], {}, "invalid_period",
                "Zaman sorusunda bir an (at), bir aralık (from, to) ya da bilinen gün (knownAt) verin.", "time"),
        refused("ters.csv", "file.export.csv", ["parsel"], {"from": "2024-06-01", "to": "2024-01-01"},
                "invalid_period", "Aralığın bitişi başlangıcından sonra olmalı; [from, to) bitişi içermez.",
                "time.to"),
    ]


def build():
    return {"format": "kentos.time-export-cases", "version": 1,
            "note": ("ADR 0646 §2. Bağımsız başvuru scripts/fixtures/time_export_cases.py: anlar ve pencerenin kuralı "
                     "temporal_cases.py'den (ADR 0210), kayıt zamanı ADR 0537'nin kuralıyla; KentOS kodu yok. "
                     "KentOS'un yazdığı dosyalar --written ile OGR ve pyarrow'la okunur."),
            "drawing": "drawing.kcad", "cases": cases()}


# ── The files KentOS wrote ────────────────────────────────────────────


def numbers_in(path):
    """The `no` values a written file holds (all its tables), or the eastings of an XYZ file's vertices."""
    if path.suffix == ".xyz":
        xs = set()
        for line in path.read_text("utf-8").splitlines():
            parts = line.replace(",", " ").split()
            if len(parts) >= 2:
                try:
                    xs.add(round(float(parts[0]), 3))
                except ValueError:
                    pass
        return xs
    if path.suffix == ".parquet":
        import pyarrow.parquet as pq
        return {str(v) for v in pq.read_table(path).column("no").to_pylist()}
    from osgeo import gdal, ogr
    ogr.UseExceptions()
    gdal.PushErrorHandler("CPLQuietErrorHandler")
    name = f"/vsizip/{path}" if path.name.endswith(".zip") else str(path)
    ds = ogr.Open(name)
    if ds is None:
        raise SystemExit(f"{path.name}: OGR açamadı")
    out = set()
    for i in range(ds.GetLayerCount()):
        lyr = ds.GetLayerByIndex(i)
        if lyr.GetLayerDefn().GetFieldIndex("no") < 0:
            continue
        for f in lyr:
            out.add(str(f.GetField("no")))
    return out


def check_written():
    bad = 0
    for c in cases():
        if c["status"] != "completed":
            continue
        path = OUT / c["name"]
        if not path.exists():
            print(f"{c['name']}: yazılmamış (önce KENTOS_WRITE_TIME_EXPORT=1 ile başsız sunucunun testi)")
            bad += 1
            continue
        nos = [n for ns in c["kept"].values() for n in ns]
        if path.suffix == ".xyz":
            want = {round(500000.0 + 30 * n + dx, 3) for n in nos for dx in (0.0, 20.0)}
        else:
            want = {str(n) for n in nos}
        got = numbers_in(path)
        if got != want:
            print(f"{c['name']}: beklenen {sorted(want)}, dosyada {sorted(got)}")
            bad += 1
    if bad:
        return 1
    print(f"Zamanla dışa aktarılan {sum(c['status'] == 'completed' for c in cases())} dosya başvuruyla aynı")
    return 0


def main():
    if "--written" in sys.argv:
        return check_written()
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    kcad = kr.container(kr.document(drawing()))
    if "--check" in sys.argv:
        if (DIR / "cases.json").read_text("utf-8") != text or (DIR / "drawing.kcad").read_bytes() != kcad:
            print("fixtures/time-export/v1 eski: betiği --check'siz çalıştırın", file=sys.stderr)
            return 1
        print(f"Zamanla dışa aktarma durumları tutarlı: {len(cases())} durum")
        return 0
    DIR.mkdir(parents=True, exist_ok=True)
    (DIR / "drawing.kcad").write_bytes(kcad)
    (DIR / "cases.json").write_text(text, "utf-8")
    print(f"yazıldı: fixtures/time-export/v1 ({len(cases())} durum)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
