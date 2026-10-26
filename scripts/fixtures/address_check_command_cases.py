#!/usr/bin/env python3
"""Writes fixtures/commands/v1/cad.addresses.check.json: the shared cases of `cad.addresses.check` v1 (docs/adr/0579),
their expectations built here, without KentOS code: the locator is built by the address reference's own builder
(scripts/fixtures/geocode_cases.py, `build`), and the check's rule is the ADR's:

- the counts: doors, streets (the locator's groups), lines, neighbourhoods, areas; what was left out;
- repeats: a door number one street has more than once (by the door's key), its street, neighbourhood and door as the
  first of its doors writes them;
- codes: an address code on more than one door;
- spellings: a street whose doors write its name in more than one way: each way and how many doors write it, the most
  written first, on a tie the one written first;
- (docs/adr/0647 §1) neighbourhood and district spellings: doors grouped by the neighbourhood's key (its words without
  the MAHALLE and KÖY tags) and the district's, or by the district's key alone; a group whose doors write the name in
  more than one way, by the same rule, `within` the district its first door writes (none for a district);
- each list's count, then its first `limit` by their first door; the doors by their objects' persistent ids.

The desktop (crates/native/application/tests/all/fixtures.rs) and Python (python/tests/test_command_cases.py) run them.

    python3 scripts/fixtures/address_check_command_cases.py           # writes the file
    python3 scripts/fixtures/address_check_command_cases.py --check   # compares it with the one on disk
"""
import copy
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import geocode_cases as ref  # noqa: E402  (the address reference: no KentOS code)

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "fixtures/commands/v1/cad.addresses.check.json"
STYLE = {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}
X0, Y0 = 500000.0, 4400000.0


def node(id_, name):
    return {"id": id_, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
            "style": dict(STYLE), "children": []}


LOCATOR = {
    "id": "adres-1",
    "name": "Numarataj",
    "doors": {"layer": "kapi", "street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE",
              "code": "UAVT"},
    "streets": {"layer": "yol", "name": "YOL_ADI"},
    "areas": [{"layer": "mahalle", "level": "neighbourhood", "name": "AD"}],
    "minScore": 85,
    "reach": 100.0,
}
SECOND = {
    "id": "adres-2",
    "name": "Yalnız kapılar",
    "doors": {"layer": "kapi2", "street": "YOL", "door": "NO"},
    "minScore": 85,
    "reach": 100.0,
}


def door(id_, layer, street, no, hood="Kızılay", district="Çankaya", code="", x=0.0, fields=None):
    names = fields or {"street": "YOL_ADI", "door": "KAPI_NO", "neighbourhood": "MAHALLE", "district": "ILCE",
                       "code": "UAVT"}
    attrs = {names["street"]: street, names["door"]: no}
    if "neighbourhood" in names and hood:
        attrs[names["neighbourhood"]] = hood
    if "district" in names and district:
        attrs[names["district"]] = district
    if "code" in names and code:
        attrs[names["code"]] = code
    return {"kind": "point", "id": id_, "layerId": layer, "attrs": attrs, "p": {"x": X0 + x, "y": Y0 + 10.0}}


def square(x0, y0, x1, y1):
    return [{"x": X0 + x0, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y0}, {"x": X0 + x1, "y": Y0 + y1},
            {"x": X0 + x0, "y": Y0 + y1}]


