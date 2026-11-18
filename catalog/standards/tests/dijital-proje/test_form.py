"""EK-9 formunun yazılmış dosyadan cevapları (docs/adr/8074): KentOS'un iki dosyasının ölçülmüş olguları
(fixtures/ifc/v1/olgular/, ``kentos-ifc``'in ``facts_tests``'i yazar) ve her soru için geçen ve kalan birer örnek.

    python3 -I catalog/standards/tests/dijital-proje/test_form.py
"""

import copy
import json
import sys
import unittest
from pathlib import Path

PAKET = Path(__file__).resolve().parents[2] / "dijital-proje"
KOK = PAKET.parents[2]
sys.path.insert(0, str(PAKET / "src"))

from dijital_proje import form, mimari  # noqa: E402


def olgular(ad):
    return json.loads((KOK / "fixtures/ifc/v1/olgular" / f"{ad}.json").read_text(encoding="utf-8"))


BINA = olgular("kesit-binasi")
YEREL = olgular("yerel-sistem")
# KentOS'un dosyasından başka bir programın bir kapıyı ve yalnız onun andığı kayıtları çıkardığı dosya
# (scripts/fixtures/ifc_missing_door_case.py): EK-9/8'in kabul durumu.
EKSIK = olgular("kesit-binasi-kapi-eksik")
KAPI = json.loads((KOK / "fixtures/ifc/v1/eksik/kesit-binasi-kapi-eksik.json").read_text(encoding="utf-8"))

_GUID = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$"


def kimlik(gid):
    """KentOS'un GlobalId'den kalıcı kimliği: IFC'nin GUID sıkıştırmasının tersi (kentos_ifc::step::unguid)."""
    n = _GUID.index(gid[0])
    for c in gid[1:]:
        n = (n << 6) | _GUID.index(c)
    h = f"{n:032x}"
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"
KUNYE = olgular("kesit-binasi-kunye")
# Teslim edilen dosyanın adı başlığındaki: fixtures'ta kısa adla saklanır.
KUNYE["file"]["name"] = KUNYE["file"]["header"]["name"]


def satir(f, no):
    return form.cevapla(f)["satirlar"][no - 1]


def kanit(f, no):
    return satir(f, no)["kanit"]


def kodlar(f, no):
    return [b["code"] for b in satir(f, no)["bulgular"]]


def degis(f, **yollar):
    """Olguların derin kopyası, verilen değişikliklerle."""
    g = copy.deepcopy(f)
    for yol, deger in yollar.items():
        parca = yol.split("__")
        hedef = g
        for p in parca[:-1]:
            hedef = hedef[int(p)] if isinstance(hedef, list) else hedef[p]
        son = parca[-1]
        if isinstance(hedef, list):
            hedef[int(son)] = deger
        else:
            hedef[son] = deger
    return g


def nesne(f, sinif):
    return next(o for o in f["objects"] if o["class"] == sinif)


