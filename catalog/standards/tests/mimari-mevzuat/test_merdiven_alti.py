"""Merdiven altı kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/merdiven-alti.json, beklenenler
Planlı Alanlar İmar Yönetmeliği 31/6'nın metninden elle) ve kapalı mahal adlarının okunması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_merdiven_alti.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import merdiven_alti, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "merdiven-alti.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 15)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = merdiven_alti.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("PAİY 31/6"))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Adlar(unittest.TestCase):
    def test_kapali_merdiven_alti_adlari(self):
        for ad in ["Merdiven altı", "merdiven altı deposu", "MERDİVEN ALTI", "Dolap", "Gömme dolap", "Depo", "Kiler",
                   "Ardiye", "Vestiyer"]:
            with self.subTest(ad=ad):
                self.assertTrue(merdiven_alti.kapali(ad))
        for ad in ["Salon", "Hol", "Merdiven evi", "Merdiven", "Altı", "", "Yatak odası"]:
            with self.subTest(ad=ad):
                self.assertFalse(merdiven_alti.kapali(ad))


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_merdiven_alti_satirini_verir(self):
        durum = next(d for d in durumlar() if d["id"] == "salona-acik")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("paiy_under_stair_open", [b["code"] for b in cevap["findings"]])
        self.assertIn("PAİY 31/6", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
