"""Otel ve yurt yatak odasının kapısından kaçış uzaklığının sınamaları (docs/adr/8156): deponun bağımsız durumları
(fixtures/standards/v1/yatak-kacisi.json, beklenenler BYKHY 50/4, 50/5 ve Ek-5/B'den elle) ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_yatak_kacisi.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import mimari, yatak_kacisi  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "yatak-kacisi.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 20)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = yatak_kacisi.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("BYKHY 50/4 · RG "), b)
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Okuma(unittest.TestCase):
    def test_yatak_odasi_adlari(self):
        for ad in ("Oda 101", "Otel odası 12", "Süit 301", "Yatak odası", "Konuk odası", "Öğrenci odası (4)",
                   "Yatakhane"):
            self.assertTrue(yatak_kacisi.yatak_odasi(ad), ad)
        for ad in ("Süit 301 banyosu", "Kat holü", "Koridor", "Toplantı odası", "Lobi", "Etüt odası", ""):
            self.assertFalse(yatak_kacisi.yatak_odasi(ad), ad)


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari(self):
        durum = next(d for d in durumlar() if d["id"] == "otel-tek-yon-asar")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("bykhy_bedroom_escape_distance", [b["code"] for b in cevap["findings"]])
        self.assertIn("BYKHY 50/4", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