ENTITIES = [
    door(1, "kapi", "Atatürk Bulvarı", "12", code="1001", x=10),
    door(2, "kapi", "ATATÜRK BULVARI", "14", code="1002", x=20),
    door(3, "kapi", "Atatürk Blv.", "16", code="1003", x=30),
    door(4, "kapi", "Atatürk Bulvarı", "12", code="1004", x=40),
    door(5, "kapi", "Gazi Mustafa Kemal Cd.", "3", code="1001", x=50),
    door(6, "kapi", "Gazi Mustafa Kemal Caddesi", "5", x=60),
    door(7, "kapi", "Gazi Mustafa Kemal Caddesi", "7", code="1006", x=70),
    door(8, "kapi", "", "9", x=80),
    door(9, "kapi", "Tunalı Hilmi Cd.", "", x=90),
    {"kind": "line", "id": 10, "layerId": "kapi", "attrs": {}, "a": {"x": X0, "y": Y0}, "b": {"x": X0 + 5, "y": Y0}},
    {"kind": "polyline", "id": 11, "layerId": "yol", "attrs": {"YOL_ADI": "Atatürk Bulvarı"},
     "pts": [{"x": X0 + 5, "y": Y0 + 5}, {"x": X0 + 95, "y": Y0 + 5}]},
    {"kind": "polyline", "id": 12, "layerId": "yol", "attrs": {},
     "pts": [{"x": X0 + 5, "y": Y0 + 20}, {"x": X0 + 95, "y": Y0 + 20}]},
    {"kind": "point", "id": 13, "layerId": "yol", "attrs": {"YOL_ADI": "Esat Cd."}, "p": {"x": X0 + 50, "y": Y0 + 30}},
    {"kind": "polyline", "id": 14, "layerId": "yol", "attrs": {"YOL_ADI": "Esat Caddesi"},
     "pts": [{"x": X0 + 5, "y": Y0 + 40}, {"x": X0 + 95, "y": Y0 + 40}]},
    {"kind": "polygon", "id": 15, "layerId": "mahalle", "attrs": {"AD": "Kızılay Mahallesi"}, "pts": square(0, 0, 100, 50)},
    {"kind": "polygon", "id": 16, "layerId": "mahalle", "attrs": {}, "pts": square(200, 0, 300, 50)},
    {"kind": "point", "id": 17, "layerId": "mahalle", "attrs": {"AD": "Bahçelievler"}, "p": {"x": X0 + 150, "y": Y0}},
    # docs/adr/0647 §1: the neighbourhood and the district written otherwise.
    door(18, "kapi", "Esat Cd.", "2", hood="KIZILAY Mah.", district="ÇANKAYA", x=95),
    door(19, "kapi", "Esat Cd.", "4", hood="Kızılay Mahallesi", district=" Çankaya ", x=96),
]

SETUP = {
    "format": "kentos.document",
    "version": 1,
    "name": "Adresler",
    "settings": {"srid": 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                 "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow",
                 "addressLocators": [copy.deepcopy(LOCATOR), copy.deepcopy(SECOND)]},
    "origin": {"x": X0, "y": Y0},
    "layers": [node("kapi", "Kapı"), node("kapi2", "Kapı 2"), node("yol", "Yol orta hattı"), node("mahalle", "Mahalle"),
               node("0", "0")],
    "activeLayer": "0",
    "entities": ENTITIES,
    "styles": {"items": [], "categories": []},
}


def entities_of(setup):
    return setup["entities"]


def scene(setup, locator):
    """The address reference's scene of a drawing: the objects of the locator's layers in the drawing's order (their
    place: their index), points as doors, lines and paths as streets, areas as areas; the rest counted."""
    out = {"perMetre": 1.0, "doors": [], "streets": [], "areas": [], "notDoors": 0, "notStreets": 0, "notAreas": 0}
    ents = entities_of(setup)
    d, s = locator.get("doors"), locator.get("streets")
    for order, e in enumerate(ents):
        attrs = e["attrs"]

        def val(names, key):
            f = names.get(key)
            return attrs.get(f, "") if f else ""

        if d and e["layerId"] == d["layer"]:
            if e["kind"] != "point":
                out["notDoors"] += 1
            else:
                out["doors"].append({"order": order, "at": (e["p"]["x"], e["p"]["y"]), "street": val(d, "street"),
                                     "door": val(d, "door"), "neighbourhood": val(d, "neighbourhood"),
                                     "district": val(d, "district"), "province": val(d, "province"),
                                     "postcode": val(d, "postcode"), "building": val(d, "building"),
                                     "code": val(d, "code")})
        if s and e["layerId"] == s["layer"]:
            if e["kind"] not in ("line", "polyline", "arc"):
                out["notStreets"] += 1
            else:
                pts = [(p["x"], p["y"]) for p in e["pts"]] if e["kind"] == "polyline" else \
                    [(e["a"]["x"], e["a"]["y"]), (e["b"]["x"], e["b"]["y"])]
                out["streets"].append({"order": order, "pts": pts, "name": val(s, "name"),
                                       "neighbourhood": val(s, "neighbourhood"), "district": val(s, "district")})
        for a in locator.get("areas", []):
            if e["layerId"] != a["layer"]:
                continue
            if e["kind"] not in ("polygon", "circle"):
                out["notAreas"] += 1
                continue
            ring = [(p["x"], p["y"]) for p in e["pts"]]
            out["areas"].append({"order": order, "level": a["level"], "rings": [ring], "name": val(a, "name"),
                                 "district": val(a, "district")})
    return out


def uid(setup, order):
    return f"$uidOf:{entities_of(setup)[order]['id']}"


