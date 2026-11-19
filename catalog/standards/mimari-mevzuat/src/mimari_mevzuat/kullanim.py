"""Kullanım sınıfına özel kaçış hükümleri: Binaların Yangından Korunması Hakkında Yönetmelik 49/1-a, 50/1-a, 50/1-b ve
52 (docs/adr/8107; plan/bim-plan.md B22, BIM-13'ün konut dışı kullanımları).

- **Kullanım sınıfı (8–17. maddeler):** binanın kullanımı bina programının yazdığından (``uses``), o yoksa mahallerin
  adlarından okunur. Sağlık yapısı (11/1-b: hastane, huzurevi, bakımevi, klinik), konaklama (10: otel, motel,
  pansiyon, yurt, yatakhane) ve 52'nin binaları (fabrika, imalathane, mağaza, dükkân, depo, büro, ayakta tedavi
  merkezi). Okunamayan kullanım kuralı kapsam dışı bırakır; satır nedenini söyler.
- **49/1-a:** sağlık yapısında kullanıcı yükü 15 kişiyi aşan hasta yatak odası ya da süit (adı Hasta, Koğuş, Yatak
  odası, Süit) için en az 2 kapı. "Birbirinden uzakta" bir ölçü değildir: kapıların yeri satırın notunda idareye
  bırakılır.
- **50/1-a:** otelde yatak odaları iç koridordan en az 60 dakika yangına dayanıklı duvarla ayrılır (iki mahal
  arasındaki duvarların beyan edilen sınıfı; açıklıkları 50/1-b'nin işi). Toplam yatak sayısı 20'den fazla ya da kat
  sayısı ikiden fazla otelde her katta en az 2 çıkış (yatak odası olan her katta: kaçış merdivenleri, zemin katta dışarı
  açılan kapılar da). Yatak sayısı ve merdivenin korunumu ya da basınçlandırılması çizimde yoktur: tek çıkışlı katta
  yapı yüksekliği 15,50 m ve üstündeyse kaldı, altındaysa eksik veri (yatak 20'den az, merdiven korunumlu ya da
  basınçlandırılmışsa tek merdiven yeter).
- **50/1-b:** iç koridora açılan yatak odası kapıları en az 30 dakika yangına dayanıklı ve kendiliğinden kapanır
  (``C``). Beyan yoksa eksik veri.
- **52 (9.9.2009):** fabrika, imalathane, mağaza, dükkân, depo, büro ve ayakta tedavi merkezinde en az 2 bağımsız kaçış
  merdiveni ya da başka çıkış; zemin kattan başka mahalli her katta sayılan kaçış merdivenleridir. Tek merdiven beş
  koşulun hepsi birlikte tutunca yeter: (a) yapı yüksekliği 21,50 m'den az, (b) her katın kullanıcısı 50'den az,
  (c) kaçış uzaklıkları Ek-5/B'ye uygun, (ç) yapımda yanmaz ürünler, (d) kolay alevlenen madde kullanılmıyor. (a) ya
  da (b) tutmazsa kaldı; tutarsa (c)–(ç)–(d) çizimden kanıtlanmaz: uzman incelemesi (kaçış uzaklıklarını Mimari
  denetimin BYKHY 32 bulguları söyler).

**Okunan olgular** (``facts.building``, KentOS'un ölçtüğü): binaların kullanımı, katlar, mahaller (adı, katı, kaçış
merdiveni olup olmadığı), kullanıcı yükü olan mahaller (Ek-5/A'ya göre kişi), kapılar (iki yanı, beyan edilen yangın
sınıfı), her iki mahal arasındaki ayrımın duvarları ve beyan edilen sınıfları, yapı yüksekliği.
"""

from __future__ import annotations

from datetime import date
from typing import Any

from . import katman
from . import yururluk as y
from .yangin_sinifi import sinif

MADDELER = [("BYKHY", "49/1-a"), ("BYKHY", "50/1-a"), ("BYKHY", "50/1-b"), ("BYKHY", "52")]

#: Bir kat zemin kat sayılır, kotu ±0,00'a bundan yakınsa, m.
_ZEMIN = 0.01

