"""Emsal sınırları kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/emsal.json, beklenenler
Planlı Alanlar İmar Yönetmeliği 22/1 ve 44/1'in metninden elle) ve ortak yüzün bulguları.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_emsal.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import emsal, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "emsal.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 20)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = emsal.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith(("PAİY 22/1-", "PAİY 20/8-j", "PAİY 5/8")))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_emsal_bulgusunu_ve_satirlarini_verir(self):
        durum = next(d for d in durumlar() if d["id"] == "kapici-30-konut")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        kodlar = [b["code"] for b in cevap["findings"]]
        self.assertIn("paiy_caretaker_not_required", kodlar)
        self.assertTrue(all(not k.startswith("_") for b in cevap["findings"] for k in b))
        maddeler = [r["rule"] for r in cevap["rules"]]
        for m in ("PAİY 22/1-ç", "PAİY 22/1-f", "PAİY 22/1-k", "PAİY 22/1-l"):
            self.assertIn(m, maddeler)

    def test_surumler_maddelerin_hallerini_tasir(self):
        for madde, gun, etiket in (("22/1-ç", "2019-07-25", "25.7.2019"), ("22/1-f", "2021-01-23", "23.1.2021"),
                                   ("22/1-k", "2019-07-25", "25.7.2019"), ("22/1-l", "2021-07-11", "11.7.2021")):
            with self.subTest(madde=madde):
                hal = y.haller("PAİY", madde)[0]
                self.assertEqual((hal.bas.isoformat(), hal.etiket), (gun, etiket))


if __name__ == "__main__":
    unittest.main()
