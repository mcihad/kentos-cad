"""İmar denetiminin sembol uyuşmazlığı ve hata katmanlarının GeoPackage'a yazılması (docs/adr/4640; plan/map-cad.md
MAP-09), held by the test's own rules (no KentOS code in the expectations).

1. `cad.zoning.check`'s `symbolMismatch` on the shared case's drawing (fixtures/commands/v1/cad.zoning.check.json,
   “sembol kararı göstermiyor …”): the test finds a use's symbol on its own, as ADR 4240 writes the rule (the one fill
   symbol of the level's MPYY branch in the library whose name is the use's label, Turkish lower case, “î” as “i”, a
   trailing “ (…)” dropped): S1, a trade area drawn with the park's symbol, is the only finding; the park with its own,
   the housing area (no such symbol) and the area without a symbol are not judged.
2. The error layers of İmar planını denetle and Topolojiyi kurallarla denetle written to a GeoPackage through the run's
   sink (İşlem's Sonucun yeri): the drawing does not change; the file, read by this test with sqlite3 and its own
   reading of the GeoPackage geometry blob (OGC 12-128, 2.1.3: “GP”, version, flags, srs_id, envelope, then WKB), holds
   one table each: the zoning findings' regions as polygons (areas by the test's own shoelace equal to the shared
   case's regions), the topology findings as points with their keys.
"""

from __future__ import annotations

import json
import sqlite3
import struct
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts/fixtures"))

import kentos.cad as cad  # noqa: E402

import kcad_v2_reference as kr  # noqa: E402

LIBRARY = json.loads((ROOT / "crates/native/style/assets/system-library.kstil").read_text("utf-8"))["items"]
PROFILE = json.loads((ROOT / "catalog/v1/profiles/imar-plani.json").read_text("utf-8"))
USES = {v["code"]: v["label"] for v in next(d for d in PROFILE["domains"] if d["id"] == "plan-kullanim")["values"]}


def alike(name: str) -> str:
    name = name.replace("İ", "i").replace("I", "ı").lower().replace("î", "i")
    return name[: -len(" (…)")].strip() if name.endswith(" (…)") else name.strip()


def own_symbol(branch: str, use: str) -> str | None:
    same = [i["id"] for i in LIBRARY if i.get("kind") == "symbol" and i["path"][:2] == ["MPYY", branch]
            and (i.get("symbol") or {}).get("type") == "fill" and alike(i["name"]) == alike(USES[use])]
    return same[0] if len(same) == 1 else None


def case_setup(file: str, prefix: str) -> dict:
    cases = json.loads((ROOT / "fixtures/commands/v1" / file).read_text("utf-8"))["cases"]
    return next(c for c in cases if c["name"].startswith(prefix))


def opened(setup: dict) -> cad.Document:
    return cad.Document.from_bytes(kr.container(kr.document(setup)))


def geometry(blob: bytes) -> tuple[int, int, list]:
    """A GeoPackage geometry blob read here: (srs_id, WKB type, rings or point)."""
    assert blob[:2] == b"GP", blob[:2]
    flags = blob[3]
    order = "<" if flags & 1 else ">"
    srs = struct.unpack(order + "i", blob[4:8])[0]
    envelope = {0: 0, 1: 32, 2: 48, 3: 48, 4: 64}[(flags >> 1) & 7]
    wkb = blob[8 + envelope:]
    o = "<" if wkb[0] == 1 else ">"
    kind = struct.unpack(o + "I", wkb[1:5])[0]
    if kind == 1:
        return srs, kind, list(struct.unpack(o + "dd", wkb[5:21]))
    assert kind == 3, kind
    rings, at = [], 9
    for _ in range(struct.unpack(o + "I", wkb[5:9])[0]):
        n = struct.unpack(o + "I", wkb[at:at + 4])[0]
        at += 4
        rings.append([struct.unpack(o + "dd", wkb[at + 16 * k:at + 16 * k + 16]) for k in range(n)])
        at += 16 * n
    return srs, kind, rings


def shoelace(pts: list) -> float:
    ox, oy = pts[0][0], pts[0][1]
    rel = [(x - ox, y - oy) for x, y in pts]
    return abs(sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(rel, rel[1:] + rel[:1]))) / 2.0


