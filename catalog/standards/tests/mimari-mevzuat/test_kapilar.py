"""Kaçış yolu kapıları kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kapilar.json, beklenenler
BYKHY 47'nin metninden elle) ve yangın sınıfının okunması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kapilar.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kapilar  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kapilar.json"


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]
        self.assertGreaterEqual(len(durumlar), 20)
        for durum in durumlar:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kapilar.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("BYKHY 47/"))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Sinif(unittest.TestCase):
    def test_ts_en_13501_2_yazimi_okunur(self):
        for metin, dakika, duman, kapanma in [("EI2 60-S200C5", 60, True, True), ("EI 60-Sa C", 60, True, True),
                                              ("EI 30", 30, False, False), ("E 90-Sm", 90, True, False),
                                              ("EI2 90-S200 C5", 90, True, True), ("EI 90 C", 90, False, True)]:
            with self.subTest(metin=metin):
                s = kapilar.sinif(metin)
                self.assertEqual((s["dakika"], s["duman"], s["kapanma"]), (dakika, duman, kapanma))
        for metin in ["60 dk", "", None, "I 60", "dayanıklı"]:
            with self.subTest(metin=metin):
                self.assertIsNone(kapilar.sinif(metin))


if __name__ == "__main__":
    unittest.main()
