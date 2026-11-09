"""Koordinat dönüştür's decision point by point (`cad.crs.transform`'s `flagged`, docs/adr/3700; plan/map-cad.md MAP-02),
held by an independent reading: which areas of use a point lies beyond is worked out here from the places' own
latitudes and longitudes and EPSG's boxes (fixtures/crs/v1/areas.json, PROJ's database), with Python's own box test.

Four places (Bursa, Eskişehir, Ankara, Edirne) are written in TUREF / TM30 by the command itself from their degrees.
Then:

- to TUREF / TM33 (one datum, no operation) a place is beyond the source's area when TM30's box does not hold it and
  beyond the target's when TM33's does not; every place is converted all the same;
- to WGS 84 the target is the world and the operation TUREF → WGS 84 (EPSG:5261) holds Türkiye, its box PROJ's (as
  crs_vertical_cases.py asserts): only the source's notes stay;
- a place within every area gets no note, and the warning counts the flagged.
"""

from __future__ import annotations

import json
import sys
import unittest
import warnings
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
from kentos import cad
from kentos.cad.types import CrsPoint, CrsSide


BOXES = {a["srid"]: (a["west"], a["south"], a["east"], a["north"])
         for a in json.loads((ROOT / "fixtures/crs/v1/areas.json").read_text("utf-8"))["areas"]}
# EPSG:5261's area of use, PROJ's (crs_vertical_cases.py's TURKIYE).
TURKIYE = (25.62, 34.42, 44.83, 43.45)
PLACES = {"Bursa": (40.1826, 29.0665), "Eskişehir": (39.7767, 30.5206), "Ankara": (39.9334, 32.8597),
          "Edirne": (41.6772, 26.5557)}


def holds(box, lon, lat):
    west, south, east, north = box
    return west <= lon <= east and south <= lat <= north


class PointNotes(unittest.TestCase):
    def setUp(self) -> None:
        setup = json.loads((ROOT / "fixtures/commands/v1/cad.crs.transform.json").read_text("utf-8"))["setup"]
        self.doc = cad.Document.from_bytes(json.dumps(setup).encode("utf-8"))
        degrees = [CrsPoint(x=lon, y=lat) for lat, lon in PLACES.values()]
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")
            tm30 = cad.crs.transform(self.doc, points=degrees, from_=CrsSide(srid=5252), to=CrsSide(srid=5254))
        self.tm30 = [CrsPoint(x=p.x, y=p.y) for p in tm30.points]

    def flagged(self, to: int) -> tuple[list[dict], list[str]]:
        with warnings.catch_warnings(record=True) as said:
            warnings.simplefilter("always")
            r = cad.crs.transform(self.doc, points=self.tm30, to=CrsSide(srid=to))
        self.assertTrue(all(p is not None for p in r.points), "every place converted")
        return [f.to_json() for f in r.flagged] if r.flagged else [], [str(w.message) for w in said]

    def expected(self, boxes: tuple) -> list[dict]:
        names = ("outside_source_area", "outside_operation_area", "outside_target_area")
        out = []
        for i, (lat, lon) in enumerate(PLACES.values()):
            notes = [n for n, b in zip(names, boxes) if b is not None and not holds(b, lon, lat)]
            if notes:
                out.append({"index": i, "notes": notes})
        return out

    def test_to_tm33_the_source_s_and_the_target_s_areas(self) -> None:
        got, said = self.flagged(5255)
        self.assertEqual(got, self.expected((BOXES[5254], None, BOXES[5255])))
        self.assertTrue(any("outside_area" in s for s in said), said)

    def test_to_wgs84_only_the_source_s(self) -> None:
        got, _ = self.flagged(4326)
        self.assertEqual(got, self.expected((BOXES[5254], TURKIYE, BOXES[4326])))
        self.assertEqual([f["index"] for f in got], [2, 3], "Ankara and Edirne lie beyond TM30's band")


if __name__ == "__main__":
    unittest.main()