class SymbolMismatch(unittest.TestCase):
    def test_only_the_trade_area_with_the_park_s_symbol_is_found(self) -> None:
        case = case_setup("cad.zoning.check.json", "sembol kararı göstermiyor")
        setup = case["setup"]
        doc = opened(setup)
        found = [f for f in cad.zoning.check(doc).to_json()["findings"] if f["kind"] == "symbolMismatch"]
        # The test's own judgement of every symbolled decision area of the drawing.
        plans = {e["attrs"]["PlanKimlik"]: e["attrs"]["Seviye"] for e in setup["entities"] if e["layerId"] == "plan"}
        branch = {"uip": "Uygulama imar planı", "nip": "Nazım imar planı"}
        want = []
        for e, u in zip(setup["entities"], setup["uids"]):
            if e["layerId"] != "karar" or not e.get("symbol"):
                continue
            mine = own_symbol(branch[plans[e["attrs"]["Plan"]]], e["attrs"]["Kullanim"])
            if mine is not None and mine != e["symbol"]:
                want.append(u)
        self.assertEqual([f["uids"] for f in found], [[u] for u in want])
        self.assertEqual(len(want), 1)
        self.assertIn("“Park”", found[0]["message"])
        self.assertIn("“Ticaret alanı” sembolünü verir", found[0]["message"])


class ErrorLayersToGeoPackage(unittest.TestCase):
    def run_to_file(self, setup: dict, tool: str, layer: str) -> tuple[cad.Document, int, Path]:
        doc = opened(setup)
        before = len(list(doc.entities()))
        folder = Path(tempfile.mkdtemp(prefix="kentos-gpkg-"))
        path = folder / "bulgular.gpkg"
        ran = cad.processing.run(doc, tool=tool, parameters={"write": True, "layer": {"newName": layer}},
                                 sink=cad.RunSink.from_json({"path": str(path)}))
        self.assertIsNotNone(ran.sink)
        self.assertEqual(len(list(doc.entities())), before, "the drawing does not change")
        return doc, int(setup["settings"]["srid"]), path

    def test_the_zoning_regions_are_polygons_in_the_file(self) -> None:
        setup = json.loads((ROOT / "fixtures/processing/v1/zoning-yollar.kcad").read_text("utf-8"))
        _, srid, path = self.run_to_file(setup, "zoning.check", "İmar bulguları")
        db = sqlite3.connect(path)
        self.assertEqual(db.execute("select table_name, data_type, srs_id from gpkg_contents").fetchall(),
                         [("İmar bulguları", "features", srid)])
        rows = db.execute('select geom, "Bölge", "İleti" from "İmar bulguları" order by fid').fetchall()
        findings = case_setup("cad.zoning.check.json",
                              "yol genişliği çizilenden geniş: yola giren kararlar ve parseller")["steps"][0]["result"]
        regions = [(f, k, r) for f in findings["output"]["findings"] for k, r in enumerate(f["regions"])]
        self.assertEqual(len(rows), len(regions))
        for (blob, part, message), (f, k, r) in zip(rows, regions):
            srs, kind, rings = geometry(blob)
            self.assertEqual((srs, kind), (srid, 3))
            self.assertEqual((part, message), (f"{k + 1}/{len(f['regions'])}", f["message"]))
            outer = [(p["x"], p["y"]) for p in r["outer"]["pts"]]
            self.assertAlmostEqual(shoelace(rings[0][:-1] if rings[0][0] == rings[0][-1] else rings[0]),
                                   shoelace(outer), places=6)
        db.close()

    def test_the_topology_findings_are_points_in_the_file(self) -> None:
        setup = json.loads((ROOT / "fixtures/processing/v1/topology.kcad").read_text("utf-8"))
        doc, srid, path = self.run_to_file(setup, "topology.check", "Topoloji bulguları")
        db = sqlite3.connect(path)
        self.assertEqual(db.execute("select table_name, data_type, srs_id from gpkg_contents").fetchall(),
                         [("Topoloji bulguları", "features", srid)])
        rows = db.execute('select geom, "Anahtar" from "Topoloji bulguları" order by fid').fetchall()
        keys = [f["key"] for f in cad.topology.check(doc).to_json()["findings"]]
        self.assertEqual(sorted(k for _, k in rows), sorted(keys))
        for blob, _ in rows:
            srs, kind, _ = geometry(blob)
            self.assertEqual((srs, kind), (srid, 1))
        db.close()


if __name__ == "__main__":
    unittest.main()
