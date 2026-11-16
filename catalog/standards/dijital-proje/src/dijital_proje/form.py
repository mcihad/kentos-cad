"""EK-9 Model Kalite Kontrol Formu, yazılmış dosyadan (docs/adr/8074; plan/ifc-uyum.md IFC-08, bim-plan B13).

Kural dosyayı okumaz: KentOS'un yazılmış IFC dosyasından ölçtüğü olgulara (``kentos.ifcFacts`` 1) ve ev sahibinin
eklediklerine bakar: dosyayı yazdığı modelin nesneleri (``expected``: kalıcı kimlik, GlobalId, sınıf) ve dışa aktarımın
bıraktıkları (``export.left``). Tablo 9.1'in (mimari) 21 sorusunun her biri için iki şey ayrı yazılır:

- **cevap** — formun resmî sütunu: ``Evet``, ``Hayır`` ya da boş (müellifin cevaplayacağı);
- **kanıt** — KentOS'un dosyadan bulduğu: ``geçti``, ``kaldı`` ya da ``inceleme``. İnceleme Evet'e çevrilmez.

Her bulgu nesnesini (GlobalId ve çizimdeki kalıcı kimliği), sınıfını, alan yolunu, maddeyi ve eki, ölçülen ve istenen
değeri ve düzeltmenin yerini söyler.
"""

from __future__ import annotations

import math
import re
from typing import Any

from . import ek34, zorunlu

RG = "RG 5.8.2026/33331"

#: Tablo 9.1'in soruları: numara, grup, disiplin, başlık, soru; PDF sayfası.
SORULAR = [
    (1, "GENEL KONTROL", "Genel", "Dosya Boyutu",
     "Dosya boyutu, Bakanlıkça belirlenen kısıtlamalara uygun mu?", 238),
    (2, "GENEL KONTROL", "Genel", "Proje Bilgileri",
     "Proje bilgileri Yönetmeliğin 13 üncü maddesinde belirtilen esaslara göre dolduruldu mu?", 238),
    (3, "İSİMLENDİRME", "Genel", "Proje ve Dosya İsimlendirmesi",
     "Proje ve dosya isimlendirmesi Yönetmeliğin 6 ncı maddesine uygun mu?", 238),
    (4, "İSİMLENDİRME", "Mimari", "Mahal No ve Mahal İsmi",
     "Mahal numaraları ve mahal isimlendirmesi Yönetmeliğin 10 uncu Maddesine uygun mu?", 238),
    (5, "İSİMLENDİRME", "Genel", "Varlıklar",
     "Varlıkların isimlendirmesi Yönetmeliğin 11 inci maddesine uygun mu?", 238),
    (6, "MODEL YAPILANMASI", "Genel", "Koordinat Sistemi",
     "Model doğru koordinat sisteminde konumlandırıldı mı? Bu koordinat verileri Yönetmeliğin 13 üncü maddesinde verilen ilgili proje öznitelikleri kullanılarak atandı mı?", 238),
    (7, "MODEL YAPILANMASI", "Genel", "Gelişim seviyesi",
     "Modelde yer alan varlıklar Yönetmeliğin 14 üncü maddesinde verilen gelişim seviyesine uygun olarak modellendi mi?", 238),
    (8, "MODEL YAPILANMASI", "Genel", "IFC Sınıfları",
     "Modelde yer alan varlıklar Yönetmeliğin 12 inci maddesinde tanımlanan “Zorunlu IFC Sınıfları” listesindeki doğru sınıflar kullanılarak tanımlandı mı?", 238),
    (9, "MODEL YAPILANMASI", "Genel", "IFC Sınıfları",
     "Yönetmeliğin 13 üncü maddesinde belirtilen projedeki mevcut “Zorunlu IFC Sınıfları” için zorunlu öznitelikler ve özellikler tanımlandı mı?", 239),
    (10, "MODEL YAPILANMASI", "Genel", "Emsal hesabı",
     "Mahal (ifcSpace) ve Mekansal Zon (ifcSpatialZone) alanları emsal hesap tablolarıyla uyumlu mu?", 239),
    (11, "MODEL YAPILANMASI", "Genel", "IFC Sınıfları",
     "Modelde “Proje”, “Saha”, “Bina” ve en az bir “Bina Katı” ile ilgili IFC Sınıfları bulunuyor mu?", 239),
    (12, "DİSİPLİN KONTROLLERİ", "Mimari", "Mahal",
     "Mimari modelde “Mahal” (ifcSpace) IFC Kategorisi yer alıyor mu?", 239),
    (13, "DİSİPLİN KONTROLLERİ", "Mimari", "Mahal",
     "Mahaller diğer mahallerle örtüşmeyecek şekilde ve doğru katta modellendi mi?", 239),
    (14, "DİSİPLİN KONTROLLERİ", "Mimari", "Mahal",
     "Mahaller kapalı olarak ve üst sınırları doğru tanımlandı mı?", 239),
    (15, "DİSİPLİN KONTROLLERİ", "Genel", "Varlıklar",
     "Varlıklar doğru katta veya doğru kotta modellendi mi?", 239),
    (16, "DİSİPLİN KONTROLLERİ", "Genel", "Varlıklar",
     "Modelden yinelenen varlıklar kaldırıldı mı?", 239),
    (17, "TESLİM VE FORMAT", "Genel", "IFC Sınıfları",
     "Model IFC’ye uygun olarak oluşturuldu mu (IfcWall, IfcSlab, IfcWindow vb. sınıfları doğru atandı mı)?", 239),
    (18, "TESLİM VE FORMAT", "Genel", "IFC Versiyonu",
     "Model dosyası, teslime ilişkin kısıtlamalarda tanımlanan IFC versiyonunda teslim edildi mi? Dosya uzantısı IFC mi?", 239),
    (19, "TESLİM VE FORMAT", "Genel", "Ön Tanımlı Tip",
     "Varlıklar IFC’de doğru Ön Tanımlı Tip (Predefined Type) kullanılarak temsil edildi mi?", 239),
    (20, "TESLİM VE FORMAT", "Genel", "Özellikler",
     "Özellikler doğru P_set’ler altında listelendi mi?", 239),
    (21, "TESLİM VE FORMAT", "Genel", "PDF ve IFC uyumu",
     "Teslim edilen PDF paftaları BIM modelinden üretildi mi?", 239),
]

#: Her sorunun maddesi ve kaynak satırları (katalogun kimlikleriyle; metinleri olgulardaki ``sources``): bulgunun iletisi
#: maddeyle biter, kaynağı maddeyi, Resmî Gazete'yi, satırları ve formun sorusunu sayfalarıyla söyler.
KAYNAK = {
    1: ("15", []), 2: ("13", ["T7.1"]), 3: ("6", ["M1.1.1", "T2.1"]), 4: ("10", ["T3.1"]), 5: ("11", ["M5.5"]),
    6: ("13", ["T7.6", "T7.7"]), 7: ("14", []), 8: ("12", ["M5.1"]), 9: ("13", []), 10: ("15", []),
    11: ("12", ["M5.1"]), 12: ("15", []), 13: ("15", []), 14: ("15", []), 15: ("15", []), 16: ("15", []),
    17: ("12", ["M5.1"]), 18: ("15", ["M5.4"]), 19: ("13", []), 20: ("13", []), 21: ("9", []),
}

