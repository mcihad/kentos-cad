"""Kullanım sınıfına özel kaçış hükümlerinin sınamaları (docs/adr/8107): deponun bağımsız durumları
(fixtures/standards/v1/kullanim.json, beklenenler BYKHY 49/1-a, 50/1-a, 50/1-b ve 52'nin metninden elle) ve kullanım
sınıfının okunması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kullanim.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kullanim  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kullanim.json"


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]
        self.assertGreaterEqual(len(durumlar), 20)
        for durum in durumlar:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kullanim.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["repair"] and b["rule"])
                    if b["code"] != "regulation_date_needed":
                        self.assertTrue(b["uids"] and b["rule"].startswith("BYKHY "), b)
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Siniflar(unittest.TestCase):
    def test_kullanim_bina_programindan_ya_da_sikica_mahal_adlarindan_okunur(self):
        for uses, beklenen in [(["Otel"], {"konaklama"}), (["Hastane"], {"saglik"}), (["Büro"], {"m52"}),
                               (["Ayakta tedavi merkezi"], {"m52"}), (["Konut"], set()), (["Öğrenci yurdu"], {"konaklama"}),
                               (["Huzurevi"], {"saglik"}), (["Depo"], {"m52"})]:
            with self.subTest(uses=uses):
                self.assertEqual(kullanim.siniflar({"uses": uses})[0], beklenen)
        # Programsız: otelin bürosu ve küçük deposu binayı 52'nin binası yapmaz (12/2, 16/2).
        otel = {"spaces": [{"name": "Otel odası 101"}, {"name": "Ofis"}, {"name": "Depo"}], "units": 1}
        self.assertEqual(kullanim.siniflar(otel)[0], {"konaklama"})
        self.assertEqual(kullanim.siniflar({"spaces": [{"name": "Depo"}], "units": 1})[0], set())

    def test_oda_ve_koridor_adlari(self):
        for ad in ["Oda 101", "Otel odası", "Yatak odası", "Süit", "Misafir odası", "Yatakhane"]:
            self.assertTrue(kullanim._otel_odasi(ad), ad)
        for ad in ["Resepsiyon", "Toplantı odası", "Koridor", "Ofis"]:
            self.assertFalse(kullanim._otel_odasi(ad), ad)
        for ad in ["Koridor", "Kat koridoru", "Kat holü", "Hol"]:
            self.assertTrue(kullanim._koridor(ad), ad)
        for ad in ["Hasta odası", "Koğuş", "Hasta yatak odası"]:
            self.assertTrue(kullanim._hasta_odasi(ad), ad)


if __name__ == "__main__":
    unittest.main()
