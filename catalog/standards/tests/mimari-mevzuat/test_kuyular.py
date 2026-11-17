"""Şaft ve asansör kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kuyular.json, beklenenler
BYKHY 4/ccc, 25/3, 41/9, 62/2, 62/4, 62/6, 62/7 ve Ek-3/B 12'nin metninden elle) ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kuyular.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kuyular, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kuyular.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 20)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kuyular.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("BYKHY "))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Okuma(unittest.TestCase):
    def test_yuksek_bina_iki_kosuldan_biriyle(self):
        hal = y.haller("BYKHY", "25/3")[0]
        for h, s, beklenen in [(21.5, 30.5, False), (21.51, 0.0, True), (0.0, 30.51, True), (12.0, 13.0, False)]:
            with self.subTest(bina=h, yapi=s):
                self.assertEqual(kuyular.yuksek({"height": h, "structureHeight": s}, hal)[0], beklenen)

    def test_kat_kapisinin_satiri_9_7_2015_ten(self):
        from datetime import date
        self.assertIsNone(y.hal("BYKHY", "62/7", date(2015, 7, 8)))
        self.assertEqual(y.hal("BYKHY", "62/7", date(2015, 7, 9)).anahtar["high"], 60)


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari_ve_bulgusu(self):
        durum = next(d for d in durumlar() if d["id"] == "asansor-ofise-aciliyor")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("bykhy_elevator_door_room", [b["code"] for b in cevap["findings"]])
        kurallar = [r["rule"] for r in cevap["rules"]]
        for m in ("25/3", "41/9", "62/2", "62/4", "62/6", "62/7"):
            self.assertIn(f"BYKHY {m}", kurallar)


if __name__ == "__main__":
    unittest.main()
