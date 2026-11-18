"""Komşu binaya bitişik duvar kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/bitisik.json, beklenenler
PAİY 39/4, BYKHY 25/1 ve 25/2'den elle; scripts/fixtures/party_wall_cases.py) ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_bitisik.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import bitisik, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "bitisik.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 15)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = bitisik.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith(("BYKHY ", "PAİY ")))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari(self):
        durum = next(d for d in durumlar() if d["id"] == "pencere-sinifsiz")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("paiy_party_wall_opening", [b["code"] for b in cevap["findings"]])
        kurallar = [r["rule"] for r in cevap["rules"]]
        for madde in ("PAİY 39/4", "BYKHY 25/1", "BYKHY 25/2"):
            self.assertIn(madde, kurallar)


if __name__ == "__main__":
    unittest.main()
