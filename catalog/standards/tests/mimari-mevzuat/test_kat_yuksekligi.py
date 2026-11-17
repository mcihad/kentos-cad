"""Kat yüksekliği kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kat-yuksekligi.json, beklenenler
Planlı Alanlar İmar Yönetmeliği 28/1'in bentlerinden ve hâllerinden, Sivas 28/1'den, plan notlu durumlarda 69/3'ten
elle; docs/adr/8083, 8084, 8085).

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kat_yuksekligi.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kat_yuksekligi, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kat-yuksekligi.json"
KURALLAR = ("PAİY 28/1", "Sivas İmar Yön. 28/1", "Plan notu · ", "regulation_")


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 15)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kat_yuksekligi.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    if "uids" in e:
                        self.assertEqual(b["uids"], e["uids"], b["message"])
                    self.assertTrue(b["repair"], b["message"])
                    self.assertTrue(b["rule"] and b["rule"].startswith(KURALLAR), b["rule"])
                    if b["code"] != "regulation_text_unknown":
                        self.assertTrue(b["uids"], b["message"])
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])
                self.assertTrue(all(s["note"] for s in sonuc.satirlar), sonuc.satirlar)
                self.assertTrue(all("sayilar" not in s for s in sonuc.satirlar), sonuc.satirlar)


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satiri_verir(self):
        durum = durumlar()[1]
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("PAİY 28/1-c", [r["rule"] for r in cevap["rules"]])
        self.assertTrue(all(not k.startswith("_") for b in cevap["findings"] for k in b))


if __name__ == "__main__":
    unittest.main()