#: Kanıt durumları ve formun resmî cevabı: inceleme boş kalır, Evet'e çevrilmez.
GECTI, KALDI, INCELEME = "geçti", "kaldı", "inceleme"
CEVAP = {GECTI: "Evet", KALDI: "Hayır", INCELEME: ""}

#: Örtüşme eşiği (m²): ``kalite.ESIK`` ile aynı; altı yuvarlamadır.
ESIK = 0.01

#: Vekil sınıf: zorunlu sınıflardan biri yerine kullanılamaz (EK-5 Tablo 5.1).
VEKIL = "IfcBuildingElementProxy"

#: Künye: Mimari › Proje › Ruhsat takımı'nın künyesi.
KUNYE = "Mimari › Proje › Ruhsat takımı'nın künyesi"

#: EK-7 Tablo 7.1–7.5'in alanlarının künyedeki kutusu (sınıf, öznitelik ya da set yolu). Burada olmayanları KentOS
#: kendisi yazar (rol ARCHITECT, kişi ile kuruluşun bağı): eksikse dosya elle değişmiştir.
KUNYE_YERI = {
    ("IfcProject", "Name"): "Proje kodu",
    ("IfcProject", "Description"): "Proje açıklaması (IFC modelinde)",
    ("IfcProject", "Pset_ProjectCommon.ProjectType"): "Dijital teslim kapsamı › Yapı",
    ("IfcPerson", "Identification"): "Mimarlar Odası sicil no",
    ("IfcPerson", "FamilyName"): "Mimar (müellif) adı soyadı",
    ("IfcPerson", "GivenName"): "Mimar (müellif) adı soyadı",
    ("IfcPersonAndOrganization", "ThePerson"): "Mimar (müellif) adı soyadı",
    ("IfcPersonAndOrganization", "TheOrganization"): "Büronun adı",
    ("IfcOrganization", "Identification"): "Büronun vergi kimlik no",
    ("IfcOrganization", "Name"): "Büronun adı",
    ("IfcOrganization", "Addresses"): "Büronun adresi",
    ("IfcActorRole", "Description"): "Mimarın unvanı",
}

#: Kayıt hiç yoksa künyenin boş kutusu: kişi ile kuruluşun bağı kuruluş olmadan yazılmaz, rol mimar olmadan.
KAYIT_YERI = {"IfcPerson": "Mimar (müellif) adı soyadı", "IfcPersonAndOrganization": "Büronun adı",
              "IfcOrganization": "Büronun adı", "IfcActorRole": "Mimar (müellif) adı soyadı"}

#: Mekânsal yapının sınıfları: kat ve kot denetiminin dışında.
YAPI = ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey", "IfcSpace", "IfcSpatialZone", "IfcZone")


def cevapla(ifc: dict[str, Any]) -> dict[str, Any]:
    """Formun 21 satırı: ``{"tablo": "9.1", "dosya": ad, "satirlar": [{no, grup, disiplin, baslik, soru, cevap, kanit,
    gerekce, bulgular}]}``."""
    d = _Dosya(ifc)
    kaynaklar = ifc.get("sources") or {}
    satirlar = []
    for no, grup, disiplin, baslik, soru, sayfa in SORULAR:
        kanit, gerekce, bulgular = getattr(d, f"s{no}")()
        madde, kimlikler = KAYNAK[no]
        for b in bulgular:
            b["message"] = f"{b['message']} — Dijital Proje Yön. {madde}"
            satir = kaynaklar.get(f"T9.1/{no}") or f"EK-9 Tablo 9.1 satır {no} · PDF s. {sayfa}"
            ek = [b.pop("ek")] if b.get("ek") else []
            b["cite"] = " · ".join([f"Dijital Proje Yön. {madde}", RG, *(kaynaklar[k] for k in kimlikler if k in kaynaklar),
                                    *ek, satir])
            b.setdefault("rule", f"form_{no}")
            b.setdefault("severity", "error")
            # Nesnesi olmayan bulgu çizimde binanın yerini gösterir (ev sahibinin verdiği).
            b.setdefault("at", list(ifc.get("at") or [0.0, 0.0]))
            b.setdefault("fix", None)
        satirlar.append({
            "no": no,
            "grup": grup,
            "disiplin": disiplin,
            "baslik": baslik,
            "soru": soru,
            "cevap": CEVAP[kanit],
            "kanit": kanit,
            "gerekce": gerekce,
            "bulgular": bulgular,
        })
    return {"tablo": "9.1", "dosya": (ifc.get("file") or {}).get("name"), "satirlar": satirlar}


#: Çizimdeki formun başlığı: Tablo 9.1'in sütunları ve KentOS'un notu. Bu başlıklı tablo formdur (KentOS onu tanıyıp
#: Ruhsat takımında cevaplarını yeniden yazar); masaüstünün ``FORM_HEADER``'ıyla aynıdır.
BASLIK = ["No", "Kategori / Disiplin", "Kontrol Başlığı", "Açıklama", "Evet / Hayır", "Not"]


def tablo(ifc: dict[str, Any] | None) -> list[list[str]]:
    """Formun çizimdeki tablosu: başlık; her grubun adı kendi satırında, sonra soruları: numara, disiplin, başlık, soru,
    resmî cevap (Evet, Hayır ya da müellifin cevaplayacağı boş) ve KentOS'un notu (kanıtın gerekçesi). Olgu yoksa
    (çizimde duvar yok) sorular cevapsızdır."""
    if ifc:
        satirlar = cevapla(ifc)["satirlar"]
    else:
        satirlar = [{"no": no, "grup": grup, "disiplin": disiplin, "baslik": baslik, "soru": soru, "cevap": "",
                     "gerekce": "Çizimde bina yok: model yazılmadı.", "bulgular": []}
                    for no, grup, disiplin, baslik, soru, _ in SORULAR]
    out = [list(BASLIK)]
    grup = None
    for s in satirlar:
        if s["grup"] != grup:
            grup = s["grup"]
            out.append(["", grup, "", "", "", ""])
        not_ = _cumle(s["gerekce"])
        if s["bulgular"]:
            not_ += " Mimari denetim › Mevzuat nesneleriyle listeler."
        out.append([str(s["no"]), s["disiplin"], s["baslik"], s["soru"], s["cevap"], not_])
    return out