_SAGLIK = ("HASTANE", "HUZUREVI", "BAKIMEVI", "KLINIK", "SAGLIK", "REHABILITASYON")
#: Yurt: "yurdu" da (ünsüz yumuşaması).
_KONAKLAMA = ("OTEL", "MOTEL", "PANSIYON", "KONAKLAMA", "YURT", "YURD", "YATAKHANE", "HOSTEL")
#: 52'nin saydığı binalar: fabrika, imalathane, mağaza, dükkân, depo, büro, ayakta tedavi merkezi.
_MADDE_52 = ("FABRIKA", "IMALAT", "MAGAZA", "DUKKAN", "DEPO", "BURO", "OFIS", "POLIKLINIK", "DISPANSER")


def _katla(metin: str) -> str:
    tablo = str.maketrans({"ç": "C", "Ç": "C", "ğ": "G", "Ğ": "G", "ı": "I", "İ": "I", "i": "I", "ö": "O", "Ö": "O",
                           "ş": "S", "Ş": "S", "ü": "U", "Ü": "U", "â": "A", "Â": "A", "î": "I", "Î": "I"})
    return metin.translate(tablo).upper()


def _sozcukler(metin: str | None) -> list[str]:
    return [w for w in "".join(c if c.isalnum() else " " for c in _katla(metin or "")).split() if w]


def _var(w: list[str], kokler: tuple[str, ...]) -> bool:
    return any(x.startswith(k) for x in w for k in kokler)


def _ayakta(w: list[str]) -> bool:
    return any(a.startswith("AYAKTA") and b.startswith("TEDAVI") for a, b in zip(w, w[1:]))


def siniflar(bina: dict[str, Any]) -> tuple[set[str], str]:
    """Binanın bu kuralın okuduğu kullanım sınıfları (``saglik``, ``konaklama``, ``m52``) ve nereden okunduğu.

    Bina programının kullanımı yazılmışsa ondan. Yazılmamışsa mahallerin adlarından, sıkı okumayla: yalnız konut
    bölümlü bina bunların hiçbiri değildir; hasta ya da koğuş adı sağlık, otel, motel, pansiyon, yurt ya da yatakhane
    adı konaklama yapısı yapar; ikisi de yoksa mağaza, dükkân, büro, ofis, fabrika, imalathane ya da ayakta tedavi adı
    52'nin binası. Depo adı tek başına saymaz: binanın içindeki küçük depo ve başka binanın bürosu ana binanın sınıfına
    girer (16/2, 12/2)."""
    kullanim = list(bina.get("uses") or [])
    if kullanim:
        out = set()
        for m in kullanim:
            w = _sozcukler(m)
            if _ayakta(w) or _var(w, _MADDE_52):
                out.add("m52")
            elif _var(w, _SAGLIK):
                out.add("saglik")
            if _var(w, _KONAKLAMA):
                out.add("konaklama")
        return out, "bina programının kullanımı (“" + "”, “".join(kullanim) + "”)"
    nereden = "mahallerin adları (bina programında kullanım yazılmamış)"
    konut, bolum = int(bina.get("dwellings") or 0), int(bina.get("units") or 0)
    if konut > 0 and konut == bolum:
        return set(), nereden
    adlar = [_sozcukler(s.get("name")) for s in bina.get("spaces") or []]
    out = set()
    if any(_var(w, ("HASTA", "KOGUS")) for w in adlar):
        out.add("saglik")
    if any(_var(w, _KONAKLAMA) for w in adlar):
        out.add("konaklama")
    if not out and any(_ayakta(w) or _var(w, tuple(k for k in _MADDE_52 if k != "DEPO")) for w in adlar):
        out.add("m52")
    return out, nereden


def _koridor(ad: str | None) -> bool:
    w = _sozcukler(ad)
    return any(x.startswith("KORIDOR") for x in w) or any(x in ("HOL", "HOLU") for x in w)


def _otel_odasi(ad: str | None) -> bool:
    w = _sozcukler(ad)
    if not w:
        return False
    if any(x.startswith(k) for x in w for k in ("YATAK", "SUIT", "KONUK", "MISAFIR")):
        return True
    return w[0] == "ODA" or (len(w) >= 2 and w[0] == "OTEL" and w[1].startswith("ODA"))


