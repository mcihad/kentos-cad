#!/usr/bin/env python3
"""Independent reference of Veri aktar (docs/adr/0216), written from the ADR alone, no KentOS code.

Writes fixtures/processing/v1/transfer.kcad (the drawing), tapu.csv and noktalar.csv (the files) and
fixtures/processing/v1/transfer.json (the tool run on them, in the processing cases' format,
fixtures/processing/README.md).

The rules (ADR 0216 §2–§4):

1. Keys: Anahtarla birleştir's (ADR 0200 §6): trimmed; two numbers as numbers (`002` is `2`), otherwise texts exactly;
   a key the source has more than once takes its first row (scripts/fixtures/spatial_query_cases.py's `join_plan`).
2. The field map: rows of a target field, a source column (or none) and a default. An empty map is every source column
   but the key, under its own name. A row's value is its column's cell when that is not blank, else the default when
   that is not blank, else none.
3. A matched target: for each row in the map's order, Yalnız boşlara leaves a field with a value; a value goes through
   the target layer's field of that name (scripts/fixtures/layer_field_cases.py's `check`: its canonical text, or why
   not: the value is not written and the reason is a note); none clears the field (a required field is not cleared:
   its reason is a note). With Geometriyi güncelle, the source object's geometry when the kinds are the same (else a
   note).
4. With Eşleşmeyenleri ekle, a first source row no target took becomes an object on the target layer: the key under
   the target key's name, the map's values, then the layer fields' defaults; a layer source gives its object's
   geometry, a file its east and north columns' point (read by the decimal rule; unread: not added), or, with neither
   column chosen, a record without a geometry (docs/adr/0410 §6; one column alone is refused); a required field left
   empty: not added, with its reason.
5. The report: the targets in order (Güncellendi, Değişmedi, Eşleşmedi), then the source rows no target took in order
   (Eklendi, Eklenmedi, Kullanılmadı); a change is “name: before → after”, (boş) for none; notes joined by spaces.
6. The target layer (ADR 0216 §3): an empty one is taken (a run may only add to it); a group or a locked layer is not
   (what a run adds goes to a layer, and a locked one would leave it out): the run does not start.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from layer_field_cases import check, parse_decimal, trim  # noqa: E402
from spatial_query_cases import join_key, join_plan  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 487000, 4420000


def P(x, y):
    return {"x": E + x, "y": N + y}


def rect(x0, y0, x1, y1):
    return [P(x0, y0), P(x1, y0), P(x1, y1), P(x0, y1)]


def layer(i, name, color, fields=None):
    node = {"id": i, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": {"color": color, "lineType": "continuous", "lineWeight": 0.25}, "children": []}
    if fields:
        node["fields"] = fields
    return node


PARCEL_FIELDS = [
    {"name": "Parsel", "kind": "integer", "required": True},
    {"name": "Ada", "kind": "integer", "required": True},
    {"name": "Malik", "kind": "text", "length": 40},
    {"name": "Hisse", "kind": "text"},
    {"name": "Alan", "kind": "decimal", "scale": 2},
    {"name": "Tescil", "kind": "date"},
    {"name": "Durum", "kind": "text", "default": "Beyanlı"},
    {"name": "Mahalle", "kind": "text", "required": True, "default": "Merkez"},
]

LAYERS = [
    layer("cizim", "Çizim", "fg"),
    layer("parsel", "Parsel", "#E5484D", PARCEL_FIELDS),
    layer("olcum", "Yeni ölçüm", "#3E63DD"),
    layer("nokta", "Nokta", "#30A46C"),
    layer("yeni", "Yeni parseller", "#F76B15"),
    {**layer("arsiv", "Arşiv", "#8E8C99"), "locked": True},
    {**layer("paftalar", "Paftalar", "fg"), "type": "group", "children": [layer("pafta1", "Pafta 1", "#E5484D")]},
]


def ent(i, kind, lay, attrs, **geom):
    e = {"kind": kind, "id": i, "layerId": lay, "attrs": attrs}
    e.update(geom)
    return e


ENTITIES = [
    # The parcels: 1 owned, 2 without an owner, 3 owned with an area, 4 bare, 5 with no source row, 6 without a key.
    ent(1, "polygon", "parsel", {"Parsel": "1", "Ada": "101", "Malik": "Ali Veli", "Hisse": "1", "Mahalle": "Merkez"}, pts=rect(0, 0, 20, 20)),
    ent(2, "polygon", "parsel", {"Parsel": "2", "Ada": "101", "Malik": "", "Mahalle": "Merkez"}, pts=rect(20, 0, 40, 20)),
    ent(3, "polygon", "parsel", {"Parsel": "3", "Ada": "101", "Malik": "Ayşe Kaya", "Hisse": "1/2", "Alan": "400.00", "Mahalle": "Yeni"}, pts=rect(40, 0, 60, 20)),
    ent(4, "polygon", "parsel", {"Parsel": "4", "Ada": "101", "Mahalle": "Merkez"}, pts=rect(60, 0, 80, 20)),
    ent(5, "polygon", "parsel", {"Parsel": "5", "Ada": "101", "Malik": "Veli Can", "Mahalle": "Merkez"}, pts=rect(80, 0, 100, 20)),
    ent(6, "polygon", "parsel", {"Ada": "101", "Mahalle": "Merkez"}, pts=rect(100, 0, 120, 20)),
    # The new survey: parcel 2 measured again, 3 as it was, 4 drawn as a line, 7 new, 8 new without its block, 2 twice.
    ent(11, "polygon", "olcum", {"Parsel": "2", "Ada": "101", "Malik": "Fatma Ak", "Alan": "410.5"}, pts=[P(20, 0), P(40.5, 0), P(40.5, 20), P(20, 20)]),
    ent(12, "polygon", "olcum", {"Parsel": "3", "Ada": "101", "Malik": "Ayşe Kaya", "Alan": "400"}, pts=rect(40, 0, 60, 20)),
    ent(13, "polyline", "olcum", {"Parsel": "4", "Ada": "101", "Malik": "Can Er"}, pts=[P(60, 0), P(80, 0), P(80, 20)]),
    ent(14, "polygon", "olcum", {"Parsel": "7", "Ada": "102", "Malik": "Zeynep Tan"}, pts=rect(0, 30, 20, 50)),
    ent(15, "polygon", "olcum", {"Parsel": "8", "Malik": "Ada yok"}, pts=rect(20, 30, 40, 50)),
    ent(16, "polygon", "olcum", {"Parsel": "002", "Ada": "101", "Malik": "Yinelenen"}, pts=rect(20, 60, 40, 80)),
    # Points by name.
    ent(21, "point", "nokta", {"Ad": "N1", "Kod": "eski"}, p=P(5, 100)),
    ent(22, "point", "nokta", {"Ad": "N2"}, p=P(15, 100)),
]

DOCUMENT = {
    "format": "kentos.document",
    "version": 1,
    "name": "İşlem durumları: veri aktarma",
    "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                 "workspace": "gis", "drawingFont": "barlow"},
    "origin": {"x": E, "y": N},
    "layers": LAYERS,
    "activeLayer": "cizim",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}

# The deed list: separated by semicolons, decimal commas, dates as typed.
TAPU = [
    ["PARSEL", "ADA", "MALIK_ADI", "HISSE", "YUZOLCUM", "TESCIL_TARIHI"],
    ["1", "101", "Ali Veli", "1", "412,25", "03.05.2021"],
    ["002", "101", "Fatma Ak", "1/2", "410,5", "2020-01-15"],
    ["3", "101", "", "1/2", "12,5x", ""],
    ["4", "101", "Can Er", "1", "399,999", "31.02.2020"],
    ["9", "102", "Zeynep Tan", "1", "", ""],
    ["1", "101", "Tekrar", "", "", ""],
    ["", "103", "Anahtarsız", "", "", ""],
]

# Points: a name, east (Y) and north (X) as surveyors write them, a code.
NOKTALAR = [
    ["NOKTA", "Y", "X", "KOD"],
    ["N1", f"{E + 5}", f"{N + 100}", "sınır"],
    ["N3", f"{E + 25},50", f"{N + 100},25", "poligon"],
    ["N4", "abc", f"{N + 100}", "bina"],
]

FILES = {"tapu.csv": TAPU, "noktalar.csv": NOKTALAR}

BASE_KEYS = {"kind", "id", "layerId", "attrs", "label", "color", "symbol", "lineWeight"}


def objects(lay):
    return [e for e in ENTITIES if e["layerId"] == lay]


def nodes(tree):
    for n in tree:
        yield n
        yield from nodes(n.get("children", []))


def fields_of(lay):
    return next((n.get("fields", []) for n in nodes(LAYERS) if n["id"] == lay), [])


TARGET_LABEL = "“Hedef katman”"


def target_problem(lay):
    """Why the target layer is not one a run adds to (ADR 0216 §3), as the dialog's check says it; none when it is."""
    node = next(n for n in nodes(LAYERS) if n["id"] == lay)
    if node["type"] == "group":
        return f"{TARGET_LABEL}: grup değil, bir katman seçin; eklenen nesneler bu katmana yazılır."
    if node["locked"]:
        return f"{TARGET_LABEL}: katman kilitli. Kilidi Katmanlar panelinden açın ya da başka katman seçin."
    return None


