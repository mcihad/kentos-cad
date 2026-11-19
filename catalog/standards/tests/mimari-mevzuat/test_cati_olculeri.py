"""Çatının ölçülerinin sınamaları (docs/adr/8159): deponun bağımsız durumları (fixtures/standards/v1/cati-olculeri.json,
beklenenler PAİY 40/4, 40/5, 42/1, BYKHY 58/2 ve Sivas 40/5-e, 40/5-g, 42/5'ten elle), düzeltmeler ve ortak yüz.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_cati_olculeri.py
"""

import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import cati_olculeri, mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "cati-olculeri.json"
CATI = "00000000-0000-4000-8000-000000000801"


def durumlar() -> list[dict]:
    return json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        hepsi = durumlar()
        self.assertGreaterEqual(len(hepsi), 16)
        for durum in hepsi:
            olgu = {"building": durum["building"], "permit": durum["permit"]}
            gun, neden = y.basvuru(durum["permit"])
            sonuc = cati_olculeri.kontrol(olgu, gun, neden)
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in sonuc.bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(sonuc.bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(sonuc.bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    # Çatının bulgusu çatıyı, çatı örtüsünü aşan duvarınki duvarı seçer (docs/adr/8165).
                    self.assertEqual(b["uids"], e.get("uids", [CATI]))
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    self.assertTrue(b["repair"] and b["rule"] and len(b["at"]) == 2, b)
                    self.duzeltme(b, e.get("fix"), durum["building"])
            satirlar = {s["rule"]: [s["state"], s["version"]] for s in sonuc.satirlar}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(satirlar, durum["expect"]["rules"], durum["neden"])

    def duzeltme(self, bulgu: dict, beklenen: dict | None, bina: dict) -> None:
        """Bulgunun düzeltmesi: ``cad.arch.edit``'in çatı üstündeki işlemi; parçalar çizilen değerleriyle, yalnız
        düzeltilen değişir."""
        if beklenen is None:
            self.assertNotIn("fix", bulgu)
            return
        fix = bulgu["fix"]
        self.assertEqual((fix["label"], fix["command"], fix["input"]["uids"]),
                         (beklenen["label"], "cad.arch.edit", [CATI]))
        islem = fix["input"]["operation"]
        self.assertEqual(islem["kind"], "set")
        if "rise" in beklenen:
            i, deger = beklenen["rise"]
            ciz = bina["roofs"][0]["form"]["parts"]
            self.assertEqual(len(islem["parts"]), len(ciz))
            for k, (p, q) in enumerate(zip(islem["parts"], ciz)):
                self.assertEqual((p["kind"], p["at"], p["width"], p["depth"]),
                                 (q["kind"], {"x": q["at"][0], "y": q["at"][1]}, q["width"], q["depth"]))
                self.assertEqual(p.get("rise"), deger if k == i else q.get("rise"))
        if "slopes" in beklenen:
            self.assertEqual(islem["slopes"], beklenen["slopes"])
        if "snowGuard" in beklenen:
            self.assertEqual(islem["snowGuard"], beklenen["snowGuard"])


class Bicim(unittest.TestCase):
    def test_egim_onda_bire_yuvarlanir(self):
        self.assertEqual([cati_olculeri.egim(v) for v in (75.0, 40.0, 42.5, 42.46, 42.45)],
                         ["%75", "%40", "%42,5", "%42,5", "%42,5"])


class OrtakYuz(unittest.TestCase):
    def test_mimari_denetim_bulgunun_yerini_ve_duzeltmesini_verir(self):
        durum = next(d for d in durumlar() if d["id"] == "hava-bacasi")
        cevap = mimari.denetle({"building": durum["building"], "permit": durum["permit"]})
        bulgu = next(b for b in cevap["findings"] if b["code"] == "paiy_roof_vent_high")
        self.assertEqual(bulgu["at"], [10.0, 8.0])
        self.assertEqual(bulgu["fix"]["command"], "cad.arch.edit")
        self.assertNotIn("_iz", bulgu)
        self.assertIn("PAİY 40/5", [r["rule"] for r in cevap["rules"]])


if __name__ == "__main__":
    unittest.main()
