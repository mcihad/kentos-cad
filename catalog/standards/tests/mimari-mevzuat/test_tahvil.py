"""Bina yüksekliği ile kat adedinin tahvilinin sınamaları (docs/adr/8157): deponun bağımsız durumları
(fixtures/standards/v1/tahvil.json, beklenenler PAİY 9/1-c, 9/1-a, 28/1 ve Sivas 28/1-c'den elle) ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_tahvil.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import mimari, tahvil  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "tahvil.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 20)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = tahvil.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["repair"], b)
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Hesap(unittest.TestCase):
    def test_tahvilin_iki_yonu(self):
        bina = next(d for d in durumlar() if d["id"] == "yukseklik-2-kat")["building"]
        gun = y.gun_degeri("02.07.2026")
        self.assertEqual([tahvil.yukseklik(bina, n, gun, 1.2) for n in (1, 2, 3, 4)], [5.2, 9.2, 13.2, 15.6])
        self.assertEqual([tahvil.kat_adedi(bina, h, gun, 1.2) for h in (5.19, 5.2, 9.5, 13.2, 15.6)], [0, 1, 2, 3, 4])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari(self):
        durum = next(d for d in durumlar() if d["id"] == "yukseklik-3-kat")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("paiy_height_storeys_conversion", [b["code"] for b in cevap["findings"]])
        self.assertIn("PAİY 9/1-c", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
