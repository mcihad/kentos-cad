"""Bina yüksekliği ve kat adedi kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/bina-yuksekligi.json,
beklenenler Planlı Alanlar İmar Yönetmeliği 5/11, 4/1-n, 4/1-ppp, 4/1-d ve 9/1-c'den elle; docs/adr/8098).

    python3 -I catalog/standards/tests/mimari-mevzuat/test_bina_yuksekligi.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import bina_yuksekligi, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "bina-yuksekligi.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 8)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = bina_yuksekligi.kontrol(olgu, gun, neden)
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
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("Plan notu · "))
            satirlar = {s["rule"]: s["state"] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])
                self.assertTrue(all(s["note"] and s["version"] is None and s["gazette"] for s in sonuc.satirlar))


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_planin_kat_adedini_okur(self):
        durum = next(d for d in durumlar() if d["id"] == "kat-adedi-asiyor")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("paiy_storey_count_over", [b["code"] for b in cevap["findings"]])
        self.assertIn("Plan notu: kat adedi", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
