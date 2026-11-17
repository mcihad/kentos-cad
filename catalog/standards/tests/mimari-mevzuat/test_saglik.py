"""Sağlık yapılarının sınamaları (docs/adr/8110): deponun bağımsız durumları (fixtures/standards/v1/saglik.json,
beklenenler BYKHY 49/1-b'nin ve 49/2'nin metninden elle).

    python3 -I catalog/standards/tests/mimari-mevzuat/test_saglik.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import saglik  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "saglik.json"


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]
        self.assertGreaterEqual(len(durumlar), 20)
        for durum in durumlar:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = saglik.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["repair"] and b["rule"].startswith("BYKHY 49/"), b)
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


if __name__ == "__main__":
    unittest.main()
