"""Asansör kuralının ve kural setinin sınamaları: deponun bağımsız durumları (fixtures/standards/v1/asansor.json,
beklenenler mevzuat metninden elle), hâllerin günleri ve tarihin okunması.

    python3 -I catalog/standards/tests/mimari-mevzuat/test_asansor.py
"""

import json
import sys
import unittest
from datetime import date
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "mimari-mevzuat"
sys.path.insert(0, str(PAKET / "src"))

from mimari_mevzuat import mimari  # noqa: E402
from mimari_mevzuat import yururluk as y  # noqa: E402

DEPO = PAKET.parents[2]
DURUMLAR = DEPO / "fixtures" / "standards" / "v1" / "asansor.json"
SAYILAR = ("units", "common", "annexes", "other", "technical", "inhabited", "elevators")


def olgular(durum: dict) -> dict:
    """Durumun kısa yazılmış katları olguların biçiminde."""
    katlar = []
    for kimlik, ad, kot, sayi in durum["building"]["storeys"]:
        katlar.append({"level": kimlik, "name": ad, "elevation": kot} | {k: sayi.get(k, 0) for k in SAYILAR})
    bina = dict(durum["building"], storeys=katlar)
    return {"building": bina, "permit": durum["permit"]}


class BagimsizDurumlar(unittest.TestCase):
    def test_her_durum_beklenen_sonucu_verir(self):
        durumlar = json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]
        self.assertGreaterEqual(len(durumlar), 20)
        for durum in durumlar:
            cevap = mimari.denetle(olgular(durum))
            bulgular = cevap["findings"]
            beklenen = durum["expect"]["findings"]
            with self.subTest(durum=durum["id"], ne="bulgular"):
                self.assertEqual([b["code"] for b in bulgular], [b["code"] for b in beklenen],
                                 f"{durum['neden']}\n{json.dumps(bulgular, ensure_ascii=False, indent=1)}")
            for b, e in zip(bulgular, beklenen):
                with self.subTest(durum=durum["id"], kod=e["code"]):
                    self.assertEqual(b.get("state"), e.get("state"), b["message"])
                    for parca in e.get("has", []):
                        self.assertIn(parca, b["message"])
                    if "rule" in e:
                        self.assertEqual(b["rule"], e["rule"])
            satirlar = {s["rule"]: s for s in cevap["rules"]}
            with self.subTest(durum=durum["id"], ne="satırlar"):
                self.assertEqual(sorted(satirlar), sorted(durum["expect"]["rules"]),
                                 json.dumps(cevap["rules"], ensure_ascii=False, indent=1))
            for madde, (sonuc, hal) in durum["expect"]["rules"].items():
                with self.subTest(durum=durum["id"], madde=madde):
                    self.assertEqual(satirlar[madde]["state"], sonuc, satirlar[madde]["note"])
                    self.assertEqual(satirlar[madde]["version"], hal)

    def test_kalan_bulgu_maddesini_ve_onarimini_soyler(self):
        for durum in json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]:
            for b in mimari.denetle(olgular(durum))["findings"]:
                with self.subTest(durum=durum["id"], kod=b["code"]):
                    self.assertTrue(b["repair"].strip())
                    self.assertTrue(b["rule"])
                    self.assertNotIn("_iz", b)


class KuralSeti(unittest.TestCase):
    def test_hallerin_gunleri_birbirini_izler(self):
        for madde in y.veri()["articles"]:
            haller = y.haller(madde["regulation"], madde["article"])
            for once, sonra in zip(haller, haller[1:]):
                with self.subTest(madde=f"{madde['regulation']} {madde['article']}", hal=sonra.kimlik):
                    self.assertEqual(once.bit, date.fromordinal(sonra.bas.toordinal() - 1))
            self.assertIsNone(haller[-1].bit, "son hâl yürürlükte")

    def test_her_halin_metni_ve_kaynagi_var(self):
        for madde in y.veri()["articles"]:
            self.assertTrue(madde["transition"].strip())
            for v in madde["versions"]:
                with self.subTest(madde=madde["article"], hal=v["id"]):
                    self.assertGreater(len(v["text"]), 80)
                    self.assertTrue(v["source"].strip())

    def test_gecis_gunu_yeni_hale_aittir(self):
        self.assertEqual(y.hal("PAİY", "34/1", date(2026, 6, 30)).kimlik, "2021")
        self.assertEqual(y.hal("PAİY", "34/1", date(2026, 7, 1)).kimlik, "2026")
        self.assertIsNone(y.hal("PAİY", "34/8", date(2021, 7, 10)))

    def test_tarih_yazimlari(self):
        for metin, gun in [("02.07.2026", date(2026, 7, 2)), ("2.7.2026", date(2026, 7, 2)),
                           ("02/07/2026", date(2026, 7, 2)), ("2026-07-02", date(2026, 7, 2))]:
            self.assertEqual(y.gun_degeri(metin), gun)
        for metin in ["31.02.2026", "2026", "7/2026", "dün"]:
            self.assertIsNone(y.gun_degeri(metin))

    def test_gun_yoksa_kapsanan_hallerin_ilk_gunleri_denenir(self):
        self.assertEqual(y.adaylar([("PAİY", "34/1"), ("PAİY", "34/8")]), [date(2021, 7, 11), date(2026, 7, 1)])
        self.assertEqual(y.adaylar([("PAİY", "34/1"), ("PAİY", "34/8"), ("Sivas", "34/1")]),
                         [date(2021, 7, 11), date(2022, 11, 10), date(2026, 7, 1)])


class EklentiYuzu(unittest.TestCase):
    """``kentos.ext`` üzerinden (bulut işi, Eklenti denetimi): aynı kural, durumu iletinin başında, kaldı hata."""

    def test_eklenti_denetimi_ayni_bulguyu_durumuyla_verir(self):
        sys.path.insert(0, str(DEPO / "python"))
        from kentos import ext
        from mimari_mevzuat import denetim

        durumlar = {d["id"]: d for d in json.loads(DURUMLAR.read_text(encoding="utf-8"))["cases"]}
        for kimlik, onek, agirlik in [("a-02-07-2026", "Bodrum kat", "error"),
                                      ("a-tarih-yok", "Eksik veri: Başvuru tarihi", "warning"),
                                      ("a-2019", "Uzman incelemesi: Başvuru günü", "info")]:
            olgu = olgular(durumlar[kimlik])
            facts = ext.GisAuditFacts({"layers": [], "cut": False, "project": {"tolerance": 0.0, "layers": [],
                                                                               "cut": False, **olgu}})
            bulgular = denetim.asansor_denetimi(facts)
            with self.subTest(durum=kimlik):
                self.assertEqual(len(bulgular), 1)
                self.assertTrue(bulgular[0].message.startswith(onek), bulgular[0].message)
                self.assertEqual(bulgular[0].severity, agirlik)
                self.assertIn("[", bulgular[0].message)


if __name__ == "__main__":
    unittest.main()
