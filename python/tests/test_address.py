"""kentos.address (docs/adr/0229 §14) against the independent reference's cases (fixtures/geocode/v1/cases.json,
scripts/fixtures/geocode_cases.py), the whole way Python asks: each scene (metres, and millimetres in a local project)
drawn as a drawing (areas on the level's layer, streets on ``yol``, doors on ``kapi`` with their attributes), opened
as a Document, its address data defined in the project through ``cad.addresses.define`` and built from the drawing as
the apps build it, every address and place asked through kentos.address; and an address's parts."""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from kentos import address, cad

ROOT = Path(__file__).resolve().parents[2]
CASES = json.loads((ROOT / "fixtures/geocode/v1/cases.json").read_text(encoding="utf-8"))
LEVEL_LAYERS = {"neighbourhood": "mahalle", "district": "ilce", "province": "il"}
LOCATOR = {
    "id": "adres-1",
    "name": "Numarataj",
    "doors": {"layer": "kapi", "street": "YOL", "door": "KAPI", "neighbourhood": "MAHALLE", "district": "ILCE", "province": "IL",
              "postcode": "PK", "building": "BINA", "code": "KOD"},
    "streets": {"layer": "yol", "name": "AD", "neighbourhood": "MAHALLE", "district": "ILCE"},
    "areas": [
        {"layer": "mahalle", "level": "neighbourhood", "name": "AD", "district": "ILCE"},
        {"layer": "ilce", "level": "district", "name": "AD"},
        {"layer": "il", "level": "province", "name": "AD"},
    ],
    "minScore": 85,
    "reach": 100.0,
}


def drawing(scene: dict[str, Any]) -> cad.Document:
    """A scene as a v1 drawing whose objects keep the scene's order."""

    def layer(id: str, name: str) -> dict[str, Any]:
        return {"id": id, "name": name, "type": "layer", "visible": True, "locked": False, "expanded": True,
                "style": {"color": "fg", "lineType": "continuous", "lineWeight": 0.25}, "children": []}

    def kept(pairs: list[tuple[str, str]]) -> dict[str, str]:
        return {k: v for k, v in pairs if v}

    def xy(p: list[float]) -> dict[str, float]:
        return {"x": p[0], "y": p[1]}

    items: list[tuple[int, dict[str, Any]]] = []
    for a in scene["areas"]:
        items.append((a["order"], {"kind": "polygon", "layerId": LEVEL_LAYERS[a["level"]], "attrs": kept([("AD", a["name"]), ("ILCE", a["district"])]),
                                   "pts": [xy(p) for p in a["rings"][0]]}))
    for s in scene["streets"]:
        items.append((s["order"], {"kind": "polyline", "layerId": "yol",
                                   "attrs": kept([("AD", s["name"]), ("MAHALLE", s["neighbourhood"]), ("ILCE", s["district"])]),
                                   "pts": [xy(p) for p in s["pts"]]}))
    for d in scene["doors"]:
        attrs = kept([("YOL", d["street"]), ("KAPI", d["door"]), ("MAHALLE", d["neighbourhood"]), ("ILCE", d["district"]),
                      ("IL", d["province"]), ("PK", d["postcode"]), ("BINA", d["building"]), ("KOD", d["code"])])
        items.append((d["order"], {"kind": "point", "layerId": "kapi", "attrs": attrs, "p": xy(d["at"])}))
    items.sort(key=lambda i: i[0])
    entities = [{**e, "id": n + 1} for n, (_, e) in enumerate(items)]
    millimetres = scene["perMetre"] == 1000.0
    settings: dict[str, Any] = {"srid": 0 if millimetres else 5254, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2",
                                "angleUnit": "grad", "plotScale": 1000, "workspace": "gis", "drawingFont": "barlow"}
    if millimetres:
        settings["drawingUnit"] = "mm"
    doc = {
        "format": "kentos.document",
        "version": 1,
        "name": "Adresler",
        "settings": settings,
        "origin": xy(scene["doors"][0]["at"]),
        "layers": [layer("il", "İl"), layer("ilce", "İlçe"), layer("mahalle", "Mahalle"), layer("yol", "Yol"), layer("kapi", "Kapı")],
        "activeLayer": "kapi",
        "entities": entities,
        "styles": {"items": [], "categories": []},
    }
    return cad.Document.from_bytes(json.dumps(doc, ensure_ascii=False).encode("utf-8"))


