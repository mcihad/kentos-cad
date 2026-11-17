"""Kaplamaların yanıcılığı kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/kaplamalar.json,
beklenenler BYKHY 4/ccc, 26/2, 26/4 ve Ek-2/Ç'den elle), Ek-2/Ç'nin okunması ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_kaplamalar.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import kaplamalar, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "kaplamalar.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 15)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = kaplamalar.kontrol(olgu, gun, neden)
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


class Ek2C(unittest.TestCase):
    def test_tablonun_satirlari(self):
        from datetime import date
        tablo = y.hal("BYKHY", "26/4", date(2026, 7, 2)).anahtar["ek2c"]
        for sinif, doseme, beklenen in [
            ("A1", False, "hic_yanmaz"), ("A2-s1,d0", False, "zor_yanici"), ("A2-s2,d0", False, "zor_alevlenici"),
            ("C-s3,d2", False, "zor_alevlenici"), ("D-s1,d0", False, "normal_alevlenici"), ("E-d2", False, "normal_alevlenici"),
            ("F", False, "kolay_alevlenici"), ("A1fl", True, "hic_yanmaz"), ("A2fl-s1", True, "zor_yanici"),
            ("Bfl-s1", True, "zor_alevlenici"), ("Cfl-s1", True, "zor_alevlenici"), ("A2fl-s2", True, "normal_alevlenici"),
            ("Dfl-s2", True, "normal_alevlenici"), ("Efl", True, "normal_alevlenici"), ("Ffl", True, "kolay_alevlenici"),
        ]:
            with self.subTest(sinif=sinif):
                self.assertEqual(kaplamalar.ozellik(sinif, doseme, tablo), (beklenen, None))


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari(self):
        durum = next(d for d in durumlar() if d["id"] == "tavan-normal")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("bykhy_ceiling_reaction", [b["code"] for b in cevap["findings"]])
        kurallar = [r["rule"] for r in cevap["rules"]]
        self.assertIn("BYKHY 26/2", kurallar)
        self.assertIn("BYKHY 26/4", kurallar)


if __name__ == "__main__":
    unittest.main()