def _hasta_odasi(ad: str | None) -> bool:
    w = _sozcukler(ad)
    return any(x.startswith(k) for x in w for k in ("HASTA", "KOGUS", "SUIT")) or (
        any(x.startswith("YATAK") for x in w) and any(x.startswith("ODA") for x in w))


def _kisi(n: float) -> str:
    return f"{n:.0f}" if abs(n - round(n)) < 1e-9 else f"{n:.1f}"


def _m(x: float) -> str:
    return f"{x:.2f}".replace(".", ",")


def _satir(hal: y.Hal, durum: str, not_: str) -> dict[str, Any]:
    return {"rule": f"{hal.yonetmelik} {hal.madde}", "title": hal.baslik, "version": hal.etiket, "gazette": hal.rg,
            "state": durum, "note": not_}


def _bulgu(code: str, ileti: str, onarim: str, hal: y.Hal, uids: list[str], iz: str,
           state: str | None = None) -> dict[str, Any]:
    out: dict[str, Any] = {"code": code, "message": f"{ileti} — {hal.kisa()}", "repair": onarim, "uids": uids,
                           "rule": hal.kural(), "_iz": iz}
    if state:
        out["state"] = state
    return out


def _bilinmiyor(madde: str, gun: date) -> dict[str, Any]:
    ilk = y.haller("BYKHY", madde)[0]
    return {"rule": f"BYKHY {madde}", "title": ilk.baslik, "version": None, "gazette": None, "state": y.UZMAN,
            "note": f"başvuru günü ({y.yaz(gun)}) kural setinin bildiği ilk hâlden ({ilk.etiket}) önce"}


def _kapilari(bina: dict[str, Any], uid: str) -> list[dict[str, Any]]:
    return [k for k in bina.get("doors") or [] if uid in (k.get("between") or [])]


def _katlar(bina: dict[str, Any]) -> list[dict[str, Any]]:
    return sorted(bina.get("storeys") or [], key=lambda s: float(s.get("elevation") or 0.0))


def _zemin(kat: dict[str, Any]) -> bool:
    return abs(katman.kot(kat)) < _ZEMIN


def _cikislar(bina: dict[str, Any], kat: dict[str, Any]) -> int:
    """Bir katın çıkışları: kaçış merdivenleri; zemin katta dışarı açılan kapılar da."""
    seviye = kat["level"]
    n = sum(1 for s in bina.get("spaces") or [] if s.get("stair") and s.get("level") == seviye)
    if _zemin(kat):
        n += sum(1 for k in bina.get("doors") or []
                 if k.get("level") == seviye and None in (k.get("between") or [None, None]))
    return n


def hasta_odalari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """49/1-a: kalabalık hasta yatak odalarının iki kapısı."""
    sinir = float(hal.anahtar["persons"])
    odalar = [m for m in bina.get("rooms") or [] if _hasta_odasi(m.get("name")) and float(m.get("persons") or 0) > sinir]
    bulgular = []
    for m in odalar:
        kapilar = _kapilari(bina, m["uid"])
        if len(kapilar) < 2:
            bulgular.append(_bulgu(
                "bykhy_patient_room_doors",
                f"“{m.get('name') or 'Hasta odası'}” ({_kisi(float(m['persons']))} kişi, 15'ten fazla): {len(kapilar)} "
                f"kapısı var, birbirinden uzakta 2 kapı gerekir", "Odaya öbür kapıdan uzak ikinci bir kapı açın.", hal,
                [m["uid"]] + [k["uid"] for k in kapilar], f"{m['uid']}:kapı"))
    if not odalar:
        satir = _satir(hal, y.KAPSAM_DISI, "kullanıcı yükü 15 kişiyi aşan hasta yatak odası yok")
    elif bulgular:
        satir = _satir(hal, y.KALDI, f"{len(bulgular)} hasta odasının 2 kapısı yok")
    else:
        satir = _satir(hal, y.GECTI, f"{len(odalar)} kalabalık hasta odasının 2 kapısı var; birbirinden uzaklıkları "
                                      "idarenin takdirinde")
    return bulgular, satir