def field(lay, name):
    return next((f for f in fields_of(lay) if f["name"] == name), None)


def geometry(e):
    return {k: v for k, v in e.items() if k not in BASE_KEYS}


def source_table(kind, source):
    """The header and rows a source gives, and the objects behind the rows (a layer's)."""
    if kind == "file":
        table = FILES[source]
        return table[0], table[1:], None
    objs = objects(source)
    header = sorted({k for o in objs for k in o["attrs"]})
    return header, [[o["attrs"].get(h, "") for h in header] for o in objs], objs


def shown(v):
    return v if v not in (None, "") else "(boş)"


def run(values):
    """The tool on the drawing: the report, the changes and the log, as the ADR says."""
    target_layer = values["target"]["layerId"]
    targets = objects(target_layer)
    kind = values["sourceKind"]
    header, rows, objs = source_table(kind, values["file"] if kind == "file" else values["source"]["layerId"])
    target_key = values["targetKey"]
    key = trim(values.get("sourceKey") or "") or target_key
    if key not in header:
        return {"refused": f"Kaynakta “{key}” alanı yok; alanları: {', '.join(h for h in header if h)}."}
    k = header.index(key)
    given = values.get("map") or [{"target": h, "source": h, "default": ""} for i, h in enumerate(header) if i != k and h and header.index(h) == i]
    # Names are read trimmed; a default as typed.
    mapping = [{"target": trim(m["target"]), "source": trim(m["source"]), "default": m["default"]} for m in given]
    for m in mapping:
        if m["source"] and m["source"] not in header:
            return {"refused": f"Kaynakta “{m['source']}” alanı yok; alanları: {', '.join(h for h in header if h)}."}
    append = values.get("append", False)
    east, north = trim(values.get("east", "")), trim(values.get("north", ""))
    records = append and kind == "file" and not east and not north
    if append and kind == "file" and not records:
        if not east or not north:
            return {"refused": "Doğu ve Kuzey sütunlarının ikisini de seçin; ikisi de boşsa satırlar geometrisiz kayıt olarak eklenir."}
        for c in (east, north):
            if c not in header:
                return {"refused": f"Kaynakta “{c}” alanı yok; alanları: {', '.join(h for h in header if h)}."}

    def cell(r, name):
        row = rows[r]
        i = header.index(name)
        return row[i] if i < len(row) else ""

    def value(r, m):
        if m["source"]:
            c = cell(r, m["source"])
            if trim(c) != "":
                return c
        return m["default"] if trim(m["default"]) != "" else None

    plan = join_plan([t["attrs"].get(target_key) for t in targets], [row[k] if k < len(row) else "" for row in rows])
    existing = values.get("existing", "overwrite")
    report, update, add, log = [], [], [], []
    misfits = 0
    for n, t in enumerate(targets):
        key_text = trim(t["attrs"].get(target_key, ""))
        r = plan["matches"][n]
        if r is None:
            report.append([key_text, "Eşleşmedi", "", "" if key_text else "Anahtarı boş."])
            continue
        attrs = dict(t["attrs"])
        changes, notes = [], []
        for m in mapping:
            name = m["target"]
            old = attrs.get(name)
            if existing == "empty" and old is not None and trim(old) != "":
                continue
            text = value(r, m)
            f = field(target_layer, name)
            if text is None:
                if old is None:
                    continue
                if f is not None and "error" in check(f, ""):
                    notes.append(check(f, "")["message"])
                    misfits += 1
                    continue
                del attrs[name]
                changes.append(f"{name}: {shown(old)} → (boş)")
                continue
            if f is not None:
                res = check(f, text)
                if "error" in res:
                    notes.append(res["message"])
                    misfits += 1
                    continue
                text = res["value"]
            if old == text:
                continue
            attrs[name] = text
            changes.append(f"{name}: {shown(old)} → {text}")
        patch = {}
        if values.get("geometry") and objs is not None:
            src = objs[r]
            if src["kind"] != t["kind"]:
                notes.append("Geometrinin türü farklı; geometri güncellenmedi.")
            elif geometry(src) != geometry(t):
                patch = geometry(src)
                changes.append("geometri")
        if attrs != t["attrs"] or patch:
            update.append({"id": t["id"], "attrs": attrs, **({"geometry": patch} if patch else {})})
        report.append([key_text, "Güncellendi" if changes else "Değişmedi", "; ".join(changes), " ".join(notes)])
    # The source rows no target took, in order.
    used = {m for m in plan["matches"] if m is not None}
    firsts = {}
    for r, row in enumerate(rows):
        kt = row[k] if k < len(row) else ""
        jk = join_key(kt)
        if jk is not None and jk not in firsts:
            firsts[jk] = r
    added = not_added = unused = 0
    for r, row in enumerate(rows):
        if r in used:
            continue
        kt = trim(row[k] if k < len(row) else "")
        jk = join_key(kt) if kt else None
        if jk is None:
            report.append(["", "Kullanılmadı", "", "Anahtarı boş."])
            unused += 1
            continue
        if firsts[jk] != r:
            report.append([kt, "Kullanılmadı", "", "Anahtar kaynakta yineleniyor; ilk satır alındı."])
            unused += 1
            continue
        if not append:
            report.append([kt, "Kullanılmadı", "", ""])
            unused += 1
            continue
        # A new object, in this order: the key, the map's values, the layer fields' defaults, the required fields, the
        # geometry; a key the field does not take, a required field left empty or an unread point leaves it out.
        attrs, written, notes = {}, [], []
        ok = True
        f = field(target_layer, target_key)
        res = check(f, kt) if f is not None else {"value": kt}
        if "error" in res:
            notes.append(res["message"])
            ok = False
        else:
            attrs[target_key] = res["value"]
            written.append(f"{target_key}: {res['value']}")
        for m in mapping:
            name = m["target"]
            if name == target_key:
                continue
            text = value(r, m)
            if text is None:
                continue
            f = field(target_layer, name)
            if f is not None:
                res = check(f, text)
                if "error" in res:
                    notes.append(res["message"])
                    misfits += 1
                    continue
                text = res["value"]
            attrs[name] = text
            written.append(f"{name}: {text}")
        for f in fields_of(target_layer):
            if f["name"] in attrs or trim(f.get("default", "")) == "":
                continue
            res = check(f, f["default"])
            if "value" in res and res["value"] != "":
                attrs[f["name"]] = res["value"]
                written.append(f"{f['name']}: {res['value']}")
        for f in fields_of(target_layer):
            if f.get("required") and f["name"] != target_key and trim(attrs.get(f["name"], "")) == "":
                notes.append(check(f, "")["message"])
                ok = False
        geom = None
        if objs is not None:
            geom = geometry(objs[r])
        elif records:
            geom = {}
        else:
            ex, nx = parse_decimal(trim(cell(r, east))), parse_decimal(trim(cell(r, north)))
            if ex is None or nx is None:
                notes.append("Doğu ya da kuzey okunamadı; nokta eklenmedi.")
                ok = False
            else:
                geom = {"p": {"x": float(ex[0]), "y": float(nx[0])}}
        if not ok:
            report.append([kt, "Eklenmedi", "", " ".join(notes)])
            not_added += 1
            continue
        kind_of = objs[r]["kind"] if objs is not None else ("record" if records else "point")
        add.append({"kind": kind_of, "layerId": target_layer, "attrs": attrs, **geom})
        report.append([kt, "Eklendi", "; ".join(written), " ".join(notes)])
        added += 1
    updated = sum(1 for row in report if row[1] == "Güncellendi")
    unmatched = sum(1 for row in report if row[1] == "Eşleşmedi")
    if plan["repeated"]:
        log.append({"level": "warn", "text": f"{plan['repeated']} anahtar kaynakta birden çok kez var; ilk satırları alındı."})
    if misfits:
        log.append({"level": "warn", "text": f"{misfits} değer alanına uymadığı için yazılmadı; nedenleri raporda."})
    if not_added:
        log.append({"level": "warn", "text": f"{not_added} kaynak satırı eklenemedi; nedenleri raporda."})
    summary = f"{updated} nesne güncellendi, {added} eklendi; {unmatched} hedefin eşi yok, {unused + not_added} kaynak satırı kullanılmadı."
    return {"report": report, "update": update, "add": add, "log": log, "summary": summary, "updated": updated, "added": added}


