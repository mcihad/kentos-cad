"""EK-3 ve EK-4 kurallarının sınamaları (docs/adr/8064): her kural için bir geçen, bir kalan durum, elle yazılmış
olgularla; kural verisinin kataloğun olduğu.

    python3 -I catalog/standards/dijital-proje/tests/test_ek34.py
"""

import json
import subprocess
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import ek34  # noqa: E402

DEPO = PAKET.parents[2]


def katman(ad, nesne=1, tarama=0, cizgi="continuous", kimlik=None):
    return {"id": kimlik or ad, "name": ad, "objects": nesne, "hatches": tarama, "first": [f"{ad}-1"],
            "firstHatches": [f"{ad}-t"] if tarama else [], "lineType": cizgi, "linePattern": None}


def oda(uid, numara, ad="Salon", kod="K00", at=(0.0, 0.0), kat="zemin"):
    return {"uid": uid, "number": numara, "name": ad, "level": kat, "levelCode": kod, "at": list(at)}


def bul(cad):
    return ek34.denetle({"cad": cad})


def kurallar(cad):
    return sorted({b["rule"] for b in bul(cad)})


# Dört mahal, kuzeydoğu, güneydoğu, güneybatı, kuzeybatı: saat yönünde 001…004.
SAAT = [oda("a", "K00_D01_001", at=(5, 5)), oda("b", "K00_D01_002", at=(5, -5)),
        oda("c", "K00_D01_003", at=(-5, -5)), oda("d", "K00_D01_004", at=(-5, 5))]


class Ek3(unittest.TestCase):
    def test_numara_kalibi(self):
        self.assertEqual(kurallar({"rooms": SAAT}), [])
        b = bul({"rooms": [oda("x", "K0_D01_1")]})
        self.assertEqual([x["rule"] for x in b], ["number_pattern"])
        self.assertIn("Kat_Bölüm_Sıra", b[0]["message"])
        self.assertIn("EK-3 m. 3.1.1 · PDF s. 14", b[0]["cite"])
        self.assertTrue(b[0]["message"].endswith("— Dijital Proje Yön. 7"))

    def test_kat_kodu(self):
        b = bul({"rooms": [oda("x", "K01_D01_001", kod="K00")]})
        self.assertEqual([x["rule"] for x in b], ["storey_code"])
        self.assertEqual(b[0]["fix"]["command"], "cad.spaces.edit")
        self.assertEqual(b[0]["fix"]["input"], {"uids": ["x"], "operation": {"kind": "set", "number": "K00_D01_001"}})
        self.assertEqual(kurallar({"rooms": [oda("x", "K00_D01_001", kod="K00")]}), [])
        self.assertEqual(kurallar({"rooms": [oda("x", "K01_D01_001", kod=None)]}), [], "katsız mahal sınanmaz")

    def test_sira_bosluksuz_ve_tekrarsiz(self):
        bosluk = [oda("a", "K00_D01_001"), oda("b", "K00_D01_003", at=(1, 0))]
        b = bul({"rooms": bosluk})
        self.assertEqual([x["rule"] for x in b], ["sequence"])
        self.assertEqual(b[0]["fix"]["command"], "cad.spaces.number")
        self.assertEqual(b[0]["fix"]["input"]["first"], "K00_D01_001")
        tekrar = [oda("a", "K00_D01_001"), oda("b", "K00_D01_001", at=(1, 0))]
        self.assertIn("001 birden çok mahalde", bul({"rooms": tekrar})[0]["message"])
        # Bölümler ayrı sayılır: iki bölümün ikisi de 001'den.
        self.assertEqual(kurallar({"rooms": [oda("a", "K00_D01_001"), oda("b", "K00_D02_001", at=(3, 0))]}), [])

    def test_saat_yonu(self):
        ters = [{**r, "number": f"K00_D01_{n:03d}"} for r, n in zip(SAAT, (1, 4, 3, 2))]
        b = bul({"rooms": ters})
        self.assertEqual([x["rule"] for x in b], ["clockwise"])
        # Düzeltme mahalleri saat yönünde verir.
        self.assertEqual(b[0]["fix"]["input"]["uids"], ["a", "b", "c", "d"])
        # 001 güneybatıdan başlasa da saat yönünde döngü geçer.
        kaydir = [{**r, "number": f"K00_D01_{n:03d}"} for r, n in zip(SAAT, (3, 4, 1, 2))]
        self.assertEqual(kurallar({"rooms": kaydir}), [])

    def test_mahal_adi(self):
        b = bul({"rooms": [oda("x", "K00_D01_001", ad="Yatak Odasi 2")]})
        self.assertEqual([x["rule"] for x in b], ["room_name"])
        self.assertEqual(b[0]["fix"]["input"]["operation"]["name"], "Yatak Odası 2")
        self.assertEqual(kurallar({"rooms": [oda("x", "K00_D01_001", ad="yatak odası 1")]}), [], "büyük/küçük harf serbest")
        self.assertEqual(kurallar({"rooms": [oda("x", "K00_D01_001", ad="Hobi atölyesi")]}), [], "projeye özgü (3.3)")