def name_spellings(loc, key_of, field, within_of):
    """ADR 0647 §1: doors grouped by `key_of` (none: left out), each group whose doors write `field` in more than one
    way; the groups by their first doors."""
    groups = {}
    for d in loc["doors"]:
        k = key_of(d)
        if k is None:
            continue
        groups.setdefault(k, []).append(d)
    out = []
    for doors in groups.values():
        written = []
        for d in doors:
            t = d["texts"][field]
            for w in written:
                if w[0] == t:
                    w[1] += 1
                    break
            else:
                written.append([t, 1, d["order"]])
        if len(written) < 2:
            continue
        written.sort(key=lambda w: (-w[1], w[2]))
        out.append({"within": within_of(doors[0]), "written": [{"text": t, "doors": n} for t, n, _ in written],
                    "orders": sorted(d["order"] for d in doors)})
    out.sort(key=lambda g: g["orders"][0])
    return out


def check(setup, locator, limit):
    sc = scene(setup, locator)
    loc = ref.build(sc)
    unnamed_lines = sum(1 for st in sc["streets"] if ref.street_value(st["name"]) is None)
    unnamed_areas = 0
    for a in sc["areas"]:
        name = ref.area_value(a["name"]) if a["level"] == "neighbourhood" else ref.words_key(a["name"])
        if not name:
            unnamed_areas += 1
    repeats = []
    for g in loc["groups"]:
        seen = {}
        for i in g["doors"]:
            seen.setdefault(loc["doors"][i]["key"], []).append(i)
        for key, doors in seen.items():
            if len(doors) > 1:
                first = loc["doors"][doors[0]]["texts"]
                repeats.append({"street": first["street"], "neighbourhood": first["neighbourhood"], "door": first["door"],
                                "orders": [loc["doors"][i]["order"] for i in doors]})
    repeats.sort(key=lambda r: r["orders"][0])
    codes = []
    for code, doors in loc["codes"].items():
        if len(doors) > 1:
            codes.append({"code": code, "orders": sorted(loc["doors"][i]["order"] for i in doors)})
    codes.sort(key=lambda c: (c["orders"][0], c["code"]))
    spellings = []
    for g in loc["groups"]:
        written = []
        for i in g["doors"]:
            t = loc["doors"][i]["texts"]["street"]
            for w in written:
                if w[0] == t:
                    w[1] += 1
                    break
            else:
                written.append([t, 1, loc["doors"][i]["order"]])
        if len(written) < 2:
            continue
        written.sort(key=lambda w: (-w[1], w[2]))
        first = loc["doors"][g["doors"][0]]["texts"]
        spellings.append({"neighbourhood": first["neighbourhood"], "district": first["district"],
                          "written": [{"text": t, "doors": n} for t, n, _ in written],
                          "orders": sorted(loc["doors"][i]["order"] for i in g["doors"])})
    spellings.sort(key=lambda s: s["orders"][0])
    hoods = name_spellings(loc, lambda d: (d["group"][1], d["group"][2]) if d["group"][1] else None, "neighbourhood",
                           lambda d: d["texts"]["district"])
    districts = name_spellings(loc, lambda d: d["group"][2] or None, "district", lambda d: "")

    def uids(orders):
        return [uid(setup, o) for o in orders]

    return {
        "locator": locator["id"],
        "doors": len(loc["doors"]),
        "streets": len(loc["groups"]),
        "lines": len(loc["lines"]),
        "neighbourhoods": len(loc["hoods"]),
        "areas": len(loc["areas"]),
        "notDoors": sc["notDoors"],
        "emptyStreets": loc["emptyStreets"],
        "unreadDoors": loc["unreadDoors"],
        "notStreets": sc["notStreets"],
        "unnamedLines": unnamed_lines,
        "notAreas": sc["notAreas"],
        "unnamedAreas": unnamed_areas,
        "repeated": len(repeats),
        "repeats": [{"street": r["street"], "neighbourhood": r["neighbourhood"], "door": r["door"],
                     "uids": uids(r["orders"])} for r in repeats[:limit]],
        "coded": len(codes),
        "codes": [{"code": c["code"], "uids": uids(c["orders"])} for c in codes[:limit]],
        "spelled": len(spellings),
        "spellings": [{"neighbourhood": s["neighbourhood"], "district": s["district"], "written": s["written"],
                       "uids": uids(s["orders"])} for s in spellings[:limit]],
        "neighbourhoodsSpelled": len(hoods),
        "neighbourhoodSpellings": [{"within": h["within"], "written": h["written"], "uids": uids(h["orders"])}
                                   for h in hoods[:limit]],
        "districtsSpelled": len(districts),
        "districtSpellings": [{"within": h["within"], "written": h["written"], "uids": uids(h["orders"])}
                              for h in districts[:limit]],
        "revision": "$current",
    }


