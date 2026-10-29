#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.addresses.compare.json: the shared cases of `cad.addresses.compare` v1
(docs/adr/0736), their expectations built here without KentOS code, and the register files they read
(fixtures/geocode/v1/register/). The locator is built by the address reference's own builder
(scripts/fixtures/geocode_cases.py, `build`) from the drawing as address_check_command_cases.py reads it; the
comparison's rule is the ADR's:

- the register: a table file whose first row names the columns (written here, so its rows are known) or a layer of
  the drawing (its objects in the drawing's order, their attributes the columns, a point its position);
- a row with no code is left out (counted); a number is an optional sign, digits with one decimal point (a lone comma
  is one) and an optional exponent;
- a door whose code no row has; a door whose code is a row's (its first) but whose street (the name, and the kind
  where both say one), door number, neighbourhood or district the row writes otherwise, by the locator's keys; a
  neighbourhood or district compared only when the address data's doors have that field; a door farther from its
  row's position than the tolerance (metres, sqrt of the squares, divided by the unit per metre);
- the register's addresses no door has: rows whose code is on no door, with a street and a door number that read,
  grouped by (street, door, neighbourhood, district) keys, an address drawn when a door has its street and number and
  agrees on the neighbourhood and district (the same, or one of them not said);
- a code on more than one row;
- the door lists by the drawing's order, the register's by their first rows; each list's count, then its first `limit`.

The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/address_compare_cases.py           # writes the files
    python3 scripts/fixtures/address_compare_cases.py --check   # compares them with the ones on disk
