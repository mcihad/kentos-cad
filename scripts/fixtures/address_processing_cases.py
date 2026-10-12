#!/usr/bin/env python3
"""Adres's İşlemler cases (docs/adr/0229 §11, §12), written from the ADR without KentOS code.

Writes fixtures/processing/v1/address.kcad (a drawing: the metres scene of scripts/fixtures/geocode_cases.py as doors,
streets and areas with a locator in its settings, the events whose addresses are written, the applications whose
columns are matched), fixtures/processing/v1/adresler.csv (a table of one-line addresses) and
fixtures/processing/v1/address.json (Adresleri eşle and Konumlara adres yaz run on them, in the processing cases'
format, fixtures/processing/README.md). The places, scores and texts are the locator's reference's
(geocode_cases.py); the fields, the report, the summary and the refusals are the ADR's §11 and §12.

    python3 scripts/fixtures/address_processing_cases.py [--check]
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import geocode_cases as g  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PROC_DIR = ROOT / "fixtures" / "processing" / "v1"
E, N = 500000.0, 4400000.0
STYLE = {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}

LOCATOR = {
    "id": "adres-1",
    "name": "Numarataj",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE",
              "province": "IL", "postcode": "POSTA_KODU", "building": "BINA_ADI", "code": "UAVT"},
    "streets": {"layer": "yol", "name": "YOL_ADI", "neighbourhood": "MAHALLE", "district": "ILCE"},
    "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD", "district": "ILCE"},
              {"layer": "ilce", "level": "district", "name": "AD"},
              {"layer": "il", "level": "province", "name": "AD"}],
    "minScore": 85,
    "reach": 100.0,
}

EVENTS = [(32, 106), (100, 300), (180, 380), (450, 52), (300, 30), (1000, 1000)]
APPLICATIONS = [
    {"Mahalle": "Kızılay", "Yol": "ATATÜRK BULVARI", "Kapı": "008"},
    {"Mahalle": "Cumhuriyet Mahallesi", "Yol": "1203/1 Sokak", "Kapı": "No: 5"},
    {"Mahalle": "", "Yol": "Atatürk Bulvarı", "Kapı": "6/A"},
    {"Mahalle": "Bahçelievler", "Yol": "Gazi Mustafa Kemal Blv.", "Kapı": ""},
]
ROWS = [
    ("1", "Kızılay Mah. Atatürk Blv. No:4 Çankaya/Ankara"),
    ("2", "Atatürk Bulvarı No:2"),
    ("3", "Kızılay Mahallesi Atatürk Bulvarı No:6"),
    ("4", "İnönü Caddesi No:1"),
    ("5", ""),
    ("6", "1000000002"),
    ("7", "Kızılay Mah. Atatürk Blv. No:99"),
]


def node(id_, name, children=()):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True, "style": dict(STYLE), "children": list(children)}


def drawing():
    scene = g.city(1.0, E, N, 1.0)
    entities, eid = [], 0

    def add(kind, layer, attrs, **geom):
        nonlocal eid
        eid += 1
        entities.append({"kind": kind, "id": eid, "layerId": layer, "attrs": attrs, **geom})
        return eid

    def xy(p):
        return {"x": p[0], "y": p[1]}

    # The scene's objects in its order: areas, streets, doors (their orders are the drawing's).
    for a in scene["areas"]:
        layer = {"neighbourhood": "mahalle", "district": "ilce", "province": "il"}[a["level"]]
        attrs = {"AD": a["name"]}
        if a["district"]:
            attrs["ILCE"] = a["district"]
        add("polygon", layer, attrs, pts=[xy(p) for p in a["rings"][0]])
    for s in scene["streets"]:
        attrs = {"YOL_ADI": s["name"]}
        if s["neighbourhood"]:
            attrs["MAHALLE"] = s["neighbourhood"]
        if s["district"]:
            attrs["ILCE"] = s["district"]
        add("polyline", "yol", attrs, pts=[xy(p) for p in s["pts"]])
    for d in scene["doors"]:
        attrs = {k: v for k, v in (("YOL_ADI", d["street"]), ("KAPI_NO", d["door"]), ("MAHALLE", d["neighbourhood"]),
                                   ("ILCE", d["district"]), ("IL", d["province"]), ("POSTA_KODU", d["postcode"]),
                                   ("BINA_ADI", d["building"]), ("UAVT", d["code"])) if v}
        add("point", "kapi", attrs, p=xy(d["at"]))
    events = [add("point", "olay", {"Ad": f"O{i + 1}"}, p={"x": E + x, "y": N + y}) for i, (x, y) in enumerate(EVENTS)]
    apps = [add("point", "basvuru", {**{k: v for k, v in a.items() if v}, "No": f"B{i + 1}"}, p={"x": E - 50.0 - i, "y": N - 50.0})
            for i, a in enumerate(APPLICATIONS)]
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": "Adresler",
        "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad", "plotScale": 1000,
                     "workspace": "gis", "drawingFont": "barlow", "addressLocators": [LOCATOR]},
        "origin": {"x": E, "y": N},
        "layers": [node("kapi", "Kapı"), node("yol", "Yol orta hattı"), node("mahalle", "Mahalle"), node("ilce", "İlçe"),
                   node("il", "İl"), node("olay", "Olay"), node("basvuru", "Başvuru"), node("0", "0")],
        "activeLayer": "0",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return doc, scene, events, apps


def csv_text():
    def cell(t):
        return '"' + t.replace('"', '""') + '"' if ("," in t or '"' in t) else t
    return "No,Adres\n" + "".join(f"{no},{cell(a)}\n" for no, a in ROWS)


FIELDS = ["Mahalle", "Yol", "Kapı no", "İlçe", "İl", "Posta kodu"]
LAYER = {"id": "islem-adresler", "name": "Adresler", "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.25, "fill": "#E5484D26"}}


def hit_attrs(base, c, status, text):
    attrs = dict(base)
    attrs["Durum"] = status
    attrs["Puan"] = str(c["score"])
    attrs["Eşleşme türü"] = c["kind"]
    attrs["Eşleşen adres"] = c["label"]
    t = c["texts"]
    for name, key in zip(FIELDS, ("neighbourhood", "street", "door", "district", "province", "postcode")):
        if t[key]:
            attrs[name] = t[key]
    attrs["Kaynak"] = LOCATOR["name"]
    attrs["Girdi adresi"] = text
    return attrs


def geocode_case(cid, title, loc, rows, labels, base_attrs, values, single, tied_too, min_score):
    added, report = [], []
    matched = tied = unmatched = empty = 0
    for i, row in enumerate(rows):
        if single:
            text = row.strip()
            parts = g.parse(text)
            code_text = text
        else:
            text = line_of(*row)
            parts = g.from_columns({"neighbourhood": row[0], "street": row[1], "door": row[2], "district": row[3], "province": row[4], "postcode": row[5]})
            code_text = ""
        if not text:
            empty += 1
            report.append([labels[i], "", "", "", "Adres boş"])
            continue
        cands, status = g.find(loc, code_text, parts, min_score, 20)
        best = cands[0] if cands else None
        if status == "Eşleşti":
            matched += 1
            written = True
        elif status == "Belirsiz":
            tied += 1
            written = tied_too
        else:
            unmatched += 1
            written = False
        if best is not None and written:
            added.append({"kind": "point", "layerId": LAYER["id"], "attrs": hit_attrs(base_attrs[i], best, status, text),
                          "p": {"x": best["at"][0], "y": best["at"][1]}})
        elif best is not None:
            report.append([labels[i], text, best["label"], best["score"], status])
        else:
            report.append([labels[i], text, "", "", "Aday yok"])
    total = len(rows)
    summary = f"{total} satırdan {matched} eşleşti, {tied} belirsiz, {unmatched} eşleşmedi"
    if empty:
        summary += f", {empty} boş"
    summary += f"; {len(added)} nokta yazıldı."
    outputs = {"count": len(added)}
    if report:
        outputs["table"] = {"columns": ["Satır", "Girdi adresi", "En iyi aday", "Puan", "Durum"], "rows": report}
    expect = {"status": "ok", "summary": summary, "undo": "Adresleri eşle", "layers": [LAYER] if added else [], "added": added, "outputs": outputs}
    return {"id": cid, "title": title, "document": "address.kcad", "run": {"tool": "address.geocode"}, "values": values, "expect": expect}


def line_of(hood, street, door, district, province, postcode):
    parts = []
    head = (street.strip() + " " + door.strip()).strip()
    if head:
        parts.append(head)
    n = hood.strip()
    if n:
        parts.append(n if g.tagged_hood(n) else n + " Mahallesi")
    parts += [x.strip() for x in (district, province, postcode) if x.strip()]
    return ", ".join(parts)


REVERSE_FIELDS = ["Adres", "Mahalle", "Yol", "Kapı no", "İlçe", "İl", "Posta kodu", "Adres uzaklığı", "Adres türü", "Adres kaynağı"]


def reverse_case(cid, title, loc, doc, events, reach, values):
    updated, count, missing = [], 0, 0
    by_id = {e["id"]: e for e in doc["entities"]}
    for i, eid in enumerate(events):
        e = by_id[eid]
        p = (e["p"]["x"], e["p"]["y"])
        place = g.reverse(loc, p, reach)
        attrs = dict(e["attrs"])
        if place is None:
            missing += 1
            for f in REVERSE_FIELDS:
                attrs.pop(f, None)
        else:
            count += 1
            t = place["texts"]
            values_ = [place["label"], t["neighbourhood"], t["street"], t["door"], t["district"], t["province"], t["postcode"],
                       "" if place["distance"] is None else f"{place['distance']:.2f}", place["kind"], LOCATOR["name"]]
            for f, v in zip(REVERSE_FIELDS, values_):
                if v:
                    attrs[f] = v
                else:
                    attrs.pop(f, None)
        if attrs != e["attrs"]:
            updated.append({"id": eid, "attrs": attrs})
    total = len(events)
    expect = {"status": "ok", "summary": f"{total} nesneden {count} nesnenin adresi yazıldı; {missing} nesnenin adresi bulunamadı.",
              "undo": "Konumlara adres yaz", "updated": updated, "outputs": {"count": count, "missing": missing}}
    return {"id": cid, "title": title, "document": "address.kcad", "run": {"tool": "address.reverse"}, "values": values, "expect": expect}


def build():
    doc, scene, events, apps = drawing()
    loc = g.build(scene)
    rows = [a for _, a in ROWS]
    labels = [str(i + 2) for i in range(len(ROWS))]
    base = [{k: v for k, v in (("No", no), ("Adres", a)) if v.strip()} for no, a in ROWS]
    file_values = {"sourceKind": "file", "file": "adresler.csv", "form": "single", "address": "Adres", "via": "locator:adres-1", "tied": True, "layer": {"newName": "Adresler"}}
    cases = [
        geocode_case("geocode-file", "Adresleri eşle: dosyadan tek alan; kapı, belirsiz (yazılır), aradeğer, eşleşmeyen, boş satır ve adres kodu",
                     loc, rows, labels, base, file_values, True, True, 85),
        geocode_case("geocode-file-strict", "Adresleri eşle: en düşük puan 95, belirsizler yazılmaz; aradeğer (90) eşleşmez",
                     loc, rows, labels, base, {**file_values, "tied": False, "minScore": 95}, True, False, 95),
    ]
    app_rows = [(a["Mahalle"], a["Yol"], a["Kapı"], "", "", "") for a in APPLICATIONS]
    by_id = {e["id"]: e for e in doc["entities"]}
    app_attrs = [dict(by_id[i]["attrs"]) for i in apps]
    cases.append(geocode_case(
        "geocode-columns", "Adresleri eşle: Başvuru katmanının öznitelikleri, ayrı alanlar (mahalle, yol, kapı no); kapısız yol düzeyi",
        loc, app_rows, [f"#{i}" for i in apps], app_attrs,
        {"sourceKind": "layer", "source": {"scope": "layer", "layerId": "basvuru"}, "form": "columns", "neighbourhood": "Mahalle",
         "street": "Yol", "door": "Kapı", "via": "locator:adres-1", "tied": True, "layer": {"newName": "Adresler"}},
        False, True, 85))
    cases.append(reverse_case("reverse-events", "Konumlara adres yaz: olaylara en yakın kapı, yol ya da içinde kaldıkları bölge (100 m)",
                              loc, doc, events, 100.0, {"input": {"scope": "layer", "layerId": "olay"}, "via": "locator:adres-1"}))
    cases.append(reverse_case("reverse-reach", "Konumlara adres yaz, en çok 3 m: kapı ve yol yalnız çok yakında; bölge ya da hiçbiri",
                              loc, doc, events, 3.0, {"input": {"scope": "layer", "layerId": "olay"}, "via": "locator:adres-1", "reach": 3}))
    cases.append({"id": "geocode-no-source", "title": "Adresleri eşle: kaynak seçilmemiş", "document": "address.kcad", "run": {"tool": "address.geocode"},
                  "values": {**file_values, "via": ""},
                  "expect": {"status": "invalid", "issues": [{"param": "via", "message": "“Adres kaynağı”: projenin adres verisini ya da bir sağlayıcıyı seçin."}]}})
    cases.append({"id": "geocode-online-here", "title": "Adresleri eşle: çevrimiçi sağlayıcı, ağı olmayan ortamda reddedilir", "document": "address.kcad",
                  "run": {"tool": "address.geocode"}, "values": {**file_values, "via": "provider:nominatim-osm"},
                  "expect": {"status": "error", "message": "Bu ortamda çevrimiçi adres sağlayıcısı yok; projenin adres verisini seçin."}})
    cases.append({"id": "geocode-unknown-locator", "title": "Adresleri eşle: projede olmayan adres verisi", "document": "address.kcad",
                  "run": {"tool": "address.geocode"}, "values": {**file_values, "via": "locator:adres-9"},
                  "expect": {"status": "error", "message": "“adres-9” adres verisi projede yok; Adres verisi penceresinden tanımlayın."}})
    proc = {"format": "kentos.processing-cases", "version": 1, "tolerance": 1e-9, "documents": {}, "files": {"adresler.csv": "adresler.csv"}, "cases": cases}
    return doc, proc


def main():
    doc, proc = build()
    outs = [
        (PROC_DIR / "address.json", json.dumps(proc, ensure_ascii=False, indent=1) + "\n"),
        (PROC_DIR / "address.kcad", json.dumps(doc, ensure_ascii=False, indent=1) + "\n"),
        (PROC_DIR / "adresler.csv", csv_text()),
    ]
    if "--check" in sys.argv[1:]:
        bad = [p for p, t in outs if not p.exists() or p.read_text("utf-8") != t]
        for p in bad:
            print(f"{p.relative_to(ROOT)} yeniden üretilenle aynı değil (betiği --check olmadan çalıştırın)", file=sys.stderr)
        if bad:
            return 1
        print("adres durumları tutarlı: fixtures/processing/v1/address.*, adresler.csv")
        return 0
    for p, t in outs:
        p.write_text(t, "utf-8")
        print(f"yazıldı: {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
