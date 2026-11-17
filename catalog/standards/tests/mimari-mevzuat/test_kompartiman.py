"""Yangın kompartımanı kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kompartiman.json,
beklenenler BYKHY 4/tt, 24 ve Ek-4'ün metninden elle), sınıfın okunması ve kompartımanların kurulması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kompartiman.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kompartiman, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402
from mimari_mevzuat.yangin_sinifi import ayirir  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kompartiman.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 20)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kompartiman.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("BYKHY 24/"))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Okuma(unittest.TestCase):
    def test_kompartimani_ayiran_sinif(self):
        for metin, beklenen in [("REI 60", True), ("EI 60", True), ("EI2 60-S200C5", True), ("REI 120", True),
                                ("EI 30", False), ("E 60", False), ("EW 60", False), ("R 90", None), ("", None),
                                (None, None), ("60 dk", None)]:
            with self.subTest(metin=metin):
                self.assertEqual(ayirir(metin, 60), beklenen)

    def test_tehlikeli_alan_adlari(self):
        for ad in ["Kazan dairesi", "KAZAN DAİRESİ", "Otopark", "Kapalı otopark", "Elektrik odası", "Trafo merkezi",
                   "Orta gerilim merkezi", "Jeneratör odası", "Garaj"]:
            with self.subTest(ad=ad):
                self.assertTrue(kompartiman.tehlikeli(ad))
        for ad in ["Koridor", "Ofis", "Orta salon", "Depo", ""]:
            with self.subTest(ad=ad):
                self.assertFalse(kompartiman.tehlikeli(ad))

    def test_ayrimsiz_mahaller_bir_kompartimandir(self):
        a, b, c = "a", "b", "c"
        bina = {"spaces": [{"uid": x, "area": 1.0} for x in (a, b, c)],
                "separations": [{"rooms": [a, b], "open": 1.0},
                                {"rooms": [b, c], "open": 0.0, "walls": [{"uid": "w", "kind": "wall", "fireRating": "REI 60", "length": 3.0}]}]}
        self.assertEqual(kompartiman.kompartimanlar(bina, 60), [[a, b], [c]])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_kompartiman_satirlarini_verir(self):
        durum = next(d for d in durumlar() if d["id"] == "buro-asan")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("bykhy_compartment_area_over", [b["code"] for b in cevap["findings"]])
        for m in ("BYKHY 24/2", "BYKHY 24/6", "BYKHY 24/7"):
            self.assertIn(m, [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