class Ek4(unittest.TestCase):
    def test_katman_adi(self):
        iyi = [katman(a) for a in ("MM-DUVA-TUGLA", "MM-XXXX-MAHAL", "MM-DUVA-TUGLA-T", "MM-DUVA-TUGLA-MEVCUT",
                                    "MM-CEPH-TAS-2", "ST-BETN-C30", "PY-BITK-AGAC-CINAR")]
        self.assertEqual(kurallar({"layers": iyi}), [])
        b = bul({"layers": [katman("mm-duva-tuğla")]})
        self.assertEqual([x["rule"] for x in b], ["layer_name"])
        self.assertEqual(b[0]["fix"]["input"], {"operation": "rename", "layer": "mm-duva-tuğla", "name": "MM-DUVA-TUGLA"})
        for ad in ("Çizim", "MM-DUVAR-TUGLA", "XX-DUVA-TUGLA", "MM-DUVA", "MM-DUVA-MALZEMEADICOKUZUNXX", "MM DUVA TUGLA"):
            with self.subTest(ad=ad):
                self.assertEqual(kurallar({"layers": [katman(ad)]}), ["layer_name"])
        self.assertIsNone(bul({"layers": [katman("Çizim")]})[0]["fix"], "Türkçe ad önerilemez")
        self.assertEqual(kurallar({"layers": [katman("Çizim", nesne=0)]}), [], "boş katman sınanmaz")

    def test_sifir_katmani(self):
        b = bul({"layers": [katman("0", nesne=3)]})
        self.assertEqual([x["rule"] for x in b], ["layer_zero"])
        self.assertEqual(b[0]["uids"], ["0-1"])
        self.assertEqual(kurallar({"layers": [katman("0", nesne=0)]}), [])

    def test_tarama_katmani(self):
        self.assertEqual(kurallar({"layers": [katman("MM-DUVA-TUGLA-T", tarama=2)]}), [])
        b = bul({"layers": [katman("MM-DUVA-TUGLA", tarama=2)]})
        self.assertEqual([x["rule"] for x in b], ["hatch_layer"])
        self.assertEqual(b[0]["uids"], ["MM-DUVA-TUGLA-t"])

    def test_emsal_katmani(self):
        self.assertEqual(kurallar({"layers": [katman("MM-EMSL-DAHIL"), katman("MM-EMSL-HARIC")]}), [])
        self.assertEqual(kurallar({"layers": [katman("MM-EMSL-BALKON")]}), ["emsal_layer"])

    def test_aks_cizgisi(self):
        grid = [{"uid": "g", "layer": "aks"}]
        self.assertEqual(kurallar({"layers": [katman("MM-AKSS-GENEL", cizgi="dashdot", kimlik="aks")], "grids": grid}), [])
        b = bul({"layers": [katman("MM-AKSS-GENEL", cizgi="continuous", kimlik="aks")], "grids": grid})
        self.assertEqual([x["rule"] for x in b], ["axis_line"])
        self.assertIn("sürekli çizgiyle", b[0]["message"])
        self.assertEqual(b[0]["fix"]["input"], {"operation": "style", "layer": "aks", "lineType": "dashdot"})

    def test_olcu_yazisi(self):
        self.assertEqual(kurallar({"dimensionFonts": [{"font": "romans.shx", "count": 2}]}), [])
        b = bul({"dimensionFonts": [{"font": "Barlow", "count": 8}]})
        self.assertEqual([(x["rule"], x["severity"]) for x in b], [("dimension_font", "warning")])
        a = bul({"dimensionFonts": [{"font": "Arimo", "count": 8}]})
        self.assertEqual([(x["rule"], x["severity"]) for x in a], [("dimension_font", "info")])
        self.assertIn("K29", a[0]["message"])

    def test_blok_adi(self):
        self.assertEqual(kurallar({"blocks": [{"name": "MM-TEFR-YATAK-160", "inserts": 1, "layers": []}]}), [])
        for ad in ("Yatak 160", "MM-TEFR-ÇALIŞMA", "QQ-TEFR", "MM-TEF"):
            with self.subTest(ad=ad):
                self.assertEqual(kurallar({"blocks": [{"name": ad, "inserts": 1, "layers": []}]}), ["block_name"])


class Veri(unittest.TestCase):
    def test_veri_katalogunkidir(self):
        betik = DEPO / "scripts" / "mevzuat" / "ek34_veri.py"
        if betik.exists():
            r = subprocess.run([sys.executable, "-I", str(betik), "--check"], capture_output=True, text=True)
            self.assertEqual(r.returncode, 0, r.stdout + r.stderr)

    def test_her_kuralin_kaynagi_var(self):
        d = ek34.veri()
        for kural in ("number_pattern", "storey_code", "sequence", "clockwise", "room_name", "layer_name",
                      "layer_zero", "hatch_layer", "emsal_layer", "axis_line", "dimension_font", "block_name"):
            with self.subTest(kural=kural):
                self.assertTrue(all("PDF s." in c for c in d["rules"][kural]))
        self.assertEqual(len(d["rooms"]), 152)


if __name__ == "__main__":
    unittest.main()
