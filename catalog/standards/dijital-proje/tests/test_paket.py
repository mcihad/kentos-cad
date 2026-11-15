"""2B ruhsat paketi kuralının sınamaları (docs/adr/8066): her kural için bir geçen, bir kalan durum.

    python3 -I catalog/standards/dijital-proje/tests/test_paket.py
"""

import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import paket, ruhsat  # noqa: E402

AD = "000000_00_MM_XXXX_PL_MMC_K00_01_000.pdf"


def ornek(**k):
    p = {"pdfa": True, "mode": "separate", "files": [{"name": AD, "sheets": ["s1"]}],
         "sheets": [{"id": "s1", "number": "UP-01", "name": "Zemin kat planı", "paper": [420.0, 297.0]}]}
    p.update(k)
    return {"package": p}


def kurallar(facts):
    return [b["rule"] for b in paket.denetle(facts)]


class Paket(unittest.TestCase):
    def test_gecen_paket(self):
        self.assertEqual(kurallar(ornek()), [])
        self.assertEqual(kurallar({}), [], "paketsiz olgu")

    def test_pdfa(self):
        b = paket.denetle(ornek(pdfa=False))
        self.assertEqual([x["rule"] for x in b], ["package_pdfa"])
        self.assertTrue(b[0]["message"].endswith("— Dijital Proje Yön. 4/2"))

    def test_bir_dosya_bir_pafta(self):
        tek = ornek(mode="single", files=[{"name": AD, "sheets": ["s1", "s2"]}],
                    sheets=[{"id": i, "number": i, "name": i, "paper": [420.0, 297.0]} for i in ("s1", "s2")])
        b = paket.denetle(tek)
        self.assertEqual([x["rule"] for x in b], ["package_one_sheet"])
        self.assertIn("2 pafta var", b[0]["message"])

    def test_kagit(self):
        for w, h in ((210, 297), (297, 210), (594, 420), (841, 594), (1189, 841), (891, 420)):
            with self.subTest(kagit=(w, h)):
                self.assertEqual(kurallar(ornek(sheets=[{"id": "s1", "number": "UP-01", "name": "", "paper": [w, h]}])), [])
        for w, h in ((500, 300), (148, 210), (430, 297)):
            with self.subTest(kagit=(w, h)):
                self.assertEqual(kurallar(ornek(sheets=[{"id": "s1", "number": "UP-01", "name": "", "paper": [w, h]}])),
                                 ["package_paper"])

    def test_ad(self):
        for ad in ("001-UP-01 Zemin kat planı.pdf", "000000_00_MM_XXXX_PL_MMC_K00_1_000.pdf",
                   "000000_00_QQ_XXXX_PL_MMC_K00_01_000.pdf", "000000_00_MM_XXXX_ZZ_MMC_K00_01_000.pdf",
                   "000000_00_MM_XXXX_PL_ZZZ_K00_01_000.pdf", "000000_00_mm_XXXX_PL_MMC_K00_01_000.pdf"):
            with self.subTest(ad=ad):
                self.assertEqual(kurallar(ornek(files=[{"name": ad, "sheets": ["s1"]}])), ["package_name"])
        b = paket.denetle(ornek(files=[{"name": "001-UP-01.pdf", "sheets": ["s1"]}]))
        self.assertIn("9 alan alt tireyle", b[0]["message"])
        self.assertIn("EK-2 Tablo 2.1 · PDF s. 3", b[0]["cite"])

    def test_ruhsat_iki_kurali_birlikte_verir(self):
        olgu = {**ornek(pdfa=False), "cad": {"derived": [{"kind": "section", "name": "B-B kesiti", "mark": "m",
                                                          "state": "stale", "inserts": 1}]}}
        self.assertEqual(sorted(b["rule"] for b in ruhsat.denetle(olgu)), ["derived_stale", "package_pdfa"])


if __name__ == "__main__":
    unittest.main()