def _cumle(s: str) -> str:
    """Bir notun cümle olarak yazılışı: ilk harfi Türkçenin büyük harfiyle (i İ, ı I), sonu noktalı."""
    s = s.strip()
    if not s:
        return s
    ilk = {"i": "İ", "ı": "I"}.get(s[0], s[0].upper())
    s = ilk + s[1:]
    return s if s[-1] in ".?!" else s + "."


def denetle(ifc: dict[str, Any] | None) -> list[dict[str, Any]]:
    """Formun kalan satırlarının bulguları, satır sırasıyla (Mimari denetimin Mevzuat grubu)."""
    if not ifc:
        return []
    out: list[dict[str, Any]] = []
    for satir in cevapla(ifc)["satirlar"]:
        out.extend(satir["bulgular"])
    return out


class _Dosya:
    """Olguların üstünde formun soruları; her biri (kanıt, gerekçe, bulgular) verir."""

    def __init__(self, ifc: dict[str, Any]):
        self.ifc = ifc
        self.file = ifc.get("file") or {}
        self.objects = ifc.get("objects") or []
        self.by_gid = {o["globalId"]: o for o in self.objects}
        self.expected = ifc.get("expected") or []
        self.uid = {e["globalId"]: e.get("uid") for e in self.expected}
        self.bodies = ifc.get("bodies") or {}
        self.data = ek34.veri()
        self._katlar: list[tuple[float, dict[str, Any]]] | None = None

    # --- yardımcılar ----------------------------------------------------------------------------------------------

    def of(self, *classes: str) -> list[dict[str, Any]]:
        return [o for o in self.objects if o["class"] in classes]

    def bulgu(self, kod: str, mesaj: str, onarim: str, gids: list[str], ek: str | None = None) -> dict[str, Any]:
        """Bir bulgu: nesneleri GlobalId'leriyle ve çizimdeki kalıcı kimlikleriyle; ``ek`` sorunun kaynaklarına
        eklenen satır (EK-9/9'da eksik alanın tablosu). Madde ve kaynak sorunun (``KAYNAK``)."""
        b = {
            "code": kod,
            "message": mesaj,
            "repair": onarim,
            "globalIds": gids,
            "uids": [u for u in (self.uid.get(g) for g in gids) if u],
        }
        if ek:
            b["ek"] = ek
        return b

    def ad(self, o: dict[str, Any]) -> str:
        return o.get("name") or o.get("longName") or f"{o['class']} {o['globalId']}"

    def storeys(self) -> list[tuple[float, dict[str, Any]]]:
        """Katlar kotlarına göre sıralı (kotu yazılmamış kat dışarıda); bir kez sıralanır."""
        if self._katlar is None:
            ks = [(o["elevation"], o) for o in self.of("IfcBuildingStorey") if isinstance(o.get("elevation"), (int, float))]
            self._katlar = sorted(ks, key=lambda k: k[0])
        return self._katlar

    def band(self, storey_gid: str | None) -> tuple[float, float] | None:
        """Katın kot aralığı: kendi kotundan bir üstündekine (en üst katta üstü açık)."""
        ks = self.storeys()
        for i, (z, o) in enumerate(ks):
            if o["globalId"] == storey_gid:
                ust = ks[i + 1][0] if i + 1 < len(ks) else float("inf")
                return z, ust
        return None

    # --- sorular --------------------------------------------------------------------------------------------------

    def s1(self):
        mb = int(self.file.get("bytes") or 0) / 1_000_000
        return INCELEME, f"Dosya {mb:.1f} MB. Bakanlığın dosya boyutu sınırı yayımlanmadı; sınırı müellif ve idare bilir.", []

    def s2(self):
        """EK-7 Tablo 7.1–7.5: proje, kişi, kuruluş ve rol kayıtlarının zorunlu öznitelikleri; her eksik künyenin
        kutusuyla."""
        eksik: list[str] = []
        kutular: list[str] = []

        def kutu(sinif: str, ad: str, kayit_yok: bool = False):
            k = KAYIT_YERI.get(sinif) if kayit_yok else KUNYE_YERI.get((sinif, ad))
            if k and k not in kutular:
                kutular.append(k)

        proje = self.of("IfcProject")
        for g in self.ifc.get("gaps") or []:
            if g["class"] == "IfcProject" and str(g["table"]).startswith("7.1"):
                eksik.append(f"IfcProject: {g['tr']} ({g['path']})")
                kutu("IfcProject", g["path"])
        kayitlar = self.ifc.get("records") or {}
        for r in self.ifc.get("rows") or []:
            tablo = str(r["table"])
            if not r.get("mandatory") or not r.get("attribute") or tablo not in ("7.2", "7.3", "7.4", "7.5"):
                continue
            olanlar = kayitlar.get(r["class"]) or []
            if not olanlar:
                yok = f"{r['class']} kaydı yok"
                if yok not in eksik:
                    eksik.append(yok)
                kutu(r["class"], r["name"], kayit_yok=True)
                continue
            for k in olanlar:
                a = k.get("attributes") or {}
                # IFC'nin kuralı: kullanıcı tanımlı rol yalnız rol USERDEFINED iken yazılır (K48).
                if r["name"] == "UserDefinedRole" and a.get("Role") != "USERDEFINED":
                    continue
                if not _dolu(a.get(r["name"])):
                    eksik.append(f"{r['class']} #{k['step']}: {r['tr']} ({r['name']})")
                    kutu(r["class"], r["name"])
        if not proje:
            eksik.insert(0, "IfcProject yok")
        if not eksik:
            return GECTI, "EK-7'nin proje, kişi, kuruluş ve rol bilgileri dosyada dolu.", []
        gids = [p["globalId"] for p in proje]
        if kutular:
            onarim = f"{KUNYE}nde doldurun: {', '.join(kutular)}; dosyayı yeniden yazın."
        else:
            onarim = "Bu alanları KentOS yazar: dosya yazıldıktan sonra değişmiş; dosyayı yeniden yazın."
        bulgular = [self.bulgu(
            "digital_project_info_missing",
            f"IFC modelinin proje bilgileri eksik: {'; '.join(eksik[:6])}{' …' if len(eksik) > 6 else ''}",
            onarim, gids)]
        gerekce = f"künyede eksik: {', '.join(kutular)}" if kutular else f"{len(eksik)} proje bilgisi eksik"
        return KALDI, gerekce, bulgular

    def s3(self):
        """Md. 6, EK-1 ve EK-2 Tablo 2.1: dosyanın adı şablonda, proje kodu verilmiş, başlık dosyanın adını taşıyor."""
        ad = str(self.file.get("name") or "")
        govde = re.sub(r"\.ifc$", "", ad, flags=re.IGNORECASE)
        sorun = _dosya_adi(govde, self.data)
        baslik = ((self.file.get("header") or {}).get("name") or "").strip()
        if baslik and baslik != ad:
            sorun.append(f"başlıktaki ad “{baslik}” dosyanın adı değil")
        if not sorun:
            return GECTI, f"Dosyanın adı EK-2 Tablo 2.1'e göre: {ad}.", []
        kod = "digital_project_code_missing" if any("proje kodu" in s for s in sorun) else "digital_file_name_wrong"
        proje = [p["globalId"] for p in self.of("IfcProject")]
        if kod == "digital_project_code_missing":
            onarim = (f"Proje kodunu idareden alın (EK-1: başvuru yılının iki hanesi ve dört haneli sıra no) ve {KUNYE}nde "
                      "Proje kodu'na yazın; KentOS dosyayı EK-2'ye göre adlandırır.")
        else:
            onarim = "Dosyayı KentOS'un dışa aktarımıyla yeniden yazın; adını sonradan değiştirmeyin (EK-2 Tablo 2.1)."
        return KALDI, "; ".join(sorun), [self.bulgu(
            kod, f"“{ad}”: {'; '.join(sorun)}", onarim, proje)]

    def s4(self):
        """Md. 10, EK-3: her mahalin numarası ve adı; adlar tekil ve Tablo 3.2'deki yazımla."""
        mahaller = self.of("IfcSpace")
        if not mahaller:
            return KALDI, "dosyada mahal yok", []
        adsiz = [m for m in mahaller if not _dolu(m.get("longName")) or _varsayilan(m.get("name"))]
        gorulen: dict[str, list[str]] = {}
        for m in mahaller:
            gorulen.setdefault((m.get("name") or "").strip(), []).append(m["globalId"])
        yinelenen = {a: g for a, g in gorulen.items() if a and len(g) > 1}
        yazim = {ek34._kucuk(t["name"]): t["name"] for t in self.data["rooms"]}
        katli = {ek34._katla(t["name"]): t["name"] for t in self.data["rooms"]}
        yanlis = []
        for m in mahaller:
            ad = (m.get("longName") or "").strip()
            govde = re.sub(r"\s*\d+$", "", ad).strip()
            if govde and ek34._kucuk(govde) not in yazim and ek34._katla(govde) in katli:
                yanlis.append((m, katli[ek34._katla(govde)]))
        bulgular = []
        if adsiz:
            bulgular.append(self.bulgu(
                "digital_space_unnamed", f"{len(adsiz)} mahalin numarası ya da adı yok",
                "Mahalleri seçip Öznitelikler'de numarasını ve adını verin; seçili mahalin sekmesindeki Mahal numarala "
                "sırayla numaralar.",
                [m["globalId"] for m in adsiz]))
        for a, gids in yinelenen.items():
            bulgular.append(self.bulgu(
                "digital_space_name_duplicate", f"“{a}” adı {len(gids)} mahalde", "Mahal numaraları tekil olmalı; "
                "yinelenenleri yeniden numaralayın.", gids))
        for m, dogru in yanlis:
            bulgular.append(self.bulgu(
                "digital_ek3_room_name", f"Mahal adı “{m.get('longName')}” Tablo 3.2'deki yazımla değil: “{dogru}”",
                f"Adı “{dogru}” yapın.", [m["globalId"]]))
        if bulgular:
            return KALDI, f"{len(adsiz)} adsız, {len(yinelenen)} yinelenen, {len(yanlis)} yazımı farklı mahal.", bulgular
        return GECTI, f"{len(mahaller)} mahalin numarası ve adı var, adları tekil.", []

    def s5(self):
        """Md. 11, EK-5 m. 5.5 ve Tablo 5.3: adın kalıbı Disiplin-KategoriKodu-Açıklama; kodu sınıfının. Disiplini ya
        da kodu yanlış ad kalır. Açıklaması boş ad (MM-DVR) şablonun üçüncü alanını boş bırakır; yönetmelik alanın boş
        kalıp kalamayacağını söylemez (5.5.3: karakter sınırı yok): kanıt incelemedir, nesneleri uyarıyla listelenir
        (K49)."""
        kodlar = self.ifc.get("classCodes") or {}
        yanlis, bos = [], []
        for o in self.objects:
            kod = kodlar.get(o["class"])
            # Tablo 5.1'de kodu olmayan sınıf (proje: adı proje kodudur) kalıba bağlı değil.
            if not kod or kod.strip("-") == "":
                continue
            p = (o.get("name") or "").split("-", 2)
            if len(p) < 2 or p[0] != "MM" or p[1] != kod:
                yanlis.append((o, kod))
            elif len(p) == 2 or not p[2].strip():
                bos.append(o)
        bulgular = []
        if yanlis:
            ornek = ", ".join(f"“{self.ad(o)}” (beklenen MM-{k}-…)" for o, k in yanlis[:4])
            bulgular.append(self.bulgu(
                "digital_entity_name_wrong", f"{len(yanlis)} nesnenin adı EK-5'in kalıbında değil: {ornek}",
                "Nesnenin adını Disiplin-KategoriKodu-Açıklama kalıbında yazın (MM-DVR-Dış_duvar_25).",
                [o["globalId"] for o, _ in yanlis]))
        if bos:
            ornek = ", ".join(sorted({self.ad(o) for o in bos})[:4])
            b = self.bulgu(
                "digital_entity_name_no_description",
                f"{len(bos)} nesnenin adında açıklama yok ({ornek}): EK-5 Tablo 5.3'ün üçüncü alanı boş",
                "Adın açıklaması nesnenin kendi verisinden yazılır: duvara bir duvar stili, kapıya ve pencereye numara "
                "verin; açıklamanın boş kalıp kalamayacağına müellif karar verir.",
                [o["globalId"] for o in bos])
            b["severity"] = "warning"
            bulgular.append(b)
        if yanlis:
            return KALDI, f"{len(yanlis)} nesnenin adı kalıpta değil.", bulgular
        if bos:
            return INCELEME, (f"{len(bos)} nesnenin adında açıklama yok; disiplin ve kategori kodu doğru. Açıklamanın boş "
                              "kalıp kalamayacağı yönetmelikte yazılı değil."), bulgular
        return GECTI, "Adlar EK-5 m. 5.5'in kalıbında: MM-<kategori kodu>-<açıklama>.", []

    def s6(self):
        """Md. 13, EK-7 Tablo 7.6–7.7: TUREF 3° dilimi (EPSG:5253–5259), harita dönüşümünün zorunlu alanları."""
        kayitlar = self.ifc.get("records") or {}
        crs = kayitlar.get("IfcProjectedCRS") or []
        donusum = kayitlar.get("IfcMapConversion") or []
        sorun = []
        if not crs or not donusum:
            # The export writes both or neither, and says why it wrote neither.
            yok = " ve ".join(n for n, k in (("koordinat sistemi (IfcProjectedCRS)", crs),
                                             ("harita dönüşümü (IfcMapConversion)", donusum)) if not k)
            neden = (self.ifc.get("export") or {}).get("crsWhy") or "model yerel koordinatlarda"
            sorun.append(f"dosyada {yok} yok ({neden})")
        for r in self.ifc.get("rows") or []:
            if not r.get("mandatory") or str(r["table"]) not in ("7.6", "7.7"):
                continue
            for k in (crs if r["class"] == "IfcProjectedCRS" else donusum if r["class"] == "IfcMapConversion" else []):
                if not _dolu((k.get("attributes") or {}).get(r["name"])):
                    sorun.append(f"{r['class']}: {r['tr']} ({r['name']}) yok")
        for k in crs:
            ad = str((k.get("attributes") or {}).get("Name") or "")
            m = re.fullmatch(r"EPSG:(\d+)", ad.strip())
            if not m or not 5253 <= int(m.group(1)) <= 5259:
                sorun.append(f"sistem {ad or 'adsız'} bir TUREF 3° dilimi (EPSG:5253–5259) değil")
        if not sorun:
            ad = (crs[0].get("attributes") or {}).get("Name")
            k = donusum[0].get("attributes") or {}
            return GECTI, f"{ad} (EK-7 Tablo 7.6){_yerel(k)}, ±0.00 {float(k.get('OrthogonalHeight') or 0.0):.2f} m.", []
        proje = [p["globalId"] for p in self.of("IfcProject")]
        return KALDI, "; ".join(sorun), [self.bulgu(
            "digital_system_wrong", f"IFC modelinin koordinat sistemi: {'; '.join(sorun[:4])}",
            "Dosya › Proje ayarları › Koordinat sistemi'nde binanın TUREF 3° dilimini (EPSG:5253–5259; Sivas TM33, "
            "EPSG:5255) seçin; çizim yerel koordinatlardaysa Özel sistem… ile bu dilimin üstünde yerel sistemi ortak "
            "noktalardan benzerlik dönüşümüyle tanımlayın. ±0,00'ın mutlak kotunu Proje ayarları › Mimari › İmar "
            "durumu'nda (±0,00 kotu) verin.", proje)]

    def s7(self):
        return INCELEME, "EK-8'in gelişim seviyesi müellifin 3B incelemesiyle değerlendirilir; KentOS kaydını tutar.", []

    def s8(self):
        """Md. 12, EK-5 Tablo 5.1: kaynakta beklenen her nesne dosyada; vekil sınıf yok."""
        eksik = [e for e in self.expected if e["globalId"] not in self.by_gid]
        vekil = self.of(VEKIL)
        bulgular = []
        if eksik:
            bulgular.append({**self.bulgu(
                "digital_model_object_missing", f"Modelin {len(eksik)} nesnesi dosyada yok",
                "Dosyayı yeniden yazın; eksik nesne dışa aktarımın kayıp listesinde nedeniyle yazar.", []),
                "uids": [e["uid"] for e in eksik if e.get("uid")]})
        if vekil:
            bulgular.append(self.bulgu(
                "digital_proxy_class", f"{len(vekil)} nesne vekil sınıfta ({VEKIL})",
                "Nesneyi kendi sınıfında yazın (kitaplık bloğunun kartında IFC sınıfını seçin).",
                [o["globalId"] for o in vekil]))
        if not self.expected:
            return INCELEME, "Kaynak modelin nesne listesi verilmedi; sınıflar yalnız vekil sınıf için denetlendi.", bulgular
        if bulgular:
            return KALDI, f"{len(eksik)} nesne eksik, {len(vekil)} vekil sınıfta.", bulgular
        return GECTI, f"Kaynaktaki {len(self.expected)} nesnenin hepsi dosyada, vekil sınıf yok.", []

    def s9(self):
        """Md. 13, EK-6 ve EK-7: katalogun zorunlu satırları dosyanın her nesnesinde dolu. Projenin kendi alanları
        (Tablo 7.1) 2. sorunun bulgusudur: burada sayılır, yinelenmez. Modelin bağlamının alanları (Tablo 7.8) kayıttan
        okunur: 2. soru Tablo 7.1–7.5'i, 6. soru 7.6–7.7'yi okur."""
        gaps = list(self.ifc.get("gaps") or [])
        kayitlar = self.ifc.get("records") or {}
        for r in self.ifc.get("rows") or []:
            if not r.get("mandatory") or not r.get("attribute") or str(r["table"]) != "7.8":
                continue
            eksik = [k for k in kayitlar.get(r["class"]) or [] if not _dolu((k.get("attributes") or {}).get(r["name"]))]
            if eksik:
                gaps.append({"id": r["id"], "class": r["class"], "path": r["name"], "tr": r["tr"], "table": "7.8",
                             "page": r["page"], "missing": [], "of": len(kayitlar.get(r["class"]) or []),
                             "records": [f"#{k['step']}" for k in eksik]})
        bulgular = []
        for g in gaps:
            if g["class"] == "IfcProject" and str(g["table"]).startswith("7.1"):
                continue
            ek = "EK-7" if str(g["table"]).startswith("7.") else "EK-6"
            n = len(g.get("missing") or [])
            if g.get("records"):
                # A record's field (the context's): no object of the drawing to name; KentOS does not write it yet.
                bulgular.append(self.bulgu(
                    "digital_required_field",
                    f"{g['class']} {', '.join(g['records'])}: {g['tr']} ({g['path']}) yok",
                    f"KentOS bu alanı ({g['path']}, {ek} Tablo {g['table']}) IFC'ye henüz yazmıyor; yazıldığında bu bulgu "
                    "kapanır.", [], f"{ek} Tablo {g['table']} · PDF s. {g['page']}"))
                continue
            bulgular.append(self.bulgu(
                "digital_required_field",
                f"{n}/{g['of']} {g['class']} nesnesinde {g['tr']} ({g['path']}) yok",
                zorunlu._onarim(g), list(g.get("missing") or []), f"{ek} Tablo {g['table']} · PDF s. {g['page']}"))
        if gaps:
            ilk = ", ".join(f"{g['class']} {g['tr']} ({len(g.get('missing') or g.get('records') or [])}/{g['of']})"
                            for g in gaps[:4])
            fazla = f" ve {len(gaps) - 4} başka" if len(gaps) > 4 else ""
            return KALDI, f"{len(gaps)} zorunlu alan eksik: {ilk}{fazla}", bulgular
        return GECTI, "EK-6 ve EK-7'nin zorunlu alanları dosyanın her nesnesinde dolu.", []

    def s10(self):
        """Mahal ve mekânsal zon alanları emsal tablosuyla: zonlar dosyada mı, durumu DAHIL/HARIC/DIGER mi."""
        zonlar = self.of("IfcSpatialZone")
        if not zonlar:
            return KALDI, "dosyada mekânsal zon yok: mahallerin emsal durumu (DAHIL, HARIC, DIGER) okunamaz", [self.bulgu(
                "digital_emsal_zones_missing", "Dosyada emsal zonu (IfcSpatialZone) yok",
                "Mahallerin emsal durumunu Mimari › Alan zonlarında verin; dosyayı yeniden yazın.",
                [p["globalId"] for p in self.of("IfcProject")])]
        durumlar: dict[str, int] = {}
        for z in zonlar:
            # EK-6 Tablo 6.60: the zone's emsal status is its national set's; its object type is the zone's kind.
            durum = str((z.get("sets") or {}).get("TREpys_EmsalOzellikSeti", {}).get("EmsalDurumu") or "durumu yok")
            durumlar[durum] = durumlar.get(durum, 0) + 1
        ozet = ", ".join(f"{n} {d}" for d, n in sorted(durumlar.items()))
        return INCELEME, f"{len(zonlar)} zon ({ozet}); alanların emsal hesap tablosuyla karşılaştırılması müellifin.", []

    def s11(self):
        sayi = {c: len(self.of(c)) for c in ("IfcProject", "IfcSite", "IfcBuilding", "IfcBuildingStorey")}
        eksik = [c for c, n in sayi.items() if n == 0]
        if sayi["IfcProject"] > 1:
            eksik.append(f"{sayi['IfcProject']} proje (tek olmalı)")
        if not eksik:
            return GECTI, f"Proje, {sayi['IfcSite']} saha, {sayi['IfcBuilding']} bina, {sayi['IfcBuildingStorey']} kat.", []
        return KALDI, "eksik: " + ", ".join(eksik), [self.bulgu(
            "digital_structure_missing", "Mekânsal yapıda eksik: " + ", ".join(eksik),
            "Projenin katlarını Kat penceresinde tanımlayın; dosyayı yeniden yazın.", [])]

    def s12(self):
        n = len(self.of("IfcSpace"))
        if n:
            return GECTI, f"{n} mahal.", []
        return KALDI, "dosyada mahal yok", [self.bulgu(
            "digital_no_spaces", "Mimari modelde mahal (IfcSpace) yok",
            "Mimari › Mahal ile odaları mahal olarak çizin (Toplu mahal bul duvarlardan bulur); IFC'de her biri bir "
            "IfcSpace olur.", [])]

    def s13(self):
        """Gerçek çokgenle örtüşme (aynı katta, eşiğin üstünde) ve mahalin tabanı katının kot aralığında."""
        if not self.bodies:
            return INCELEME, "Gövdeler ölçülmedi; örtüşme ve kat denetlenemedi.", []
        bulgular = []
        for o in self.bodies.get("overlaps") or []:
            if float(o.get("area") or 0.0) < ESIK:
                continue
            a, b = self.by_gid.get(o["a"], {}), self.by_gid.get(o["b"], {})
            bulgular.append(self.bulgu(
                "digital_quality_room_overlap",
                f"“{self.ad(a)}” ile “{self.ad(b)}” {float(o['area']):.2f} m² örtüşüyor",
                "İki mahalden birinin sınırını ortak duvarın yüzüne çekin ya da fazla olan mahali silin.",
                [o["a"], o["b"]]))
        for s in self.bodies.get("spaces") or []:
            band = self.band(s.get("storey"))
            if band is None:
                continue
            z, pay = float(s["zMin"]), float(s.get("rounding") or 0.0)
            if z < band[0] - pay or z >= band[1] + pay:
                o = self.by_gid.get(s["globalId"], {})
                bulgular.append(self.bulgu(
                    "digital_space_wrong_storey",
                    f"“{self.ad(o)}” mahalinin tabanı {z:.2f} m, katının aralığında ({band[0]:.2f}–{band[1]:.2f}) değil",
                    "Mahali doğru katta çizin ya da katın kotunu düzeltin.", [s["globalId"]]))
        if bulgular:
            return KALDI, f"{len(bulgular)} mahal bulgusu.", bulgular
        return GECTI, "Mahaller örtüşmüyor, her biri kendi katında.", []

    def s14(self):
        """Mahal kapalı katı; üstü üst katın kotunu aşmıyor (döşemede ya da çatıda kalır)."""
        if not self.bodies:
            return INCELEME, "Gövdeler ölçülmedi; kapalılık denetlenemedi.", []
        bulgular = []
        for s in self.bodies.get("spaces") or []:
            o = self.by_gid.get(s["globalId"], {})
            if s.get("closed") is not True:
                bulgular.append(self.bulgu(
                    "digital_space_open", f"“{self.ad(o)}” mahalinin gövdesi kapalı değil",
                    "Mahalin sınırını kapalı bir çizgiyle yeniden bulun.", [s["globalId"]]))
            band = self.band(s.get("storey"))
            if band and band[1] != float("inf") and float(s["zMax"]) > band[1] + float(s.get("rounding") or 0.0):
                bulgular.append(self.bulgu(
                    "digital_space_top_wrong",
                    f"“{self.ad(o)}” mahalinin üstü {float(s['zMax']):.2f} m, üst katın kotunu ({band[1]:.2f}) aşıyor",
                    "Mahalin yüksekliğini katın yüksekliğine çekin.", [s["globalId"]]))
        if bulgular:
            return KALDI, f"{len(bulgular)} mahal bulgusu.", bulgular
        n = len(self.bodies.get("spaces") or [])
        return GECTI, f"{n} mahalin gövdesi kapalı, üstü katının içinde.", []

    def s15(self):
        """Her elemanın katı var ve gövdesi katının kot aralığına değiyor."""
        if not self.bodies:
            return INCELEME, "Gövdeler ölçülmedi; kot denetlenemedi.", []
        bulgular = []
        govde = {b["globalId"]: b for b in self.bodies.get("bodies") or []}
        katsiz, disarida = [], []
        for o in self.objects:
            if o["class"] in YAPI or o["globalId"] not in govde:
                continue
            if not o.get("storey"):
                katsiz.append(o)
                continue
            band = self.band(o["storey"])
            b = govde[o["globalId"]]
            pay = float(b.get("rounding") or 0.0)
            if band and (float(b["zMax"]) < band[0] - pay or float(b["zMin"]) >= band[1] + pay):
                disarida.append((o, b, band))
        if katsiz:
            bulgular.append(self.bulgu(
                "digital_entity_no_storey", f"{len(katsiz)} nesnenin katı yok",
                "Nesneyi bir kata atayın (Kat › Kata ata).", [o["globalId"] for o in katsiz]))
        for o, b, band in disarida:
            bulgular.append(self.bulgu(
                "digital_entity_wrong_storey",
                f"“{self.ad(o)}” {float(b['zMin']):.2f}–{float(b['zMax']):.2f} m'de, katının aralığında "
                f"({band[0]:.2f}–{band[1]:.2f}) değil",
                "Nesneyi doğru kata atayın ya da kotunu düzeltin.", [o["globalId"]]))
        if bulgular:
            return KALDI, f"{len(katsiz)} katsız, {len(disarida)} katının dışında nesne.", bulgular
        return GECTI, "Her nesnenin katı var, gövdesi katının aralığında.", []

    def s16(self):
        """Yinelenen GlobalId (okuma bulgusu); aynı sınıf ve adda aynı yükseklikte iki gövde."""
        ikiz = [f for f in self.ifc.get("findings") or [] if f.get("code") == "global_id_duplicate"]
        bulgular = []
        if ikiz:
            bulgular.append(self.bulgu(
                "digital_duplicate_global_id", f"{len(ikiz)} GlobalId iki nesnede: " +
                "; ".join(f["message"] for f in ikiz[:3]), "Dosyayı yeniden yazın; elle düzenlenmiş dosyada kimlikleri "
                "tekilleştirin.", []))
        govde = {b["globalId"]: b for b in self.bodies.get("bodies") or []}
        # Ad EK-5'in kalıbında türün adıdır (bir katın sekiz kolonu aynı adı taşır): aynı ad ve kattakiler arasında
        # gövdesi aynı yerde olanlar aranır.
        gorulen: dict[tuple, list[str]] = {}
        for o in self.objects:
            if o["globalId"] in govde and o["class"] not in YAPI:
                gorulen.setdefault((o["class"], o.get("name"), o.get("storey")), []).append(o["globalId"])
        for (sinif, ad, _), gids in gorulen.items():
            for kume in self._ust_uste(gids, govde):
                bulgular.append(self.bulgu(
                    "digital_duplicate_entity", f"{len(kume)} {sinif} “{ad}” aynı yerde",
                    "Yinelenen nesneyi silin.", kume))
        if bulgular:
            return KALDI, f"{len(bulgular)} yineleme bulgusu.", bulgular
        return GECTI, "Yinelenen kimlik ya da üst üste aynı nesne yok.", []

    @staticmethod
    def _ust_uste(gids: list[str], govde: dict[str, dict[str, Any]]) -> list[list[str]]:
        """Gövdeleri aynı yerde olan nesnelerin kümeleri: alt ve üst kotu ve plandaki kutusu, iki gövdenin ölçü
        sınırının iki katı içinde aynı (yalnız ikiden çok üyeli kümeler). Kümeler alt kotlarına ve kutunun batı kenarına
        göre, hücresi bu sınırdan küçük olmayan bir ızgarada aranır: aynı yerdeki iki gövde aynı ya da komşu hücrededir."""
        yerler: list[tuple[str, list[float], float]] = []
        for g in gids:
            b = govde[g]
            kutu = b.get("box")
            if kutu:
                yerler.append((g, [float(b["zMin"]), float(b["zMax"]), *map(float, kutu)], float(b.get("rounding") or 0.0)))
        if not yerler:
            return []
        hucre = max(2.0 * max(p for _, _, p in yerler), 1e-9)
        kumeler: list[tuple[list[float], float, list[str]]] = []
        izgara: dict[tuple[int, int], list[int]] = {}
        for g, y, pay in yerler:
            h = (math.floor(y[0] / hucre), math.floor(y[2] / hucre))
            bulunan = None
            for dz in (-1, 0, 1):
                for dx in (-1, 0, 1):
                    for k in izgara.get((h[0] + dz, h[1] + dx), ()):
                        ilk, ilk_pay, _ = kumeler[k]
                        sinir = 2.0 * max(pay, ilk_pay)
                        if all(abs(a - b) <= sinir for a, b in zip(y, ilk)):
                            bulunan = k
                            break
                    if bulunan is not None:
                        break
                if bulunan is not None:
                    break
            if bulunan is None:
                kumeler.append((y, pay, [g]))
                izgara.setdefault(h, []).append(len(kumeler) - 1)
            else:
                kumeler[bulunan][2].append(g)
        return [uyeler for _, _, uyeler in kumeler if len(uyeler) > 1]

    def s17(self):
        """Sınıf ile kaynağın eşlemesi: beklenen sınıfla yazılan aynı; vekil yok."""
        yanlis = []
        for e in self.expected:
            o = self.by_gid.get(e["globalId"])
            if o and e.get("class") and o["class"].lower() != str(e["class"]).lower():
                yanlis.append((o, e["class"]))
        vekil = self.of(VEKIL)
        bulgular = []
        if yanlis:
            ornek = ", ".join(f"“{self.ad(o)}” {o['class']} (beklenen {c})" for o, c in yanlis[:4])
            bulgular.append(self.bulgu(
                "digital_class_wrong", f"{len(yanlis)} nesne beklenen sınıfta değil: {ornek}",
                "Dosyayı yeniden yazın; sınıf çizimdeki nesnenin türünden gelir.", [o["globalId"] for o, _ in yanlis]))
        if vekil:
            bulgular.append(self.bulgu(
                "digital_proxy_class", f"{len(vekil)} nesne vekil sınıfta ({VEKIL})",
                "Nesneyi kendi sınıfında yazın.", [o["globalId"] for o in vekil]))
        if bulgular:
            return KALDI, f"{len(yanlis)} yanlış sınıf, {len(vekil)} vekil.", bulgular
        if not self.expected:
            return INCELEME, "Kaynak modelin nesne listesi verilmedi; yalnız vekil sınıf denetlendi.", []
        return GECTI, "Sınıflar kaynağın eşlemesiyle aynı, vekil yok.", []

    def s18(self):
        """Başlıktaki şema adı profilinki (IFC4X3_ADD2, EK-5 m. 5.4), uzantı .ifc."""
        istenen = (self.ifc.get("profile") or {}).get("schema") or "IFC4X3_ADD2"
        sema = self.file.get("schema")
        uzanti = self.file.get("extension")
        sorun = []
        if sema != istenen:
            sorun.append(f"şema {sema} (istenen {istenen})")
        if uzanti != "ifc":
            sorun.append(f"uzantı .{uzanti} (istenen .ifc)")
        if not sorun and (self.ifc.get("export") or {}).get("memory"):
            # Bellekte yazılan dosya teslim edilen değildir: onun sürümü ve uzantısı müellifin denetimidir.
            return INCELEME, (f"Teslim edilen dosyayı denetleyin: KentOS modeli IFC 4.3 ({sema}) şemasıyla ve .ifc "
                              "uzantısıyla yazar; bu form bellekte yazılan dosyayı okudu."), []
        if not sorun:
            return GECTI, f"{sema}, .ifc.", []
        return KALDI, "; ".join(sorun), [self.bulgu(
            "digital_version_wrong", "Dosya: " + "; ".join(sorun),
            "Dosyayı KentOS'un IFC 4.3 dışa aktarımıyla .ifc olarak yazın.", [])]

    def s19(self):
        """Ön tanımlı tip: NOTDEFINED belirlenebilen tipin yerine kullanılmaz; USERDEFINED tip adını ister."""
        tanimsiz, adsiz = [], []
        for o in self.objects:
            p = o.get("predefinedType")
            if p == "NOTDEFINED":
                tanimsiz.append(o)
            elif p == "USERDEFINED" and not _dolu(o.get("objectType")):
                adsiz.append(o)
        bulgular = []
        if tanimsiz:
            bulgular.append(self.bulgu(
                "digital_predefined_notdefined",
                f"{len(tanimsiz)} nesnenin ön tanımlı tipi NOTDEFINED: " + ", ".join(self.ad(o) for o in tanimsiz[:4]),
                "Nesneyi tipi olan bir araçla yeniden çizin (merdivende kol biçimi, çatıda biçim); belirlenemiyorsa "
                "gerekçesini incelemeye yazın.", [o["globalId"] for o in tanimsiz]))
        if adsiz:
            bulgular.append(self.bulgu(
                "digital_predefined_userdefined_unnamed",
                f"{len(adsiz)} nesne USERDEFINED tipte ama tip adı (ObjectType) yok",
                "USERDEFINED tipli nesnenin tip adını yazın.", [o["globalId"] for o in adsiz]))
        if bulgular:
            return KALDI, f"{len(tanimsiz)} NOTDEFINED, {len(adsiz)} adsız USERDEFINED.", bulgular
        return GECTI, "Ön tanımlı tipler belirli; USERDEFINED olanların tip adı var.", []

    def s20(self):
        """Her standart set IFC 4.3'te var, sınıfına uygun, özellikleri setinin."""
        if self.file.get("schema") != ((self.ifc.get("profile") or {}).get("schema") or "IFC4X3_ADD2"):
            return INCELEME, "Dosya IFC 4.3 değil; setleri kendi sürümünün sözlüğüyle denetlenmeli.", []
        denetim = self.ifc.get("setChecks") or []
        if not denetim:
            return GECTI, "Standart setler IFC 4.3'ün sözlüğünde, sınıflarına uygun; özellikler setlerinin.", []
        neden = {"unknown_set": "IFC 4.3'te böyle bir set yok", "not_for_class": "set bu sınıf için değil",
                 "unknown_item": "özellik bu setin değil"}
        bulgular = []
        for c in denetim:
            yol = c["set"] + (f".{c['item']}" if c.get("item") else "")
            bulgular.append(self.bulgu(
                "digital_pset_wrong", f"{yol} ({c['class']}): {neden.get(c['why'], c['why'])}",
                "Özelliği EK-6'daki setinde yazın.", [c["globalId"]]))
        return KALDI, f"{len(bulgular)} set bulgusu.", bulgular

    def s21(self):
        """Paftalar modelden: dışa aktarımın modele almadığı nesne yok (BIM-04'ün kaydı ayrıca türetimdedir)."""
        left = ((self.ifc.get("export") or {}).get("left")) or []
        if not left:
            if "export" not in self.ifc:
                return INCELEME, "Dışa aktarımın kaydı verilmedi; paftaların modelden üretildiği denetlenemedi.", []
            return GECTI, "Çizimin her nesnesi modelde; paftalar aynı modelden.", []
        sayi: dict[str, int] = {}
        for x in left:
            sayi[str(x.get("what"))] = sayi.get(str(x.get("what")), 0) + 1
        ne = ", ".join(f"{n} {w}" for w, n in sayi.items())
        return KALDI, f"{len(left)} nesne modele girmedi ({ne})", [{**self.bulgu(
            "digital_model_loss", f"Çizimin nesneleri IFC modeline girmiyor: {ne}",
            "Gövdesi kurulamayan nesneyi (yüksekliği olmayan duvar …) düzeltin, açıklığı bir duvara bağlayın.", []), "uids": [x["uid"] for x in left if x.get("uid")]}]