def texts(t: address.Texts) -> dict[str, str]:
    return {k: getattr(t, k) for k in ("street", "door", "neighbourhood", "district", "province", "postcode", "building", "code")}


class AddressTest(unittest.TestCase):
    def near(self, a: tuple[float, float], b: list[float], what: str) -> None:
        self.assertLessEqual(abs(a[0] - b[0]), 1e-6, what)
        self.assertLessEqual(abs(a[1] - b[1]), 1e-6, what)

    def same_found(self, found: address.Found, want: dict[str, Any], what: str) -> None:
        self.assertEqual(found.status, want["status"], what)
        self.assertEqual(len(found.candidates), len(want["candidates"]), what)
        for i, (c, w) in enumerate(zip(found.candidates, want["candidates"], strict=True)):
            at = f"{what} #{i}"
            self.assertEqual((c.kind, c.score, c.label), (w["kind"], w["score"], w["label"]), at)
            self.assertEqual(texts(c.texts), w["texts"], at)
            self.near(c.at, w["at"], at)

    def test_an_addresss_parts_are_the_references(self) -> None:
        for case in CASES["parses"]:
            p = address.parse(case["text"])
            want = case["parsed"]
            got = {"postcode": p.postcode, "street": p.street, "streetKind": p.street_kind, "door": p.door,
                   "neighbourhood": p.neighbourhood, "district": p.district, "province": p.province, "rest": p.rest,
                   "untagged": p.untagged}
            self.assertEqual(got, want, case["text"])

    def test_each_scene_answers_as_the_reference_answers(self) -> None:
        for s in CASES["scenes"]:
            with self.subTest(scene=s["name"]):
                doc = drawing(s["scene"])
                address.define(doc, LOCATOR)
                self.assertEqual([loc.id for loc in address.locators(doc)], ["adres-1"])
                self.assertFalse(doc.info().can_undo, "a project setting, not an undo step")
                loc = address.Locator(doc)
                checked = loc.check()
                c = s["counts"]
                self.assertEqual(checked.counts["doors"], c["doors"])
                self.assertEqual(checked.left_out["emptyStreets"], c["emptyStreets"])
                self.assertEqual(checked.left_out["unreadDoors"], c["unreadDoors"])
                for q in s["forward"]:
                    self.same_found(loc.locate(q["text"], min_score=s["minScore"]), q, f"{s['name']}: {q['text']}")
                for q in s["columns"]:
                    found = loc.locate(min_score=s["minScore"], **q["columns"])
                    self.same_found(found, q, f"{s['name']}: {q['columns']}")
                for q in s["reverse"]:
                    what = f"{s['name']}: {q['at']} within {q['reach']} m"
                    place = loc.reverse((q["at"][0], q["at"][1]), q["reach"])
                    want = q["place"]
                    if want is None:
                        self.assertIsNone(place, what)
                        continue
                    assert place is not None, what
                    self.assertEqual((place.kind, place.label), (want["kind"], want["label"]), what)
                    self.assertEqual(texts(place.texts), want["texts"], what)
                    self.near(place.at, want["at"], what)
                    if want["distance"] is None:
                        self.assertIsNone(place.distance, what)
                    else:
                        assert place.distance is not None, what
                        self.assertLessEqual(abs(place.distance - want["distance"]), 1e-9, what)

    def test_refusals_say_why(self) -> None:
        doc = drawing(CASES["scenes"][0]["scene"])
        with self.assertRaises(address.AddressError):
            address.Locator(doc)
        address.define(doc, LOCATOR)
        with self.assertRaises(address.AddressError):
            address.Locator(doc, "adres-9")
        loc = address.Locator(doc)
        with self.assertRaises(address.AddressError):
            loc.locate()
        with self.assertRaises(address.AddressError):
            loc.reverse((500000.0, 4400000.0), 0.0)
        address.remove(doc, "adres-1")
        self.assertEqual(address.locators(doc), [])


if __name__ == "__main__":
    unittest.main()
