"""Paftaların modelden üretildiği kuralın sınamaları (docs/adr/8065): her kural için bir geçen, bir kalan durum, elle
yazılmış olgularla.

    python3 -I catalog/standards/tests/dijital-proje/test_turetim.py
"""

import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import mimari, turetim  # noqa: E402


def kesit(durum, ad="B-B kesiti", isaret="m1", yer=1, tur="section"):
    return {"kind": tur, "name": ad, "mark": isaret, "state": durum, "inserts": yer}


def katman(ad, model=0, serbest=0):
    return {"id": ad.lower(), "name": ad, "objects": model + serbest, "hatches": 0, "first": [], "firstHatches": [],
            "model": model, "annotations": 0, "free": serbest, "firstFree": [f"{ad}-s{i}" for i in range(serbest)],
            "lineType": "continuous", "linePattern": None}


def kurallar(cad):
    return [b["rule"] for b in turetim.denetle({"cad": cad})]


class Turetim(unittest.TestCase):
    def test_eski_kesit_ve_gorunus(self):
        self.assertEqual(kurallar({"derived": [kesit("current"), kesit("notDrawn", yer=0)]}), [])
        b = turetim.denetle({"cad": {"derived": [kesit("stale"), kesit("stale", "Kuzey görünüşü", "m2", tur="elevation")]}})
        self.assertEqual([x["rule"] for x in b], ["derived_stale", "derived_stale"])
        self.assertEqual(b[0]["severity"], "error")
        self.assertEqual(b[0]["fix"]["input"], {"operation": {"kind": "refresh", "mark": "m1"}})
        self.assertIn("eski kesit basılır", b[0]["message"])
        self.assertIn("eski görünüş basılır", b[1]["message"])
        self.assertIn("EK-9 Tablo 9.1 satır 21 · PDF s. 239", b[0]["cite"])
        self.assertTrue(b[0]["message"].endswith("— Dijital Proje Yön. 9"))

    def test_kaynagi_silinmis_kesit(self):
        self.assertEqual(kurallar({"derived": [kesit("gone", isaret=None, yer=0)]}), [], "yerleşimi yoksa paftada değil")
        self.assertEqual(kurallar({"derived": [kesit("gone", isaret=None, yer=1)]}), ["derived_gone"])

    def test_model_katmaninda_serbest_cizim(self):
        self.assertEqual(kurallar({"layers": [katman("MM-DUVA-TUGLA", model=5), katman("MM-XXXX-DETAY", serbest=9)]}), [])
        b = turetim.denetle({"cad": {"layers": [katman("MM-DUVA-TUGLA", model=5, serbest=14)]}})
        self.assertEqual([x["rule"] for x in b], ["free_on_model_layer"])
        self.assertIn("14 serbest çizim", b[0]["message"])
        self.assertEqual(len(b[0]["uids"]), 14)

    def test_kopmus_bag(self):
        self.assertEqual(kurallar({"broken": []}), [])
        b = turetim.denetle({"cad": {"broken": [{"kind": "tag", "uid": "t"}, {"kind": "dimension", "uid": "d"}]}})
        self.assertEqual([(x["rule"], x["uids"]) for x in b], [("annotation_broken", ["t"]), ("annotation_broken", ["d"])])

    def test_mimari_iki_kurali_birlikte_verir(self):
        cad = {"derived": [kesit("stale")], "dimensionFonts": [{"font": "Barlow", "count": 1}]}
        self.assertEqual(sorted(b["rule"] for b in mimari.denetle({"cad": cad})), ["derived_stale", "dimension_font"])


if __name__ == "__main__":
    unittest.main()