def _yerel(k: dict[str, Any]) -> str:
    """Yerel sistemde çizilmiş modelin haritaya oturuşu: dönüşü, ölçeği ve başlangıcı; sistemin kendisinde çizilene
    hiçbir şey. Ölçek ``Scale`` ile ölçekli dönüşümün çarpanlarının çarpımıdır."""
    a, o = float(k.get("XAxisAbscissa") or 1.0), float(k.get("XAxisOrdinate") or 0.0)
    olcek = float(k.get("Scale") or 1.0)
    fx, fy = olcek * float(k.get("FactorX") or 1.0), olcek * float(k.get("FactorY") or 1.0)
    if o == 0.0 and a > 0.0 and fx == 1.0 and fy == 1.0:
        return ""
    don = math.degrees(math.atan2(o, a))
    olcu = f"ölçeği {fx:.6f}" if fx == fy else f"ölçekleri {fx:.6f} ve {fy:.6f}"
    e, n = float(k.get("Eastings") or 0.0), float(k.get("Northings") or 0.0)
    return f"; yerel sistem {don:.4f}° döndürülmüş, {olcu}, başlangıcı {e:.3f} {n:.3f}"


def _dolu(v: Any) -> bool:
    """Bir değerin verildiği: boş yazı, boş liste ve null değer değildir."""
    if v is None:
        return False
    if isinstance(v, str):
        return bool(v.strip())
    if isinstance(v, (list, tuple, dict)):
        return bool(v)
    return True