UNCHANGED = {"canUndo": False, "canRedo": False, "dirty": False, "revision": "same"}


def ok(name, input_, setup=None, locator=None, limit=100):
    s = setup or SETUP
    loc = locator or s["settings"]["addressLocators"][0]
    out = {"name": name, "steps": [{"op": "execute", "input": input_,
                                     "result": {"status": "completed", "output": check(s, loc, limit), "warnings": []},
                                     "expect": UNCHANGED}]}
    if setup is not None:
        out["setup"] = setup
    return out


def plan(name, input_):
    loc = SETUP["settings"]["addressLocators"][0]
    return {"name": name, "steps": [{"op": "plan", "input": input_,
                                     "result": {"status": "completed", "output": check(SETUP, loc, 100), "warnings": []},
                                     "expect": UNCHANGED}]}


def refused(name, input_, code, message, path, setup=None):
    err = {"code": code, "message": message}
    if path is not None:
        err["path"] = path
    out = {"name": name, "steps": [{"op": "execute", "input": input_, "result": {"status": "failed", "error": err},
                                     "expect": UNCHANGED}]}
    if setup is not None:
        out["setup"] = setup
    return out


def with_entities(ents, locators=None):
    s = copy.deepcopy(SETUP)
    s["entities"] = ents
    if locators is not None:
        s["settings"]["addressLocators"] = locators
    return s


