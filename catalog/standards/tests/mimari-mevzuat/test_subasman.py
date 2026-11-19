"""Subasman kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/subasman.json, beklenenler PAİY
4/1-yyy'den ve planın aksine hükmünden elle, docs/adr/8153, 8158) ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_subasman.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import mimari, subasman  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "subasman.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 14)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = subasman.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    # Binanın kendi notunun kuralı binanın adıyla başlar (docs/adr/8163).
                    madde = b["rule"].startswith(("BYKHY ", "PAİY ", "Plan notu · ")) or " plan notu · " in b["rule"]
                    self.assertTrue(b["uids"] and b["repair"] and madde, b)
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_satirlari(self):
        durum = next(d for d in durumlar() if d["id"] == "zemin-150")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("paiy_plinth_level", [b["code"] for b in cevap["findings"]])
        self.assertIn("PAİY 4/1-yyy", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