def otel_katlari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """50/1-a: yatak odalarının iç koridordan ayrımı ve katların çıkışları."""
    dakika = int(hal.anahtar["minutes"])
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    odalar = {u for u, s in mahaller.items() if _otel_odasi(s.get("name"))}
    if not odalar:
        return [], _satir(hal, y.KAPSAM_DISI, "yatak odası yok")
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = 0
    for a in bina.get("separations") or []:
        iki = a.get("rooms") or []
        oda = next((u for u in iki if u in odalar), None)
        koridor = next((u for u in iki if u != oda and _koridor((mahaller.get(u) or {}).get("name"))), None)
        if oda is None or koridor is None:
            continue
        ad = mahaller[oda].get("name") or "Yatak odası"
        for d in a.get("walls") or []:
            beyan = d.get("fireRating")
            okunan = sinif(beyan)
            uids = [d["uid"], oda, koridor]
            if okunan is None:
                eksik += 1
                neden = f"“{beyan}” okunamadı" if beyan else "yangın sınıfı beyan edilmemiş"
                bulgular.append(_bulgu(
                    "bykhy_hotel_room_wall",
                    f"“{ad}” ile iç koridor arasındaki duvarın {neden}; 50/1-a en az {dakika} dakika ister", "Duvarın "
                    f"(ya da stilinin) yangın dayanımını beyan edin (EI {dakika}).", hal, uids, f"{d['uid']}:beyan",
                    y.EKSIK_VERI))
            elif okunan["dakika"] < dakika:
                kalan += 1
                bulgular.append(_bulgu(
                    "bykhy_hotel_room_wall",
                    f"“{ad}” ile iç koridor arasındaki duvar “{beyan}”: {okunan['dakika']} dakika < {dakika} dakika",
                    f"Duvarı en az {dakika} dakika yangına dayanıklı yapın ve beyan edin (EI {dakika}).", hal, uids,
                    f"{d['uid']}:sınıf"))
    # Her katta en az 2 çıkış: yatak odası olan katlar.
    katlar = [k for k in _katlar(bina) if any(mahaller[u].get("level") == k["level"] for u in odalar)]
    cok_katli = len(_katlar(bina)) > int(hal.anahtar["storeys"])
    yuksek = float(bina.get("structureHeight") or 0.0) >= float(hal.anahtar["height"])
    tek = [(k, _cikislar(bina, k)) for k in katlar]
    tek = [(k, n) for k, n in tek if n < 2]
    for k, n in tek:
        uids = [u for u in odalar if mahaller[u].get("level") == k["level"]]
        if cok_katli and yuksek:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_hotel_storey_exits",
                f"{k.get('name') or k['level']}: {n} çıkış; kat sayısı ikiden fazla otelde her katta en az 2 çıkış "
                f"gerekir, yapı yüksekliği {_m(float(bina.get('structureHeight') or 0))} m 15,50 m'nin altında değil",
                "Kata ikinci bir kaçış merdiveni ya da çıkış ekleyin.", hal, uids, f"{k['level']}:çıkış"))
        else:
            eksik += 1
            neden = ("kat sayısı ikiden fazla; yatak 20'den az ve merdiven korunumlu ya da basınçlandırılmışsa tek "
                     "merdiven yeter") if cok_katli else "toplam yatak 20'den fazlaysa her katta 2 çıkış gerekir"
            bulgular.append(_bulgu(
                "bykhy_hotel_storey_exits",
                f"{k.get('name') or k['level']}: {n} çıkış; {neden}: yatak sayısı ve merdivenin korunumu çizimde yok",
                "Otelin toplam yatak sayısını ve merdivenin korunumlu ya da basınçlandırılmış olduğunu belgeleyin; "
                "gerekiyorsa ikinci çıkış ekleyin.", hal, uids, f"{k['level']}:çıkış", y.EKSIK_VERI))
    if kalan:
        satir = _satir(hal, y.KALDI, f"{kalan} yetmeyen ayrım ya da kat")
    elif eksik:
        satir = _satir(hal, y.EKSIK_VERI, f"{eksik} beyansız duvar ya da yatak sayısına bağlı kat")
    else:
        satir = _satir(hal, y.GECTI, f"{len(odalar)} yatak odası koridordan en az {dakika} dakika ayrılmış, "
                                     f"{len(katlar)} katın her birinde en az 2 çıkış")
    return bulgular, satir