"""
import copy
import json
import math
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import address_check_command_cases as ac  # noqa: E402  (the drawing's reading: no KentOS code)
import geocode_cases as ref  # noqa: E402  (the address reference: no KentOS code)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.addresses.compare.json"
REGISTER = ROOT / "fixtures/geocode/v1/register"
FIX = "$fixture:geocode/v1/register/"
X0, Y0 = ac.X0, ac.Y0

LOCATOR = {
    "id": "adres-1",
    "name": "Numarataj",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE",
              "code": "UAVT"},
    "minScore": 85,
    "reach": 100.0,
}
# Doors with no neighbourhood or district field.
PLAIN = {
    "id": "adres-2",
    "name": "Yalnız yol ve kapı",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "code": "UAVT"},
    "minScore": 85,
    "reach": 100.0,
}
# Doors with no code field.
UNCODED = {
    "id": "adres-3",
    "name": "Kodsuz",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO"},
    "minScore": 85,
    "reach": 100.0,
}


def door(id_, street, no, code="", hood="Kızılay", district="Çankaya", x=0.0):
    return ac.door(id_, "kapi", street, no, hood=hood, district=district, code=code, x=x)


ENTITIES = [
    door(1, "Atatürk Bulvarı", "12", "1000000001", x=10),
    door(2, "Atatürk Bulvarı", "14", "1000000002", x=20),
    door(3, "Gazi Mustafa Kemal Cd.", "3", "1000000003", x=30),
    door(4, "Esat Cd.", "2", "1000000004", x=40),
    door(5, "Esat Cd.", "4", "1000000099", x=50),
    door(6, "Esat Cd.", "6", "", x=60),
    door(7, "Tunalı Hilmi Cd.", "10", "1000000007", x=70),
    door(8, "Tunalı Hilmi Cd.", "12", " 1000000008 ", x=80),
    door(9, "Tunalı Hilmi Cd.", "14", "1000000009", district="ANKARA ÇANKAYA", x=90),
    {"kind": "line", "id": 10, "layerId": "kapi", "attrs": {}, "a": {"x": X0, "y": Y0}, "b": {"x": X0 + 5, "y": Y0}},
    door(11, "", "9", "1000000098", x=95),
]

COLUMNS = ["ADRES_NO", "YOL", "DIS_KAPI", "MAHALLE", "ILCE", "X", "Y"]
ROWS = [
    ["1000000001", "ATATÜRK BLV.", "12", "Kızılay Mahallesi", "Çankaya", "500012", "4400010"],
    ["1000000002", "Atatürk Bulvarı", "16", "Kızılay", "Çankaya", "500020", "4400010"],
    ["1000000003", "Gazi Mustafa Kemal Bulvarı", "3", "Kızılay", "Çankaya", "", ""],
    ["1000000004", "Esat Caddesi", "2", "Bahçelievler", "Çankaya", "500040", "4400010"],
    ["1000000007", "Tunalı Hilmi Caddesi", "10", "Kızılay", "Çankaya", "500070", "4400035"],
    ["1000000008", "Tunalı Hilmi Cd.", "12", "Kızılay", "Çankaya", "500080,0", "4400010,5"],
    ["1000000009", "Tunalı Hilmi Cd.", "14", "Kızılay", "Çankaya", "500090", "4400010"],
    ["1000000010", "Kuğulu Sk.", "1", "Kızılay", "Çankaya", "500100", "4400060"],
    ["1000000011", "KUĞULU SOKAK", "1", "Kızılay", "Çankaya", "500100", "4400060"],
    ["1000000012", "Atatürk Bulvarı", "12", "Kızılay", "Çankaya", "500010", "4400010"],
    ["1000000001", "Atatürk Bulvarı", "12", "Kızılay", "Çankaya", "", ""],
    ["", "Esat Cd.", "8", "Kızılay", "Çankaya", "", ""],
    ["1000000013", "Esat Caddesi", "6", "Kızılay", "Çankaya", "", ""],
    ["1000000014", "Ziya Gökalp Cd.", "", "Kızılay", "Çankaya", "1e3", "x"],
]
# The register as a layer of the drawing: a point per row, where the row says, else on the door its code names.
REGISTER_LAYER = "kayit"


def files():
    """The register files: name → bytes."""
    def text(rows, sep, bom=False):
        body = "\n".join(sep.join(r) for r in [COLUMNS] + rows) + "\n"
        return (b"\xef\xbb\xbf" if bom else b"") + body.encode("utf-8")
    return {
        "uavt.csv": text(ROWS, ";", bom=True),
        "uavt.txt": text(ROWS, "\t"),
        "bos.csv": b"",
        "eski.xls": b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1" + bytes(56),
    }


def number(text):
    """A register's number (the ADR's grammar), or None."""
    t = text.strip()
    if t.count(",") == 1 and "." not in t:
        t = t.replace(",", ".")
    if not re.fullmatch(r"[+-]?(\d+(\.\d*)?|\.\d+)([eE][+-]?\d+)?", t):
        return None
    v = float(t)
    return v if math.isfinite(v) else None


def setup_with(entities=None, locators=None):
    s = {
        "format": "kentos.document",
        "version": 1,
        "name": "Adres kaydı",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow",
                     "addressLocators": copy.deepcopy(locators if locators is not None else [LOCATOR, PLAIN, UNCODED])},
        "origin": {"x": X0, "y": Y0},
        "layers": [ac.node("kapi", "Kapı"), ac.node(REGISTER_LAYER, "Kayıt"), ac.node("0", "0")],
        "activeLayer": "0",
        "entities": copy.deepcopy(entities if entities is not None else ENTITIES),
        "styles": {"items": [], "categories": []},
    }
    return s


def layer_rows(first_id=100, positions=True):
    """The register's rows as points of the register layer: their ids from `first_id`, each where its row says or on
    the door its code names (else at the origin)."""
    doors_at = {e["attrs"].get("UAVT", "").strip(): (e["p"]["x"], e["p"]["y"]) for e in ENTITIES
                if e["kind"] == "point" and e["attrs"].get("UAVT", "").strip()}
    out = []
    for i, r in enumerate(ROWS):
        attrs = {c: v for c, v in zip(COLUMNS[:5], r[:5]) if v != ""}
        x, y = number(r[5]) if r[5] else None, number(r[6]) if r[6] else None
        if x is not None and y is not None and positions:
            at = (x, y)
        else:
            at = doors_at.get(r[0], (X0, Y0))
        out.append({"kind": "point", "id": first_id + i, "layerId": REGISTER_LAYER, "attrs": attrs,
                    "p": {"x": at[0], "y": at[1]}})
    return out


# ── The comparison, from the ADR ─────────────────────────────────────────────

def register_rows(table, columns, fields, layer_points=None):
    """The rows read: (row, code, street, door, hood, district, at) for each coded row, and the uncoded count."""
    header = table[0]
    idx = {k: header.index(v) for k, v in columns.items()}
    rows, uncoded = [], 0
    for i, r in enumerate(table[1:]):
        def cell(k):
            if k not in idx or k not in fields:
                return None
            j = idx[k]
            return r[j].strip() if j < len(r) else ""
        code = r[idx["code"]].strip() if idx["code"] < len(r) else ""
        if not code:
            uncoded += 1
            continue
        at = None
        if "x" in idx and "y" in idx:
            x = number(r[idx["x"]]) if idx["x"] < len(r) else None
            y = number(r[idx["y"]]) if idx["y"] < len(r) else None
            if x is not None and y is not None:
                at = (x, y)
        elif layer_points is not None:
            at = layer_points[i]
        rows.append({"row": i + 1, "code": code, "street": cell("street"), "door": cell("door"),
                     "neighbourhood": cell("neighbourhood"), "district": cell("district"), "at": at})
    return rows, uncoded


def compare(setup, locator, rows, tolerance):
    sc = ac.scene(setup, locator)
    loc = ref.build(sc)
    doors = sorted(loc["doors"], key=lambda d: d["order"])
    first, repeats = {}, []
    for i, r in enumerate(rows):
        if r["code"] not in first:
            first[r["code"]] = i
        else:
            rep = next((x for x in repeats if x["code"] == r["code"]), None)
            if rep is None:
                repeats.append({"code": r["code"], "rows": [rows[first[r["code"]]]["row"], r["row"]],
                                "first": first[r["code"]]})
            else:
                rep["rows"].append(r["row"])
    repeats.sort(key=lambda x: x["first"])
    out = {"doors": len(loc["doors"]), "coded": 0, "matched": 0, "unregistered": [], "differences": [], "distant": [],
           "undrawn": [], "repeats": [{"code": x["code"], "rows": x["rows"]} for x in repeats]}
    for d in doors:
        code = d["texts"]["code"]
        if not code:
            continue
        out["coded"] += 1
        if code not in first:
            out["unregistered"].append({"order": d["order"], "code": code, "street": d["texts"]["street"],
                                        "door": d["texts"]["door"]})
            continue
        out["matched"] += 1
        r = rows[first[code]]
        name, hood, district = d["group"]
        kind = ref.street_value(d["texts"]["street"])[1]
        parts = []

        def differ(part, said, same, drawing):
            if said is not None and said != "" and not same(said):
                parts.append({"part": part, "drawing": drawing, "register": said})

        def same_street(t):
            sv = ref.street_value(t)
            return sv is not None and sv[0] == name and (sv[1] is None or kind is None or sv[1] == kind)
        differ("street", r["street"], same_street, d["texts"]["street"])
        differ("door", r["door"], lambda t: ref.door_value(t) == d["key"], d["texts"]["door"])
        differ("neighbourhood", r["neighbourhood"], lambda t: ref.area_value(t) == hood, d["texts"]["neighbourhood"])
        differ("district", r["district"], lambda t: ref.words_key(t) == district, d["texts"]["district"])
        if parts:
            out["differences"].append({"order": d["order"], "code": code, "row": r["row"], "parts": parts})
        if r["at"] is not None:
            dx, dy = d["at"][0] - r["at"][0], d["at"][1] - r["at"][1]
            distance = math.sqrt(dx * dx + dy * dy) / loc["perMetre"]
            if distance > tolerance:
                out["distant"].append({"order": d["order"], "code": code, "row": r["row"], "distance": distance})
    codes_on_doors = set(loc["codes"])

    def agree(a, b):
        return a == b or a == "" or b == ""
    groups, order = {}, []
    for r in rows:
        if r["code"] in codes_on_doors or r["street"] is None or r["door"] is None:
            continue
        sv, number_ = ref.street_value(r["street"]), ref.door_value(r["door"])
        if sv is None or number_ is None:
            continue
        hood = ref.area_value(r["neighbourhood"]) if r["neighbourhood"] is not None else ""
        district = ref.words_key(r["district"]) if r["district"] is not None else ""
        drawn = any(d["group"][0] == sv[0] and d["key"] == number_ and agree(d["group"][1], hood)
                    and agree(d["group"][2], district) for d in loc["doors"])
        if drawn:
            continue
        key = (sv[0], number_, hood, district)
        if key not in groups:
            groups[key] = {"street": r["street"], "door": r["door"], "neighbourhood": r["neighbourhood"] or "",
                           "codes": [], "rows": []}
            order.append(key)
        u = groups[key]
        if r["code"] not in u["codes"]:
            u["codes"].append(r["code"])
        u["rows"].append(r["row"])
    out["undrawn"] = [groups[k] for k in order]
    return out


def answer(setup, locator, table, columns, tolerance=10.0, limit=100, layer=None):
    fields = {"street", "door"}
    doors = locator["doors"]
    if "neighbourhood" in doors:
        fields.add("neighbourhood")
    if "district" in doors:
        fields.add("district")
    points = None
    row_ids = None
    if layer is not None:
        objs = [e for e in setup["entities"] if e["layerId"] == layer]
        points = [(e["p"]["x"], e["p"]["y"]) if e["kind"] == "point" else None for e in objs]
        row_ids = [e["id"] for e in objs]
    rows, uncoded = register_rows(table, columns, fields, points)
    c = compare(setup, locator, rows, tolerance)

    def uid(order):
        return ac.uid(setup, order)

    def row_uids(rs):
        return [f"$uidOf:{row_ids[r - 1]}" for r in rs] if row_ids else None

    def with_uids(item, rs):
        u = row_uids(rs)
        if u:
            item["uids"] = u
        return item
    return {
        "locator": locator["id"],
        "doors": c["doors"],
        "coded": c["coded"],
        "matched": c["matched"],
        "rows": len(rows),
        "uncodedRows": uncoded,
        "unregistered": len(c["unregistered"]),
        "unregisteredDoors": [{"uid": uid(u["order"]), "code": u["code"], "street": u["street"], "door": u["door"]}
                              for u in c["unregistered"][:limit]],
        "differing": len(c["differences"]),
        "differences": [{"uid": uid(d["order"]), "code": d["code"], "row": d["row"], "parts": d["parts"]}
                        for d in c["differences"][:limit]],
        "distant": len(c["distant"]),
        "distances": [{"uid": uid(d["order"]), "code": d["code"], "row": d["row"], "distance": d["distance"]}
                      for d in c["distant"][:limit]],
        "undrawn": len(c["undrawn"]),
        "undrawnAddresses": [with_uids(dict(u), u["rows"]) for u in c["undrawn"][:limit]],
        "repeated": len(c["repeats"]),
        "registerRepeats": [with_uids(dict(r), r["rows"]) for r in c["repeats"][:limit]],
        "revision": "$current",
    }


# ── The cases ────────────────────────────────────────────────────────────────

UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}
FULL = {"code": "ADRES_NO", "street": "YOL", "door": "DIS_KAPI", "neighbourhood": "MAHALLE", "district": "ILCE",
        "x": "X", "y": "Y"}
TABLE = [COLUMNS] + ROWS


def reg(path="uavt.csv", **columns):
    out = {"path": FIX + path}
    out.update(columns or {k: v for k, v in FULL.items()})
    return out


def keys(columns):
    """The command's column names → the reference's keys."""
    return {k: v for k, v in columns.items() if k in FULL}


def ok(name, inp, setup=None, locator=LOCATOR, table=TABLE, tolerance=10.0, limit=100, layer=None, op="execute",
       warnings=(), before=()):
    s = setup or setup_with()
    columns = keys(inp["register"])
    out = answer(s, locator, table, columns, tolerance, limit, layer)
    case = {"name": name, "steps": list(before) + [
        {"op": op, "input": inp, "result": {"status": "completed", "output": out, "warnings": list(warnings)},
         "expect": UNCHANGED}]}
    if setup is not None:
        case["setup"] = setup
    return case


def refused(name, inp, code, message, path, setup=None, before=(), op="execute"):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    case = {"name": name, "steps": list(before) + [
        {"op": op, "input": inp, "result": {"status": "failed", "error": err}, "expect": UNCHANGED}]}
    if setup is not None:
        case["setup"] = setup
    return case


def cases():
    out = []
    out.append(ok("Belediyenin UAVT tablosu (CSV): kayıtta olmayan kod, farklı yazılan adres, uzak kapı, kayıtta olup "
                  "çizimde olmayan adres, yinelenen kod", {"register": reg()}))
    out.append(ok("Plan: aynı cevap, çizim değişmez", {"register": reg()}, op="plan"))
    out.append(ok("Sekmeyle ayrılmış metin tablosu: aynı cevap", {"register": reg("uavt.txt")}))
    out.append(ok("Her listeden ilk 1", {"register": reg(), "limit": 1}, limit=1))
    out.append(ok("Tolerans 30 m: uzak kapı yok", {"register": reg(), "tolerance": 30.0}, tolerance=30.0))
    out.append(ok("Tolerans 0,4 m: yarım metre uzaktaki kapı da", {"register": reg(), "tolerance": 0.4}, tolerance=0.4))
    out.append(ok("Yalnız kod sütunu: farklar, yer ve çizimde olmayan adresler karşılaştırılmaz",
                  {"register": reg(code="ADRES_NO")}))
    out.append(ok("Yeri vermeyen kayıt: uzak kapı yok", {"register": reg(code="ADRES_NO", street="YOL", door="DIS_KAPI",
                                                                        neighbourhood="MAHALLE", district="ILCE")}))
    out.append(ok("Kapılarında mahalle ve ilçe alanı olmayan adres verisi: onlar karşılaştırılmaz",
                  {"locator": "adres-2", "register": reg()}, locator=PLAIN))
    # The register as a layer of the drawing.
    with_layer = setup_with(ENTITIES + layer_rows())
    out.append(ok("Çizimdeki kayıt katmanı: yeri noktaları, satırları nesnelerinin kimlikleriyle",
                  {"register": {"layer": REGISTER_LAYER, "code": "ADRES_NO", "street": "YOL", "door": "DIS_KAPI",
                                "neighbourhood": "MAHALLE", "district": "ILCE"}},
                  setup=with_layer, layer=REGISTER_LAYER))
    out.append(ok("Kayıt katmanı, yalnız kodla", {"register": {"layer": REGISTER_LAYER, "code": "ADRES_NO"}},
                  setup=with_layer, layer=REGISTER_LAYER))
    # A drawing's file: the register's path from its folder.
    out.append(ok("Çizimin klasöründen göreli yol", {"register": {**reg(), "path": "kayit/uavt.csv"},
                                                   "drawing": "$tmp:ac01/cizim.kcad"},
                  before=[{"op": "copyFile", "from": FIX + "uavt.csv", "to": "$tmp:ac01/kayit/uavt.csv"}]))
    out.append(ok("Doğrulama", {"register": reg()}, op="validate"))
    out[-1]["steps"][-1]["result"] = {"status": "completed", "output": None, "warnings": []}
    # Refusals, in their order.
    msg_limit = "Liste sınırı 1 ile 10 000 arasında bir tam sayı olmalı."
    out.append(refused("Sınır 0", {"register": reg(), "limit": 0}, "invalid_limit", msg_limit, "limit"))
    out.append(refused("Sınır 10 001", {"register": reg(), "limit": 10001}, "invalid_limit", msg_limit, "limit"))
    out.append(ok("Sınırın en çoğu 10 000", {"register": reg(), "limit": 10000}, limit=10000))
    msg_tol = "Tolerans 0'dan büyük, en çok 10 000 m olmalı."
    out.append(refused("Tolerans 0", {"register": reg(), "tolerance": 0.0}, "invalid_tolerance", msg_tol, "tolerance"))
    out.append(refused("Tolerans 10 001 m", {"register": reg(), "tolerance": 10001.0}, "invalid_tolerance", msg_tol,
                       "tolerance"))
    msg_missing = ("Karşılaştırılacak adres kaydını verin: bir tablo dosyası (path) ya da çizimin bir katmanı (layer), "
                   "ikisinden biri.")
    out.append(refused("Ne dosya ne katman", {"register": {"code": "ADRES_NO"}}, "register_missing", msg_missing,
                       "register"))
    out.append(refused("Hem dosya hem katman", {"register": {**reg(), "layer": REGISTER_LAYER}}, "register_missing",
                       msg_missing, "register"))
    out.append(refused("Katmanla sayfa verilmez", {"register": {"layer": REGISTER_LAYER, "sheet": "Sayfa1",
                                                                "code": "ADRES_NO"}}, "invalid_input",
                       "Sayfa (sheet) yalnız tablo dosyasıyla verilir; katmandan okunan kayıtta sayfa yoktur.",
                       "register.sheet"))
    out.append(refused("Boş kod sütunu", {"register": reg(code=" ")}, "invalid_input",
                       "Adres kodunun sütununu (code) verin.", "register.code"))
    out.append(refused("Adres verisi olmayan proje", {"register": reg()}, "no_address_data",
                       "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın.",
                       None, setup=setup_with(locators=[])))
    out.append(refused("Projede olmayan adres verisi", {"locator": "yok", "register": reg()}, "unknown_locator",
                       "Projede “yok” kimlikli adres verisi yok.", "locator"))
    out.append(refused("Kapılarında kod alanı olmayan adres verisi", {"locator": "adres-3", "register": reg()},
                       "no_address_codes",
                       "“Kodsuz” adres verisinin kapılarında adres kodu alanı yok; Adres verisi penceresinde kapıların "
                       "Adres kodu alanını seçin.", "locator"))
    out.append(refused("Boş dosya yolu", {"register": {**reg(), "path": "  "}}, "invalid_file_path",
                       "Adres kaydının dosya yolu boş; bir tablo dosyasının yolunu verin.", "register.path"))
    out.append(refused("Göreli yol, çizimin dosyası yok", {"register": {**reg(), "path": "kayit/uavt.csv"}},
                       "invalid_file_path",
                       "“kayit/uavt.csv” göreli bir yol, ama çizimin dosyası yok; mutlak yolu verin ya da çizimi önce "
                       "kaydedin.", "register.path"))
    out.append(refused("Olmayan dosya", {"register": reg("yok.csv")}, "file_not_found", f"{FIX}yok.csv dosyası yok.",
                       "register.path"))
    out.append(refused("Boş dosya", {"register": reg("bos.csv")}, "register_unreadable", "Adres kaydında satır yok.",
                       "register.path"))
    out.append(refused("Eski Excel dosyası", {"register": reg("eski.xls")}, "register_unreadable",
                       "Eski Excel biçimi (.xls) okunmuyor. Dosyayı Excel'de XLSX ya da CSV olarak kaydedip seçin.",
                       "register.path"))
    out.append(refused("Metin tablosunda sayfa adı", {"register": {**reg(), "sheet": "Sayfa1"}}, "register_unreadable",
                       "Adres kaydı tek sayfalı bir metin tablosu; sayfa adı verilmez.", "register.sheet"))
    out.append(refused("Çizimde olmayan katman", {"register": {"layer": "yok", "code": "ADRES_NO"}}, "layer_not_found",
                       "“yok” katmanı çizimde yok.", "register.layer"))
    out.append(refused("Kayıtta olmayan sütun", {"register": reg(code="ADRES_KODU")}, "register_column_missing",
                       "Adres kaydında “ADRES_KODU” sütunu yok; sütunları: " + ", ".join(COLUMNS) + ".", "register.code"))
    out.append(refused("Kayıtta olmayan yer sütunu", {"register": {**reg(), "x": "DOGU"}}, "register_column_missing",
                       "Adres kaydında “DOGU” sütunu yok; sütunları: " + ", ".join(COLUMNS) + ".", "register.x"))
    out.append(refused("Kayıt katmanında olmayan alan", {"register": {"layer": REGISTER_LAYER, "code": "UAVT_NO"}},
                       "register_column_missing",
                       "“Kayıt” katmanında “UAVT_NO” alanı yok; alanları: " + ", ".join(sorted(COLUMNS[:5])) + ".",
                       "register.code", setup=with_layer))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.addresses.compare", "commandVersion": 1,
            "title": "Adres kaydıyla karşılaştır",
            "note": "Written by scripts/fixtures/address_compare_cases.py from docs/adr/0736 and the address reference; do not edit by hand.",
            "setup": setup_with(), "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    written = files()
    if "--check" in sys.argv:
        bad = []
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            bad.append(str(OUT.relative_to(ROOT)))
        for name, data in written.items():
            p = REGISTER / name
            if not p.exists() or p.read_bytes() != data:
                bad.append(str(p.relative_to(ROOT)))
        if bad:
            sys.exit("güncel değil: " + ", ".join(bad) + "; betiği --check'siz çalıştırın.")
        print(f"{OUT.relative_to(ROOT)} güncel: {len(build()['cases'])} durum.")
        return
    REGISTER.mkdir(parents=True, exist_ok=True)
    for name, data in written.items():
        (REGISTER / name).write_bytes(data)
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
