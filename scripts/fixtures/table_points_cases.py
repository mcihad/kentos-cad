#!/usr/bin/env python3
"""The shared cases of Tablodan nokta (docs/adr/0223 §5), İşlemler's tool on both platforms.

    python3 scripts/fixtures/table_points_cases.py           # writes the drawing, the tables and the cases
    python3 scripts/fixtures/table_points_cases.py --check   # writes nothing; compares

Writes fixtures/processing/v1/table-points.kcad (a WGS 84 / UTM 36N drawing),
nokta-duzlem.csv (plane coordinates), nokta-wgs84.ods (longitudes and
latitudes, an OpenDocument spreadsheet written here by hand) and
table-points.json, the runs' expectations. The rule is written here from the
ADR on its own: the tables are read by table_file_cases.py's rule within
İşlemler's limits, a cell's number by docs/adr/0180's rule, a geographic
cell also as degrees, minutes and seconds (the rule below), and a point is
moved from WGS 84 with PROJ (pyproj, EPSG:4326 → EPSG:32636), not with
KentOS.

- The source: a file (its first row the columns' names, trimmed; a row's
  number its line, the names' line 1) or objects (their attributes; the
  columns every key in code point order, a missing one empty; a row's
  number the object's #id). A column the source does not have refuses the
  run: “Kaynakta “X” alanı yok; alanları: …”.
- A row's east and north: empty → “X” boş; in the drawing's or a plane
  system: docs/adr/0180's number (a sign, digits, one point or comma),
  else “X” sayı değil: <the cell>; in a geographic system (the source's
  when it is not the project's, else the project's): the number, else
  degrees, minutes and seconds (spaces or ° ′ ″ ' "; a hemisphere letter K,
  G, D, B or N, S, E, W at the end, G B S W turning it negative; only the
  last part with a fraction; minutes and seconds below 60), the longitude
  within ±180 and the latitude within ±90, else “X” coğrafi sınırın dışında
  (boylam ±180, enlem ±90): <the cell>. The elevation: an empty cell none,
  else the number, else “Z” sayı değil: <the cell>.
- The point: from the source's system into the project's when they differ
  (PROJ), its elevation as it is; its attributes the row's filled cells by
  their columns' names and, when an Ad column is given, Ad its cell.
- The run: a new layer Noktalar (the conversion tools' look, #0D74CE), the
  points in the rows' order, one undo step “Tablodan nokta”; “N satırdan M
  nokta yazıldı.”; the rows that gave none in the table Satır, Neden, and
  “K satır atlandı; nedenleri tabloda.”
"""
import io
import json
import sys
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import table_file_cases as tf  # noqa: E402
from layer_ops_cases import new_id  # noqa: E402

from pyproj import Transformer  # noqa: E402

ROOT = HERE.parents[1]
PROC = ROOT / "fixtures/processing/v1"
TOL = 1e-6
COLOR = "#0D74CE"
LAYER = "Noktalar"
TOOL = "convert.tablePoints"
UNDO = "Tablodan nokta"
WGS84_TO_UTM36 = Transformer.from_crs("EPSG:4326", "EPSG:32636", always_xy=True)

# ── The drawing ─────────────────────────────────────────────────────────


def style(color):
    return {"color": color, "lineType": "continuous", "lineWeight": 0.25}