def cases():
    out = [
        ok("Projenin ilk adres verisi: sayılar, dışarıda kalanlar, yinelenen kapı, kod ve yazılış", {}),
        ok("Kimliğiyle adres verisi", {"locator": "adres-1"}),
        ok("Her listeden ilk 1", {"limit": 1}, limit=1),
        ok("Sınırın en çoğu 100", {"limit": 100}),
        plan("Plan: aynı cevap, çizim değişmez", {}),
        refused("Sınır 0 reddedilir", {"limit": 0}, "invalid_limit",
                "Liste sınırı 1 ile 100 arasında bir tam sayı olmalı.", "limit"),
        refused("Sınır 101 reddedilir", {"limit": 101}, "invalid_limit",
                "Liste sınırı 1 ile 100 arasında bir tam sayı olmalı.", "limit"),
        refused("Projede olmayan kimlik", {"locator": "yok"}, "unknown_locator",
                "Projede “yok” kimlikli adres verisi yok.", "locator"),
        refused("Adres verisi olmayan proje", {}, "no_address_data",
                "Projede adres verisi yok; önce Adres verisi penceresinde ya da cad.addresses.define ile tanımlayın.",
                None, setup=with_entities(ENTITIES, [])),
    ]
    # The second locator: doors alone, other field names.
    f2 = {"street": "YOL", "door": "NO"}
    second = [door(1, "kapi2", "İnönü Sk.", "1", fields=f2), door(2, "kapi2", "İNÖNÜ SOKAK", "1", fields=f2),
              door(3, "kapi2", "İnönü Sokağı", "2", fields=f2), door(4, "kapi2", "inönü sk", "3", fields=f2)]
    s2 = with_entities(second)
    out.append(ok("İkinci adres verisi: yalnız kapılar, başka alan adları", {"locator": "adres-2"}, setup=s2,
                  locator=SECOND))
    # No repeats, codes or spellings.
    clean = [door(1, "kapi", "Atatürk Bulvarı", "1", code="1"), door(2, "kapi", "Atatürk Bulvarı", "2", code="2")]
    out.append(ok("Temiz veri: hiçbir liste yok", {}, setup=with_entities(clean)))
    # One street in two neighbourhoods: two streets, no spelling.
    two = [door(1, "kapi", "Okul Sk.", "1", hood="Kızılay"), door(2, "kapi", "OKUL SOKAK", "1", hood="Bahçelievler")]
    out.append(ok("Aynı ad iki mahallede: iki yol, yinelenen yok", {}, setup=with_entities(two)))
    # A tie: two spellings written by two doors each, the one written first first.
    tie = [door(1, "kapi", "Okul Sk.", "1"), door(2, "kapi", "Okul Sokak", "2"), door(3, "kapi", "Okul Sokak", "3"),
           door(4, "kapi", "Okul Sk.", "4")]
    out.append(ok("Eşit yazılış: önce yazılan önce", {}, setup=with_entities(tie)))
    # The most written first, whatever came first.
    most = [door(1, "kapi", "Okul Sk.", "1"), door(2, "kapi", "Okul Sokak", "2"), door(3, "kapi", "Okul Sokak", "3")]
    out.append(ok("En çok yazılan önce", {}, setup=with_entities(most)))
    # Three doors with one number.
    three = [door(1, "kapi", "Lale Sk.", "5"), door(2, "kapi", "Lale Sk.", "5"), door(3, "kapi", "Lale Sk.", "5/A"),
             door(4, "kapi", "Lale Sk.", "5")]
    out.append(ok("Üç kapı bir numarada; 5/A ayrı", {}, setup=with_entities(three)))
    # A code on three doors of three streets.
    coded = [door(1, "kapi", "Lale Sk.", "1", code="77"), door(2, "kapi", "Gül Sk.", "1", code="77"),
             door(3, "kapi", "Menekşe Sk.", "1", code="77"), door(4, "kapi", "Menekşe Sk.", "2", code="78")]
    out.append(ok("Bir kod üç yolda", {}, setup=with_entities(coded)))
    # Codes ordered by their first door.
    order = [door(1, "kapi", "Lale Sk.", "1", code="B"), door(2, "kapi", "Lale Sk.", "2", code="A"),
             door(3, "kapi", "Lale Sk.", "3", code="A"), door(4, "kapi", "Lale Sk.", "4", code="B")]
    out.append(ok("Kodlar ilk kapılarına göre", {}, setup=with_entities(order)))
    # Leading and trailing spaces are not another spelling; the code is trimmed too.
    spaces = [door(1, "kapi", "  Lale Sk. ", "1", code=" 9 "), door(2, "kapi", "Lale Sk.", "2", code="9")]
    out.append(ok("Baştaki ve sondaki boşluklar başka yazılış değil", {}, setup=with_entities(spaces)))
    # Doors left out are in no list.
    left = [door(1, "kapi", "", "1", code="5"), door(2, "kapi", "Lale Sk.", "", code="5"),
            door(3, "kapi", "Lale Sk.", "1", code="5")]
    out.append(ok("Dışarıda kalan kapı listelere girmez", {}, setup=with_entities(left)))
    # docs/adr/0647 §1: the same neighbourhood in two districts is two; a district written three ways.
    names = [door(1, "kapi", "Lale Sk.", "1", hood="Yenimahalle", district="Çankaya"),
             door(2, "kapi", "Gül Sk.", "1", hood="YENİMAHALLE Mahallesi", district="ÇANKAYA"),
             door(3, "kapi", "Lale Sk.", "1", hood="Yenimahalle", district="Keçiören"),
             door(4, "kapi", "Gül Sk.", "2", hood="Yeni Mahalle", district="Çankaya"),
             door(5, "kapi", "Lale Sk.", "3", hood="", district="çankaya")]
    out.append(ok("Mahalle ve ilçe yazılışları: iki ilçede aynı mahalle iki, ilçe üç biçimde", {},
                  setup=with_entities(names)))
    # An empty drawing.
    out.append(ok("Boş çizim: sıfırlar", {}, setup=with_entities([])))
    # Many repeats cut at the limit, counted whole.
    many = []
    for k in range(6):
        many.append(door(2 * k + 1, "kapi", f"Sokak {k + 1}", "1", x=k))
        many.append(door(2 * k + 2, "kapi", f"Sokak {k + 1}", "1", x=k + 0.5))
    out.append(ok("Altı yinelenen, sınır 4", {"limit": 4}, setup=with_entities(many), limit=4))
    return out


def build():
    return {"format": "kentos.command-cases", "version": 1, "command": "cad.addresses.check", "commandVersion": 1,
            "title": "Adres verisini denetle",
            "note": "ADR 0579. Bağımsız başvuru scripts/fixtures/address_check_command_cases.py: dizin adres "
                    "başvurusunun kurucusuyla (geocode_cases.py), denetim ADR'nin kuralıyla; KentOS kodu yok.",
            "setup": SETUP, "cases": cases()}


def main():
    text = json.dumps(build(), ensure_ascii=False, indent=1) + "\n"
    if "--check" in sys.argv:
        if not OUT.exists() or OUT.read_text(encoding="utf-8") != text:
            sys.exit(f"{OUT.relative_to(ROOT)} güncel değil; betiği --check'siz çalıştırın.")
        print(f"{OUT.relative_to(ROOT)} güncel: {len(build()['cases'])} durum.")
        return
    OUT.write_text(text, encoding="utf-8")
    print(f"{OUT.relative_to(ROOT)} yazıldı: {len(build()['cases'])} durum.")


if __name__ == "__main__":
    main()