COLUMNS = ["Anahtar", "Sonuç", "Değişiklikler", "Not"]


def case(cid, title, values):
    problem = target_problem(values["target"]["layerId"])
    if problem:
        return {"id": cid, "title": title, "document": "transfer.kcad", "run": {"tool": "attributes.transfer"}, "values": values,
                "expect": {"status": "invalid", "issues": [{"param": "target", "message": problem}]}}
    out = run(values)
    if "refused" in out:
        return {"id": cid, "title": title, "document": "transfer.kcad", "run": {"tool": "attributes.transfer"}, "values": values,
                "expect": {"status": "error", "message": out["refused"]}}
    report_only = values.get("reportOnly", False)
    expect = {"status": "ok",
              "summary": f"Yalnız rapor, çizim değişmedi: {out['summary']}" if report_only else out["summary"],
              "undo": None if report_only or not (out["update"] or out["add"]) else "Veri aktar"}
    if out["log"]:
        expect["log"] = out["log"]
    if not report_only:
        if out["update"]:
            expect["updated"] = [{"id": u["id"], "attrs": u["attrs"], **({"geometry": u["geometry"]} if "geometry" in u else {})} for u in out["update"]]
        if out["add"]:
            expect["added"] = out["add"]
    expect["outputs"] = {"count": 0 if report_only else out["updated"], "added": 0 if report_only else out["added"],
                         "table": {"columns": COLUMNS, "rows": out["report"]}}
    return {"id": cid, "title": title, "document": "transfer.kcad", "run": {"tool": "attributes.transfer"}, "values": values, "expect": expect}