def _varsayilan(ad: str | None) -> bool:
    """Numarası ve adı olmayan mahalin KentOS'un verdiği adı (MM-MHL-Mahal)."""
    return not ad or ad.strip().endswith("-Mahal")


def _dosya_adi(govde: str, data: dict[str, Any]) -> list[str]:
    """EK-2 Tablo 2.1'in şablonu: dokuz alan, uzunlukları ve türleri; proje kodu verilmiş; disiplin Tablo 2.2'de."""
    alanlar = govde.split("_")
    sablon = data.get("fileTemplate") or []
    if len(alanlar) != len(sablon):
        return [f"dosyanın adı {len(alanlar)} alan, EK-2 Tablo 2.1'in şablonu {len(sablon)} alan "
                f"({'_'.join(s['name'] for s in sablon)}); proje kodu (EK-1) verilmemiş olabilir"]
    sorun = []
    for deger, s in zip(alanlar, sablon):
        tur = {"Alfanümerik": r"[A-Z0-9]", "Alfabetik": r"[A-Z]", "Sayısal": r"[0-9]"}.get(s["kind"], r".")
        if not re.fullmatch(rf"{tur}{{{s['length']}}}", deger):
            sorun.append(f"{s['name']} “{deger}” {s['length']} karakter {s['kind'].lower()} değil")
    if alanlar[0] == "000000":
        sorun.append("proje kodu (EK-1) verilmemiş: 000000")
    disiplinler = {d["code"] for d in data.get("disciplines") or []}
    if alanlar[2] not in disiplinler:
        sorun.append(f"disiplin “{alanlar[2]}” EK-2 Tablo 2.2'de yok")
    return sorun