def otel_kapilari(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """50/1-b: iç koridora açılan yatak odası kapıları."""
    dakika = int(hal.anahtar["minutes"])
    mahaller = {s["uid"]: s for s in bina.get("spaces") or []}
    kapilar = []
    for k in bina.get("doors") or []:
        iki = [u for u in k.get("between") or [] if u]
        oda = next((u for u in iki if _otel_odasi((mahaller.get(u) or {}).get("name"))), None)
        if oda and any(u != oda and _koridor((mahaller.get(u) or {}).get("name")) for u in iki):
            kapilar.append((k, oda))
    bulgular: list[dict[str, Any]] = []
    kalan = eksik = 0
    for k, oda in kapilar:
        ad = mahaller[oda].get("name") or "Yatak odası"
        beyan = k.get("fireRating")
        okunan = sinif(beyan)
        uids = [k["uid"], oda]
        if okunan is None:
            eksik += 1
            neden = f"“{beyan}” okunamadı" if beyan else "yangın sınıfı beyan edilmemiş"
            bulgular.append(_bulgu(
                "bykhy_hotel_room_door",
                f"“{ad}” kapısının {neden}; 50/1-b iç koridora açılan kapıda en az {dakika} dakika ve kendiliğinden "
                "kapanma ister", f"Kapının (ya da tipinin) yangın sınıfını beyan edin (EI2 {dakika}-C5).", hal, uids,
                f"{k['uid']}:beyan", y.EKSIK_VERI))
            continue
        eksikler = []
        if okunan["dakika"] < dakika:
            eksikler.append(f"{okunan['dakika']} dakika < {dakika} dakika")
        if not okunan["kapanma"]:
            eksikler.append("kendiliğinden kapanmıyor (C yok)")
        if eksikler:
            kalan += 1
            bulgular.append(_bulgu(
                "bykhy_hotel_room_door", f"“{ad}” kapısı “{beyan}”: " + "; ".join(eksikler),
                f"Kapıyı en az {dakika} dakika dayanıklı ve kendiliğinden kapanan yapın, beyan edin (EI2 {dakika}-C5).",
                hal, uids, f"{k['uid']}:sınıf"))
    if not kapilar:
        satir = _satir(hal, y.KAPSAM_DISI, "iç koridora açılan yatak odası kapısı yok")
    elif kalan:
        satir = _satir(hal, y.KALDI, f"{kalan} kapı yetmiyor")
    elif eksik:
        satir = _satir(hal, y.EKSIK_VERI, f"{eksik} kapının yangın sınıfı beyan edilmemiş ya da okunamadı")
    else:
        satir = _satir(hal, y.GECTI, f"{len(kapilar)} kapı en az {dakika} dakika ve kendiliğinden kapanıyor")
    return bulgular, satir


def merdivenler(bina: dict[str, Any], hal: y.Hal) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    """52: iki bağımsız kaçış merdiveni ya da tek merdivenin beş koşulu."""
    mahaller = bina.get("spaces") or []
    insanlar = bina.get("rooms") or []
    katlar = [k for k in _katlar(bina) if not _zemin(k)
              and any(s.get("level") == k["level"] and not s.get("stair") for s in mahaller)]
    if not katlar:
        return [], _satir(hal, y.KAPSAM_DISI, "zemin kattan başka mahalli kat yok: çıkışları 39/2 ve 32/8 söyler")
    tek = []
    for k in katlar:
        merdiven = [s["uid"] for s in mahaller if s.get("stair") and s.get("level") == k["level"]]
        if len(merdiven) < 2:
            tek.append((k, merdiven))
    if not tek:
        return [], _satir(hal, y.GECTI, f"{len(katlar)} katın her birinde en az 2 kaçış merdiveni")
    yukseklik = float(bina.get("structureHeight") or 0.0)
    sinir_y, sinir_k = float(hal.anahtar["height"]), float(hal.anahtar["persons"])
    kisi: dict[str, float] = {}
    for m in insanlar:
        kisi[m.get("level") or ""] = kisi.get(m.get("level") or "", 0.0) + float(m.get("persons") or 0.0)
    kalabalik = [k for k in _katlar(bina) if kisi.get(k["level"], 0.0) >= sinir_k]
    tutmayan = []
    if yukseklik >= sinir_y:
        tutmayan.append(f"yapı yüksekliği {_m(yukseklik)} m 21,50 m'den az değil (a)")
    for k in kalabalik:
        tutmayan.append(f"{k.get('name') or k['level']} {_kisi(kisi[k['level']])} kişi, 50'den az değil (b)")
    bulgular = []
    for k, merdiven in tek:
        uids = merdiven or [s["uid"] for s in mahaller if s.get("level") == k["level"]][:1]
        if tutmayan:
            bulgular.append(_bulgu(
                "bykhy_single_stair",
                f"{k.get('name') or k['level']}: {len(merdiven)} kaçış merdiveni; en az 2 bağımsız kaçış merdiveni "
                "gerekir, tek merdivenin koşulları tutmuyor: " + "; ".join(tutmayan),
                "İkinci bir bağımsız kaçış merdiveni ekleyin.", hal, uids, f"{k['level']}:merdiven"))
        else:
            bulgular.append(_bulgu(
                "bykhy_single_stair",
                f"{k.get('name') or k['level']}: {len(merdiven)} kaçış merdiveni; tek merdiven (c) kaçış uzaklıkları "
                "Ek-5/B'ye uygun, (ç) yapımda yanmaz ürünler, (d) kolay alevlenen madde kullanılmıyorsa yeter: "
                "çizimden kanıtlanmaz",
                "Kaçış uzaklıklarını Mimari denetimin BYKHY 32 bulgularıyla, yapım ürünlerini ve kullanılan maddeleri "
                "belgeleyin; idare karar verir.", hal, uids, f"{k['level']}:koşullar", y.UZMAN))
    if tutmayan:
        satir = _satir(hal, y.KALDI, f"{len(tek)} katta tek kaçış merdiveni; " + "; ".join(tutmayan))
    else:
        satir = _satir(hal, y.UZMAN, f"{len(tek)} katta tek kaçış merdiveni; (a) ve (b) tutuyor, (c)–(d) idarenin")
    return bulgular, satir


def kural(bina: dict[str, Any], gun: date) -> y.Sonuc:
    """Başvuru günü ``gun`` olan projenin kullanım sınıfına özel bulguları."""
    sinif_, nereden = siniflar(bina)
    bulgular: list[dict[str, Any]] = []
    satirlar: list[dict[str, Any]] = []
    for madde, gereken, denetim in (("49/1-a", "saglik", hasta_odalari), ("50/1-a", "konaklama", otel_katlari),
                                    ("50/1-b", "konaklama", otel_kapilari), ("52", "m52", merdivenler)):
        hal = y.hal("BYKHY", madde, gun)
        if hal is None:
            satirlar.append(_bilinmiyor(madde, gun))
            continue
        if gereken not in sinif_:
            kim = {"saglik": "sağlık yapısı", "konaklama": "konaklama yapısı",
                   "m52": "fabrika, imalathane, mağaza, dükkân, depo, büro ya da ayakta tedavi merkezi"}[gereken]
            satirlar.append(_satir(hal, y.KAPSAM_DISI, f"{nereden} {kim} değil"))
            continue
        b, s = denetim(bina, hal)
        bulgular += b
        satirlar.append(s)
    return y.Sonuc(bulgular, satirlar)


def kontrol(facts: dict[str, Any], gun: date | None, neden: str | None) -> y.Sonuc:
    """Projenin kullanım sınıfına özel kaçış bulguları (``yururluk.denetle`` ile)."""
    bina = facts.get("building") or {}
    if not bina.get("storeys"):
        return y.Sonuc([], [])
    return y.denetle(lambda g: kural(bina, g), MADDELER, gun, neden, "kullanım sınıfı kuralı")