def L(i):
    return {"scope": "layer", "layerId": i}


DEED_MAP = [
    {"target": "Malik", "source": "MALIK_ADI", "default": ""},
    {"target": "Hisse", "source": "HISSE", "default": ""},
    {"target": "Alan", "source": "YUZOLCUM", "default": ""},
    {"target": "Tescil", "source": "TESCIL_TARIHI", "default": ""},
    {"target": "Durum", "source": "", "default": "Tescilli"},
]


def build():
    deed = {"target": L("parsel"), "targetKey": "Parsel", "sourceKind": "file", "file": "tapu.csv", "sourceKey": "PARSEL",
            "map": DEED_MAP, "existing": "overwrite", "geometry": False, "append": False, "east": "", "north": "", "reportOnly": False}
    survey = {"target": L("parsel"), "targetKey": "Parsel", "sourceKind": "layer", "source": L("olcum"), "sourceKey": "",
              "map": [], "existing": "overwrite", "geometry": True, "append": True, "east": "", "north": "", "reportOnly": False}
    points = {"target": L("nokta"), "targetKey": "Ad", "sourceKind": "file", "file": "noktalar.csv", "sourceKey": "NOKTA",
              "map": [{"target": "Kod", "source": "KOD", "default": ""}], "existing": "overwrite", "geometry": False, "append": True,
              "east": "Y", "north": "X", "reportOnly": False}
    cases = [
        case("deed-mapped", "Tapu listesinden alan eşlemesiyle: tipli alanlar (ondalık, tarih), varsayılan, uymayan değerler, yinelenen ve boş anahtar", deed),
        case("deed-empty-only", "Aynı liste, Yalnız boşlara: dolu alanlar kalır", {**deed, "existing": "empty"}),
        case("survey-geometry-append", "Yeni ölçüm katmanından: aynı adlarla, geometri güncellenir (türü farklı olan söylenir), eşleşmeyenler eklenir (zorunlu alanı boş olan eklenmez)", survey),
        case("survey-report-only", "Aynı aktarım, Yalnız raporla: tablo aynı, çizim değişmez", {**survey, "reportOnly": True}),
        case("points-append", "Nokta dosyasından: var olan noktanın kodu güncellenir, yenisi doğu ve kuzeyinden nokta olur, okunamayan koordinat eklenmez", points),
        case("missing-key", "Kaynak anahtar sütunu yok: reddedilir", {**deed, "sourceKey": "ADA_PARSEL"}),
        case("records-append", "Dosyadan doğu ve kuzey seçilmeden: yeni satırlar geometrisiz kayıt olarak eklenir", {**points, "east": "", "north": ""}),
        case("half-coordinates", "Yalnız doğu sütunu seçildi: reddedilir", {**points, "north": ""}),
        case("empty-target", "Boş hedef katmana: yalnız eklenir (yinelenen anahtar kullanılmaz)", {**survey, "target": L("yeni"), "geometry": False}),
        case("locked-target", "Kilitli hedef katman: çalıştırma başlamaz", {**deed, "target": L("arsiv")}),
        case("group-target", "Hedef bir grup: çalıştırma başlamaz", {**deed, "target": L("paftalar")}),
    ]
    return {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-9, "documents": {},
            "files": {"tapu.csv": "tapu.csv", "noktalar.csv": "noktalar.csv"}, "cases": cases}


def csv_text(table):
    return "".join(";".join(row) + "\n" for row in table)


def main():
    proc = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    doc = json.dumps(DOCUMENT, ensure_ascii=False, indent=1) + "\n"
    outs = [(PROC_DIR / "transfer.json", proc), (PROC_DIR / "transfer.kcad", doc),
            (PROC_DIR / "tapu.csv", csv_text(TAPU)), (PROC_DIR / "noktalar.csv", csv_text(NOKTALAR))]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text() != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("veri aktarma durumları tutarlı: fixtures/processing/v1/transfer.*")
        return 0
    for p, t in outs:
        p.write_text(t)
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