LAYERS = [
    {"id": "cizim", "name": "Çizim", "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style("fg"), "children": []},
    {"id": "olcum", "name": "Ölçüm", "type": "layer", "visible": True, "locked": False, "expanded": True, "style": style("#E5484D"), "children": []},
]


def ent(i, attrs, x, y):
    return {"id": i, "kind": "point", "layerId": "olcum", "attrs": attrs, "p": {"x": x, "y": y}}


# Measured points whose attributes hold their geographic coordinates (their own places elsewhere).
ENTITIES = [
    ent(1, {"Nokta": "K1", "Boylam": "32.8597", "Enlem": "39.9334", "Not": "kuzey"}, 488000, 4420000),
    ent(2, {"Nokta": "K2", "Boylam": "32°51'40\"", "Enlem": "39°56'05\"K"}, 488010, 4420010),
    ent(3, {"Nokta": "K3", "Boylam": "", "Enlem": "39.9"}, 488020, 4420020),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: Tablodan nokta",
    "settings": {"srid": 32636, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": 488000, "y": 4420000},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# ── The tables ──────────────────────────────────────────────────────────

PLANE = (
    "Ad;Y;X;Z;Kod\n"
    "P1;488100,125;4420200,5;812,40;SN\n"
    "P2;488150;4420250.75;;YL\n"
    "P3;;4420300;800;SN\n"
    "P4;488200;kuzey;800;SN\n"
    "P5;488250;4420350;yüksek;SN\n"
    "P6;  488300.5 ;4420400;805.125;\n"
)

ODS_CELL = '<table:table-cell office:value-type="string"><text:p>{}</text:p></table:table-cell>'
ODS_FLOAT = '<table:table-cell office:value-type="float" office:value="{0}"><text:p>{0}</text:p></table:table-cell>'


def ods_row(cells):
    return "<table:table-row>" + "".join(cells) + "</table:table-row>"


def wgs84_ods():
    """Longitudes and latitudes: numbers, decimal commas, degrees, minutes and seconds, a hemisphere, past the
    bounds, words; a second sheet the tool does not read."""
    s = lambda t: ODS_CELL.format(t)
    f = lambda v: ODS_FLOAT.format(v)
    rows = [
        ods_row([s("Nokta"), s("Boylam"), s("Enlem"), s("Yükseklik"), s("Tarih")]),
        ods_row([s("A1"), f("32.8597"), f("39.9334"), f("938.5"),
                 '<table:table-cell office:value-type="date" office:date-value="2024-10-01"><text:p>01.10.2024</text:p></table:table-cell>']),
        ods_row([s("A2"), s("32,86"), s("39,93"), s(""), s("")]),
        ods_row([s("A3"), s("32°51′35″"), s("39 55 48.2 K"), f("901"), s("")]),
        ods_row([s("A4"), s("32 51 35 D"), s("39.9"), s(""), s("")]),
        ods_row([s("A5"), f("190"), f("39.9"), s(""), s("")]),
        ods_row([s("A6"), f("32.9"), s("kuzey"), s(""), s("")]),
        ods_row([s("A7"), f("32.9"), f("39.95"), s("çok"), s("")]),
    ]
    content = (
        tf.ODS_HEAD
        + '<table:table table:name="Noktalar">' + "".join(rows) + "</table:table>"
        + '<table:table table:name="Notlar">' + ods_row([s("başka sayfa")]) + "</table:table>"
        + tf.ODS_TAIL
    )
    kind = "application/vnd.oasis.opendocument.spreadsheet"
    return tf.zipped([("mimetype", kind), ("META-INF/manifest.xml", tf.MANIFEST.format(kind=kind)), ("content.xml", content)], stored_first=True)


FILES = {"nokta-duzlem.csv": PLANE.encode("utf-8"), "nokta-wgs84.ods": wgs84_ods()}

# ── The rule ────────────────────────────────────────────────────────────

SPACES = " \t\n\r\x0b\x0c                 　﻿"


def js_trim(t):
    return t.strip(SPACES)


def read_number(t):
    """docs/adr/0180's number: a sign, digits and one decimal point or comma."""
    t = js_trim(t)
    body = t[1:] if t[:1] in "+-" else t
    if not body or any(c not in "0123456789.," for c in body):
        return None
    if sum(c.isdigit() for c in body) == 0 or sum(c in ".," for c in body) > 1:
        return None
    return float(t.lstrip("+").replace(",", "."))


def parse_angle(text):
    """Degrees, minutes and seconds as the rule says (crs.rs's doc, read here on its own)."""
    t = text.strip()
    negative_letter = False
    if t and t[-1] in "KGDBNSEWkgdbnsew":
        negative_letter = t[-1].upper() in "GBSW"
        t = t[:-1]
    body = t.strip()
    sign = 1.0
    if body.startswith("-"):
        body, sign = body[1:], -1.0
    elif body.startswith("+"):
        body = body[1:]
    parts, cur = [], ""
    for c in body:
        if c.isspace() or c in "°′″'\"":
            if cur:
                parts.append(cur)
            cur = ""
        else:
            cur += c
    if cur:
        parts.append(cur)
    if not parts or len(parts) > 3:
        return None
    values = []
    for p in parts:
        if not p or not all(c.isdigit() or c == "." for c in p) or p.count(".") > 1 or p.startswith(".") or p.endswith("."):
            return None
        values.append(float(p))
    if any(v != int(v) for v in values[:-1]):
        return None
    if any(v >= 60 for v in values[1:]):
        return None
    deg = values[0] + (values[1] / 60 if len(values) > 1 else 0) + (values[2] / 3600 if len(values) > 2 else 0)
    return (-sign if negative_letter else sign) * deg


def point_row(cells, geographic, move):
    """(x, y, z) or a fault (name, column)."""

    def coordinate(k):
        t = cells[k]
        if not js_trim(t):
            return ("empty", k)
        v = read_number(t) if not geographic else (read_number(t) if read_number(t) is not None else parse_angle(t))
        if v is None:
            return ("number", k)
        if geographic and not abs(v) <= (180 if k == 0 else 90):
            return ("range", k)
        return v

    e = coordinate(0)
    if isinstance(e, tuple):
        return e
    n = coordinate(1)
    if isinstance(n, tuple):
        return n
    z = None
    if js_trim(cells[2]):
        z = read_number(cells[2])
        if z is None:
            return ("number", 2)
    if move:
        e, n = WGS84_TO_UTM36.transform(e, n)
    return (e, n, z)


def reason(fault, names, cells):
    why, k = fault
    if why == "empty":
        return f"“{names[k]}” boş"
    if why == "number":
        return f"“{names[k]}” sayı değil: {js_trim(cells[k])}"
    return f"“{names[k]}” coğrafi sınırın dışında (boylam ±180, enlem ±90): {js_trim(cells[k])}"


def source(file=None, objects=None):
    """The header and rows as İşlemler reads them (a file within its limits, or the objects' attributes)."""
    if objects is not None:
        header = sorted({k for e in objects for k in e["attrs"]})
        rows = [[e["attrs"].get(h, "") for h in header] for e in objects]
        return header, rows, [f"#{e['id']}" for e in objects]
    read = tf.read(FILES[file], tf.PROCESSING["rows"], tf.PROCESSING["columns"])
    first, *rows = read["sheets"][0]["rows"]
    header = [c.strip() for c in first]
    return header, rows, [str(i + 2) for i in range(len(rows))]


def run(values, file=None, objects=None):
    header, rows, labels = source(file, objects)
    cols = [values["east"], values["north"], values.get("elevation", "")]
    name_col = values.get("name", "")
    for c in cols + [name_col]:
        if c and c not in header:
            names = [h for h in header if h]
            return {"status": "error", "message": f"Kaynakta “{c}” alanı yok; alanları: {', '.join(names)}."}
    at = [header.index(c) if c else -1 for c in cols]
    geographic = values["system"] == "4326"
    move = values["system"] == "4326"
    layer = new_id(LAYER, set())
    added, report = [], []
    for i, r in enumerate(rows):
        cells = [(r[k] if 0 <= k < len(r) else "") for k in at]
        got = point_row(cells, geographic, move)
        if len(got) == 2:
            report.append([labels[i], reason(got, cols, cells)])
            continue
        attrs = {h: (r[k] if k < len(r) else "") for k, h in enumerate(header) if k < len(r) and r[k].strip()}
        if name_col:
            nk = header.index(name_col)
            if nk < len(r) and r[nk].strip():
                attrs["Ad"] = r[nk]
        o = {"kind": "point", "layerId": layer, "attrs": attrs, "p": {"x": got[0], "y": got[1]}}
        if got[2] is not None:
            o["z"] = got[2]
        added.append(o)
    expect = {"status": "ok", "summary": f"{len(rows)} satırdan {len(added)} nokta yazıldı.", "undo": UNDO if added else None}
    if added:
        expect["layers"] = [{"id": layer, "name": LAYER, "style": {**style(COLOR), "fill": f"{COLOR}26"}}]
        expect["added"] = added
    outputs = {"count": len(added)}
    if report:
        expect["log"] = [{"level": "warn", "text": f"{len(report)} satır atlandı; nedenleri tabloda."}]
        outputs["table"] = {"columns": ["Satır", "Neden"], "rows": report}
    expect["outputs"] = outputs
    return expect


def case(cid, title, values, expect, selection=None):
    c = {"id": cid, "title": title, "document": "table-points.kcad", "run": {"tool": TOOL}, "values": values, "expect": expect}
    if selection is not None:
        c["selection"] = selection
    return c


def cases():
    plane = {"sourceKind": "file", "file": "nokta-duzlem.csv", "east": "Y", "north": "X", "elevation": "Z", "name": "Ad", "system": "project"}
    wgs = {"sourceKind": "file", "file": "nokta-wgs84.ods", "east": "Boylam", "north": "Enlem", "elevation": "Yükseklik", "name": "Nokta", "system": "4326"}
    layer = {"sourceKind": "layer", "source": {"scope": "layer", "layerId": "olcum"}, "east": "Boylam", "north": "Enlem", "name": "Nokta", "system": "4326"}
    missing = {**plane, "elevation": "Kot"}
    return [
        case("plane-csv", "Tablodan nokta: projenin sistemindeki CSV; boş, sayı olmayan ve kotu sayı olmayan satırlar atlanır",
             plane, run(plane, file="nokta-duzlem.csv")),
        case("wgs84-ods", "Tablodan nokta: ODS'deki boylam ve enlemler (ondalık, virgüllü, derece dakika saniye, yarım küre) WGS 84'ten UTM 36N'ye",
             wgs, run(wgs, file="nokta-wgs84.ods")),
        case("wgs84-layer", "Tablodan nokta: Ölçüm katmanının nesnelerinin öznitelikleri satırdır",
             layer, run(layer, objects=ENTITIES)),
        case("missing-column", "Tablodan nokta: kaynakta olmayan sütun çalıştırmayı reddeder",
             missing, run(missing, file="nokta-duzlem.csv")),
    ]


def text_of(v):
    return json.dumps(v, ensure_ascii=False, indent=1) + "\n"


def outputs():
    doc = {"format": "kentos.processing-cases", "version": 1, "tolerance": TOL, "toleranceFields": ["z"], "documents": {},
           "files": {name: name for name in FILES}, "cases": cases()}
    return [(PROC / "table-points.json", text_of(doc).encode("utf-8")), (PROC / "table-points.kcad", text_of(DOCUMENT).encode("utf-8"))] + [
        (PROC / name, data) for name, data in FILES.items()
    ]


def main():
    outs = outputs()
    if "--check" in sys.argv[1:]:
        bad = [p for p, data in outs if not p.exists() or p.read_bytes() != data]
        for p in bad:
            print(f"{p.relative_to(ROOT)}: diskteki dosya kurallardan üretilenle aynı değil", file=sys.stderr)
        if bad:
            return 1
        print("Tablodan nokta'nın durumları tutarlı: fixtures/processing/v1/table-points*")
        return 0
    for p, data in outs:
        p.write_bytes(data)
    print(f"written: {', '.join(str(p.relative_to(ROOT)) for p, _ in outs)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
