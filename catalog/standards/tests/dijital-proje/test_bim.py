"""BIM ruhsat paketinin kararı (docs/adr/8075): paftaların ve paketin kuralı, formun kalan satırları ve müellifin
cevapları birlikte; beş sonuç ayrı. Olgular KentOS'un dosyalarınınkidir (fixtures/ifc/v1/olgular/), teslim edilen dosya
gibi (``export.memory`` yanlış) ve kaynağın beklentisiyle.

    python3 -I catalog/standards/tests/dijital-proje/test_bim.py
"""

import copy
import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
KOK = PAKET.parents[2]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import bim, form  # noqa: E402

AD = "260123_00_MM_XXXX_PL_MMC_K00_01_000.pdf"
CIZIM = {"package": {"pdfa": True, "mode": "separate", "files": [{"name": AD, "sheets": ["s1"]}],
                     "sheets": [{"id": "s1", "number": "UP-01", "name": "Zemin kat planı", "paper": [420.0, 297.0]}]}}


def teslim(ad):
    """Teslim edilen dosyanın olguları: adı başlığındaki, kaynağın beklentisi dosyanın kendi nesneleri, kayıp yok."""
    f = json.loads((KOK / "fixtures/ifc/v1/olgular" / f"{ad}.json").read_text(encoding="utf-8"))
    f["file"]["name"] = f["file"]["header"]["name"]
    f["expected"] = [{"uid": o["globalId"], "globalId": o["globalId"], "class": o["class"]} for o in f["objects"]]
    f["export"] = {"left": [], "memory": False}
    return f


def tam(f):
    """Formun kalan satırları giderilmiş olgular: zorunlu alanlar ve bağlamın kuzeyi dolu."""
    g = copy.deepcopy(f)
    g["gaps"] = []
    for k in g["records"].get("IfcGeometricRepresentationContext") or []:
        k["attributes"]["TrueNorth"] = "#99"
    return g


#: Formun incelemeye bıraktığı sorular (dosya boyutu, emsal tablosu). Gelişim seviyesi (7) kaynağı verilen
#: modelde gövdelerden cevaplanır (docs/adr/8133).
INCELEME = [1, 10]


def cevaplar(yes=True):
    return [{"no": n, "yes": yes, "note": "inceledim"} for n in INCELEME]


def kodlar(r):
    return [b["code"] for b in r["bulgular"]]


class Paket(unittest.TestCase):
    def test_tam_model_ve_cevaplarla_paket_temiz(self):
        r = bim.denetle(CIZIM, tam(teslim("kesit-binasi-kunye")), cevaplar())
        self.assertEqual(r["bulgular"], [])
        self.assertEqual([s["key"] for s in r["sonuclar"]], ["okunuyor", "sema", "profil", "model_pafta", "idare"])
        self.assertEqual([s["holds"] for s in r["sonuclar"]], [True, True, True, True, None])
        satir = {s["no"]: s for s in r["satirlar"]}
        self.assertEqual(len(satir), 21)
        self.assertTrue(all(s["cevap"] == "Evet" for s in satir.values()), [s for s in satir.values() if s["cevap"] != "Evet"])
        # Resmî cevap müellifin, kanıt KentOS'un: incelemede ikisi ayrı.
        self.assertEqual((satir[1]["cevap"], satir[1]["kanit"], satir[1]["not"]), ("Evet", form.INCELEME, "inceledim (müellif)"))

    def test_cevapsiz_inceleme_ve_hayir_yayini_durdurur(self):
        f = tam(teslim("kesit-binasi-kunye"))
        r = bim.denetle(CIZIM, f, [])
        self.assertEqual(kodlar(r), ["digital_form_unanswered"] * 2)
        self.assertIs(r["sonuclar"][2]["holds"], False)
        r = bim.denetle(CIZIM, f, cevaplar(yes=False))
        self.assertEqual(kodlar(r), ["digital_form_author_no"] * 2)
        self.assertEqual({s["no"]: s["cevap"] for s in r["satirlar"]}[10], "Hayır")
        self.assertEqual({s["no"]: s["cevap"] for s in r["satirlar"]}[7], "Evet", "gövdelerden")

    def test_kaniti_degisen_cevap_yayini_durdurur(self):
        """ADR 8077 §7: satırlar KentOS'un notunu taşır; o nota dayanan cevap geçer, başka nota dayanan durdurur."""
        f = tam(teslim("kesit-binasi-kunye"))
        satir = {s["no"]: s for s in bim.denetle(CIZIM, f, cevaplar())["satirlar"]}
        dayanakli = [dict(c, basis=satir[c["no"]]["gerekce"]) for c in cevaplar()]
        self.assertEqual(bim.denetle(CIZIM, f, dayanakli)["bulgular"], [])
        dayanakli[-1]["basis"] = "3 zon (3 DAHIL); eski kanıt"
        r = bim.denetle(CIZIM, f, dayanakli)
        self.assertEqual(kodlar(r), ["digital_form_answer_stale"])
        self.assertIs(r["sonuclar"][2]["holds"], False)

    def test_kalan_form_satiri_durdurur(self):
        r = bim.denetle(CIZIM, teslim("kesit-binasi-kunye"), cevaplar())
        self.assertIn("digital_required_field", kodlar(r))
        self.assertIs(r["sonuclar"][2]["holds"], False)
        self.assertIn("9", r["sonuclar"][2]["words"])

    def test_eski_pafta_ve_kayip_nesne_model_ile_pafta_uyumunu_bozar(self):
        cizim = {**CIZIM, "cad": {"derived": [{"kind": "section", "name": "B-B kesiti", "mark": "m", "state": "stale",
                                               "inserts": 1}]}}
        r = bim.denetle(cizim, tam(teslim("kesit-binasi-kunye")), cevaplar())
        self.assertEqual(kodlar(r), ["digital_derived_stale"])
        self.assertIs(r["sonuclar"][3]["holds"], False)
        f = tam(teslim("kesit-binasi-kunye"))
        f["export"]["left"] = [{"uid": "u-1", "what": "duvar", "why": "kesit modeli gövdesini kuramadı"}]
        r = bim.denetle(CIZIM, f, cevaplar())
        self.assertEqual(kodlar(r), ["digital_model_loss"])
        self.assertIs(r["sonuclar"][3]["holds"], False)

    def test_okuyucunun_hatasi_semayi_bozar(self):
        f = tam(teslim("kesit-binasi-kunye"))
        f["findings"] = [{"code": "unresolved_reference", "severity": "error", "step": 3, "line": 9,
                          "message": "#3, #99'u anıyor; dosyada yok."}]
        r = bim.denetle(CIZIM, f, cevaplar())
        self.assertIs(r["sonuclar"][1]["holds"], False)
        self.assertIn("#99", r["sonuclar"][1]["words"])

    def test_paket_kurali_da_durdurur(self):
        cizim = copy.deepcopy(CIZIM)
        cizim["package"]["pdfa"] = False
        r = bim.denetle(cizim, tam(teslim("kesit-binasi-kunye")), cevaplar())
        self.assertEqual(kodlar(r), ["digital_package_pdfa"])


if __name__ == "__main__":
    unittest.main()
