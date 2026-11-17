"""Yangına dayanım süreleri kuralının sınamaları: deponun bağımsız durumları (fixtures/standards/v1/dayanim.json,
beklenenler BYKHY Ek-3/B ve Ek-3/C'nin tablolarından elle) ve tablonun hücrelerinin okunması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_dayanim.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import dayanim, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "dayanim.json"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 18)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = dayanim.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    self.assertTrue(b["uids"] and b["repair"] and b["rule"].startswith("BYKHY Ek-3"))
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])


class Tablo(unittest.TestCase):
    def test_hucreler(self):
        self.assertEqual(dayanim.hucre("120(3)"), {"dakika": 120, "notlar": [3]})
        self.assertEqual(dayanim.hucre("15(2)(4)"), {"dakika": 15, "notlar": [2, 4]})
        self.assertEqual(dayanim.hucre("60"), {"dakika": 60, "notlar": []})
        self.assertEqual(dayanim.hucre("---"), {"yok": True})
        self.assertEqual(dayanim.hucre("İzin verilmez"), {"izinsiz": True})

    def test_tablonun_her_hucresi_okunur(self):
        hal = y.haller("BYKHY", "Ek-3")[0]
        for satir in hal.anahtar["rows"]:
            for dizi in (satir.get("all"), satir.get("dry"), satir.get("wet")):
                for h in dizi or []:
                    with self.subTest(satir=satir["id"], hucre=h):
                        dayanim.hucre(h)


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_ek3_satirini_verir(self):
        durum = next(d for d in durumlar() if d["id"] == "kolon-yetmiyor")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        self.assertIn("bykhy_element_resistance", [b["code"] for b in cevap["findings"]])
        self.assertIn("BYKHY Ek-3", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