def beklenen(f):
    """Kaynak modelin beklentisi olarak dosyanın kendi nesneleri (uid: GlobalId'in kendisi)."""
    g = copy.deepcopy(f)
    g["expected"] = [{"uid": o["globalId"], "globalId": o["globalId"], "class": o["class"]} for o in f["objects"]
                     if o["class"] not in ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey")]
    g["export"] = {"left": []}
    return g


class GercekDosyalar(unittest.TestCase):
    def test_kesit_binasi_satir_satir(self):
        r = form.cevapla(BINA)
        self.assertEqual(len(r["satirlar"]), 21)
        durum = {s["no"]: s["kanit"] for s in r["satirlar"]}
        for no in (4, 11, 12, 13, 14, 15, 16, 18, 19, 20):
            self.assertEqual(durum[no], form.GECTI, (no, satir(BINA, no)["gerekce"]))
        for no in (2, 3, 6, 9):
            self.assertEqual(durum[no], form.KALDI, no)
        for no in (1, 7, 8, 10, 17, 21):
            self.assertEqual(durum[no], form.INCELEME, no)
        # İnceleme Evet'e çevrilmez; kalan Hayır'dır.
        self.assertEqual(satir(BINA, 1)["cevap"], "")
        self.assertEqual(satir(BINA, 9)["cevap"], "Hayır")
        self.assertEqual(satir(BINA, 12)["cevap"], "Evet")

    def test_kunyeli_dosya_ad_sistem_ve_kisiler(self):
        self.assertEqual(kanit(KUNYE, 3), form.GECTI)
        self.assertEqual(kanit(KUNYE, 6), form.GECTI)
        # Künyeli dosyada kişi, kuruluş, rol, projenin açıklaması ve tipi dolu.
        self.assertEqual(kanit(KUNYE, 2), form.GECTI, satir(KUNYE, 2)["bulgular"])
        # Künyesiz dosyada eksiklerin her biri künyenin kutusuyla söylenir.
        bulgu = satir(BINA, 2)["bulgular"][0]
        self.assertIn("IfcProject: Açıklama (Description)", bulgu["message"])
        self.assertIn("IfcPerson kaydı yok", bulgu["message"])
        for kutu in ("Proje açıklaması (IFC modelinde)", "Dijital teslim kapsamı › Yapı", "Mimar (müellif) adı soyadı",
                     "Büronun adı"):
            self.assertIn(kutu, bulgu["repair"])

    def test_beklenen_nesneler_ve_disa_aktarim_verilince(self):
        f = beklenen(BINA)
        self.assertEqual(kanit(f, 8), form.GECTI)
        self.assertEqual(kanit(f, 17), form.GECTI)
        self.assertEqual(kanit(f, 21), form.GECTI)

    def test_mimari_denetim_dosyadan_bulgulari_kullanir(self):
        cad = {"cad": {"rooms": [], "roomOverlaps": [{"a": "r1", "b": "r2", "area": 1.0, "at": [0, 0]}]}}
        kodlar_ = [b["code"] for b in mimari.denetle(cad, BINA)]
        self.assertIn("digital_required_field", kodlar_)
        # Çizimden ölçülen örtüşme dosya verilince yinelenmez (dosyanın örtüşmesi sorudadır).
        self.assertNotIn("digital_quality_room_overlap", kodlar_)
        self.assertIn("digital_quality_room_overlap", [b["code"] for b in mimari.denetle(cad)])


class MuellifCevaplari(unittest.TestCase):
    """Çizimde tutulan müellif cevapları (docs/adr/8077): basılan formda incelemeye kalan sorunun resmî cevabı ve notu;
    dosyadan cevaplanan soruya verilen cevap okunmaz."""

    def test_inceleme_sorusu_muellifin_cevabini_alir(self):
        satirlar = form.tablo(KUNYE, [{"no": 1, "yes": True, "note": "Sınır yayımlanmadı"}, {"no": 7, "yes": False}])
        bir = next(r for r in satirlar if r[0] == "1")
        yedi = next(r for r in satirlar if r[0] == "7")
        self.assertEqual((bir[4], bir[5]), ("Evet", "Sınır yayımlanmadı (müellif)"))
        self.assertEqual(yedi[4], "Hayır")
        on = next(r for r in satirlar if r[0] == "10")
        self.assertEqual(on[4], "", "cevaplanmayan inceleme sorusu boş kalır")

    def test_kaniti_degisen_cevap_eskir(self):
        """ADR 8077 §7: cevap dayandığı KentOS notunu tutar; not artık başkaysa cevap okunmaz, soru yeniden açılır."""
        satir = next(s for s in form.cevapla(KUNYE)["satirlar"] if s["no"] == 10)
        guncel = form.cevapli([satir], [{"no": 10, "yes": True, "basis": satir["gerekce"]}])[0]
        self.assertEqual((guncel["cevap"], guncel["muellif"], guncel["eski"]), ("Evet", True, False))
        eski = form.cevapli([satir], [{"no": 10, "yes": True, "basis": "3 zon (3 DAHIL); eski kanıt"}])[0]
        self.assertEqual((eski["cevap"], eski["muellif"], eski["eski"]), ("", False, True))
        self.assertIn("yeniden cevaplayın", eski["gerekce"])
        dayanaksiz = form.cevapli([satir], [{"no": 10, "yes": True}])[0]
        self.assertTrue(dayanaksiz["muellif"], "dayanağı söylenmeyen cevap eskimez")

    def test_dosyadan_cevaplanan_soruya_cevap_okunmaz(self):
        self.assertEqual(form.tablo(KUNYE), form.tablo(KUNYE, [{"no": 4, "yes": False, "note": "Okunmaz"}]))


class IkinciOkuyucu(unittest.TestCase):
    """KentOS'un okuduğu olgulardan form, başka bir okuyucunun (scripts/fixtures/ifc_form_reference.py) aynı dosyalardan
    kurduğu olgulardaki formla satır satır aynı: kanıt, bulguların kodu ve nesneleri."""

    def test_bes_dosyada_ayrisan_satir_yok(self):
        for ad in ("kesit-binasi", "kesit-binasi-kunye", "kesit-binasi-tam", "yerel-sistem", "kesit-binasi-kapi-eksik"):
            ref = json.loads((KOK / "fixtures/ifc/v1/form" / f"{ad}.json").read_text(encoding="utf-8"))
            bizim = [{"no": r["no"], "kanit": r["kanit"],
                      "bulgular": sorted([b["code"], sorted(b.get("globalIds") or [])] for b in r["bulgular"])}
                     for r in form.cevapla(olgular(ad))["satirlar"]]
            for a, b in zip(bizim, ref["rows"]):
                self.assertEqual(a, b, f"{ad}: soru {a['no']}")


class BagimsizIds(unittest.TestCase):
    """Profilin IDS'i bağımsız bir denetleyicide (IfcTester; scripts/fixtures/ids_ifctester_check.py) beş dosyada
    koşuldu: bulduğu eksikler KentOS'un okuduğu olgulardaki eksiklerle satır ve nesne düzeyinde aynı."""

    def test_bes_dosyada_ifctester_ile_ayni(self):
        for ad in ("kesit-binasi", "kesit-binasi-kunye", "kesit-binasi-tam", "yerel-sistem", "kesit-binasi-kapi-eksik"):
            ref = json.loads((KOK / "fixtures/ifc/v1/ids-sonuc" / f"{ad}.json").read_text(encoding="utf-8"))
            self.assertEqual(ref["differ"], {}, ad)
            f = olgular(ad)
            bizim = {g["id"]: sorted(g["missing"]) for g in f["gaps"] if not g["class"].startswith(("IfcPerson",
                     "IfcOrganization", "IfcActorRole", "IfcPostalAddress", "IfcProjectedCRS", "IfcMapConversion",
                     "IfcGeometricRepresentationContext"))}
            self.assertEqual(bizim, ref["rows"]["gaps"], ad)


class EksikKapi(unittest.TestCase):
    """Üç kapıdan biri ve ilgili başvuruları dosyadan çıkarıldı: şema geçerli, kalan kapıların zorunlu alan (IDS) sonucu
    değişmedi; EK-9/8 eksik kapıyı kaynağın kimliğiyle bulur."""

    def test_sema_gecerli_ve_ids_degismedi(self):
        self.assertEqual(KAPI["validator"], [])
        self.assertEqual((KAPI["doors"]["before"], KAPI["doors"]["after"]), (5, 4))
        self.assertEqual([x for x in EKSIK["findings"] if x["severity"] == "error"], [])
        kapi = KAPI["removed"]["globalId"]
        once = {(g["id"]): sorted(m for m in g["missing"] if m != kapi) for g in BINA["gaps"]}
        sonra = {(g["id"]): sorted(g["missing"]) for g in EKSIK["gaps"]}
        self.assertEqual(once, sonra, "kalan kapıların ve öbür nesnelerin eksikleri aynı: IDS eksik kapıyı görmez")

    def test_8_eksik_kapiyi_kaynak_kimligiyle_bulur(self):
        f = copy.deepcopy(EKSIK)
        f["expected"] = [{"uid": kimlik(o["globalId"]), "globalId": o["globalId"], "class": o["class"]}
                         for o in BINA["objects"]]
        f["export"] = {"left": [], "memory": True}
        self.assertEqual(kanit(f, 8), form.KALDI)
        b = satir(f, 8)["bulgular"]
        self.assertEqual([x["code"] for x in b], ["digital_model_object_missing"])
        self.assertEqual(b[0]["uids"], [kimlik(KAPI["removed"]["globalId"])])
        self.assertEqual(kimlik(KAPI["removed"]["globalId"]), "0192f5a1-4242-7000-8000-00000000000e")
        # Sınıfı da beklenen; dosyanın kendi nesnelerinde yanlış sınıf yok.
        self.assertEqual(kodlar(f, 17), [])


class TabloVeKaynak(unittest.TestCase):
    def test_cizimdeki_tablo(self):
        t = form.tablo(beklenen(BINA))
        self.assertEqual(t[0], form.BASLIK)
        self.assertEqual(len(t), 1 + 5 + 21)
        gruplar = [r[1] for r in t if r[0] == ""]
        self.assertEqual(gruplar, ["GENEL KONTROL", "İSİMLENDİRME", "MODEL YAPILANMASI", "DİSİPLİN KONTROLLERİ",
                                   "TESLİM VE FORMAT"])
        satir = {r[0]: r for r in t if r[0]}
        self.assertEqual(satir["9"][4], "Hayır")
        self.assertTrue(satir["9"][5].endswith("Mimari denetim › Mevzuat nesneleriyle listeler."), satir["9"][5])
        # İnceleme boş kalır; not cümledir.
        self.assertEqual(satir["1"][4], "")
        self.assertTrue(satir["3"][5].startswith("Dosyanın adı") and satir["12"][5] == "10 mahal.", satir["3"][5])
        # Olgu yoksa (duvarsız çizim) sorular cevapsız.
        bos = form.tablo(None)
        self.assertEqual(len(bos), 1 + 5 + 21)
        self.assertTrue(all(r[4] == "" for r in bos[1:]))

    def test_kaynak_madde_ek_ve_sayfa(self):
        b = satir(BINA, 3)["bulgular"][0]
        self.assertTrue(b["message"].endswith("— Dijital Proje Yön. 6"), b["message"])
        self.assertTrue(b["cite"].startswith("Dijital Proje Yön. 6 · RG 5.8.2026/33331 · EK-1 m. 1.1.1 · PDF s. 1"),
                        b["cite"])
        self.assertTrue(b["cite"].endswith("EK-9 Tablo 9.1 satır 3 · PDF s. 238"), b["cite"])
        sistem = satir(BINA, 6)["bulgular"][0]["cite"]
        self.assertIn("EK-7 Tablo 7.6 · PDF s. 227–228", sistem)
        # Zorunlu alanın kaynağı kendi tablosu ve sayfası.
        alan = satir(BINA, 9)["bulgular"][0]["cite"]
        self.assertRegex(alan, r"EK-6 Tablo 6\.\d+ · PDF s\. \d+ · EK-9 Tablo 9\.1 satır 9")

    def test_yerel_sistem_ve_bellekteki_dosya(self):
        self.assertEqual(kanit(YEREL, 6), form.GECTI)
        g = satir(YEREL, 6)["gerekce"]
        self.assertIn("EPSG:5256", g)
        self.assertIn("döndürülmüş", g)
        self.assertIn("1285.40", g)
        # Sistemsiz dosyada dışa aktarımın nedeni söylenir.
        f = copy.deepcopy(BINA)
        f["export"] = {"left": [], "crsWhy": "±0,00'ın mutlak kotu bilinmiyor"}
        self.assertIn("±0,00'ın mutlak kotu bilinmiyor", satir(f, 6)["bulgular"][0]["message"])
        # Bellekte yazılan dosyanın sürümü teslim edilenin değildir.
        f["export"]["memory"] = True
        self.assertEqual(kanit(f, 18), form.INCELEME)
        self.assertTrue(satir(f, 18)["gerekce"].startswith("Teslim edilen dosyayı denetleyin"))


class OburDisiplinler(unittest.TestCase):
    """EK-9 Tablo 9.2 (docs/adr/8114): mimari dışı disiplinin modeli 16 soruyla; disiplin dosyanın adının EK-2
    alanından. Başka bir programın kesit binasının statik modeli (fixtures/ifc/v1/statik/, olguları ``facts_tests``)."""

    def statik(self, disiplin="ST"):
        f = olgular("kesit-binasi-statik")
        f["file"]["name"] = f"260123_00_{disiplin}_XXXX_MD_XXX_NNN_01_000.ifc"
        return f

    def test_disiplin_dosyanin_adindan(self):
        self.assertEqual(form.tablosu(self.statik()), "9.2")
        self.assertEqual(form.tablosu(self.statik("MK")), "9.2")
        self.assertEqual(form.tablosu(self.statik("IM")), "9.2")
        self.assertEqual(form.tablosu(self.statik("MM")), "9.1")
        # Adı şablonda olmayan dosyanın disiplini bilinmez: mimari sayılır.
        self.assertEqual(form.tablosu(olgular("kesit-binasi-statik")), "9.1")
        self.assertEqual(form.tablosu(BINA), "9.1")

    def test_on_alti_satir_ve_karsiliklari(self):
        c = form.cevapla(self.statik())
        self.assertEqual(c["tablo"], "9.2")
        self.assertEqual([s["no"] for s in c["satirlar"]], list(range(1, 17)))
        self.assertEqual([s["esas"] for s in c["satirlar"]], [1, 2, 3, 5, 6, 7, 8, 9, 10, 15, 16, 17, 18, 19, 20, 21])
        # Tablo 9.2'nin metni: 12. soru "dönüştürüldü mü", 9.1'inki "oluşturuldu mu".
        self.assertIn("dönüştürüldü", c["satirlar"][11]["soru"])
        kanit = {s["no"]: s["kanit"] for s in c["satirlar"]}
        # Mahal ve zon taşımayan disiplin modelinde emsal sorusu inceleme, kaldı değil.
        self.assertEqual(kanit[9], form.INCELEME)
        self.assertEqual(c["satirlar"][8]["bulgular"], [])
        # Statik modelin eksikleri: proje bilgileri, dosyanın adı, varlık adları, sistem, zorunlu alanlar, katlar.
        self.assertEqual([n for n, k in kanit.items() if k == form.KALDI], [2, 3, 4, 5, 8, 10])
        self.assertEqual(kanit[13], form.GECTI)

    def test_onarim_programdan_bagimsiz_kaynak_9_2(self):
        c = form.cevapla(self.statik())
        for s in c["satirlar"]:
            for b in s["bulgular"]:
                self.assertNotIn("Mimari ›", b["repair"], b)
                self.assertNotIn("künye", b["repair"], b)
                self.assertIn(f"EK-9 Tablo 9.2 satır {s['no']} · PDF s. ", b["cite"])
                self.assertEqual(b["rule"], f"form92_{s['no']}")
        proje = c["satirlar"][1]
        self.assertNotIn("Mimar (müellif)", proje["gerekce"])
        self.assertTrue(proje["gerekce"].startswith("IFC modelinin proje bilgileri eksik"), proje["gerekce"])

    def test_mahalli_disiplin_modelinde_emsal_okunur(self):
        # Mahali ve zonu olan bir model 9.2'de de 9.1'in 10. sorusunun kanıtını alır.
        f = copy.deepcopy(BINA)
        f["file"]["name"] = "260123_00_IM_XXXX_MD_XXX_NNN_01_000.ifc"
        c = form.cevapla(f)
        self.assertEqual(c["tablo"], "9.2")
        on = {s["esas"]: s for s in form.cevapla(BINA)["satirlar"]}[10]
        self.assertEqual(c["satirlar"][8]["kanit"], on["kanit"])


class Sorular(unittest.TestCase):
    """Her soru için kalan bir örnek (geçenler gerçek dosyalarda)."""

    def test_1_her_zaman_inceleme(self):
        self.assertEqual(kanit(BINA, 1), form.INCELEME)

    def test_2_kisi_adi_yoksa(self):
        f = copy.deepcopy(KUNYE)
        f["records"]["IfcPerson"][0]["attributes"]["FamilyName"] = None
        b = satir(f, 2)["bulgular"][0]
        self.assertIn("IfcPerson", b["message"])
        # The family name has its own künye field (docs/adr/8124).
        self.assertIn("Mimarın soyadı (IFC modelinde)", b["repair"])
        # KentOS'un kendi yazdığı alan (rol) eksikse dosya sonradan değişmiştir.
        f = copy.deepcopy(KUNYE)
        f["records"]["IfcActorRole"][0]["attributes"]["Role"] = None
        self.assertIn("dosya yazıldıktan sonra değişmiş", satir(f, 2)["bulgular"][0]["repair"])
        # Kullanıcı tanımlı rol yalnız rol USERDEFINED iken aranır.
        f = copy.deepcopy(KUNYE)
        f["records"]["IfcActorRole"][0]["attributes"]["Role"] = "USERDEFINED"
        self.assertIn("Kullanıcı Tanımlı Rol", satir(f, 2)["bulgular"][0]["message"])

    def test_3_ad_sablonda_degil(self):
        f = degis(KUNYE, file__name="bina.ifc")
        self.assertEqual(kanit(f, 3), form.KALDI)
        f = degis(KUNYE, file__name="000000_00_MM_XXXX_MD_XXX_NNN_01_000.ifc")
        self.assertEqual(kodlar(f, 3), ["digital_project_code_missing"])

    def test_4_mahal_adsiz_ve_yinelenen(self):
        f = copy.deepcopy(BINA)
        mahaller = [o for o in f["objects"] if o["class"] == "IfcSpace"]
        mahaller[0]["longName"] = None
        mahaller[1]["name"] = mahaller[2]["name"]
        self.assertEqual(sorted(kodlar(f, 4)), ["digital_space_name_duplicate", "digital_space_unnamed"])

    def test_5_ad_kalipta_degil(self):
        f = copy.deepcopy(BINA)
        nesne(f, "IfcWall")["name"] = "Duvar 1"
        self.assertEqual(kodlar(f, 5), ["digital_entity_name_wrong"])
        # Açıklaması boş ad kalmaz, incelemedir; nesneleri uyarıyla listelenir (K49).
        f = copy.deepcopy(BINA)
        nesne(f, "IfcWall")["name"] = "MM-DVR"
        self.assertEqual(kanit(f, 5), form.INCELEME)
        b = satir(f, 5)["bulgular"]
        self.assertEqual([(x["code"], x["severity"]) for x in b], [("digital_entity_name_no_description", "warning")])
        nesne(f, "IfcSlab")["name"] = "MM-KLN-Döşeme"
        self.assertEqual(kanit(f, 5), form.KALDI)

    def test_6_sistem_turef_degil(self):
        f = copy.deepcopy(KUNYE)
        f["records"]["IfcProjectedCRS"][0]["attributes"]["Name"] = "EPSG:32636"
        self.assertEqual(kanit(f, 6), form.KALDI)
        self.assertIn("TUREF", satir(f, 6)["gerekce"])

    def test_8_ve_17_eksik_vekil_ve_yanlis_sinif(self):
        f = beklenen(BINA)
        duvar = nesne(f, "IfcWall")
        f["objects"] = [o for o in f["objects"] if o is not duvar]
        self.assertEqual(kodlar(f, 8), ["digital_model_object_missing"])
        self.assertEqual(satir(f, 8)["bulgular"][0]["uids"], [duvar["globalId"]])
        f = beklenen(BINA)
        nesne(f, "IfcColumn")["class"] = "IfcBuildingElementProxy"
        self.assertIn("digital_proxy_class", kodlar(f, 8))
        self.assertEqual(sorted(kodlar(f, 17)), ["digital_class_wrong", "digital_proxy_class"])

    def test_9_zorunlu_alan(self):
        self.assertEqual(set(kodlar(BINA, 9)), {"digital_required_field"})
        # Modelin bağlamının gerçek kuzeyi (EK-7 Tablo 7.8) kayıttan; KentOS henüz yazmıyor.
        kuzey = [b for b in satir(BINA, 9)["bulgular"] if "(TrueNorth)" in b["message"]]
        self.assertEqual(len(kuzey), 1)
        self.assertIn("henüz yazmıyor", kuzey[0]["repair"])
        f = degis(BINA, gaps=[])
        f["records"]["IfcGeometricRepresentationContext"][0]["attributes"]["TrueNorth"] = "#99"
        self.assertEqual(kanit(f, 9), form.GECTI)

    def test_10_zon_yoksa_kalir(self):
        f = copy.deepcopy(BINA)
        f["objects"] = [o for o in f["objects"] if o["class"] != "IfcSpatialZone"]
        self.assertEqual(kodlar(f, 10), ["digital_emsal_zones_missing"])

    def test_11_ve_12_yapi_ve_mahal_yoksa(self):
        f = copy.deepcopy(BINA)
        f["objects"] = [o for o in f["objects"] if o["class"] not in ("IfcBuildingStorey", "IfcSpace")]
        self.assertEqual(kanit(f, 11), form.KALDI)
        self.assertEqual(kodlar(f, 12), ["digital_no_spaces"])

    def test_13_ortusme_ve_yanlis_kat(self):
        f = copy.deepcopy(BINA)
        a, b = f["bodies"]["spaces"][0]["globalId"], f["bodies"]["spaces"][1]["globalId"]
        f["bodies"]["overlaps"] = [{"a": a, "b": b, "area": 0.5}]
        self.assertEqual(kodlar(f, 13), ["digital_quality_room_overlap"])
        f["bodies"]["overlaps"] = [{"a": a, "b": b, "area": 0.001}]
        self.assertEqual(kanit(f, 13), form.GECTI, "eşiğin altı yuvarlamadır")
        f["bodies"]["spaces"][0]["zMin"] = -5.0
        self.assertEqual(kodlar(f, 13), ["digital_space_wrong_storey"])

    def test_14_acik_ve_tavani_tasan_mahal(self):
        f = copy.deepcopy(BINA)
        f["bodies"]["spaces"][0]["closed"] = False
        alt = min(f["bodies"]["spaces"], key=lambda s: s["zMin"])
        alt["zMax"] = 99.0
        self.assertEqual(sorted(kodlar(f, 14))[:2], ["digital_space_open", "digital_space_top_wrong"])

    def test_15_katsiz_ve_katinin_disinda(self):
        f = copy.deepcopy(BINA)
        duvar = nesne(f, "IfcWall")
        duvar["storey"] = None
        self.assertEqual(kodlar(f, 15), ["digital_entity_no_storey"])
        f = copy.deepcopy(BINA)
        g = nesne(f, "IfcColumn")["globalId"]
        govde = next(b for b in f["bodies"]["bodies"] if b["globalId"] == g)
        govde["zMin"], govde["zMax"] = 50.0, 53.0
        self.assertEqual(kodlar(f, 15), ["digital_entity_wrong_storey"])

    def test_16_yinelenen_kimlik_ve_ust_uste_nesne(self):
        f = copy.deepcopy(BINA)
        f["findings"] = [{"code": "global_id_duplicate", "severity": "error", "step": 5, "line": 9,
                          "message": "“x” GlobalId'si iki nesnede: #3 ve #5."}]
        self.assertEqual(kodlar(f, 16), ["digital_duplicate_global_id"])
        f = copy.deepcopy(BINA)
        kolon = nesne(f, "IfcColumn")
        ikiz = dict(kolon, globalId="ikiz-kolon")
        f["objects"].append(ikiz)
        govde = next(b for b in f["bodies"]["bodies"] if b["globalId"] == kolon["globalId"])
        f["bodies"]["bodies"].append(dict(govde, globalId="ikiz-kolon"))
        self.assertEqual(kodlar(f, 16), ["digital_duplicate_entity"])

    def test_18_surum_ve_uzanti(self):
        self.assertEqual(kodlar(degis(BINA, file__schema="IFC4"), 18), ["digital_version_wrong"])
        self.assertEqual(kanit(degis(BINA, file__extension="ifczip"), 18), form.KALDI)

    def test_19_on_tanimli_tip(self):
        f = copy.deepcopy(BINA)
        nesne(f, "IfcWall")["predefinedType"] = "NOTDEFINED"
        nesne(f, "IfcSlab")["predefinedType"] = "USERDEFINED"
        nesne(f, "IfcSlab")["objectType"] = None
        self.assertEqual(sorted(kodlar(f, 19)), ["digital_predefined_notdefined", "digital_predefined_userdefined_unnamed"])

    def test_20_set_yanlis_ve_baska_surum(self):
        f = copy.deepcopy(BINA)
        f["setChecks"] = [{"globalId": nesne(f, "IfcWall")["globalId"], "set": "Pset_WallCommon", "item": "Renk",
                           "fromType": False, "class": "IfcWall", "why": "unknown_item"}]
        self.assertEqual(kodlar(f, 20), ["digital_pset_wrong"])
        self.assertEqual(kanit(degis(BINA, file__schema="IFC4"), 20), form.INCELEME)

    def test_21_modele_girmeyen(self):
        f = beklenen(BINA)
        f["export"] = {"left": [{"uid": "u-rampa", "what": "rampa", "why": "gövdesi yok"}]}
        self.assertEqual(kodlar(f, 21), ["digital_model_loss"])
        self.assertEqual(satir(f, 21)["bulgular"][0]["uids"], ["u-rampa"])


class EmsalZonlari(unittest.TestCase):
    """EK-9/10, kat kat ve durum durum (docs/adr/8125): elle yazılmış iki katlı olgular."""

    def dosya(self, zonlar, tablo):
        """``zonlar``: (zon, kat, durum, alan); ``tablo``: (kat, durum, alan)."""
        nesneler = [{"class": "IfcProject", "globalId": "p"},
                    {"class": "IfcBuildingStorey", "globalId": "k0", "longName": "Zemin kat"},
                    {"class": "IfcBuildingStorey", "globalId": "k1", "longName": "1. kat"}]
        govde = []
        for gid, kat, durum, alan in zonlar:
            nesneler.append({"class": "IfcSpatialZone", "globalId": gid,
                             "sets": {"TREpys_EmsalOzellikSeti": {"EmsalDurumu": durum}}})
            govde.append({"globalId": gid, "storey": kat, "area": alan, "areaBound": 0.001})
        return {"objects": nesneler, "bodies": {"zones": govde, "spaces": [], "overlaps": [], "bodies": []},
                "emsal": [{"storey": k, "state": d, "area": a} for k, d, a in tablo]}

    def s10(self, f):
        d = form._Dosya(f)
        return d.s10()

    def test_tablo_yoksa_inceleme(self):
        f = self.dosya([("z0", "k0", "DAHIL", 100.0)], [])
        self.assertEqual(self.s10(f)[0], form.INCELEME)

    def test_kat_kat_ayni_alan_gecer(self):
        f = self.dosya([("z0", "k0", "DAHIL", 100.0), ("z1", "k1", "DAHIL", 90.0), ("z2", "k0", "HARIC", 20.0)],
                       [("k0", "DAHIL", 100.0), ("k1", "DAHIL", 90.0), ("k0", "HARIC", 20.0)])
        kanit, _, bulgular = self.s10(f)
        self.assertEqual(kanit, form.GECTI)
        self.assertEqual(bulgular, [])

    def test_katlar_arasinda_yer_degistiren_alan_kalir(self):
        # Genel toplam 190 m² iki tarafta da: zemin kattan 10 m² birinci kata geçmiş.
        f = self.dosya([("z0", "k0", "DAHIL", 90.0), ("z1", "k1", "DAHIL", 100.0)],
                       [("k0", "DAHIL", 100.0), ("k1", "DAHIL", 90.0)])
        kanit, _, bulgular = self.s10(f)
        self.assertEqual(kanit, form.KALDI)
        self.assertEqual([b["code"] for b in bulgular], ["digital_emsal_zone_area"] * 2)
        self.assertIn("Zemin kat, DAHIL: dosyadaki zonlar 90.00 m², emsal tablosunda 100.00 m² (fark -10.00 m²)",
                      bulgular[0]["message"])

    def test_yanlis_emsal_durumu_kalir(self):
        f = self.dosya([("z0", "k0", "HARIC", 100.0)], [("k0", "DAHIL", 100.0)])
        kanit, _, bulgular = self.s10(f)
        self.assertEqual(kanit, form.KALDI)
        self.assertEqual(len(bulgular), 2, "DAHIL eksik, HARIC fazla")

    def test_govde_sinirinin_icindeki_fark_ayni_sayilir(self):
        f = self.dosya([("z0", "k0", "DAHIL", 100.0004)], [("k0", "DAHIL", 100.0)])
        self.assertEqual(self.s10(f)[0], form.GECTI)


if __name__ == "__main__":
    unittest.main()
